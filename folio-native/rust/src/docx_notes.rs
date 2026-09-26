//! Footnotes and endnotes (W11.01). A reference is an inline `noteRef` node,
//! `{"kind": "footnote"|"endnote", "id", "text", "sourceXml"}`, whose text is
//! the note's paragraphs joined by line breaks. Unchanged notes keep their
//! XML; edited ones are rewritten as plain paragraphs; new ones are added to
//! `word/footnotes.xml` or `word/endnotes.xml`, created when missing.
use crate::office::{attr, parse, replace_ranges, Package};
use roxmltree::Node;
use serde_json::{json, Value};
use std::collections::HashMap;

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const REL_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// Longest note text accepted from the editor.
const MAX_TEXT: usize = 65536;

struct Kind {
    name: &'static str,
    part: &'static str,
    root: &'static str,
    note: &'static str,
    reference: &'static str,
    mark: &'static str,
    style: &'static str,
    text_style: &'static str,
    content_type: &'static str,
    rel_type: &'static str,
}

const FOOTNOTE: Kind = Kind {
    name: "footnote",
    part: "word/footnotes.xml",
    root: "footnotes",
    note: "footnote",
    reference: "footnoteReference",
    mark: "footnoteRef",
    style: "FootnoteReference",
    text_style: "FootnoteText",
    content_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml",
    rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes",
};
const ENDNOTE: Kind = Kind {
    name: "endnote",
    part: "word/endnotes.xml",
    root: "endnotes",
    note: "endnote",
    reference: "endnoteReference",
    mark: "endnoteRef",
    style: "EndnoteReference",
    text_style: "EndnoteText",
    content_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml",
    rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes",
};

fn kind(name: &str) -> Option<&'static Kind> {
    match name {
        "footnote" => Some(&FOOTNOTE),
        "endnote" => Some(&ENDNOTE),
        _ => None,
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A note's text: its paragraphs, without the leading space Word puts after
/// the note mark.
fn note_text(note: Node) -> String {
    let paragraphs: Vec<String> = note
        .children()
        .filter(|n| n.has_tag_name((W, "p")))
        .map(|p| {
            p.descendants()
                .filter_map(|n| match n.tag_name().name() {
                    "t" if n.tag_name().namespace() == Some(W) => {
                        Some(n.text().unwrap_or("").to_string())
                    }
                    "tab" if n.tag_name().namespace() == Some(W) => Some("\t".to_string()),
                    _ => None,
                })
                .collect()
        })
        .collect();
    let mut text = paragraphs.join("\n");
    if text.starts_with(' ') {
        text.remove(0);
    }
    text
}

/// The inline node for a `w:footnoteReference` or `w:endnoteReference` in
/// `run`. The run's XML is kept when it holds nothing else to show.
pub fn read(run: Node, reference: Node) -> Option<Value> {
    let kind = match reference.tag_name().name() {
        "footnoteReference" => "footnote",
        "endnoteReference" => "endnote",
        _ => return None,
    };
    let id = attr(reference, "id")?;
    let mut node = json!({"type":"noteRef","attrs":{"kind":kind,"id":id,"text":""}});
    let alone = run
        .children()
        .filter(|n| n.is_element())
        .all(|n| n.has_tag_name((W, "rPr")) || n == reference);
    if alone {
        node["attrs"]["sourceXml"] = json!(&run.document().input_text()[run.range()]);
    }
    Some(node)
}

/// The notes' texts by id, for each kind.
fn texts(pkg: &Package, k: &Kind) -> HashMap<String, String> {
    let Ok(xml) = pkg.text(k.part) else {
        return HashMap::new();
    };
    let Ok(d) = parse(xml) else {
        return HashMap::new();
    };
    d.root_element()
        .children()
        .filter(|n| n.has_tag_name((W, k.note)))
        .filter_map(|n| Some((attr(n, "id")?.to_string(), note_text(n))))
        .collect()
}

fn walk(v: &mut Value, f: &mut impl FnMut(&mut Value)) {
    if v["type"] == "noteRef" {
        f(v);
    }
    if let Some(children) = v.get_mut("content").and_then(Value::as_array_mut) {
        for c in children {
            walk(c, f);
        }
    }
}

/// Fills each imported reference with its note's text.
pub fn import(document: &mut Value, pkg: &Package) {
    let foot = texts(pkg, &FOOTNOTE);
    let end = texts(pkg, &ENDNOTE);
    walk(document, &mut |n| {
        let id = n["attrs"]["id"].as_str().unwrap_or("").to_string();
        let texts = if n["attrs"]["kind"] == "endnote" {
            &end
        } else {
            &foot
        };
        if let Some(text) = texts.get(&id) {
            n["attrs"]["text"] = json!(text);
        }
    });
}

/// Checks a `noteRef` node from the editor. A kept source run must be a
/// single reference of the same kind and id.
pub fn validate(n: &Value) -> Result<(), String> {
    if n["type"] != "noteRef" {
        return Ok(());
    }
    let a = &n["attrs"];
    let k = kind(a["kind"].as_str().unwrap_or("")).ok_or("Invalid note kind")?;
    if !a["text"].is_string() || a["text"].as_str().unwrap().len() > MAX_TEXT {
        return Err("Invalid note text".into());
    }
    let id = match &a["id"] {
        Value::Null => None,
        Value::String(id)
            if !id.is_empty() && id.len() < 12 && id.chars().all(|c| c.is_ascii_digit()) =>
        {
            Some(id.as_str())
        }
        _ => return Err("Invalid note id".into()),
    };
    if let Some(source) = a.get("sourceXml").filter(|s| !s.is_null()) {
        let source = source.as_str().ok_or("Invalid note source")?;
        let id = id.ok_or("A note source needs its id")?;
        let wrapped = format!("<w:p xmlns:w=\"{W}\">{source}</w:p>");
        let d = parse(&wrapped)?;
        let runs: Vec<_> = d
            .root_element()
            .children()
            .filter(|c| c.is_element())
            .collect();
        let ok = runs.len() == 1
            && runs[0].has_tag_name((W, "r"))
            && runs[0]
                .children()
                .filter(|c| c.is_element() && !c.has_tag_name((W, "rPr")))
                .map(|c| c.has_tag_name((W, k.reference)) && attr(c, "id") == Some(id))
                .collect::<Vec<_>>()
                == [true];
        if !ok {
            return Err("Note source does not match its reference".into());
        }
    }
    Ok(())
}

/// The run for a reference: its source run, or a new superscript one.
pub fn write(n: &Value, marks: &str) -> String {
    let a = &n["attrs"];
    if let Some(source) = a["sourceXml"].as_str() {
        return source.to_string();
    }
    let k = kind(a["kind"].as_str().unwrap_or("")).unwrap_or(&FOOTNOTE);
    format!(
        "<w:r><w:rPr><w:rStyle w:val=\"{}\"/>{marks}<w:vertAlign w:val=\"superscript\"/></w:rPr><w:{} w:id=\"{}\"/></w:r>",
        k.style,
        k.reference,
        esc(a["id"].as_str().unwrap_or("0"))
    )
}

fn note_xml(k: &Kind, id: &str, text: &str, first_ppr: Option<&str>) -> String {
    let ppr = first_ppr
        .map(str::to_string)
        .unwrap_or_else(|| format!("<w:pPr><w:pStyle w:val=\"{}\"/></w:pPr>", k.text_style));
    let paragraphs = text
        .split('\n')
        .enumerate()
        .map(|(i, line)| {
            let mark = if i == 0 {
                format!(
                    "<w:r><w:rPr><w:rStyle w:val=\"{}\"/><w:vertAlign w:val=\"superscript\"/></w:rPr><w:{}/></w:r><w:r><w:t xml:space=\"preserve\"> </w:t></w:r>",
                    k.style, k.mark
                )
            } else {
                String::new()
            };
            let runs = line
                .split('\t')
                .enumerate()
                .map(|(j, part)| {
                    format!(
                        "{}{}",
                        if j > 0 { "<w:r><w:tab/></w:r>" } else { "" },
                        if part.is_empty() {
                            String::new()
                        } else {
                            format!("<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r>", esc(part))
                        }
                    )
                })
                .collect::<String>();
            format!("<w:p>{}{mark}{runs}</w:p>", if i == 0 { ppr.as_str() } else { "" })
        })
        .collect::<String>();
    format!(
        "<w:{} w:id=\"{}\">{paragraphs}</w:{}>",
        k.note,
        esc(id),
        k.note
    )
}

/// The notes part for `k`, created with Word's separators when missing,
/// with its relationship and content type.
fn ensure_part(pkg: &mut Package, k: &Kind) -> Result<(), String> {
    if pkg.entries.contains_key(k.part) {
        return Ok(());
    }
    let separators = format!(
        "<w:{n} w:type=\"separator\" w:id=\"-1\"><w:p><w:pPr><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr><w:r><w:separator/></w:r></w:p></w:{n}><w:{n} w:type=\"continuationSeparator\" w:id=\"0\"><w:p><w:pPr><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr><w:r><w:continuationSeparator/></w:r></w:p></w:{n}>",
        n = k.note
    );
    pkg.set(
        k.part,
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:{} xmlns:w=\"{W}\" xmlns:r=\"{R}\">{separators}</w:{}>",
            k.root, k.root
        ),
    );
    let rels_name = "word/_rels/document.xml.rels";
    let rels = pkg
        .text(rels_name)
        .map(String::from)
        .unwrap_or_else(|_| format!("<Relationships xmlns=\"{REL_NS}\"></Relationships>"));
    let target = k.part.trim_start_matches("word/");
    if !rels.contains(&format!("Target=\"{target}\"")) {
        let end = rels
            .rfind("</Relationships>")
            .ok_or("Invalid relationships")?;
        let mut n = 1;
        while rels.contains(&format!("Id=\"folioNotes{n}\"")) {
            n += 1;
        }
        pkg.set(
            rels_name,
            format!(
                "{}<Relationship Id=\"folioNotes{n}\" Type=\"{}\" Target=\"{target}\"/>{}",
                &rels[..end],
                k.rel_type,
                &rels[end..]
            ),
        );
    }
    let types = pkg.text("[Content_Types].xml")?.to_string();
    let part_name = format!("/{}", k.part);
    if !types.contains(&format!("PartName=\"{part_name}\"")) {
        let end = types.rfind("</Types>").ok_or("Invalid content types")?;
        pkg.set(
            "[Content_Types].xml",
            format!(
                "{}<Override PartName=\"{part_name}\" ContentType=\"{}\"/>{}",
                &types[..end],
                k.content_type,
                &types[end..]
            ),
        );
    }
    Ok(())
}

/// Before export: gives new references an id and writes new and edited
/// notes into the package. Returns the document with those ids.
pub fn prepare(document: &Value, pkg: &mut Package) -> Result<Value, String> {
    let mut document = document.clone();
    for k in [&FOOTNOTE, &ENDNOTE] {
        let mut refs: Vec<(Option<String>, String)> = vec![];
        walk(&mut document, &mut |n| {
            if n["attrs"]["kind"] == k.name {
                refs.push((
                    n["attrs"]["id"].as_str().map(String::from),
                    n["attrs"]["text"].as_str().unwrap_or("").to_string(),
                ));
            }
        });
        let existing = texts(pkg, k);
        let changed: Vec<&(Option<String>, String)> = refs
            .iter()
            .filter(|(id, text)| match id {
                None => true,
                Some(id) => {
                    existing.get(id).is_some_and(|t| t != text) || !existing.contains_key(id)
                }
            })
            .collect();
        if changed.is_empty() {
            continue;
        }
        ensure_part(pkg, k)?;
        let xml = pkg.text(k.part)?.to_string();
        let d = parse(&xml)?;
        let root = d.root_element();
        let mut next_id = root
            .children()
            .filter(|n| n.has_tag_name((W, k.note)))
            .filter_map(|n| attr(n, "id")?.parse::<i64>().ok())
            .max()
            .unwrap_or(0)
            .max(0)
            + 1;
        let mut edits = vec![];
        let mut added = String::new();
        let mut assigned: Vec<String> = vec![];
        for (id, text) in &refs {
            match id {
                Some(id) => {
                    let Some(note) = root.children().find(|n| {
                        n.has_tag_name((W, k.note)) && attr(*n, "id") == Some(id.as_str())
                    }) else {
                        added.push_str(&note_xml(k, id, text, None));
                        continue;
                    };
                    if existing.get(id) == Some(text) {
                        continue;
                    }
                    // Keep the first paragraph's properties.
                    let ppr = note
                        .children()
                        .find(|n| n.has_tag_name((W, "p")))
                        .and_then(|p| p.children().find(|c| c.has_tag_name((W, "pPr"))))
                        .map(|p| &xml[p.range()]);
                    edits.push((note.range(), note_xml(k, id, text, ppr)));
                }
                None => {
                    let id = next_id.to_string();
                    next_id += 1;
                    added.push_str(&note_xml(k, &id, text, None));
                    assigned.push(id);
                }
            }
        }
        let mut xml = replace_ranges(&xml, edits);
        if !added.is_empty() {
            let end = xml
                .rfind(&format!("</w:{}>", k.root))
                .ok_or("Invalid notes part")?;
            xml.insert_str(end, &added);
        }
        pkg.set(k.part, xml);
        let mut assigned = assigned.into_iter();
        walk(&mut document, &mut |n| {
            if n["attrs"]["kind"] == k.name && n["attrs"]["id"].is_null() {
                if let Some(id) = assigned.next() {
                    n["attrs"]["id"] = json!(id);
                }
            }
        });
    }
    Ok(document)
}
