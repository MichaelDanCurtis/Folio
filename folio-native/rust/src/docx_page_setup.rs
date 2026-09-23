//! Section-owned page geometry. Read projections never rewrite the package;
//! explicit changes patch only the selected properties in their source scope.
use crate::office::*;
use roxmltree::Node;
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub const KEY: &str = "pageLayout";

pub fn check_model(before: &Value, after: &Value) -> Result<(), String> {
    let Some(sections) = before["attrs"][KEY]["sections"].as_array() else {
        return Ok(());
    };
    let blocks = after["content"].as_array().ok_or("Invalid document")?;
    let mut previous = None;
    for section in sections {
        let Some(id) = section["endOfficeId"].as_str() else {
            continue;
        };
        let index = blocks.iter().rposition(|b| {
            b["attrs"]["officeId"] == id
                && matches!(
                    b["type"].as_str(),
                    Some("paragraph" | "heading" | "officeBlock")
                )
        });
        if index.is_none() || previous.is_some_and(|p| index.unwrap() <= p) {
            return Err("This edit would remove or reorder a section boundary. Keep its paragraph, or edit the section break in Word.".into());
        }
        previous = index;
    }
    Ok(())
}

pub fn strip_paragraph_boundary(fragment: &str, namespaces: &str) -> Result<String, String> {
    let prefix = format!("<w:body xmlns:w=\"{W}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"{namespaces}>");
    let wrapped = format!("{prefix}{fragment}</w:body>");
    let d = parse(&wrapped)?;
    let edits = d
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((W, "p")))
        .filter_map(|p| child(p, "pPr"))
        .filter_map(|p| child(p, "sectPr"))
        .map(|s| (s.range(), String::new()))
        .collect();
    let result = replace_ranges(&wrapped, edits);
    Ok(result[prefix.len()..result.len() - 9].to_string())
}

pub fn check_retained(before: &str, after: &str) -> Result<(), String> {
    fn canonical(n: Node) -> Value {
        let mut attrs = n
            .attributes()
            .map(|a| {
                (
                    format!("{}:{}", a.namespace().unwrap_or(""), a.name()),
                    a.value(),
                )
            })
            .collect::<Vec<_>>();
        attrs.sort();
        json!([
            n.tag_name().namespace(),
            n.tag_name().name(),
            attrs,
            n.children()
                .filter(|c| c.is_element())
                .map(canonical)
                .collect::<Vec<_>>()
        ])
    }
    fn properties(xml: &str) -> Result<Vec<Value>, String> {
        let d = parse(xml)?;
        Ok(sections(document_body(&d)?)
            .into_iter()
            .flatten()
            .map(canonical)
            .collect())
    }
    if properties(before)? != properties(after)? {
        return Err("This edit would remove or duplicate a section boundary. Keep its paragraph, or edit the section break in Word.".into());
    }
    Ok(())
}
const ORDER: &[&str] = &[
    "headerReference",
    "footerReference",
    "footnotePr",
    "endnotePr",
    "type",
    "pgSz",
    "pgMar",
    "paperSrc",
    "pgBorders",
    "lnNumType",
    "pgNumType",
    "cols",
    "formProt",
    "vAlign",
    "noEndnote",
    "titlePg",
    "textDirection",
    "bidi",
    "rtlGutter",
    "docGrid",
    "printerSettings",
    "sectPrChange",
];

pub(crate) fn sections<'a, 'i>(body: Node<'a, 'i>) -> Vec<Option<Node<'a, 'i>>> {
    let mut result = body
        .descendants()
        .filter(|n| {
            n.has_tag_name((W, "sectPr"))
                && !n.ancestors().skip(1).any(|a| {
                    a.has_tag_name((W, "sectPrChange"))
                        || a.has_tag_name((W, "tc"))
                        || a.has_tag_name((W, "txbxContent"))
                })
        })
        .map(Some)
        .collect::<Vec<_>>();
    if child(body, "sectPr").is_none() {
        result.push(None);
    }
    result
}
pub(crate) fn document_body<'a, 'i>(
    d: &'a roxmltree::Document<'i>,
) -> Result<Node<'a, 'i>, String> {
    d.descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .ok_or("Word body is missing".into())
}
pub(crate) fn value(n: Option<Node>, name: &str, fallback: i64) -> i64 {
    n.and_then(|n| n.attribute((W, name)))
        .and_then(|v| {
            if let Ok(n) = v.parse::<i64>() {
                return Some(n);
            }
            for (unit, scale) in [
                ("in", 1440.0),
                ("pt", 20.0),
                ("cm", 1440.0 / 2.54),
                ("mm", 1440.0 / 25.4),
                ("pc", 240.0),
                ("pi", 240.0),
            ] {
                if let Some(value) = v.strip_suffix(unit).and_then(|v| v.parse::<f64>().ok()) {
                    let twips = value * scale;
                    if twips.is_finite() && twips.abs() < i32::MAX as f64 {
                        return Some(twips.round() as i64);
                    }
                }
            }
            None
        })
        .unwrap_or(fallback)
}
fn on(n: Option<Node>) -> bool {
    n.is_some_and(|n| !matches!(n.attribute((W, "val")), Some("0" | "false" | "off")))
}
fn section_key(xml: &str, n: Option<Node>, index: usize) -> String {
    format!(
        "section-{index}-{}",
        fingerprint(n.map(|n| raw(xml, n)).unwrap_or("").as_bytes())
    )
}
pub fn read(pkg: &Package) -> Result<Value, String> {
    let xml = pkg.text("word/document.xml")?;
    let d = parse(xml)?;
    let body = document_body(&d)?;
    let blocks = body
        .children()
        .filter(|n| n.is_element() && !n.has_tag_name((W, "sectPr")))
        .collect::<Vec<_>>();
    let settings = crate::docx_tabs::part(pkg, "settings")?;
    let flags = if let Ok(x) = pkg.text(&settings) {
        let d = parse(x)?;
        json!({"mirrorMargins":on(child(d.root_element(),"mirrorMargins")),"gutterTop":on(child(d.root_element(),"gutterAtTop"))})
    } else {
        json!({"mirrorMargins":false,"gutterTop":false})
    };
    let mut rows = vec![];
    let mut first = 0;
    for (index, s) in sections(body).into_iter().enumerate() {
        let size = s.and_then(|n| child(n, "pgSz"));
        let margin = s.and_then(|n| child(n, "pgMar"));
        let width = value(size, "w", 12240);
        let height = value(size, "h", 15840);
        let last = s.filter(|n| n.parent() != Some(body)).and_then(|n| {
            blocks
                .iter()
                .position(|p| p.range().start <= n.range().start && p.range().end >= n.range().end)
        });
        let columns = crate::docx_columns::read(s);
        let geometry = json!({"width":width,"height":height,"orientation":size.and_then(|n|n.attribute((W,"orient"))).unwrap_or("portrait"),
            "top":value(margin,"top",1440),"bottom":value(margin,"bottom",1440),"left":value(margin,"left",1440),"right":value(margin,"right",1440),
            "header":value(margin,"header",720),"footer":value(margin,"footer",720),"gutter":value(margin,"gutter",0),
            "start":s.and_then(|n|child(n,"type")).and_then(|n|n.attribute((W,"val"))).unwrap_or("nextPage")});
        rows.push(json!({"key":section_key(xml,s,index),"index":index,"firstBlock":first,"lastBlock":last.unwrap_or(blocks.len().saturating_sub(1)),
            "endOfficeId":last.map(|i|format!("word-{i}")),"geometry":geometry,"columns":columns["count"],"columnLayout":columns,
            "sizeExplicit":size.is_some(),"marginsExplicit":margin.is_some(),"rtlGutter":s.is_some_and(|n|on(child(n,"rtlGutter"))),
            "hasTrackedSettings":s.is_some_and(|n|child(n,"sectPrChange").is_some())}));
        first = last.map(|i| i + 1).unwrap_or(blocks.len());
    }
    // Continuous sections may omit page geometry and inherit it from the
    // following section. Resolve backwards without materializing XML defaults.
    for i in (0..rows.len().saturating_sub(1)).rev() {
        if rows[i]["geometry"]["start"] == "continuous" {
            for (explicit, keys) in [
                ("sizeExplicit", vec!["width", "height", "orientation"]),
                (
                    "marginsExplicit",
                    vec![
                        "top", "bottom", "left", "right", "header", "footer", "gutter",
                    ],
                ),
            ] {
                if rows[i][explicit] == false {
                    for key in keys {
                        rows[i]["geometry"][key] = rows[i + 1]["geometry"][key].clone();
                    }
                    rows[i]["inheritedPageGeometry"] = json!(true);
                }
            }
        }
    }
    Ok(
        json!({"version":1,"sections":rows,"mirrorMargins":flags["mirrorMargins"],"gutterTop":flags["gutterTop"]}),
    )
}

fn validate(g: &Value, gutter_top: bool) -> Result<(), String> {
    let number = |k: &str| {
        g[k].as_i64()
            .ok_or(format!("Enter a whole page measurement for {k}"))
    };
    let width = number("width")?;
    let height = number("height")?;
    if !(1440..=31680).contains(&width) || !(1440..=31680).contains(&height) {
        return Err("Paper dimensions must be between 1 and 22 inches".into());
    }
    for name in ["left", "right", "header", "footer", "gutter"] {
        if !(0..=31680).contains(&number(name)?) {
            return Err(format!("{name} must be a nonnegative page measurement"));
        }
    }
    for name in ["top", "bottom"] {
        if !(-31680..=31680).contains(&number(name)?) {
            return Err(format!("{name} is outside the supported page range"));
        }
    }
    let gutter = number("gutter")?;
    if number("left")? + number("right")? + if gutter_top { 0 } else { gutter } >= width
        || number("top")?.abs() + number("bottom")?.abs() + if gutter_top { gutter } else { 0 }
            >= height
    {
        return Err("The margins and gutter must leave space for document text".into());
    }
    if number("header")? >= height || number("footer")? >= height {
        return Err("Header and footer distances must fit inside the paper".into());
    }
    if !matches!(g["orientation"].as_str(), Some("portrait" | "landscape")) {
        return Err("Choose Portrait or Landscape".into());
    }
    if !matches!(
        g["start"].as_str(),
        Some("nextPage" | "continuous" | "evenPage" | "oddPage" | "nextColumn")
    ) {
        return Err("Choose a valid section start".into());
    }
    Ok(())
}
pub(crate) fn patch_leaf(
    xml: Option<&str>,
    tag: &str,
    attrs: &[(&str, String)],
    remove: &[&str],
) -> Result<String, String> {
    let source = xml
        .map(String::from)
        .unwrap_or_else(|| format!("<w:{tag} xmlns:w=\"{W}\"/>"));
    let d = parse(&source)?;
    let n = d.root_element();
    let mut edits = vec![];
    for a in n.attributes() {
        if a.namespace() == Some(W)
            && (attrs.iter().any(|(k, _)| *k == a.name()) || remove.contains(&a.name()))
        {
            edits.push((a.range(), String::new()));
        }
    }
    let mut prefix = n
        .lookup_prefix(W)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let mut addition = String::new();
    if prefix.is_none() {
        let mut name = "folioPage".to_string();
        let mut i = 0;
        while n.lookup_namespace_uri(Some(&name)).is_some() {
            i += 1;
            name = format!("folioPage{i}");
        }
        addition.push_str(&format!(" xmlns:{name}=\"{W}\""));
        prefix = Some(name);
    }
    let prefix = prefix.unwrap();
    for (key, value) in attrs {
        addition.push_str(&format!(" {prefix}:{key}=\"{}\"", esc(value)));
    }
    let pos = source[1..]
        .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .ok_or("Invalid page property")?
        + 1;
    edits.push((pos..pos, addition));
    let output = replace_ranges(&source, edits);
    parse(&output)?;
    Ok(output)
}
pub(crate) fn set_child(xml: &str, tag: &str, fragment: &str) -> Result<String, String> {
    let d = parse(xml)?;
    let root = d.root_element();
    let matches = root
        .children()
        .filter(|n| n.has_tag_name((W, tag)))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err("Duplicate section properties need repair in Word before editing".into());
    }
    if let Some(n) = matches.first() {
        return Ok(replace_ranges(xml, vec![(n.range(), fragment.into())]));
    }
    let rank = ORDER.iter().position(|s| *s == tag).unwrap();
    let mut inserted = false;
    let mut inner = String::new();
    for n in root.children() {
        if !inserted
            && n.tag_name().namespace() == Some(W)
            && ORDER
                .iter()
                .position(|s| *s == n.tag_name().name())
                .is_some_and(|i| i > rank)
        {
            inner.push_str(fragment);
            inserted = true;
        }
        inner.push_str(raw(xml, n));
    }
    if !inserted {
        inner.push_str(fragment);
    }
    Ok(replace_inner(xml, root, &inner))
}
pub(crate) fn changed_section(
    xml: &str,
    patch: &Value,
    geometry: &Value,
    tracking: Option<&Value>,
    id: u64,
) -> Result<String, String> {
    let original = parse(xml)?;
    let root = original.root_element();
    let mut output = xml.to_string();
    for (tag, keys) in [
        ("pgSz", vec!["width", "height", "orientation"]),
        (
            "pgMar",
            vec![
                "top", "bottom", "left", "right", "header", "footer", "gutter",
            ],
        ),
        ("type", vec!["start"]),
    ] {
        if !keys.iter().any(|k| patch.get(k).is_some()) {
            continue;
        }
        let existing = child(root, tag).map(|n| scoped_record(xml, n));
        let attrs = keys
            .iter()
            .map(|k| {
                let name = match *k {
                    "width" => "w",
                    "height" => "h",
                    "orientation" => "orient",
                    "start" => "val",
                    s => s,
                };
                (
                    name,
                    geometry[*k]
                        .as_str()
                        .map(String::from)
                        .unwrap_or_else(|| geometry[*k].to_string()),
                )
            })
            .collect::<Vec<_>>();
        let remove = if tag == "pgSz" { vec!["code"] } else { vec![] };
        let leaf = patch_leaf(existing.as_deref(), tag, &attrs, &remove)?;
        output = set_child(&output, tag, &leaf)?;
    }
    if let Some(columns) = patch.get("columnLayout") {
        output = crate::docx_columns::patch(&output, columns)?;
    }
    if let Some(settings) = tracking.filter(|s| s["enabled"] == true && s["formatting"] == true) {
        crate::docx_tracking::validate_settings(settings)?;
        if child(root, "sectPrChange").is_some() {
            return Err("Review the pending section formatting change before recording another page setup change".into());
        }
        // Header/footer references do not belong to CT_SectPrBase snapshots.
        let previous = root
            .children()
            .filter(|n| {
                !n.has_tag_name((W, "headerReference")) && !n.has_tag_name((W, "footerReference"))
            })
            .map(|n| {
                if n.is_element() {
                    scoped_record(xml, n)
                } else {
                    raw(xml, n).into()
                }
            })
            .collect::<String>();
        let change=format!("<w:sectPrChange xmlns:w=\"{W}\" w:id=\"{id}\" w:author=\"{}\" w:date=\"{}\"><w:sectPr>{previous}</w:sectPr></w:sectPrChange>",esc(settings["author"].as_str().unwrap()),chrono::Utc::now().to_rfc3339());
        output = set_child(&output, "sectPrChange", &change)?;
    }
    Ok(output)
}
pub fn apply(pkg: &Package, r: &Value, tracking: Option<&Value>) -> Result<Package, String> {
    if crate::docx_revisions::protection(pkg)? {
        return Err("Unlock document protection in Word before changing page setup".into());
    }
    let patch = r["patch"]
        .as_object()
        .ok_or("Page setup changes are missing")?;
    let allowed = [
        "width",
        "height",
        "orientation",
        "top",
        "bottom",
        "left",
        "right",
        "header",
        "footer",
        "gutter",
        "start",
        "columnLayout",
    ];
    if patch.is_empty() || patch.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err("Choose supported page setup properties".into());
    }
    let catalog = read(pkg)?;
    let index = r["section"].as_u64().ok_or("Choose a section")? as usize;
    let selected = catalog["sections"]
        .get(index)
        .ok_or("The section no longer exists")?;
    if r["sectionKey"] != selected["key"] {
        return Err("Page setup changed. Reopen the dialog".into());
    }
    if !matches!(r["scope"].as_str(), Some("section" | "document")) {
        return Err("Choose a page setup scope".into());
    }
    let xml = pkg.text("word/document.xml")?;
    let d = parse(xml)?;
    let body = document_body(&d)?;
    let mut id = d
        .descendants()
        .filter_map(|n| n.attribute((W, "id")).and_then(|v| v.parse::<u64>().ok()))
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("Revision identifier overflow")?;
    let mut edits = vec![];
    let mut append = String::new();
    for (i, s) in sections(body).iter().enumerate() {
        if r["scope"] == "section" && i != index {
            continue;
        }
        let old = &catalog["sections"][i]["geometry"];
        let mut geometry = old.clone();
        for (k, v) in patch {
            if k != "columnLayout" {
                geometry[k] = v.clone();
            }
        }
        let columns = patch
            .get("columnLayout")
            .unwrap_or(&catalog["sections"][i]["columnLayout"]);
        if &geometry == old && columns == &catalog["sections"][i]["columnLayout"] {
            continue;
        }
        validate(&geometry, catalog["gutterTop"] == true)?;
        if patch.contains_key("columnLayout") {
            crate::docx_columns::validate(columns, &geometry, catalog["gutterTop"] == true)?;
        }
        if s.is_some_and(|n| child(n, "sectPrChange").is_some()) {
            return Err(
                "Review the pending section formatting change before changing page setup".into(),
            );
        }
        let scoped = s
            .map(|n| scoped_record(xml, n))
            .unwrap_or_else(|| format!("<w:sectPr xmlns:w=\"{W}\"/>"));
        let output = changed_section(&scoped, &r["patch"], &geometry, tracking, id)?;
        id = id.checked_add(1).ok_or("Revision identifier overflow")?;
        if let Some(n) = s {
            edits.push((n.range(), output));
        } else {
            append = output;
        }
    }
    if edits.is_empty() && append.is_empty() {
        return Ok(pkg.clone());
    }
    let mut output = replace_ranges(xml, edits);
    if !append.is_empty() {
        let d = parse(&output)?;
        let body = document_body(&d)?;
        let inner = body.children().map(|n| raw(&output, n)).collect::<String>() + &append;
        output = replace_inner(&output, body, &inner);
    }
    parse(&output)?;
    let mut next = pkg.clone();
    next.set("word/document.xml", output);
    Package::from_bytes(next.bytes()?)
}
pub fn hydrate(document: &mut Value, source: &Value) {
    if document["attrs"].get(KEY).is_none() {
        document["attrs"][KEY] = source["attrs"][KEY].clone();
    }
}
