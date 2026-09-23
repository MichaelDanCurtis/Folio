//! Word review threads. Unknown records and anchors stay in the source package;
//! export fails if a body edit would silently erase an unmanaged anchor.
use crate::{document_bookmarks as ranges, document_comments as comments, office::*};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const W14: &str = "http://schemas.microsoft.com/office/word/2010/wordml";
const W15: &str = "http://schemas.microsoft.com/office/word/2012/wordml";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments";
const RX: &str = "http://schemas.microsoft.com/office/2011/relationships/commentsExtended";
const MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml";
const MIMEX: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml";
pub const EVENTS: &str = "_folioCommentEvents";
fn marker(n: roxmltree::Node) -> bool {
    ["commentRangeStart", "commentRangeEnd", "commentReference"]
        .iter()
        .any(|tag| n.has_tag_name((W, *tag)))
}
fn part(pkg: &Package, kind: &str) -> Result<Option<String>, String> {
    let Ok(xml) = pkg.text("word/_rels/document.xml.rels") else {
        return Ok(None);
    };
    let dom = parse(xml)?;
    let nodes = dom
        .root_element()
        .children()
        .filter(|n| attr(*n, "Type") == Some(kind))
        .collect::<Vec<_>>();
    if nodes.len() > 1 {
        return Err("Ambiguous Word comment relationships".into());
    }
    let Some(n) = nodes.first() else {
        return Ok(None);
    };
    if attr(*n, "TargetMode") == Some("External") {
        return Err("External Word comment parts are unsupported".into());
    }
    let target = attr(*n, "Target").ok_or("Missing Word comment part target")?;
    let path = if target.starts_with('/') {
        target[1..].to_string()
    } else {
        format!("word/{target}")
    };
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => (),
            ".." => {
                if segments.pop().is_none() {
                    return Err("Invalid Word comment part target".into());
                }
            }
            _ => segments.push(segment),
        }
    }
    Ok(Some(segments.join("/")))
}
fn attach(pkg: &mut Package, path: &str, kind: &str, mime: &str) -> Result<(), String> {
    let rels = pkg
        .text("word/_rels/document.xml.rels")
        .map(String::from)
        .unwrap_or(format!("<Relationships xmlns=\"{REL_NS}\"/>"));
    let dom = parse(&rels)?;
    let root = dom.root_element();
    if !root.children().any(|n| attr(n, "Type") == Some(kind)) {
        let mut serial = 1;
        while root
            .children()
            .any(|n| attr(n, "Id") == Some(&format!("folioComment{serial}")))
        {
            serial += 1;
        }
        let inner = root.children().map(|n| raw(&rels, n)).collect::<String>();
        pkg.set("word/_rels/document.xml.rels",replace_inner(&rels,root,&format!("{inner}<Relationship xmlns=\"{REL_NS}\" Id=\"folioComment{serial}\" Type=\"{kind}\" Target=\"/{}\"/>",esc(path))));
    }
    let ct = pkg.text("[Content_Types].xml")?.to_string();
    let dom = parse(&ct)?;
    let root = dom.root_element();
    if !root
        .children()
        .any(|n| attr(n, "PartName") == Some(&format!("/{path}")))
    {
        let inner = root.children().map(|n| raw(&ct, n)).collect::<String>();
        pkg.set("[Content_Types].xml",replace_inner(&ct,root,&format!("{inner}<Override xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\" PartName=\"/{}\" ContentType=\"{mime}\"/>",esc(path))));
    }
    Ok(())
}
fn text(n: roxmltree::Node) -> String {
    n.descendants()
        .filter(|p| p.has_tag_name((W, "p")))
        .map(|p| {
            p.descendants()
                .filter(|n| n.is_element())
                .map(|n| {
                    if n.has_tag_name((W, "t")) {
                        n.text().unwrap_or("")
                    } else if n.has_tag_name((W, "tab")) {
                        "\t"
                    } else if n.has_tag_name((W, "br")) || n.has_tag_name((W, "cr")) {
                        "\n"
                    } else {
                        ""
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn last_para<'a, 'i>(n: roxmltree::Node<'a, 'i>) -> Option<roxmltree::Node<'a, 'i>> {
    n.descendants().filter(|p| p.has_tag_name((W, "p"))).last()
}
fn para_id(n: roxmltree::Node) -> Option<String> {
    last_para(n)?.attribute((W14, "paraId")).map(String::from)
}
fn rich(n: roxmltree::Node) -> bool {
    n.descendants().filter(|n| n.is_element()).any(|n| {
        ![
            "comment",
            "p",
            "pPr",
            "pStyle",
            "r",
            "rPr",
            "rStyle",
            "t",
            "tab",
            "br",
            "cr",
            "annotationRef",
        ]
        .iter()
        .any(|tag| n.has_tag_name((W, *tag)))
    })
}
fn anchors(document: &Value, xml: &str) -> Result<HashMap<String, (usize, usize)>, String> {
    let dom = parse(xml)?;
    let body = dom
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .ok_or("Word body missing")?;
    let mut sources = HashMap::new();
    for (i, n) in body
        .children()
        .filter(|n| n.is_element() && !n.has_tag_name((W, "sectPr")))
        .enumerate()
    {
        crate::docx_tables::source_map(xml, n, format!("word-{i}"), &mut sources);
    }
    let mut positions: HashMap<String, (Vec<usize>, Vec<usize>, Vec<usize>)> = HashMap::new();
    for p in &ranges::Index::new(document).paragraphs {
        let Some(source) = ranges::at(document, &p.path)["attrs"]["officeId"]
            .as_str()
            .and_then(|id| sources.get(id))
        else {
            continue;
        };
        // Parse in original namespace scope, including nonstandard prefixes.
        let ns = dom
            .root_element()
            .namespaces()
            .filter(|n| !matches!(n.name(), Some("w" | "r")))
            .map(|n| match n.name() {
                Some(p) => format!(" xmlns:{p}=\"{}\"", esc(n.uri())),
                None => format!(" xmlns=\"{}\"", esc(n.uri())),
            })
            .collect::<String>();
        let wrapped = crate::docx::wrap_fragment(source, &ns);
        let paragraph = parse(&wrapped)?;
        if paragraph
            .descendants()
            .filter(|n| n.has_tag_name((W, "r")))
            .any(|r| {
                r.children().any(|n| {
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
                        .any(|tag| n.has_tag_name((W, *tag)))
                })
            })
        {
            continue;
        }
        let mut offset = p.start;
        for n in paragraph.descendants().filter(|n| n.is_element()) {
            if marker(n) {
                let Some(id) = n.attribute((W, "id")) else {
                    continue;
                };
                let entry = positions.entry(id.into()).or_default();
                match n.tag_name().name() {
                    "commentRangeStart" => entry.0.push(offset),
                    "commentRangeEnd" => entry.1.push(offset),
                    _ => entry.2.push(offset),
                }
            } else if n.has_tag_name((W, "t")) {
                offset += n.text().unwrap_or("").encode_utf16().count();
            } else if ["tab", "br", "cr"]
                .iter()
                .any(|tag| n.has_tag_name((W, *tag)))
            {
                offset += 1;
            }
        }
    }
    let mut result = HashMap::new();
    for (id, (a, b, r)) in positions {
        let total = body
            .descendants()
            .filter(|n| marker(*n) && n.attribute((W, "id")) == Some(&id))
            .count();
        if r.len() == 1 && total == a.len() + b.len() + r.len() {
            if a.len() == 1 && b.len() == 1 && a[0] <= b[0] {
                result.insert(id, (a[0], b[0]));
            } else if a.is_empty() && b.is_empty() {
                result.insert(id, (r[0], r[0]));
            }
        }
    }
    Ok(result)
}
pub fn import(document: &mut Value, pkg: &Package) -> Result<bool, String> {
    let Some(path) = part(pkg, R)? else {
        return Ok(false);
    };
    let xml = pkg.text(&path)?;
    let dom = parse(xml)?;
    let extended = part(pkg, RX)?
        .map(|p| pkg.text(&p).map(String::from))
        .transpose()?;
    let mut parents = HashMap::new();
    let mut done = HashSet::new();
    let mut duplicate_paras = HashSet::new();
    let mut seen = HashSet::new();
    if let Some(xml) = &extended {
        for n in parse(xml)?
            .descendants()
            .filter(|n| n.has_tag_name((W15, "commentEx")))
        {
            if let Some(id) = n.attribute((W15, "paraId")) {
                if !seen.insert(id.to_string()) {
                    duplicate_paras.insert(id.to_string());
                }
                if let Some(parent) = n.attribute((W15, "paraIdParent")) {
                    parents.insert(id.to_string(), parent.to_string());
                }
                if matches!(n.attribute((W15, "done")), Some("1" | "true" | "on")) {
                    done.insert(id.to_string());
                }
            }
        }
    }
    let records = dom
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((W, "comment")))
        .collect::<Vec<_>>();
    let mut ids = HashSet::new();
    let mut duplicates = HashSet::new();
    let mut paras = HashMap::new();
    for n in &records {
        let id = n.attribute((W, "id")).unwrap_or("").to_string();
        if !ids.insert(id.clone()) {
            duplicates.insert(id.clone());
        }
        if let Some(para) = para_id(*n) {
            if paras.insert(para.clone(), id).is_some() {
                duplicate_paras.insert(para);
            }
        }
    }
    let positions = anchors(document, pkg.text("word/document.xml")?)?;
    let mut messages = HashMap::new();
    let mut roots = HashMap::new();
    for n in &records {
        let Some(id) = n
            .attribute((W, "id"))
            .filter(|id| id.parse::<u32>().is_ok() && !duplicates.contains(*id))
        else {
            continue;
        };
        let para = para_id(*n);
        if para.as_ref().is_some_and(|p| duplicate_paras.contains(p)) {
            continue;
        }
        let mut root = id.to_string();
        let mut p = para.clone();
        let mut visited = HashSet::new();
        let mut valid = true;
        while let Some(parent) = p.as_ref().and_then(|p| parents.get(p)) {
            if !visited.insert(parent.clone()) || duplicate_paras.contains(parent) {
                valid = false;
                break;
            }
            let Some(parent_id) = paras.get(parent) else {
                valid = false;
                break;
            };
            root = parent_id.clone();
            p = Some(parent.clone());
        }
        if !valid || !positions.contains_key(&root) {
            continue;
        }
        // Reply anchors, when present, must agree with the parent thread.
        if positions
            .get(id)
            .is_some_and(|r| Some(r) != positions.get(&root))
        {
            continue;
        }
        let mut m = json!({"id":format!("word-message-{id}"),"sourceId":id,"author":n.attribute((W,"author")).unwrap_or(""),"initials":n.attribute((W,"initials")).unwrap_or(""),"text":text(*n),"date":n.attribute((W,"date")).unwrap_or(""),"readOnly":rich(*n)});
        if let Some(para) = para {
            m["paraId"] = json!(para);
        }
        messages.insert(id.to_string(), m);
        roots.insert(id.to_string(), root);
    }
    // A partially representable conversation stays wholly in the source. In
    // particular, deleting its root must not orphan an unmodeled reply.
    let mut blocked_roots = HashSet::new();
    for n in &records {
        let id = n.attribute((W, "id")).unwrap_or("");
        if messages.contains_key(id) {
            continue;
        }
        let mut p = para_id(*n);
        let mut root = id.to_string();
        let mut visited = HashSet::new();
        while let Some(parent) = p.as_ref().and_then(|p| parents.get(p)) {
            if !visited.insert(parent.clone()) {
                break;
            }
            let Some(parent_id) = paras.get(parent) else {
                break;
            };
            root = parent_id.clone();
            p = Some(parent.clone());
        }
        blocked_roots.insert(root);
    }
    let mut threads = vec![];
    let mut consumed = HashSet::new();
    for n in &records {
        let id = n.attribute((W, "id")).unwrap_or("");
        if blocked_roots.contains(id) || roots.get(id).map(String::as_str) != Some(id) {
            continue;
        }
        let Some(m) = messages.get(id) else {
            continue;
        };
        let mut list = vec![m.clone()];
        consumed.insert(id.to_string());
        for reply in &records {
            let rid = reply.attribute((W, "id")).unwrap_or("");
            if rid != id && roots.get(rid).map(String::as_str) == Some(id) {
                if let Some(m) = messages.get(rid) {
                    list.push(m.clone());
                    consumed.insert(rid.to_string());
                }
            }
        }
        let (start, end) = positions[id];
        threads.push(json!({"id":format!("word-thread-{id}"),"start":start,"end":end,"resolved":m["paraId"].as_str().is_some_and(|p| done.contains(p)),"messages":list}));
    }
    threads.sort_by_key(|t| t["start"].as_u64());
    document["attrs"][comments::KEY] = json!(threads);
    comments::validate(document)?;
    Ok(consumed.len() != records.len())
}
pub fn source_ids(document: Option<&Value>) -> HashSet<String> {
    document
        .into_iter()
        .flat_map(comments::entries)
        .flat_map(|t| t["messages"].as_array().into_iter().flatten())
        .filter_map(|m| m["sourceId"].as_str().map(String::from))
        .collect()
}
pub fn strip(xml: &str, managed: &HashSet<String>) -> Result<String, String> {
    let dom = parse(xml)?;
    Ok(replace_ranges(
        xml,
        dom.descendants()
            .filter(|n| {
                marker(*n)
                    && n.attribute((W, "id"))
                        .is_some_and(|id| managed.contains(id))
            })
            .map(|n| (n.range(), String::new()))
            .collect(),
    ))
}
fn add_attribute(xml: &str, n: roxmltree::Node, attribute: &str) -> String {
    let at = n.range().start + opening_tag_end(raw(xml, n)).unwrap();
    let at = if xml.as_bytes()[at - 1] == b'/' {
        at - 1
    } else {
        at
    };
    replace_ranges(xml, vec![(at..at, attribute.to_string())])
}
fn extension_record(
    source: Option<roxmltree::Node>,
    xml: &str,
    para: &str,
    parent: Option<&str>,
    done: bool,
) -> Result<String, String> {
    let Some(source) = source else {
        return Ok(format!(
            "<w15:commentEx xmlns:w15=\"{W15}\" w15:paraId=\"{}\" w15:done=\"{}\"{}/>",
            esc(para),
            if done { 1 } else { 0 },
            parent
                .map(|p| format!(" w15:paraIdParent=\"{}\"", esc(p)))
                .unwrap_or_default()
        ));
    };
    let scoped = scoped_record(xml, source);
    let dom = parse(&scoped)?;
    let root = dom.root_element();
    let mut replacements = vec![];
    let mut additions = String::new();
    for (name, value) in [
        ("done", Some(if done { "1" } else { "0" })),
        ("paraIdParent", parent),
    ] {
        if let Some(a) = root
            .attributes()
            .find(|a| a.namespace() == Some(W15) && a.name() == name)
        {
            if let Some(value) = value {
                replacements.push((a.range_value(), esc(value)));
            } else {
                replacements.push((a.range(), String::new()));
            }
        } else if let Some(value) = value {
            additions.push_str(&format!(" folioW15:{name}=\"{}\"", esc(value)));
        }
    }
    let changed = replace_ranges(&scoped, replacements);
    if additions.is_empty() {
        Ok(changed)
    } else {
        let dom = parse(&changed)?;
        Ok(add_attribute(
            &changed,
            dom.root_element(),
            &format!(" xmlns:folioW15=\"{W15}\"{additions}"),
        ))
    }
}
fn fresh(used: &mut HashSet<u32>, start: &mut u32) -> Result<u32, String> {
    while used.contains(start) {
        *start = start
            .checked_add(1)
            .filter(|n| *n < 0x80000000)
            .ok_or("Too many Word comment identifiers")?;
    }
    used.insert(*start);
    Ok(*start)
}
fn record(
    message: &Value,
    id: u32,
    para: &str,
    source: Option<roxmltree::Node>,
    xml: &str,
) -> Result<String, String> {
    if let Some(source) = source {
        if text(source) == message["text"].as_str().unwrap_or("") {
            let scoped = scoped_record(xml, source);
            let d = parse(&scoped)?;
            if let Some(p) = last_para(d.root_element()) {
                if p.attribute((W14, "paraId")).is_none() {
                    return Ok(add_attribute(
                        &scoped,
                        p,
                        &format!(" xmlns:folioW14=\"{W14}\" folioW14:paraId=\"{para}\""),
                    ));
                }
            }
            return Ok(scoped);
        }
        if rich(source) {
            return Err("A rich Word comment cannot be flattened during export".into());
        }
    }
    let paragraphs = message["text"]
        .as_str()
        .unwrap_or("")
        .split('\n')
        .collect::<Vec<_>>();
    let source_paragraphs = source
        .into_iter()
        .flat_map(|n| n.descendants().filter(|p| p.has_tag_name((W, "p"))))
        .collect::<Vec<_>>();
    let mut content = String::new();
    for (i, line) in paragraphs.iter().enumerate() {
        let previous = source_paragraphs
            .get(i)
            .or_else(|| source_paragraphs.last())
            .copied();
        let paragraph_properties = previous
            .and_then(|p| child(p, "pPr"))
            .map(|p| scoped_record(xml, p))
            .unwrap_or_default();
        let run_properties = previous
            .and_then(|p| p.children().find(|n| n.has_tag_name((W, "r"))))
            .and_then(|r| child(r, "rPr"))
            .map(|p| scoped_record(xml, p))
            .unwrap_or_default();
        content.push_str(&format!(
            "<w:p xmlns:w=\"{W}\" xmlns:w14=\"{W14}\"{}>{paragraph_properties}<w:r>{run_properties}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
            if i + 1 == paragraphs.len() {
                format!(" w14:paraId=\"{para}\"")
            } else {
                String::new()
            },
            esc(line)
        ));
    }
    if let Some(source) = source {
        let scoped = scoped_record(xml, source);
        let dom = parse(&scoped)?;
        return Ok(replace_inner(&scoped, dom.root_element(), &content));
    }
    Ok(format!("<w:comment xmlns:w=\"{W}\" xmlns:w14=\"{W14}\" w:id=\"{id}\" w:author=\"{}\" w:initials=\"{}\"{}>{content}</w:comment>",esc(message["author"].as_str().unwrap_or("")),esc(message["initials"].as_str().unwrap_or("")),message["date"].as_str().filter(|s| !s.is_empty()).map(|s| format!(" w:date=\"{}\"",esc(s))).unwrap_or_default()))
}
/// Add comment events to the same logical paragraphs used by bookmarks and
/// update only managed comment records in the Office package.
pub fn project(
    document: &Value,
    baseline: Option<&Value>,
    pkg: &mut Package,
) -> Result<Value, String> {
    comments::validate(document)?;
    let mut result = document.clone();
    fn clear(n: &mut Value) {
        if let Some(a) = n.get_mut("attrs").and_then(Value::as_object_mut) {
            a.remove(EVENTS);
        }
        if let Some(c) = n.get_mut("content").and_then(Value::as_array_mut) {
            for n in c {
                clear(n);
            }
        }
    }
    clear(&mut result);
    let mut managed = source_ids(baseline);
    managed.extend(source_ids(Some(document)));
    if comments::entries(document).is_empty() && managed.is_empty() {
        return Ok(result);
    }
    let path = part(pkg, R)?.unwrap_or("word/comments.xml".into());
    let pathx = part(pkg, RX)?.unwrap_or("word/commentsExtended.xml".into());
    let xml = pkg
        .text(&path)
        .map(String::from)
        .unwrap_or(format!("<w:comments xmlns:w=\"{W}\"/>"));
    let dom = parse(&xml)?;
    let root = dom.root_element();
    let xmlx = pkg
        .text(&pathx)
        .map(String::from)
        .unwrap_or(format!("<w15:commentsEx xmlns:w15=\"{W15}\"/>"));
    let domx = parse(&xmlx)?;
    let sources = root
        .children()
        .filter(|n| n.has_tag_name((W, "comment")))
        .filter_map(|n| Some((n.attribute((W, "id"))?.to_string(), n)))
        .collect::<HashMap<_, _>>();
    let mut used = sources
        .keys()
        .filter_map(|s| s.parse::<u32>().ok())
        .collect::<HashSet<_>>();
    let mut serial = 0;
    used.extend(
        parse(pkg.text("word/document.xml")?)?
            .descendants()
            .filter(|n| marker(*n))
            .filter_map(|n| n.attribute((W, "id")))
            .filter_map(|s| s.parse::<u32>().ok()),
    );
    let mut used_paras = dom
        .descendants()
        .filter_map(|n| n.attribute((W14, "paraId")))
        .filter_map(|s| u32::from_str_radix(s, 16).ok())
        .collect::<HashSet<_>>();
    used_paras.extend(
        domx.descendants()
            .filter_map(|n| n.attribute((W15, "paraId")))
            .filter_map(|s| u32::from_str_radix(s, 16).ok()),
    );
    let mut para_serial = 1;
    let managed_paras = sources
        .iter()
        .filter(|(id, _)| managed.contains(*id))
        .filter_map(|(_, n)| para_id(*n))
        .collect::<HashSet<_>>();
    let ex_sources = domx
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((W15, "commentEx")))
        .filter_map(|n| Some((n.attribute((W15, "paraId"))?.to_string(), n)))
        .collect::<HashMap<_, _>>();
    let mut records = String::new();
    let mut ex_records = String::new();
    let index = ranges::Index::new(document);
    for thread in comments::entries(document) {
        let current_paras = thread["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|m| m["sourceId"].as_str().and_then(|id| sources.get(id)))
            .filter_map(|n| para_id(*n))
            .collect::<HashSet<_>>();
        let mut parent = String::new();
        let mut root_id = 0;
        for (i, m) in thread["messages"].as_array().unwrap().iter().enumerate() {
            let source = m["sourceId"]
                .as_str()
                .and_then(|id| sources.get(id))
                .copied();
            let id = if let Some(source) = source {
                source
                    .attribute((W, "id"))
                    .unwrap()
                    .parse::<u32>()
                    .map_err(|_| "Invalid Word comment identifier")?
            } else {
                fresh(&mut used, &mut serial)?
            };
            let para = source
                .and_then(para_id)
                .unwrap_or(format!("{:08X}", fresh(&mut used_paras, &mut para_serial)?));
            if i == 0 {
                parent = para.clone();
                root_id = id;
            }
            records.push_str(&record(m, id, &para, source, &xml)?);
            ex_records.push_str(&extension_record(
                ex_sources.get(&para).copied(),
                &xmlx,
                &para,
                if i == 0 {
                    None
                } else {
                    Some(
                        ex_sources
                            .get(&para)
                            .and_then(|n| n.attribute((W15, "paraIdParent")))
                            .filter(|p| current_paras.contains(*p))
                            .unwrap_or(&parent),
                    )
                },
                thread["resolved"] == true,
            )?);
        }
        for (edge, start) in [("start", true), ("end", false)] {
            let offset = thread[edge].as_u64().unwrap() as usize;
            let p = index
                .paragraph(offset)
                .ok_or("Comment destination is unavailable")?;
            let node = ranges::at_mut(&mut result, &p.path);
            if node["attrs"].get(EVENTS).is_none() {
                node["attrs"][EVENTS] = json!([]);
            }
            let marker = if start {
                format!("<w:commentRangeStart xmlns:w=\"{W}\" w:id=\"{root_id}\"/>")
            } else {
                format!("<w:commentRangeEnd xmlns:w=\"{W}\" w:id=\"{root_id}\"/><w:r xmlns:w=\"{W}\"><w:rPr><w:rStyle w:val=\"CommentReference\"/></w:rPr><w:commentReference w:id=\"{root_id}\"/></w:r>")
            };
            node["attrs"][EVENTS]
                .as_array_mut()
                .unwrap()
                .push(json!({"offset":offset-p.start,"xml":marker,"id":root_id,"start":start}));
        }
    }
    let retained = root
        .children()
        .filter(|n| {
            !n.has_tag_name((W, "comment"))
                || !n
                    .attribute((W, "id"))
                    .is_some_and(|id| managed.contains(id))
        })
        .map(|n| raw(&xml, n))
        .collect::<String>();
    let retainedx = domx
        .root_element()
        .children()
        .filter(|n| {
            !n.has_tag_name((W15, "commentEx"))
                || !n
                    .attribute((W15, "paraId"))
                    .is_some_and(|id| managed_paras.contains(id))
        })
        .map(|n| raw(&xmlx, n))
        .collect::<String>();
    pkg.set(&path, replace_inner(&xml, root, &(retained + &records)));
    pkg.set(
        &pathx,
        replace_inner(&xmlx, domx.root_element(), &(retainedx + &ex_records)),
    );
    let remaining = source_ids(Some(document));
    let deleted_paras = sources
        .iter()
        .filter(|(id, _)| managed.contains(*id) && !remaining.contains(*id))
        .filter_map(|(_, n)| para_id(*n))
        .collect::<HashSet<_>>();
    prune_modern_metadata(pkg, &deleted_paras)?;
    attach(pkg, &path, R, MIME)?;
    attach(pkg, &pathx, RX, MIMEX)?;
    Ok(result)
}
fn prune_modern_metadata(pkg: &mut Package, deleted: &HashSet<String>) -> Result<(), String> {
    if deleted.is_empty() {
        return Ok(());
    }
    const CID: &str = "http://schemas.microsoft.com/office/word/2016/wordml/cid";
    const CEX: &str = "http://schemas.microsoft.com/office/word/2018/wordml/cex";
    let mut durable = HashSet::new();
    if let Some(path) = part(
        pkg,
        "http://schemas.microsoft.com/office/2016/09/relationships/commentsIds",
    )? {
        let xml = pkg.text(&path)?.to_string();
        let dom = parse(&xml)?;
        let removed = dom
            .root_element()
            .children()
            .filter(|n| {
                n.has_tag_name((CID, "commentId"))
                    && n.attribute((CID, "paraId"))
                        .is_some_and(|p| deleted.contains(p))
            })
            .collect::<Vec<_>>();
        for n in &removed {
            if let Some(id) = n.attribute((CID, "durableId")) {
                durable.insert(id.to_string());
            }
        }
        if !removed.is_empty() {
            pkg.set(
                &path,
                replace_ranges(
                    &xml,
                    removed.iter().map(|n| (n.range(), String::new())).collect(),
                ),
            );
        }
    }
    if !durable.is_empty() {
        if let Some(path) = part(
            pkg,
            "http://schemas.microsoft.com/office/2018/08/relationships/commentsExtensible",
        )? {
            let xml = pkg.text(&path)?.to_string();
            let dom = parse(&xml)?;
            let removed = dom
                .root_element()
                .children()
                .filter(|n| {
                    n.has_tag_name((CEX, "commentExtensible"))
                        && n.attribute((CEX, "durableId"))
                            .is_some_and(|p| durable.contains(p))
                })
                .map(|n| (n.range(), String::new()))
                .collect::<Vec<_>>();
            if !removed.is_empty() {
                pkg.set(&path, replace_ranges(&xml, removed));
            }
        }
    }
    Ok(())
}
pub fn check(
    before: &str,
    after: &str,
    managed: &HashSet<String>,
    document: &Value,
) -> Result<(), String> {
    fn markers(xml: &str) -> Result<Vec<(String, String, Vec<(String, String)>)>, String> {
        Ok(parse(xml)?
            .descendants()
            .filter(|n| marker(*n))
            .map(|n| {
                (
                    n.attribute((W, "id")).unwrap_or("").to_string(),
                    n.tag_name().name().to_string(),
                    n.attributes()
                        .map(|a| (a.name().to_string(), a.value().to_string()))
                        .collect(),
                )
            })
            .collect())
    }
    let old = markers(before)?
        .into_iter()
        .filter(|r| !managed.contains(&r.0))
        .collect::<Vec<_>>();
    let unmanaged = old.iter().map(|r| r.0.clone()).collect::<HashSet<_>>();
    let new = markers(after)?;
    let mut retained = new
        .iter()
        .filter(|r| unmanaged.contains(&r.0))
        .cloned()
        .collect::<Vec<_>>();
    let mut old = old;
    old.sort();
    retained.sort();
    if old != retained {
        return Err("This edit changes a preserved Word comment anchor that Folio cannot yet update. Undo the edit or save a Folio workspace; the Word file is unchanged.".into());
    }
    fn events(n: &Value, out: &mut HashSet<u64>) {
        if let Some(e) = n["attrs"][EVENTS].as_array() {
            for e in e {
                out.insert(e["id"].as_u64().unwrap());
            }
        }
        if let Some(c) = n["content"].as_array() {
            for c in c {
                events(c, out)
            }
        }
    }
    let mut expected = HashSet::new();
    events(document, &mut expected);
    for id in expected {
        for tag in ["commentRangeStart", "commentRangeEnd", "commentReference"] {
            if new
                .iter()
                .filter(|r| r.0 == id.to_string() && r.1 == tag)
                .count()
                != 1
            {
                return Err(
                    "A comment anchor could not be written. The Word file is unchanged.".into(),
                );
            }
        }
    }
    Ok(())
}
