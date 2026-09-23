//! Explicit section-boundary transactions. Ordinary rich-text edits cannot
//! remove boundaries; this module owns their section-formatting consequences.
use crate::{docx_page_setup as pages, office::*};
use roxmltree::Node;
use serde_json::{json, Value};
use std::collections::BTreeMap;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn blocks<'a, 'i>(body: Node<'a, 'i>) -> Vec<Node<'a, 'i>> {
    body.children()
        .filter(|n| n.is_element() && !n.has_tag_name((W, "sectPr")))
        .collect()
}
fn xml_text_current(p: Node) -> String {
    p.descendants()
        .filter(|n| n.is_element())
        .filter_map(|n| {
            if n.has_tag_name((W, "t")) {
                Some(n.text().unwrap_or("").to_string())
            } else if n.has_tag_name((W, "tab")) {
                Some("\t".into())
            } else if n.has_tag_name((W, "br")) || n.has_tag_name((W, "cr")) {
                Some("\n".into())
            } else {
                None
            }
        })
        .collect()
}
fn paragraph_ready(p: Node) -> Result<(), String> {
    if !p.has_tag_name((W, "p")) {
        return Err(
            "Place the insertion point in a document paragraph outside a table or preserved object"
                .into(),
        );
    }
    if crate::docx_revisions::has_changes(p) {
        return Err(
            "Review this paragraph's tracked changes before changing its section break".into(),
        );
    }
    for n in p.descendants().filter(|n| n.is_element()) {
        if n.ancestors()
            .any(|a| a.has_tag_name((W, "pPr")) || a.has_tag_name((W, "rPr")))
        {
            continue;
        }
        if n.tag_name().namespace() != Some(W)
            || ![
                "p",
                "r",
                "t",
                "tab",
                "br",
                "cr",
                "hyperlink",
                "bookmarkStart",
                "bookmarkEnd",
                "commentRangeStart",
                "commentRangeEnd",
                "commentReference",
                "proofErr",
                "permStart",
                "permEnd",
                "lastRenderedPageBreak",
            ]
            .contains(&n.tag_name().name())
        {
            return Err("This paragraph contains an object that cannot yet be split safely. Choose another paragraph for the section break".into());
        }
    }
    Ok(())
}
fn body_paths(document: &Value) -> Vec<Vec<usize>> {
    fn walk(node: &Value, here: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if matches!(
            node["type"].as_str(),
            Some(
                "doc" | "bulletList" | "orderedList" | "listItem" | "blockquote" | "workbookBlock"
            )
        ) {
            for (i, n) in node["content"].as_array().into_iter().flatten().enumerate() {
                here.push(i);
                walk(n, here, out);
                here.pop();
            }
        } else {
            out.push(here.clone());
        }
    }
    let mut out = vec![];
    walk(document, &mut vec![], &mut out);
    out
}
fn body_index(document: &Value, path: &[usize]) -> Result<(usize, usize), String> {
    let paths = body_paths(document);
    paths
        .iter()
        .position(|p| p == path)
        .map(|i| (i, paths.len()))
        .ok_or("The selected paragraph no longer exists".into())
}
/// The newline edit is explicit so repeated paragraphs retain exact history
/// positions. Re-imported identifiers are not used to infer a text difference.
pub fn journal(pkg: &Package, document: &Value, r: &Value) -> Result<Value, String> {
    let index = crate::document_bookmarks::Index::new(document);
    let xml = pkg.text("word/document.xml")?;
    let dom = parse(xml)?;
    let body = pages::document_body(&dom)?;
    let (path, offset, deleted) = if r["action"] == "insert" {
        let path = r["path"]
            .as_array()
            .ok_or("Choose a paragraph")?
            .iter()
            .map(|v| {
                v.as_u64()
                    .map(|n| n as usize)
                    .ok_or("Invalid paragraph path")
            })
            .collect::<Result<Vec<_>, _>>()?;
        (
            path,
            r["offset"].as_u64().ok_or("Choose an insertion point")? as usize,
            false,
        )
    } else {
        let section = r["section"].as_u64().ok_or("Choose a section break")? as usize;
        let node = pages::sections(body)
            .get(section)
            .copied()
            .flatten()
            .ok_or("Section break missing")?;
        let p = node
            .parent()
            .and_then(|n| n.parent())
            .ok_or("Section paragraph missing")?;
        if !p
            .next_siblings()
            .skip(1)
            .find(|n| n.is_element())
            .is_some_and(|n| n.has_tag_name((W, "p")))
        {
            return Ok(json!([]));
        }
        let source = blocks(body);
        let paths = body_paths(document);
        if source.len() != paths.len() {
            return Err(
                "The preserved document layout needs additional section-break support".into(),
            );
        }
        let block = source
            .iter()
            .position(|n| *n == p)
            .ok_or("Section paragraph missing")?;
        (
            paths[block].clone(),
            xml_text_current(p).encode_utf16().count(),
            true,
        )
    };
    let paragraph = index
        .paragraphs
        .iter()
        .find(|p| p.path == path)
        .ok_or("The section break does not map to editable document text")?;
    let at = paragraph.start + offset;
    if at > paragraph.end || !index.boundary(at) {
        return Err("Invalid section-break text position".into());
    }
    Ok(json!([{"start":at,"end":at+usize::from(deleted),"text":if deleted{""}else{"\n"}}]))
}
fn selected<'a, 'i>(
    pkg: &'a Package,
    document: &Value,
    r: &Value,
    d: &'a roxmltree::Document<'i>,
) -> Result<(usize, usize, Node<'a, 'i>), String> {
    let path = r["path"]
        .as_array()
        .ok_or("Select a paragraph")?
        .iter()
        .map(|v| {
            v.as_u64()
                .map(|v| v as usize)
                .ok_or("Invalid paragraph path".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut node = document;
    for &i in &path {
        if matches!(
            node["type"].as_str(),
            Some(
                "table"
                    | "tableRow"
                    | "tableCell"
                    | "tableHeader"
                    | "workbookBlock"
                    | "officeBlock"
            )
        ) {
            return Err(
                "Section breaks belong in the document body, outside tables and linked blocks"
                    .into(),
            );
        }
        node = node["content"]
            .get(i)
            .ok_or("The selected paragraph no longer exists")?;
    }
    if !matches!(node["type"].as_str(), Some("paragraph" | "heading")) {
        return Err("Place the insertion point in a document paragraph".into());
    }
    let (index, count) = body_index(document, &path)?;
    let body = pages::document_body(d)?;
    if blocks(body).len() != count {
        return Err("The preserved document layout needs additional section-break support".into());
    }
    let p = *blocks(body)
        .get(index)
        .ok_or("The selected paragraph changed")?;
    paragraph_ready(p)?;
    let text = crate::workspace::document_text(node);
    if xml_text_current(p) != text {
        return Err(
            "The document's preserved paragraph layout needs additional section-break support"
                .into(),
        );
    }
    let offset = r["offset"]
        .as_u64()
        .ok_or("Choose a text insertion point")? as usize;
    let units = text.encode_utf16().collect::<Vec<_>>();
    if offset > units.len() || String::from_utf16(&units[..offset]).is_err() {
        return Err("The insertion point must be between complete characters".into());
    }
    let catalog = pages::read(pkg)?;
    let section = catalog["sections"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| index <= s["lastBlock"].as_u64().unwrap_or(0) as usize)
        .ok_or("Section not found")?;
    Ok((index, section, p))
}
pub fn context(pkg: &Package, document: &Value, r: &Value) -> Result<Value, String> {
    let mut out = pages::read(pkg)?;
    out["protected"] = json!(crate::docx_revisions::protection(pkg)?);
    let xml = pkg.text("word/document.xml")?;
    let d = parse(xml)?;
    match selected(pkg, document, r, &d) {
        Ok((block, section, p)) => {
            out["targetBlock"] = json!(block);
            out["targetSection"] = json!(section);
            out["targetKey"] = json!(fingerprint(raw(xml, p).as_bytes()));
            out["canInsert"] = json!(true);
        }
        Err(reason) => {
            out["canInsert"] = json!(false);
            out["insertReason"] = json!(reason);
        }
    }
    Ok(out)
}
fn replace_properties(paragraph: &str, section: Option<&str>) -> Result<String, String> {
    let d = parse(paragraph)?;
    let p = d.root_element();
    let props = child(p, "pPr")
        .map(|n| scoped_record(paragraph, n))
        .unwrap_or_else(|| format!("<w:pPr xmlns:w=\"{W}\"/>"));
    let pd = parse(&props)?;
    let root = pd.root_element();
    let inner = root
        .children()
        .filter(|n| !n.has_tag_name((W, "sectPr")))
        .map(|n| raw(&props, n))
        .collect::<String>()
        + section.unwrap_or("");
    let changed = replace_inner(&props, root, &inner);
    if let Some(old) = child(p, "pPr") {
        Ok(replace_ranges(paragraph, vec![(old.range(), changed)]))
    } else {
        let content = p.children().map(|n| raw(paragraph, n)).collect::<String>();
        Ok(replace_inner(paragraph, p, &(changed + &content)))
    }
}
fn without_references(section: &str) -> Result<String, String> {
    let d = parse(section)?;
    Ok(replace_ranges(
        section,
        d.root_element()
            .children()
            .filter(|n| {
                n.has_tag_name((W, "headerReference")) || n.has_tag_name((W, "footerReference"))
            })
            .map(|n| (n.range(), String::new()))
            .collect(),
    ))
}
fn split_paragraph(source: &str, offset: usize) -> Result<(String, String), String> {
    fn split(
        xml: &str,
        n: Node,
        remaining: &mut usize,
        past_split: &mut bool,
    ) -> Result<(String, String), String> {
        if n.has_tag_name((W, "t")) {
            let text = n.text().unwrap_or("");
            if text.is_empty() {
                return Ok(if *past_split {
                    (String::new(), scoped_record(xml, n))
                } else {
                    (scoped_record(xml, n), String::new())
                });
            }
            let len = text.encode_utf16().count();
            let take = (*remaining).min(len);
            let units = text.encode_utf16().collect::<Vec<_>>();
            let left = String::from_utf16(&units[..take])
                .map_err(|_| "Cannot split a Unicode character")?;
            let right = String::from_utf16(&units[take..])
                .map_err(|_| "Cannot split a Unicode character")?;
            *remaining -= take;
            if !right.is_empty() {
                *past_split = true;
            }
            let shell = scoped_record(xml, n);
            let td = parse(&shell)?;
            let mut edits = td
                .root_element()
                .attributes()
                .filter(|a| {
                    a.namespace() == Some("http://www.w3.org/XML/1998/namespace")
                        && a.name() == "space"
                })
                .map(|a| (a.range(), String::new()))
                .collect::<Vec<_>>();
            let at = shell[1..]
                .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .unwrap()
                + 1;
            edits.push((at..at, " xml:space=\"preserve\"".into()));
            let shell = replace_ranges(&shell, edits);
            let td = parse(&shell)?;
            let make = |s: &str| {
                if s.is_empty() {
                    String::new()
                } else {
                    replace_inner(&shell, td.root_element(), &esc(s))
                }
            };
            return Ok((make(&left), make(&right)));
        }
        if ["tab", "br", "cr"].iter().any(|t| n.has_tag_name((W, *t))) {
            if *remaining > 0 {
                *remaining -= 1;
                return Ok((scoped_record(xml, n), String::new()));
            }
            *past_split = true;
            return Ok((String::new(), scoped_record(xml, n)));
        }
        if ["p", "r", "hyperlink"]
            .iter()
            .any(|t| n.has_tag_name((W, *t)))
        {
            let mut properties = String::new();
            let mut left = String::new();
            let mut right = String::new();
            for c in n.children() {
                if c.has_tag_name((W, "pPr")) || c.has_tag_name((W, "rPr")) {
                    properties.push_str(&scoped_record(xml, c));
                    continue;
                }
                let (a, b) = split(xml, c, remaining, past_split)?;
                left.push_str(&a);
                right.push_str(&b);
            }
            let shell = scoped_record(xml, n);
            let d = parse(&shell)?;
            let root = d.root_element();
            let wrap = |s: String| {
                if s.is_empty() && !n.has_tag_name((W, "p")) {
                    String::new()
                } else {
                    replace_inner(&shell, root, &(properties.clone() + &s))
                }
            };
            return Ok((wrap(left), wrap(right)));
        }
        let value = if n.is_element() {
            scoped_record(xml, n)
        } else {
            raw(xml, n).into()
        };
        if !*past_split {
            Ok((value, String::new()))
        } else {
            Ok((String::new(), value))
        }
    }
    let d = parse(source)?;
    let mut remaining = offset;
    let (left, mut right) = split(source, d.root_element(), &mut remaining, &mut false)?;
    if remaining != 0 {
        return Err("The insertion point exceeds the paragraph".into());
    }
    // Word's durable paragraph identifiers may not be duplicated by a split.
    let d = parse(&right)?;
    let edits = d
        .root_element()
        .attributes()
        .filter(|a| {
            a.namespace() == Some("http://schemas.microsoft.com/office/word/2010/wordml")
                && ["paraId", "textId"].contains(&a.name())
        })
        .map(|a| (a.range(), String::new()))
        .collect();
    right = replace_ranges(&right, edits);
    Ok((left, right))
}
fn section_source(xml: &str, node: Option<Node>) -> String {
    node.map(|n| scoped_record(xml, n))
        .unwrap_or_else(|| format!("<w:sectPr xmlns:w=\"{W}\"/>"))
}
fn effective_references(
    xml: &str,
    sections: &[Option<Node>],
    through: usize,
) -> BTreeMap<(String, String), String> {
    let mut out = BTreeMap::new();
    for section in sections.iter().take(through + 1).flatten() {
        for c in section.children().filter(|n| {
            n.has_tag_name((W, "headerReference")) || n.has_tag_name((W, "footerReference"))
        }) {
            out.insert(
                (
                    c.tag_name().name().into(),
                    c.attribute((W, "type")).unwrap_or("default").into(),
                ),
                scoped_record(xml, c),
            );
        }
    }
    out
}
pub fn apply(
    pkg: &Package,
    document: &Value,
    r: &Value,
    tracking: Option<&Value>,
) -> Result<Package, String> {
    if crate::docx_revisions::protection(pkg)? {
        return Err("Unlock document protection before changing section breaks".into());
    }
    if tracking.is_some_and(|s| s["enabled"] == true) {
        return Err("Recording section-break insertions and deletions is not supported yet. Keep recording on to retain the document unchanged, or turn it off explicitly before editing the break".into());
    }
    let xml = pkg.text("word/document.xml")?;
    let d = parse(xml)?;
    let body = pages::document_body(&d)?;
    let nodes = pages::sections(body);
    let catalog = pages::read(pkg)?;
    let output = match r["action"].as_str() {
        Some("insert") => {
            let (block, index, p) = selected(pkg, document, r, &d)?;
            if r["targetKey"] != fingerprint(raw(xml, p).as_bytes()) {
                return Err("The paragraph changed. Reopen Section breaks".into());
            }
            let start = r["start"]
                .as_str()
                .filter(|s| ["nextPage", "continuous", "evenPage", "oddPage"].contains(s))
                .ok_or("Choose a section break type")?;
            let old = nodes[index];
            if old.is_some_and(|s| crate::docx_revisions::has_changes(s)) {
                return Err(
                    "Review the section's pending formatting before inserting a break".into(),
                );
            }
            let left_section = section_source(xml, old);
            let mut geometry = catalog["sections"][index]["geometry"].clone();
            geometry["start"] = json!(start);
            let mut patch = json!({"start":start});
            if catalog["sections"][index]["inheritedPageGeometry"] == true {
                if catalog["sections"][index]["sizeExplicit"] == false {
                    patch["width"] = geometry["width"].clone();
                }
                if catalog["sections"][index]["marginsExplicit"] == false {
                    patch["left"] = geometry["left"].clone();
                }
            }
            let right_section = pages::changed_section(
                &without_references(&left_section)?,
                &patch,
                &geometry,
                None,
                0,
            )?;
            let updated = if let Some(s) = old {
                replace_ranges(xml, vec![(s.range(), right_section)])
            } else {
                let inner =
                    body.children().map(|n| raw(xml, n)).collect::<String>() + &right_section;
                replace_inner(xml, body, &inner)
            };
            let dom = parse(&updated)?;
            let p = blocks(pages::document_body(&dom)?)[block];
            let (left, right) = split_paragraph(
                &scoped_record(&updated, p),
                r["offset"].as_u64().unwrap() as usize,
            )?;
            let left = replace_properties(&left, Some(&left_section))?;
            replace_ranges(&updated, vec![(p.range(), left + &right)])
        }
        Some("remove") => {
            let index = r["section"].as_u64().ok_or("Choose a section break")? as usize;
            let row = catalog["sections"]
                .get(index)
                .ok_or("Section no longer exists")?;
            if r["sectionKey"] != row["key"] {
                return Err("Sections changed. Reopen Section breaks".into());
            }
            if index + 1 >= nodes.len() {
                return Err("The final document section cannot be removed".into());
            }
            let section = nodes[index].ok_or("The section break is missing")?;
            let p = section
                .parent()
                .and_then(|n| n.parent())
                .filter(|n| n.has_tag_name((W, "p")) && n.parent() == Some(body))
                .ok_or("This preserved section boundary needs additional support")?;
            paragraph_ready(p)?;
            if [nodes[index], nodes[index + 1]]
                .into_iter()
                .flatten()
                .any(crate::docx_revisions::has_changes)
            {
                return Err("Review pending section formatting before removing this break".into());
            }
            let following = nodes[index + 1];
            // Materialize the following section's effective headers/footers so
            // removal of its predecessor does not change inherited content.
            let mut next_section = without_references(&section_source(xml, following))?;
            let references = effective_references(xml, &nodes, index + 1);
            let refs = ["headerReference", "footerReference"]
                .into_iter()
                .flat_map(|tag| {
                    references
                        .iter()
                        .filter(move |((name, _), _)| name == tag)
                        .map(|(_, xml)| xml.clone())
                })
                .collect::<String>();
            let nd = parse(&next_section)?;
            let root = nd.root_element();
            let inner = root
                .children()
                .map(|n| raw(&next_section, n))
                .collect::<String>();
            next_section = replace_inner(&next_section, root, &(refs + &inner));
            let mut prepared = if let Some(s) = following {
                replace_ranges(xml, vec![(s.range(), next_section)])
            } else {
                let inner =
                    body.children().map(|n| raw(xml, n)).collect::<String>() + &next_section;
                replace_inner(xml, body, &inner)
            };
            let dom = parse(&prepared)?;
            let b = pages::document_body(&dom)?;
            let boundary = pages::sections(b)[index].unwrap();
            let p = boundary.parent().unwrap().parent().unwrap();
            let next = p
                .next_siblings()
                .skip(1)
                .find(|n| n.is_element() && !n.has_tag_name((W, "sectPr")));
            if let Some(next) = next.filter(|n| n.has_tag_name((W, "p"))) {
                paragraph_ready(next)?;
                let prefix = p
                    .children()
                    .filter(|n| !n.has_tag_name((W, "pPr")))
                    .map(|n| {
                        if n.is_element() {
                            scoped_record(&prepared, n)
                        } else {
                            raw(&prepared, n).into()
                        }
                    })
                    .collect::<String>();
                let props = child(next, "pPr")
                    .map(|n| scoped_record(&prepared, n))
                    .unwrap_or_default();
                let suffix = next
                    .children()
                    .filter(|n| !n.has_tag_name((W, "pPr")))
                    .map(|n| {
                        if n.is_element() {
                            scoped_record(&prepared, n)
                        } else {
                            raw(&prepared, n).into()
                        }
                    })
                    .collect::<String>();
                let shell = scoped_record(&prepared, next);
                let sd = parse(&shell)?;
                let joined = replace_inner(&shell, sd.root_element(), &(props + &prefix + &suffix));
                prepared =
                    replace_ranges(&prepared, vec![(p.range().start..next.range().end, joined)]);
            } else {
                prepared = replace_ranges(&prepared, vec![(boundary.range(), String::new())]);
            }
            prepared
        }
        _ => return Err("Choose Insert or Remove section break".into()),
    };
    parse(&output)?;
    let mut result = pkg.clone();
    result.set("word/document.xml", output);
    result.original = result.bytes()?;
    Ok(result)
}
