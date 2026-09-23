//! Review Word's retained revision records without flattening their XML payload.
use crate::office::*;
use roxmltree::Node;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const W14: &str = "http://schemas.microsoft.com/office/word/2010/wordml";
const PROPERTIES: &[&str] = &[
    "rPrChange",
    "pPrChange",
    "tblPrChange",
    "tblPrExChange",
    "tblGridChange",
    "trPrChange",
    "tcPrChange",
    "sectPrChange",
];
pub(crate) fn is_property(n: Node) -> bool {
    n.tag_name().namespace() == Some(W) && PROPERTIES.contains(&n.tag_name().name())
}
fn candidate(n: Node) -> bool {
    n.is_element()
        && ((n.tag_name().namespace() == Some(W)
            && (PROPERTIES.contains(&n.tag_name().name())
                || [
                    "ins",
                    "del",
                    "moveFrom",
                    "moveTo",
                    "cellIns",
                    "cellDel",
                    "cellMerge",
                    "numberingChange",
                    "customXmlInsRangeStart",
                    "customXmlDelRangeStart",
                    "customXmlMoveFromRangeStart",
                    "customXmlMoveToRangeStart",
                ]
                .contains(&n.tag_name().name())))
            || (n.tag_name().namespace() == Some(W14)
                && ["conflictIns", "conflictDel"].contains(&n.tag_name().name())))
}
fn active(n: Node) -> bool {
    candidate(n) && !n.ancestors().skip(1).any(is_property)
}
pub fn has_changes(n: Node) -> bool {
    n.descendants().any(active)
}
fn parent_is(n: Node, tag: &str) -> bool {
    n.parent().is_some_and(|p| p.has_tag_name((W, tag)))
}
fn paragraph_mark(n: Node) -> bool {
    parent_is(n, "rPr") && n.parent().is_some_and(|p| parent_is(p, "pPr"))
}
fn body_block(n: Node) -> Option<String> {
    let body = n.ancestors().find(|n| n.has_tag_name((W, "body")))?;
    body.children()
        .filter(|n| n.is_element() && !n.has_tag_name((W, "sectPr")))
        .enumerate()
        .find(|(_, p)| p.range().start <= n.range().start && p.range().end >= n.range().end)
        .map(|(i, _)| format!("word-{i}"))
}
fn story(path: &str, root: Node) -> String {
    match root.tag_name().name() {
        "document" => "Document".into(),
        "hdr" => "Header".into(),
        "ftr" => "Footer".into(),
        "footnotes" => "Footnotes".into(),
        "endnotes" => "Endnotes".into(),
        "comments" => "Comments".into(),
        "styles" => "Styles".into(),
        "numbering" => "Numbering".into(),
        _ => path.into(),
    }
}
fn parts(pkg: &Package) -> Vec<String> {
    // Content types identify real Word story parts, including alternate paths.
    let Ok(xml) = pkg.text("[Content_Types].xml") else {
        return vec![];
    };
    let Ok(dom) = parse(xml) else { return vec![] };
    let mut result = dom
        .root_element()
        .children()
        .filter_map(|n| {
            let kind = attr(n, "ContentType")?;
            let accepted = [
                "document.main+xml",
                "template.main+xml",
                "header+xml",
                "footer+xml",
                "footnotes+xml",
                "endnotes+xml",
                "comments+xml",
                "styles+xml",
                "numbering+xml",
            ]
            .iter()
            .any(|k| {
                kind == format!(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.{k}"
                )
            });
            accepted.then(|| {
                attr(n, "PartName")
                    .unwrap_or("")
                    .trim_start_matches('/')
                    .to_string()
            })
        })
        .filter(|p| pkg.entries.contains_key(p))
        .collect::<Vec<_>>();
    result.sort_by_key(|p| (p != "word/document.xml", p.clone()));
    result.dedup();
    result
}
pub fn preview(n: Node) -> Value {
    let mut spans: Vec<Value> = vec![];
    fn push(spans: &mut Vec<Value>, kind: &str, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = spans.last_mut().filter(|m| m["kind"] == kind) {
            last["text"] = json!(format!("{}{text}", last["text"].as_str().unwrap()));
        } else {
            spans.push(json!({"kind":kind,"text":text}));
        }
    }
    for p in n.descendants().filter(|p| p.is_element()) {
        if p.ancestors().skip(1).any(is_property) {
            continue;
        }
        let kind = if p
            .ancestors()
            .any(|a| a.has_tag_name((W, "del")) || a.has_tag_name((W, "moveFrom")))
        {
            "delete"
        } else if p
            .ancestors()
            .any(|a| a.has_tag_name((W, "ins")) || a.has_tag_name((W, "moveTo")))
        {
            "insert"
        } else {
            "unchanged"
        };
        if p.has_tag_name((W, "t")) || p.has_tag_name((W, "delText")) {
            push(&mut spans, kind, p.text().unwrap_or(""));
        } else if p.has_tag_name((W, "tab")) {
            push(&mut spans, kind, "\t");
        } else if p.has_tag_name((W, "br")) || p.has_tag_name((W, "cr")) {
            push(&mut spans, kind, "\n");
        } else if p != n && p.has_tag_name((W, "p")) && !spans.is_empty() {
            push(&mut spans, "unchanged", "\n");
        }
    }
    json!(spans)
}
fn paragraph<'a, 'i>(n: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    n.ancestors().find(|p| p.has_tag_name((W, "p")))
}
fn merge_target<'a, 'i>(n: Node<'a, 'i>) -> Result<(Node<'a, 'i>, Node<'a, 'i>), String> {
    let p = paragraph(n).ok_or("Paragraph mark has no paragraph")?;
    let next = p
        .next_siblings()
        .skip(1)
        .find(|p| {
            p.is_element() && !(crate::docx_moves::is_move(n) && crate::docx_moves::is_boundary(*p))
        })
        .filter(|p| p.has_tag_name((W, "p")))
        .ok_or(
            "Joining this paragraph across an object or story boundary needs additional support",
        )?;
    if [p, next]
        .iter()
        .any(|p| p.descendants().any(|n| n.has_tag_name((W, "sectPr"))))
    {
        return Err("Joining tracked section boundaries needs additional support".into());
    }
    Ok((p, next))
}
fn effective_tag<'a>(n: Node<'a, '_>) -> &'a str {
    match n.tag_name().name() {
        "moveFrom" => "del",
        "moveTo" => "ins",
        other => other,
    }
}
fn whole_moved_paragraph(n: Node) -> bool {
    let Some(p) = paragraph(n) else { return false };
    !p.descendants().any(|c| c.has_tag_name((W, "sectPr")))
        && p.children().all(|c| {
            if c.is_text() {
                c.text().unwrap_or("").trim().is_empty()
            } else {
                c.has_tag_name((W, "pPr"))
                    || c.tag_name() == n.tag_name()
                    || crate::docx_moves::is_boundary(c)
            }
        })
}
fn support(n: Node, accept: bool) -> (bool, String) {
    let tag = effective_tag(n);
    if is_property(n) {
        let name = tag.trim_end_matches("Change");
        if parent_is(n, name)
            && n.children().filter(|p| p.is_element()).count() == 1
            && child(n, name).is_some()
        {
            return (true, String::new());
        }
        return (
            false,
            "The earlier formatting snapshot is incomplete".into(),
        );
    }
    if matches!(tag, "ins" | "del") && n.tag_name().namespace() == Some(W) {
        if paragraph_mark(n) {
            if (tag == "del") != accept {
                return (true, String::new());
            }
            if crate::docx_moves::is_move(n) && whole_moved_paragraph(n) {
                return (true, String::new());
            }
            return match merge_target(n) {
                Ok(_) => (true, String::new()),
                Err(e) => (false, e),
            };
        }
        if parent_is(n, "trPr") {
            let table = n.ancestors().find(|p| p.has_tag_name((W, "tbl")));
            if (tag == "del") == accept
                && table.is_some_and(|t| t.descendants().any(|p| p.has_tag_name((W, "vMerge"))))
            {
                return (
                    false,
                    "Reviewing tracked rows with vertical merges needs additional support".into(),
                );
            }
            return (true, String::new());
        }
        if n.children().any(|p| p.is_element())
            && !n
                .parent()
                .is_some_and(|p| p.tag_name().name().ends_with("Pr"))
        {
            return (true, String::new());
        }
    }
    (
        false,
        "This revision type is preserved and needs additional review support".into(),
    )
}
fn kind(n: Node) -> String {
    let tag = n.tag_name().name();
    if crate::docx_moves::is_move(n) {
        return match (tag, paragraph_mark(n)) {
            ("moveFrom", true) => "Moved paragraph from",
            ("moveTo", true) => "Moved paragraph to",
            ("moveFrom", false) => "Moved from",
            _ => "Moved to",
        }
        .into();
    }
    if is_property(n) {
        return match tag {
            "rPrChange" => "Text formatting",
            "pPrChange" => "Paragraph formatting",
            "sectPrChange" => "Section formatting",
            _ => "Table formatting",
        }
        .into();
    }
    if paragraph_mark(n) {
        return if tag == "ins" {
            "Inserted paragraph break"
        } else {
            "Deleted paragraph break"
        }
        .into();
    }
    if parent_is(n, "trPr") {
        return if tag == "ins" {
            "Inserted table row"
        } else {
            "Deleted table row"
        }
        .into();
    }
    match tag {
        "ins" => "Inserted content",
        "del" => "Deleted content",
        "moveFrom" | "moveTo" => "Moved content",
        _ => "Other tracked change",
    }
    .into()
}
fn format_changes(n: Node) -> Value {
    if !is_property(n) {
        return json!([]);
    }
    fn label(name: &str) -> String {
        match name {
            "b" => "Bold",
            "i" => "Italic",
            "u" => "Underline",
            "strike" => "Strikethrough",
            "dstrike" => "Double strikethrough",
            "bCs" => "Bold (complex script)",
            "iCs" => "Italic (complex script)",
            "sz" => "Font size",
            "szCs" => "Font size (complex script)",
            "rFonts" => "Fonts",
            "color" => "Text color",
            "highlight" => "Highlight",
            "shd" => "Shading",
            "jc" => "Alignment",
            "pStyle" => "Paragraph style",
            "rStyle" => "Character style",
            "spacing" => "Spacing",
            "ind" => "Indentation",
            "tabs" => "Tab stops",
            "keepNext" => "Keep with next",
            "keepLines" => "Keep lines together",
            "pageBreakBefore" => "Page break before",
            "widowControl" => "Widow and orphan control",
            "outlineLvl" => "Outline level",
            "numPr" => "Numbering",
            "caps" => "All capitals",
            "smallCaps" => "Small capitals",
            "vertAlign" => "Text position",
            "vanish" => "Hidden text",
            "lang" => "Language",
            "tblW" => "Table width",
            "tblInd" => "Table indentation",
            "tblLayout" => "Table layout",
            "tblBorders" => "Table borders",
            "tcBorders" => "Cell borders",
            "tcW" => "Cell width",
            "gridSpan" => "Column span",
            "vMerge" => "Vertical merge",
            "gridCol" => "Grid column",
            "pgSz" => "Page size",
            "pgMar" => "Page margins",
            "cols" => "Columns",
            "trHeight" => "Row height",
            "cantSplit" => "Keep row together",
            "vAlign" => "Vertical alignment",
            "tblHeader" => "Repeat heading row",
            "sectPr" => "Section settings",
            "rPr" => "Paragraph mark formatting",
            _ => {
                return name
                    .chars()
                    .enumerate()
                    .flat_map(|(i, c)| {
                        if i > 0 && c.is_uppercase() {
                            vec![' ', c.to_ascii_lowercase()]
                        } else {
                            vec![c]
                        }
                    })
                    .collect()
            }
        }
        .into()
    }
    fn properties(n: Node) -> BTreeMap<String, String> {
        fn measurement(value: &str) -> String {
            let numeric = value.parse::<f64>().ok().map(|n| n / 1440.0).or_else(|| {
                [
                    ("in", 1.0),
                    ("pt", 1.0 / 72.0),
                    ("cm", 1.0 / 2.54),
                    ("mm", 1.0 / 25.4),
                    ("pc", 1.0 / 6.0),
                    ("pi", 1.0 / 6.0),
                ]
                .into_iter()
                .find_map(|(unit, scale)| {
                    value
                        .strip_suffix(unit)?
                        .parse::<f64>()
                        .ok()
                        .map(|n| n * scale)
                })
            });
            if let Some(number) = numeric.filter(|n| n.is_finite()) {
                let formatted = format!("{number:.6}");
                format!(
                    "{} in",
                    formatted.trim_end_matches('0').trim_end_matches('.')
                )
            } else {
                format!("Unrecognized measurement: {value}")
            }
        }
        let mut result: BTreeMap<String, String> = BTreeMap::new();
        for p in n.children().filter(|p| p.is_element() && !active(*p)) {
            let name = p.tag_name().name();
            if p.has_tag_name((W, "cols")) && n.has_tag_name((W, "sectPr")) {
                let columns = crate::docx_columns::read(Some(n));
                result.insert("Column count".into(), columns["count"].to_string());
                for (key, label) in [
                    ("equalWidth", "Equal column width"),
                    ("separator", "Line between columns"),
                ] {
                    result.insert(
                        label.into(),
                        if columns[key] == true { "On" } else { "Off" }.into(),
                    );
                }
                if columns["equalWidth"] == true {
                    result.insert(
                        "Column spacing".into(),
                        measurement(&columns["space"].to_string()),
                    );
                } else if let Some(definitions) = columns["columns"].as_array() {
                    for (i, definition) in definitions.iter().enumerate() {
                        result.insert(
                            format!("Column {} width", i + 1),
                            measurement(&definition["width"].to_string()),
                        );
                        if i + 1 < definitions.len() {
                            result.insert(
                                format!("Space after column {}", i + 1),
                                measurement(&definition["space"].to_string()),
                            );
                        }
                    }
                }
                continue;
            }
            if p.has_tag_name((W, "bidi")) && n.has_tag_name((W, "sectPr")) {
                result.insert(
                    "Section direction".into(),
                    if matches!(p.attribute((W, "val")), Some("0" | "false" | "off")) {
                        "Left to right"
                    } else {
                        "Right to left"
                    }
                    .into(),
                );
                continue;
            }
            if p.tag_name().namespace() == Some(W) && ["pgSz", "pgMar"].contains(&name) {
                for a in p.attributes() {
                    let known = match (name, a.namespace(), a.name()) {
                        ("pgSz", Some(W), "w") => Some("Page width"),
                        ("pgSz", Some(W), "h") => Some("Page height"),
                        ("pgMar", Some(W), "top") => Some("Top margin"),
                        ("pgMar", Some(W), "bottom") => Some("Bottom margin"),
                        ("pgMar", Some(W), "left") => Some("Left margin"),
                        ("pgMar", Some(W), "right") => Some("Right margin"),
                        ("pgMar", Some(W), "header") => Some("Header from edge"),
                        ("pgMar", Some(W), "footer") => Some("Footer from edge"),
                        ("pgMar", Some(W), "gutter") => Some("Gutter"),
                        _ => None,
                    };
                    if let Some(label) = known {
                        result.insert(label.into(), measurement(a.value()));
                    } else if name == "pgSz" && a.namespace() == Some(W) && a.name() == "orient" {
                        result.insert(
                            "Orientation".into(),
                            match a.value() {
                                "landscape" => "Landscape".into(),
                                "portrait" => "Portrait".into(),
                                value => format!("Other orientation: {value}"),
                            },
                        );
                    } else if name == "pgSz" && a.namespace() == Some(W) && a.name() == "code" {
                        result.insert(
                            "Printer paper setting".into(),
                            format!("Paper code {}", a.value()),
                        );
                    } else {
                        result.insert(
                            format!("{} — {}", label(name), label(a.name())),
                            a.value().into(),
                        );
                    }
                }
                continue;
            }
            if p.has_tag_name((W, "type")) && n.has_tag_name((W, "sectPr")) {
                result.insert(
                    "Section start".into(),
                    match p.attribute((W, "val")) {
                        Some("nextPage") => "New page".into(),
                        Some("continuous") => "Continuous".into(),
                        Some("evenPage") => "Even page".into(),
                        Some("oddPage") => "Odd page".into(),
                        Some("nextColumn") => "New column".into(),
                        Some(value) => format!("Other section start: {value}"),
                        None => "Unspecified section start".into(),
                    },
                );
                continue;
            }
            let value = if p.children().any(|n| n.is_element()) {
                properties(p)
                    .into_iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            } else if ["sz", "szCs"].contains(&name) {
                attr(p, "val")
                    .and_then(|v| v.parse::<f64>().ok())
                    .map(|n| format!("{} pt", n / 2.0))
                    .unwrap_or("Not specified".into())
            } else if [
                "b",
                "i",
                "bCs",
                "iCs",
                "strike",
                "dstrike",
                "caps",
                "smallCaps",
                "vanish",
                "keepNext",
                "keepLines",
                "pageBreakBefore",
                "widowControl",
                "cantSplit",
                "tblHeader",
            ]
            .contains(&name)
            {
                if matches!(attr(p, "val"), Some("0" | "false" | "off")) {
                    "Off".into()
                } else {
                    "On".into()
                }
            } else if p.attributes().len() == 0 {
                "On".into()
            } else {
                p.attributes()
                    .map(|a| {
                        if a.name() == "val" {
                            a.value().to_string()
                        } else {
                            format!("{}: {}", label(a.name()), a.value())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let label = label(name);
            result
                .entry(label)
                .and_modify(|v| {
                    v.push_str("; ");
                    v.push_str(&value)
                })
                .or_insert(value);
        }
        result
    }
    let Some(previous) = n.children().find(|p| p.is_element()) else {
        return json!([]);
    };
    let mut before = properties(previous);
    let mut after = properties(n.parent().unwrap());
    if n.has_tag_name((W, "pPrChange")) {
        // These are separate tracked properties and are absent by definition
        // from the old paragraph-property snapshot.
        for key in ["Paragraph mark formatting", "Section settings"] {
            before.remove(key);
            after.remove(key);
        }
    }
    if n.has_tag_name((W, "sectPrChange")) {
        for key in ["header reference", "footer reference"] {
            before.remove(key);
            after.remove(key);
        }
    }
    let names = before
        .keys()
        .chain(after.keys())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    json!(names.into_iter().filter(|k|before.get(k)!=after.get(k)).map(|k|json!({"property":k,"before":before.get(&k).map(String::as_str).unwrap_or("Inherited / default"),"after":after.get(&k).map(String::as_str).unwrap_or("Inherited / default")})).collect::<Vec<_>>())
}
fn key(path: &str, xml: &str, n: Node) -> String {
    fingerprint(format!("{path}:{}:{}", n.range().start, raw(xml, n)).as_bytes())
}
pub(crate) fn protection(pkg: &Package) -> Result<bool, String> {
    let types = parse(pkg.text("[Content_Types].xml")?)?;
    for path in types
        .descendants()
        .filter(|n| {
            attr(*n, "ContentType")
                == Some(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml",
                )
        })
        .filter_map(|n| attr(n, "PartName"))
        .map(|p| p.trim_start_matches('/'))
    {
        let Ok(xml) = pkg.text(path) else { continue };
        // Settings can be stored under a relationship-selected nonstandard name.
        if !xml.contains("documentProtection") {
            continue;
        }
        let d = parse(xml)?;
        if d.descendants().any(|n| {
            n.has_tag_name((W, "documentProtection"))
                && matches!(n.attribute((W, "enforcement")), Some("1" | "true" | "on"))
        }) {
            return Ok(true);
        }
    }
    Ok(false)
}
pub fn list(pkg: &Package) -> Result<Value, String> {
    let protected = protection(pkg)?;
    let mut records = vec![];
    for path in parts(pkg) {
        let xml = pkg.text(&path)?;
        let dom = parse(xml)?;
        let moves = crate::docx_moves::Catalog::read(dom.root_element());
        for n in dom.descendants().filter(|n| active(*n)) {
            let capability = |accept| {
                if !crate::docx_moves::is_move(n) {
                    return support(n, accept);
                }
                match moves.group(n) {
                    Err(reason) => (false, reason),
                    Ok(group) => dom
                        .descendants()
                        .filter(|m| {
                            group.members.contains(&m.range().start)
                                && crate::docx_moves::is_move(*m)
                        })
                        .map(|m| support(m, accept))
                        .find(|(ok, _)| !ok)
                        .unwrap_or((true, String::new())),
                }
            };
            let (can_accept, accept_reason) = capability(true);
            let (can_reject, reject_reason) = capability(false);
            let movement = if crate::docx_moves::is_move(n) {
                moves.group(n).ok().map(|g| {
                    let members=dom.descendants().filter(|m|g.members.contains(&m.range().start) && crate::docx_moves::is_move(*m)).collect::<Vec<_>>();
                    let opposite=members.iter().find(|m|m.tag_name()!=n.tag_name());
                    json!({"name":g.name,"role":if n.has_tag_name((W,"moveFrom")){"source"}else{"destination"},
                        "memberKeys":members.iter().map(|m|key(&path,xml,*m)).collect::<Vec<_>>(),
                        "counterpartKey":opposite.map(|m|key(&path,xml,*m)),"memberCount":members.len()})
                })
            } else {
                None
            };
            let container = n
                .ancestors()
                .find(|p| {
                    p.has_tag_name((W, "p"))
                        || p.has_tag_name((W, "tr"))
                        || p.has_tag_name((W, "tbl"))
                })
                .unwrap_or(n);
            records.push(json!({"key":key(&path,xml,n),"part":path,"story":story(&path,dom.root_element()),"sourceId":n.attribute((W,"id")),
                "author":n.attribute((W,"author")).unwrap_or(""),"date":n.attribute((W,"date")).unwrap_or(""),"kind":kind(n),"tag":n.tag_name().name(),
                "preview":preview(container),"change":preview(n),"formatChanges":format_changes(n),"officeId":body_block(n),"canAccept":can_accept&&!protected,"canReject":can_reject&&!protected,
                "acceptReason":if protected{"This document has enforced editing protection. Unlock it in Word before reviewing changes.".into()}else{accept_reason},
                "rejectReason":if protected{"This document has enforced editing protection. Unlock it in Word before reviewing changes.".into()}else{reject_reason},
                "containsChanges":n.descendants().skip(1).any(active),"move":movement}));
        }
    }
    Ok(json!({"revisions":records,"protected":protected}))
}
fn canonical(n: Node, out: &mut String) {
    if n.is_text() {
        out.push_str(&format!("T{:?}", n.text().unwrap_or("")));
        return;
    }
    if !n.is_element() {
        return;
    }
    out.push_str(&format!(
        "E{:?}:{:?}",
        n.tag_name().namespace(),
        n.tag_name().name()
    ));
    let mut a = n
        .attributes()
        .map(|a| (a.namespace(), a.name(), a.value()))
        .collect::<Vec<_>>();
    a.sort();
    out.push_str(&format!("{a:?}"));
    for c in n.children() {
        canonical(c, out);
    }
    out.push('/');
}
/// Generic paragraph editing must not silently erase still-pending source edits.
pub fn check_retained(before: &str, after: &str) -> Result<(), String> {
    let original = parse(before)?;
    let exported = parse(after)?;
    crate::docx_moves::Catalog::read(original.root_element())
        .check_after(original.root_element(), exported.root_element())?;
    fn signatures(xml: &str) -> Result<BTreeMap<String, usize>, String> {
        let dom = parse(xml)?;
        let mut result = BTreeMap::new();
        for n in dom.descendants().filter(|n| active(*n)) {
            let mut s = String::new();
            canonical(n, &mut s);
            *result.entry(fingerprint(s.as_bytes())).or_default() += 1;
        }
        Ok(result)
    }
    let before = signatures(before)?;
    if before.is_empty() {
        return Ok(());
    }
    let after = signatures(after)?;
    if before
        .iter()
        .any(|(k, v)| after.get(k).copied().unwrap_or(0) < *v)
    {
        return Err("Review tracked changes in this content before replacing or removing it. Nothing was exported.".into());
    }
    Ok(())
}
// These temporary identities survive scoped XML rewrites and are consumed before
// returning a package. Source byte offsets cannot identify nested edits reliably.
const TARGET: &str = "folioReviewTemporaryTarget";
fn unwrapped(xml: &str, n: Node, restore_deleted: bool) -> Result<String, String> {
    let mut result = String::new();
    for child in n.children() {
        let mut fragment = if child.is_element() {
            scoped_record(xml, child)
        } else {
            raw(xml, child).into()
        };
        if restore_deleted && child.is_element() {
            let dom = parse(&fragment)?;
            let mut edits = vec![];
            for text in dom.descendants().filter(|p| {
                (p.has_tag_name((W, "delText")) || p.has_tag_name((W, "delInstrText")))
                    && !p
                        .ancestors()
                        .skip(1)
                        .any(|a| a.has_tag_name((W, "del")) || a.has_tag_name((W, "moveFrom")))
            }) {
                let name = if text.tag_name().name() == "delText" {
                    "t"
                } else {
                    "instrText"
                };
                // Preserve the existing namespace prefix, attributes and escaping.
                let source = raw(&fragment, text);
                let old = source[1..]
                    .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .next()
                    .unwrap();
                let replacement = old
                    .rsplit_once(':')
                    .map(|(prefix, _)| format!("{prefix}:{name}"))
                    .unwrap_or(name.into());
                let mut renamed =
                    source.replacen(&format!("<{old}"), &format!("<{replacement}"), 1);
                if let Some(end) = renamed.rfind(&format!("</{old}>")) {
                    renamed.replace_range(end.., &format!("</{replacement}>"));
                }
                edits.push((text.range(), renamed));
            }
            fragment = replace_ranges(&fragment, edits);
        }
        result.push_str(&fragment);
    }
    Ok(result)
}
const VIEW_BLOCKS: &str = "folio-review-blocks";
fn apply_one(pkg: &mut Package, path: &str, target: &str, accept: bool) -> Result<(), String> {
    let xml = pkg.text(path)?.to_string();
    let dom = parse(&xml)?;
    let Some(n) = dom
        .descendants()
        .find(|n| active(*n) && n.attribute(TARGET) == Some(target))
    else {
        // A selected containing deletion may already have removed this subtree.
        return Ok(());
    };
    let (supported, reason) = support(n, accept);
    if !supported {
        return Err(reason);
    }
    let tag = effective_tag(n);
    let mut range = n.range();
    let replacement = if is_property(n) {
        if accept {
            String::new()
        } else {
            let parent = n.parent().unwrap();
            range = parent.range();
            let previous = child(n, tag.trim_end_matches("Change")).unwrap();
            let mut previous = scoped_record(&xml, previous);
            if tag == "sectPrChange" {
                // Header/footer links are outside the previous-section snapshot.
                let references = parent
                    .children()
                    .filter(|c| {
                        c.has_tag_name((W, "headerReference"))
                            || c.has_tag_name((W, "footerReference"))
                    })
                    .map(|c| scoped_record(&xml, c))
                    .collect::<String>();
                let d = parse(&previous)?;
                let root = d.root_element();
                let inner = root
                    .children()
                    .filter(|c| {
                        !c.has_tag_name((W, "headerReference"))
                            && !c.has_tag_name((W, "footerReference"))
                    })
                    .map(|c| raw(&previous, c))
                    .collect::<String>();
                previous = replace_inner(&previous, root, &(references + &inner));
            }
            if tag == "pPrChange" || tag == "rPrChange" {
                // Previous pPr excludes paragraph-mark and section properties.
                let retain = parent
                    .children()
                    .filter(|c| {
                        *c != n
                            && (active(*c)
                                || c.has_tag_name((W, "rPr"))
                                || c.has_tag_name((W, "sectPr")))
                    })
                    .map(|n| scoped_record(&xml, n))
                    .collect::<String>();
                if !retain.is_empty() {
                    let d = parse(&previous)?;
                    let root = d.root_element();
                    let inner = root
                        .children()
                        .map(|n| raw(&previous, n))
                        .collect::<String>();
                    previous = replace_inner(&previous, root, &(inner + &retain));
                }
            }
            previous
        }
    } else if paragraph_mark(n) {
        let join = (tag == "del") == accept;
        if join {
            let p = paragraph(n).unwrap();
            if crate::docx_moves::is_move(n)
                && !p.descendants().any(|c| c.has_tag_name((W, "sectPr")))
                && p.children().all(|c| {
                    c.has_tag_name((W, "pPr"))
                        || (c.is_text() && c.text().unwrap_or("").trim().is_empty())
                })
            {
                // All moved runs on this side have already been removed. The
                // moved mark removes this paragraph, including at story end.
                let last_in_cell = p.parent().is_some_and(|n| n.has_tag_name((W, "tc")))
                    && !p.next_siblings().skip(1).any(|n| n.is_element());
                let needs_cell_end = last_in_cell
                    && !p
                        .prev_siblings()
                        .skip(1)
                        .find(|n| n.is_element())
                        .is_some_and(|n| n.has_tag_name((W, "p")));
                let replacement = if needs_cell_end {
                    format!("<w:p xmlns:w=\"{W}\"/>")
                } else {
                    String::new()
                };
                let output = replace_ranges(&xml, vec![(p.range(), replacement)]);
                parse(&output)?;
                pkg.set(path, output);
                return Ok(());
            }
            let (p, next) = merge_target(n)?;
            range = p.range().start..next.range().end;
            let prefix = p
                .children()
                .filter(|n| !n.has_tag_name((W, "pPr")))
                .map(|n| {
                    if n.is_element() {
                        scoped_record(&xml, n)
                    } else {
                        raw(&xml, n).into()
                    }
                })
                .collect::<String>();
            let properties = child(next, "pPr")
                .map(|n| scoped_record(&xml, n))
                .unwrap_or_default();
            let suffix = next
                .children()
                .filter(|n| !n.has_tag_name((W, "pPr")))
                .map(|n| {
                    if n.is_element() {
                        scoped_record(&xml, n)
                    } else {
                        raw(&xml, n).into()
                    }
                })
                .collect::<String>();
            let mut fragment = scoped_record(&xml, next);
            // Only a transient display copy carries this attribute. A joined
            // paragraph must still be reachable from either source paragraph.
            if let Some(first) = p.attribute(VIEW_BLOCKS) {
                let other = next.attribute(VIEW_BLOCKS).unwrap_or("");
                let d = parse(&fragment)?;
                let root = d.root_element();
                // Replace only our private attribute; retain namespace-qualified
                // Word attributes and their original spelling unchanged.
                if let Some(a) = root.attributes().find(|a| a.name() == VIEW_BLOCKS) {
                    fragment = replace_ranges(
                        &fragment,
                        vec![(a.range(), format!("{VIEW_BLOCKS}=\"{first} {other}\""))],
                    );
                } else {
                    let at = opening_tag_end(&fragment).ok_or("Invalid review paragraph")?;
                    fragment.insert_str(at, &format!(" {VIEW_BLOCKS}=\"{first}\""));
                }
            }
            let d = parse(&fragment)?;
            let between = &xml[p.range().end..next.range().start];
            replace_inner(
                &fragment,
                d.root_element(),
                &(properties + &prefix + &suffix),
            ) + between
        } else {
            String::new()
        }
    } else if parent_is(n, "trPr") {
        if (tag == "del") == accept {
            let row = n.parent().unwrap().parent().unwrap();
            range = row.range();
            let table = row.parent().filter(|p| p.has_tag_name((W, "tbl")));
            if let Some(table) =
                table.filter(|t| t.children().filter(|n| n.has_tag_name((W, "tr"))).count() == 1)
            {
                range = table.range();
                // A cell must retain a paragraph after its last nested table.
                if table.parent().is_some_and(|p| p.has_tag_name((W, "tc")))
                    && !table
                        .next_siblings()
                        .skip(1)
                        .any(|p| p.has_tag_name((W, "p")))
                {
                    let output =
                        replace_ranges(&xml, vec![(range, format!("<w:p xmlns:w=\"{W}\"/>"))]);
                    parse(&output)?;
                    pkg.set(path, output);
                    return Ok(());
                }
            }
        }
        String::new()
    } else if (tag == "ins") == accept {
        unwrapped(&xml, n, tag == "del")?
    } else {
        String::new()
    };
    let output = replace_ranges(&xml, vec![(range, replacement)]);
    parse(&output)?;
    pkg.set(path, output);
    Ok(())
}
fn check_ranges(before: &str, after: &str) -> Result<(), String> {
    fn ranges(
        xml: &str,
        start: &str,
        end: &str,
    ) -> Result<BTreeMap<String, (usize, usize)>, String> {
        let d = parse(xml)?;
        let mut result: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for n in d
            .descendants()
            .filter(|n| n.has_tag_name((W, start)) || n.has_tag_name((W, end)))
        {
            if let Some(id) = n.attribute((W, "id")) {
                let v = result.entry(id.into()).or_default();
                if n.tag_name().name() == start {
                    v.0 += 1
                } else {
                    v.1 += 1
                };
            }
        }
        Ok(result)
    }
    for (start, end) in [
        ("bookmarkStart", "bookmarkEnd"),
        ("commentRangeStart", "commentRangeEnd"),
        ("permStart", "permEnd"),
        ("moveFromRangeStart", "moveFromRangeEnd"),
        ("moveToRangeStart", "moveToRangeEnd"),
    ] {
        let a = ranges(before, start, end)?;
        let b = ranges(after, start, end)?;
        if a.iter()
            .any(|(id, (starts, ends))| starts == ends && b.get(id).is_some_and(|(s, e)| s != e))
        {
            return Err("This change crosses a bookmark, comment, or protected range that needs additional review support. Nothing changed.".into());
        }
    }
    Ok(())
}
pub fn resolve(pkg: &Package, request: &Value) -> Result<Package, String> {
    if protection(pkg)? {
        return Err(
            "Unlock document protection in Word before accepting or rejecting changes.".into(),
        );
    }
    let accept = match request["action"].as_str() {
        Some("accept") => true,
        Some("reject") => false,
        _ => return Err("Choose Accept or Reject".into()),
    };
    let initial = list(pkg)?;
    let mut wanted = request["keys"]
        .as_array()
        .ok_or("Select tracked changes to review")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(String::from)
                .ok_or("Invalid change identifier")
        })
        .collect::<Result<HashSet<_>, _>>()?;
    if wanted.is_empty() {
        return Err("Select tracked changes to review".into());
    }
    let records = initial["revisions"].as_array().unwrap();
    if records
        .iter()
        .filter(|r| wanted.contains(r["key"].as_str().unwrap()))
        .count()
        != wanted.len()
    {
        return Err("The change list is stale. Reopen Track Changes.".into());
    }
    // A named move is one review operation, even if a filter shows one side.
    let linked = records
        .iter()
        .filter(|r| wanted.contains(r["key"].as_str().unwrap()))
        .flat_map(|r| r["move"]["memberKeys"].as_array().into_iter().flatten())
        .filter_map(|k| k.as_str().map(String::from))
        .collect::<Vec<_>>();
    wanted.extend(linked);
    let capability = if accept { "canAccept" } else { "canReject" };
    let reason = if accept {
        "acceptReason"
    } else {
        "rejectReason"
    };
    if let Some(r) = records
        .iter()
        .find(|r| wanted.contains(r["key"].as_str().unwrap()) && r[capability] != true)
    {
        return Err(r[reason].as_str().unwrap().into());
    }
    let mut next = pkg.clone();
    for path in parts(pkg) {
        let xml = pkg.text(&path)?;
        let dom = parse(xml)?;
        let moves = crate::docx_moves::Catalog::read(dom.root_element());
        if dom
            .descendants()
            .any(|n| n.attribute(TARGET).is_some() || n.attribute(VIEW_BLOCKS).is_some())
        {
            return Err("Reserved temporary revision attribute in source".into());
        }
        let mut selected = vec![];
        let mut edits = vec![];
        for group in moves.valid_groups().filter(|g| {
            dom.descendants().any(|n| {
                g.members.contains(&n.range().start)
                    && active(n)
                    && wanted.contains(&key(&path, xml, n))
            })
        }) {
            edits.extend(group.markers.iter().cloned().map(|r| (r, String::new())));
        }
        for n in dom
            .descendants()
            .filter(|n| active(*n) && wanted.contains(&key(&path, xml, *n)))
        {
            let id = selected.len().to_string();
            let source = raw(xml, n);
            let end = source[1..]
                .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .unwrap()
                + 1
                + n.range().start;
            edits.push((end..end, format!(" {TARGET}=\"{id}\"")));
            selected.push(id);
        }
        if selected.is_empty() {
            continue;
        }
        next.set(&path, replace_ranges(xml, edits));
        for id in selected.iter().rev() {
            apply_one(&mut next, &path, id, accept)?;
        }
        let output = next.text(&path)?;
        check_ranges(xml, output)?;
        let d = parse(output)?;
        moves.check_after(dom.root_element(), d.root_element())?;
        if d.descendants().any(|n| n.attribute(TARGET).is_some()) {
            return Err("Some selected revisions could not be resolved. Nothing changed.".into());
        }
    }
    next.original = next.bytes()?;
    Ok(next)
}

/// Read-only display projection. It never returns a package for persistence and
/// does not treat hidden markup as accepted/rejected changes. Source review keys
/// remain the authority for actions; temporary keys are used only on the copy.
pub fn display(pkg: &Package, mode: &str) -> Result<Value, String> {
    if !matches!(mode, "markup" | "current" | "original") {
        return Err("Choose All markup, Current or Original".into());
    }
    let mut copy = pkg.clone();
    let source = pkg.text("word/document.xml")?;
    let dom = parse(source)?;
    let body = dom
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .ok_or("Missing Word body")?;
    if dom
        .descendants()
        .any(|n| n.attribute(VIEW_BLOCKS).is_some() || n.attribute(TARGET).is_some())
    {
        return Err("Reserved review attribute in source".into());
    }
    let mut edits = vec![];
    for (i, n) in body
        .children()
        .filter(|n| n.is_element() && !n.has_tag_name((W, "sectPr")))
        .enumerate()
    {
        let start = n.range().start;
        let at = source[start + 1..]
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap()
            + start
            + 1;
        edits.push((at..at, format!(" {VIEW_BLOCKS}=\"word-{i}\"")));
    }
    copy.set("word/document.xml", replace_ranges(source, edits));
    if mode != "markup" {
        // Viewing a protected file is allowed; this path cannot write it. Keep
        // the protected flag on the original records used by UI actions.
        for path in parts(pkg) {
            let xml = copy.text(&path)?.to_string();
            let dom = parse(&xml)?;
            let moves = crate::docx_moves::Catalog::read(dom.root_element());
            let mut targets = vec![];
            let mut edits = vec![];
            for n in dom.descendants().filter(|n| active(*n)) {
                let (supported, reason) = if crate::docx_moves::is_move(n) {
                    moves
                        .group(n)
                        .map(|_| support(n, mode == "current"))
                        .unwrap_or_else(|reason| (false, reason))
                } else {
                    support(n, mode == "current")
                };
                if !supported {
                    return Err(format!(
                        "{} view cannot yet display {}: {reason}",
                        if mode == "current" {
                            "Current"
                        } else {
                            "Original"
                        },
                        kind(n)
                    ));
                }
                let id = targets.len().to_string();
                let at = xml[n.range().start + 1..]
                    .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .unwrap()
                    + n.range().start
                    + 1;
                edits.push((at..at, format!(" {TARGET}=\"{id}\"")));
                targets.push(id);
            }
            for group in moves.valid_groups() {
                edits.extend(group.markers.iter().cloned().map(|r| (r, String::new())));
            }
            copy.set(&path, replace_ranges(&xml, edits));
            for target in targets.iter().rev() {
                apply_one(&mut copy, &path, target, mode == "current")?;
            }
        }
    }
    let (mut document, report) = crate::docx::import(&copy)?;
    let xml = copy.text("word/document.xml")?;
    let dom = parse(xml)?;
    let body = dom
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let anchors = body
        .children()
        .filter(|n| n.is_element() && !n.has_tag_name((W, "sectPr")))
        .map(|n| {
            n.attribute(VIEW_BLOCKS)
                .unwrap_or("")
                .split_whitespace()
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (node, anchors) in document["content"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(anchors)
    {
        node["attrs"]["reviewBlocks"] = json!(anchors);
    }
    Ok(json!({"mode":mode,"document":document,"report":report}))
}

/// Upgrade only unchanged source blocks. Older saved workspaces may have exposed
/// formatting revisions as ordinary paragraphs; actual user edits remain intact.
pub fn hydrate(document: &mut Value, source: &Value, baseline: &Value) {
    if let Some(nodes) = document["content"].as_array_mut() {
        for node in nodes {
            let id = node["attrs"]["officeId"].clone();
            if id.is_null() {
                continue;
            }
            let old = baseline["content"]
                .as_array()
                .and_then(|a| a.iter().find(|b| b["attrs"]["officeId"] == id));
            let fresh = source["content"].as_array().and_then(|a| {
                a.iter().find(|b| {
                    b["attrs"]["officeId"] == id
                        && (b["attrs"].get("revisionPreview").is_some()
                            || b["attrs"]["hiddenReviewMarker"] == true)
                })
            });
            if let Some(fresh) = fresh.filter(|_| old == Some(node)) {
                *node = fresh.clone();
            }
        }
    }
}
pub fn check_model(before: &Value, after: &Value) -> Result<(), String> {
    fn collect(n: &Value, out: &mut BTreeMap<String, usize>) {
        if n["attrs"].get("revisionPreview").is_some() || n["attrs"]["hiddenReviewMarker"] == true {
            *out.entry(n.to_string()).or_default() += 1;
        } else if let Some(a) = n["content"].as_array() {
            for c in a {
                collect(c, out);
            }
        }
    }
    let mut a = BTreeMap::new();
    let mut b = BTreeMap::new();
    collect(before, &mut a);
    collect(after, &mut b);
    if a.iter().any(|(k, n)| b.get(k).copied().unwrap_or(0) < *n) {
        return Err("Use Review > Track Changes to accept or reject this content before changing or removing it.".into());
    }
    Ok(())
}

const BINDING: &str = "folioReviewTemporaryBinding";
pub fn mark_binding(fragment: &str, namespaces: &str, index: usize) -> Result<String, String> {
    let xml = crate::docx::wrap_fragment(fragment, namespaces);
    let dom = parse(&xml)?;
    let mut result = String::new();
    for n in dom.root_element().children() {
        if !n.is_element() {
            result.push_str(raw(&xml, n));
            continue;
        }
        if n.attribute(BINDING).is_some() {
            return Err("Reserved review binding attribute in source".into());
        }
        let source = scoped_record(&xml, n);
        let end = source[1..]
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap()
            + 1;
        result.push_str(&replace_ranges(
            &source,
            vec![(end..end, format!(" {BINDING}=\"{index}\""))],
        ));
    }
    Ok(result)
}
/// Reattach each live block only if every exported paragraph/table survives
/// untouched and contiguous. A paragraph join across its boundary is atomic-fail.
pub fn restore_bindings(
    before: &Package,
    after: &mut Package,
    original: &Value,
) -> Result<BTreeMap<usize, (usize, Value)>, String> {
    fn inventory(xml: &str) -> Result<BTreeMap<usize, Vec<(usize, String)>>, String> {
        let d = parse(xml)?;
        let body = d
            .descendants()
            .find(|n| n.has_tag_name((W, "body")))
            .ok_or("Word body missing")?;
        let mut result: BTreeMap<usize, Vec<(usize, String)>> = BTreeMap::new();
        for (i, n) in body
            .children()
            .filter(|n| n.is_element() && !n.has_tag_name((W, "sectPr")))
            .enumerate()
        {
            if let Some(id) = n.attribute(BINDING) {
                let id = id.parse::<usize>().map_err(|_| "Invalid review binding")?;
                let mut s = String::new();
                canonical(n, &mut s);
                result.entry(id).or_default().push((i, s));
            }
        }
        Ok(result)
    }
    let a = inventory(before.text("word/document.xml")?)?;
    let xml = after.text("word/document.xml")?.to_string();
    let b = inventory(&xml)?;
    let mut result = BTreeMap::new();
    for (id, source) in a {
        let current = b
            .get(&id)
            .ok_or("This change would remove a live workbook block. Its link is unchanged.")?;
        if source
            .iter()
            .map(|(_, s)| s)
            .ne(current.iter().map(|(_, s)| s))
            || current.windows(2).any(|p| p[1].0 != p[0].0 + 1)
        {
            return Err("This change crosses a live workbook block boundary. Review the surrounding changes separately; the link is unchanged.".into());
        }
        let node = original["content"]
            .get(id)
            .filter(|n| n["type"] == "workbookBlock")
            .ok_or("Live workbook block changed during review")?;
        result.insert(current[0].0, (current.len(), node.clone()));
    }
    let d = parse(&xml)?;
    let edits = d
        .descendants()
        .filter_map(|n| {
            n.attributes()
                .find(|a| a.name() == BINDING)
                .map(|a| (a.range(), String::new()))
        })
        .collect();
    after.set("word/document.xml", replace_ranges(&xml, edits));
    after.original = after.bytes()?;
    Ok(result)
}

/// Collapse exported live workbook blocks back into native embeds and translate
/// annotation ranges from the expanded Word text into the native body story.
pub fn restore_model_bindings(
    document: &mut Value,
    bindings: &BTreeMap<usize, (usize, Value)>,
) -> Result<(), String> {
    if bindings.is_empty() {
        return Ok(());
    }
    let before = document.clone();
    let nodes = before["content"].as_array().ok_or("Invalid document")?;
    let mut merged = vec![];
    let mut edits = vec![];
    let mut i = 0;
    let mut offset = 0;
    while i < nodes.len() {
        if let Some((count, node)) = bindings.get(&i) {
            let end = i
                .checked_add(*count)
                .filter(|n| *n <= nodes.len())
                .ok_or("Invalid workbook binding range")?;
            let text = crate::document_bookmarks::Index::new(
                &json!({"type":"doc","content":&nodes[i..end]}),
            )
            .text;
            let length = text.encode_utf16().count();
            edits.push(json!({"start":offset,"end":offset+length,"text":"\u{fffc}\n"}));
            offset += length;
            merged.push(node.clone());
            i = end;
        } else {
            offset += crate::document_bookmarks::Index::new(
                &json!({"type":"doc","content":[nodes[i].clone()]}),
            )
            .text
            .encode_utf16()
            .count();
            merged.push(nodes[i].clone());
            i += 1;
        }
    }
    document["content"] = json!(merged);
    let exact = json!(edits);
    crate::document_bookmarks::rebase(&before, document, Some(&exact))?;
    crate::document_comments::rebase(&before, document, Some(&exact))?;
    Ok(())
}
