//! Table boundaries change independently of worksheet row/column structure.
use super::*;

pub(super) fn apply(
    next: &mut Value,
    calculated: &Value,
    si: usize,
    ti: usize,
    range: &Value,
    date1904: bool,
) -> Result<Option<Vec<(usize, usize)>>, String> {
    let table = next["sheets"][si]["tables"][ti].clone();
    let old = area(&table)?;
    let new = Area::parse(range.as_str().ok_or("Enter the new table range")?.trim())?;
    let header = table["headerRows"].as_u64().unwrap_or(1) as usize;
    let totals = table["totalsRows"].as_u64().unwrap_or(0) as usize;
    if new.top != old.top || !new.overlaps(old) {
        return Err("Keep the header in the same row and overlap the existing table".into());
    }
    if new.bottom - new.top + 1 <= header + totals {
        return Err(
            "Include at least one data row, plus the header and totals row if shown".into(),
        );
    }
    if new.bottom - new.top + 1 - header - totals > 100000 {
        return Err("Tables currently support up to 100,000 data rows".into());
    }
    if list(&next["sheets"][si])
        .any(|t| t["id"] != table["id"] && area(t).is_ok_and(|a| a.overlaps(new)))
    {
        return Err("The resized table would overlap another table".into());
    }
    let first = new.top + header;
    let end = new.bottom + 1 - totals;
    let names: Vec<String> = next["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().into())
        .collect();
    let sheet_id = next["sheets"][si]["id"].as_str().unwrap().to_string();
    let moving_totals = totals == 1 && new.bottom != old.bottom;
    let left = old.left.max(new.left);
    let right = old.right.min(new.right);
    if moving_totals
        && (left..=right).any(|c| {
            next["sheets"][si]["cells"][address(new.bottom, c)]
                .as_str()
                .is_some_and(|v| !v.is_empty())
        })
    {
        return Err("The new totals row contains data. Choose an empty totals row or hide totals before resizing. Nothing changed.".into());
    }

    // Cells excluded from the table remain ordinary worksheet cells. Resolve
    // their local references before removing their implicit table context.
    for (cell, value) in next["sheets"][si]["cells"].as_object_mut().unwrap() {
        let (r, c) = parse_address(cell)?;
        if old.contains(r, c)
            && !new.contains(r, c)
            && !(moving_totals && r == old.bottom && c >= left && c <= right)
        {
            if let Some(f) = value.as_str().filter(|f| f.starts_with('=')) {
                *value = json!(table_refs::materialize_local(f, &table, r)?);
            }
        }
    }

    let mut columns = vec![];
    let mut used: HashSet<String> = table["columns"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(ci, _)| old.left + ci >= new.left && old.left + ci <= new.right)
        .map(|(_, c)| c["name"].as_str().unwrap().to_lowercase())
        .collect();
    let mut max_id = table["columns"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["id"].as_u64())
        .max()
        .unwrap_or(0);
    let mut removed = HashMap::new();
    for (ci, column) in table["columns"].as_array().unwrap().iter().enumerate() {
        if old.left + ci < new.left || old.left + ci > new.right {
            removed.insert(column["name"].as_str().unwrap().to_lowercase(), None);
        }
    }
    for c in new.left..=new.right {
        if c >= old.left && c <= old.right {
            columns.push(table["columns"][c - old.left].clone());
        } else {
            let cell = address(new.top, c);
            let raw = next["sheets"][si]["cells"][&cell].as_str().unwrap_or("");
            let input = if header == 0 {
                String::new()
            } else if raw.starts_with('=') {
                value_text(&calculated[&sheet_id][&cell])
            } else {
                raw.strip_prefix('\'').unwrap_or(raw).into()
            };
            let label = unique_header(&input, &mut used, c - new.left);
            max_id += 1;
            columns.push(json!({"id":max_id,"name":label}));
            if header != 0 {
                next["sheets"][si]["cells"][&cell] = json!(text_cell(&label));
            }
        }
    }
    let offset = |ci: usize| -> Option<usize> {
        let c = old.left + ci;
        (c >= new.left && c <= new.right).then(|| c - new.left)
    };
    let mut filters = serde_json::Map::new();
    for (key, value) in table["filters"]
        .as_object()
        .ok_or("Invalid table filters")?
    {
        if let Some(ci) = offset(key.parse().map_err(|_| "Invalid filter column")?) {
            filters.insert(ci.to_string(), value.clone());
        }
    }
    let sort: Vec<_> = table
        .get("sort")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|key| {
            offset(key["column"].as_u64()? as usize).map(|ci| {
                let mut key = key.clone();
                key["column"] = json!(ci);
                key
            })
        })
        .collect();
    let t = &mut next["sheets"][si]["tables"][ti];
    t["range"] = json!(new.text());
    t["columns"] = json!(columns);
    t["filters"] = json!(filters);
    if table.get("sort").is_some() {
        t["sort"] = json!(sort);
    }
    t["filteredRows"] = json!([]);

    if moving_totals {
        // Swap only the retained table columns with the empty destination.
        // Formatting and package-bound annotations follow the totals cells.
        for field in ["cells", "cellStyles", "officeStyles", "cached"] {
            if let Some(cells) = next["sheets"][si]
                .get_mut(field)
                .and_then(Value::as_object_mut)
            {
                for c in left..=right {
                    let from = address(old.bottom, c);
                    let to = address(new.bottom, c);
                    let source = cells.remove(&from);
                    let destination = cells.remove(&to);
                    if let Some(value) = source {
                        // This relocates the totals cell, rather than filling a
                        // formula into another record. Keep its A1 operands;
                        // structured references use the new table boundaries.
                        cells.insert(to, value);
                    }
                    if let Some(value) = destination {
                        cells.insert(from, value);
                    }
                }
            }
        }
    }
    rename_columns_everywhere(next, table["name"].as_str().unwrap(), &removed)?;

    // Keep explicit values and formula exceptions in newly included rows.
    // Empty cells inherit their column's formula, copied from the first row.
    for c in new.left..=new.right {
        if let Some(f) = next["sheets"][si]["tables"][ti]["columns"][c - new.left]["formula"]
            .as_str()
            .map(String::from)
        {
            for r in first..end {
                let was_body = old.contains(r, c) && r < old.bottom + 1 - totals;
                if !was_body
                    && next["sheets"][si]["cells"][address(r, c)]
                        .as_str()
                        .is_none_or(str::is_empty)
                {
                    next["sheets"][si]["cells"][address(r, c)] = json!(format!(
                        "={}",
                        copy_formula(
                            f.trim_start_matches('='),
                            &names,
                            si,
                            &address(first, c),
                            &address(r, c)
                        )?
                    ));
                }
            }
        }
    }
    validate(&next["sheets"])?;
    let mut calculator = crate::calculation::WorkbookCalculator::default();
    calculator.sync_calendar(&next["sheets"], date1904);
    if let Some(error) = calculator.error {
        return Err(error);
    }
    let filters = next["sheets"][si]["tables"][ti]["filters"]
        .as_object()
        .unwrap();
    let mut hidden = vec![];
    for r in first..end {
        for (column, rule) in filters {
            let c = new.left
                + column
                    .parse::<usize>()
                    .map_err(|_| "Invalid filter column")?;
            if !matches_filter(&calculator.values[&sheet_id][address(r, c)], rule)? {
                hidden.push(r + 1);
                break;
            }
        }
    }
    next["sheets"][si]["tables"][ti]["filteredRows"] = json!(hidden);
    Ok(moving_totals.then(|| vec![(old.bottom, new.bottom), (new.bottom, old.bottom)]))
}
