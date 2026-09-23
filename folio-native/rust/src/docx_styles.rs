//! Named Word styles. Declarations stay separate from resolved properties so
//! edits to a base style update its dependents without flattening inheritance.
use crate::office::*;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub const KEY: &str = "wordStyles";
const TOGGLES: &[&str] = &[
    "bold",
    "italic",
    "strike",
    "caps",
    "smallCaps",
    "outline",
    "shadow",
    "emboss",
    "imprint",
    "hidden",
];
const RUN: &[(&str, &str)] = &[
    ("b", "bold"),
    ("i", "italic"),
    ("strike", "strike"),
    ("caps", "caps"),
    ("smallCaps", "smallCaps"),
    ("outline", "outline"),
    ("shadow", "shadow"),
    ("emboss", "emboss"),
    ("imprint", "imprint"),
    ("vanish", "hidden"),
];
fn wc<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children().find(|n| n.has_tag_name((W, name)))
}
fn val<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<&'a str> {
    wc(n, name).and_then(|n| n.attribute((W, "val")))
}
fn on(n: roxmltree::Node) -> bool {
    !matches!(n.attribute((W, "val")), Some("0" | "false" | "off"))
}
fn run(pr: Option<roxmltree::Node>) -> Value {
    let mut r = Map::new();
    if let Some(pr) = pr {
        for (tag, key) in RUN {
            if let Some(n) = wc(pr, tag) {
                r.insert((*key).into(), json!(on(n)));
            }
        }
        // Retain literal/theme font and color declarations independently. Theme
        // resolution belongs to the renderer; copying a literal is not a theme.
        for (tag, keys) in [
            (
                "rFonts",
                &[
                    "ascii",
                    "hAnsi",
                    "eastAsia",
                    "cs",
                    "asciiTheme",
                    "hAnsiTheme",
                    "eastAsiaTheme",
                    "cstheme",
                    "hint",
                ][..],
            ),
            (
                "color",
                &["val", "themeColor", "themeTint", "themeShade"][..],
            ),
        ] {
            if let Some(n) = wc(pr, tag) {
                let mut a = Map::new();
                for k in keys {
                    if let Some(v) = n.attribute((W, *k)) {
                        a.insert((*k).into(), json!(v));
                    }
                }
                r.insert(tag.into(), Value::Object(a));
            }
        }
        for tag in ["sz", "szCs", "spacing", "position", "kern"] {
            if let Some(v) = val(pr, tag).and_then(|s| s.parse::<i64>().ok()) {
                r.insert(tag.into(), json!(v));
            }
        }
        for tag in ["u", "vertAlign", "highlight"] {
            if let Some(v) = val(pr, tag) {
                r.insert(tag.into(), json!(v));
            }
        }
    }
    Value::Object(r)
}
fn properties(p: Option<roxmltree::Node>, r: Option<roxmltree::Node>) -> Value {
    let mut result = json!({"paragraph":crate::docx_paragraph::read(p),"tabs":[],"run":run(r)});
    result["paragraph"]
        .as_object_mut()
        .unwrap()
        .remove("outlineLevel");
    if let Some(p) = p {
        for (tag, key) in [("jc", "alignment"), ("outlineLvl", "outlineLevel")] {
            if let Some(s) = val(p, tag) {
                if key == "outlineLevel" {
                    if let Ok(n) = s.parse::<u64>() {
                        result[key] = json!(n);
                    }
                } else {
                    result[key] = json!(s);
                }
            }
        }
        if let Some(tabs) = wc(p, "tabs") {
            result["tabs"] = json!(tabs.children().filter(|n|n.has_tag_name((W,"tab"))).filter_map(|n| {
                Some(json!({"position":n.attribute((W,"pos"))?.parse::<i32>().ok()?,
                    "alignment":n.attribute((W,"val")).unwrap_or("left"),"leader":n.attribute((W,"leader")).unwrap_or("none")}))
            }).collect::<Vec<_>>());
        }
    }
    result
}
pub fn read(pkg: &Package) -> Result<Value, String> {
    let path = crate::docx_tabs::part(pkg, "styles")?;
    let mut catalog = json!({"defaults":properties(None,None),"styles":[]});
    let Ok(xml) = pkg.text(&path) else {
        return Ok(catalog);
    };
    let d = parse(xml)?;
    let root = d.root_element();
    if let Some(dd) = wc(root, "docDefaults") {
        catalog["defaults"] = properties(
            wc(dd, "pPrDefault").and_then(|n| wc(n, "pPr")),
            wc(dd, "rPrDefault").and_then(|n| wc(n, "rPr")),
        );
    }
    let mut styles = vec![];
    for n in root.children().filter(|n| n.has_tag_name((W, "style"))) {
        let Some(id) = n.attribute((W, "styleId")) else {
            continue;
        };
        let typ = n.attribute((W, "type")).unwrap_or("paragraph");
        let mut style = json!({"id":id,"type":typ,"name":val(n,"name").unwrap_or(id),
            "customStyle":matches!(n.attribute((W,"customStyle")),Some("1"|"true"|"on")),
            "properties":properties(wc(n,"pPr"),wc(n,"rPr"))});
        for (tag, key) in [("basedOn", "basedOn"), ("next", "next"), ("link", "link")] {
            if let Some(v) = val(n, tag) {
                style[key] = json!(v);
            }
        }
        if matches!(n.attribute((W, "default")), Some("1" | "true" | "on")) {
            style["default"] = json!(true);
            if typ == "paragraph" {
                catalog["defaultStyle"] = json!(id);
            }
        }
        styles.push(style);
    }
    const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
    if let Ok(xml) = pkg.text(&crate::docx_tabs::part(pkg, "theme")?) {
        let d = parse(xml)?;
        let mut theme = json!({"fonts":{},"colors":{}});
        for group in ["majorFont", "minorFont"] {
            if let Some(n) = d.descendants().find(|n| n.has_tag_name((A, group))) {
                for script in ["latin", "ea", "cs"] {
                    if let Some(face) = n
                        .children()
                        .find(|n| n.has_tag_name((A, script)))
                        .and_then(|n| n.attribute("typeface"))
                        .filter(|s| !s.is_empty())
                    {
                        theme["fonts"][group][script] = json!(face);
                    }
                }
            }
        }
        if let Some(colors) = d.descendants().find(|n| n.has_tag_name((A, "clrScheme"))) {
            for c in colors.children().filter(|n| n.is_element()) {
                if let Some(v) = c
                    .children()
                    .find(|n| n.is_element())
                    .and_then(|n| n.attribute("lastClr").or_else(|| n.attribute("val")))
                    .filter(|s| s.len() == 6 && s.bytes().all(|c| c.is_ascii_hexdigit()))
                {
                    theme["colors"][c.tag_name().name()] = json!(v);
                }
            }
        }
        catalog["theme"] = theme;
    }
    catalog["styles"] = json!(styles);
    Ok(catalog)
}
pub(crate) fn style<'a>(catalog: &'a Value, id: &str) -> Option<&'a Value> {
    catalog["styles"].as_array()?.iter().find(|s| s["id"] == id)
}
fn merge_props(out: &mut Value, layer: &Value, toggle: bool) {
    for k in ["paragraph", "run"] {
        for (name, value) in layer[k].as_object().into_iter().flatten() {
            if k == "run" && toggle && TOGGLES.contains(&name.as_str()) {
                if value == true {
                    out[k][name] = json!(!out[k][name].as_bool().unwrap_or(false));
                }
            } else if k == "run" && value.is_object() {
                if name == "rFonts" {
                    for (literal, themed) in [
                        ("ascii", "asciiTheme"),
                        ("hAnsi", "hAnsiTheme"),
                        ("eastAsia", "eastAsiaTheme"),
                        ("cs", "cstheme"),
                    ] {
                        if value.get(literal).is_some() || value.get(themed).is_some() {
                            if let Some(fonts) = out[k][name].as_object_mut() {
                                fonts.remove(literal);
                                fonts.remove(themed);
                            }
                        }
                    }
                    for (a, v) in value.as_object().unwrap() {
                        out[k][name][a] = v.clone();
                    }
                } else {
                    out[k][name] = value.clone();
                }
            } else {
                out[k][name] = value.clone();
            }
        }
    }
    for k in ["alignment", "outlineLevel"] {
        if let Some(v) = layer.get(k) {
            out[k] = v.clone();
        }
    }
    let mut tabs: BTreeMap<i64, Value> = out["tabs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| Some((t["position"].as_i64()?, t.clone())))
        .collect();
    for t in layer["tabs"].as_array().into_iter().flatten() {
        if let Some(p) = t["position"].as_i64() {
            if t["alignment"] == "clear" {
                tabs.remove(&p);
            } else {
                tabs.insert(p, t.clone());
            }
        }
    }
    out["tabs"] = json!(tabs.into_values().collect::<Vec<_>>());
}
/// Resolve the supported declaration groups with same-type basedOn and Word's
/// toggle semantics. Imported broken chains terminate; newly edited cycles fail.
pub fn resolve(catalog: &Value, id: Option<&str>) -> Value {
    let mut out = properties(None, None);
    merge_props(&mut out, &catalog["defaults"], false);
    let mut seen = HashSet::new();
    let mut chain = vec![];
    let mut current = id
        .or_else(|| catalog["defaultStyle"].as_str())
        .and_then(|id| style(catalog, id));
    while let Some(s) = current {
        let Some(_id) = s["id"].as_str().filter(|id| seen.insert(id.to_string())) else {
            break;
        };
        chain.push(&s["properties"]);
        current = s["basedOn"]
            .as_str()
            .and_then(|id| style(catalog, id))
            .filter(|base| base["type"] == s["type"]);
    }
    for layer in chain.into_iter().rev() {
        merge_props(&mut out, layer, true);
    }
    for (key, themed, script) in [
        ("ascii", "asciiTheme", "latin"),
        ("hAnsi", "hAnsiTheme", "latin"),
        ("eastAsia", "eastAsiaTheme", "ea"),
        ("cs", "cstheme", "cs"),
    ] {
        if let Some(theme) = out["run"]["rFonts"][themed].as_str() {
            let group = if theme.starts_with("major") {
                "majorFont"
            } else {
                "minorFont"
            };
            if let Some(face) = catalog["theme"]["fonts"][group][script].as_str() {
                out["run"]["rFonts"][key] = json!(face);
            }
        }
    }
    let color = &out["run"]["color"];
    if color.get("themeTint").is_none() && color.get("themeShade").is_none() {
        if let Some(name) = color["themeColor"].as_str() {
            let key = match name {
                "background1" => "lt1",
                "text1" => "dk1",
                "background2" => "lt2",
                "text2" => "dk2",
                _ => name,
            };
            if let Some(v) = catalog["theme"]["colors"][key].as_str() {
                out["run"]["color"]["val"] = json!(v);
            }
        }
    }
    out
}
fn valid_text(v: &Value, max: usize) -> bool {
    v.as_str().is_some_and(|s| {
        !s.is_empty() && s.chars().count() <= max && !s.chars().any(|c| c.is_control())
    })
}
fn validate_properties(p: &Value) -> Result<(), String> {
    if !p.is_object() || !p["run"].is_object() {
        return Err("Invalid style properties".into());
    }
    crate::docx_paragraph::validate(
        &json!({"attrs":{"paragraphFormat":{"inherited":{},"direct":p["paragraph"]}}}),
    )?;
    crate::docx_tabs::validate_node(
        &json!({"attrs":{"tabs":{"inherited":[],"direct":p["tabs"]}}}),
    )?;
    if p.get("alignment").is_some_and(|v| {
        !v.as_str().is_some_and(|s| {
            [
                "left",
                "right",
                "center",
                "both",
                "start",
                "end",
                "distribute",
                "thaiDistribute",
                "numTab",
                "highKashida",
                "mediumKashida",
                "lowKashida",
            ]
            .contains(&s)
        })
    }) {
        return Err("Invalid style alignment".into());
    }
    if p.get("outlineLevel")
        .is_some_and(|v| !v.as_u64().is_some_and(|n| n <= 9))
    {
        return Err("Invalid style outline level".into());
    }
    for (key, v) in p["run"].as_object().unwrap() {
        let ok = if TOGGLES.contains(&key.as_str()) {
            v.is_boolean()
        } else if ["sz", "szCs", "spacing", "position", "kern"].contains(&key.as_str()) {
            v.as_i64().is_some_and(|n| {
                if ["sz", "szCs"].contains(&key.as_str()) {
                    (1..=3276).contains(&n)
                } else {
                    (-31680..=31680).contains(&n)
                }
            })
        } else if key == "rFonts" || key == "color" {
            v.as_object().is_some_and(|a| {
                a.iter().all(|(k, v)| {
                    let keys = if key == "rFonts" {
                        &[
                            "ascii",
                            "hAnsi",
                            "eastAsia",
                            "cs",
                            "asciiTheme",
                            "hAnsiTheme",
                            "eastAsiaTheme",
                            "cstheme",
                            "hint",
                        ][..]
                    } else {
                        &["val", "themeColor", "themeTint", "themeShade"][..]
                    };
                    keys.contains(&k.as_str()) && valid_text(v, 256)
                })
            })
        } else if ["u", "vertAlign", "highlight"].contains(&key.as_str()) {
            valid_text(v, 64)
        } else {
            false
        };
        if !ok {
            return Err(format!("Invalid style text property: {key}"));
        }
    }
    Ok(())
}
/// Structural validation accepts preserved dangling references and cycles on
/// import. Commands validate changed inheritance chains more strictly below.
pub fn validate(c: &Value) -> Result<(), String> {
    validate_properties(&c["defaults"])?;
    let styles = c["styles"].as_array().ok_or("Invalid style catalog")?;
    if styles.len() > 4096 {
        return Err("Too many document styles".into());
    }
    let mut ids = HashSet::new();
    for s in styles {
        if !valid_text(&s["id"], 256)
            || !valid_text(&s["name"], 256)
            || !ids.insert(s["id"].as_str())
            || !s["type"]
                .as_str()
                .is_some_and(|s| ["paragraph", "character", "table", "numbering"].contains(&s))
        {
            return Err("Invalid or duplicate document style".into());
        }
        if s.get("customStyle").is_some_and(|v| !v.is_boolean()) {
            return Err("Invalid style origin".into());
        }
        for k in ["basedOn", "next", "link"] {
            if s.get(k).is_some_and(|v| !valid_text(v, 256)) {
                return Err(format!("Invalid style {k}"));
            }
        }
        validate_properties(&s["properties"])?;
    }
    Ok(())
}
// Replace named children in schema order. Unchanged nodes, attributes and foreign
// extension XML are retained verbatim, including alternate namespace prefixes.
fn patch_children(
    xml: &str,
    n: roxmltree::Node,
    changes: &BTreeMap<&str, String>,
    order: &[&str],
) -> String {
    let rank = |name: &str| order.iter().position(|k| *k == name).unwrap_or(usize::MAX);
    let mut children = vec![];
    for c in n.children() {
        if c.tag_name().namespace() == Some(W) && changes.contains_key(c.tag_name().name()) {
            continue;
        }
        children.push((rank(c.tag_name().name()), raw(xml, c).to_string()));
    }
    for (name, text) in changes {
        if !text.is_empty() {
            children.push((rank(name), text.clone()));
        }
    }
    children.sort_by_key(|(rank, _)| *rank);
    let result = replace_inner(
        xml,
        n,
        &children.into_iter().map(|(_, s)| s).collect::<String>(),
    );
    let d = parse(&result).expect("patch preserves well-formed XML");
    // Callers pass the element being patched as the document root.
    raw(&result, d.root_element()).to_string()
}

fn element(tag: &str, value: &Value) -> String {
    if value.is_null() {
        return String::new();
    }
    let val = if let Some(b) = value.as_bool() {
        if b {
            "1".into()
        } else {
            "0".into()
        }
    } else if let Some(s) = value.as_str() {
        s.into()
    } else {
        value.to_string()
    };
    format!("<w:{tag} xmlns:w=\"{W}\" w:val=\"{}\"/>", esc(&val))
}
fn patch_run(xml: &str, after: &Value, before: &Value) -> Result<String, String> {
    let d = parse(xml)?;
    let mut changes = BTreeMap::new();
    for (tag, key) in RUN {
        if after[*key] != before[*key] {
            changes.insert(*tag, element(tag, &after[*key]));
        }
    }
    for tag in [
        "sz",
        "szCs",
        "spacing",
        "position",
        "kern",
        "u",
        "vertAlign",
        "highlight",
    ] {
        if after[tag] != before[tag] {
            changes.insert(tag, element(tag, &after[tag]));
        }
    }
    for tag in ["rFonts", "color"] {
        if after[tag] == before[tag] {
            continue;
        }
        let original = wc(d.root_element(), tag);
        let mut attrs = String::new();
        let known = if tag == "rFonts" {
            &[
                "ascii",
                "hAnsi",
                "eastAsia",
                "cs",
                "asciiTheme",
                "hAnsiTheme",
                "eastAsiaTheme",
                "cstheme",
                "hint",
            ][..]
        } else {
            &["val", "themeColor", "themeTint", "themeShade"][..]
        };
        let mut ns = format!(" xmlns:w=\"{W}\"");
        let mut inner = String::new();
        // Namespace bindings can be declared only on this leaf. Retain them
        // when replacing known attributes, together with unknown attributes.
        if let Some(n) = original {
            for binding in n.namespaces().filter(|ns| ns.name() != Some("w")) {
                ns.push_str(&format!(
                    " xmlns{}=\"{}\"",
                    binding.name().map(|s| format!(":{s}")).unwrap_or_default(),
                    esc(binding.uri())
                ));
            }
            for a in n
                .attributes()
                .filter(|a| a.namespace() != Some(W) || !known.contains(&a.name()))
            {
                let name = a
                    .namespace()
                    .and_then(|uri| n.lookup_prefix(uri))
                    .map(|p| format!("{p}:{}", a.name()))
                    .unwrap_or(a.name().into());
                attrs.push_str(&format!(" {name}=\"{}\"", esc(a.value())));
            }
            inner = n.children().map(|n| raw(xml, n)).collect();
        }
        for (key, v) in after[tag].as_object().into_iter().flatten() {
            attrs.push_str(&format!(" w:{key}=\"{}\"", esc(v.as_str().unwrap_or(""))));
        }
        changes.insert(
            tag,
            if after[tag].is_null() && attrs.is_empty() && inner.is_empty() {
                String::new()
            } else {
                format!("<w:{tag}{ns}{attrs}>{inner}</w:{tag}>")
            },
        );
    }
    if changes.is_empty() {
        return Ok(xml.into());
    }
    Ok(patch_children(
        xml,
        d.root_element(),
        &changes,
        &[
            "rStyle",
            "rFonts",
            "b",
            "bCs",
            "i",
            "iCs",
            "caps",
            "smallCaps",
            "strike",
            "dstrike",
            "outline",
            "shadow",
            "emboss",
            "imprint",
            "noProof",
            "snapToGrid",
            "vanish",
            "webHidden",
            "color",
            "spacing",
            "w",
            "kern",
            "position",
            "sz",
            "szCs",
            "highlight",
            "u",
            "effect",
            "bdr",
            "shd",
            "fitText",
            "vertAlign",
            "rtl",
            "cs",
            "em",
            "lang",
            "eastAsianLayout",
            "specVanish",
            "oMath",
            "rPrChange",
        ],
    ))
}
fn patch_style(xml: &str, after: &Value, before: &Value) -> Result<String, String> {
    let d = parse(xml)?;
    let root = d.root_element();
    let mut changes = BTreeMap::new();
    for (tag, key) in [
        ("name", "name"),
        ("basedOn", "basedOn"),
        ("next", "next"),
        ("link", "link"),
    ] {
        if after[key] != before[key] {
            changes.insert(tag, element(tag, &after[key]));
        }
    }
    let namespaces = root
        .namespaces()
        .filter(|ns| !matches!(ns.name(), Some("w" | "r")))
        .map(|ns| {
            format!(
                " xmlns{}=\"{}\"",
                ns.name().map(|s| format!(":{s}")).unwrap_or_default(),
                esc(ns.uri())
            )
        })
        .collect::<String>();
    let a = &after["properties"];
    let b = &before["properties"];
    if a["paragraph"] != b["paragraph"]
        || a["tabs"] != b["tabs"]
        || a["alignment"] != b["alignment"]
        || a["outlineLevel"] != b["outlineLevel"]
    {
        let old = wc(root, "pPr")
            .map(|n| standalone_xml(xml, n))
            .unwrap_or(format!("<w:pPr xmlns:w=\"{W}\"/>"));
        // docx_paragraph patches only changed declared attributes.
        let node = json!({"attrs":{"paragraphFormat":{"direct":a["paragraph"]}}});
        let prev = json!({"attrs":{"paragraphFormat":{"direct":b["paragraph"]}}});
        let mut p = crate::docx_paragraph::patch(old, &namespaces, &node, Some(&prev));
        for (tag, key) in [("jc", "alignment"), ("outlineLvl", "outlineLevel")] {
            if a[key] != b[key] {
                p = crate::docx::set_prop(&p, &namespaces, tag, element(tag, &a[key]));
            }
        }
        if a["tabs"] != b["tabs"] {
            p = crate::docx::set_prop(
                &p,
                &namespaces,
                "tabs",
                crate::docx_tabs::xml(&json!({"direct":a["tabs"]})),
            );
        }
        changes.insert("pPr", p);
    }
    if a["run"] != b["run"] {
        let old = wc(root, "rPr")
            .map(|n| standalone_xml(xml, n))
            .unwrap_or(format!("<w:rPr xmlns:w=\"{W}\"/>"));
        changes.insert("rPr", patch_run(&old, &a["run"], &b["run"])?);
    }
    Ok(patch_children(
        xml,
        root,
        &changes,
        &[
            "name",
            "aliases",
            "basedOn",
            "next",
            "link",
            "autoRedefine",
            "hidden",
            "uiPriority",
            "semiHidden",
            "unhideWhenUsed",
            "qFormat",
            "locked",
            "personal",
            "personalCompose",
            "personalReply",
            "rsid",
            "pPr",
            "rPr",
            "tblPr",
            "trPr",
            "tcPr",
            "tblStylePr",
        ],
    ))
}
/// Patch only changed declarations in the source style part. No-op exports keep
/// the original ZIP; unrelated style nodes and package parts stay untouched.
pub fn write(document: &Value, pkg: &mut Package) -> Result<(), String> {
    let Some(after) = document["attrs"].get(KEY) else {
        return Ok(());
    };
    let before = read(pkg)?;
    if *after == before {
        return Ok(());
    }
    validate(after)?;
    if after["defaults"] != before["defaults"]
        || after["defaultStyle"] != before["defaultStyle"]
        || after["theme"] != before["theme"]
    {
        return Err("Changing document-wide style defaults is not yet supported".into());
    }
    let path = crate::docx_tabs::part(pkg, "styles")?;
    let xml = pkg
        .text(&path)
        .map(String::from)
        .unwrap_or(format!("<w:styles xmlns:w=\"{W}\"/>"));
    let d = parse(&xml)?;
    let mut inner = String::new();
    let mut seen = HashSet::new();
    for n in d.root_element().children() {
        if n.has_tag_name((W, "style")) {
            if let Some(id) = n.attribute((W, "styleId")) {
                seen.insert(id.to_string());
                match style(after, id) {
                    Some(a) if Some(a) == style(&before, id) => inner.push_str(raw(&xml, n)),
                    Some(a) => inner.push_str(&patch_style(
                        &standalone_xml(&xml, n),
                        a,
                        style(&before, id).unwrap_or(&Value::Null),
                    )?),
                    None => {}
                }
                continue;
            }
        }
        inner.push_str(raw(&xml, n));
    }
    for a in after["styles"].as_array().unwrap() {
        let id = a["id"].as_str().unwrap();
        if !seen.contains(id) {
            let stub = format!(
                "<w:style xmlns:w=\"{W}\" w:type=\"{}\" w:customStyle=\"{}\" w:styleId=\"{}\"/>",
                esc(a["type"].as_str().unwrap()),
                if a["customStyle"] == false { "0" } else { "1" },
                esc(id)
            );
            inner.push_str(&patch_style(&stub, a, &Value::Null)?);
        }
    }
    pkg.set(&path, replace_inner(&xml, d.root_element(), &inner));
    let rels = pkg
        .text("word/_rels/document.xml.rels")
        .map(String::from)
        .unwrap_or(format!("<Relationships xmlns=\"{REL_NS}\"/>"));
    let rel = parse(&rels)?;
    let root = rel.root_element();
    let typ = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
    if !root
        .children()
        .any(|n| attr(n, "Type") == Some(typ) && attr(n, "TargetMode") != Some("External"))
    {
        let mut i = 1;
        while root
            .children()
            .any(|n| attr(n, "Id") == Some(&format!("folioStyles{i}")))
        {
            i += 1;
        }
        let inner = root.children().map(|n| raw(&rels, n)).collect::<String>()
            + &format!(
                "<Relationship Id=\"folioStyles{i}\" Type=\"{typ}\" Target=\"/{}\"/>",
                esc(&path)
            );
        pkg.set(
            "word/_rels/document.xml.rels",
            replace_inner(&rels, root, &inner),
        );
    }
    let types = pkg.text("[Content_Types].xml")?.to_string();
    let d = parse(&types)?;
    let root = d.root_element();
    let name = format!("/{path}");
    if !root.children().any(|n| attr(n, "PartName") == Some(&name)) {
        let inner=root.children().map(|n|raw(&types,n)).collect::<String>()+&format!("<Override PartName=\"{}\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>",esc(&name));
        pkg.set("[Content_Types].xml", replace_inner(&types, root, &inner));
    }
    Ok(())
}
pub fn hydrate(document: &mut Value, source: &Value) {
    if document["attrs"].get(KEY).is_none() {
        if let Some(c) = source["attrs"].get(KEY) {
            document["attrs"][KEY] = c.clone();
        }
    }
    fn collect(n: &Value, out: &mut std::collections::HashMap<String, Value>) {
        if let Some(id) = n["attrs"]["officeId"].as_str() {
            out.insert(id.into(), n["attrs"]["paragraphFormat"].clone());
        }
        for child in n["content"].as_array().into_iter().flatten() {
            collect(child, out);
        }
    }
    fn apply(n: &mut Value, source: &std::collections::HashMap<String, Value>) {
        if let Some(old) = n["attrs"]["officeId"]
            .as_str()
            .and_then(|id| source.get(id))
        {
            if n["attrs"]["paragraphFormat"]["styleId"] == old["styleId"]
                && n["attrs"]["paragraphFormat"].is_object()
            {
                for key in [
                    "inheritedRun",
                    "inheritedAlignment",
                    "inheritedOutlineLevel",
                ] {
                    if n["attrs"]["paragraphFormat"].get(key).is_none() {
                        if let Some(v) = old.get(key) {
                            n["attrs"]["paragraphFormat"][key] = v.clone();
                        }
                    }
                }
            }
        }
        for child in n
            .get_mut("content")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            apply(child, source);
        }
    }
    let mut values = std::collections::HashMap::new();
    collect(source, &mut values);
    apply(document, &values);
}
/// A revision-checked workspace command is the only style-catalog edit route.
/// It preserves IDs on rename and rejects changed dangling/cyclic inheritance.
pub fn edit(catalog: &Value, request: &Value) -> Result<Value, String> {
    let mut c = catalog.clone();
    match request["action"].as_str() {
        Some("upsert") => {
            let mut s = request["style"].clone();
            if let Some(old) = s["id"].as_str().and_then(|id| style(&c, id)) {
                if old["type"] != s["type"] || old["default"] != s["default"] {
                    return Err("A style's type and default status cannot change".into());
                }
                if s.get("customStyle").is_some() && s["customStyle"] != old["customStyle"] {
                    return Err("A style's built-in status cannot change".into());
                }
                s["customStyle"] = old["customStyle"].clone();
            } else {
                if s.get("default").is_some() {
                    return Err("New styles cannot replace the document default".into());
                }
                s["customStyle"] = json!(true);
            }
            let styles = c["styles"].as_array_mut().ok_or("Invalid style catalog")?;
            if let Some(i) = styles.iter().position(|x| x["id"] == s["id"]) {
                styles[i] = s.clone();
            } else {
                styles.push(s.clone());
            }
            validate(&c)?;
            for key in ["basedOn", "next", "link"] {
                if let Some(id) = s[key].as_str() {
                    let target =
                        style(&c, id).ok_or(format!("Style {key} refers to a missing style"))?;
                    if (key == "basedOn" || key == "next") && target["type"] != s["type"] {
                        return Err("Related styles must have matching types".into());
                    }
                }
            }
            let mut seen = HashSet::new();
            let mut current = Some(&s);
            while let Some(s) = current {
                let id = s["id"].as_str().unwrap();
                if !seen.insert(id) {
                    return Err("Styles cannot inherit from themselves or form a cycle".into());
                }
                current = s["basedOn"].as_str().and_then(|id| style(&c, id));
            }
        }
        Some("delete") => {
            let id = request["styleId"]
                .as_str()
                .ok_or("Missing style identity")?;
            let s = style(&c, id).ok_or("Style no longer exists")?;
            if s["default"] == true || s["customStyle"] == false {
                return Err("Built-in and default styles can be modified, but not deleted".into());
            }
            if let Some(replacement) = request.get("replacementStyleId") {
                let target = replacement
                    .as_str()
                    .and_then(|id| style(&c, id))
                    .filter(|target| target["type"] == "paragraph" && s["type"] == "paragraph")
                    .ok_or("Choose an existing paragraph style as the replacement")?;
                if target["id"] == id {
                    return Err("Choose a different replacement style".into());
                }
                let mut changed = vec![];
                for other in c["styles"].as_array_mut().unwrap() {
                    if other["id"] == id {
                        continue;
                    }
                    for key in ["basedOn", "next", "link"] {
                        if other[key] == id {
                            other[key] = replacement.clone();
                            if key == "basedOn" {
                                changed.push(other["id"].clone());
                            }
                        }
                    }
                }
                // Replacing a base by its own descendant would introduce a
                // cycle. Unrelated malformed imported chains remain preserved.
                for start in changed {
                    let mut seen = HashSet::new();
                    let mut current = start.as_str().and_then(|id| style(&c, id));
                    while let Some(s) = current {
                        if !seen.insert(s["id"].as_str().unwrap()) {
                            return Err(
                                "The replacement would create a style inheritance cycle".into()
                            );
                        }
                        current = s["basedOn"].as_str().and_then(|id| style(&c, id));
                    }
                }
            } else if c["styles"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| ["basedOn", "next", "link"].iter().any(|k| s[*k] == id))
            {
                return Err(
                    "This style is referenced by another style. Choose a replacement.".into(),
                );
            }
            c["styles"]
                .as_array_mut()
                .unwrap()
                .retain(|s| s["id"] != id);
            validate(&c)?;
        }
        _ => return Err("Unknown style action".into()),
    }
    Ok(c)
}

pub fn command(
    document: &Value,
    request: &Value,
    pkg: &mut Package,
    baseline: Option<&Value>,
) -> Result<Value, String> {
    let mut doc = document.clone();
    let raw_catalog = doc["attrs"].get(KEY).cloned().unwrap_or(read(pkg)?);
    let mut catalog = with_origins(&raw_catalog, &read(pkg)?);
    if request["action"] == "apply" {
        let id = request["styleId"]
            .as_str()
            .ok_or("Missing style identity")?;
        crate::builtin_styles::materialize(&mut catalog, id);
        let s = style(&catalog, id)
            .filter(|s| s["type"] == "paragraph")
            .ok_or("Choose an existing paragraph style")?;
        let resolved = resolve(&catalog, Some(id));
        let paths = request["paths"]
            .as_array()
            .filter(|a| !a.is_empty() && a.len() <= 10000)
            .ok_or("Select at least one paragraph")?;
        let mut seen = HashSet::new();
        for path in paths {
            let indices = path
                .as_array()
                .filter(|a| !a.is_empty() && a.len() <= 40)
                .ok_or("Invalid paragraph path")?;
            if !seen.insert(path.to_string()) {
                return Err("Duplicate paragraph selection".into());
            }
            let mut node = &mut doc;
            for i in indices {
                node = node
                    .get_mut("content")
                    .and_then(Value::as_array_mut)
                    .and_then(|a| i.as_u64().and_then(|i| a.get_mut(i as usize)))
                    .ok_or("The selected paragraph no longer exists")?;
            }
            if !matches!(node["type"].as_str(), Some("paragraph" | "heading")) {
                return Err("Select editable paragraphs to apply a style".into());
            }
            node["attrs"]["paragraphFormat"]["styleId"] = s["id"].clone();
            let level = node["attrs"]["paragraphFormat"]["direct"]["outlineLevel"]
                .as_u64()
                .or_else(|| resolved["outlineLevel"].as_u64());
            crate::docx_paragraph::set_outline(node, level);
        }
    } else {
        if request["action"] == "delete" && request.get("replacementStyleId").is_none() {
            fn used(n: &Value, id: &Value) -> bool {
                n["attrs"]["paragraphFormat"]["styleId"] == *id
                    || n["content"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|n| used(n, id))
            }
            if used(&doc, &request["styleId"]) {
                return Err("This style is still used by the document".into());
            }
            // Preserved headers, fields, footnotes and character styles can also
            // refer to this identity. Reject rather than silently orphan them.
            let id = request["styleId"]
                .as_str()
                .ok_or("Missing style identity")?;
            let effective =
                Package::from_bytes(crate::docx::export(document, Some(pkg), baseline)?)?;
            for (name, bytes) in &effective.entries {
                if name.ends_with(".xml") && name != &crate::docx_tabs::part(pkg, "styles")? {
                    if let Ok(xml) = std::str::from_utf8(bytes) {
                        if let Ok(d) = parse(xml) {
                            if d.descendants().any(|n| {
                                n.tag_name().namespace() == Some(W)
                                    && ["pStyle", "rStyle", "tblStyle"]
                                        .contains(&n.tag_name().name())
                                    && n.attribute((W, "val")) == Some(id)
                            }) {
                                return Err("This style is used in preserved Word content".into());
                            }
                        }
                    }
                }
            }
        }
        catalog = edit(&catalog, request)?;
        if request["action"] == "delete" {
            if let Some(replacement) = request["replacementStyleId"].as_str() {
                let id = request["styleId"].as_str().unwrap();
                replace_paragraph_style(&mut doc, id, replacement, &catalog);
                replace_package_references(pkg, id, replacement)?;
            }
        }
    }
    if doc == *document && document["attrs"].get(KEY) == Some(&catalog) {
        return Ok(doc);
    }
    doc["attrs"][KEY] = catalog;
    write(&doc, pkg)?;
    crate::docx_tabs::Catalog::read(pkg)?.refresh(&mut doc)?;
    Ok(doc)
}

/// Only schema-defined style references change. Text, field instructions,
/// foreign attributes, namespaces and unrelated XML remain byte-for-byte intact.
fn replace_package_references(
    pkg: &mut Package,
    id: &str,
    replacement: &str,
) -> Result<(), String> {
    for (name, bytes) in &mut pkg.entries {
        if !name.ends_with(".xml") {
            continue;
        }
        let Ok(xml) = std::str::from_utf8(bytes) else {
            continue;
        };
        let Ok(dom) = parse(xml) else {
            continue;
        };
        let edits = dom
            .descendants()
            .filter(|n| {
                n.tag_name().namespace() == Some(W)
                    && [
                        "pStyle",
                        "rStyle",
                        "tblStyle",
                        "basedOn",
                        "next",
                        "link",
                        "numStyleLink",
                        "styleLink",
                    ]
                    .contains(&n.tag_name().name())
            })
            .filter_map(|n| {
                n.attributes()
                    .find(|a| a.namespace() == Some(W) && a.name() == "val" && a.value() == id)
            })
            .map(|a| (a.range_value(), esc(replacement)))
            .collect::<Vec<_>>();
        if !edits.is_empty() {
            *bytes = replace_ranges(xml, edits).into_bytes();
        }
    }
    Ok(())
}
fn replace_paragraph_style(node: &mut Value, id: &str, replacement: &str, catalog: &Value) {
    let implicit = if node["type"] == "heading" {
        format!("Heading{}", node["attrs"]["level"].as_u64().unwrap_or(1))
    } else {
        catalog["defaultStyle"]
            .as_str()
            .unwrap_or("Normal")
            .to_string()
    };
    if matches!(node["type"].as_str(), Some("paragraph" | "heading"))
        && node["attrs"]["paragraphFormat"]["styleId"]
            .as_str()
            .unwrap_or(&implicit)
            == id
    {
        node["attrs"]["paragraphFormat"]["styleId"] = json!(replacement);
        let resolved = resolve(catalog, Some(replacement));
        let level = node["attrs"]["paragraphFormat"]["direct"]["outlineLevel"]
            .as_u64()
            .or_else(|| resolved["outlineLevel"].as_u64());
        crate::docx_paragraph::set_outline(node, level);
    }
    for child in node
        .get_mut("content")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
    {
        replace_paragraph_style(child, id, replacement, catalog);
    }
}

/// Upgrade origin metadata in memory using the preserved source. IDs absent
/// from that source were created locally; do not misclassify them as built in.
pub fn with_origins(catalog: &Value, source: &Value) -> Value {
    let mut result = catalog.clone();
    for s in result["styles"].as_array_mut().into_iter().flatten() {
        if s.get("customStyle").is_none() {
            s["customStyle"] = s["id"]
                .as_str()
                .and_then(|id| style(source, id))
                .map(|s| s["customStyle"].clone())
                .unwrap_or(json!(true));
        }
    }
    result
}
