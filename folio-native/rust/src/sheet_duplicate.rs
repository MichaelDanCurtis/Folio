//! Move or Copy > Create a copy (X03.06): a new sheet with the source's
//! cells, formats, sizes, merges, rules, notes and view, named "Name (2)".
use serde_json::{json, Value};
use std::collections::HashSet;

/// Excel's name for a copy of `name`: "Budget (2)", or the next free number.
/// The base is shortened so the result fits the 31-character limit.
pub fn copy_name(name: &str, taken: &[String]) -> String {
    let taken: HashSet<String> = taken.iter().map(|n| n.to_lowercase()).collect();
    // A copy of "Budget (2)" is "Budget (3)", not "Budget (2) (2)".
    let base = match name.rsplit_once(" (") {
        Some((stem, rest))
            if rest.ends_with(')')
                && rest[..rest.len() - 1].parse::<u32>().is_ok()
                && !stem.is_empty() =>
        {
            stem
        }
        _ => name,
    };
    (2..)
        .map(|n| {
            let suffix = format!(" ({n})");
            let room = 31 - suffix.chars().count();
            format!("{}{suffix}", base.chars().take(room).collect::<String>())
        })
        .find(|candidate| !taken.contains(&candidate.to_lowercase()))
        .unwrap()
}

/// A table name not in `taken`: the name's stem plus the next number, as
/// Excel names a copied table ("Sales" → "Sales2").
fn table_name(name: &str, taken: &HashSet<String>) -> String {
    let stem = name.trim_end_matches(|c: char| c.is_ascii_digit());
    let stem = if stem.is_empty() { name } else { stem };
    (2..)
        .map(|n| format!("{stem}{n}"))
        .find(|candidate| !taken.contains(&candidate.to_lowercase()))
        .unwrap()
}

/// The workspace with a copy of sheet `sheetId` appended, and the copy's
/// intended position (just after the source).
pub fn append_copy(workspace: &Value, r: &Value) -> Result<(Value, usize), String> {
    let mut next = workspace.clone();
    let sheets = next["sheets"].as_array_mut().ok_or("Worksheets missing")?;
    if sheets.len() >= 30 {
        return Err("Maximum 30 sheets".into());
    }
    let index = sheets
        .iter()
        .position(|s| s["id"] == r["sheetId"])
        .ok_or("Worksheet no longer exists")?;
    let names: Vec<String> = sheets
        .iter()
        .filter_map(|s| s["name"].as_str().map(String::from))
        .collect();
    let mut copy = sheets[index].clone();
    let object = copy.as_object_mut().ok_or("Invalid worksheet")?;
    // Package identity belongs to the source; the export writes new parts.
    for key in ["officePath", "cellWatches", "scenarios"] {
        object.remove(key);
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    copy["id"] = json!(format!("sheet-{stamp}"));
    copy["name"] = json!(copy_name(
        sheets[index]["name"].as_str().unwrap_or("Sheet"),
        &names
    ));
    let mut taken: HashSet<String> = sheets
        .iter()
        .flat_map(crate::tables::list)
        .filter_map(|t| t["name"].as_str().map(str::to_lowercase))
        .collect();
    let mut renames = vec![];
    if let Some(tables) = copy.get_mut("tables").and_then(Value::as_array_mut) {
        for (i, table) in tables.iter_mut().enumerate() {
            let old = table["name"].as_str().unwrap_or("Table").to_string();
            let new = table_name(&old, &taken);
            taken.insert(new.to_lowercase());
            let t = table.as_object_mut().ok_or("Invalid table")?;
            t.remove("officePath");
            t.remove("officeId");
            t.insert("name".into(), json!(new));
            t.insert("id".into(), json!(format!("table-{stamp}-{i}")));
            renames.push((old, new));
        }
    }
    // Formulas in the copy that name its tables use the copies' names.
    if !renames.is_empty() {
        if let Some(cells) = copy["cells"].as_object_mut() {
            for value in cells.values_mut() {
                let Some(formula) = value.as_str().filter(|v| v.starts_with('=')) else {
                    continue;
                };
                let mut text = formula.to_string();
                for (old, new) in &renames {
                    text = crate::table_refs::rename_table(&text, old, new)?;
                }
                *value = json!(text);
            }
        }
    }
    sheets.push(copy);
    Ok((next, index + 1))
}
