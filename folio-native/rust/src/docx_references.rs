//! Captions, cross-references and tables of contents (W10.03–W10.05,
//! W10.08). SEQ, REF and PAGEREF fields are `pageField` nodes whose cached
//! result Folio recalculates on Update Fields, as Word does; a table of
//! contents is a `tableOfContents` block of heading entries, written as
//! Word's TOC field so Word can update it too.
use crate::document_bookmarks::Index;
use crate::office::attr;
use roxmltree::Node;
use serde_json::{json, Value};
use std::collections::HashMap;

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// What Word shows for a reference to a bookmark that no longer exists.
pub const MISSING: &str = "Error! Reference source not found.";

/// Page labels found by laying the document out.
#[derive(Default)]
pub struct Pages {
    /// Top-level heading number (in reading order) -> page.
    pub headings: Vec<String>,
    /// Bookmark name (lower case) -> page.
    pub bookmarks: HashMap<String, String>,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn roman(mut n: u32, upper: bool) -> String {
    let table = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for (v, s) in table {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    if upper {
        out.to_uppercase()
    } else {
        out
    }
}

fn alphabetic(n: u32, upper: bool) -> String {
    // Word repeats the letter: 27 is "aa".
    let letter = (b'a' + ((n.max(1) - 1) % 26) as u8) as char;
    let s: String = std::iter::repeat_n(letter, ((n.max(1) - 1) / 26 + 1) as usize).collect();
    if upper {
        s.to_uppercase()
    } else {
        s
    }
}

/// A number in the field's `\* format`.
fn formatted(n: u32, code: &str) -> String {
    let tokens: Vec<&str> = code.split_whitespace().collect();
    let format = tokens
        .windows(2)
        .find(|w| w[0] == "\\*" && w[1] != "MERGEFORMAT" && w[1] != "CHARFORMAT")
        .map(|w| w[1]);
    match format {
        Some("ROMAN") => roman(n, true),
        Some("roman") => roman(n, false),
        Some("ALPHABETIC") => alphabetic(n, true),
        Some("alphabetic") => alphabetic(n, false),
        _ => n.to_string(),
    }
}

fn switch(code: &str, name: &str) -> bool {
    code.split_whitespace().any(|t| t == name)
}

fn switch_value(code: &str, name: &str) -> Option<u32> {
    let tokens: Vec<&str> = code.split_whitespace().collect();
    tokens
        .windows(2)
        .find(|w| w[0] == name)
        .and_then(|w| w[1].parse().ok())
}

/// Sets a field's cached result, dropping the imported source (which holds
/// the old result).
fn set_result(field: &mut Value, text: &str) -> bool {
    let text: String = text
        .chars()
        .map(|c| {
            if matches!(c, '\n' | '\r' | '\t') {
                ' '
            } else {
                c
            }
        })
        .take(1000)
        .collect();
    let text = text.trim().to_string();
    if field["text"] == text.as_str() {
        return false;
    }
    field["text"] = json!(text);
    if let Some(attrs) = field["attrs"].as_object_mut() {
        attrs.remove("sourceXml");
        attrs.remove("sourceMarks");
        attrs.insert("dirty".into(), json!(false));
    }
    true
}

/// Walks the document's fields and headings in reading order.
fn walk(v: &mut Value, f: &mut impl FnMut(&mut Value)) {
    f(v);
    if let Some(children) = v.get_mut("content").and_then(Value::as_array_mut) {
        for c in children {
            walk(c, f);
        }
    }
}

fn instruction(n: &Value) -> Option<(String, String)> {
    if n["type"] != "pageField" || n["attrs"]["locked"] == true {
        return None;
    }
    let code = n["attrs"]["instruction"].as_str()?.to_string();
    let kind = n["attrs"]["kind"].as_str()?.to_string();
    Some((kind, code))
}

/// The text a bookmark covers.
fn bookmark_text(index: &Index, document: &Value, name: &str) -> Option<String> {
    let entry = crate::document_bookmarks::entries(document)
        .iter()
        .find(|b| {
            b["name"]
                .as_str()
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })?;
    let start = entry["start"].as_u64()? as usize;
    let end = entry["end"].as_u64()? as usize;
    let units: Vec<u16> = index.text.encode_utf16().collect();
    Some(String::from_utf16_lossy(
        units.get(start..end.min(units.len()))?,
    ))
}

/// Recalculates SEQ, REF and PAGEREF results and tables of contents.
/// Returns whether anything changed.
pub fn update(document: &mut Value, pages: &Pages) -> bool {
    let mut changed = false;
    // SEQ: count per identifier in reading order; `\s n` restarts after a
    // heading of level n or above, `\r n` resets, `\c` repeats.
    let mut counters: HashMap<String, u32> = HashMap::new();
    let mut resets: HashMap<String, u64> = HashMap::new();
    walk(document, &mut |n| {
        if n["type"] == "heading" {
            let level = n["attrs"]["level"].as_u64().unwrap_or(1);
            for (id, at) in &resets {
                if level <= *at {
                    counters.insert(id.clone(), 0);
                }
            }
            return;
        }
        let Some((kind, code)) = instruction(n) else {
            return;
        };
        if kind != "SEQ" {
            return;
        }
        let Some(id) = crate::docx_fields::target(&code).map(str::to_lowercase) else {
            return;
        };
        if let Some(level) = switch_value(&code, "\\s") {
            resets.insert(id.clone(), level as u64);
        }
        let count = counters.entry(id).or_insert(0);
        if let Some(value) = switch_value(&code, "\\r") {
            *count = value;
        } else if !switch(&code, "\\c") {
            *count += 1;
        }
        let text = if switch(&code, "\\h") {
            String::new()
        } else {
            formatted(*count, &code)
        };
        changed |= set_result(n, &text);
    });
    // REF and PAGEREF read the numbered captions, so they come after SEQ.
    let index = Index::new(document);
    let snapshot = document.clone();
    walk(document, &mut |n| {
        let Some((kind, code)) = instruction(n) else {
            return;
        };
        let Some(target) = crate::docx_fields::target(&code) else {
            return;
        };
        match kind.as_str() {
            "REF" => {
                let text =
                    bookmark_text(&index, &snapshot, target).unwrap_or_else(|| MISSING.into());
                changed |= set_result(n, &text);
            }
            "PAGEREF" => {
                let exists = bookmark_text(&index, &snapshot, target).is_some();
                if !exists {
                    changed |= set_result(n, MISSING);
                } else if let Some(page) = pages.bookmarks.get(&target.to_lowercase()) {
                    changed |= set_result(n, page);
                }
            }
            _ => {}
        }
    });
    // Tables of contents list the top-level headings up to their level.
    let headings: Vec<(u64, String)> = document["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| n["type"] == "heading")
        .map(|n| {
            (
                n["attrs"]["level"].as_u64().unwrap_or(1),
                crate::workspace::document_text(n)
                    .replace(['\n', '\t'], " ")
                    .trim()
                    .to_string(),
            )
        })
        .collect();
    if let Some(blocks) = document["content"].as_array_mut() {
        for block in blocks.iter_mut().filter(|b| b["type"] == "tableOfContents") {
            let levels = block["attrs"]["levels"].as_u64().unwrap_or(3).clamp(1, 9);
            let old = block["attrs"]["entries"].clone();
            let entries: Vec<Value> = headings
                .iter()
                .enumerate()
                .filter(|(_, (level, text))| *level <= levels && !text.is_empty())
                .map(|(i, (level, text))| {
                    // Keep a known page when layout didn't run.
                    let page = pages
                        .headings
                        .get(i)
                        .cloned()
                        .filter(|p| !p.is_empty())
                        .or_else(|| {
                            old.as_array().and_then(|o| {
                                o.iter()
                                    .find(|e| e["text"] == text.as_str() && e["level"] == *level)
                                    .and_then(|e| e["page"].as_str().map(String::from))
                            })
                        })
                        .unwrap_or_default();
                    json!({"text": text.chars().take(1000).collect::<String>(), "level": level, "page": page})
                })
                .collect();
            if old != json!(entries) {
                block["attrs"]["entries"] = json!(entries);
                changed = true;
            }
        }
    }
    changed
}

/// Page labels for `document` from the laid-out preview document: headings
/// in reading order, and bookmarks by name (export keeps their names).
pub fn pages(preview: &Value, block_pages: &[String]) -> Pages {
    let mut pages = Pages::default();
    for (i, block) in preview["content"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        if block["type"] == "heading" {
            pages
                .headings
                .push(block_pages.get(i).cloned().unwrap_or_default());
        }
    }
    let index = Index::new(preview);
    for b in crate::document_bookmarks::entries(preview) {
        let (Some(name), Some(start)) = (b["name"].as_str(), b["start"].as_u64()) else {
            continue;
        };
        if let Some(top) = index.paragraph(start as usize).and_then(|p| p.path.first()) {
            if let Some(page) = block_pages.get(*top).filter(|p| !p.is_empty()) {
                pages.bookmarks.insert(name.to_lowercase(), page.clone());
            }
        }
    }
    pages
}

pub fn validate(n: &Value) -> Result<(), String> {
    if n["type"] != "tableOfContents" {
        return Ok(());
    }
    let a = &n["attrs"];
    if !a["levels"].as_u64().is_some_and(|l| (1..=9).contains(&l)) {
        return Err("Invalid table of contents levels".into());
    }
    let entries = a["entries"].as_array().ok_or("Invalid table of contents")?;
    if entries.len() > 5000 {
        return Err("Table of contents is too long".into());
    }
    for e in entries {
        let text = e["text"]
            .as_str()
            .ok_or("Invalid table of contents entry")?;
        let page = e["page"]
            .as_str()
            .ok_or("Invalid table of contents entry")?;
        if text.chars().count() > 1000
            || page.chars().count() > 20
            || !e["level"].as_u64().is_some_and(|l| (1..=9).contains(&l))
        {
            return Err("Invalid table of contents entry".into());
        }
    }
    if n.get("content")
        .is_some_and(|c| c.as_array().is_none_or(|c| !c.is_empty()))
    {
        return Err("Invalid table of contents".into());
    }
    Ok(())
}

/// A table of contents as Word's TOC field in a content control.
pub fn write(n: &Value) -> String {
    let levels = n["attrs"]["levels"].as_u64().unwrap_or(3).clamp(1, 9);
    let entries = n["attrs"]["entries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let begin = format!(
        "<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> TOC \\o \"1-{levels}\" \\u </w:instrText></w:r><w:r><w:fldChar w:fldCharType=\"separate\"/></w:r>"
    );
    let mut body = String::new();
    if entries.is_empty() {
        body.push_str(&format!(
            "<w:p>{begin}<w:r><w:t>No table of contents entries found.</w:t></w:r></w:p>"
        ));
    }
    for (i, e) in entries.iter().enumerate() {
        let level = e["level"].as_u64().unwrap_or(1);
        body.push_str(&format!(
            "<w:p><w:pPr><w:pStyle w:val=\"TOC{level}\"/><w:tabs><w:tab w:val=\"right\" w:leader=\"dot\" w:pos=\"9350\"/></w:tabs></w:pPr>{}<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>{}</w:t></w:r></w:p>",
            if i == 0 { begin.as_str() } else { "" },
            esc(e["text"].as_str().unwrap_or("")),
            esc(e["page"].as_str().unwrap_or(""))
        ));
    }
    body.push_str("<w:p><w:r><w:fldChar w:fldCharType=\"end\"/></w:r></w:p>");
    format!(
        "<w:sdt><w:sdtPr><w:docPartObj><w:docPartGallery w:val=\"Table of Contents\"/><w:docPartUnique/></w:docPartObj></w:sdtPr><w:sdtContent>{body}</w:sdtContent></w:sdt>"
    )
}

/// A Word table of contents (a content control holding a TOC field) as a
/// `tableOfContents` block, or None for any other content control.
pub fn read(sdt: Node, id: &str) -> Option<Value> {
    let gallery = sdt
        .descendants()
        .find(|n| n.has_tag_name((W, "docPartGallery")))
        .and_then(|n| attr(n, "val"))?;
    if gallery != "Table of Contents" {
        return None;
    }
    let content = sdt.children().find(|n| n.has_tag_name((W, "sdtContent")))?;
    let code: String = content
        .descendants()
        .filter(|n| n.has_tag_name((W, "instrText")))
        .filter_map(|n| n.text())
        .collect();
    if !code.trim_start().starts_with("TOC") {
        return None;
    }
    let levels = code
        .split("\\o")
        .nth(1)
        .and_then(|s| s.trim().trim_start_matches('"').split('"').next())
        .and_then(|range| range.split('-').nth(1))
        .and_then(|l| l.trim().parse::<u64>().ok())
        .unwrap_or(3)
        .clamp(1, 9);
    let mut entries = vec![];
    for p in content.children().filter(|n| n.has_tag_name((W, "p"))) {
        let style = p
            .children()
            .find(|n| n.has_tag_name((W, "pPr")))
            .and_then(|pr| pr.children().find(|n| n.has_tag_name((W, "pStyle"))))
            .and_then(|s| attr(s, "val"))
            .unwrap_or("");
        let Some(level) = style
            .to_ascii_lowercase()
            .trim_start_matches("toc")
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|l| (1..=9).contains(l))
        else {
            continue;
        };
        // The entry's text, then the page after its last tab. Field
        // instructions (PAGEREF) are not text.
        let mut parts = vec![String::new()];
        for n in p.descendants() {
            if n.has_tag_name((W, "t")) {
                parts.last_mut().unwrap().push_str(n.text().unwrap_or(""));
            } else if n.has_tag_name((W, "tab"))
                && n.parent().is_some_and(|r| r.has_tag_name((W, "r")))
            {
                parts.push(String::new());
            }
        }
        let page = if parts.len() > 1 {
            parts.pop().unwrap_or_default()
        } else {
            String::new()
        };
        let text = parts.join(" ").trim().to_string();
        if !text.is_empty() {
            entries.push(json!({"text": text.chars().take(1000).collect::<String>(), "level": level, "page": page.trim().chars().take(20).collect::<String>()}));
        }
    }
    Some(
        json!({"type":"tableOfContents","attrs":{"officeId":id,"levels":levels,"entries":entries}}),
    )
}

// ── Inserting captions, cross-references and tables of contents ────────────

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Rebases bookmarks and comments from `previous` onto `next` by their text.
fn rebased(previous: &Value, mut next: Value) -> Result<Value, String> {
    crate::document_bookmarks::rebase(previous, &mut next, None)?;
    crate::document_comments::rebase(previous, &mut next, None)?;
    Ok(next)
}

/// Inserts an inline node at a body-story offset, splitting a text run.
fn insert_inline(document: &mut Value, offset: usize, node: Value) -> Result<(), String> {
    let index = Index::new(document);
    let p = index
        .paragraph(offset)
        .ok_or("Place the cursor in a paragraph first.")?;
    let (path, at) = (p.path.clone(), offset - p.start);
    let paragraph = crate::document_bookmarks::at_mut(document, &path);
    if !paragraph["content"].is_array() {
        paragraph["content"] = json!([]);
    }
    let content = paragraph["content"].as_array_mut().unwrap();
    let mut position = 0;
    for i in 0..content.len() {
        let len = utf16_len(&crate::workspace::document_text(&content[i]));
        if at == position {
            content.insert(i, node);
            return Ok(());
        }
        if at < position + len && content[i]["type"] == "text" {
            let units: Vec<u16> = content[i]["text"]
                .as_str()
                .unwrap_or("")
                .encode_utf16()
                .collect();
            let split = at - position;
            let mut right = content[i].clone();
            content[i]["text"] = json!(String::from_utf16_lossy(&units[..split]));
            right["text"] = json!(String::from_utf16_lossy(&units[split..]));
            content.insert(i + 1, node);
            content.insert(i + 2, right);
            return Ok(());
        }
        position += len;
    }
    content.push(node);
    Ok(())
}

fn seq_field(label: &str) -> Value {
    json!({"type":"pageField","text":"?","attrs":{"kind":"SEQ",
        "instruction":format!(" SEQ {label} \\* ARABIC "),"locked":false,"dirty":true}})
}

fn is_label(label: &str) -> bool {
    !label.is_empty() && label.len() <= 40 && label.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// A caption paragraph (`Figure 2: text`, in the Caption style) after the
/// paragraph at `offset`, numbered with the others.
pub fn insert_caption(
    previous: &Value,
    offset: usize,
    label: &str,
    text: &str,
) -> Result<Value, String> {
    if !is_label(label) {
        return Err("A caption label is one word of letters and numbers.".into());
    }
    if text.chars().count() > 1000 || text.contains(['\n', '\r']) {
        return Err("Keep the caption on one line.".into());
    }
    let mut next = previous.clone();
    let index = Index::new(&next);
    let path = index
        .paragraph(offset)
        .map(|p| p.path.clone())
        .ok_or("Place the cursor in a paragraph first.")?;
    let mut content = vec![
        json!({"type":"text","text":format!("{label} ")}),
        seq_field(label),
    ];
    if !text.trim().is_empty() {
        content.push(json!({"type":"text","text":format!(": {}", text.trim())}));
    }
    let caption = json!({"type":"paragraph","attrs":{"paragraphFormat":{"inherited":{},"direct":{},"styleId":"Caption"}},"content":content});
    let (parent, last) = path.split_at(path.len() - 1);
    crate::document_bookmarks::at_mut(&mut next, parent)["content"]
        .as_array_mut()
        .ok_or("Invalid document")?
        .insert(last[0] + 1, caption);
    let next = rebased(previous, next)?;
    let mut numbered = next.clone();
    update(&mut numbered, &Pages::default());
    rebased(&next, numbered)
}

/// A table of contents before the top-level block at `offset`.
pub fn insert_toc(previous: &Value, offset: usize, levels: u64) -> Result<Value, String> {
    let mut next = previous.clone();
    let index = Index::new(&next);
    let top = index
        .paragraph(offset)
        .and_then(|p| p.path.first().copied())
        .unwrap_or(0);
    next["content"]
        .as_array_mut()
        .ok_or("Invalid document")?
        .insert(
            top,
            json!({"type":"tableOfContents","attrs":{"levels":levels.clamp(1, 9),"entries":[]}}),
        );
    let next = rebased(previous, next)?;
    let mut filled = next.clone();
    update(&mut filled, &Pages::default());
    rebased(&next, filled)
}

/// What a cross-reference can point at: top-level headings and captions,
/// and visible bookmarks.
pub fn targets(document: &Value) -> Value {
    let mut headings = vec![];
    let mut captions = vec![];
    for (i, n) in document["content"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let text = crate::workspace::document_text(n).replace(['\n', '\t'], " ");
        if n["type"] == "heading" && !text.trim().is_empty() {
            headings.push(json!({"index":i,"level":n["attrs"]["level"],"text":text.trim()}));
        } else if n["type"] == "paragraph"
            && n["content"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|c| c["type"] == "pageField" && c["attrs"]["kind"] == "SEQ")
        {
            captions.push(json!({"index":i,"text":text.trim()}));
        }
    }
    let bookmarks: Vec<Value> = crate::document_bookmarks::entries(document)
        .iter()
        .filter_map(|b| b["name"].as_str())
        .filter(|n| !n.starts_with('_'))
        .map(|n| json!(n))
        .collect();
    json!({"headings":headings,"captions":captions,"bookmarks":bookmarks})
}

/// The story range a heading or caption target covers: the whole heading,
/// or a caption's label and number.
fn target_range(document: &Value, kind: &str, block: usize) -> Option<(usize, usize)> {
    let index = Index::new(document);
    let p = index.paragraphs.iter().find(|p| p.path == [block])?;
    if kind == "heading" {
        return Some((p.start, p.end));
    }
    let mut end = p.start;
    for c in document["content"][block]["content"].as_array()? {
        end += utf16_len(&crate::workspace::document_text(c));
        if c["type"] == "pageField" && c["attrs"]["kind"] == "SEQ" {
            return Some((p.start, end));
        }
    }
    None
}

/// A REF or PAGEREF field at `offset` pointing at `target`
/// (`{"kind":"heading"|"caption", "index"}` or `{"kind":"bookmark","name"}`),
/// adding Word's hidden `_Ref` bookmark when the target has none.
pub fn insert_cross_reference(
    previous: &Value,
    offset: usize,
    target: &Value,
    page: bool,
) -> Result<Value, String> {
    let kind = target["kind"].as_str().unwrap_or("");
    let name = match kind {
        "bookmark" => {
            let name = target["name"].as_str().ok_or("Choose a bookmark")?;
            if !crate::document_bookmarks::entries(previous)
                .iter()
                .any(|b| {
                    b["name"]
                        .as_str()
                        .is_some_and(|n| n.eq_ignore_ascii_case(name))
                })
            {
                return Err("That bookmark no longer exists.".into());
            }
            name.to_string()
        }
        "heading" | "caption" => {
            let block = target["index"]
                .as_u64()
                .ok_or("Choose a heading or caption")? as usize;
            let (start, end) = target_range(previous, kind, block)
                .ok_or("That heading or caption no longer exists.")?;
            let existing = crate::document_bookmarks::entries(previous)
                .iter()
                .find_map(|b| {
                    (b["start"] == start && b["end"] == end)
                        .then(|| b["name"].as_str().map(String::from))
                        .flatten()
                });
            existing.unwrap_or_default()
        }
        _ => return Err("Choose what to refer to.".into()),
    };
    let mut start_doc = previous.clone();
    // A heading or caption without a bookmark gets one, as Word does.
    let name = if name.is_empty() {
        let used: Vec<String> = crate::document_bookmarks::entries(previous)
            .iter()
            .filter_map(|b| b["name"].as_str().map(str::to_lowercase))
            .collect();
        let mut n = 100_000_001u64;
        while used.contains(&format!("_ref{n}")) {
            n += 1;
        }
        let name = format!("_Ref{n}");
        let block = target["index"].as_u64().unwrap_or(0) as usize;
        let (start, end) = target_range(previous, kind, block).ok_or("Invalid target")?;
        let mut list = crate::document_bookmarks::entries(previous).to_vec();
        list.push(json!({"name":name,"start":start,"end":end}));
        start_doc["attrs"]["bookmarks"] = json!(list);
        crate::document_bookmarks::validate(&start_doc)?;
        name
    } else {
        name
    };
    let code = if page {
        format!(" PAGEREF {name} \\h ")
    } else {
        format!(" REF {name} \\h ")
    };
    let field = json!({"type":"pageField","text":"?","attrs":{
        "kind":if page {"PAGEREF"} else {"REF"},"instruction":code,"locked":false,"dirty":true}});
    let mut next = start_doc.clone();
    insert_inline(&mut next, offset, field)?;
    let next = rebased(&start_doc, next)?;
    let mut resolved = next.clone();
    update(&mut resolved, &Pages::default());
    rebased(&next, resolved)
}
