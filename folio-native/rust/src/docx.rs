use crate::office::*;
use serde_json::{json, Value};
use std::collections::HashMap;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

pub fn import(pkg: &Package) -> Result<(Value, Value), String> {
    let xml = pkg.text("word/document.xml")?;
    let dom = parse(xml)?;
    let body = dom
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .ok_or("Word document body is missing")?;
    let rels = relations(pkg);
    let numbering = crate::docx_numbering::Catalog::read(pkg)?;
    let tabs = crate::docx_tabs::Catalog::read(pkg)?;
    let mut blocks = vec![];
    let mut warnings = vec![];
    for (i, n) in body
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() != "sectPr")
        .enumerate()
    {
        let node = read_block(n, &rels, &numbering, &tabs, &format!("word-{i}"));
        if node["type"] == "officeBlock" {
            warnings.push(
                "Some Word objects are preserved in the file and shown as read-only blocks.".into(),
            );
        }
        blocks.push(node);
    }
    if blocks.is_empty() {
        blocks.push(json!({"type":"paragraph"}));
    }
    for(part,label)in [("word/header","Headers and footers are preserved but not paginated in the editor."),("word/media","Embedded images and media are preserved; complex image layout may render as a preserved block.")] {if pkg.entries.keys().any(|k|k.starts_with(part)){warnings.push(label.into());}}
    if crate::docx_revisions::has_changes(dom.root_element()) {
        warnings.push("Use Review > Track Changes to review preserved revisions, or Record changes to choose an author and record supported edits.".into());
    }
    let mut document = json!({"type":"doc","content":blocks});
    crate::docx_notes::import(&mut document, pkg);
    document["attrs"][crate::docx_page_setup::KEY] = crate::docx_page_setup::read(pkg)?;
    document["attrs"][crate::docx_styles::KEY] = crate::docx_styles::read(pkg)?;
    if let Some(default) = tabs.default {
        document["attrs"]["defaultTabStop"] = json!(default);
    }
    if crate::docx_bookmarks::import(&mut document, xml)? {
        warnings.push("Some bookmarks use preserved Word objects or advanced ranges. They remain in the source; editing their containing content may prevent Word export.".into());
    }
    if crate::docx_comments::import(&mut document, pkg)? {
        warnings.push("Some comments use advanced content or anchors and remain preserved in the source. Editing their containing content may prevent Word export.".into());
    }
    warnings.push(
        "Word pagination, floating objects, and advanced layout are not yet reproduced exactly."
            .into(),
    );
    warnings.sort();
    warnings.dedup();
    Ok((document, report("DOCX", warnings, pkg.entries.len())))
}
fn relations(pkg: &Package) -> HashMap<String, String> {
    let mut r = HashMap::new();
    if let Ok(xml) = pkg.text("word/_rels/document.xml.rels") {
        if let Ok(d) = parse(xml) {
            for n in d
                .descendants()
                .filter(|n| n.tag_name().name() == "Relationship")
            {
                if let (Some(id), Some(target)) = (attr(n, "Id"), attr(n, "Target")) {
                    r.insert(id.into(), target.into());
                }
            }
        }
    }
    r
}
pub(crate) fn read_block(
    node: roxmltree::Node,
    rels: &HashMap<String, String>,
    numbering: &crate::docx_numbering::Catalog,
    tabs: &crate::docx_tabs::Catalog,
    id: &str,
) -> Value {
    let typ = node.tag_name().name();
    if crate::docx_moves::is_boundary(node) {
        return json!({"type":"officeBlock","attrs":{"officeId":id,"hiddenReviewMarker":true},"content":[]});
    }
    if crate::docx_revisions::has_changes(node) {
        let preview = crate::docx_revisions::preview(node);
        let text = preview
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| s["text"].as_str())
            .collect::<String>();
        return json!({"type":"officeBlock","attrs":{"officeId":id,"label":"Tracked changes","revisionPreview":preview},"content":[{"type":"paragraph","content":[{"type":"text","text":text}]}]});
    }
    if typ == "p" {
        if let Ok(Some((projected, fields))) = crate::docx_fields::project(node) {
            if let Ok(d) = parse(&projected) {
                let mut value = read_block(d.root_element(), rels, numbering, tabs, id);
                if let Some(content) = value["content"].as_array_mut() {
                    for n in content {
                        if let Some((_, field)) =
                            fields.iter().find(|(token, _)| n["text"] == *token)
                        {
                            let marks = n.get("marks").cloned().unwrap_or(json!([]));
                            *n = field.clone();
                            n["attrs"]["sourceMarks"] = marks.clone();
                            if marks.as_array().is_some_and(|m| !m.is_empty()) {
                                n["marks"] = marks;
                            }
                        }
                    }
                }
                return value;
            }
        }
    }
    if typ == "p"
        && !node.descendants().any(|n| {
            n.is_element()
                && n.tag_name().name() == "drawing"
                && !crate::docx_pictures::is_inline_picture(n)
        })
        && !node.descendants().any(|n| {
            n.is_element()
                && matches!(
                    n.tag_name().name(),
                    "pict"
                        | "object"
                        | "fldChar"
                        | "fldSimple"
                        | "instrText"
                        | "ins"
                        | "del"
                        | "sdt"
                        | "oMath"
                )
        })
    {
        let mut attrs = json!({"officeId":id});
        let pr = child(node, "pPr");
        let style = pr
            .and_then(|p| child(p, "pStyle"))
            .and_then(|s| attr(s, "val"))
            .unwrap_or("");
        let heading = tabs
            .paragraph_format(node, &Value::Null)
            .and_then(|f| crate::docx_paragraph::outline_level(&f))
            .or_else(|| {
                style
                    .to_lowercase()
                    .strip_prefix("heading")
                    .and_then(|s| s.parse::<u64>().ok())
                    .filter(|n| (1..=9).contains(n))
                    .map(|n| n - 1)
            })
            .filter(|n| *n < 9)
            .map(|n| n + 1);
        if let Some(level) = heading {
            attrs["level"] = json!(level.clamp(1, 9));
        }
        if let Some(align) = pr.and_then(|p| child(p, "jc")).and_then(|n| attr(n, "val")) {
            attrs["textAlign"] = json!(if align == "both" { "justify" } else { align });
        }
        if let Some(list) = numbering.paragraph(node) {
            attrs["numbering"] = list;
        }
        if let Some(t) = tabs.paragraph(node, &attrs["numbering"]) {
            attrs["tabs"] = t;
        }
        if let Some(p) = tabs.paragraph_format(node, &attrs["numbering"]) {
            attrs["paragraphFormat"] = p;
        }
        let mut content = vec![];
        for run in node
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "r")
        {
            let rp = child(run, "rPr");
            let mut marks = vec![];
            for (xml, mark) in [
                ("b", "bold"),
                ("i", "italic"),
                ("u", "underline"),
                ("strike", "strike"),
            ] {
                if let Some(p) = rp.and_then(|p| child(p, xml)) {
                    if matches!(attr(p, "val"), Some("0" | "false" | "off" | "none")) {
                        marks.push(json!({"type":mark,"attrs":{"value":false}}));
                    } else {
                        marks.push(json!({"type":mark}));
                    }
                }
            }
            let mut ts = serde_json::Map::new();
            if let Some(c) = rp
                .and_then(|p| child(p, "color"))
                .and_then(|n| attr(n, "val"))
            {
                if c != "auto" {
                    ts.insert("color".into(), json!(format!("#{c}")));
                }
            }
            if let Some(s) = rp
                .and_then(|p| child(p, "sz"))
                .and_then(|n| attr(n, "val"))
                .and_then(|s| s.parse::<f64>().ok())
            {
                ts.insert("fontSize".into(), json!(s / 2.0));
            }
            if let Some(f) = rp
                .and_then(|p| child(p, "rFonts"))
                .and_then(|n| attr(n, "ascii"))
            {
                ts.insert("fontFamily".into(), json!(f));
            }
            if !ts.is_empty() {
                marks.push(json!({"type":"textStyle","attrs":ts}));
            }
            if let Some(color) = rp
                .and_then(|p| child(p, "shd"))
                .and_then(|n| attr(n, "fill"))
                .filter(|s| s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            {
                marks.push(json!({"type":"highlight","attrs":{"color":format!("#{color}")}}));
            }
            if let Some(color) = rp
                .and_then(|p| child(p, "highlight"))
                .and_then(|n| attr(n, "val"))
            {
                marks.push(json!({"type":"highlight","attrs":{"color":highlight_color(color)}}));
            }
            if let Some(link) = run
                .ancestors()
                .find(|n| n.is_element() && n.tag_name().name() == "hyperlink")
                .and_then(|n| {
                    n.attribute((W, "anchor"))
                        .map(|name| format!("#{name}"))
                        .or_else(|| attr(n, "id").and_then(|id| rels.get(id)).cloned())
                })
            {
                marks.push(json!({"type":"link","attrs":{"href":link}}));
            }
            for part in run.children().filter(|n| n.is_element()) {
                match part.tag_name().name() {
                    "t" => {
                        let text = part.text().unwrap_or("");
                        if !text.is_empty() {
                            let mut text = json!({"type":"text","text":text});
                            if !marks.is_empty() {
                                text["marks"] = json!(marks);
                            }
                            content.push(text);
                        }
                    }
                    "footnoteReference" | "endnoteReference" => {
                        if let Some(mut note) = crate::docx_notes::read(run, part) {
                            if !marks.is_empty() {
                                note["marks"] = json!(marks);
                            }
                            content.push(note);
                        }
                    }
                    "drawing" => {
                        if let Some(mut image) = crate::docx_pictures::read(part, rels) {
                            if !marks.is_empty() {
                                image["marks"] = json!(marks);
                            }
                            content.push(image);
                        }
                    }
                    "tab" => {
                        let mut tab = json!({"type":"text","text":"\t"});
                        if !marks.is_empty() {
                            tab["marks"] = json!(marks);
                        }
                        content.push(tab);
                    }
                    "br" | "cr" => {
                        let mut value = json!({"type":"hardBreak"});
                        if !marks.is_empty() {
                            value["marks"] = json!(marks);
                        }
                        if let Some(kind) = part
                            .attribute((W, "type"))
                            .filter(|v| ["page", "column", "textWrapping"].contains(v))
                        {
                            value["attrs"]["breakType"] = json!(kind);
                        }
                        if let Some(clear) = part.attribute((W, "clear")) {
                            value["attrs"]["clear"] = json!(clear);
                        }
                        content.push(value);
                    }
                    _ => {}
                }
            }
        }
        return json!({"type":if heading.is_some(){"heading"}else{"paragraph"},"attrs":attrs,"content":content});
    }
    if typ == "tbl" {
        if let Some(table) = crate::docx_tables::read(node, rels, numbering, tabs, id) {
            return table;
        }
    }
    if typ == "sdt" {
        if let Some(toc) = crate::docx_references::read(node, id) {
            return toc;
        }
    }
    json!({"type":"officeBlock","attrs":{"officeId":id,"label":format!("Preserved Word {typ}")},"content":[{"type":"paragraph","content":[{"type":"text","text":xml_text(node)}]}]})
}
pub fn export(
    document: &Value,
    pkg: Option<&Package>,
    baseline: Option<&Value>,
) -> Result<Vec<u8>, String> {
    export_impl(document, pkg, baseline, false)
}
pub(crate) fn export_review(
    document: &Value,
    pkg: Option<&Package>,
    baseline: Option<&Value>,
) -> Result<Package, String> {
    Package::from_bytes(export_impl(document, pkg, baseline, true)?)
}
fn export_impl(
    document: &Value,
    pkg: Option<&Package>,
    baseline: Option<&Value>,
    review: bool,
) -> Result<Vec<u8>, String> {
    if let (Some(p), Some(b)) = (pkg, baseline) {
        if document == b && !review {
            return Ok(p.original.clone());
        }
    }
    let mut result = pkg.cloned().unwrap_or_else(new_package);
    let noted = crate::docx_notes::prepare(document, &mut result)?;
    let document = &noted;
    crate::docx_tabs::write_default(document, &mut result)?;
    crate::docx_styles::write(document, &mut result)?;
    let numbered = crate::docx_numbering::prepare(document, &mut result)?;
    let original_xml = result.text("word/document.xml")?.to_string();
    let projected = crate::docx_bookmarks::project(&numbered, &original_xml)?;
    let commented = crate::docx_comments::project(&projected, baseline, &mut result)?;
    let document = &commented;
    let mut managed = crate::docx_bookmarks::source_ids(baseline);
    managed.extend(crate::docx_bookmarks::source_ids(Some(document)));
    let xml = crate::docx_bookmarks::strip(&original_xml, &managed)?;
    let mut managed_comments = crate::docx_comments::source_ids(baseline);
    managed_comments.extend(crate::docx_comments::source_ids(Some(document)));
    let xml = crate::docx_comments::strip(&xml, &managed_comments)?;
    let dom = parse(&xml)?;
    let body = dom
        .descendants()
        .find(|n| n.tag_name().name() == "body")
        .ok_or("Word body missing")?;
    let originals: Vec<_> = body
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() != "sectPr")
        .collect();
    let baseline_nodes = baseline.and_then(|b| b["content"].as_array());
    let mut old = HashMap::new();
    let mut sources = HashMap::new();
    for (i, n) in originals.iter().enumerate() {
        crate::docx_tables::source_map(&xml, *n, format!("word-{i}"), &mut sources);
    }
    if let Some(nodes) = baseline_nodes {
        for b in nodes {
            crate::docx_tables::collect_old(b, &sources, &mut old);
        }
    }
    let namespaces = dom
        .root_element()
        .namespaces()
        .filter(|n| !matches!(n.name(), Some("w" | "r")))
        .map(|n| match n.name() {
            Some(prefix) => format!(" xmlns:{prefix}=\"{}\"", esc(n.uri())),
            None => format!(" xmlns=\"{}\"", esc(n.uri())),
        })
        .collect::<String>();
    let mut links = relations(&result);
    let mut by_target: HashMap<String, String> = links
        .iter()
        .map(|(id, target)| (target.clone(), id.clone()))
        .collect();
    fn collect_links(n: &Value, out: &mut Vec<String>) {
        if let Some(marks) = n["marks"].as_array() {
            for m in marks {
                if m["type"] == "link" {
                    if let Some(href) = m["attrs"]["href"].as_str() {
                        if href.starts_with("https://")
                            || href.starts_with("http://")
                            || href.starts_with("mailto:")
                        {
                            out.push(href.into());
                        }
                    }
                }
            }
        }
        if let Some(a) = n["content"].as_array() {
            for c in a {
                collect_links(c, out)
            }
        }
    }
    let mut targets = vec![];
    collect_links(document, &mut targets);
    let mut additions = String::new();
    for target in targets {
        if !by_target.contains_key(&target) {
            let mut num = links.len() + 1;
            while links.contains_key(&format!("folioLink{num}")) {
                num += 1;
            }
            let id = format!("folioLink{num}");
            additions.push_str(&format!("<Relationship Id=\"{id}\" Type=\"{R}/hyperlink\" Target=\"{}\" TargetMode=\"External\"/>",esc(&target)));
            links.insert(id.clone(), target.clone());
            by_target.insert(target, id);
        }
    }
    let mut content = String::new();
    let blocks = document["content"].as_array().ok_or("Invalid document")?;
    for (i, n) in blocks.iter().enumerate() {
        let mut written = write_block(n, &old, &by_target, &namespaces);
        // Enter duplicates the paragraph identity. The section break belongs
        // to the final resulting paragraph, just like its original end mark.
        if let Some(id) = n["attrs"]["officeId"].as_str() {
            if blocks[i + 1..]
                .iter()
                .any(|later| later["attrs"]["officeId"] == id)
            {
                written = crate::docx_page_setup::strip_paragraph_boundary(&written, &namespaces)?;
            }
        }
        if review && n["type"] == "workbookBlock" {
            content.push_str(&crate::docx_revisions::mark_binding(
                &written,
                &namespaces,
                i,
            )?);
        } else {
            content.push_str(&written);
        }
    }
    if let Some(section) = child(body, "sectPr") {
        content.push_str(raw(&xml, section));
    }
    let output_xml = replace_inner(&xml, body, &content);
    crate::docx_bookmarks::check_generated(document, &output_xml)?;
    crate::docx_bookmarks::check_preserved(&original_xml, &output_xml, &managed)?;
    crate::docx_comments::check(&original_xml, &output_xml, &managed_comments, document)?;
    crate::docx_revisions::check_retained(&original_xml, &output_xml)?;
    crate::docx_page_setup::check_retained(&original_xml, &output_xml)?;
    result.set("word/document.xml", output_xml);
    if !additions.is_empty() {
        let rels = result
            .text("word/_rels/document.xml.rels")
            .map(String::from)
            .unwrap_or(format!("<Relationships xmlns=\"{REL_NS}\"/>"));
        let d = parse(&rels)?;
        let root = d.root_element();
        let inner = root.children().map(|n| raw(&rels, n)).collect::<String>();
        result.set(
            "word/_rels/document.xml.rels",
            replace_inner(&rels, root, &(inner + &additions)),
        );
    }
    result.bytes()
}
pub(crate) fn write_block(
    node: &Value,
    old: &HashMap<&str, (&Value, &str)>,
    links: &HashMap<String, String>,
    namespaces: &str,
) -> String {
    if let Some(events) = node["attrs"].get(crate::docx_comments::EVENTS) {
        let mut plain = node.clone();
        plain["attrs"]
            .as_object_mut()
            .unwrap()
            .remove(crate::docx_comments::EVENTS);
        if plain["attrs"].as_object().is_some_and(|a| a.is_empty()) {
            plain.as_object_mut().unwrap().remove("attrs");
        }
        let xml = write_block(&plain, old, links, namespaces);
        return crate::docx_bookmarks::inject_xml(&xml, events, namespaces);
    }
    if let Some(events) = node["attrs"].get(crate::docx_bookmarks::EVENTS) {
        let mut plain = node.clone();
        plain["attrs"]
            .as_object_mut()
            .unwrap()
            .remove(crate::docx_bookmarks::EVENTS);
        if plain["attrs"].as_object().is_some_and(|a| a.is_empty()) {
            plain.as_object_mut().unwrap().remove("attrs");
        }
        let xml = write_block_content(&plain, old, links, namespaces);
        return crate::docx_bookmarks::inject(&xml, events, namespaces);
    }
    write_block_content(node, old, links, namespaces)
}
fn write_block_content(
    node: &Value,
    old: &HashMap<&str, (&Value, &str)>,
    links: &HashMap<String, String>,
    namespaces: &str,
) -> String {
    let previous = node["attrs"]["officeId"]
        .as_str()
        .and_then(|id| old.get(id));
    if let Some((before, xml)) = previous {
        if *before == node {
            return xml.to_string();
        }
    }
    if node["type"] == "officeBlock" {
        return previous
            .map(|(_, xml)| xml.to_string())
            .unwrap_or_else(|| paragraph(&crate::workspace::document_text(node), ""));
    }
    if node["type"] == "tableOfContents" {
        return crate::docx_references::write(node);
    }
    if node["type"] == "workbookBlock" {
        return node["content"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|n| write_block(n, old, links, namespaces))
                    .collect()
            })
            .unwrap_or_default();
    }
    match node["type"].as_str().unwrap_or("") {
        "paragraph" | "heading" => {
            let mut props = previous
                .and_then(|(_, xml)| extract_props(xml, namespaces))
                .unwrap_or_default();
            let before = previous.map(|p| p.0);
            let heading_changed = before.is_none()
                || before.is_some_and(|b| {
                    b["type"] != node["type"] || b["attrs"]["level"] != node["attrs"]["level"]
                });
            let align_changed = before.is_none()
                || before.is_some_and(|b| b["attrs"]["textAlign"] != node["attrs"]["textAlign"]);
            if heading_changed {
                props = set_prop(
                    &props,
                    namespaces,
                    "pStyle",
                    if node["type"] == "heading" {
                        format!(
                            "<w:pStyle w:val=\"Heading{}\"/>",
                            node["attrs"]["level"].as_u64().unwrap_or(1)
                        )
                    } else {
                        String::new()
                    },
                );
            }
            let style = &node["attrs"]["paragraphFormat"]["styleId"];
            let style_changed =
                before.is_some_and(|b| b["attrs"]["paragraphFormat"]["styleId"] != *style);
            let inherited_outline_changed = before.is_some_and(|b| {
                b["attrs"]["paragraphFormat"]["inheritedOutlineLevel"]
                    != node["attrs"]["paragraphFormat"]["inheritedOutlineLevel"]
            });
            // Explicit outline edits change structure without substituting the visual style.
            let direct_outline = node["attrs"]["paragraphFormat"]["direct"]
                .get("outlineLevel")
                .is_some()
                || before.is_some_and(|b| {
                    b["attrs"]["paragraphFormat"]["direct"]
                        .get("outlineLevel")
                        .is_some()
                });
            if (direct_outline && !style.is_null())
                || style_changed
                || (inherited_outline_changed && !style.is_null())
                || (before.is_none()
                    && !style.is_null()
                    && (node["type"] != "heading"
                        || node["attrs"]["paragraphFormat"]["inheritedOutlineLevel"]
                            .as_u64()
                            .is_some_and(|n| Some(n + 1) == node["attrs"]["level"].as_u64())))
            {
                props = set_prop(
                    &props,
                    namespaces,
                    "pStyle",
                    style
                        .as_str()
                        .map(|s| format!("<w:pStyle w:val=\"{}\"/>", esc(s)))
                        .unwrap_or_default(),
                );
            }
            if align_changed {
                props = set_prop(
                    &props,
                    namespaces,
                    "jc",
                    node["attrs"]["textAlign"]
                        .as_str()
                        .map(|a| {
                            format!(
                                "<w:jc w:val=\"{}\"/>",
                                esc(if a == "justify" { "both" } else { a })
                            )
                        })
                        .unwrap_or_default(),
                );
            }
            if before.is_none()
                || before.is_some_and(|b| b["attrs"]["numbering"] != node["attrs"]["numbering"])
            {
                let numbering = &node["attrs"]["numbering"];
                let property = if let Some(id) = numbering["numId"].as_str() {
                    format!(
                        "<w:numPr><w:ilvl w:val=\"{}\"/><w:numId w:val=\"{}\"/></w:numPr>",
                        numbering["level"].as_u64().unwrap_or(0),
                        esc(id)
                    )
                } else if before.is_some_and(|b| !b["attrs"]["numbering"].is_null()) {
                    "<w:numPr><w:numId w:val=\"0\"/></w:numPr>".into()
                } else {
                    String::new()
                };
                props = set_prop(&props, namespaces, "numPr", property);
            }
            if before.is_none()
                || before.is_some_and(|b| {
                    b["attrs"]["tabs"]["direct"] != node["attrs"]["tabs"]["direct"]
                })
            {
                props = set_prop(
                    &props,
                    namespaces,
                    "tabs",
                    crate::docx_tabs::xml(&node["attrs"]["tabs"]),
                );
            }
            props = crate::docx_paragraph::patch(props, namespaces, node, before);
            if let Some((b, original)) = previous.filter(|(b, _)| b["content"] == node["content"]) {
                let _ = b;
                let wrapped = wrap_fragment(original, namespaces);
                if let Ok(d) = parse(&wrapped) {
                    if let Some(p) = d
                        .root_element()
                        .children()
                        .find(|n| n.has_tag_name((W, "p")))
                    {
                        let inline = p
                            .children()
                            .filter(|n| !n.has_tag_name((W, "pPr")))
                            .map(|n| raw(&wrapped, n))
                            .collect::<String>();
                        let changed = replace_inner(&wrapped, p, &format!("{props}{inline}"));
                        if let Ok(d) = parse(&changed) {
                            if let Some(p) = d
                                .root_element()
                                .children()
                                .find(|n| n.has_tag_name((W, "p")))
                            {
                                return raw(&changed, p).into();
                            }
                        }
                    }
                }
            }
            let inline = node["content"]
                .as_array()
                .map(|a| a.iter().map(|n| write_inline(n, links)).collect::<String>())
                .unwrap_or_default();
            format!("<w:p xmlns:w=\"{W}\" xmlns:r=\"{R}\"{namespaces}>{props}{inline}</w:p>")
        }
        "table" => crate::docx_tables::write(node, old, links, namespaces),
        _ => paragraph(&crate::workspace::document_text(node), ""),
    }
}
pub(crate) fn paragraph(text: &str, props: &str) -> String {
    format!(
        "<w:p xmlns:w=\"{W}\">{props}<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        esc(text)
    )
}
fn write_inline(n: &Value, links: &HashMap<String, String>) -> String {
    let mut marks = String::new();
    if let Some(a) = n["marks"].as_array() {
        for m in a {
            match m["type"].as_str().unwrap_or("") {
                "bold" | "italic" | "underline" | "strike" => {
                    let tag = match m["type"].as_str().unwrap() {
                        "bold" => "b",
                        "italic" => "i",
                        "underline" => "u",
                        _ => "strike",
                    };
                    let val = if m["attrs"]["value"] == false {
                        if tag == "u" {
                            "none"
                        } else {
                            "0"
                        }
                    } else if tag == "u" {
                        "single"
                    } else {
                        "1"
                    };
                    marks.push_str(&format!("<w:{tag} w:val=\"{val}\"/>"));
                }
                "highlight" => {
                    if let Some(c) = m["attrs"]["color"].as_str() {
                        marks.push_str(&format!(
                            "<w:shd w:val=\"clear\" w:fill=\"{}\"/>",
                            esc(c.trim_start_matches('#'))
                        ));
                    }
                }
                "textStyle" => {
                    if let Some(c) = m["attrs"]["color"].as_str() {
                        marks.push_str(&format!(
                            "<w:color w:val=\"{}\"/>",
                            esc(c.trim_start_matches('#'))
                        ));
                    }
                    if let Some(s) = m["attrs"]["fontSize"].as_f64().or_else(|| {
                        m["attrs"]["fontSize"]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                    }) {
                        marks.push_str(&format!("<w:sz w:val=\"{}\"/>", (s * 2.0).round() as u32));
                    }
                    if let Some(f) = m["attrs"]["fontFamily"].as_str() {
                        marks.push_str(&format!(
                            "<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\"/>",
                            esc(f),
                            esc(f)
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    if n["type"] == "pageField" {
        return crate::docx_fields::write(n, &marks);
    }
    if n["type"] == "noteRef" {
        return crate::docx_notes::write(n, &marks);
    }
    if n["type"] == "image" {
        // Drawing ids must be unique in a document; new ones count up from
        // well above Word's own.
        static NEXT: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(1_000_000);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return crate::docx_pictures::write(n, &marks, id);
    }
    let text = n["text"].as_str().unwrap_or("");
    let mut inline = String::new();
    if n["type"] == "hardBreak" {
        let mut attrs = String::new();
        if let Some(kind) = n["attrs"]["breakType"]
            .as_str()
            .filter(|v| ["page", "column", "textWrapping"].contains(v))
        {
            attrs.push_str(&format!(" w:type=\"{kind}\""));
        }
        if let Some(clear) = n["attrs"]["clear"].as_str() {
            attrs.push_str(&format!(" w:clear=\"{}\"", esc(clear)));
        }
        inline.push_str(&format!("<w:br{attrs}/>"));
    } else {
        for (i, part) in text.split('\t').enumerate() {
            if i > 0 {
                inline.push_str("<w:tab/>");
            }
            if !part.is_empty() {
                inline.push_str(&format!("<w:t xml:space=\"preserve\">{}</w:t>", esc(part)));
            }
        }
    }
    let run = format!("<w:r><w:rPr>{marks}</w:rPr>{inline}</w:r>");
    let href = n["marks"]
        .as_array()
        .and_then(|a| a.iter().find(|m| m["type"] == "link"))
        .and_then(|m| m["attrs"]["href"].as_str());
    if let Some(anchor) = href.and_then(|h| h.strip_prefix('#')) {
        return format!(
            "<w:hyperlink w:anchor=\"{}\">{run}</w:hyperlink>",
            esc(anchor)
        );
    }
    let id = href.and_then(|h| links.get(h));
    if let Some(id) = id {
        format!("<w:hyperlink r:id=\"{}\">{run}</w:hyperlink>", esc(id))
    } else {
        run
    }
}
pub(crate) fn new_package() -> Package {
    let mut p = Package::default();
    p.set("[Content_Types].xml",format!("{XML_HEAD}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/></Types>"));
    p.set("_rels/.rels",format!("{XML_HEAD}<Relationships xmlns=\"{REL_NS}\"><Relationship Id=\"rId1\" Type=\"{R}/officeDocument\" Target=\"word/document.xml\"/></Relationships>"));
    p.set("word/_rels/document.xml.rels",format!("{XML_HEAD}<Relationships xmlns=\"{REL_NS}\"><Relationship Id=\"rIdStyles\" Type=\"{R}/styles\" Target=\"styles.xml\"/></Relationships>"));
    p.set("word/document.xml",format!("{XML_HEAD}<w:document xmlns:w=\"{W}\" xmlns:r=\"{R}\"><w:body><w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\"/></w:sectPr></w:body></w:document>"));
    // Match the native editor's house styles, so creating an unused style does
    // not restyle an existing native document when inheritance is materialized.
    let headings = [
        (48, 0, 270, false),
        (36, 270, 180, false),
        (29, 210, 180, false),
        (30, 120, 0, true),
        (27, 120, 0, true),
        (24, 120, 0, true),
    ];
    p.set("word/styles.xml",format!("{XML_HEAD}<w:styles xmlns:w=\"{W}\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Aptos\" w:hAnsi=\"Aptos\"/><w:color w:val=\"242424\"/><w:sz w:val=\"24\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:before=\"0\" w:after=\"180\" w:line=\"336\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style>{}</w:styles>",headings.iter().enumerate().map(|(i,(size,before,after,bold))|format!("<w:style w:type=\"paragraph\" w:styleId=\"Heading{}\"><w:name w:val=\"Heading {}\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:pPr><w:spacing w:before=\"{before}\" w:after=\"{after}\"/><w:outlineLvl w:val=\"{i}\"/></w:pPr><w:rPr>{}<w:sz w:val=\"{size}\"/></w:rPr></w:style>",i+1,i+1,if *bold {"<w:b/>"} else {""})).collect::<String>()));
    p
}

pub(crate) fn wrap_fragment(xml: &str, namespaces: &str) -> String {
    format!("<root xmlns:w=\"{W}\" xmlns:r=\"{R}\"{namespaces}>{xml}</root>")
}
fn extract_props(xml: &str, namespaces: &str) -> Option<String> {
    let wrapped = wrap_fragment(xml, namespaces);
    let d = parse(&wrapped).ok()?;
    let p = d
        .root_element()
        .children()
        .find(|n| n.has_tag_name((W, "p")))?;
    let props = p.children().find(|n| n.has_tag_name((W, "pPr")))?;
    Some(raw(&wrapped, props).into())
}
pub(crate) fn set_prop(props: &str, namespaces: &str, name: &str, value: String) -> String {
    // Replace the entire property (including compound numPr), not just its
    // first self-closing descendant. Keep unmodeled properties and schema order.
    let wrapped = wrap_fragment(props, namespaces);
    let Ok(d) = parse(&wrapped) else {
        return props.into();
    };
    let order = [
        "pStyle",
        "keepNext",
        "keepLines",
        "pageBreakBefore",
        "framePr",
        "widowControl",
        "numPr",
        "suppressLineNumbers",
        "pBdr",
        "shd",
        "tabs",
        "suppressAutoHyphens",
        "kinsoku",
        "wordWrap",
        "overflowPunct",
        "topLinePunct",
        "autoSpaceDE",
        "autoSpaceDN",
        "bidi",
        "adjustRightInd",
        "snapToGrid",
        "spacing",
        "ind",
        "contextualSpacing",
        "mirrorIndents",
        "suppressOverlap",
        "jc",
        "textDirection",
        "textAlignment",
        "textboxTightWrap",
        "outlineLvl",
        "divId",
        "cnfStyle",
        "rPr",
        "sectPr",
        "pPrChange",
    ];
    let rank = |n: &str| order.iter().position(|s| *s == n).unwrap_or(usize::MAX);
    let mut children = vec![];
    if let Some(p) = d
        .root_element()
        .children()
        .find(|n| n.has_tag_name((W, "pPr")))
    {
        for n in p.children() {
            if n.has_tag_name((W, name)) {
                continue;
            }
            children.push((rank(n.tag_name().name()), raw(&wrapped, n).to_string()));
        }
    }
    if !value.is_empty() {
        children.push((rank(name), value));
    }
    children.sort_by_key(|(rank, _)| *rank);
    let inner = children.into_iter().map(|(_, xml)| xml).collect::<String>();
    if let Some(p) = d
        .root_element()
        .children()
        .find(|n| n.has_tag_name((W, "pPr")))
    {
        let patched = replace_inner(&wrapped, p, &inner);
        if let Ok(dom) = parse(&patched) {
            if let Some(p) = dom
                .root_element()
                .children()
                .find(|n| n.has_tag_name((W, "pPr")))
            {
                return standalone_xml(&patched, p);
            }
        }
    }
    format!("<w:pPr xmlns:w=\"{W}\">{inner}</w:pPr>")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formatted_tabs_are_word_elements_and_round_trip_with_marks() {
        let doc = json!({"type":"doc","content":[{"type":"paragraph","content":[
            {"type":"text","text":"\tA\t\tB\t","marks":[{"type":"bold"},{"type":"link","attrs":{"href":"https://example.org"}}]}
        ]}]});
        let pkg = Package::from_bytes(export(&doc, None, None).unwrap()).unwrap();
        let xml = parse(pkg.text("word/document.xml").unwrap()).unwrap();
        assert_eq!(
            xml.descendants()
                .filter(|n| n.has_tag_name((W, "tab")))
                .count(),
            4
        );
        assert!(!xml
            .descendants()
            .filter(|n| n.has_tag_name((W, "t")))
            .any(|n| n.text().unwrap_or("").contains('\t')));
        let imported = import(&pkg).unwrap().0;
        let runs = imported["content"][0]["content"].as_array().unwrap();
        assert_eq!(
            runs.iter()
                .map(|r| r["text"].as_str().unwrap())
                .collect::<String>(),
            "\tA\t\tB\t"
        );
        assert!(runs
            .iter()
            .all(|r| r["marks"] == doc["content"][0]["content"][0]["marks"]));
    }
    #[test]
    fn word_package_preservation_and_edit() {
        let p = Package::from_bytes(include_bytes!("../../test/fixtures/source.docx").to_vec())
            .unwrap();
        let (doc, _) = import(&p).unwrap();
        assert_eq!(export(&doc, Some(&p), Some(&doc)).unwrap(), p.original);
        let mut edited = doc.clone();
        edited["content"][1]["content"][0]["text"] = json!("Net revenue ");
        edited["content"][1]["attrs"]["textAlign"] = json!("center");
        let out = Package::from_bytes(export(&edited, Some(&p), Some(&doc)).unwrap()).unwrap();
        for (name, bytes) in &p.entries {
            if name != "word/document.xml" {
                assert_eq!(&out.entries[name], bytes, "changed {name}");
            }
        }
        let reimport = import(&out).unwrap().0;
        assert!(crate::workspace::document_text(&reimport).contains("Net revenue"));
        assert_eq!(reimport["content"][1]["attrs"]["textAlign"], "center");
        let dest = std::env::temp_dir().join("folio-verified-edited.docx");
        std::fs::write(dest, &out.original).unwrap();
    }
    #[test]
    fn new_word_file_with_table_and_link() {
        let d = json!({"type":"doc","content":[{"type":"heading","attrs":{"level":1},"content":[{"type":"text","text":"Export verified"}]},{"type":"paragraph","content":[{"type":"text","text":"OpenAI","marks":[{"type":"link","attrs":{"href":"https://openai.com"}}]}]},{"type":"table","content":[{"type":"tableRow","content":[{"type":"tableCell","content":[{"type":"paragraph","content":[{"type":"text","text":"1200"}]}]}]}]}]});
        let bytes = export(&d, None, None).unwrap();
        let pkg = Package::from_bytes(bytes.clone()).unwrap();
        assert!(pkg
            .text("word/_rels/document.xml.rels")
            .unwrap()
            .contains("https://openai.com"));
        import(&pkg).unwrap();
        std::fs::write(std::env::temp_dir().join("folio-verified-new.docx"), bytes).unwrap();
    }
}

fn highlight_color(s: &str) -> String {
    match s {
        "yellow" => "#FFFF00",
        "green" => "#00FF00",
        "cyan" => "#00FFFF",
        "magenta" => "#FF00FF",
        "blue" => "#0000FF",
        "red" => "#FF0000",
        "darkBlue" => "#000080",
        "darkCyan" => "#008080",
        "darkGreen" => "#008000",
        "darkMagenta" => "#800080",
        "darkRed" => "#800000",
        "darkYellow" => "#808000",
        "darkGray" => "#808080",
        "lightGray" => "#C0C0C0",
        "black" => "#000000",
        _ => "#FFFFFF",
    }
    .into()
}
