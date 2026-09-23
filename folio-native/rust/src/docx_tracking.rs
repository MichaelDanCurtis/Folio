//! Package/editor bridge for authored revisions. Source paragraphs are aligned
//! by exact logical edits and container boundaries, never by editor officeIds
//! (Enter can duplicate those) or by searching for repeated text.
use crate::{document_bookmarks::Index, docx, docx_track_edit as track, office::*};
use roxmltree::Node;
use serde_json::{json, Value};
use std::collections::HashMap;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

struct Region<'a, 'i> {
    nodes: Vec<Node<'a, 'i>>,
    paragraphs: Option<Vec<String>>,
}
fn regions<'a, 'i>(xml: &str, parent: Node<'a, 'i>) -> Vec<Region<'a, 'i>> {
    let mut out: Vec<Region> = vec![];
    for n in parent
        .children()
        .filter(|n| n.is_element() && !matches!(n.tag_name().name(), "sectPr" | "tcPr"))
    {
        let p = scoped_record(xml, n);
        if n.has_tag_name((W, "p")) && track::supports_paragraph(&p) {
            if let Some(last) = out.last_mut().filter(|r| r.paragraphs.is_some()) {
                last.nodes.push(n);
                last.paragraphs.as_mut().unwrap().push(p);
            } else {
                out.push(Region {
                    nodes: vec![n],
                    paragraphs: Some(vec![p]),
                });
            }
        } else {
            out.push(Region {
                nodes: vec![n],
                paragraphs: None,
            });
        }
    }
    out
}
fn project_container(xml: &str, parent: Node) -> Result<String, String> {
    let mut patches = vec![];
    for r in regions(xml, parent) {
        if let Some(p) = r.paragraphs {
            // A paragraph marker cannot join across an object boundary. Such
            // source content remains preserved until that case is supported.
            if let Ok(current) = track::current_story_view(&p) {
                patches.push((
                    r.nodes[0].range().start..r.nodes.last().unwrap().range().end,
                    current.concat(),
                ));
            }
        } else if r.nodes[0].has_tag_name((W, "tbl")) {
            let n = r.nodes[0];
            if let Some(current) = project_table(xml, n)? {
                patches.push((n.range(), current));
            }
        }
    }
    let scoped = scoped_record(xml, parent);
    // Apply in the original namespace context before extracting the container.
    let start = parent.range().start;
    let raw_parent = raw(xml, parent);
    let patched = replace_ranges(
        raw_parent,
        patches
            .into_iter()
            .map(|(r, v)| ((r.start - start)..(r.end - start), v))
            .collect(),
    );
    let d = parse(&scoped)?;
    let open = opening_tag_end(&patched).ok_or("Invalid Word container")? + 1;
    let close = patched.rfind("</").unwrap_or(open);
    Ok(replace_inner(
        &scoped,
        d.root_element(),
        &patched[open..close],
    ))
}
fn project_table(xml: &str, table: Node) -> Result<Option<String>, String> {
    let mut patches = vec![];
    for cell in table
        .children()
        .filter(|n| n.has_tag_name((W, "tr")))
        .flat_map(|r| r.children().filter(|n| n.has_tag_name((W, "tc"))))
    {
        patches.push((cell.range(), project_container(xml, cell)?));
    }
    let mut result = replace_ranges(xml, patches);
    // The table starts at the same byte offset after its descendants are edited.
    let d = parse(&result)?;
    let n = d
        .descendants()
        .find(|n| n.range().start == table.range().start && n.has_tag_name((W, "tbl")))
        .ok_or("Missing current table")?;
    if crate::docx_revisions::has_changes(n) {
        return Ok(None);
    }
    result = scoped_record(&result, n);
    Ok(Some(result))
}
pub fn import_current(pkg: &Package) -> Result<(Value, Value), String> {
    let xml = pkg.text("word/document.xml")?;
    let d = parse(xml)?;
    let node = d
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .ok_or("Word body missing")?;
    let current = project_container(xml, node)?;
    let mut copy = pkg.clone();
    copy.set(
        "word/document.xml",
        replace_ranges(xml, vec![(node.range(), current)]),
    );
    docx::import(&copy)
}
fn plain(value: &Value) -> Value {
    let mut value = value.clone();
    fn clean(n: &mut Value) {
        if let Some(a) = n.as_array_mut() {
            for n in a {
                clean(n);
            }
            return;
        }
        if let Some(a) = n.get_mut("attrs").and_then(Value::as_object_mut) {
            a.remove("officeId");
            if a.is_empty() {
                n.as_object_mut().unwrap().remove("attrs");
            }
        }
        if let Some(a) = n.get_mut("content").and_then(Value::as_array_mut) {
            for n in a {
                clean(n);
            }
        }
    }
    clean(&mut value);
    value
}
fn settings(value: &Value) -> Value {
    let mut n = plain(value);
    n.as_object_mut().unwrap().remove("content");
    n
}
fn model_xml(nodes: &[Value]) -> Result<Vec<String>, String> {
    fn check_inline(node: &Value) -> Result<(), String> {
        if let Some(marks) = node["marks"].as_array() {
            for mark in marks {
                if !matches!(
                    mark["type"].as_str(),
                    Some("bold" | "italic" | "underline" | "strike" | "highlight" | "textStyle")
                ) {
                    return Err("Recording changes to links or this text format needs additional support. Your draft has not been saved.".into());
                }
            }
        }
        if let Some(children) = node["content"].as_array() {
            for c in children {
                check_inline(c)?;
            }
        }
        Ok(())
    }
    nodes
        .iter()
        .map(|n| {
            check_inline(n)?;
            if n["type"] != "paragraph" && n["type"] != "heading" {
                return Err("Tracked paragraph edits cannot cross a document object".into());
            }
            let fragment = docx::write_block(n, &HashMap::new(), &HashMap::new(), "");
            let wrapped = format!("<root xmlns:w=\"{W}\">{fragment}</root>");
            let d = parse(&wrapped)?;
            let p = d
                .root_element()
                .children()
                .find(|n| n.is_element())
                .ok_or("Missing editor paragraph")?;
            if !p.has_tag_name((W, "p")) {
                return Err("Expected one editor paragraph".into());
            }
            Ok(scoped_record(&wrapped, p))
        })
        .collect()
}
fn length(nodes: &[Value]) -> usize {
    Index::new(&json!({"type":"doc","content":nodes}))
        .text
        .encode_utf16()
        .count()
}
fn local_edits(
    edits: &[Value],
    start: usize,
    end: usize,
    include_end: bool,
) -> Result<Vec<Value>, String> {
    let mut result = vec![];
    for edit in edits {
        let a = edit["start"].as_u64().ok_or("Invalid edit start")? as usize;
        let b = edit["end"].as_u64().ok_or("Invalid edit end")? as usize;
        if a > end || (a == end && !include_end) || (b <= start && a < start) {
            continue;
        }
        if a < start || b > end {
            return Err(
                "Tracked edits cannot cross an unsupported object or table cell boundary".into(),
            );
        }
        result.push(json!({"start":a-start,"end":b-start,"text":edit["text"]}));
    }
    Ok(result)
}
struct Recorder {
    reserved: String,
}
impl Recorder {
    fn container(
        &mut self,
        xml: &str,
        parent: Node,
        before: &[Value],
        after: &[Value],
        edits: &[Value],
        author: &track::Author,
    ) -> Result<String, String> {
        let (mut old_index, mut new_index, mut offset) = (0, 0, 0);
        let mut patches = vec![];
        for r in regions(xml, parent) {
            let projected = r
                .paragraphs
                .as_ref()
                .and_then(|p| track::current_story_view(p).ok());
            let count = projected.as_ref().map_or(r.nodes.len(), Vec::len);
            let old = before
                .get(old_index..old_index + count)
                .ok_or("Source/editor block mapping changed")?;
            let old_len = length(old);
            // Quill can append a new final empty paragraph after the existing
            // terminal newline. That point belongs to the last editable region,
            // whereas a shared boundary belongs to the following region.
            let mut exact = local_edits(
                edits,
                offset,
                offset + old_len,
                projected.is_some() && old_index + count == before.len(),
            )?;
            for edit in &mut exact {
                if edit["start"] == old_len && edit["end"] == old_len {
                    // The editor's final newline is a sentinel. Word records
                    // a new paragraph by inserting a break before its final
                    // mark, preserving that mark on the new last paragraph.
                    let text = edit["text"].as_str().ok_or("Invalid inserted text")?;
                    let inner = text
                        .strip_suffix('\n')
                        .ok_or("An appended paragraph must retain its final newline")?;
                    *edit = json!({"start":old_len - 1,"end":old_len - 1,
                        "text":format!("\n{inner}")});
                }
            }
            let mut new_len = old_len as i64;
            for e in &exact {
                new_len += e["text"]
                    .as_str()
                    .ok_or("Invalid inserted text")?
                    .encode_utf16()
                    .count() as i64
                    - (e["end"].as_i64().unwrap() - e["start"].as_i64().unwrap());
            }
            if new_len <= 0 {
                return Err(
                    "Tracked edits must retain the final paragraph in each container".into(),
                );
            }
            let mut new_end = new_index;
            let mut used = 0;
            while used < new_len as usize {
                let n = after
                    .get(new_end)
                    .ok_or("Missing replacement document block")?;
                used += length(std::slice::from_ref(n));
                new_end += 1;
            }
            if used != new_len as usize {
                return Err("Tracked edits changed an object boundary".into());
            }
            let new = &after[new_index..new_end];
            let changed = plain(&json!(old)) != plain(&json!(new));
            if changed || !exact.is_empty() {
                let output = if projected.is_some() {
                    let result = track::record_projected_story(
                        r.paragraphs.as_ref().unwrap(),
                        &model_xml(old)?,
                        &model_xml(new)?,
                        &json!(exact),
                        author,
                        &self.reserved,
                    )?;
                    result.concat()
                } else if r.nodes[0].has_tag_name((W, "tbl"))
                    && old.len() == 1
                    && new.len() == 1
                    && old[0]["type"] == "table"
                    && new[0]["type"] == "table"
                {
                    self.table(xml, r.nodes[0], &old[0], &new[0], &exact, author)?
                } else {
                    return Err(
                        "Recording changes to this Word object needs additional support".into(),
                    );
                };
                self.reserved.insert_str(self.reserved.len() - 7, &output);
                patches.push((
                    r.nodes[0].range().start..r.nodes.last().unwrap().range().end,
                    output,
                ));
            }
            old_index += count;
            new_index = new_end;
            offset += old_len;
        }
        if old_index != before.len() || new_index != after.len() {
            return Err("Source/editor container mapping changed".into());
        }
        let start = parent.range().start;
        let scoped = scoped_record(xml, parent);
        let modified = replace_ranges(
            raw(xml, parent),
            patches
                .into_iter()
                .map(|(r, v)| ((r.start - start)..(r.end - start), v))
                .collect(),
        );
        let d = parse(&scoped)?;
        let open = opening_tag_end(&modified).ok_or("Invalid edited container")? + 1;
        let close = modified.rfind("</").unwrap_or(open);
        Ok(replace_inner(
            &scoped,
            d.root_element(),
            &modified[open..close],
        ))
    }
    fn table(
        &mut self,
        xml: &str,
        table: Node,
        before: &Value,
        after: &Value,
        edits: &[Value],
        author: &track::Author,
    ) -> Result<String, String> {
        if settings(before) != settings(after) {
            return Err("Tracked table property changes need additional support".into());
        }
        let rows = table
            .children()
            .filter(|n| n.has_tag_name((W, "tr")))
            .collect::<Vec<_>>();
        let old = before["content"].as_array().ok_or("Missing table rows")?;
        let new = after["content"].as_array().ok_or("Missing table rows")?;
        if rows.len() != old.len() || rows.len() != new.len() {
            return Err("Tracked row insertion or deletion needs additional support".into());
        }
        let (mut patches, mut offset) = (vec![], 0);
        for ((row, old), new) in rows.iter().zip(old).zip(new) {
            if settings(old) != settings(new) {
                return Err("Tracked row property changes need additional support".into());
            }
            let cells = row
                .children()
                .filter(|n| n.has_tag_name((W, "tc")))
                .collect::<Vec<_>>();
            let old = old["content"].as_array().ok_or("Missing cells")?;
            let new = new["content"].as_array().ok_or("Missing cells")?;
            if cells.len() != old.len() || cells.len() != new.len() {
                return Err("Tracked cell structure changes need additional support".into());
            }
            for ((cell, old), new) in cells.iter().zip(old).zip(new) {
                if settings(old) != settings(new) {
                    return Err("Tracked cell property changes need additional support".into());
                }
                let old = old["content"].as_array().ok_or("Missing cell contents")?;
                let new = new["content"].as_array().ok_or("Missing cell contents")?;
                let len = length(old);
                let exact = local_edits(edits, offset, offset + len, false)?;
                let output = self.container(xml, *cell, old, new, &exact, author)?;
                patches.push((cell.range(), output));
                offset += len;
            }
        }
        let result = replace_ranges(xml, patches);
        let d = parse(&result)?;
        let n = d
            .descendants()
            .find(|n| n.range().start == table.range().start && n.has_tag_name((W, "tbl")))
            .ok_or("Edited table missing")?;
        Ok(scoped_record(&result, n))
    }
}
pub fn edit(
    pkg: &Package,
    before: &Value,
    after: &Value,
    exact: &Value,
    author: &track::Author,
) -> Result<Package, String> {
    // Validate the journal independently before using it to align containers.
    crate::document_bookmarks::inverse_edits(before, after, exact)?;
    if crate::docx_revisions::list(pkg)?["protected"] == true {
        return Err("This document has enforced editing protection".into());
    }
    if settings(before) != settings(after) {
        return Err("Tracked document setting changes need additional support".into());
    }
    validate_projection(pkg, before)?;
    let xml = pkg.text("word/document.xml")?;
    let d = parse(xml)?;
    let root = d
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .ok_or("Word body missing")?;
    let mut recorder = Recorder {
        reserved: format!("<root>{}</root>", scoped_record(xml, d.root_element())),
    };
    let output = recorder.container(
        xml,
        root,
        before["content"].as_array().ok_or("Missing document")?,
        after["content"].as_array().ok_or("Missing document")?,
        exact.as_array().ok_or("Exact edits required")?,
        author,
    )?;
    let mut result = pkg.clone();
    result.set(
        "word/document.xml",
        replace_ranges(xml, vec![(root.range(), output)]),
    );
    result.original = result.bytes()?;
    Ok(result)
}

pub fn validate_projection(pkg: &Package, document: &Value) -> Result<(), String> {
    let current = import_current(pkg)?.0;
    if Index::new(&current).text != Index::new(document).text {
        return Err("The editable document no longer matches its tracked source".into());
    }
    Ok(())
}
pub fn validate_settings(settings: &Value) -> Result<(), String> {
    if settings["version"] != 1
        || !settings["enabled"].is_boolean()
        || !settings["formatting"].is_boolean()
    {
        return Err("Invalid Track Changes settings".into());
    }
    let name = settings["author"].as_str().ok_or("Enter a review author")?;
    if name.trim().is_empty() || name.len() > 512 || name.chars().any(char::is_control) {
        return Err("Enter a valid review author".into());
    }
    Ok(())
}
pub fn read_settings(pkg: &Package) -> Result<Value, String> {
    let path = crate::docx_tabs::part(pkg, "settings")?;
    let Ok(xml) = pkg.text(&path) else {
        return Ok(json!({"enabled":false,"formatting":true}));
    };
    let d = parse(xml)?;
    let enabled = |tag: &str| {
        d.root_element()
            .children()
            .find(|n| n.has_tag_name((W, tag)))
            .is_some_and(|n| !matches!(n.attribute((W, "val")), Some("0" | "false" | "off")))
    };
    Ok(json!({"enabled":enabled("trackRevisions"),"formatting":!enabled("doNotTrackFormatting")}))
}
pub fn write_settings(pkg: &mut Package, settings: &Value) -> Result<(), String> {
    validate_settings(settings)?;
    let path = crate::docx_tabs::ensure_settings(pkg)?;
    // CT_Settings orders revision options after revisionView and before
    // documentProtection/defaultTabStop. All other settings stay byte intact.
    let preceding = [
        "writeProtection",
        "view",
        "zoom",
        "removePersonalInformation",
        "removeDateAndTime",
        "doNotDisplayPageBoundaries",
        "displayBackgroundShape",
        "printPostScriptOverText",
        "printFractionalCharacterWidth",
        "printFormsData",
        "embedTrueTypeFonts",
        "embedSystemFonts",
        "saveSubsetFonts",
        "saveFormsData",
        "mirrorMargins",
        "alignBordersAndEdges",
        "bordersDoNotSurroundHeader",
        "bordersDoNotSurroundFooter",
        "gutterAtTop",
        "hideSpellingErrors",
        "hideGrammaticalErrors",
        "activeWritingStyle",
        "proofState",
        "formsDesign",
        "attachedTemplate",
        "linkStyles",
        "stylePaneFormatFilter",
        "stylePaneSortMethod",
        "documentType",
        "mailMerge",
        "revisionView",
        "trackRevisions",
        "doNotTrackMoves",
        "doNotTrackFormatting",
    ];
    for (tag, enabled) in [
        ("trackRevisions", settings["enabled"] == true),
        ("doNotTrackFormatting", settings["formatting"] == false),
    ] {
        let xml = pkg.text(&path)?.to_string();
        let d = parse(&xml)?;
        let root = d.root_element();
        let rank = preceding.iter().position(|s| *s == tag).unwrap();
        let property = if enabled {
            format!("<w:{tag} xmlns:w=\"{W}\"/>")
        } else {
            String::new()
        };
        let mut inner = String::new();
        let mut placed = false;
        for n in root.children() {
            if n.has_tag_name((W, tag)) {
                continue;
            }
            if !placed
                && n.is_element()
                && preceding
                    .iter()
                    .position(|s| *s == n.tag_name().name())
                    .is_none_or(|p| p > rank)
            {
                inner.push_str(&property);
                placed = true;
            }
            inner.push_str(raw(&xml, n));
        }
        if !placed {
            inner.push_str(&property);
        }
        pkg.set(&path, replace_inner(&xml, root, &inner));
    }
    pkg.original = pkg.bytes()?;
    Ok(())
}
pub fn check_bindings(document: &Value) -> Result<(), String> {
    if document["type"] == "workbookBlock" {
        return Err(
            "Recording tracked edits around live workbook links needs additional support".into(),
        );
    }
    if let Some(children) = document["content"].as_array() {
        for n in children {
            check_bindings(n)?;
        }
    }
    Ok(())
}
