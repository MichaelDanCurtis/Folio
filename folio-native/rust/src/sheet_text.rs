use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

pub fn literal(value: &str) -> Result<&str, String> {
    if let Some(text) = value.strip_prefix('\'') {
        return Ok(text);
    }
    if value.starts_with('=')
        || value.parse::<f64>().is_ok()
        || matches!(value.to_uppercase().as_str(), "TRUE" | "FALSE")
    {
        return Err("Select a text cell to format individual characters.".into());
    }
    Ok(value)
}

pub fn byte_at(text: &str, offset: usize) -> Result<usize, String> {
    let mut units = 0;
    for (byte, c) in text.char_indices() {
        if units == offset {
            return Ok(byte);
        }
        units += c.len_utf16();
    }
    if units == offset {
        Ok(text.len())
    } else {
        Err("Invalid character boundary.".into())
    }
}

pub fn validate_style(style: &Value) -> Result<&Map<String, Value>, String> {
    let style = style.as_object().ok_or("Invalid text format")?;
    if style
        .keys()
        .any(|k| !crate::xlsx_styles::FONT_KEYS.contains(&k.as_str()))
    {
        return Err("Only font properties apply to selected characters.".into());
    }
    crate::xlsx_styles::validate_format(style)?;
    Ok(style)
}

pub fn read_edits(text: &str, values: &Value) -> Result<Vec<(usize, usize, Value)>, String> {
    let values = values.as_array().ok_or("Invalid character formats")?;
    if values.len() > 65536 {
        return Err("Too many character formats".into());
    }
    let mut end = 0;
    let mut edits = vec![];
    for v in values {
        let start = v["start"].as_u64().ok_or("Invalid character start")? as usize;
        let stop = v["end"].as_u64().ok_or("Invalid character end")? as usize;
        if start < end || start >= stop {
            return Err("Invalid character format ranges".into());
        }
        byte_at(text, start)?;
        byte_at(text, stop)?;
        validate_style(&v["style"])?;
        edits.push((start, stop, v["style"].clone()));
        end = stop;
    }
    Ok(edits)
}

/// Keep a disjoint, coalesced final-property map rather than a growing command
/// log. Reapplying this map after source package transformations is idempotent.
fn overlay(
    text: &str,
    old: &Value,
    start: usize,
    end: usize,
    style: &Value,
) -> Result<Value, String> {
    let old = read_edits(text, old)?;
    let mut cuts = BTreeSet::from([start, end]);
    for (a, b, _) in &old {
        cuts.insert(*a);
        cuts.insert(*b);
    }
    let cuts: Vec<_> = cuts.into_iter().collect();
    let mut result: Vec<Value> = vec![];
    let mut index = 0;
    for span in cuts.windows(2) {
        let (a, b) = (span[0], span[1]);
        while index < old.len() && old[index].1 <= a {
            index += 1;
        }
        let mut merged = Map::new();
        if let Some((lo, hi, spec)) = old.get(index) {
            if *lo <= a && b <= *hi {
                merged.extend(spec.as_object().unwrap().clone());
            }
        }
        if start <= a && b <= end {
            merged.extend(style.as_object().unwrap().clone());
        }
        if merged.is_empty() {
            continue;
        }
        let merged = Value::Object(merged);
        if let Some(last) = result
            .last_mut()
            .filter(|v| v["end"] == a && v["style"] == merged)
        {
            last["end"] = json!(b);
        } else {
            result.push(json!({"start":a,"end":b,"style":merged}));
        }
    }
    Ok(json!(result))
}

pub fn change(workspace: &Value, request: &Value) -> Result<Value, String> {
    if request["baseRevision"] != workspace["revision"] {
        return Err("The workbook changed. Open Font again to review the text.".into());
    }
    validate_style(&request["style"])?;
    let id = request["sheetId"].as_str().ok_or("Worksheet missing")?;
    let address = request["address"].as_str().ok_or("Cell missing")?;
    crate::formula::parse_address(address)?;
    let mut next = workspace.clone();
    let sheet = next["sheets"]
        .as_array_mut()
        .ok_or("Worksheets missing")?
        .iter_mut()
        .find(|s| s["id"] == id)
        .ok_or("Worksheet missing")?;
    let raw = sheet["cells"][address].as_str().unwrap_or("");
    let text = literal(raw)?.to_string();
    if request["expectedText"] != text {
        return Err("The cell text changed. Select it again.".into());
    }
    let start = request["start"]
        .as_u64()
        .ok_or("Select some characters first.")? as usize;
    let end = request["end"]
        .as_u64()
        .ok_or("Select some characters first.")? as usize;
    byte_at(&text, start)?;
    byte_at(&text, end)?;
    if start >= end {
        return Err("Select some characters first.".into());
    }
    if !sheet["cellStyles"].is_object() {
        sheet["cellStyles"] = json!({});
    }
    if !sheet["cellStyles"][address].is_object() {
        sheet["cellStyles"][address] = json!({});
    }
    let cell = &mut sheet["cellStyles"][address];
    let mut edits = if cell["_richTextSource"] == text {
        cell["_richTextEdits"].clone()
    } else {
        json!([])
    };
    let whole = cell.get("_fontOverrides").cloned().unwrap_or(json!({}));
    validate_style(&whole)?;
    let length = text.encode_utf16().count();
    if !whole.as_object().unwrap().is_empty() {
        edits = overlay(&text, &edits, 0, length, &whole)?;
    }
    edits = overlay(&text, &edits, start, end, &request["style"])?;
    let existing = cell["_richText"]
        .as_array()
        .filter(|runs| {
            runs.iter()
                .map(|r| r["text"].as_str().unwrap_or(""))
                .collect::<String>()
                == text
        })
        .cloned()
        .unwrap_or_else(|| vec![json!({"text":text,"style":{}})]);
    let mut projected = vec![];
    let mut offset = 0;
    for run in existing {
        let part = run["text"].as_str().ok_or("Invalid rich text")?;
        let end_run = offset + part.encode_utf16().count();
        let cuts = BTreeSet::from([
            offset,
            end_run,
            start.clamp(offset, end_run),
            end.clamp(offset, end_run),
        ]);
        let cuts: Vec<_> = cuts.into_iter().collect();
        for span in cuts.windows(2) {
            let (a, b) = (span[0], span[1]);
            let mut font = run["style"].as_object().cloned().unwrap_or_default();
            font.extend(whole.as_object().unwrap().clone());
            if start <= a && b <= end {
                font.extend(request["style"].as_object().unwrap().clone());
            }
            let value = &part[byte_at(part, a - offset)?..byte_at(part, b - offset)?];
            projected.push(json!({"text":value,"style":font}));
        }
        offset = end_run;
    }
    cell["_richText"] = json!(projected);
    cell["_richTextSource"] = json!(text);
    cell["_richTextEdits"] = edits;
    cell.as_object_mut().unwrap().remove("_fontOverrides");
    Ok(next)
}
