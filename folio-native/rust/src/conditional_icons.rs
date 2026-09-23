//! Conditional icons are a display channel, never a cell value or stored style.
use crate::{
    conditional_data_bar::{S, X14, XM},
    office::*,
};
use serde_json::{json, Value};

pub const SETS: &[&str] = &[
    "3Arrows",
    "3ArrowsGray",
    "3Flags",
    "3TrafficLights1",
    "3TrafficLights2",
    "3Signs",
    "3Symbols",
    "3Symbols2",
    "4Arrows",
    "4ArrowsGray",
    "4RedToBlack",
    "4Rating",
    "4TrafficLights",
    "5Arrows",
    "5ArrowsGray",
    "5Rating",
    "5Quarters",
    "3Stars",
    "3Triangles",
    "5Boxes",
];
pub fn count(set: &str) -> Option<usize> {
    SETS.contains(&set)
        .then(|| (set.as_bytes()[0] - b'0') as usize)
}
fn flag(v: &Value, key: &str, default: bool) -> bool {
    v[key].as_bool().unwrap_or(default)
}
pub fn extended(v: &Value) -> bool {
    flag(v, "extended", false)
        || v.get("icons").is_some()
        || ["3Stars", "3Triangles", "5Boxes"].contains(&v["set"].as_str().unwrap_or(""))
}
pub fn validate(v: &Value) -> Result<(), String> {
    v.as_object().ok_or("Icon-set settings missing")?;
    let custom = v.get("icons").is_some();
    let set = v["set"].as_str().unwrap_or("");
    let set_size = count(set);
    if set_size.is_none() && !(custom && set == "NoIcons") {
        return Err("Choose a supported icon set".into());
    }
    for key in ["reverse", "showValue", "percent", "extended"] {
        if v.get(key).is_some_and(|v| !v.is_boolean()) {
            return Err(format!("Invalid icon-set {key}"));
        }
    }
    let points = v["thresholds"]
        .as_array()
        .ok_or("Icon thresholds missing")?;
    let size = if custom {
        points.len()
    } else {
        set_size.unwrap()
    };
    if !(3..=5).contains(&size) || points.len() != size {
        return Err("The number of thresholds must match the icon set".into());
    }
    for p in points {
        let t = p["type"].as_str().unwrap_or("");
        if !["num", "percent", "percentile", "formula"].contains(&t) {
            return Err("Choose Number, Percent, Percentile or Formula for icon thresholds".into());
        }
        if p.get("gte").is_some_and(|v| !v.is_boolean()) {
            return Err("Invalid icon comparison".into());
        }
        let input = p["value"].as_str().ok_or("Enter an icon threshold")?;
        if t == "formula" {
            if input.is_empty()
                || input.len() > 8192
                || !crate::conditional_data_bar::absolute_scalar_formula(input)
            {
                return Err("Icon formulas need absolute cell references, such as $B$1, without ranges or arrays".into());
            }
        } else if input
            .parse::<f64>()
            .ok()
            .is_none_or(|n| !n.is_finite() || t != "num" && !(0.0..=100.0).contains(&n))
        {
            return Err(
                "Enter valid icon thresholds; percentages must be between 0 and 100".into(),
            );
        }
    }
    // The first CFVO is a placeholder, not an evaluated threshold.
    for pair in points[1..].windows(2) {
        if pair[0]["type"] == pair[1]["type"]
            && pair[0]["type"] != "formula"
            && pair[0]["value"].as_str().unwrap().parse::<f64>().unwrap()
                > pair[1]["value"].as_str().unwrap().parse::<f64>().unwrap()
        {
            return Err("Icon thresholds must be in ascending order".into());
        }
    }
    if let Some(icons) = v.get("icons") {
        let icons = icons.as_array().ok_or("Invalid custom icons")?;
        if icons.len() != size {
            return Err("Choose one icon for each category".into());
        }
        for icon in icons {
            let set = icon["set"].as_str().ok_or("Missing custom icon set")?;
            let size = if set == "NoIcons" {
                1
            } else {
                count(set).ok_or("Unknown custom icon set")?
            };
            if icon["index"].as_u64().is_none_or(|i| i >= size as u64) {
                return Err("Invalid custom icon index".into());
            }
        }
    }
    Ok(())
}
/// Extended icon rules have no standard fallback. Do not silently discard
/// source metadata while changing a standard rule into an extension-only one.
pub fn validate_promotion(r: &Value) -> Result<(), String> {
    if !extended(&r["iconSet"]) {
        return Ok(());
    }
    let message = "This rule contains additional Excel metadata. Keep its original icon style until that metadata can be converted.";
    if r["groupExtras"].as_str().is_some_and(|s| !s.is_empty()) || r.get("officeDxfXml").is_some() {
        return Err(message.into());
    }
    for (key, allowed) in [
        ("officeXml", &["type", "priority", "stopIfTrue"][..]),
        ("groupXml", &["sqref"][..]),
    ] {
        if let Some(xml) = r[key].as_str() {
            let doc = parse(xml)?;
            if doc
                .root_element()
                .attributes()
                .any(|a| a.namespace().is_some() || !allowed.contains(&a.name()))
            {
                return Err(message.into());
            }
        }
    }
    Ok(())
}
fn read_flag(n: roxmltree::Node<'_, '_>, key: &str, default: bool) -> Result<bool, String> {
    match attr(n, key) {
        Some("0" | "false") => Ok(false),
        Some("1" | "true") => Ok(true),
        None => Ok(default),
        _ => Err(format!("Invalid {key}")),
    }
}
pub fn read(n: roxmltree::Node<'_, '_>) -> Result<Value, String> {
    let ext = n.has_tag_name((X14, "iconSet"));
    let ns = if ext { X14 } else { S };
    if !n.has_tag_name((ns, "iconSet"))
        || n.attributes().any(|a| {
            a.namespace().is_some()
                || !["iconSet", "reverse", "showValue", "percent"].contains(&a.name())
                    && !(ext && a.name() == "custom")
        })
    {
        return Err("Unmodeled icon-set attributes".into());
    }
    let mut value = json!({"set":attr(n,"iconSet").unwrap_or("3TrafficLights1"),"extended":ext,
        "reverse":read_flag(n,"reverse",false)?,"showValue":read_flag(n,"showValue",true)?,
        "percent":read_flag(n,"percent",true)?});
    if !ext && extended(&value) {
        return Err("This icon set requires an Excel extension".into());
    }
    let custom = ext && read_flag(n, "custom", false)?;
    let mut points = vec![];
    let mut icons = vec![];
    for c in n.children().filter(|c| c.is_element()) {
        if c.has_tag_name((ns, "cfvo")) {
            if c.attributes().any(|a| {
                a.namespace().is_some()
                    || !["type", "gte"].contains(&a.name()) && !(a.name() == "val" && !ext)
            }) || c.children().filter(|c| c.is_element()).any(|c| {
                !ext || !c.has_tag_name((XM, "f"))
                    || c.attributes().len() > 0
                    || c.children().any(|n| n.is_element())
            }) || c.children().filter(|c| c.is_element()).count() > 1
            {
                return Err("Unmodeled icon threshold".into());
            }
            let input = if ext {
                c.children()
                    .find(|c| c.has_tag_name((XM, "f")))
                    .and_then(|c| c.text())
            } else {
                attr(c, "val")
            }
            .unwrap_or("");
            points.push(json!({"type":attr(c,"type").unwrap_or(""),"value":input,"gte":read_flag(c,"gte",true)?}));
        } else if ext && custom && c.has_tag_name((X14, "cfIcon")) {
            if c.attributes()
                .any(|a| a.namespace().is_some() || !["iconSet", "iconId"].contains(&a.name()))
                || c.children().any(|c| c.is_element())
            {
                return Err("Unmodeled custom icon".into());
            }
            icons.push(json!({"set":attr(c,"iconSet").unwrap_or(""),"index":attr(c,"iconId").and_then(|s|s.parse::<u64>().ok())}));
        } else {
            return Err("Unmodeled icon-set children".into());
        }
    }
    value["thresholds"] = json!(points);
    if custom {
        value["icons"] = json!(icons);
    }
    validate(&value)?;
    Ok(value)
}
pub fn read_extension(n: roxmltree::Node<'_, '_>, sqref: &str) -> Result<Value, String> {
    let id = attr(n, "id")
        .filter(|s| !s.is_empty())
        .ok_or("Missing icon rule ID")?;
    let priority = attr(n, "priority")
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .ok_or("Missing icon priority")?;
    if n.attributes().any(|a| {
        a.namespace().is_some() || !["id", "type", "priority", "stopIfTrue"].contains(&a.name())
    }) || read_flag(n, "stopIfTrue", false)?
        || n.children().filter(|c| c.is_element()).count() != 1
    {
        return Err("Unmodeled extended icon rule".into());
    }
    let node = n
        .children()
        .find(|c| c.has_tag_name((X14, "iconSet")))
        .ok_or("Missing icon set")?;
    let rule = json!({"type":"iconSet","sqref":sqref,"priority":priority,"style":{},"iconSet":read(node)?,"officeIconSetId":id});
    crate::conditional_format::validate_rule(&rule)?;
    Ok(rule)
}
pub fn xml(v: &Value, ext: bool) -> String {
    let tag = if ext { "x14:" } else { "" };
    let namespace = if ext {
        String::new()
    } else {
        format!(" xmlns=\"{S}\"")
    };
    let custom = v.get("icons").is_some();
    let mut out = format!(
        "<{tag}iconSet{namespace} iconSet=\"{}\" reverse=\"{}\" showValue=\"{}\" percent=\"{}\"{}>",
        v["set"].as_str().unwrap(),
        u8::from(flag(v, "reverse", false)),
        u8::from(flag(v, "showValue", true)),
        u8::from(flag(v, "percent", true)),
        if ext {
            format!(" custom=\"{}\"", u8::from(custom))
        } else {
            String::new()
        }
    );
    for p in v["thresholds"].as_array().unwrap() {
        let t = p["type"].as_str().unwrap();
        let value = esc(p["value"].as_str().unwrap().trim_start_matches('='));
        let gte = u8::from(flag(p, "gte", true));
        out += &if ext {
            format!("<x14:cfvo type=\"{t}\" gte=\"{gte}\"><xm:f>{value}</xm:f></x14:cfvo>")
        } else {
            format!("<cfvo type=\"{t}\" val=\"{value}\" gte=\"{gte}\"/>")
        };
    }
    for icon in v["icons"].as_array().into_iter().flatten() {
        out += &format!(
            "<x14:cfIcon iconSet=\"{}\" iconId=\"{}\"/>",
            icon["set"].as_str().unwrap(),
            icon["index"].as_u64().unwrap()
        );
    }
    out += &format!("</{tag}iconSet>");
    out
}
pub fn extension_xml(r: &Value, id: &str) -> String {
    format!("<x14:conditionalFormatting xmlns:xm=\"{XM}\"><x14:cfRule type=\"iconSet\" id=\"{}\" priority=\"{}\">{}</x14:cfRule><xm:sqref>{}</xm:sqref></x14:conditionalFormatting>",
        esc(id),r["priority"].as_u64().unwrap(),xml(&r["iconSet"],true),esc(r["sqref"].as_str().unwrap()))
}
pub fn evaluate(
    v: &Value,
    numbers: &[f64],
    value: f64,
    mut formula: impl FnMut(&str) -> Result<f64, String>,
) -> Result<Option<Value>, String> {
    let Some(&min) = numbers.first() else {
        return Ok(None);
    };
    let max = *numbers.last().unwrap();
    let mut category = 0;
    let mut previous = f64::NEG_INFINITY;
    for (i, p) in v["thresholds"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .skip(1)
    {
        let input = p["value"].as_str().unwrap();
        let threshold = match p["type"].as_str().unwrap() {
            "num" => input.parse().unwrap(),
            "percent" => {
                let p = input.parse::<f64>().unwrap() / 100.0;
                (1.0 - p) * min + p * max
            }
            "percentile" => {
                let f = (numbers.len() - 1) as f64 * input.parse::<f64>().unwrap() / 100.0;
                let i = f.floor() as usize;
                let j = (i + 1).min(numbers.len() - 1);
                (1.0 - f.fract()) * numbers[i] + f.fract() * numbers[j]
            }
            _ => {
                let input_trimmed = input.trim().trim_start_matches('=');
                if extended(v)
                    && !input_trimmed.starts_with(['-', '+'])
                    && input_trimmed.parse::<f64>().is_ok()
                {
                    0.0
                } else {
                    formula(input)?
                }
            }
        };
        if !threshold.is_finite() || threshold < previous {
            return Err("Icon thresholds must be finite and in ascending order".into());
        }
        previous = threshold;
        if value > threshold || flag(p, "gte", true) && value == threshold {
            category = i;
        }
    }
    let size = v["thresholds"].as_array().unwrap().len();
    let (set, index) = if let Some(icons) = v["icons"].as_array() {
        (
            icons[category]["set"].as_str().unwrap(),
            icons[category]["index"].as_u64().unwrap() as usize,
        )
    } else {
        (
            v["set"].as_str().unwrap(),
            if flag(v, "reverse", false) {
                size - 1 - category
            } else {
                category
            },
        )
    };
    Ok(Some(
        json!({"set":set,"index":index,"category":category,"count":size,"showValue":flag(v,"showValue",true)}),
    ))
}
