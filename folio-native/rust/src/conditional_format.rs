//! Conditional formats are derived display overlays, never ordinary cell styles.
use crate::{
    calculation::WorkbookCalculator,
    formula::{address, parse_address},
    office::*,
    tables::Area,
};
use crate::{conditional_data_bar as bars, conditional_icons as icons};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const TYPES: &[&str] = &[
    "cellIs",
    "expression",
    "containsText",
    "notContainsText",
    "beginsWith",
    "endsWith",
    "containsBlanks",
    "notContainsBlanks",
    "containsErrors",
    "notContainsErrors",
    "duplicateValues",
    "uniqueValues",
    "top10",
    "aboveAverage",
    "colorScale",
    "dataBar",
    "iconSet",
    "timePeriod",
];
const OPS: &[&str] = &[
    "between",
    "notBetween",
    "equal",
    "notEqual",
    "lessThan",
    "lessThanOrEqual",
    "greaterThan",
    "greaterThanOrEqual",
];
const ATTRS: &[&str] = &[
    "type",
    "operator",
    "priority",
    "stopIfTrue",
    "text",
    "rank",
    "percent",
    "bottom",
    "aboveAverage",
    "equalAverage",
    "stdDev",
    "timePeriod",
];
const BOOLS: &[&str] = &[
    "stopIfTrue",
    "percent",
    "bottom",
    "aboveAverage",
    "equalAverage",
];
const INTS: &[&str] = &["priority", "rank", "stdDev"];
const STYLE_KEYS: &[&str] = &[
    "bold",
    "italic",
    "strike",
    "underline",
    "color",
    "fill",
    "numFmt",
];
pub fn rules(s: &Value) -> impl Iterator<Item = &Value> {
    s.get("conditionalFormats")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}
pub(crate) fn areas(r: &Value) -> Result<Vec<Area>, String> {
    let text = r["sqref"]
        .as_str()
        .ok_or("Conditional-format range missing")?;
    if text.len() > 65536 {
        return Err("Conditional-format range is too long".into());
    }
    let a = text
        .split_whitespace()
        .map(Area::parse)
        .collect::<Result<Vec<_>, _>>()?;
    if a.is_empty() {
        return Err("Choose at least one cell range".into());
    }
    Ok(a)
}
fn flag(r: &Value, key: &str) -> bool {
    r[key].as_bool().unwrap_or(false)
}
fn formulas(r: &Value) -> Vec<String> {
    r["formulas"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect()
}
fn rgb(n: roxmltree::Node<'_, '_>) -> Option<String> {
    if n.attributes()
        .any(|a| a.namespace().is_none() && a.name() != "rgb")
    {
        return None;
    }
    let text = attr(n, "rgb")?;
    if text.len() != 8 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", &text[2..]))
}
fn read_style(xml: &str) -> Result<(Value, bool), String> {
    let d = parse(xml)?;
    let mut style = Map::new();
    let mut unsupported = false;
    for n in d.root_element().children().filter(|n| n.is_element()) {
        if n.tag_name().namespace() != Some(S) {
            unsupported = true;
            continue;
        }
        match n.tag_name().name() {
            "font" => {
                for p in n.children().filter(|n| n.is_element()) {
                    let key = match p.tag_name().name() {
                        "b" => "bold",
                        "i" => "italic",
                        "strike" => "strike",
                        "u" => "underline",
                        "color" => "color",
                        _ => {
                            unsupported = true;
                            continue;
                        }
                    };
                    if p.tag_name().namespace() != Some(S) {
                        unsupported = true;
                        continue;
                    }
                    let value = match key {
                        "underline" => json!(attr(p, "val").unwrap_or("single")),
                        "color" => {
                            if let Some(color) = rgb(p) {
                                json!(color)
                            } else if attr(p, "auto") == Some("1") {
                                json!("automatic")
                            } else {
                                unsupported = true;
                                continue;
                            }
                        }
                        _ => json!(!matches!(attr(p, "val"), Some("0" | "false"))),
                    };
                    style.insert(key.into(), value);
                }
            }
            "fill" => {
                if let Some(p) = child(n, "patternFill") {
                    if attr(p, "patternType") == Some("solid") {
                        if let Some(color) = child(p, "fgColor").and_then(rgb) {
                            style.insert("fill".into(), json!(color));
                        } else {
                            unsupported = true;
                        }
                    } else {
                        unsupported = true;
                    }
                } else {
                    unsupported = true;
                }
            }
            "numFmt" => {
                if let Some(code) = attr(n, "formatCode") {
                    style.insert("numFmt".into(), json!(code));
                } else {
                    unsupported = true;
                }
            }
            _ => unsupported = true,
        }
    }
    if crate::xlsx_styles::validate_format(&style).is_err() {
        unsupported = true;
    }
    Ok((Value::Object(style), unsupported))
}
fn style_xml(style: &Value) -> Result<String, String> {
    let mut inner = String::new();
    let mut font = String::new();
    for (key, tag) in [("bold", "b"), ("italic", "i"), ("strike", "strike")] {
        if let Some(v) = style[key].as_bool() {
            font += &format!("<{tag} val=\"{}\"/>", u8::from(v));
        }
    }
    if let Some(v) = style["underline"].as_str() {
        font += &format!("<u val=\"{}\"/>", esc(v));
    }
    if let Some(v) = style["color"].as_str() {
        font += &if v == "automatic" {
            "<color auto=\"1\"/>".into()
        } else {
            format!("<color rgb=\"FF{}\"/>", &v[1..])
        };
    }
    if !font.is_empty() {
        inner += &format!("<font>{font}</font>");
    }
    if let Some(v) = style["numFmt"].as_str() {
        inner += &format!("<numFmt numFmtId=\"164\" formatCode=\"{}\"/>", esc(v));
    }
    if let Some(v) = style["fill"].as_str() {
        inner += &format!("<fill><patternFill patternType=\"solid\"><fgColor rgb=\"FF{}\"/><bgColor indexed=\"64\"/></patternFill></fill>", &v[1..]);
    }
    Ok(format!("<dxf xmlns=\"{S}\">{inner}</dxf>"))
}
// Patch only modeled differential properties; preserve vendor attributes and
// unknown metadata when a user changes a supported color/font/number format.
fn merge_style_xml(source: &str, replacement: &str) -> Result<String, String> {
    let old = parse(source)?;
    let new = parse(replacement)?;
    let a = old.root_element();
    let b = new.root_element();
    let mut attrs = b
        .attributes()
        .filter(|a| a.namespace().is_none())
        .map(|a| (a.name(), Some(a.value().to_string())))
        .collect::<Vec<_>>();
    if matches!(b.tag_name().name(), "color" | "fgColor") {
        for name in ["rgb", "auto", "indexed", "theme", "tint"] {
            if b.attribute(name).is_none() {
                attrs.push((name, None));
            }
        }
    }
    let base = attributes(source, &attrs)?;
    let controlled: &[&str] = match b.tag_name().name() {
        "dxf" => &["font", "numFmt", "fill"],
        "font" => &["b", "i", "strike", "u", "color"],
        "fill" => &["patternFill"],
        "patternFill" => &["fgColor", "bgColor"],
        _ => &[],
    };
    if controlled.is_empty() {
        return Ok(base);
    }
    let mut inner = String::new();
    for n in b.children().filter(|n| n.is_element()) {
        let fresh = scoped_record(replacement, n);
        inner += &if let Some(prior) = a
            .children()
            .find(|p| p.tag_name() == n.tag_name() && p.is_element())
        {
            merge_style_xml(&scoped_record(source, prior), &fresh)?
        } else {
            fresh
        };
    }
    for n in a.children().filter(|n| {
        !n.is_element()
            || n.tag_name().namespace() != Some(S)
            || !controlled.contains(&n.tag_name().name())
    }) {
        inner += &if n.is_element() {
            scoped_record(source, n)
        } else {
            raw(source, n).to_string()
        };
    }
    crate::xlsx::fragment_inner(&base, &inner)
}

pub fn read(pkg: &Package, path: &str, sheet: &mut Value) -> Result<(), String> {
    let xml = pkg.text(path)?;
    let doc = parse(xml)?;
    let styles = pkg
        .entries
        .get("xl/styles.xml")
        .map(|_| pkg.text("xl/styles.xml"))
        .transpose()?;
    let sd = styles.map(parse).transpose()?;
    let dxfs: Vec<_> = sd
        .as_ref()
        .and_then(|d| child(d.root_element(), "dxfs"))
        .into_iter()
        .flat_map(|n| n.children())
        .filter(|n| n.has_tag_name((S, "dxf")))
        .collect();
    let mut list = vec![];
    for group in doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((S, "conditionalFormatting")))
    {
        let group_xml = crate::xlsx::fragment_inner(&scoped_record(xml, group), "")?;
        let group_extras = group
            .children()
            .filter(|n| n.is_element() && !n.has_tag_name((S, "cfRule")))
            .map(|n| scoped_record(xml, n))
            .collect::<String>();
        for (ordinal, n) in group
            .children()
            .filter(|n| n.has_tag_name((S, "cfRule")))
            .enumerate()
        {
            let mut r = json!({"sqref":attr(group,"sqref").unwrap_or(""), "officeXml":scoped_record(xml,n), "groupXml":group_xml,"style":{}});
            if ordinal == 0 && !group_extras.is_empty() {
                r["groupExtras"] = json!(group_extras);
            }
            for key in ATTRS {
                if let Some(v) = attr(n, key) {
                    r[key] = if BOOLS.contains(key) {
                        json!(matches!(v, "1" | "true"))
                    } else if INTS.contains(key) {
                        v.parse::<u64>().map(|v| json!(v)).unwrap_or(json!(v))
                    } else {
                        json!(v)
                    };
                }
            }
            r["formulas"] = json!(n
                .children()
                .filter(|n| n.has_tag_name((S, "formula")))
                .map(|n| n.text().unwrap_or("").to_string())
                .collect::<Vec<_>>());
            let mut unsupported = attr(group, "pivot").is_some_and(|v| matches!(v, "1" | "true"));
            if let Some(id) = attr(n, "dxfId") {
                if let Some(dxf) = id.parse::<usize>().ok().and_then(|i| dxfs.get(i)) {
                    let original = scoped_record(styles.unwrap(), *dxf);
                    let (style, unknown) = read_style(&original)?;
                    unsupported |= unknown;
                    r["style"] = style;
                    r["officeDxfXml"] = json!(original);
                } else {
                    unsupported = true;
                }
            }
            if let Some(scale) = child(n, "colorScale") {
                let mut points = vec![];
                let thresholds: Vec<_> = scale
                    .children()
                    .filter(|n| n.has_tag_name((S, "cfvo")))
                    .collect();
                let colors: Vec<_> = scale
                    .children()
                    .filter(|n| n.has_tag_name((S, "color")))
                    .collect();
                if colors.len() != thresholds.len() {
                    unsupported = true;
                }
                for (t, c) in thresholds.iter().zip(colors) {
                    let color = rgb(c);
                    if color.is_none() {
                        unsupported = true;
                    }
                    points.push(json!({"type":attr(*t,"type").unwrap_or(""),"value":attr(*t,"val").unwrap_or(""),"color":color}));
                }
                r["scale"] = json!(points);
            }
            if let Some(bar) = child(n, "dataBar") {
                match bars::read_standard(bar) {
                    Ok(bar) => r["dataBar"] = bar,
                    Err(_) => unsupported = true,
                }
                if let Some(id) = bars::link(n) {
                    r["officeDataBarId"] = json!(id);
                }
            }
            if let Some(icon) = child(n, "iconSet") {
                match icons::read(icon) {
                    Ok(icon) => r["iconSet"] = icon,
                    Err(_) => unsupported = true,
                }
            }
            unsupported |= n.children().any(|n| {
                n.is_element()
                    && !bars::link_container(n)
                    && (!matches!(
                        n.tag_name().name(),
                        "formula" | "colorScale" | "dataBar" | "iconSet"
                    ) || n.tag_name().namespace() != Some(S))
            });
            if unsupported || r["priority"].as_u64().is_none() || validate_rule(&r).is_err() {
                r["unsupported"] = json!(true);
            }
            list.push(r);
        }
    }
    sheet["extendedConditionalFormats"] = json!(bars::read_extensions(&doc, &mut list));
    for r in &mut list {
        if validate_rule(r).is_err() {
            r["unsupported"] = json!(true);
        }
    }
    sheet["conditionalRightToLeft"] = json!(doc
        .descendants()
        .find(|n| n.has_tag_name((S, "sheetView")) && attr(*n, "workbookViewId") == Some("0"))
        .is_some_and(|n| matches!(attr(n, "rightToLeft"), Some("1" | "true"))));
    list.sort_by_key(|r| r["priority"].as_u64().unwrap_or(u64::MAX));
    sheet["conditionalFormats"] = json!(list);
    Ok(())
}
pub fn hydrate(pkg: &Package, path: &str, sheet: &mut Value) -> Result<(), String> {
    if sheet.get("conditionalFormats").is_none() {
        return read(pkg, path, sheet);
    }
    // Older recovery files kept graphical rules opaque. Only upgrade unchanged
    // source records. Extended rules could not be edited while the flag was set.
    let upgrade = |r: &Value| {
        matches!(
            r["type"].as_str(),
            Some("dataBar" | "iconSet" | "timePeriod")
        ) && r["unsupported"] == true
            && (r["type"] == "timePeriod" || r.get(r["type"].as_str().unwrap()).is_none())
    };
    if rules(sheet).any(upgrade) || flag(sheet, "extendedConditionalFormats") {
        let mut fresh = json!({});
        read(pkg, path, &mut fresh)?;
        let had_extensions = flag(sheet, "extendedConditionalFormats");
        let list = sheet["conditionalFormats"].as_array_mut().unwrap();
        for rule in &mut *list {
            if upgrade(rule) {
                if let Some(new) = rules(&fresh).find(|r| {
                    r["officeXml"] == rule["officeXml"]
                        && r["sqref"] == rule["sqref"]
                        && r["unsupported"] != true
                }) {
                    let priority = rule["priority"].clone();
                    *rule = new.clone();
                    rule["priority"] = priority;
                }
            }
        }
        if had_extensions {
            for rule in rules(&fresh).filter(|r| r.get("officeIconSetId").is_some()) {
                if !list
                    .iter()
                    .any(|r| r["officeIconSetId"] == rule["officeIconSetId"])
                {
                    list.push(rule.clone());
                }
            }
            list.sort_by_key(|r| r["priority"].as_u64().unwrap_or(u64::MAX));
        }
        for k in ["extendedConditionalFormats", "conditionalRightToLeft"] {
            sheet[k] = fresh[k].clone();
        }
    }
    Ok(())
}
pub(crate) fn validate_rule(r: &Value) -> Result<(), String> {
    areas(r)?;
    let kind = r["type"]
        .as_str()
        .ok_or("Choose a conditional-format type")?;
    if !TYPES.contains(&kind) {
        return Err("This conditional-format type is preserved but not editable yet".into());
    }
    let style = r["style"]
        .as_object()
        .ok_or("Conditional format needs a style")?;
    if style.keys().any(|k| !STYLE_KEYS.contains(&k.as_str())) {
        return Err("Unsupported conditional style property".into());
    }
    crate::xlsx_styles::validate_format(style)?;
    for key in BOOLS {
        if r.get(*key).is_some_and(|v| !v.is_boolean()) {
            return Err(format!("Invalid {key}"));
        }
    }
    for key in INTS {
        if r.get(*key).is_some_and(|v| v.as_u64().is_none()) {
            return Err(format!("Invalid {key}"));
        }
    }
    if r["priority"].as_u64() == Some(0) {
        return Err("Rule priorities start at 1".into());
    }
    if r.get("formulas").is_some_and(|v| {
        v.as_array().is_none_or(|a| {
            a.iter()
                .any(|v| v.as_str().is_none_or(|s| s.is_empty() || s.len() > 8192))
        })
    }) {
        return Err("Invalid conditional-format formula".into());
    }
    let f = formulas(r);
    if kind == "expression" && f.len() != 1 {
        return Err("Enter one formula".into());
    }
    if kind == "timePeriod" {
        if !crate::conditional_dates::PERIODS.contains(&r["timePeriod"].as_str().unwrap_or("")) {
            return Err("Choose a supported date period".into());
        }
        if f.len() > 1 {
            return Err("A date rule accepts at most one formula".into());
        }
    }
    if kind == "cellIs" {
        let op = r["operator"].as_str().unwrap_or("");
        if !OPS.contains(&op) {
            return Err("Choose a comparison".into());
        }
        if f.len()
            != if matches!(op, "between" | "notBetween") {
                2
            } else {
                1
            }
        {
            return Err("Enter the comparison value or formula".into());
        }
    }
    if matches!(
        kind,
        "containsText" | "notContainsText" | "beginsWith" | "endsWith"
    ) && r["text"]
        .as_str()
        .is_none_or(|t| t.is_empty() || t.len() > 8192)
    {
        return Err("Enter text to match".into());
    }
    if kind == "top10"
        && r["rank"]
            .as_u64()
            .is_none_or(|v| v == 0 || v > if flag(r, "percent") { 100 } else { 1000 })
    {
        return Err("Enter a rank from 1 to 1000, or a percentage from 1 to 100".into());
    }
    if r["stdDev"].as_u64().is_some_and(|v| v > 3) {
        return Err("Choose 0 to 3 standard deviations".into());
    }
    if kind == "colorScale" {
        let points = r["scale"].as_array().ok_or("Color scale missing")?;
        if !(2..=3).contains(&points.len()) {
            return Err("Choose a two- or three-color scale".into());
        }
        for (i, p) in points.iter().enumerate() {
            let t = p["type"].as_str().unwrap_or("");
            if !["min", "max", "num", "percent", "percentile", "formula"].contains(&t)
                || t == "min" && i != 0
                || t == "max" && i != points.len() - 1
            {
                return Err("Invalid color-scale threshold".into());
            }
            if matches!(t, "num" | "percent" | "percentile")
                && p["value"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .is_none_or(|v| !v.is_finite() || t != "num" && !(0.0..=100.0).contains(&v))
            {
                return Err("Invalid color-scale value".into());
            }
            if t == "formula"
                && p["value"]
                    .as_str()
                    .is_none_or(|s| s.is_empty() || s.len() > 8192)
            {
                return Err("Enter a threshold formula".into());
            }
            crate::xlsx_styles::validate_format(&Map::from_iter([(
                "fill".into(),
                p["color"].clone(),
            )]))?;
        }
    }
    if kind == "dataBar" {
        bars::validate(&r["dataBar"])?;
    }
    if kind == "iconSet" {
        icons::validate(&r["iconSet"])?;
        icons::validate_promotion(r)?;
    }
    Ok(())
}
pub fn validate(s: &Value) -> Result<(), String> {
    if let Some(v) = s.get("conditionalFormats") {
        let list = v.as_array().ok_or("Invalid conditional formats")?;
        if list.len() > 4096 {
            return Err("Too many conditional-format rules".into());
        }
        for r in list {
            if !flag(r, "unsupported") {
                validate_rule(r)?;
            }
        }
    }
    Ok(())
}
pub fn bind(sheets: &mut Value, projection: &Value) {
    for s in sheets.as_array_mut().into_iter().flatten() {
        if let Some(p) = projection
            .as_array()
            .into_iter()
            .flatten()
            .find(|p| p["name"] == s["name"])
        {
            for k in [
                "conditionalFormats",
                "extendedConditionalFormats",
                "conditionalRightToLeft",
            ] {
                if let Some(v) = p.get(k) {
                    s[k] = v.clone();
                }
            }
        }
    }
}
pub fn change(w: &Value, request: &Value) -> Result<Value, String> {
    if request["baseRevision"] != w["revision"] {
        return Err("The workbook changed. Reopen Conditional formatting.".into());
    }
    if request["action"] == "batch" {
        let changes = request["changes"]
            .as_array()
            .ok_or("Missing rule changes")?;
        if changes.len() > 256 {
            return Err("Apply the current changes before making more rule edits".into());
        }
        let mut next = w.clone();
        for change_request in changes {
            if change_request["action"] == "batch" {
                return Err("Nested rule changes are not allowed".into());
            }
            let mut edit = change_request.clone();
            edit["baseRevision"] = w["revision"].clone();
            edit["sheetId"] = request["sheetId"].clone();
            next = change(&next, &edit)?;
        }
        return Ok(next);
    }
    let index = w["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["id"] == request["sheetId"])
        .ok_or("Worksheet missing")?;
    let sheet = &w["sheets"][index];
    if flag(sheet, "extendedConditionalFormats") {
        return Err("This worksheet has extended Excel rules. Edit its rules in Excel until extension editing is supported.".into());
    }
    if request["action"] == "replace" {
        let proposed = request["rules"]
            .as_array()
            .ok_or("Missing proposed rules")?;
        if proposed.len() > 4096 {
            return Err("Too many conditional-format rules".into());
        }
        let original = rules(sheet).collect::<Vec<_>>();
        let mut used = std::collections::HashSet::new();
        let mut final_rules = vec![];
        for item in proposed {
            let origin = item
                .get("origin")
                .filter(|v| !v.is_null())
                .map(|v| v.as_u64().ok_or("Invalid source rule"))
                .transpose()?;
            let rule = if let Some(origin) = origin {
                let origin = origin as usize;
                if origin >= original.len() || !used.insert(origin) {
                    return Err("The source rule list changed".into());
                }
                if item["rule"] == *original[origin] {
                    original[origin].clone()
                } else {
                    let edited = change(
                        w,
                        &json!({"sheetId":request["sheetId"],"baseRevision":request["baseRevision"],"action":"edit","index":origin,"rule":item["rule"]}),
                    )?;
                    edited["sheets"][index]["conditionalFormats"][origin].clone()
                }
            } else {
                let added = change(
                    w,
                    &json!({"sheetId":request["sheetId"],"baseRevision":request["baseRevision"],"action":"add","rule":item["rule"]}),
                )?;
                added["sheets"][index]["conditionalFormats"][0].clone()
            };
            final_rules.push(rule);
        }
        for (i, r) in final_rules.iter_mut().enumerate() {
            r["priority"] = json!(i + 1);
        }
        let mut next = w.clone();
        next["sheets"][index]["conditionalFormats"] = json!(final_rules);
        validate(&next["sheets"][index])?;
        return Ok(next);
    }
    let mut list: Vec<_> = rules(sheet).cloned().collect();
    let action = request["action"].as_str().unwrap_or("add");
    let position = || {
        request["index"]
            .as_u64()
            .map(|v| v as usize)
            .filter(|v| *v < list.len())
            .ok_or("Rule no longer exists")
    };
    match action {
        "add" | "edit" => {
            let mut rule = request["rule"].clone();
            let object = rule.as_object_mut().ok_or("Rule missing")?;
            for k in [
                "officeXml",
                "officeDxfXml",
                "groupXml",
                "groupExtras",
                "officeDataBarId",
                "officeIconSetId",
                "unsupported",
            ] {
                object.remove(k);
            }
            let refs = areas(&rule)?;
            rule["sqref"] = json!(refs.iter().map(|a| a.text()).collect::<Vec<_>>().join(" "));
            validate_rule(&rule)?;
            let names = w["sheets"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["name"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>();
            let anchor = address(refs[0].top, refs[0].left);
            if let Some(formula) = predicate_formula(
                rule["type"].as_str().unwrap(),
                rule["text"].as_str().unwrap_or(""),
                &anchor,
            ) {
                rule["formulas"] = json!([formula]);
                if matches!(
                    rule["type"].as_str(),
                    Some("containsText" | "notContainsText" | "beginsWith" | "endsWith")
                ) {
                    rule["operator"] = json!(if rule["type"] == "notContainsText" {
                        "notContains"
                    } else {
                        rule["type"].as_str().unwrap()
                    });
                }
            }

            // A style-only edit must retain an imported predicate. Regenerate only
            // when the period or anchor changes, or when authoring a new rule.
            let retain_date_formula = action == "edit" && {
                let old = &list[position()?];
                old["type"] == "timePeriod"
                    && old["timePeriod"] == rule["timePeriod"]
                    && old["sqref"] == rule["sqref"]
            };
            if rule["type"] == "timePeriod" && retain_date_formula {
                rule["formulas"] = list[position()?]["formulas"].clone();
            } else if rule["type"] == "timePeriod" {
                rule["formulas"] = json!([crate::conditional_dates::formula(
                    rule["timePeriod"].as_str().unwrap(),
                    &anchor
                )?]);
            }
            for f in formulas(&rule)
                .into_iter()
                .chain(
                    rule["scale"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|p| p["type"] == "formula")
                        .filter_map(|p| p["value"].as_str().map(String::from)),
                )
                .chain(
                    rule["dataBar"]["thresholds"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|p| p["type"] == "formula")
                        .filter_map(|p| p["value"].as_str().map(String::from)),
                )
                .chain(
                    rule["iconSet"]["thresholds"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .skip(1)
                        .filter(|p| p["type"] == "formula")
                        .filter_map(|p| p["value"].as_str().map(String::from)),
                )
            {
                crate::structure::copy_formula(
                    f.trim_start_matches('='),
                    &names,
                    index,
                    &anchor,
                    &anchor,
                )?;
            }
            if action == "add" {
                list.insert(0, rule);
            } else {
                let i = position()?;
                if flag(&list[i], "unsupported") {
                    return Err("This Excel rule is preserved but cannot be edited yet".into());
                }
                for k in [
                    "officeXml",
                    "officeDxfXml",
                    "groupXml",
                    "groupExtras",
                    "officeDataBarId",
                    "officeIconSetId",
                ] {
                    if let Some(v) = list[i].get(k) {
                        rule[k] = v.clone();
                    }
                }
                if rule["style"] != list[i]["style"] {
                    if let Some(source) = list[i]["officeDxfXml"].as_str() {
                        rule["officeDxfXml"] =
                            json!(merge_style_xml(source, &style_xml(&rule["style"])?)?);
                    }
                }
                list[i] = rule;
            }
        }
        "delete" => {
            let i = position()?;
            list.remove(i);
        }
        "move" => {
            let i = position()?;
            let to = request["to"]
                .as_u64()
                .filter(|n| (*n as usize) < list.len())
                .ok_or("Invalid rule position")? as usize;
            let r = list.remove(i);
            list.insert(to, r);
        }
        _ => return Err("Unknown conditional-format action".into()),
    }
    for (i, r) in list.iter_mut().enumerate() {
        r["priority"] = json!(i + 1);
    }
    let mut next = w.clone();
    next["sheets"][index]["conditionalFormats"] = json!(list);
    validate(&next["sheets"][index])?;
    Ok(next)
}
fn attributes(xml: &str, attrs: &[(&str, Option<String>)]) -> Result<String, String> {
    let d = parse(xml)?;
    Ok(crate::xlsx_tables::attributes(xml, d.root_element(), attrs))
}
pub fn write(
    pkg: &mut Package,
    sheet: &Value,
    before: Option<&Value>,
    path: &str,
) -> Result<(), String> {
    if sheet.get("conditionalFormats").is_none()
        || before.is_some_and(|b| b["conditionalFormats"] == sheet["conditionalFormats"])
    {
        return Ok(());
    }
    validate(sheet)?;
    let mut groups = String::new();
    let mut bar_groups = String::new();
    let mut used_ids: std::collections::HashSet<String> = rules(sheet)
        .flat_map(|r| {
            ["officeDataBarId", "officeIconSetId"]
                .into_iter()
                .filter_map(move |k| r[k].as_str().map(str::to_owned))
        })
        .map(|id| id.to_ascii_uppercase())
        .collect();
    // New copied rules must not reuse another worksheet's extension GUID.
    for (name, bytes) in &pkg.entries {
        if name.starts_with("xl/worksheets/") && name.ends_with(".xml") {
            let xml = std::str::from_utf8(bytes).map_err(|_| "Invalid worksheet text")?;
            let doc = parse(xml)?;
            used_ids.extend(
                doc.descendants()
                    .filter(|n| n.is_element() && n.tag_name().name() == "cfRule")
                    .filter_map(|n| n.attribute("id"))
                    .map(str::to_ascii_uppercase),
            );
        }
    }
    for (ordinal, r) in rules(sheet).enumerate() {
        if r["type"] == "iconSet" && icons::extended(&r["iconSet"]) && !flag(r, "unsupported") {
            let id = if let Some(id) = r["officeIconSetId"].as_str() {
                id.to_owned()
            } else {
                let mut i = ordinal + 1;
                loop {
                    let id = format!("{{F01101C0-0000-4000-8000-{i:012X}}}");
                    if used_ids.insert(id.clone()) {
                        break id;
                    }
                    i += 1;
                }
            };
            bar_groups += &icons::extension_xml(r, &id);
            continue;
        }
        let bar_id = if r["type"] == "dataBar"
            && r["dataBar"]["extended"] == true
            && !flag(r, "unsupported")
        {
            let id = if let Some(id) = r["officeDataBarId"].as_str() {
                id.to_owned()
            } else {
                let mut i = ordinal + 1;
                loop {
                    let id = format!("{{F0110DB0-0000-4000-8000-{i:012X}}}");
                    if used_ids.insert(id.clone()) {
                        break id;
                    }
                    i += 1;
                }
            };
            bar_groups += &bars::extension_xml(r, &id);
            Some(id)
        } else {
            None
        };
        let mut xml = r["officeXml"]
            .as_str()
            .map(String::from)
            .unwrap_or(format!("<cfRule xmlns=\"{S}\"/>"));
        let attrs = ATTRS
            .iter()
            .map(|k| {
                (
                    *k,
                    r.get(*k).map(|v| {
                        if let Some(v) = v.as_str() {
                            v.into()
                        } else if let Some(v) = v.as_bool() {
                            u8::from(v).to_string()
                        } else {
                            v.to_string()
                        }
                    }),
                )
            })
            .collect::<Vec<_>>();
        xml = attributes(&xml, &attrs)?;
        let dxf = if let Some(dxf) = r["officeDxfXml"].as_str() {
            Some(dxf.to_owned())
        } else if !flag(r, "unsupported") && r["style"].as_object().is_some_and(|s| !s.is_empty()) {
            Some(style_xml(&r["style"])?)
        } else {
            None
        };
        let id = if let Some(dxf) = dxf {
            crate::xlsx_styles::ensure(pkg)?;
            let (styles, id) =
                crate::xlsx_styles::section(pkg.text("xl/styles.xml")?, "dxfs", &dxf)?;
            pkg.set("xl/styles.xml", styles);
            Some(id.to_string())
        } else {
            None
        };
        if id.is_some() || !flag(r, "unsupported") {
            xml = attributes(&xml, &[("dxfId", id)])?;
        }
        if !flag(r, "unsupported") {
            let d = parse(&xml)?;
            let extra = d
                .root_element()
                .children()
                .filter(|n| {
                    n.is_element()
                        && !bars::link_container(*n)
                        && !(n.tag_name().namespace() == Some(S)
                            && matches!(
                                n.tag_name().name(),
                                "formula" | "colorScale" | "dataBar" | "iconSet"
                            ))
                })
                .map(|n| scoped_record(&xml, n))
                .collect::<String>();
            let mut inner = formulas(r)
                .iter()
                .map(|f| {
                    format!(
                        "<formula xmlns=\"{S}\">{}</formula>",
                        esc(f.trim_start_matches('='))
                    )
                })
                .collect::<String>();
            if r["type"] == "colorScale" {
                let points = r["scale"].as_array().unwrap();
                inner += &format!(
                    "<colorScale xmlns=\"{S}\">{}{}</colorScale>",
                    points
                        .iter()
                        .map(|p| format!(
                            "<cfvo type=\"{}\"{}/>",
                            p["type"].as_str().unwrap(),
                            if matches!(p["type"].as_str(), Some("min" | "max")) {
                                String::new()
                            } else {
                                format!(
                                    " val=\"{}\"",
                                    esc(p["value"].as_str().unwrap().trim_start_matches('='))
                                )
                            }
                        ))
                        .collect::<String>(),
                    points
                        .iter()
                        .map(|p| format!(
                            "<color rgb=\"FF{}\"/>",
                            &p["color"].as_str().unwrap()[1..]
                        ))
                        .collect::<String>()
                );
            }
            if r["type"] == "dataBar" {
                inner += &bars::standard_xml(&r["dataBar"], bar_id.as_deref());
            }
            if r["type"] == "iconSet" {
                inner += &icons::xml(&r["iconSet"], false);
            }
            xml = crate::xlsx::fragment_inner(&xml, &(inner + &extra))?;
        }
        let group = r["groupXml"]
            .as_str()
            .map(String::from)
            .unwrap_or(format!("<conditionalFormatting xmlns=\"{S}\"/>"));
        groups += &crate::xlsx::fragment_inner(
            &attributes(
                &group,
                &[("sqref", Some(r["sqref"].as_str().unwrap_or("").into()))],
            )?,
            &(xml + r["groupExtras"].as_str().unwrap_or("")),
        )?;
    }
    let xml = pkg.text(path)?.to_string();
    let doc = parse(&xml)?;
    let root = doc.root_element();
    let existing: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name((S, "conditionalFormatting")))
        .collect();
    let mut edits = vec![];
    if let Some(first) = existing.first() {
        edits.push((first.range(), groups));
        for n in existing.iter().skip(1) {
            edits.push((n.range(), String::new()));
        }
    } else if !groups.is_empty() {
        let at = root
            .children()
            .find(|n| {
                n.is_element()
                    && n.tag_name().namespace() == Some(S)
                    && matches!(
                        n.tag_name().name(),
                        "dataValidations"
                            | "hyperlinks"
                            | "printOptions"
                            | "pageMargins"
                            | "pageSetup"
                            | "headerFooter"
                            | "rowBreaks"
                            | "colBreaks"
                            | "customProperties"
                            | "cellWatches"
                            | "ignoredErrors"
                            | "smartTags"
                            | "drawing"
                            | "legacyDrawing"
                            | "legacyDrawingHF"
                            | "picture"
                            | "oleObjects"
                            | "controls"
                            | "webPublishItems"
                            | "tableParts"
                            | "extLst"
                    )
            })
            .map(|n| n.range().start)
            .unwrap_or(
                root.range().start
                    + raw(&xml, root)
                        .rfind("</")
                        .ok_or("Worksheet closing tag missing")?,
            );
        edits.push((at..at, groups));
    }
    pkg.set(path, replace_ranges(&xml, edits));
    // An unknown extension keeps the sheet read-only; never rebuild its container.
    if !flag(sheet, "extendedConditionalFormats") {
        bars::write_extensions(pkg, path, &bar_groups)?;
    }
    Ok(())
}

struct Prepared<'a> {
    rule: &'a Value,
    areas: Vec<Area>,
    numbers: Vec<f64>,
    counts: HashMap<String, usize>,
}
fn identity(v: &Value) -> String {
    if let Some(s) = v.as_str() {
        format!("s:{}", s.to_lowercase())
    } else {
        v.to_string()
    }
}
fn scalar(
    calc: &mut WorkbookCalculator,
    id: &str,
    cell: &str,
    formula: &str,
) -> Result<Value, String> {
    let v = calc.expression(id, cell, &format!("={}", formula.trim_start_matches('=')))?;
    if v.as_array().is_none_or(|a| a.len() != 1) || v[0].as_array().is_none_or(|a| a.len() != 1) {
        return Err("A rule formula must return one value".into());
    }
    Ok(v[0][0].clone())
}
fn truth(v: &Value) -> bool {
    v.as_bool()
        .unwrap_or_else(|| v.as_f64().is_some_and(|n| n != 0.0))
}
fn shifted(
    f: &str,
    names: &[String],
    index: usize,
    anchor: &str,
    cell: &str,
) -> Result<String, String> {
    crate::structure::copy_formula(f.trim_start_matches('='), names, index, anchor, cell)
}
fn predicate_formula(kind: &str, text: &str, cell: &str) -> Option<String> {
    let text = text.replace('"', "\"\"");
    Some(match kind {
        "containsErrors" => format!("ISERROR({cell})"),
        "notContainsErrors" => format!("NOT(ISERROR({cell}))"),
        "containsBlanks" => format!("LEN(TRIM({cell}))=0"),
        "notContainsBlanks" => format!("LEN(TRIM({cell}))>0"),
        "containsText" => format!("NOT(ISERROR(SEARCH(\"{text}\",{cell})))"),
        "notContainsText" => format!("ISERROR(SEARCH(\"{text}\",{cell}))"),
        "beginsWith" => format!("LEFT({cell},LEN(\"{text}\"))=\"{text}\""),
        "endsWith" => format!("RIGHT({cell},LEN(\"{text}\"))=\"{text}\""),
        _ => return None,
    })
}
fn match_rule(
    p: &Prepared<'_>,
    calc: &mut WorkbookCalculator,
    id: &str,
    index: usize,
    names: &[String],
    cell: &str,
    value: &Value,
) -> Result<bool, String> {
    let r = p.rule;
    let kind = r["type"].as_str().unwrap_or("");
    let anchor = address(p.areas[0].top, p.areas[0].left);
    let f = formulas(r);
    let formula = if kind == "timePeriod" {
        if let Some(formula) = f.first() {
            shifted(formula, names, index, &anchor, cell)?
        } else {
            crate::conditional_dates::formula(r["timePeriod"].as_str().unwrap(), cell)?
        }
    } else if kind == "cellIs" {
        let a = shifted(&f[0], names, index, &anchor, cell)?;
        let op = r["operator"].as_str().unwrap();
        if matches!(op, "between" | "notBetween") {
            let b = shifted(&f[1], names, index, &anchor, cell)?;
            let test = format!("AND({cell}>=({a}),{cell}<=({b}))");
            if op == "notBetween" {
                format!("NOT({test})")
            } else {
                test
            }
        } else {
            let op = match op {
                "equal" => "=",
                "notEqual" => "<>",
                "lessThan" => "<",
                "lessThanOrEqual" => "<=",
                "greaterThan" => ">",
                _ => ">=",
            };
            format!("{cell}{op}({a})")
        }
    } else if kind == "expression"
        || !f.is_empty()
            && matches!(
                kind,
                "containsText"
                    | "notContainsText"
                    | "beginsWith"
                    | "endsWith"
                    | "containsBlanks"
                    | "notContainsBlanks"
                    | "containsErrors"
                    | "notContainsErrors"
            )
    {
        shifted(&f[0], names, index, &anchor, cell)?
    } else {
        match kind {
            "containsErrors" => format!("ISERROR({cell})"),
            "notContainsErrors" => format!("NOT(ISERROR({cell}))"),
            "containsBlanks" => format!("LEN(TRIM({cell}))=0"),
            "notContainsBlanks" => format!("LEN(TRIM({cell}))>0"),
            "containsText" | "notContainsText" | "beginsWith" | "endsWith" => {
                let text = r["text"].as_str().unwrap().replace('"', "\"\"");
                match kind {
                    "containsText" => format!("NOT(ISERROR(SEARCH(\"{text}\",{cell})))"),
                    "notContainsText" => format!("ISERROR(SEARCH(\"{text}\",{cell}))"),
                    "beginsWith" => format!("LEFT({cell},LEN(\"{text}\"))=\"{text}\""),
                    _ => format!("RIGHT({cell},LEN(\"{text}\"))=\"{text}\""),
                }
            }
            "duplicateValues" | "uniqueValues" => {
                if value.is_null() || value == "" {
                    return Ok(false);
                }
                let count = p.counts.get(&identity(value)).copied().unwrap_or(0);
                return Ok(if kind == "duplicateValues" {
                    count > 1
                } else {
                    count == 1
                });
            }
            "top10" => {
                let Some(n) = value.as_f64() else {
                    return Ok(false);
                };
                if p.numbers.is_empty() {
                    return Ok(false);
                }
                let rank = r["rank"].as_u64().unwrap() as usize;
                let rank = if flag(r, "percent") {
                    (p.numbers.len() * rank).div_ceil(100)
                } else {
                    rank
                };
                let rank = rank.min(p.numbers.len()).max(1);
                return Ok(if flag(r, "bottom") {
                    n <= p.numbers[rank - 1]
                } else {
                    n >= p.numbers[p.numbers.len() - rank]
                });
            }
            "aboveAverage" => {
                let Some(n) = value.as_f64() else {
                    return Ok(false);
                };
                if p.numbers.is_empty() {
                    return Ok(false);
                }
                let mean = p.numbers.iter().sum::<f64>() / p.numbers.len() as f64;
                let sd = (p.numbers.iter().map(|n| (n - mean).powi(2)).sum::<f64>()
                    / p.numbers.len() as f64)
                    .sqrt();
                let above = r["aboveAverage"].as_bool().unwrap_or(true);
                let threshold =
                    mean + if above { sd } else { -sd } * r["stdDev"].as_u64().unwrap_or(0) as f64;
                let comparison = n.partial_cmp(&threshold);
                return Ok(comparison
                    == Some(if above {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Less
                    })
                    || flag(r, "equalAverage") && n == threshold);
            }
            _ => return Ok(false),
        }
    };
    scalar(calc, id, cell, &formula).map(|v| truth(&v))
}
fn scale_color(
    p: &Prepared<'_>,
    calc: &mut WorkbookCalculator,
    id: &str,
    _cell: &str,
    n: f64,
) -> Result<Option<String>, String> {
    let Some(min) = p.numbers.first().copied() else {
        return Ok(None);
    };
    let max = *p.numbers.last().unwrap();
    let mut points = vec![];
    for point in p.rule["scale"].as_array().unwrap() {
        let input = point["value"].as_str().unwrap_or("");
        let v = match point["type"].as_str().unwrap() {
            "min" => min,
            "max" => max,
            "num" => input.parse().unwrap(),
            "percent" => min + (max - min) * input.parse::<f64>().unwrap() / 100.0,
            "percentile" => {
                let f = (p.numbers.len() - 1) as f64 * input.parse::<f64>().unwrap() / 100.0;
                let i = f.floor() as usize;
                let j = (i + 1).min(p.numbers.len() - 1);
                p.numbers[i] + (p.numbers[j] - p.numbers[i]) * f.fract()
            }
            _ => scalar(calc, id, &address(p.areas[0].top, p.areas[0].left), input)?
                .as_f64()
                .ok_or("Color-scale threshold is not numeric")?,
        };
        points.push((v, point["color"].as_str().unwrap()));
    }
    if points.windows(2).any(|w| w[0].0 > w[1].0) {
        return Err("Color-scale thresholds must be in ascending order".into());
    }
    let (a, b) = points
        .windows(2)
        .find(|w| n <= w[1].0)
        .map(|w| (w[0], w[1]))
        .unwrap_or((points[points.len() - 2], points[points.len() - 1]));
    let t = if b.0 == a.0 {
        0.0
    } else {
        ((n - a.0) / (b.0 - a.0)).clamp(0.0, 1.0)
    };
    let mut color = String::from("#");
    for i in [1, 3, 5] {
        let x = u8::from_str_radix(&a.1[i..i + 2], 16).unwrap() as f64;
        let y = u8::from_str_radix(&b.1[i..i + 2], 16).unwrap() as f64;
        color += &format!("{:02X}", (x + (y - x) * t).round() as u8);
    }
    Ok(Some(color))
}
/// Evaluates only supplied addresses; whole-column rules never expand 1M rows.
/// Aggregate predicates consider all populated cells in the rule, not the viewport.
pub fn evaluate(
    sheets: &Value,
    index: usize,
    cells: &[String],
    calc: &mut WorkbookCalculator,
) -> Result<Value, String> {
    let sheet = &sheets[index];
    let id = sheet["id"].as_str().ok_or("Worksheet missing")?;
    let names = sheets
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    let mut ordered: Vec<_> = rules(sheet).collect();
    ordered.sort_by_key(|r| r["priority"].as_u64().unwrap_or(u64::MAX));
    if cells.len().saturating_mul(ordered.len()) > 2_000_000 {
        return Err("This view exceeds the conditional-format evaluation limit".into());
    }
    let mut prepared = vec![];
    let mut warnings = vec![];
    if flag(sheet, "extendedConditionalFormats") {
        warnings.push(
            "Extended Excel conditional formats are preserved but not displayed yet.".to_string(),
        );
    }
    if ordered.is_empty() {
        // No rule can style a cell; skip the per-cell pass.
        return Ok(json!({"styles":{},"warnings":warnings}));
    }
    for (i, rule) in ordered.iter().enumerate() {
        if flag(rule, "unsupported") {
            warnings.push(format!(
                "Rule {} is preserved but not displayed yet.",
                i + 1
            ));
            continue;
        }
        let a = areas(rule)?;
        let mut numbers = vec![];
        let mut counts = HashMap::new();
        if matches!(
            rule["type"].as_str(),
            Some(
                "top10"
                    | "aboveAverage"
                    | "duplicateValues"
                    | "uniqueValues"
                    | "colorScale"
                    | "dataBar"
                    | "iconSet"
            )
        ) {
            for (cell, value) in calc.values[id].as_object().into_iter().flatten() {
                let (row, col) = parse_address(cell)?;
                if a.iter().any(|a| a.contains(row, col)) && calc.cell_errors[id][cell] != true {
                    if let Some(n) = value.as_f64() {
                        numbers.push(n);
                    }
                    *counts.entry(identity(value)).or_insert(0) += 1;
                }
            }
            numbers.sort_by(f64::total_cmp);
        }
        prepared.push(Prepared {
            rule,
            areas: a,
            numbers,
            counts,
        });
    }
    let mut styles = Map::new();
    // Rules whose formula went over the engine's read budget. It would do so
    // for every cell, so the rule is dropped for the rest of this pass.
    let mut too_heavy = HashSet::new();
    for cell in cells {
        let (row, col) = parse_address(cell)?;
        let value = calc.values[id][cell].clone();
        let mut style = Map::new();
        for (pi, p) in prepared.iter().enumerate() {
            if !p.areas.iter().any(|a| a.contains(row, col)) || too_heavy.contains(&pi) {
                continue;
            }
            if p.rule["type"] == "iconSet" {
                if !style.contains_key("_iconSet") && calc.cell_errors[id][cell] != true {
                    if let Some(n) = value.as_f64() {
                        match icons::evaluate(&p.rule["iconSet"], &p.numbers, n, |f| {
                            scalar(calc, id, &address(p.areas[0].top, p.areas[0].left), f)?
                                .as_f64()
                                .ok_or("Icon threshold is not numeric".into())
                        }) {
                            Ok(Some(icon)) => {
                                style.insert("_iconSet".into(), icon);
                            }
                            Ok(None) => {}
                            Err(e) => {
                                let warning = format!("Rule {}: {e}", p.rule["priority"]);
                                if !warnings.contains(&warning) {
                                    warnings.push(warning);
                                }
                            }
                        }
                    }
                }
                continue;
            }
            if p.rule["type"] == "dataBar" {
                if !style.contains_key("_dataBar") && calc.cell_errors[id][cell] != true {
                    if let Some(n) = value.as_f64() {
                        match bars::geometry(
                            &p.rule["dataBar"],
                            &p.numbers,
                            n,
                            flag(sheet, "conditionalRightToLeft"),
                            |f| {
                                scalar(calc, id, &address(p.areas[0].top, p.areas[0].left), f)?
                                    .as_f64()
                                    .ok_or("Data-bar threshold is not numeric".into())
                            },
                        ) {
                            Ok(Some(bar)) => {
                                style.insert("_dataBar".into(), bar);
                            }
                            Ok(None) => {}
                            Err(e) => {
                                let warning = format!("Rule {}: {e}", p.rule["priority"]);
                                if !warnings.contains(&warning) {
                                    warnings.push(warning);
                                }
                            }
                        }
                    }
                }
                continue;
            }
            if p.rule["type"] == "colorScale" {
                if !style.contains_key("fill") && calc.cell_errors[id][cell] != true {
                    if let Some(n) = value.as_f64() {
                        match scale_color(p, calc, id, cell, n) {
                            Ok(Some(color)) => {
                                style.insert("fill".into(), json!(color));
                            }
                            Ok(None) => {}
                            Err(e) => {
                                let warning = format!("Rule {}: {e}", p.rule["priority"]);
                                if !warnings.contains(&warning) {
                                    warnings.push(warning);
                                }
                            }
                        }
                    }
                }
                continue;
            }
            // Excel treats a formula error as false; explicit ISERROR rules can
            // still format error cells. No helper cell or history entry is used.
            let matched = match match_rule(p, calc, id, index, &names, cell, &value) {
                Ok(matched) => matched,
                Err(e) if e == ironcalc_base::TOO_MANY_READS => {
                    too_heavy.insert(pi);
                    let number = ordered.iter().position(|r| std::ptr::eq(*r, p.rule));
                    warnings.push(format!(
                        "Rule {} reads too many cells and is not displayed.",
                        number.map_or(pi, |n| n) + 1
                    ));
                    false
                }
                Err(_) => false,
            };
            if matched {
                for (k, v) in p.rule["style"].as_object().unwrap() {
                    style.entry(k.clone()).or_insert_with(|| v.clone());
                }
                if flag(p.rule, "stopIfTrue") {
                    break;
                }
            }
        }
        if !style.is_empty() {
            styles.insert(cell.clone(), Value::Object(style));
        }
    }
    Ok(json!({"styles":styles,"warnings":warnings}))
}
