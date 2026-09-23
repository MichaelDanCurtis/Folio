//! Project body bookmarks into native ranges and put range markers back into
//! paragraph XML without flattening source run properties or hyperlinks.
use crate::{document_bookmarks as bm, office::*};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub const EVENTS: &str = "_folioBookmarkEvents";
fn marker(n: roxmltree::Node) -> bool {
    n.has_tag_name((W, "bookmarkStart")) || n.has_tag_name((W, "bookmarkEnd"))
}
fn namespaces(dom: &roxmltree::Document) -> String {
    dom.root_element()
        .namespaces()
        .filter(|n| !matches!(n.name(), Some("w" | "r")))
        .map(|n| match n.name() {
            Some(p) => format!(" xmlns:{p}=\"{}\"", esc(n.uri())),
            None => format!(" xmlns=\"{}\"", esc(n.uri())),
        })
        .collect()
}
pub fn import(document: &mut Value, xml: &str) -> Result<bool, String> {
    let dom = parse(xml)?;
    let body = dom
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .ok_or("Word body is missing")?;
    let mut sources = HashMap::new();
    for (i, n) in body
        .children()
        .filter(|n| n.is_element() && !n.has_tag_name((W, "sectPr")))
        .enumerate()
    {
        crate::docx_tables::source_map(xml, n, format!("word-{i}"), &mut sources);
    }
    let index = bm::Index::new(document);
    let mut starts: HashMap<String, Value> = HashMap::new();
    let mut ends: HashMap<String, usize> = HashMap::new();
    let ns = namespaces(&dom);
    for p in &index.paragraphs {
        let Some(source) = bm::at(document, &p.path)["attrs"]["officeId"]
            .as_str()
            .and_then(|id| sources.get(id))
        else {
            continue;
        };
        let wrapped = crate::docx::wrap_fragment(source, &ns);
        let paragraph = parse(&wrapped)?;
        // A bookmark around a footnote reference, symbol or another unmodeled
        // inline object must not collapse to a guessed zero-length text range.
        if paragraph
            .descendants()
            .filter(|n| n.has_tag_name((W, "r")))
            .any(|run| {
                run.children().any(|n| {
                    n.is_element()
                        && ![
                            "rPr",
                            "t",
                            "tab",
                            "br",
                            "cr",
                            "commentReference",
                            "fldChar",
                            "instrText",
                        ]
                        .iter()
                        .any(|t| n.has_tag_name((W, *t)))
                })
            })
        {
            continue;
        }
        let mut offset = p.start;
        for n in paragraph.descendants().filter(|n| n.is_element()) {
            if marker(n) {
                let Some(id) = n.attribute((W, "id")).filter(|s| s.parse::<u64>().is_ok()) else {
                    continue;
                };
                if n.has_tag_name((W, "bookmarkStart")) {
                    let Some(name) = n.attribute((W, "name")).filter(|s| bm::valid_name(s, true))
                    else {
                        continue;
                    };
                    if n.attributes().any(|a| !matches!(a.name(), "id" | "name")) {
                        continue;
                    }
                    starts.insert(id.into(), json!({"name":name,"sourceId":id,"start":offset}));
                } else {
                    ends.insert(id.into(), offset);
                }
            } else if n.has_tag_name((W, "t")) {
                offset += n.text().unwrap_or("").encode_utf16().count();
            } else if ["tab", "br", "cr"].iter().any(|t| n.has_tag_name((W, *t))) {
                offset += 1;
            }
        }
    }
    // Duplicate or unmatched identifiers are kept in the source package, not
    // assigned a guessed destination. Column bookmarks are likewise preserved.
    let mut counts: HashMap<String, (usize, usize)> = HashMap::new();
    for n in body.descendants().filter(|n| marker(*n)) {
        let count = counts
            .entry(n.attribute((W, "id")).unwrap_or("").into())
            .or_default();
        if n.has_tag_name((W, "bookmarkStart")) {
            count.0 += 1;
        } else {
            count.1 += 1;
        }
    }
    let mut list = vec![];
    let mut names = HashSet::new();
    let mut ids = starts.keys().cloned().collect::<Vec<_>>();
    ids.sort_by_key(|id| (starts[id]["start"].as_u64().unwrap_or(0), id.clone()));
    for id in ids {
        let mut b = starts.remove(&id).unwrap();
        if counts.get(&id) != Some(&(1, 1)) {
            continue;
        }
        let Some(end) = ends.get(&id) else {
            continue;
        };
        if b["start"].as_u64().unwrap_or(0) > *end as u64
            || !names.insert(b["name"].as_str().unwrap().to_lowercase())
        {
            continue;
        }
        b["end"] = json!(end);
        list.push(b);
    }
    let preserved = counts.len() != list.len();
    if !counts.is_empty() {
        document["attrs"]["bookmarks"] = json!(list);
    }
    bm::validate(document)?;
    Ok(preserved)
}
pub fn source_ids(document: Option<&Value>) -> HashSet<String> {
    document
        .into_iter()
        .flat_map(bm::entries)
        .filter_map(|b| b["sourceId"].as_str().map(String::from))
        .collect()
}
pub fn strip(xml: &str, ids: &HashSet<String>) -> Result<String, String> {
    let dom = parse(xml)?;
    Ok(replace_ranges(
        xml,
        dom.descendants()
            .filter(|n| marker(*n) && n.attribute((W, "id")).is_some_and(|id| ids.contains(id)))
            .map(|n| (n.range(), String::new()))
            .collect(),
    ))
}
pub fn project(document: &Value, source: &str) -> Result<Value, String> {
    bm::validate(document)?;
    let mut result = document.clone();
    fn clear(n: &mut Value) {
        if let Some(a) = n.get_mut("attrs").and_then(Value::as_object_mut) {
            a.remove(EVENTS);
        }
        if let Some(children) = n.get_mut("content").and_then(Value::as_array_mut) {
            for n in children {
                clear(n);
            }
        }
    }
    clear(&mut result);
    let index = bm::Index::new(document);
    let dom = parse(source)?;
    let mut used = dom
        .descendants()
        .filter(|n| marker(*n))
        .filter_map(|n| n.attribute((W, "id")))
        .filter_map(|s| s.parse::<u64>().ok())
        .collect::<HashSet<_>>();
    let mut serial = 0u64;
    for b in bm::entries(document) {
        while used.contains(&serial) {
            serial = serial
                .checked_add(1)
                .ok_or("Too many bookmark identifiers")?;
        }
        let id = serial;
        used.insert(id);
        for (edge, start) in [("start", true), ("end", false)] {
            let position = b[edge].as_u64().unwrap() as usize;
            let paragraph = index
                .paragraph(position)
                .ok_or("Bookmark destination is unavailable")?;
            let node = bm::at_mut(&mut result, &paragraph.path);
            if node["attrs"].get(EVENTS).is_none() {
                node["attrs"][EVENTS] = json!([]);
            }
            node["attrs"][EVENTS].as_array_mut().unwrap().push(json!({
                "offset":position-paragraph.start,"id":id,"start":start,"name":b["name"]
            }));
        }
    }
    Ok(result)
}

struct Event {
    offset: usize,
    xml: String,
}
struct Renderer<'a> {
    source: &'a str,
    offset: usize,
    events: Vec<Event>,
    next: usize,
}
fn shells(xml: &str, n: roxmltree::Node) -> (String, String) {
    let s = raw(xml, n);
    let end = opening_tag_end(s).unwrap();
    let name = s[1..]
        .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .next()
        .unwrap();
    (
        format!("{}>", s[..end].trim_end_matches('/')),
        format!("</{name}>"),
    )
}
impl Renderer<'_> {
    fn markers(&mut self) -> String {
        let mut out = String::new();
        while self.next < self.events.len() && self.events[self.next].offset == self.offset {
            out.push_str(&self.events[self.next].xml);
            self.next += 1;
        }
        out
    }
    fn node(&mut self, n: roxmltree::Node) -> String {
        if n.has_tag_name((W, "r")) {
            return self.run(n);
        }
        if n.has_tag_name((W, "pPr")) || !n.descendants().any(|c| c.has_tag_name((W, "r"))) {
            return raw(self.source, n).into();
        }
        let (open, close) = shells(self.source, n);
        let mut out = open;
        let fields = crate::docx_fields::atomic_ranges(n);
        let mut skip_until = 0;
        for c in n.children() {
            if c.range().start < skip_until {
                continue;
            }
            if let Some((range, length)) = fields.iter().find(|(r, _)| r.start == c.range().start) {
                if !self.events[self.next..]
                    .iter()
                    .any(|e| e.offset > self.offset && e.offset < self.offset + length)
                {
                    out.push_str(&self.markers());
                    out.push_str(&self.source[range.clone()]);
                    self.offset += length;
                    out.push_str(&self.markers());
                    skip_until = range.end;
                    continue;
                }
            }
            out.push_str(&self.node(c));
        }
        out.push_str(&self.markers());
        out.push_str(&close);
        out
    }
    fn run(&mut self, n: roxmltree::Node) -> String {
        let (open, close) = shells(self.source, n);
        let props = n
            .children()
            .find(|c| c.has_tag_name((W, "rPr")))
            .map(|c| raw(self.source, c))
            .unwrap_or("");
        let mut buffer = String::new();
        let mut out = String::new();
        let flush = |buffer: &mut String, out: &mut String| {
            if !buffer.is_empty() {
                out.push_str(&open);
                out.push_str(props);
                out.push_str(buffer);
                out.push_str(&close);
                buffer.clear();
            }
        };
        for part in n.children().filter(|p| !p.has_tag_name((W, "rPr"))) {
            if self.next < self.events.len() && self.events[self.next].offset == self.offset {
                flush(&mut buffer, &mut out);
                out.push_str(&self.markers());
            }
            if part.has_tag_name((W, "t")) {
                let text = part.text().unwrap_or("");
                let mut chunk = String::new();
                for c in text.chars() {
                    if self.next < self.events.len() && self.events[self.next].offset == self.offset
                    {
                        if !chunk.is_empty() {
                            buffer.push_str(&format!(
                                "<w:t xmlns:w=\"{W}\" xml:space=\"preserve\">{}</w:t>",
                                esc(&chunk)
                            ));
                            chunk.clear();
                        }
                        flush(&mut buffer, &mut out);
                        out.push_str(&self.markers());
                    }
                    chunk.push(c);
                    self.offset += c.len_utf16();
                }
                if !chunk.is_empty() {
                    buffer.push_str(&format!(
                        "<w:t xmlns:w=\"{W}\" xml:space=\"preserve\">{}</w:t>",
                        esc(&chunk)
                    ));
                }
            } else {
                buffer.push_str(raw(self.source, part));
                if ["tab", "br", "cr"]
                    .iter()
                    .any(|t| part.has_tag_name((W, *t)))
                {
                    self.offset += 1;
                }
            }
        }
        flush(&mut buffer, &mut out);
        out.push_str(&self.markers());
        out
    }
}
pub fn inject(xml: &str, values: &Value, namespaces: &str) -> String {
    let Some(values) = values.as_array().filter(|v| !v.is_empty()) else {
        return xml.into();
    };
    let events = values
        .iter()
        .map(|v| Event {
            offset: v["offset"].as_u64().unwrap() as usize,
            xml: if v["start"] == true {
                format!(
                    "<w:bookmarkStart xmlns:w=\"{W}\" w:id=\"{}\" w:name=\"{}\"/>",
                    v["id"],
                    esc(v["name"].as_str().unwrap())
                )
            } else {
                format!("<w:bookmarkEnd xmlns:w=\"{W}\" w:id=\"{}\"/>", v["id"])
            },
        })
        .collect::<Vec<_>>();
    inject_events(xml, events, namespaces)
}
/// Insert trusted, generated range XML using the same run-preserving renderer.
pub fn inject_xml(xml: &str, values: &Value, namespaces: &str) -> String {
    let events = values
        .as_array()
        .into_iter()
        .flatten()
        .map(|v| Event {
            offset: v["offset"].as_u64().unwrap() as usize,
            xml: v["xml"].as_str().unwrap().to_string(),
        })
        .collect();
    inject_events(xml, events, namespaces)
}
fn inject_events(xml: &str, mut events: Vec<Event>, namespaces: &str) -> String {
    events.sort_by_key(|e| e.offset); // stable: a point's start precedes its end
    let wrapped = crate::docx::wrap_fragment(xml, namespaces);
    let Ok(dom) = parse(&wrapped) else {
        return xml.into();
    };
    let Some(p) = dom
        .root_element()
        .children()
        .find(|n| n.has_tag_name((W, "p")))
    else {
        return xml.into();
    };
    let mut renderer = Renderer {
        source: &wrapped,
        offset: 0,
        events,
        next: 0,
    };
    let out = if p.descendants().any(|n| n.has_tag_name((W, "r"))) {
        renderer.node(p)
    } else {
        // Empty paragraphs still accept point bookmarks.
        let (open, close) = shells(&wrapped, p);
        let inner = p.children().map(|n| raw(&wrapped, n)).collect::<String>();
        format!("{open}{inner}{}{close}", renderer.markers())
    };
    // Export validates all generated markers before writing the package. A
    // projection mismatch must produce an ordinary error, never panic in FFI.
    if renderer.next != renderer.events.len() {
        return xml.into();
    }
    out
}
pub fn check_generated(document: &Value, xml: &str) -> Result<(), String> {
    fn collect(n: &Value, out: &mut Vec<(u64, bool, String)>) {
        if let Some(events) = n["attrs"][EVENTS].as_array() {
            for e in events {
                out.push((
                    e["id"].as_u64().unwrap(),
                    e["start"] == true,
                    if e["start"] == true {
                        e["name"].as_str().unwrap().into()
                    } else {
                        String::new()
                    },
                ));
            }
        }
        if let Some(children) = n["content"].as_array() {
            for n in children {
                collect(n, out);
            }
        }
    }
    let mut expected = vec![];
    collect(document, &mut expected);
    let ids = expected.iter().map(|e| e.0).collect::<HashSet<_>>();
    let dom = parse(xml)?;
    let mut actual = dom
        .descendants()
        .filter(|n| marker(*n))
        .filter_map(|n| {
            let id = n.attribute((W, "id"))?.parse::<u64>().ok()?;
            ids.contains(&id).then(|| {
                (
                    id,
                    n.has_tag_name((W, "bookmarkStart")),
                    n.attribute((W, "name")).unwrap_or("").to_string(),
                )
            })
        })
        .collect::<Vec<_>>();
    expected.sort();
    actual.sort();
    if expected != actual {
        return Err("A bookmark cannot be represented in the exported paragraph. The Word file was not changed.".into());
    }
    Ok(())
}
pub fn check_preserved(before: &str, after: &str, managed: &HashSet<String>) -> Result<(), String> {
    fn records(
        xml: &str,
        managed: &HashSet<String>,
    ) -> Result<Vec<(String, String, Vec<(String, String)>)>, String> {
        let dom = parse(xml)?;
        let mut out = dom
            .descendants()
            .filter(|n| {
                marker(*n)
                    && !n
                        .attribute((W, "id"))
                        .is_some_and(|id| managed.contains(id))
            })
            .map(|n| {
                (
                    n.attribute((W, "id")).unwrap_or("").to_string(),
                    n.tag_name().name().to_string(),
                    n.attributes()
                        .map(|a| (a.name().to_string(), a.value().to_string()))
                        .collect(),
                )
            })
            .collect::<Vec<_>>();
        out.sort();
        Ok(out)
    }
    // Advanced/unmodeled bookmarks must never silently disappear on export.
    let original = records(before, managed)?;
    let ids = original
        .iter()
        .map(|r| r.0.as_str())
        .collect::<HashSet<_>>();
    let current = records(after, managed)?
        .into_iter()
        .filter(|r| ids.contains(r.0.as_str()))
        .collect::<Vec<_>>();
    if original != current {
        return Err("This edit changes a preserved Word bookmark that Folio cannot yet update. Undo the edit or keep the workspace as a Folio file; the original Word file is unchanged.".into());
    }
    Ok(())
}
