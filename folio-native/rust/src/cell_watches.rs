//! Persisted cell watches, shared by the Watch Window and OOXML projection.
use crate::xlsx::fragment_inner;
use crate::{
    formula::{address, parse_address},
    office::*,
    tables::Area,
};
use serde_json::{json, Value};
use std::collections::HashSet;
pub const KEY: &str = "cellWatches";
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub fn entries(sheet: &Value) -> impl Iterator<Item = &String> {
    sheet[KEY].as_object().into_iter().flat_map(|m| m.keys())
}
pub fn validate(sheet: &Value) -> Result<(), String> {
    if let Some(watches) = sheet.get(KEY) {
        let watches = watches.as_object().ok_or("Invalid cell watches")?;
        if watches.len() > 100000 {
            return Err("A worksheet supports up to 100,000 watches".into());
        }
        for (a, v) in watches {
            parse_address(a)?;
            if *v != true {
                return Err("Invalid cell watch".into());
            }
        }
    }
    Ok(())
}
pub fn change(workspace: &Value, r: &Value) -> Result<Value, String> {
    let mut next = workspace.clone();
    match r["action"].as_str() {
        Some("add") => {
            let area = Area::parse(r["range"].as_str().ok_or("Choose cells to watch")?)?;
            let count = (area.bottom - area.top + 1) * (area.right - area.left + 1);
            if count > 100000 {
                return Err("Add at most 100,000 watched cells at once".into());
            }
            let sheet = next["sheets"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|s| s["id"] == r["sheetId"])
                .ok_or("Worksheet missing")?;
            if sheet.get(KEY).is_none() {
                sheet[KEY] = json!({});
            }
            let watches = sheet[KEY].as_object_mut().ok_or("Invalid cell watches")?;
            for row in area.top..=area.bottom {
                for col in area.left..=area.right {
                    watches.insert(address(row, col), json!(true));
                }
            }
        }
        Some("remove") => {
            let cells = r["cells"].as_array().ok_or("Choose watches to remove")?;
            if cells.len() > 100000 {
                return Err("Too many selected watches".into());
            }
            for cell in cells {
                let a = cell["address"].as_str().ok_or("Missing watched cell")?;
                parse_address(a)?;
                let sheet = next["sheets"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|s| s["id"] == cell["sheetId"])
                    .ok_or("Worksheet missing")?;
                if let Some(watches) = sheet.get_mut(KEY).and_then(Value::as_object_mut) {
                    watches.remove(a);
                }
            }
        }
        _ => return Err("Choose Add watch or Delete watch".into()),
    }
    for sheet in next["sheets"].as_array().unwrap() {
        validate(sheet)?;
    }
    Ok(next)
}

pub fn read(pkg: &Package, path: &str, sheet: &mut Value) -> Result<(), String> {
    let xml = pkg.text(path)?;
    let d = parse(xml)?;
    if !d
        .root_element()
        .children()
        .any(|n| n.has_tag_name((S, KEY)))
    {
        // Do not add empty metadata to older recovery files that never had
        // watches. A present empty map still records an intentional removal.
        return Ok(());
    }
    let mut watches = serde_json::Map::new();
    let mut unsupported = false;
    for group in d
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((S, KEY)))
    {
        for n in group.children().filter(|n| n.is_element()) {
            if n.has_tag_name((S, "cellWatch")) {
                if let Some(a) = attr(n, "r").filter(|a| parse_address(a).is_ok()) {
                    watches.insert(a.to_string(), json!(true));
                    continue;
                }
            }
            unsupported = true;
        }
    }
    sheet[KEY] = Value::Object(watches);
    if unsupported {
        sheet["unsupportedCellWatches"] = json!(true);
    }
    validate(sheet)
}

pub fn write(
    pkg: &mut Package,
    sheet: &Value,
    before: Option<&Value>,
    path: &str,
) -> Result<(), String> {
    validate(sheet)?;
    if before.is_some_and(|b| b.get(KEY) == sheet.get(KEY)) {
        return Ok(());
    }
    let wanted: HashSet<&str> = entries(sheet).map(String::as_str).collect();
    let mut seen = HashSet::new();
    let xml = pkg.text(path)?.to_string();
    let d = parse(&xml)?;
    let root = d.root_element();
    let groups: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name((S, KEY)))
        .collect();
    if groups.len() > 1 {
        return Err(
            "Multiple watch collections require additional file support. Nothing changed.".into(),
        );
    }
    let mut inner = String::new();
    if let Some(group) = groups.first() {
        for child in group.children() {
            if child.has_tag_name((S, "cellWatch")) {
                if let Some(a) = attr(child, "r").filter(|a| parse_address(a).is_ok()) {
                    if !wanted.contains(a) || !seen.insert(a.to_string()) {
                        continue;
                    }
                }
            }
            // Preserve unsupported nodes and metadata around ordinary watches.
            inner.push_str(raw(&xml, child));
        }
    }
    for a in entries(sheet) {
        if seen.insert(a.clone()) {
            inner.push_str(&format!("<cellWatch xmlns=\"{S}\" r=\"{a}\"/>"));
        }
    }
    let edit = if let Some(group) = groups.first() {
        (
            group.range(),
            if inner.trim().is_empty() {
                String::new()
            } else {
                fragment_inner(raw(&xml, *group), &inner)?
            },
        )
    } else if inner.is_empty() {
        return Ok(());
    } else {
        // CT_Worksheet: after customProperties, before ignoredErrors and drawings.
        let later = [
            "ignoredErrors",
            "smartTags",
            "drawing",
            "legacyDrawing",
            "legacyDrawingHF",
            "picture",
            "oleObjects",
            "controls",
            "webPublishItems",
            "tableParts",
            "extLst",
        ];
        let at = root
            .children()
            .find(|n| n.is_element() && later.contains(&n.tag_name().name()))
            .map(|n| n.range().start)
            .or_else(|| raw(&xml, root).rfind("</").map(|i| root.range().start + i));
        let group = format!("<cellWatches xmlns=\"{S}\">{inner}</cellWatches>");
        if let Some(at) = at {
            (at..at, group)
        } else {
            (root.range(), fragment_inner(raw(&xml, root), &group)?)
        }
    };
    pkg.set(path, replace_ranges(&xml, vec![edit]));
    Ok(())
}
