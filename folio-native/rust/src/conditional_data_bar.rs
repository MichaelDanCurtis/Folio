//! Data-bar value/geometry and standard + Office 2010 OOXML representations.
use crate::office::*;
use serde_json::{json, Map, Value};
use std::collections::HashSet;

pub const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub const X14: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
pub const XM: &str = "http://schemas.microsoft.com/office/excel/2006/main";
const LINK: &str = "{B025F937-C7B1-47D3-B67F-A62EFF666E3E}";
const EXT: &str = "{78C0D931-6437-407d-A8EE-F0AAD7539E65}";
pub fn known_extension(n: roxmltree::Node<'_, '_>) -> bool {
    n.has_tag_name((S, "ext")) && matches!(attr(n, "uri"), Some(LINK | EXT))
}
const FLAGS: &[(&str, bool)] = &[
    ("showValue", true),
    ("border", false),
    ("gradient", true),
    ("negativeBarColorSameAsPositive", false),
    ("negativeBarBorderColorSameAsPositive", true),
];
const COLORS: &[&str] = &[
    "color",
    "borderColor",
    "negativeFillColor",
    "negativeBorderColor",
    "axisColor",
];
fn flag(v: &Value, k: &str, default: bool) -> bool {
    v[k].as_bool().unwrap_or(default)
}
fn length(v: &Value, k: &str, default: u64) -> u64 {
    v[k].as_u64().unwrap_or(default)
}
fn color(n: roxmltree::Node<'_, '_>) -> Result<String, String> {
    if n.attributes()
        .any(|a| a.namespace().is_some() || a.name() != "rgb")
    {
        return Err("Theme, indexed or tinted data-bar colors are not displayed yet".into());
    }
    let rgb = attr(n, "rgb").ok_or("Data-bar color missing")?;
    let rgb = if rgb.len() == 8 { &rgb[2..] } else { rgb };
    if rgb.len() != 6 || !rgb.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid data-bar color".into());
    }
    Ok(format!("#{}", rgb.to_uppercase()))
}
pub(crate) fn absolute_scalar_formula(text: &str) -> bool {
    use ironcalc_base::expressions::{
        parser::{new_parser_english, Node},
        types::CellReferenceRC,
    };
    fn valid(n: &Node) -> bool {
        match n {
            Node::ReferenceKind {
                absolute_row,
                absolute_column,
                ..
            }
            | Node::WrongReferenceKind {
                absolute_row,
                absolute_column,
                ..
            } => *absolute_row && *absolute_column,
            Node::OpConcatenateKind { left, right }
            | Node::OpSumKind { left, right, .. }
            | Node::OpProductKind { left, right, .. }
            | Node::OpPowerKind { left, right }
            | Node::CompareKind { left, right, .. } => valid(left) && valid(right),
            Node::UnaryKind { right, .. } => valid(right),
            Node::FunctionKind { args, .. } => args.iter().all(valid),
            Node::BooleanKind(_)
            | Node::NumberKind(_)
            | Node::StringKind(_)
            | Node::ErrorKind(_)
            | Node::EmptyArgKind => true,
            _ => false,
        }
    }
    let mut parser = new_parser_english(
        vec!["Sheet1".into()],
        vec![],
        std::collections::HashMap::new(),
    );
    valid(&parser.parse(
        text.trim_start_matches('='),
        &CellReferenceRC {
            sheet: "Sheet1".into(),
            row: 1,
            column: 1,
        },
    ))
}
pub fn validate(b: &Value) -> Result<(), String> {
    b.as_object().ok_or("Data-bar settings missing")?;
    for (key, _) in FLAGS.iter().chain(std::iter::once(&("extended", false))) {
        if b.get(*key).is_some_and(|v| !v.is_boolean()) {
            return Err(format!("Invalid data-bar {key}"));
        }
    }
    for (key, default) in [("minLength", 10), ("maxLength", 90)] {
        if b.get(key).is_some_and(|v| v.as_u64().is_none()) || length(b, key, default) > 100 {
            return Err("Data-bar lengths must be whole percentages from 0 to 100".into());
        }
    }
    if length(b, "minLength", 10) > length(b, "maxLength", 90) {
        return Err("Minimum bar length exceeds maximum".into());
    }
    for key in COLORS {
        if let Some(v) = b.get(*key) {
            crate::xlsx_styles::validate_format(&Map::from_iter([("fill".into(), v.clone())]))?;
            if v.as_str().is_none_or(|s| !s.starts_with('#')) {
                return Err("Data bars need RGB colors".into());
            }
        }
    }
    if b["color"].is_null() {
        return Err("Choose a data-bar fill color".into());
    }
    if !["context", "leftToRight", "rightToLeft"]
        .contains(&b["direction"].as_str().unwrap_or("context"))
    {
        return Err("Invalid data-bar direction".into());
    }
    if !["automatic", "middle", "none"].contains(&b["axisPosition"].as_str().unwrap_or("automatic"))
    {
        return Err("Invalid data-bar axis position".into());
    }
    let p = b["thresholds"]
        .as_array()
        .ok_or("Data-bar thresholds missing")?;
    if p.len() != 2 {
        return Err("Data bars need minimum and maximum thresholds".into());
    }
    for (i, p) in p.iter().enumerate() {
        let t = p["type"].as_str().unwrap_or("");
        if ![
            "min",
            "max",
            "autoMin",
            "autoMax",
            "num",
            "percent",
            "percentile",
            "formula",
        ]
        .contains(&t)
            || i == 0 && matches!(t, "max" | "autoMax")
            || i == 1 && matches!(t, "min" | "autoMin")
            || !flag(b, "extended", false) && matches!(t, "autoMin" | "autoMax")
        {
            return Err("Invalid data-bar threshold type".into());
        }
        if matches!(t, "num" | "percent" | "percentile")
            && p["value"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .is_none_or(|n| !n.is_finite() || t != "num" && !(0.0..=100.0).contains(&n))
        {
            return Err("Enter valid data-bar threshold values".into());
        }
        if t == "formula"
            && p["value"]
                .as_str()
                .is_none_or(|s| s.is_empty() || s.len() > 8192)
        {
            return Err("Enter a data-bar threshold formula".into());
        }
        if t == "formula"
            && flag(b, "extended", false)
            && !absolute_scalar_formula(p["value"].as_str().unwrap())
        {
            return Err("Data-bar formulas need absolute cell references (such as $B$1), without ranges or arrays".into());
        }
    }
    Ok(())
}
fn read_bar(n: roxmltree::Node<'_, '_>, base: Option<&Value>) -> Result<Value, String> {
    let extended = n.tag_name().namespace() == Some(X14);
    let ns = if extended { X14 } else { S };
    let mut b = json!({"extended":extended});
    let allowed = [
        "minLength",
        "maxLength",
        "showValue",
        "border",
        "gradient",
        "direction",
        "negativeBarColorSameAsPositive",
        "negativeBarBorderColorSameAsPositive",
        "axisPosition",
    ];
    if n.attributes()
        .any(|a| a.namespace().is_some() || !allowed.contains(&a.name()))
    {
        return Err("Unmodeled data-bar attributes".into());
    }
    for (key, default) in [("minLength", 10), ("maxLength", 90)] {
        b[key] = json!(attr(n, key)
            .map(|s| s.parse::<u64>())
            .transpose()
            .map_err(|_| "Invalid bar length")?
            .unwrap_or(default));
    }
    for (key, default) in FLAGS {
        let inherited = if *key == "showValue" {
            base.and_then(|v| v[*key].as_bool()).unwrap_or(*default)
        } else {
            *default
        };
        b[*key] = json!(match attr(n, key) {
            Some("0" | "false") => false,
            Some("1" | "true") => true,
            None => inherited,
            _ => return Err(format!("Invalid {key}")),
        });
    }
    b["direction"] = json!(attr(n, "direction").unwrap_or("context"));
    b["axisPosition"] =
        json!(attr(n, "axisPosition").unwrap_or(if extended { "automatic" } else { "none" }));
    let mut points = vec![];
    for c in n.children().filter(|n| n.is_element()) {
        if c.tag_name().namespace() != Some(ns) {
            return Err("Unmodeled data-bar children".into());
        }
        match c.tag_name().name() {
            "cfvo" => {
                if c.attributes()
                    .any(|a| a.namespace().is_some() || !["type", "val"].contains(&a.name()))
                    || c.children()
                        .filter(|n| n.is_element())
                        .any(|n| !extended || !n.has_tag_name((XM, "f")))
                {
                    return Err("Unmodeled bar threshold".into());
                }
                points.push(json!({"type":attr(c,"type").unwrap_or(""),"value":if extended {c.children().find(|n|n.has_tag_name((XM,"f"))).and_then(|n|n.text()).unwrap_or("")} else {attr(c,"val").unwrap_or("")}}));
            }
            "color" | "fillColor" => b["color"] = json!(color(c)?),
            "borderColor" | "negativeFillColor" | "negativeBorderColor" | "axisColor" => {
                b[c.tag_name().name()] = json!(color(c)?)
            }
            _ => return Err("Unmodeled data-bar children".into()),
        }
    }
    if b["color"].is_null() {
        b["color"] = base.map(|v| v["color"].clone()).unwrap_or(Value::Null);
    }
    b["thresholds"] = json!(points);
    validate(&b)?;
    Ok(b)
}
pub fn read_standard(n: roxmltree::Node<'_, '_>) -> Result<Value, String> {
    read_bar(n, None)
}
pub fn link_container(n: roxmltree::Node<'_, '_>) -> bool {
    n.has_tag_name((S, "extLst"))
        && n.children().filter(|n| n.is_element()).count() == 1
        && n.children().filter(|n| n.is_element()).all(|e| {
            e.has_tag_name((S, "ext"))
                && attr(e, "uri") == Some(LINK)
                && e.children().filter(|n| n.is_element()).count() == 1
                && e.children()
                    .filter(|n| n.is_element())
                    .all(|n| n.has_tag_name((X14, "id")))
        })
}
pub fn link(n: roxmltree::Node<'_, '_>) -> Option<String> {
    n.children()
        .find(|n| link_container(*n))?
        .descendants()
        .find(|n| n.has_tag_name((X14, "id")))?
        .text()
        .map(str::to_owned)
}
/// Only fully modeled extensions unlock editing. Unknown rules keep their source XML.
pub fn read_extensions(doc: &roxmltree::Document<'_>, list: &mut Vec<Value>) -> bool {
    let mut unresolved = doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "conditionalFormattings")
        .any(|p| {
            !p.has_tag_name((X14, "conditionalFormattings"))
                || p.attributes().len() > 0
                || p.children()
                    .filter(|n| n.is_element())
                    .any(|n| !n.has_tag_name((X14, "conditionalFormatting")))
                || p.parent().is_none_or(|e| {
                    !e.has_tag_name((S, "ext"))
                        || attr(e, "uri") != Some(EXT)
                        || e.attributes()
                            .any(|a| a.namespace().is_some() || a.name() != "uri")
                        || e.children().filter(|n| n.is_element()).count() != 1
                })
        });
    let mut seen = HashSet::new();
    for group in doc.descendants().filter(|n| {
        n.is_element()
            && n.tag_name().name() == "conditionalFormatting"
            && n.tag_name().namespace() != Some(S)
    }) {
        if group.parent().is_none_or(|p| {
            !p.has_tag_name((X14, "conditionalFormattings"))
                || p.attributes().len() > 0
                || p.children()
                    .filter(|n| n.is_element())
                    .any(|n| !n.has_tag_name((X14, "conditionalFormatting")))
                || p.parent()
                    .is_none_or(|e| !e.has_tag_name((S, "ext")) || attr(e, "uri") != Some(EXT))
        }) {
            unresolved = true;
        }
        if !group.has_tag_name((X14, "conditionalFormatting")) {
            unresolved = true;
            continue;
        }
        if group.attributes().len() > 0
            || group
                .children()
                .filter(|n| n.is_element())
                .any(|n| !n.has_tag_name((X14, "cfRule")) && !n.has_tag_name((XM, "sqref")))
        {
            unresolved = true;
        }
        let sqref = group
            .children()
            .find(|n| n.has_tag_name((XM, "sqref")))
            .and_then(|n| n.text())
            .unwrap_or("");
        for n in group.children().filter(|n| n.has_tag_name((X14, "cfRule"))) {
            let id = attr(n, "id").unwrap_or("");
            if attr(n, "type") == Some("iconSet") {
                match crate::conditional_icons::read_extension(n, sqref) {
                    Ok(rule) if seen.insert(id.to_owned()) => list.push(rule),
                    _ => unresolved = true,
                }
                continue;
            }
            let supported = || -> Result<(Value, Option<usize>), String> {
                if id.is_empty()
                    || attr(n, "type") != Some("dataBar")
                    || n.attributes().any(|a| {
                        a.namespace().is_some() || !["id", "type", "priority"].contains(&a.name())
                    })
                    || n.children()
                        .filter(|n| n.is_element())
                        .any(|n| !n.has_tag_name((X14, "dataBar")))
                {
                    return Err("Unknown extended rule".into());
                }
                let i = list.iter().position(|r| r["officeDataBarId"] == id);
                if list.iter().filter(|r| r["officeDataBarId"] == id).count() > 1 {
                    return Err("Duplicate data-bar link".into());
                }
                if let Some(i) = i {
                    if list[i]["type"] != "dataBar"
                        || list[i]["sqref"] != sqref
                        || list[i]["unsupported"] == true
                    {
                        return Err("Ambiguous linked data bar".into());
                    }
                } else if attr(n, "priority")
                    .and_then(|v| v.parse::<u64>().ok())
                    .is_none_or(|n| n == 0)
                {
                    return Err("Unlinked data bar".into());
                }
                let bar = n
                    .children()
                    .find(|n| n.has_tag_name((X14, "dataBar")))
                    .ok_or("Missing extended data bar")?;
                Ok((read_bar(bar, i.map(|i| &list[i]["dataBar"]))?, i))
            };
            match supported() {
                Ok((bar, i)) if seen.insert(id.to_owned()) => {
                    if let Some(i) = i {
                        list[i]["dataBar"] = bar;
                    } else {
                        list.push(json!({"type":"dataBar","priority":attr(n,"priority").unwrap().parse::<u64>().unwrap(),"sqref":sqref,"style":{},"dataBar":bar,"officeDataBarId":id}));
                    }
                }
                _ => {
                    unresolved = true;
                    for r in list.iter_mut().filter(|r| r["officeDataBarId"] == id) {
                        r["unsupported"] = json!(true);
                    }
                }
            }
        }
    }
    for r in list
        .iter_mut()
        .filter(|r| r.get("officeDataBarId").is_some())
    {
        if !seen.contains(r["officeDataBarId"].as_str().unwrap_or("")) {
            r["unsupported"] = json!(true);
            unresolved = true;
        }
    }
    unresolved
}
fn thresholds(b: &Value, extended: bool) -> String {
    b["thresholds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let mut t = p["type"].as_str().unwrap();
            if !extended {
                t = match t {
                    "autoMin" => "min",
                    "autoMax" => "max",
                    _ => t,
                };
            }
            let val = esc(p["value"].as_str().unwrap_or("").trim_start_matches('='));
            let empty = matches!(t, "min" | "max" | "autoMin" | "autoMax");
            if extended {
                if empty {
                    format!("<x14:cfvo type=\"{t}\"/>")
                } else {
                    format!("<x14:cfvo type=\"{t}\"><xm:f>{val}</xm:f></x14:cfvo>")
                }
            } else {
                format!(
                    "<cfvo type=\"{t}\"{}/>",
                    if empty {
                        String::new()
                    } else {
                        format!(" val=\"{val}\"")
                    }
                )
            }
        })
        .collect()
}
fn color_xml(b: &Value, key: &str, tag: &str, default: &str) -> String {
    format!(
        "<{tag} rgb=\"FF{}\"/>",
        &b[key].as_str().unwrap_or(default)[1..]
    )
}
pub fn standard_xml(b: &Value, id: Option<&str>) -> String {
    let (mut min, mut max) = (length(b, "minLength", 10), length(b, "maxLength", 90));
    if id.is_some() && min == 0 && max == 100 {
        (min, max) = (10, 90);
    }
    let mut xml=format!("<dataBar xmlns=\"{S}\" minLength=\"{min}\" maxLength=\"{max}\" showValue=\"{}\">{}{}</dataBar>",u8::from(flag(b,"showValue",true)), thresholds(b,false),color_xml(b,"color","color","#638EC6"));
    if let Some(id) = id {
        xml+=&format!("<extLst xmlns=\"{S}\"><ext uri=\"{LINK}\"><x14:id xmlns:x14=\"{X14}\">{}</x14:id></ext></extLst>",esc(id));
    }
    xml
}
pub fn extension_xml(r: &Value, id: &str) -> String {
    let b = &r["dataBar"];
    let mut attrs = format!(
        "minLength=\"{}\" maxLength=\"{}\" direction=\"{}\" axisPosition=\"{}\"",
        length(b, "minLength", 10),
        length(b, "maxLength", 90),
        b["direction"].as_str().unwrap_or("context"),
        b["axisPosition"].as_str().unwrap_or("automatic")
    );
    for (key, default) in FLAGS {
        attrs += &format!(" {key}=\"{}\"", u8::from(flag(b, key, *default)));
    }
    let mut inner = thresholds(b, true);
    if flag(b, "border", false) {
        inner += &color_xml(
            b,
            "borderColor",
            "x14:borderColor",
            b["color"].as_str().unwrap(),
        );
    }
    if !flag(b, "negativeBarColorSameAsPositive", false) {
        inner += &color_xml(b, "negativeFillColor", "x14:negativeFillColor", "#FF0000");
    }
    if flag(b, "border", false) && !flag(b, "negativeBarBorderColorSameAsPositive", true) {
        inner += &color_xml(
            b,
            "negativeBorderColor",
            "x14:negativeBorderColor",
            "#FF0000",
        );
    }
    if b["axisPosition"] != "none" {
        inner += &color_xml(b, "axisColor", "x14:axisColor", "#000000");
    }
    format!("<x14:conditionalFormatting xmlns:xm=\"{XM}\"><x14:cfRule type=\"dataBar\" id=\"{}\"><x14:dataBar {attrs}>{inner}</x14:dataBar></x14:cfRule><xm:sqref>{}</xm:sqref></x14:conditionalFormatting>",esc(id),esc(r["sqref"].as_str().unwrap()))
}
pub fn write_extensions(pkg: &mut Package, path: &str, groups: &str) -> Result<(), String> {
    let xml = pkg.text(path)?.to_owned();
    let doc = parse(&xml)?;
    let root = doc.root_element();
    let ext = if groups.is_empty() {
        String::new()
    } else {
        format!("<ext xmlns=\"{S}\" uri=\"{EXT}\"><x14:conditionalFormattings xmlns:x14=\"{X14}\">{groups}</x14:conditionalFormattings></ext>")
    };
    let (range, replacement) =
        if let Some(list) = root.children().find(|n| n.has_tag_name((S, "extLst"))) {
            let mut inner = list
                .children()
                .filter(|n| n.is_element() && attr(*n, "uri") != Some(EXT))
                .map(|n| scoped_record(&xml, n))
                .collect::<String>();
            inner += &ext;
            (
                list.range(),
                if inner.is_empty() {
                    String::new()
                } else {
                    crate::xlsx::fragment_inner(&scoped_record(&xml, list), &inner)?
                },
            )
        } else {
            let at = root.range().start
                + raw(&xml, root)
                    .rfind("</")
                    .ok_or("Missing worksheet closing tag")?;
            (
                at..at,
                if ext.is_empty() {
                    String::new()
                } else {
                    format!("<extLst xmlns=\"{S}\">{ext}</extLst>")
                },
            )
        };
    pkg.set(path, replace_ranges(&xml, vec![(range, replacement)]));
    Ok(())
}

/// Normalized [0,1] geometry. The renderer applies pixel insets and clipping.
pub fn geometry(
    b: &Value,
    numbers: &[f64],
    value: f64,
    rtl: bool,
    mut formula: impl FnMut(&str) -> Result<f64, String>,
) -> Result<Option<Value>, String> {
    let Some(low) = numbers.first().copied() else {
        return Ok(None);
    };
    let high = *numbers.last().unwrap();
    let mut bounds = vec![];
    for p in b["thresholds"].as_array().unwrap() {
        let input = p["value"].as_str().unwrap_or("");
        let n = match p["type"].as_str().unwrap() {
            "min" => low,
            "max" => high,
            "autoMin" => low.min(0.0),
            "autoMax" => high.max(0.0),
            "num" => input.parse().unwrap(),
            "percent" => low + (high - low) * input.parse::<f64>().unwrap() / 100.0,
            "percentile" => {
                let f = (numbers.len() - 1) as f64 * input.parse::<f64>().unwrap() / 100.0;
                let i = f.floor() as usize;
                let j = (i + 1).min(numbers.len() - 1);
                numbers[i] + (numbers[j] - numbers[i]) * f.fract()
            }
            _ => {
                // MS-XLSX ST_CfvoType defines a bare numerical constant in an
                // x14 formula threshold as zero; a Number threshold keeps it.
                let text = input.trim().trim_start_matches('=');
                if flag(b, "extended", false)
                    && !text.starts_with(['-', '+'])
                    && text.parse::<f64>().is_ok()
                {
                    0.0
                } else {
                    formula(input)?
                }
            }
        };
        if !n.is_finite() {
            return Err("Data-bar threshold is not finite".into());
        }
        bounds.push(n);
    }
    let (min, max) = (bounds[0], bounds[1]);
    if min > max {
        return Err("Data-bar thresholds must be in ascending order".into());
    }
    if !(max - min).is_finite() {
        return Err("Data-bar threshold span exceeds the display range".into());
    }
    let shortest = length(b, "minLength", 10) as f64 / 100.0;
    let longest = length(b, "maxLength", 90) as f64 / 100.0;
    let extended = flag(b, "extended", false);
    let axis_mode = if extended {
        b["axisPosition"].as_str().unwrap_or("automatic")
    } else {
        "none"
    };
    let interpolate = |n: f64, a: f64, z: f64| {
        if a == z {
            if n >= z {
                1.0
            } else {
                0.0
            }
        } else {
            ((n - a) / (z - a)).clamp(0.0, 1.0)
        }
    };
    let mut axis = if axis_mode == "middle" {
        0.5
    } else if min < 0.0 && max > 0.0 {
        (-min / (max - min)).clamp(0.0, 1.0)
    } else if max <= 0.0 && min < 0.0 {
        1.0
    } else {
        0.0
    };
    let negative = extended && value < 0.0;
    let (mut start, mut end) = if axis_mode == "none" {
        axis = 0.0;
        (
            0.0,
            shortest + (longest - shortest) * interpolate(value, min, max),
        )
    } else if value == 0.0 {
        (axis, axis)
    } else if axis_mode == "middle" {
        // One scale on both sides: the greatest absolute threshold gets half
        // the maximum cell-width percentage.
        let magnitude = min.abs().max(max.abs());
        let bounded = value.clamp(min, max);
        let proportion = if magnitude == 0.0 {
            0.0
        } else {
            (bounded.abs() / magnitude).clamp(0.0, 1.0)
        };
        let minimum = shortest.min(longest / 2.0);
        let len = minimum + (longest / 2.0 - minimum) * proportion;
        (axis, axis + if bounded < 0.0 { -len } else { len })
    } else if negative {
        let factor = interpolate(-value, (-max).max(0.0), (-min).max(0.0));
        let len = shortest.min(axis) + (axis * longest - shortest.min(axis)).max(0.0) * factor;
        (axis, (axis - len).max(0.0))
    } else {
        let factor = interpolate(value, min.max(0.0), max.max(0.0));
        let space = 1.0 - axis;
        let len = shortest.min(space) + (space * longest - shortest.min(space)).max(0.0) * factor;
        (axis, (axis + len).min(1.0))
    };
    let reverse = match b["direction"].as_str().unwrap_or("context") {
        "rightToLeft" => true,
        "leftToRight" => false,
        _ => rtl,
    };
    if reverse {
        start = 1.0 - start;
        end = 1.0 - end;
        axis = 1.0 - axis;
    }
    let fill = if negative && !flag(b, "negativeBarColorSameAsPositive", false) {
        b["negativeFillColor"].as_str().unwrap_or("#FF0000")
    } else {
        b["color"].as_str().unwrap()
    };
    let border = if negative && !flag(b, "negativeBarBorderColorSameAsPositive", true) {
        b["negativeBorderColor"].as_str().unwrap_or("#FF0000")
    } else {
        b["borderColor"]
            .as_str()
            .unwrap_or(b["color"].as_str().unwrap())
    };
    Ok(Some(
        json!({"start":start,"end":end,"axis":if axis_mode!="none" && low<0.0 {Some(axis)} else {None},
        "color":fill,"borderColor":border,"axisColor":b["axisColor"].as_str().unwrap_or("#000000"),"border":extended && flag(b,"border",false),
        "gradient":!extended || flag(b,"gradient",true),"showValue":flag(b,"showValue",true)}),
    ))
}
