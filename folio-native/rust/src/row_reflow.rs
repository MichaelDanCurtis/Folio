//! Determine sparse automatic rows from the complete post-edit workbook,
//! including formula dependents on other worksheets. Flutter measures glyphs;
//! Rust checks completeness before publishing the edit and layout together.
use crate::{formula::parse_address, office::*, sheet_geometry::automatic_row};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub type Targets = BTreeMap<String, BTreeSet<usize>>;

pub fn targets(
    before: &Value,
    before_values: &Value,
    after: &Value,
    after_values: &Value,
    before_display: &Value,
    after_display: &Value,
    package: Option<&Package>,
) -> Result<Targets, String> {
    let mut result = Targets::new();
    for sheet in after["sheets"].as_array().ok_or("Missing worksheets")? {
        let id = sheet["id"].as_str().ok_or("Missing worksheet id")?;
        let old = before["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap_or(&Value::Null);
        let broad = [
            "columnWidths",
            "hiddenColumns",
            "defaultColumnWidth",
            "styleDefaults",
            "tables",
            "autoFilter",
            "hiddenRows",
            "defaultRowHeight",
            "defaultRowHeightCustom",
        ]
        .iter()
        .any(|key| old[key] != sheet[key]);
        let mut rows = BTreeSet::new();
        if broad {
            for source in [old, sheet] {
                for key in ["rowHeights", "rowHeightModes"] {
                    rows.extend(
                        source[key]
                            .as_object()
                            .into_iter()
                            .flatten()
                            .filter_map(|(r, _)| r.parse::<usize>().ok()),
                    );
                }
                rows.extend(
                    source["styleDefaults"]["rows"]
                        .as_object()
                        .into_iter()
                        .flatten()
                        .filter_map(|(r, _)| r.parse::<usize>().ok()),
                );
            }
        }
        // Every address in any of these maps, each visited once.
        let maps: Vec<_> = [
            &old["cells"],
            &sheet["cells"],
            &old["cellStyles"],
            &sheet["cellStyles"],
            &before_values[id],
            &after_values[id],
        ]
        .into_iter()
        .filter_map(Value::as_object)
        .collect();
        for (k, map) in maps.iter().enumerate() {
            for a in map.keys() {
                if maps[..k].iter().any(|m| m.contains_key(a)) {
                    continue;
                }
                if broad
                    || old["cells"][a] != sheet["cells"][a]
                    || old["cellStyles"][a] != sheet["cellStyles"][a]
                    || before_values[id][a] != after_values[id][a]
                    || before_display[id][a] != after_display[id][a]
                {
                    rows.insert(parse_address(a)?.0 + 1);
                }
            }
        }
        let hidden = crate::tables::hidden_rows(sheet);
        let filtered = crate::tables::filtered_rows(sheet);
        rows.retain(|r| {
            (1..=1048576).contains(r)
                && !hidden.contains(r)
                && !filtered.contains(r)
                && sheet["rowHeights"][r.to_string()].as_f64() != Some(0.0)
                && automatic_row(sheet, &r.to_string())
        });
        if !rows.is_empty() {
            if let (Some(pkg), Some(path)) = (package, sheet["officePath"].as_str()) {
                let doc = parse(pkg.text(path)?)?;
                if let Some(merges) = child(doc.root_element(), "mergeCells") {
                    for node in merges.children().filter(|n| n.is_element()) {
                        let area = crate::tables::Area::parse(
                            attr(node, "ref").ok_or("Invalid merged range")?,
                        )?;
                        rows.retain(|r| *r <= area.top || *r > area.bottom + 1);
                    }
                }
            }
        }
        if !rows.is_empty() {
            result.insert(id.into(), rows);
        }
    }
    Ok(result)
}

pub fn apply(workspace: &mut Value, targets: &Targets, measurements: &Value) -> Result<(), String> {
    let measured = measurements
        .as_object()
        .ok_or("Missing automatic row measurements")?;
    if measured.len() != targets.len() || measured.keys().any(|id| !targets.contains_key(id)) {
        return Err("Automatic row measurements do not match this edit".into());
    }
    for sheet in workspace["sheets"].as_array_mut().unwrap() {
        let id = sheet["id"].as_str().unwrap();
        let Some(rows) = targets.get(id) else {
            continue;
        };
        let sizes = measured[id].as_object().ok_or("Invalid row measurements")?;
        if sizes.len() != rows.len() {
            return Err("Every automatic row must be measured".into());
        }
        for r in rows {
            let key = r.to_string();
            let height = sizes
                .get(&key)
                .and_then(Value::as_f64)
                .filter(|n| n.is_finite() && *n > 0.0 && *n <= 409.0)
                .ok_or("Invalid automatic row measurement")?;
            if !sheet["rowHeights"].is_object() {
                sheet["rowHeights"] = json!({});
            }
            if !sheet["rowHeightModes"].is_object() {
                sheet["rowHeightModes"] = json!({});
            }
            sheet["rowHeights"][&key] = json!(height);
            sheet["rowHeightModes"][&key] = json!("auto");
        }
    }
    Ok(())
}
