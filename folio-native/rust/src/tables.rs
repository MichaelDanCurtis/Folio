//! Editable Excel tables, row sorting and persisted filter results.
use crate::{
    formula::{address, parse_address},
    structure::copy_formula,
    table_refs,
};
use ironcalc_base::types::{Table, TableColumn, TableStyleInfo};
use serde_json::{json, Value};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
};
#[path = "table_resize.rs"]
mod resize;
#[path = "table_totals.rs"]
mod totals;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub top: usize,
    pub left: usize,
    pub bottom: usize,
    pub right: usize,
}
impl Area {
    pub fn parse(s: &str) -> Result<Self, String> {
        let ends: Vec<_> = s.split(':').collect();
        if ends.len() > 2 {
            return Err("Choose a rectangular cell range".into());
        }
        let (top, left) = parse_address(&ends[0].replace('$', "").to_uppercase())?;
        let (bottom, right) = parse_address(&ends[ends.len() - 1].replace('$', "").to_uppercase())?;
        if top > bottom || left > right {
            return Err("The range must start at its top-left cell".into());
        }
        Ok(Self {
            top,
            left,
            bottom,
            right,
        })
    }
    pub fn text(self) -> String {
        format!(
            "{}:{}",
            address(self.top, self.left),
            address(self.bottom, self.right)
        )
    }
    pub fn contains(self, row: usize, col: usize) -> bool {
        row >= self.top && row <= self.bottom && col >= self.left && col <= self.right
    }
    pub fn overlaps(self, other: Self) -> bool {
        self.top <= other.bottom
            && self.bottom >= other.top
            && self.left <= other.right
            && self.right >= other.left
    }
}
pub fn list(sheet: &Value) -> impl Iterator<Item = &Value> {
    sheet
        .get("tables")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}
pub fn area(table: &Value) -> Result<Area, String> {
    Area::parse(table["range"].as_str().ok_or("Missing table range")?)
}
pub fn at(sheet: &Value, row: usize, col: usize) -> Option<&Value> {
    list(sheet).find(|t| area(t).is_ok_and(|a| a.contains(row, col)))
}
pub fn name_valid(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_alphabetic() || matches!(first, '_' | '\\'))
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '\\'))
        && name.encode_utf16().count() <= 255
        && parse_address(&name.to_uppercase()).is_err()
        && !matches!(name.to_uppercase().as_str(), "R" | "C" | "TRUE" | "FALSE")
        && !(name.to_uppercase().starts_with('R')
            && name[1..].split_once(['c', 'C']).is_some_and(|(r, c)| {
                !r.is_empty()
                    && !c.is_empty()
                    && r.bytes().chain(c.bytes()).all(|c| c.is_ascii_digit())
            }))
}
pub fn validate(sheets: &Value) -> Result<(), String> {
    let mut names = HashSet::new();
    let mut ids = HashSet::new();
    for sheet in sheets.as_array().ok_or("Missing worksheets")? {
        crate::sheet_geometry::validate(sheet)?;
        crate::sheet_view::validate(sheet)?;
        crate::sheet_sort::validate(sheet)?;
        crate::sheet_filter::validate(sheet)?;
        if let Some(rows) = sheet.get("hiddenRows") {
            if rows.as_array().is_none_or(|rows| {
                rows.iter()
                    .any(|r| r.as_u64().is_none_or(|r| r == 0 || r > 1048576))
            }) {
                return Err("Invalid hidden worksheet rows".into());
            }
        }
        let mut areas = vec![];
        for table in list(sheet) {
            let name = table["name"].as_str().ok_or("Missing table name")?;
            if !name_valid(name) || !names.insert(name.to_lowercase()) {
                return Err("Table names must be unique valid Excel names".into());
            }
            let id = table["id"].as_str().ok_or("Missing table id")?;
            if !ids.insert(id) {
                return Err("Duplicate table id".into());
            }
            let a = area(table)?;
            if areas.iter().any(|b| a.overlaps(*b)) {
                return Err("Tables cannot overlap".into());
            }
            areas.push(a);
            let header = table["headerRows"].as_u64().unwrap_or(1);
            let total = table["totalsRows"].as_u64().unwrap_or(0);
            if header > 1 || total > 1 || a.bottom - a.top + 1 < (header + total) as usize {
                return Err("Invalid table header or totals row".into());
            }
            let cols = table["columns"].as_array().ok_or("Missing table columns")?;
            if cols.len() != a.right - a.left + 1 {
                return Err("Table columns do not match its range".into());
            }
            let mut labels = HashSet::new();
            let mut column_ids = HashSet::new();
            for col in cols {
                if col["id"]
                    .as_u64()
                    .is_none_or(|id| id == 0 || id > u32::MAX as u64 || !column_ids.insert(id))
                {
                    return Err("Table columns need unique positive ids".into());
                }
                for field in ["formula", "totalsFormula"] {
                    if col.get(field).is_some_and(|f| {
                        f.as_str()
                            .is_none_or(|f| !f.starts_with('=') || f.len() > 10000)
                    }) {
                        return Err("Invalid table formula".into());
                    }
                }
                let label = col["name"].as_str().ok_or("Missing column name")?;
                if label.trim().is_empty()
                    || label.encode_utf16().count() > 255
                    || !labels.insert(label.to_lowercase())
                {
                    return Err(
                        "Table columns need unique, nonempty names of at most 255 characters"
                            .into(),
                    );
                }
            }
            if let Some(filters) = table.get("filters") {
                for (col, rule) in filters.as_object().ok_or("Invalid table filters")? {
                    if col.parse::<usize>().is_err() || col.parse::<usize>().unwrap() >= cols.len()
                    {
                        return Err("Invalid table filter column".into());
                    }
                    if rule["kind"] != "unsupported" {
                        validate_filter(rule)?;
                    }
                }
            }
            if table.get("filteredRows").is_some_and(|v| !v.is_array()) {
                return Err("Invalid filtered rows".into());
            }
            for row in table
                .get("filteredRows")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if row
                    .as_u64()
                    .is_none_or(|r| r <= a.top as u64 + header || r > a.bottom as u64 + 1 - total)
                {
                    return Err("Invalid filtered row".into());
                }
            }
        }
    }
    Ok(())
}

pub fn iron_tables(sheets: &[Value]) -> Result<HashMap<String, Table>, String> {
    let mut all = HashMap::new();
    for sheet in sheets {
        for t in list(sheet) {
            let name = t["name"].as_str().ok_or("Invalid table name")?.to_string();
            let table = Table {
                name: name.clone(),
                display_name: name.clone(),
                sheet_name: sheet["name"].as_str().unwrap().into(),
                reference: area(t)?.text(),
                totals_row_count: t["totalsRows"].as_u64().unwrap_or(0) as u32,
                header_row_count: t["headerRows"].as_u64().unwrap_or(1) as u32,
                header_row_dxf_id: None,
                data_dxf_id: None,
                totals_row_dxf_id: None,
                columns: t["columns"]
                    .as_array()
                    .ok_or("Missing table columns")?
                    .iter()
                    .enumerate()
                    .map(|(i, c)| TableColumn {
                        id: c["id"].as_u64().unwrap_or(i as u64 + 1) as u32,
                        name: c["name"].as_str().unwrap().into(),
                        totals_row_label: c["totalsLabel"].as_str().map(String::from),
                        totals_row_function: c["totalsFunction"].as_str().map(String::from),
                        ..Default::default()
                    })
                    .collect(),
                style_info: TableStyleInfo {
                    name: t["style"].as_str().map(String::from),
                    show_row_stripes: t["rowStripes"] != false,
                    show_column_stripes: t["columnStripes"] == true,
                    show_first_column: t["firstColumn"] == true,
                    show_last_column: t["lastColumn"] == true,
                },
                has_filters: t
                    .get("filters")
                    .and_then(Value::as_object)
                    .is_some_and(|f| !f.is_empty()),
            };
            all.insert(name, table);
        }
    }
    Ok(all)
}
pub fn filtered_rows(sheet: &Value) -> HashSet<usize> {
    list(sheet)
        .chain(sheet.get("autoFilter").filter(|v| !v.is_null()))
        .flat_map(|t| {
            t.get("filteredRows")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|r| {
            r.as_u64()
                .filter(|r| *r > 0 && *r <= 1048576)
                .map(|r| r as usize)
        })
        .collect()
}
pub fn hidden_rows(sheet: &Value) -> HashSet<usize> {
    let mut rows = filtered_rows(sheet);
    rows.extend(
        sheet
            .get("hiddenRows")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|r| {
                r.as_u64()
                    .filter(|r| *r > 0 && *r <= 1048576)
                    .map(|r| r as usize)
            }),
    );
    rows
}

fn value_text(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}
fn text_cell(s: &str) -> String {
    if s.starts_with(['=', '\''])
        || s.parse::<f64>().is_ok()
        || matches!(s.to_uppercase().as_str(), "TRUE" | "FALSE")
    {
        format!("'{s}")
    } else {
        s.into()
    }
}
fn unique_header(input: &str, used: &mut HashSet<String>, index: usize) -> String {
    let base = if input.trim().is_empty() {
        format!("Column{}", index + 1)
    } else {
        input.chars().take(240).collect()
    };
    let mut name = base.clone();
    let mut suffix = 2;
    while !used.insert(name.to_lowercase()) {
        name = format!("{base}{suffix}");
        suffix += 1;
    }
    name
}
fn table_index(sheet: &Value, id: &Value) -> Result<usize, String> {
    list(sheet)
        .position(|t| t["id"] == *id)
        .ok_or("The table no longer exists".into())
}

fn rename_columns_everywhere(
    w: &mut Value,
    name: &str,
    changes: &HashMap<String, Option<String>>,
) -> Result<(), String> {
    if changes.is_empty() {
        return Ok(());
    }
    for sheet in w["sheets"].as_array_mut().unwrap() {
        let old = sheet.clone();
        for (cell, v) in sheet["cells"].as_object_mut().unwrap() {
            if v.as_str().is_some_and(|s| s.starts_with('=')) {
                let (r, c) = parse_address(cell)?;
                let context = at(&old, r, c).and_then(|t| t["name"].as_str());
                *v = json!(table_refs::rename_columns(
                    v.as_str().unwrap(),
                    context,
                    name,
                    changes
                )?);
            }
        }
        if let Some(tables) = sheet.get_mut("tables").and_then(Value::as_array_mut) {
            for t in tables {
                let context = t["name"].as_str().unwrap().to_string();
                for col in t["columns"].as_array_mut().unwrap() {
                    for field in ["formula", "totalsFormula"] {
                        if let Some(f) = col.get(field).and_then(Value::as_str) {
                            col[field] = json!(table_refs::rename_columns(
                                f,
                                Some(&context),
                                name,
                                changes
                            )?);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
fn remove_references(w: &mut Value, name: &str) -> Result<(), String> {
    for sheet in w["sheets"].as_array_mut().unwrap() {
        for v in sheet["cells"].as_object_mut().unwrap().values_mut() {
            if v.as_str().is_some_and(|s| s.starts_with('=')) {
                *v = json!(table_refs::invalidate_table(v.as_str().unwrap(), name)?);
            }
        }
        if let Some(tables) = sheet.get_mut("tables").and_then(Value::as_array_mut) {
            for t in tables {
                for c in t["columns"].as_array_mut().unwrap() {
                    for field in ["formula", "totalsFormula"] {
                        if let Some(f) = c.get(field).and_then(Value::as_str) {
                            c[field] = json!(table_refs::invalidate_table(f, name)?);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn after_cells(before: &Value, next: &mut Value, edits: &[Value]) -> Result<(), String> {
    let names: Vec<_> = next["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect();
    for si in 0..next["sheets"].as_array().unwrap().len() {
        let id = next["sheets"][si]["id"].clone();
        let tables: Vec<_> = list(&before["sheets"][si]).cloned().collect();
        for (ti, old) in tables.iter().enumerate() {
            let mut a = area(old)?;
            let old_bottom = a.bottom;
            let mut relevant: Vec<_> = edits
                .iter()
                .filter(|e| e["sheetId"] == id)
                .filter_map(|e| e["address"].as_str())
                .map(|a| parse_address(&a.to_uppercase()))
                .collect::<Result<_, _>>()?;
            relevant.sort_unstable();
            if old["totalsRows"] != 1 {
                for (r, c) in &relevant {
                    if *r == a.bottom + 1
                        && *c >= a.left
                        && *c <= a.right
                        && next["sheets"][si]["cells"][address(*r, *c)]
                            .as_str()
                            .is_some_and(|v| !v.is_empty())
                    {
                        a.bottom = *r;
                    }
                }
            }
            next["sheets"][si]["tables"][ti]["range"] = json!(a.text());
            let mut used = HashSet::new();
            let mut renamed = HashMap::new();
            if old["headerRows"] != 0 {
                for col in a.left..=a.right {
                    let ci = col - a.left;
                    let cell = address(a.top, col);
                    let value = next["sheets"][si]["cells"][&cell].as_str().unwrap_or("");
                    let label =
                        unique_header(value.strip_prefix('\'').unwrap_or(value), &mut used, ci);
                    let old_name = old["columns"][ci]["name"].as_str().unwrap();
                    if label != old_name {
                        renamed.insert(old_name.to_lowercase(), Some(label.clone()));
                    }
                    next["sheets"][si]["tables"][ti]["columns"][ci]["name"] = json!(label);
                    next["sheets"][si]["cells"][&cell] = json!(text_cell(&label));
                }
            }
            rename_columns_everywhere(next, old["name"].as_str().unwrap(), &renamed)?;
            if old["totalsRows"] == 1 {
                for (row, col) in &relevant {
                    if *row == a.bottom && *col >= a.left && *col <= a.right {
                        let value = next["sheets"][si]["cells"][address(*row, *col)]
                            .as_str()
                            .unwrap_or("")
                            .to_string();
                        totals::capture(
                            &mut next["sheets"][si]["tables"][ti]["columns"][*col - a.left],
                            &value,
                            old["name"].as_str().unwrap(),
                        );
                    }
                }
            }
            if a.bottom > old_bottom {
                let first = a.top + old["headerRows"].as_u64().unwrap_or(1) as usize;
                for ci in 0..=a.right - a.left {
                    if let Some(f) = next["sheets"][si]["tables"][ti]["columns"][ci]["formula"]
                        .as_str()
                        .map(String::from)
                    {
                        for row in old_bottom + 1..=a.bottom {
                            let cell = address(row, a.left + ci);
                            if next["sheets"][si]["cells"][&cell]
                                .as_str()
                                .is_none_or(str::is_empty)
                            {
                                next["sheets"][si]["cells"][&cell] = json!(format!(
                                    "={}",
                                    copy_formula(
                                        f.strip_prefix('=').unwrap_or(&f),
                                        &names,
                                        si,
                                        &address(first, a.left + ci),
                                        &cell
                                    )?
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    validate(&next["sheets"])
}

pub fn structural(
    before: &Value,
    next: &mut Value,
    change: &crate::structure::Change,
    date1904: bool,
) -> Result<(), String> {
    crate::sheet_geometry::structural(next, change)?;
    crate::sheet_view::structural(next, change)?;
    crate::sheet_sort::structural(before, next, change)?;
    crate::sheet_filter::structural(before, next, change, date1904)?;
    use crate::structure::Edit;
    let si = change.sheet;
    let old_tables: Vec<_> = list(&before["sheets"][si]).cloned().collect();
    if matches!(change.edit, Edit::Delete) {
        for t in old_tables {
            remove_references(next, t["name"].as_str().unwrap())?;
        }
        return Ok(());
    }
    // Calculated-column definitions can contain A1 references to any sheet.
    for (sheet_index, sheet) in next["sheets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        if let Some(tables) = sheet.get_mut("tables").and_then(Value::as_array_mut) {
            for t in tables {
                let a = area(t)?;
                let first = a.top + t["headerRows"].as_u64().unwrap_or(1) as usize;
                for (ci, c) in t["columns"].as_array_mut().unwrap().iter_mut().enumerate() {
                    if let Some(f) = c["totalsFormula"].as_str() {
                        c["totalsFormula"] = json!(format!(
                            "={}",
                            change.formula(
                                f.trim_start_matches('='),
                                sheet_index,
                                &address(a.bottom, a.left + ci)
                            )?
                        ));
                    }
                    if let Some(f) = c["formula"].as_str() {
                        c["formula"] = json!(format!(
                            "={}",
                            change.formula(
                                f.trim_start_matches('='),
                                sheet_index,
                                &address(first, a.left + ci)
                            )?
                        ));
                    }
                }
            }
        }
    }
    let Edit::Axis {
        rows,
        insert,
        at,
        count,
    } = change.edit
    else {
        return Ok(());
    };
    if rows {
        if let Some(hidden) = next["sheets"][si]
            .get("hiddenRows")
            .and_then(Value::as_array)
        {
            let moved: Vec<_> = hidden
                .iter()
                .filter_map(Value::as_i64)
                .filter_map(|r| change.point(r as i32))
                .collect();
            next["sheets"][si]["hiddenRows"] = json!(moved);
        }
    }
    let mut updated = vec![];
    let mut removed = vec![];
    let mut renamed_all = vec![];
    let mut refilter_ids = HashSet::new();
    for (ti, old) in old_tables.iter().enumerate() {
        let a = area(old)?;
        let name = old["name"].as_str().unwrap().to_string();
        let Some(range) = change.range(old["range"].as_str().unwrap())? else {
            removed.push(name);
            continue;
        };
        if rows
            && !insert
            && old["headerRows"] != 0
            && at <= a.top as i32 + 1
            && at + count > a.top as i32 + 1
        {
            return Err("Keep the table header when deleting worksheet rows. Delete the full table range to remove it.".into());
        }
        let mut t = next["sheets"][si]["tables"][ti].clone();
        t["range"] = json!(range);
        let new_area = area(&t)?;
        if rows {
            if old["totalsRows"] == 1 && change.point(a.bottom as i32 + 1).is_none() {
                t["totalsRows"] = json!(0);
            }
            let hidden: Vec<_> = old
                .get("filteredRows")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_i64)
                .filter_map(|r| change.point(r as i32))
                .collect();
            t["filteredRows"] = json!(hidden);
            let old_first = a.top + old["headerRows"].as_u64().unwrap_or(1) as usize;
            let new_first = new_area.top + t["headerRows"].as_u64().unwrap_or(1) as usize;
            let moved_first = change
                .point(old_first as i32 + 1)
                .map(|r| r as usize - 1)
                .unwrap_or(new_first);
            let body_end = new_area.bottom + 1 - t["totalsRows"].as_u64().unwrap_or(0) as usize;
            for (ci, col) in t["columns"].as_array_mut().unwrap().iter_mut().enumerate() {
                if let Some(f) = col["formula"].as_str() {
                    let f = copy_formula(
                        f.trim_start_matches('='),
                        &change.names,
                        si,
                        &address(moved_first, a.left + ci),
                        &address(new_first, a.left + ci),
                    )?;
                    col["formula"] = json!(format!("={f}"));
                    if insert {
                        for row in (at as usize - 1).max(new_first)
                            ..(at as usize - 1 + count as usize).min(body_end)
                        {
                            next["sheets"][si]["cells"][address(row, a.left + ci)] =
                                json!(format!(
                                    "={}",
                                    copy_formula(
                                        &f,
                                        &change.names,
                                        si,
                                        &address(new_first, a.left + ci),
                                        &address(row, a.left + ci)
                                    )?
                                ));
                        }
                    }
                }
            }
        } else {
            let mut columns = BTreeMap::new();
            let mut renamed = HashMap::new();
            let mut filters = serde_json::Map::new();
            let mut sort = vec![];
            for (i, col) in t["columns"].as_array().unwrap().iter().enumerate() {
                if let Some(pos) = change.point((a.left + i + 1) as i32) {
                    let offset = pos as usize - 1 - new_area.left;
                    columns.insert(pos as usize - 1, col.clone());
                    if let Some(rule) = old["filters"].get(i.to_string()) {
                        filters.insert(offset.to_string(), rule.clone());
                    }
                } else {
                    renamed.insert(col["name"].as_str().unwrap().to_lowercase(), None);
                }
            }
            let mut ids = old["columns"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|c| c["id"].as_u64())
                .max()
                .unwrap_or(0);
            let mut used: HashSet<_> = columns
                .values()
                .filter_map(|c| c["name"].as_str())
                .map(str::to_lowercase)
                .collect();
            for col in new_area.left..=new_area.right {
                columns.entry(col).or_insert_with(|| {
                    ids += 1;
                    let label = unique_header("", &mut used, col - new_area.left);
                    if t["headerRows"] != 0 {
                        next["sheets"][si]["cells"][address(new_area.top, col)] = json!(label);
                    }
                    json!({"id":ids,"name":label})
                });
            }
            for key in old
                .get("sort")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(pos) = key["column"]
                    .as_u64()
                    .and_then(|c| change.point((a.left + c as usize + 1) as i32))
                {
                    let mut key = key.clone();
                    key["column"] = json!(pos as usize - 1 - new_area.left);
                    sort.push(key);
                }
            }
            if filters.len()
                < old
                    .get("filters")
                    .and_then(Value::as_object)
                    .map_or(0, |f| f.len())
            {
                refilter_ids.insert(old["id"].as_str().unwrap().to_string());
            }
            t["columns"] = json!(columns.into_values().collect::<Vec<_>>());
            if filters.is_empty() {
                t["filteredRows"] = json!([]);
            }
            t["filters"] = json!(filters);
            t["sort"] = json!(sort);
            renamed_all.push((name, renamed));
        }
        updated.push(t);
    }
    if next["sheets"][si].get("tables").is_some() {
        next["sheets"][si]["tables"] = json!(updated);
    }
    for name in removed {
        remove_references(next, &name)?;
    }
    for (name, renamed) in renamed_all {
        rename_columns_everywhere(next, &name, &renamed)?;
    }
    if !refilter_ids.is_empty() {
        let mut calc = crate::calculation::WorkbookCalculator::default();
        calc.sync_calendar(&next["sheets"], date1904);
        let sheet_id = next["sheets"][si]["id"].as_str().unwrap().to_string();
        if let Some(tables) = next["sheets"][si]
            .get_mut("tables")
            .and_then(Value::as_array_mut)
        {
            for t in tables {
                if !refilter_ids.contains(t["id"].as_str().unwrap()) {
                    continue;
                }
                let a = area(t)?;
                let first = a.top + t["headerRows"].as_u64().unwrap_or(1) as usize;
                let end = a.bottom + 1 - t["totalsRows"].as_u64().unwrap_or(0) as usize;
                let mut hidden = vec![];
                for row in first..end {
                    for (col, rule) in t["filters"].as_object().unwrap() {
                        let col = col.parse::<usize>().map_err(|_| "Invalid filter column")?;
                        if !matches_filter(
                            &calc.values[&sheet_id][address(row, a.left + col)],
                            rule,
                        )? {
                            hidden.push(row + 1);
                            break;
                        }
                    }
                }
                t["filteredRows"] = json!(hidden);
            }
        }
    }
    validate(&next["sheets"])
}
fn subtotal_number(function: &str) -> Result<u32, String> {
    match function {
        "average" => Ok(101),
        "countNums" => Ok(102),
        "count" => Ok(103),
        "max" => Ok(104),
        "min" => Ok(105),
        "stdDev" => Ok(107),
        "sum" => Ok(109),
        "var" => Ok(110),
        "none" => Ok(0),
        _ => Err(
            "This custom totals formula cannot be toggled yet. Its original cells have been kept."
                .into(),
        ),
    }
}
#[cfg(test)]
pub fn change(
    w: &Value,
    calculated: &Value,
    r: &Value,
) -> Result<(Value, Option<Vec<(usize, usize)>>), String> {
    change_calendar(w, calculated, r, false)
}
pub fn change_calendar(
    w: &Value,
    calculated: &Value,
    r: &Value,
    date1904: bool,
) -> Result<(Value, Option<Vec<(usize, usize)>>), String> {
    let mut next = w.clone();
    let index = next["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["id"] == r["sheetId"])
        .ok_or("Worksheet missing")?;
    let id = w["sheets"][index]["id"].as_str().unwrap();
    let action = r["action"].as_str().ok_or("Missing table action")?;
    let mut order = None;
    if action == "create" {
        let a = Area::parse(r["range"].as_str().ok_or("Select a range for the table")?)?;
        if a.bottom <= a.top {
            return Err("Select a header and at least one data row".into());
        }
        if a.bottom - a.top > 100000 {
            return Err("Tables currently support up to 100,000 data rows".into());
        }
        if list(&next["sheets"][index]).any(|t| area(t).is_ok_and(|b| a.overlaps(b))) {
            return Err("The selected range overlaps another table".into());
        }
        let used: HashSet<_> = next["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(list)
            .filter_map(|t| t["name"].as_str())
            .map(str::to_lowercase)
            .collect();
        let name = r["name"].as_str().map(String::from).unwrap_or_else(|| {
            (1..)
                .map(|i| format!("Table{i}"))
                .find(|name| !used.contains(&name.to_lowercase()))
                .unwrap()
        });
        if !name_valid(&name) || used.contains(&name.to_lowercase()) {
            return Err("Choose a unique table name without spaces or cell-address syntax".into());
        }
        let sheet = &mut next["sheets"][index];
        let mut labels = HashSet::new();
        let mut cols = vec![];
        for col in a.left..=a.right {
            let cell = address(a.top, col);
            let raw = sheet["cells"][&cell].as_str().unwrap_or("");
            let input = if raw.starts_with('=') {
                value_text(&calculated[id][&cell])
            } else {
                raw.strip_prefix('\'').unwrap_or(raw).into()
            };
            let label = unique_header(&input, &mut labels, col - a.left);
            sheet["cells"][&cell] = json!(text_cell(&label));
            cols.push(json!({"id":col-a.left+1,"name":label}));
        }
        if !sheet.get("tables").is_some_and(Value::is_array) {
            sheet["tables"] = json!([]);
        }
        let table_id = format!(
            "table-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        sheet["tables"].as_array_mut().unwrap().push(json!({"id":table_id,"name":name,"range":a.text(),"headerRows":1,"totalsRows":0,"columns":cols,"style":"TableStyleMedium2","rowStripes":true,"filters":{},"filteredRows":[]}));
    } else {
        let ti = table_index(&next["sheets"][index], &r["tableId"])?;
        let table = next["sheets"][index]["tables"][ti].clone();
        let a = area(&table)?;
        let first = a.top + table["headerRows"].as_u64().unwrap_or(1) as usize;
        let end = a.bottom + 1 - table["totalsRows"].as_u64().unwrap_or(0) as usize;
        match action {
            "resize" => {
                order = resize::apply(&mut next, calculated, index, ti, &r["range"], date1904)?;
            }
            "totals" | "totalFunction" => {
                totals::change(&mut next, index, ti, r)?;
            }
            "rename" => {
                let name = r["name"].as_str().ok_or("Enter a table name")?;
                if !name_valid(name)
                    || next["sheets"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .flat_map(list)
                        .any(|t| {
                            t["id"] != table["id"]
                                && t["name"]
                                    .as_str()
                                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
                        })
                {
                    return Err("Choose a unique valid table name".into());
                }
                for sheet in next["sheets"].as_array_mut().unwrap() {
                    for value in sheet["cells"].as_object_mut().unwrap().values_mut() {
                        if value.as_str().is_some_and(|v| v.starts_with('=')) {
                            *value = json!(table_refs::rename_table(
                                value.as_str().unwrap(),
                                table["name"].as_str().unwrap(),
                                name
                            )?);
                        }
                    }
                    if let Some(tables) = sheet.get_mut("tables").and_then(Value::as_array_mut) {
                        for t in tables {
                            for col in t["columns"].as_array_mut().unwrap() {
                                for field in ["formula", "totalsFormula"] {
                                    if let Some(f) = col.get(field).and_then(Value::as_str) {
                                        col[field] = json!(table_refs::rename_table(
                                            f,
                                            table["name"].as_str().unwrap(),
                                            name
                                        )?);
                                    }
                                }
                            }
                        }
                    }
                }
                next["sheets"][index]["tables"][ti]["name"] = json!(name);
            }
            "style" => {
                let style = r["style"].as_str().ok_or("Choose a table style")?;
                if ![
                    "TableStyleMedium2",
                    "TableStyleMedium4",
                    "TableStyleMedium9",
                    "TableStyleLight1",
                ]
                .contains(&style)
                {
                    return Err("Unsupported table style".into());
                }
                next["sheets"][index]["tables"][ti]["style"] = json!(style);
            }
            "columnFormula" => {
                let col = r["column"].as_u64().ok_or("Choose a table column")? as usize;
                if col > a.right - a.left {
                    return Err("Table column is out of range".into());
                }
                let formula = r["formula"]
                    .as_str()
                    .filter(|s| s.starts_with('='))
                    .ok_or("Enter a formula beginning with =")?;
                let names: Vec<_> = next["sheets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s["name"].as_str().unwrap().into())
                    .collect();
                let base = address(first, a.left + col);
                for row in first..end {
                    next["sheets"][index]["cells"][address(row, a.left + col)] = json!(format!(
                        "={}",
                        copy_formula(
                            &formula[1..],
                            &names,
                            index,
                            &base,
                            &address(row, a.left + col)
                        )?
                    ));
                }
                next["sheets"][index]["tables"][ti]["columns"][col]["formula"] = json!(formula);
            }
            "filter" | "clearFilters" => {
                let mut filters = table.get("filters").cloned().unwrap_or(json!({}));
                if action == "clearFilters" {
                    filters = json!({});
                } else {
                    let col = r["column"].as_u64().ok_or("Choose a filter column")? as usize;
                    if col > a.right - a.left {
                        return Err("Filter column is out of range".into());
                    }
                    if r["rule"].is_null() {
                        filters
                            .as_object_mut()
                            .ok_or("Invalid filters")?
                            .remove(&col.to_string());
                    } else {
                        validate_filter(&r["rule"])?;
                        filters[col.to_string()] = r["rule"].clone();
                    }
                }
                let mut hidden = vec![];
                for row in first..end {
                    let mut visible = true;
                    for (column, rule) in filters.as_object().ok_or("Invalid filters")? {
                        let col = column
                            .parse::<usize>()
                            .map_err(|_| "Invalid filter column")?;
                        if !matches_filter(&calculated[id][address(row, a.left + col)], rule)? {
                            visible = false;
                            break;
                        }
                    }
                    if !visible {
                        hidden.push(row + 1);
                    }
                }
                next["sheets"][index]["tables"][ti]["filters"] = filters;
                next["sheets"][index]["tables"][ti]["filteredRows"] = json!(hidden);
            }
            "sort" => {
                let order_rows =
                    crate::sheet_sort::rows(&mut next, calculated, r, index, a, first, end, false)?;
                let map: HashMap<_, _> = order_rows.iter().copied().collect();
                let sheet = &mut next["sheets"][index];
                // Filter results are tied to records, so carry their visibility
                // through the same stable row permutation.
                let hidden: Vec<_> = table
                    .get("filteredRows")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_u64())
                    .map(|r| {
                        map.get(&(r as usize - 1))
                            .copied()
                            .unwrap_or(r as usize - 1)
                            + 1
                    })
                    .collect();
                sheet["tables"][ti]["filteredRows"] = json!(hidden);
                sheet["tables"][ti]["sort"] = r["keys"].clone();
                sheet["tables"][ti]["sortCaseSensitive"] = json!(r["caseSensitive"] == true);
                order = Some(order_rows);
            }
            _ => return Err("Unknown table action".into()),
        }
    }
    validate(&next["sheets"])?;
    Ok((next, order))
}

pub(crate) fn validate_filter(rule: &Value) -> Result<(), String> {
    match rule["kind"].as_str() {
        Some("values") => {
            let values = rule["values"].as_array().ok_or("Choose filter values")?;
            if values.len() > 10000
                || values
                    .iter()
                    .any(|v| v.as_str().is_none_or(|s| s.len() > 10000))
            {
                return Err("Invalid filter values".into());
            }
        }
        Some("custom") => {
            let tests = rule["conditions"]
                .as_array()
                .filter(|a| !a.is_empty() && a.len() <= 2)
                .ok_or("Choose one or two conditions")?;
            if !matches!(rule["join"].as_str(), Some("and" | "or")) {
                return Err("Choose and/or for conditions".into());
            }
            for t in tests {
                if !matches!(
                    t["op"].as_str(),
                    Some(
                        "equal"
                            | "notEqual"
                            | "lessThan"
                            | "lessThanOrEqual"
                            | "greaterThan"
                            | "greaterThanOrEqual"
                            | "contains"
                            | "beginsWith"
                            | "endsWith"
                    )
                ) || t["value"].as_str().is_none_or(|s| s.len() > 10000)
                {
                    return Err("Invalid filter condition".into());
                }
            }
        }
        _ => return Err("Unsupported filter kind".into()),
    }
    Ok(())
}
fn wildcard_matches(text: &str, pattern: &str) -> bool {
    #[derive(Clone, Copy)]
    enum Token {
        Literal(char),
        One,
        Many,
    }
    let mut tokens = vec![];
    let lower = pattern.to_lowercase();
    let mut chars = lower.chars();
    while let Some(c) = chars.next() {
        tokens.push(match c {
            '~' => Token::Literal(chars.next().unwrap_or('~')),
            '*' => Token::Many,
            '?' => Token::One,
            c => Token::Literal(c),
        });
    }
    // Greedy glob matcher with one remembered star: no exponential backtracking.
    let input: Vec<_> = text.to_lowercase().chars().collect();
    let (mut i, mut j) = (0, 0);
    let mut star = None;
    while i < input.len() {
        if j < tokens.len() && matches!(tokens[j], Token::One) {
            i += 1;
            j += 1;
        } else if j < tokens.len() && matches!(tokens[j],Token::Literal(c) if c==input[i]) {
            i += 1;
            j += 1;
        } else if j < tokens.len() && matches!(tokens[j], Token::Many) {
            star = Some((j, i));
            j += 1;
        } else if let Some((at, matched)) = star {
            let next = matched + 1;
            i = next;
            j = at + 1;
            star = Some((at, next));
        } else {
            return false;
        }
    }
    while j < tokens.len() && matches!(tokens[j], Token::Many) {
        j += 1;
    }
    j == tokens.len()
}
pub fn matches_filter(value: &Value, rule: &Value) -> Result<bool, String> {
    validate_filter(rule)?;
    let text = value_text(value);
    if rule["kind"] == "values" {
        return Ok(if text.is_empty() {
            rule["blank"] == true
        } else {
            rule["values"].as_array().unwrap().iter().any(|v| {
                if let (Some(n), Ok(expected)) =
                    (value.as_f64(), v.as_str().unwrap().parse::<f64>())
                {
                    n == expected
                } else {
                    v.as_str().unwrap().to_lowercase() == text.to_lowercase()
                }
            })
        });
    }
    let mut results = vec![];
    for condition in rule["conditions"].as_array().unwrap() {
        let expected = condition["value"].as_str().unwrap();
        let order = match (value.as_f64(), expected.parse::<f64>().ok()) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            _ => text.to_lowercase().cmp(&expected.to_lowercase()),
        };
        results.push(match condition["op"].as_str().unwrap() {
            "equal" => {
                if value.is_number() {
                    order == Ordering::Equal
                } else {
                    wildcard_matches(&text, expected)
                }
            }
            "notEqual" => {
                if value.is_number() {
                    order != Ordering::Equal
                } else {
                    !wildcard_matches(&text, expected)
                }
            }
            "lessThan" => order == Ordering::Less,
            "lessThanOrEqual" => order != Ordering::Greater,
            "greaterThan" => order == Ordering::Greater,
            "greaterThanOrEqual" => order != Ordering::Less,
            "contains" => text.to_lowercase().contains(&expected.to_lowercase()),
            "beginsWith" => text.to_lowercase().starts_with(&expected.to_lowercase()),
            "endsWith" => text.to_lowercase().ends_with(&expected.to_lowercase()),
            _ => unreachable!(),
        });
    }
    Ok(if rule["join"] == "or" {
        results.into_iter().any(|b| b)
    } else {
        results.into_iter().all(|b| b)
    })
}
pub(crate) fn compare(a: &Value, b: &Value, descending: bool, case: bool) -> Ordering {
    let blank = |v: &Value| v.is_null() || v.as_str() == Some("");
    if blank(a) || blank(b) {
        return blank(a).cmp(&blank(b));
    }
    let kind = |v: &Value| {
        if v.is_number() {
            0
        } else if v.is_string() {
            1
        } else {
            2
        }
    };
    let order = match (a, b) {
        (Value::Number(a), Value::Number(b)) => a
            .as_f64()
            .unwrap_or(0.0)
            .total_cmp(&b.as_f64().unwrap_or(0.0)),
        (Value::String(a), Value::String(b)) => {
            if case {
                a.cmp(b)
            } else {
                a.to_lowercase().cmp(&b.to_lowercase())
            }
        }
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        _ => kind(a).cmp(&kind(b)),
    };
    if descending {
        order.reverse()
    } else {
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculation::WorkbookCalculator;
    fn setup() -> (Value, WorkbookCalculator) {
        let mut w = crate::workspace::seed();
        w["sheets"] = json!([{"id":"s","name":"Orders","cells":{"A1":"Product","B1":"Qty","C1":"Price","D1":"Amount","A2":"Pear","B2":"3","C2":"5","A3":"Apple","B3":"2","C3":"4","A4":"Pear","B4":"1","C4":"8","F1":"=SUM(Sales[Amount])","F2":"=SUBTOTAL(109,Sales[Amount])"}}]);
        let mut calc = WorkbookCalculator::default();
        calc.sync(&w["sheets"]);
        w = change(
            &w,
            &calc.values,
            &json!({"action":"create","sheetId":"s","range":"A1:D4","name":"Sales"}),
        )
        .unwrap()
        .0;
        let id = w["sheets"][0]["tables"][0]["id"].clone();
        calc.sync(&w["sheets"]);
        w=change(&w,&calc.values,&json!({"action":"columnFormula","sheetId":"s","tableId":id,"column":3,"formula":"=[@Qty]*[@Price]"})).unwrap().0;
        calc.sync(&w["sheets"]);
        (w, calc)
    }
    #[test]
    fn table_columns_calculate_and_filters_change_subtotals() {
        let (mut w, mut calc) = setup();
        assert_eq!(calc.error, None);
        assert_eq!(calc.values["s"]["D2"], 15.0);
        assert_eq!(calc.values["s"]["F1"], 31.0);
        assert_eq!(calc.values["s"]["F2"], 31.0);
        let id = w["sheets"][0]["tables"][0]["id"].clone();
        w=change(&w,&calc.values,&json!({"action":"filter","sheetId":"s","tableId":id,"column":0,"rule":{"kind":"values","values":["Pear"],"blank":false}})).unwrap().0;
        calc.sync(&w["sheets"]);
        assert_eq!(w["sheets"][0]["tables"][0]["filteredRows"], json!([3]));
        assert_eq!(calc.values["s"]["F1"], 31.0);
        assert_eq!(calc.values["s"]["F2"], 23.0);
        w = change(
            &w,
            &calc.values,
            &json!({"action":"rename","sheetId":"s","tableId":id,"name":"Invoices"}),
        )
        .unwrap()
        .0;
        assert_eq!(w["sheets"][0]["cells"]["F1"], "=SUM(Invoices[Amount])");
        calc.sync(&w["sheets"]);
        assert_eq!(calc.values["s"]["F2"], 23.0);
    }
    #[test]
    fn stable_multi_key_sort_carries_formulas_styles_and_visibility() {
        let (mut w, mut calc) = setup();
        w["sheets"][0]["cellStyles"] = json!({"A2":{"bold":true}});
        let id = w["sheets"][0]["tables"][0]["id"].clone();
        w["sheets"][0]["tables"][0]["filteredRows"] = json!([3]);
        let (n,map)=change(&w,&calc.values,&json!({"action":"sort","sheetId":"s","tableId":id,"keys":[{"column":0,"descending":false},{"column":1,"descending":false}]})).unwrap();
        assert_eq!(n["sheets"][0]["cells"]["A2"], "Apple");
        assert_eq!(n["sheets"][0]["cells"]["B3"], "1");
        assert_eq!(n["sheets"][0]["cells"]["B4"], "3");
        assert_eq!(n["sheets"][0]["cellStyles"]["A4"]["bold"], true);
        assert_eq!(n["sheets"][0]["tables"][0]["filteredRows"], json!([2]));
        assert_eq!(map.unwrap().len(), 3);
        assert_eq!(n["sheets"][0]["cells"]["D4"], "=[@Qty]*[@Price]");
        calc.sync(&n["sheets"]);
        assert_eq!(calc.values["s"]["D4"], 15.0);
        assert_eq!(calc.values["s"]["F1"], 31.0);
    }
    #[test]
    fn invalid_tables_and_sort_requests_leave_the_original_intact() {
        let (w, calc) = setup();
        let id = w["sheets"][0]["tables"][0]["id"].clone();
        for request in [
            json!({"action":"create","range":"A2:B5","name":"Overlap"}),
            json!({"action":"rename","name":"A1"}),
            json!({"action":"rename","name":"has spaces"}),
            json!({"action":"sort","keys":[{"column":10}]}),
            json!({"action":"filter","column":1,"rule":{"kind":"oops"}}),
        ] {
            let mut r = request;
            r["tableId"] = id.clone();
            r["sheetId"] = json!("s");
            assert!(change(&w, &calc.values, &r).is_err());
        }
        assert_eq!(w["sheets"][0]["cells"]["A2"], "Pear");
        assert_eq!(
            compare(&json!(""), &json!(8), true, false),
            Ordering::Greater
        );
        assert_eq!(
            compare(&json!("apple"), &json!("Apple"), false, false),
            Ordering::Equal
        );
    }
}
