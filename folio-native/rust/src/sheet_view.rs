//! The primary worksheet view's frozen boundary, separate from its scroll origin.
use crate::{
    formula::{address, parse_address},
    office::*,
    structure::{Change, Edit},
    xlsx_tables::{attributes, inner},
};
use serde_json::{json, Value};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

pub fn validate(sheet: &Value) -> Result<(), String> {
    let pane = &sheet["frozenPane"];
    if pane.is_null() {
        return Ok(());
    }
    if !matches!(pane["state"].as_str(), Some("frozen" | "frozenSplit")) {
        return Err("Invalid frozen pane state".into());
    }
    let rows = pane["rows"]
        .as_u64()
        .filter(|n| *n < 1048576)
        .ok_or("Invalid frozen row count")?;
    let cols = pane["columns"]
        .as_u64()
        .filter(|n| *n < 16384)
        .ok_or("Invalid frozen column count")?;
    let (r, c) = parse_address(
        pane["topLeftCell"]
            .as_str()
            .ok_or("Frozen pane origin missing")?,
    )?;
    if r < rows as usize || c < cols as usize {
        return Err("Scroll origin is inside the frozen pane".into());
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
    sheet["frozenPane"] = match request["action"].as_str() {
        Some("unfreeze") => Value::Null,
        Some("freeze") => {
            let r = request["rows"]
                .as_u64()
                .filter(|n| *n < 1048576)
                .ok_or("Choose fewer than 1,048,576 frozen rows")?;
            let c = request["columns"]
                .as_u64()
                .filter(|n| *n < 16384)
                .ok_or("Choose fewer than 16,384 frozen columns")?;
            if r == 0 && c == 0 {
                return Err("Select a cell below or to the right of the headings to freeze".into());
            }
            json!({"state":"frozen", "rows":r, "columns":c, "topLeftCell":address(r as usize, c as usize)})
        }
        _ => return Err("Choose Freeze or Unfreeze panes".into()),
    };
    validate(sheet)?;
    Ok(next)
}

fn primary<'a, 'input>(views: roxmltree::Node<'a, 'input>) -> Option<roxmltree::Node<'a, 'input>> {
    views
        .children()
        .filter(|n| n.has_tag_name((S, "sheetView")))
        .find(|n| attr(*n, "workbookViewId") == Some("0"))
        .or_else(|| child(views, "sheetView"))
}

pub fn read(pkg: &Package, path: &str, sheet: &mut Value) -> Result<(), String> {
    let doc = parse(pkg.text(path)?)?;
    sheet["frozenPane"] = Value::Null;
    if let Some(pane) = child(doc.root_element(), "sheetViews")
        .and_then(primary)
        .and_then(|v| child(v, "pane"))
    {
        if matches!(attr(pane, "state"), Some("frozen" | "frozenSplit")) {
            let count = |key| -> Result<usize, String> {
                let n = attr(pane, key)
                    .unwrap_or("0")
                    .parse::<f64>()
                    .map_err(|_| "Invalid pane count")?;
                if !n.is_finite() || n < 0.0 || n.fract() != 0.0 || n > 1048576.0 {
                    return Err("Invalid pane count".into());
                }
                Ok(n as usize)
            };
            let rows = count("ySplit")?;
            let cols = count("xSplit")?;
            sheet["frozenPane"] = json!({"state":attr(pane,"state"), "rows":rows,"columns":cols,
                "topLeftCell":attr(pane,"topLeftCell").map(String::from).unwrap_or_else(|| address(rows, cols))});
        }
    }
    validate(sheet)
}

// Anchor a nonempty frozen boundary at its first unfrozen position. Its
// location changes independently of the currently scrolled top-left cell.
// An axis with no frozen cells must remain unfrozen when inserting at 1.
pub fn moved_count(change: &Change, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    match change.edit {
        Edit::Axis { at, .. } => (change.point(count as i32 + 1).unwrap_or(at) - 1) as usize,
        _ => count,
    }
}

pub fn structural(next: &mut Value, change: &Change) -> Result<(), String> {
    let Edit::Axis { rows, at, .. } = change.edit else {
        return Ok(());
    };
    let sheet = &mut next["sheets"][change.sheet];
    if sheet["frozenPane"].is_null() {
        return Ok(());
    }
    let pane = &mut sheet["frozenPane"];
    let key = if rows { "rows" } else { "columns" };
    let count = moved_count(change, pane[key].as_u64().unwrap() as usize);
    let (r, c) = parse_address(pane["topLeftCell"].as_str().unwrap())?;
    let origin = change
        .point(if rows { r as i32 + 1 } else { c as i32 + 1 })
        .unwrap_or(at)
        .max(count as i32 + 1);
    let limit = if rows { 1048576 } else { 16384 };
    if count >= limit || origin as usize > limit {
        return Err("Frozen panes would exceed the worksheet bounds".into());
    }
    pane[key] = json!(count);
    pane["topLeftCell"] = json!(if rows {
        address(origin as usize - 1, c)
    } else {
        address(r, origin as usize - 1)
    });
    if pane["rows"] == 0 && pane["columns"] == 0 {
        sheet["frozenPane"] = Value::Null;
    }
    validate(sheet)
}

pub fn write(
    pkg: &mut Package,
    sheet: &Value,
    before: Option<&Value>,
    path: &str,
) -> Result<(), String> {
    if sheet.get("frozenPane").is_none()
        || before.is_some_and(|s| s["frozenPane"] == sheet["frozenPane"])
    {
        return Ok(());
    }
    validate(sheet)?;
    let xml = pkg.text(path)?.to_string();
    let doc = parse(&xml)?;
    let root = doc.root_element();
    let pane = &sheet["frozenPane"];
    let mut contents = String::new();
    if !pane.is_null() {
        let r = pane["rows"].as_u64().unwrap();
        let c = pane["columns"].as_u64().unwrap();
        let active = match (r > 0, c > 0) {
            (true, true) => "bottomRight",
            (true, false) => "bottomLeft",
            (false, true) => "topRight",
            _ => "topLeft",
        };
        let mut split = String::new();
        if r > 0 {
            split.push_str(&format!(" ySplit=\"{r}\""));
        }
        if c > 0 {
            split.push_str(&format!(" xSplit=\"{c}\""));
        }
        let top = esc(pane["topLeftCell"].as_str().unwrap());
        contents = format!("<pane xmlns=\"{S}\"{split} topLeftCell=\"{top}\" activePane=\"{active}\" state=\"{}\"/>", pane["state"].as_str().unwrap());
        contents.push_str(&format!(
            "<selection xmlns=\"{S}\" pane=\"{active}\" activeCell=\"{top}\" sqref=\"{top}\"/>"
        ));
    }
    let empty = format!("<sheetView xmlns=\"{S}\" workbookViewId=\"0\"/>");
    let empty_doc = parse(&empty)?;
    let views = child(root, "sheetViews");
    let view = views.and_then(primary);
    if let Some(view) = view {
        if pane.is_null() {
            if let Some(selection) = child(view, "selection") {
                contents.push_str(&attributes(&xml, selection, &[("pane", None)]));
            }
        }
        // Preserve protection, pivot selections, extensions and all view attrs.
        for node in view
            .children()
            .filter(|n| n.is_element() && !matches!(n.tag_name().name(), "pane" | "selection"))
        {
            contents.push_str(raw(&xml, node));
        }
    }
    let replacement = match view {
        Some(v) => inner(&xml, v, &contents),
        None => inner(&empty, empty_doc.root_element(), &contents),
    };
    let edit = if let Some(view) = view {
        (view.range(), replacement)
    } else if let Some(views) = views {
        (
            views.range(),
            inner(
                &xml,
                views,
                &format!(
                    "{}{replacement}",
                    views
                        .children()
                        .filter(|n| n.is_element())
                        .map(|n| raw(&xml, n))
                        .collect::<String>()
                ),
            ),
        )
    } else {
        let at = root
            .children()
            .find(|n| n.is_element() && !matches!(n.tag_name().name(), "sheetPr" | "dimension"))
            .ok_or("Worksheet data missing")?
            .range()
            .start;
        (
            at..at,
            format!("<sheetViews xmlns=\"{S}\">{replacement}</sheetViews>"),
        )
    };
    pkg.set(path, replace_ranges(&xml, vec![edit]));
    Ok(())
}

/// Collapsing a frozen axis also collapses its pane selection roles, in every
/// workbook window. Preserve the active pane's selection when roles coincide.
pub fn repair_topology(pkg: &mut Package, path: &str) -> Result<(), String> {
    let xml = pkg.text(path)?.to_string();
    let doc = parse(&xml)?;
    let Some(views) = child(doc.root_element(), "sheetViews") else {
        return Ok(());
    };
    let mut edits = vec![];
    for view in views
        .children()
        .filter(|n| n.has_tag_name((S, "sheetView")))
    {
        let Some(pane) = child(view, "pane") else {
            continue;
        };
        if !matches!(attr(pane, "state"), Some("frozen" | "frozenSplit")) {
            continue;
        }
        let rows = attr(pane, "ySplit")
            .and_then(|n| n.parse::<f64>().ok())
            .unwrap_or(0.0)
            > 0.0;
        let cols = attr(pane, "xSplit")
            .and_then(|n| n.parse::<f64>().ok())
            .unwrap_or(0.0)
            > 0.0;
        let role = |p: &str| -> &'static str {
            match (
                rows && p.starts_with("bottom"),
                cols && p.ends_with("Right"),
            ) {
                (true, true) => "bottomRight",
                (true, false) => "bottomLeft",
                (false, true) => "topRight",
                _ => "topLeft",
            }
        };
        let active = attr(pane, "activePane").unwrap_or("topLeft");
        let mut selections: Vec<_> = view
            .children()
            .filter(|n| n.has_tag_name((S, "selection")))
            .collect();
        if (rows || cols)
            && role(active) == active
            && selections.iter().all(|n| {
                let p = attr(*n, "pane").unwrap_or("topLeft");
                role(p) == p
            })
        {
            continue;
        }
        let mut contents = if rows || cols {
            attributes(&xml, pane, &[("activePane", Some(role(active).into()))])
        } else {
            String::new()
        };
        selections.sort_by_key(|n| attr(*n, "pane").unwrap_or("topLeft") != active);
        let mut roles = std::collections::BTreeSet::new();
        for selection in selections {
            let p = role(attr(selection, "pane").unwrap_or("topLeft"));
            if roles.insert(p) {
                contents.push_str(&attributes(
                    &xml,
                    selection,
                    &[("pane", (rows || cols).then(|| p.into()))],
                ));
            }
        }
        for node in view
            .children()
            .filter(|n| n.is_element() && !matches!(n.tag_name().name(), "pane" | "selection"))
        {
            contents.push_str(raw(&xml, node));
        }
        edits.push((view.range(), inner(&xml, view, &contents)));
    }
    if !edits.is_empty() {
        pkg.set(path, replace_ranges(&xml, edits));
    }
    Ok(())
}
