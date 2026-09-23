//! Sparse worksheet dimensions and manual visibility, independent of filters.
use crate::{
    office::*,
    structure::{Change, Edit},
    xlsx_tables::{attributes, inner, replace_child},
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// Missing modes on older saved workspaces retain their explicit fixed sizes.
pub fn automatic_row(sheet: &Value, row: &str) -> bool {
    match sheet["rowHeightModes"][row].as_str() {
        Some("auto") => true,
        Some("manual") => false,
        _ => sheet["rowHeights"].get(row).is_none() && sheet["defaultRowHeightCustom"] != true,
    }
}

fn row_mode(sheet: &mut Value, row: &str, mode: &str) {
    if !sheet["rowHeightModes"].is_object() {
        sheet["rowHeightModes"] = json!({});
    }
    sheet["rowHeightModes"][row] = json!(mode);
}

pub fn validate(sheet: &Value) -> Result<(), String> {
    if let Some(ranges) = sheet.get("mergedRanges") {
        for range in ranges.as_array().ok_or("Invalid merged ranges")? {
            crate::tables::Area::parse(range.as_str().ok_or("Invalid merged range")?)?;
        }
    }
    if let Some(modes) = sheet.get("rowHeightModes") {
        for (r, mode) in modes.as_object().ok_or("Invalid row height modes")? {
            if r.parse::<usize>()
                .ok()
                .filter(|r| (1..=1048576).contains(r))
                .is_none_or(|n| n.to_string() != *r)
                || !matches!(mode.as_str(), Some("auto" | "manual"))
            {
                return Err("Invalid row height mode".into());
            }
        }
    }
    if sheet
        .get("defaultRowHeightCustom")
        .is_some_and(|v| !v.is_boolean())
    {
        return Err("Invalid default row height mode".into());
    }
    for (field, bound, size) in [
        ("rowHeights", 1048576, 409.0),
        ("columnWidths", 16384, 255.0),
    ] {
        if let Some(map) = sheet.get(field) {
            for (k, v) in map.as_object().ok_or("Invalid worksheet dimensions")? {
                if k.parse::<usize>()
                    .ok()
                    .filter(|n| *n > 0 && *n <= bound && n.to_string() == *k)
                    .is_none()
                    || v.as_f64()
                        .is_none_or(|n| !n.is_finite() || n < 0.0 || n > size)
                {
                    return Err("Invalid worksheet dimension".into());
                }
            }
        }
    }
    if let Some(cols) = sheet.get("hiddenColumns") {
        if cols.as_array().is_none_or(|a| {
            a.iter()
                .any(|n| n.as_u64().is_none_or(|n| n == 0 || n > 16384))
        }) {
            return Err("Invalid hidden columns".into());
        }
    }
    for (field, max) in [("defaultRowHeight", 409.0), ("defaultColumnWidth", 255.0)] {
        if sheet.get(field).is_some_and(|v| {
            v.as_f64()
                .is_none_or(|v| !v.is_finite() || v <= 0.0 || v > max)
        }) {
            return Err("Invalid default worksheet dimension".into());
        }
    }
    Ok(())
}

pub fn change(workspace: &Value, request: &Value) -> Result<Value, String> {
    let mut next = workspace.clone();
    let sheet = next["sheets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["id"] == request["sheetId"])
        .ok_or("Worksheet missing")?;
    let rows = match request["axis"].as_str() {
        Some("rows") => true,
        Some("columns") => false,
        _ => return Err("Choose rows or columns".into()),
    };
    let limit = if rows { 1048576 } else { 16384 };
    let action = request["action"]
        .as_str()
        .ok_or("Choose a dimension action")?;
    let hidden_key = if rows { "hiddenRows" } else { "hiddenColumns" };
    let size_key = if rows { "rowHeights" } else { "columnWidths" };
    if action == "autoFit" {
        if request["baseRevision"] != workspace["revision"] {
            return Err("AutoFit needs current measurements. Try again.".into());
        }
        let at = request["at"].as_u64().ok_or("Choose a row or column")?;
        let count = request["count"].as_u64().ok_or("Choose rows or columns")?;
        if at == 0
            || at > limit as u64
            || count == 0
            || count > 100000
            || count > limit as u64 + 1 - at
        {
            return Err("Choose up to 100,000 rows or columns within the worksheet".into());
        }
        let sizes = request["sizes"].as_object().ok_or("Missing measurements")?;
        let hidden: BTreeSet<u64> = sheet[hidden_key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_u64)
            .chain(
                if rows {
                    crate::tables::filtered_rows(sheet)
                } else {
                    Default::default()
                }
                .into_iter()
                .map(|r| r as u64),
            )
            .collect();
        for (key, size) in sizes {
            let row = key
                .parse::<u64>()
                .map_err(|_| "Invalid measured dimension")?;
            if row.to_string() != *key
                || row < at
                || row >= at + count
                || hidden.contains(&row)
                || sheet[size_key][key].as_f64() == Some(0.0)
                || size.as_f64().is_none_or(|n| {
                    !n.is_finite() || n <= 0.0 || n > if rows { 409.0 } else { 255.0 }
                })
            {
                return Err(
                    "Invalid measurement; hidden rows or columns must stay unchanged".into(),
                );
            }
        }
        for row in at..at + count {
            if !hidden.contains(&row)
                && sheet[size_key][row.to_string()].as_f64() != Some(0.0)
                && !sizes.contains_key(&row.to_string())
            {
                return Err("AutoFit must measure every visible selected row or column".into());
            }
        }
        if !sizes.is_empty() {
            if !sheet[size_key].is_object() {
                sheet[size_key] = json!({});
            }
            sheet[size_key]
                .as_object_mut()
                .unwrap()
                .extend(sizes.clone());
            if rows {
                for row in sizes.keys() {
                    row_mode(sheet, row, "auto");
                }
            }
        }
        validate(sheet)?;
        return Ok(next);
    }
    let mut hidden: BTreeSet<usize> = sheet[hidden_key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .map(|n| n as usize)
        .collect();
    if action == "unhideAll" {
        hidden.clear();
        // A zero dimension is another form of hiding. Restore the default size.
        let zero_rows: Vec<String> = sheet[size_key]
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(_, v)| v.as_f64() == Some(0.0))
            .map(|(k, _)| k.clone())
            .collect();
        if let Some(sizes) = sheet[size_key].as_object_mut() {
            sizes.retain(|_, v| v.as_f64() != Some(0.0));
        }
        if rows {
            if let Some(modes) = sheet["rowHeightModes"].as_object_mut() {
                for r in zero_rows {
                    modes.remove(&r);
                }
            }
        }
    } else {
        let at = request["at"].as_u64().ok_or("Choose a row or column")? as usize;
        let count = request["count"]
            .as_u64()
            .ok_or("Choose how many rows or columns")? as usize;
        if at == 0 || count == 0 || count > 100000 || at > limit || count > limit - at + 1 {
            return Err("Choose up to 100,000 rows or columns within the worksheet".into());
        }
        let size = if action == "resize" {
            let n = request["size"].as_f64().ok_or("Enter a numeric size")?;
            let max = if rows { 409.0 } else { 255.0 };
            if !n.is_finite() || n < 0.0 || n > max {
                return Err(format!("Enter a size from 0 to {max}"));
            }
            Some(n)
        } else {
            None
        };
        if !matches!(action, "hide" | "unhide" | "resize") {
            return Err("Unknown dimension action".into());
        }
        for n in at..at + count {
            if action == "hide" || size == Some(0.0) {
                hidden.insert(n);
            } else {
                hidden.remove(&n);
            }
            if let Some(size) = size.filter(|s| *s > 0.0) {
                if !sheet[size_key].is_object() {
                    sheet[size_key] = json!({});
                }
                sheet[size_key][n.to_string()] = json!(size);
                if rows {
                    row_mode(sheet, &n.to_string(), "manual");
                }
            } else if action == "unhide" && sheet[size_key][n.to_string()].as_f64() == Some(0.0) {
                sheet[size_key]
                    .as_object_mut()
                    .unwrap()
                    .remove(&n.to_string());
                if rows {
                    if let Some(modes) = sheet["rowHeightModes"].as_object_mut() {
                        modes.remove(&n.to_string());
                    }
                }
            }
        }
        let unavailable = if rows {
            hidden
                .union(&crate::tables::filtered_rows(sheet).into_iter().collect())
                .count()
        } else {
            hidden.len()
        };
        if unavailable >= limit {
            return Err("Keep at least one row or column visible".into());
        }
    }
    sheet[hidden_key] = json!(hidden);
    validate(sheet)?;
    Ok(next)
}

/// Source merges are not rendered as editable merged cells yet. Reject fitting
/// their rows instead of deriving a destructive height from an unmerged view.
pub fn check_autofit_merges(pkg: &Package, path: &str, request: &Value) -> Result<(), String> {
    let xml = pkg.text(path)?;
    let doc = parse(xml)?;
    if let Some(merges) = child(doc.root_element(), "mergeCells") {
        let sizes = request["sizes"]
            .as_object()
            .ok_or("Missing row measurements")?;
        for merge in merges
            .children()
            .filter(|n| n.has_tag_name((S, "mergeCell")))
        {
            let area =
                crate::tables::Area::parse(attr(merge, "ref").ok_or("Invalid merged range")?)?;
            let (first, last) = if request["axis"] == "rows" {
                (area.top, area.bottom)
            } else {
                (area.left, area.right)
            };
            if sizes
                .keys()
                .any(|r| r.parse::<usize>().is_ok_and(|r| r > first && r <= last + 1))
            {
                return Err("AutoFit is not available for rows or columns containing merged cells. Set their size manually.".into());
            }
        }
    }
    Ok(())
}

pub fn read(pkg: &Package, path: &str, sheet: &mut Value) -> Result<(), String> {
    read_merges(pkg, path, sheet)?;
    let xml = pkg.text(path)?;
    let doc = parse(xml)?;
    let root = doc.root_element();
    let mut heights = serde_json::Map::new();
    let mut modes = serde_json::Map::new();
    let mut widths = serde_json::Map::new();
    let mut hidden = BTreeSet::new();
    if let Some(data) = child(root, "sheetData") {
        for row in data.children().filter(|n| n.has_tag_name((S, "row"))) {
            if let Some(r) = attr(row, "r") {
                if attr(row, "ht").is_some()
                    || matches!(attr(row, "customHeight"), Some("1" | "true"))
                {
                    modes.insert(
                        r.into(),
                        json!(if matches!(attr(row, "customHeight"), Some("1" | "true")) {
                            "manual"
                        } else {
                            "auto"
                        }),
                    );
                }
            }
            if let (Some(r), Some(h)) = (
                attr(row, "r"),
                attr(row, "ht").and_then(|v| v.parse::<f64>().ok()),
            ) {
                heights.insert(r.into(), json!(h));
            }
        }
    }
    if let Some(cols) = child(root, "cols") {
        for col in cols.children().filter(|n| n.has_tag_name((S, "col"))) {
            let first = attr(col, "min")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let last = attr(col, "max")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            if first == 0 || first > last || last > 16384 {
                return Err("Invalid source column dimensions".into());
            }
            for c in first..=last {
                if let Some(w) = attr(col, "width").and_then(|v| v.parse::<f64>().ok()) {
                    widths.insert(c.to_string(), json!(w));
                }
                if matches!(attr(col, "hidden"), Some("1" | "true"))
                    || attr(col, "width").and_then(|v| v.parse::<f64>().ok()) == Some(0.0)
                {
                    hidden.insert(c);
                } else {
                    hidden.remove(&c);
                }
            }
        }
    }
    sheet["rowHeights"] = json!(heights);
    sheet["rowHeightModes"] = json!(modes);
    sheet["columnWidths"] = json!(widths);
    sheet["hiddenColumns"] = json!(hidden);
    if let Some(format) = child(root, "sheetFormatPr") {
        if matches!(attr(format, "customHeight"), Some("1" | "true")) {
            sheet["defaultRowHeightCustom"] = json!(true);
        }
        for (key, field) in [
            ("defaultRowHeight", "defaultRowHeight"),
            ("defaultColWidth", "defaultColumnWidth"),
        ] {
            if let Some(n) = attr(format, key)
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|n| *n > 0.0)
            {
                sheet[field] = json!(n);
            }
        }
    }
    // Explicit zero-height source rows must also disappear in the viewport.
    let mut rows: BTreeSet<u64> = sheet["hiddenRows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect();
    for (r, h) in sheet["rowHeights"].as_object().unwrap() {
        if h.as_f64() == Some(0.0) {
            rows.insert(r.parse().map_err(|_| "Invalid source row")?);
        }
    }
    sheet["hiddenRows"] = json!(rows);
    validate(sheet)
}

/// Read-only merge geometry prevents ordinary text from painting through an
/// imported merged cell. The source package still owns merge editing.
pub fn read_merges(pkg: &Package, path: &str, sheet: &mut Value) -> Result<(), String> {
    let doc = parse(pkg.text(path)?)?;
    let mut ranges = vec![];
    if let Some(merges) = child(doc.root_element(), "mergeCells") {
        for node in merges
            .children()
            .filter(|n| n.has_tag_name((S, "mergeCell")))
        {
            ranges.push(
                crate::tables::Area::parse(attr(node, "ref").ok_or("Invalid merged range")?)?
                    .text(),
            );
        }
    }
    if ranges.is_empty() {
        sheet.as_object_mut().unwrap().remove("mergedRanges");
    } else {
        sheet["mergedRanges"] = json!(ranges);
    }
    Ok(())
}

pub fn hydrate_modes(pkg: &Package, path: &str, sheet: &mut Value) -> Result<(), String> {
    if sheet.get("rowHeightModes").is_some() {
        return Ok(());
    }
    let mut source = json!({});
    read(pkg, path, &mut source)?;
    let mut modes = serde_json::Map::new();
    for (r, mode) in source["rowHeightModes"].as_object().unwrap() {
        if sheet["rowHeights"].get(r) == source["rowHeights"].get(r) {
            modes.insert(r.clone(), mode.clone());
        }
    }
    for (r, _) in sheet["rowHeights"].as_object().into_iter().flatten() {
        modes.entry(r.clone()).or_insert(json!("manual"));
    }
    sheet["rowHeightModes"] = json!(modes);
    if let Some(custom) = source.get("defaultRowHeightCustom") {
        sheet["defaultRowHeightCustom"] = custom.clone();
    }
    Ok(())
}

pub fn structural(next: &mut Value, change: &Change) -> Result<(), String> {
    let Edit::Axis { rows, .. } = change.edit else {
        return Ok(());
    };
    let sheet = &mut next["sheets"][change.sheet];
    let key = if rows { "rowHeights" } else { "columnWidths" };
    let limit = if rows { 1048576 } else { 16384 };
    for key in if rows {
        vec![key, "rowHeightModes"]
    } else {
        vec![key]
    } {
        if let Some(map) = sheet[key].as_object() {
            let mut moved = serde_json::Map::new();
            for (at, size) in map {
                if let Some(to) = change.point(at.parse().map_err(|_| "Invalid dimension index")?) {
                    if to < 1 || to > limit {
                        return Err("This would push worksheet dimensions beyond the edge".into());
                    }
                    moved.insert(to.to_string(), size.clone());
                }
            }
            sheet[key] = json!(moved);
        }
    }
    if !rows {
        if let Some(hidden) = sheet["hiddenColumns"].as_array() {
            let moved: Vec<_> = hidden
                .iter()
                .filter_map(Value::as_i64)
                .filter_map(|c| change.point(c as i32))
                .collect();
            if moved.iter().any(|n| *n < 1 || *n > limit) {
                return Err("This would push hidden columns beyond the edge".into());
            }
            sheet["hiddenColumns"] = json!(moved);
        }
    }
    Ok(())
}

pub fn write(pkg: &mut Package, sheet: &Value, path: &str) -> Result<(), String> {
    if let Some(heights) = sheet["rowHeights"].as_object() {
        let xml = pkg.text(path)?.to_string();
        let doc = parse(&xml)?;
        let data = child(doc.root_element(), "sheetData").ok_or("Worksheet data missing")?;
        let mut rows = BTreeMap::new();
        for row in data.children().filter(|n| n.is_element()) {
            let r = attr(row, "r")
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or("Invalid source row")?;
            let height = heights.get(&r.to_string()).and_then(Value::as_f64);
            let source = attr(row, "ht").and_then(|v| v.parse::<f64>().ok());
            let manual = !automatic_row(sheet, &r.to_string());
            let source_manual = matches!(attr(row, "customHeight"), Some("1" | "true"));
            rows.insert(
                r,
                if height == source && manual == source_manual {
                    raw(&xml, row).into()
                } else {
                    attributes(
                        &xml,
                        row,
                        &[
                            ("ht", height.map(|n| n.to_string())),
                            ("customHeight", manual.then(|| "1".into())),
                        ],
                    )
                },
            );
        }
        for (r, h) in heights {
            rows.entry(r.parse().map_err(|_| "Invalid row height index")?)
                .or_insert_with(|| {
                    let custom = if automatic_row(sheet, r) {
                        ""
                    } else {
                        " customHeight=\"1\""
                    };
                    format!("<row xmlns=\"{S}\" r=\"{r}\" ht=\"{h}\"{custom}/>")
                });
        }
        pkg.set(
            path,
            replace_ranges(
                &xml,
                vec![(
                    data.range(),
                    inner(&xml, data, &rows.values().cloned().collect::<String>()),
                )],
            ),
        );
    }
    if sheet["columnWidths"].is_object() || sheet["hiddenColumns"].is_array() {
        let xml = pkg.text(path)?.to_string();
        let doc = parse(&xml)?;
        let mut originals = vec![None; 16385];
        if let Some(cols) = child(doc.root_element(), "cols") {
            for col in cols.children().filter(|n| n.has_tag_name((S, "col"))) {
                let lo = attr(col, "min")
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0);
                let hi = attr(col, "max")
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0);
                if lo == 0 || lo > hi || hi > 16384 {
                    return Err("Invalid source column dimensions".into());
                }
                originals[lo..=hi].fill(Some(col));
            }
        }
        let hidden: BTreeSet<_> = sheet["hiddenColumns"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_u64)
            .collect();
        let empty = format!("<col xmlns=\"{S}\"/>");
        let empty_doc = parse(&empty)?;
        let mut contents = String::new();
        let mut c = 1;
        while c <= 16384 {
            let template = originals[c];
            let source_width = template
                .and_then(|n| attr(n, "width"))
                .and_then(|s| s.parse::<f64>().ok());
            let width = if sheet["columnWidths"].is_object() {
                sheet["columnWidths"][c.to_string()].as_f64()
            } else {
                source_width
            };
            let is_hidden = hidden.contains(&(c as u64));
            let mut last = c;
            while last < 16384
                && originals[last + 1] == template
                && sheet["columnWidths"][(last + 1).to_string()].as_f64() == width
                && hidden.contains(&((last + 1) as u64)) == is_hidden
            {
                last += 1;
            }
            if template.is_some() || width.is_some() || is_hidden {
                let mut changes = vec![
                    ("min", Some(c.to_string())),
                    ("max", Some(last.to_string())),
                ];
                if sheet["hiddenColumns"].is_array() {
                    changes.push(("hidden", is_hidden.then(|| "1".into())));
                }
                if source_width != width {
                    changes.extend([
                        ("width", width.map(|v| v.to_string())),
                        ("customWidth", width.map(|_| "1".into())),
                        ("bestFit", None),
                    ]);
                }
                contents.push_str(&match template {
                    Some(n) => attributes(&xml, n, &changes),
                    None => attributes(&empty, empty_doc.root_element(), &changes),
                });
            }
            c = last + 1;
        }
        let cols = if contents.is_empty() {
            String::new()
        } else {
            format!("<cols xmlns=\"{S}\">{contents}</cols>")
        };
        let result = if child(doc.root_element(), "cols").is_some() {
            replace_child(&xml, "cols", &cols)?
        } else {
            let at = child(doc.root_element(), "sheetData")
                .ok_or("Worksheet data missing")?
                .range()
                .start;
            replace_ranges(&xml, vec![(at..at, cols)])
        };
        pkg.set(path, result);
    }
    Ok(())
}
