use crate::office::*;
use serde_json::{json, Map, Value};

pub const KEYS: &[(&str, &str)] = &[
    ("horizontalAlignment", "horizontal"),
    ("verticalAlignment", "vertical"),
    ("wrapText", "wrapText"),
    ("shrinkToFit", "shrinkToFit"),
    ("indent", "indent"),
    ("textRotation", "textRotation"),
    ("readingOrder", "readingOrder"),
    ("justifyLastLine", "justifyLastLine"),
];
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

pub fn project(node: Option<roxmltree::Node>, spec: &mut Value) {
    for (key, xml) in KEYS {
        let value = node.and_then(|n| n.attribute(*xml));
        spec[*key] = match *key {
            "horizontalAlignment" => json!(value.unwrap_or("general")),
            "verticalAlignment" => json!(value.unwrap_or("bottom")),
            "wrapText" | "shrinkToFit" | "justifyLastLine" => {
                json!(matches!(value, Some("1" | "true")))
            }
            "textRotation" => {
                let n = value.and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
                json!(if (91..=180).contains(&n) { 90 - n } else { n })
            }
            _ => json!(value.and_then(|v| v.parse::<u32>().ok()).unwrap_or(0)),
        };
    }
}

pub fn validate(key: &str, value: &Value) -> Result<(), String> {
    let valid = match key {
        "horizontalAlignment" => matches!(
            value.as_str(),
            Some(
                "general"
                    | "left"
                    | "center"
                    | "right"
                    | "fill"
                    | "justify"
                    | "centerContinuous"
                    | "distributed"
            )
        ),
        "verticalAlignment" => matches!(
            value.as_str(),
            Some("top" | "center" | "bottom" | "justify" | "distributed")
        ),
        "wrapText" | "shrinkToFit" | "justifyLastLine" => value.is_boolean(),
        "indent" => value.as_u64().is_some_and(|n| n <= 250),
        "readingOrder" => value.as_u64().is_some_and(|n| n <= 2),
        "textRotation" => value
            .as_i64()
            .is_some_and(|n| (-90..=90).contains(&n) || n == 255),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(format!("Invalid cell alignment: {key}."))
    }
}

pub fn hydrate_defaults(target: &mut Value, source: &Value) {
    fn missing(target: &mut Value, source: &Value) {
        let Some(target) = target.as_object_mut() else {
            return;
        };
        for (key, _) in KEYS {
            if !target.contains_key(*key) {
                if let Some(value) = source.get(*key) {
                    target.insert((*key).into(), value.clone());
                }
            }
        }
    }
    missing(&mut target["default"], &source["default"]);
    if let Some(rows) = target["rows"].as_object_mut() {
        for (id, style) in rows {
            missing(style, &source["rows"][id]);
        }
    }
    if let Some(columns) = target["columns"].as_array_mut() {
        for column in columns {
            if let Some(original) = source["columns"].as_array().and_then(|cols| {
                cols.iter()
                    .find(|c| c["min"] == column["min"] && c["max"] == column["max"])
            }) {
                missing(&mut column["style"], &original["style"]);
            }
        }
    }
}

/// Patch chosen properties of the effective source alignment record. Retain
/// unsupported attributes and extension namespaces when materializing an XF.
pub fn patch(
    source: Option<String>,
    spec: &Map<String, Value>,
    projected: &Value,
) -> Result<Option<String>, String> {
    let mut changes = vec![];
    for (key, tag) in KEYS {
        if let Some(value) = spec.get(*key).filter(|v| projected[*key] != **v) {
            validate(key, value)?;
            let text = if value.is_boolean() {
                if value == true {
                    "1".into()
                } else {
                    "0".into()
                }
            } else if *key == "textRotation" && value.as_i64().unwrap() < 0 {
                (90 - value.as_i64().unwrap()).to_string()
            } else {
                value
                    .as_str()
                    .map(String::from)
                    .unwrap_or_else(|| value.to_string())
            };
            changes.push((*tag, Some(text)));
        }
    }
    if changes.is_empty() {
        return Ok(None);
    }
    let source = source.unwrap_or_else(|| format!("<alignment xmlns=\"{S}\"/>"));
    let document = parse(&source)?;
    Ok(Some(crate::xlsx_tables::attributes(
        &source,
        document.root_element(),
        &changes,
    )))
}
