//! Shared stable row sorting; worksheet rows hidden from view stay in place.
use crate::office::{attr, child, esc, parse, replace_ranges, Package};
use crate::{
    formula::{address, parse_address},
    structure::copy_formula,
    tables::{self, Area},
};
use serde_json::{json, Value};
use std::{cmp::Ordering, collections::HashMap};

pub fn read(pkg: &Package, path: &str) -> Result<Option<Value>, String> {
    let doc = parse(pkg.text(path)?)?;
    let root = doc.root_element();
    let Some(sort) = child(root, "sortState")
        .or_else(|| child(root, "autoFilter").and_then(|f| child(f, "sortState")))
    else {
        return Ok(None);
    };
    if matches!(attr(sort, "columnSort"), Some("1" | "true")) {
        return Ok(None);
    }
    let a = Area::parse(attr(sort, "ref").ok_or("Sort range missing")?)?;
    let mut keys = vec![];
    for c in sort
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "sortCondition")
    {
        if attr(c, "customList").is_some() || attr(c, "sortBy").is_some_and(|v| v != "value") {
            return Ok(None);
        }
        let range = Area::parse(attr(c, "ref").ok_or("Sort column missing")?)?;
        if range.left != range.right || range.left < a.left || range.left > a.right {
            return Ok(None);
        }
        keys.push(json!({"column":range.left-a.left,"descending":matches!(attr(c,"descending"),Some("1"|"true"))}));
    }
    if keys.is_empty() || keys.len() > 64 {
        return Ok(None);
    }
    Ok(Some(
        json!({"range":a.text(),"headers":false,"keys":keys,"caseSensitive":matches!(attr(sort,"caseSensitive"),Some("1"|"true"))}),
    ))
}

pub fn write(pkg: &mut Package, sheet: &Value, path: &str) -> Result<(), String> {
    let Some(state) = sheet.get("rangeSort") else {
        return Ok(());
    };
    validate(sheet)?;
    let fragment = if state.is_null() {
        String::new()
    } else {
        let a = Area::parse(state["range"].as_str().unwrap())?;
        let first = a.top + usize::from(state["headers"] == true);
        let conditions = state["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| {
                let c = a.left + k["column"].as_u64().unwrap() as usize;
                format!(
                    "<sortCondition ref=\"{}:{}\" descending=\"{}\"/>",
                    address(first, c),
                    address(a.bottom, c),
                    u8::from(k["descending"] == true)
                )
            })
            .collect::<String>();
        format!("<sortState xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" ref=\"{}:{}\" caseSensitive=\"{}\">{conditions}</sortState>",esc(&address(first,a.left)),address(a.bottom,a.right),u8::from(state["caseSensitive"]==true))
    };
    let xml = pkg.text(path)?.to_string();
    let d = parse(&xml)?;
    let root = d.root_element();
    let change = if let Some(old) = child(root, "sortState") {
        (old.range(), fragment)
    } else {
        // CT_Worksheet places sortState immediately after autoFilter and before
        // dataConsolidate and all later layout/drawing children.
        let later = [
            "dataConsolidate",
            "customSheetViews",
            "mergeCells",
            "phoneticPr",
            "conditionalFormatting",
            "dataValidations",
            "hyperlinks",
            "printOptions",
            "pageMargins",
            "pageSetup",
            "headerFooter",
            "rowBreaks",
            "colBreaks",
            "customProperties",
            "cellWatches",
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
            .unwrap_or_else(|| xml.rfind("</").unwrap());
        (at..at, fragment)
    };
    pkg.set(path, replace_ranges(&xml, vec![change]));
    Ok(())
}

pub fn validate(sheet: &Value) -> Result<(), String> {
    let Some(s) = sheet.get("rangeSort").filter(|s| !s.is_null()) else {
        return Ok(());
    };
    let a = Area::parse(s["range"].as_str().ok_or("Invalid worksheet sort range")?)?;
    if !s["headers"].is_boolean() || !s["caseSensitive"].is_boolean() {
        return Err("Invalid worksheet sort options".into());
    }
    if s["headers"] == true && a.top == a.bottom {
        return Err("Worksheet sort needs a data row".into());
    }
    let keys = s["keys"]
        .as_array()
        .filter(|k| !k.is_empty() && k.len() <= 64)
        .ok_or("Invalid worksheet sort levels")?;
    if keys.iter().any(|k| {
        k["column"]
            .as_u64()
            .is_none_or(|c| c > (a.right - a.left) as u64)
            || k.get("descending").is_some_and(|v| !v.is_boolean())
    }) {
        return Err("Invalid worksheet sort column".into());
    }
    Ok(())
}

pub fn structural(
    before: &Value,
    next: &mut Value,
    change: &crate::structure::Change,
) -> Result<(), String> {
    let si = change.sheet;
    let Some(old) = before["sheets"][si]
        .get("rangeSort")
        .filter(|v| !v.is_null())
    else {
        return Ok(());
    };
    let crate::structure::Edit::Axis { rows, .. } = change.edit else {
        return Ok(());
    };
    let Some(range) = change.range(old["range"].as_str().unwrap())? else {
        next["sheets"][si]["rangeSort"] = Value::Null;
        return Ok(());
    };
    let a = Area::parse(&range)?;
    let previous = Area::parse(old["range"].as_str().unwrap())?;
    let mut updated = old.clone();
    updated["range"] = json!(range);
    if rows {
        if old["headers"] == true && change.point(previous.top as i32 + 1).is_none() {
            updated["headers"] = json!(false);
        }
    } else {
        let keys: Vec<_> = old["keys"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|key| {
                change
                    .point((previous.left + key["column"].as_u64().unwrap() as usize + 1) as i32)
                    .map(|c| {
                        let mut key = key.clone();
                        key["column"] = json!(c as usize - 1 - a.left);
                        key
                    })
            })
            .collect();
        updated["keys"] = json!(keys);
    }
    next["sheets"][si]["rangeSort"] = if updated["keys"].as_array().unwrap().is_empty()
        || (updated["headers"] == true && a.top == a.bottom)
    {
        Value::Null
    } else {
        updated
    };
    Ok(())
}

pub fn change(
    w: &Value,
    calculated: &Value,
    request: &Value,
) -> Result<(Value, Option<Vec<(usize, usize)>>), String> {
    let mut next = w.clone();
    let si = next["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["id"] == request["sheetId"])
        .ok_or("Worksheet missing")?;
    let area = Area::parse(
        request["range"]
            .as_str()
            .ok_or("Choose a range to sort")?
            .trim(),
    )?;
    if tables::list(&next["sheets"][si]).any(|t| tables::area(t).is_ok_and(|a| a.overlaps(area))) {
        return Err("This range includes an Excel table. Use Sort table for that table, or choose a range outside it.".into());
    }
    let headers = request["headers"]
        .as_bool()
        .ok_or("Choose whether the range has headers")?;
    let first = area.top + usize::from(headers);
    if first > area.bottom {
        return Err("Include at least one data row in the sort range".into());
    }
    let order = rows(
        &mut next,
        calculated,
        request,
        si,
        area,
        first,
        area.bottom + 1,
        true,
    )?;
    next["sheets"][si]["rangeSort"] = json!({"range":area.text(),"headers":headers,"keys":request["keys"],"caseSensitive":request["caseSensitive"]==true});
    Ok((next, Some(order)))
}

pub fn rows(
    next: &mut Value,
    calculated: &Value,
    r: &Value,
    index: usize,
    a: Area,
    first: usize,
    end: usize,
    skip_hidden: bool,
) -> Result<Vec<(usize, usize)>, String> {
    let id = next["sheets"][index]["id"].as_str().unwrap().to_string();
    let hidden = tables::hidden_rows(&next["sheets"][index]);
    if end.saturating_sub(first) > 100000 {
        return Err("Sort up to 100,000 rows".into());
    }
    let keys = r["keys"]
        .as_array()
        .filter(|k| !k.is_empty() && k.len() <= 64)
        .ok_or("Choose 1–64 sort levels")?;
    for k in keys {
        if k["column"]
            .as_u64()
            .is_none_or(|c| c > (a.right - a.left) as u64)
            || k.get("descending").is_some_and(|d| !d.is_boolean())
        {
            return Err("Invalid sort column or direction".into());
        }
    }
    let visible: Vec<_> = (first..end)
        .filter(|r| !skip_hidden || !hidden.contains(&(r + 1)))
        .collect();
    let mut rows = visible.clone();
    rows.sort_by(|left, right| {
        for key in keys {
            let col = a.left + key["column"].as_u64().unwrap() as usize;
            let order = tables::compare(
                &calculated[&id][address(*left, col)],
                &calculated[&id][address(*right, col)],
                key["descending"] == true,
                r["caseSensitive"] == true,
            );
            if order != Ordering::Equal {
                return order;
            }
        }
        Ordering::Equal
    });
    let map: HashMap<_, _> = rows
        .iter()
        .enumerate()
        .map(|(i, from)| (*from, visible[i]))
        .collect();
    let names: Vec<_> = next["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().into())
        .collect();
    let sheet = &mut next["sheets"][index];
    for field in [
        "cells",
        "cellStyles",
        "officeStyles",
        "cached",
        "cellReviews",
    ] {
        if let Some(cells) = sheet.get(field).and_then(Value::as_object) {
            let mut moved = cells.clone();
            for (cell, v) in cells {
                let (row, col) = parse_address(cell)?;
                if !map.contains_key(&row) || col < a.left || col > a.right {
                    continue;
                }
                moved.remove(cell);
                let _ = v;
            }
            for (cell, v) in cells {
                let (row, col) = parse_address(cell)?;
                if !map.contains_key(&row) || col < a.left || col > a.right {
                    continue;
                }
                let dest = address(map[&row], col);
                let value = if field == "cells" && v.as_str().is_some_and(|s| s.starts_with('=')) {
                    json!(format!(
                        "={}",
                        copy_formula(&v.as_str().unwrap()[1..], &names, index, cell, &dest)?
                    ))
                } else {
                    v.clone()
                };
                moved.insert(dest, value);
            }
            sheet[field] = json!(moved);
        }
    }

    Ok(map.into_iter().collect())
}
