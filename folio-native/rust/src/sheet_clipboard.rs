//! Clipboard snapshots stay inside the process. The OS receives only selected
//! text and an opaque token, never an attached workbook or unselected cells.
use crate::{
    formula::{address, parse_address, COLS, ROWS},
    office::*,
    tables::Area,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const LIMIT: usize = 100_000;

pub struct Snapshot {
    pub token: String,
    pub workbook: Value,
    pub values: Value,
    pub errors: Value,
    pub sheet: usize,
    pub area: Area,
    pub package: Package,
    pub projection: Value,
    pub default_column_width: Option<f64>,
}
fn bounded(area: Area) -> Result<(), String> {
    if (area.bottom - area.top + 1) * (area.right - area.left + 1) > LIMIT {
        return Err("Copy or paste up to 100,000 cells at once. Nothing changed.".into());
    }
    Ok(())
}
fn sheet_index(w: &Value, r: &Value) -> Result<usize, String> {
    w["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["id"] == r["sheetId"])
        .ok_or("Worksheet no longer exists".into())
}
fn selection(r: &Value) -> Result<Area, String> {
    Area::parse(r["range"].as_str().ok_or("Select a rectangular range")?)
}
fn plain(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.into(),
        _ => v.to_string(),
    }
}
pub fn encode(rows: &[Vec<String>]) -> String {
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|s| {
                    if s.contains(['\t', '\r', '\n', '"']) || s.is_empty() {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\r\n")
}
/// RFC-style quoting with tab separators. Quotes are structural only at the
/// start of a field; embedded CR/LF and doubled quotes remain cell content.
pub fn decode(text: &str) -> Result<Vec<Vec<String>>, String> {
    if text.len() > 32_000_000 {
        return Err("Clipboard text exceeds 32 MB".into());
    }
    let mut chars = text
        .strip_prefix('\u{feff}')
        .unwrap_or(text)
        .chars()
        .peekable();
    let mut rows = vec![];
    let mut row = vec![];
    let mut field = String::new();
    let mut quoted = false;
    let mut closed = false;
    let mut start = true;
    let mut ended_row = false;
    let mut cells = 0;
    while let Some(c) = chars.next() {
        ended_row = false;
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                field.push(c);
            }
        } else if start && c == '"' {
            quoted = true;
            start = false;
        } else if c == '\t' || c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            row.push(std::mem::take(&mut field));
            cells += 1;
            if cells > LIMIT {
                return Err("Paste up to 100,000 cells at once".into());
            }
            start = true;
            closed = false;
            if c != '\t' {
                rows.push(std::mem::take(&mut row));
                ended_row = true;
            }
        } else {
            if closed {
                return Err(
                    "Clipboard text has characters after a closing quote. Nothing changed.".into(),
                );
            }
            field.push(c);
            start = false;
        }
        if field.len() > 10_000 {
            return Err("A pasted cell exceeds 10,000 bytes. Nothing changed.".into());
        }
    }
    if quoted {
        return Err("Clipboard text contains an unclosed quoted cell. Nothing changed.".into());
    }
    if !ended_row {
        row.push(field);
        rows.push(row);
    }
    if rows.is_empty() {
        rows.push(vec![String::new()]);
    }
    let width = rows.iter().map(Vec::len).max().unwrap();
    if width * rows.len() > LIMIT {
        return Err("Paste up to 100,000 cells at once".into());
    }
    for row in &mut rows {
        row.resize(width, String::new());
    }
    Ok(rows)
}
fn guard(pkg: &Package, path: &str, area: Area, writing: bool, cells: bool) -> Result<(), String> {
    let d = parse(pkg.text(path)?)?;
    if writing && child(d.root_element(), "sheetProtection").is_some() {
        return Err(
            "Pasting into protected sheets needs protection support. Nothing changed.".into(),
        );
    }
    for n in d.descendants().filter(|n| cells && n.is_element()) {
        if n.has_tag_name((S, "mergeCell"))
            || n.has_tag_name((S, "f")) && matches!(attr(n, "t"), Some("array" | "dataTable"))
        {
            if attr(n, "ref").is_some_and(|r| Area::parse(r).is_ok_and(|a| a.overlaps(area))) {
                return Err("Pasting merged cells or array/data-table formulas needs additional support. Nothing changed.".into());
            }
        }
    }
    Ok(())
}
impl Snapshot {
    pub fn new(
        w: &Value,
        values: &Value,
        errors: &Value,
        r: &Value,
        bytes: Vec<u8>,
    ) -> Result<Self, String> {
        let sheet = sheet_index(w, r)?;
        let area = selection(r)?;
        bounded(area)?;
        let package = Package::from_bytes(bytes)?;
        let projection = crate::xlsx::import(&package)?.0;
        let captured_default_width = r
            .get("sourceDefaultColumnWidth")
            .map(|v| {
                v.as_f64()
                    .filter(|n| n.is_finite() && *n > 0.0 && *n <= 255.0)
                    .ok_or("Invalid source column width")
            })
            .transpose()?;
        let default_column_width = w["sheets"][sheet]["defaultColumnWidth"]
            .as_f64()
            .or(captured_default_width);
        Ok(Self {
            token: format!(
                "folio-cells-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ),
            workbook: w.clone(),
            values: values.clone(),
            errors: errors.clone(),
            sheet,
            area,
            package,
            projection,
            default_column_width,
        })
    }
    pub fn response(&self, formatted: &Value) -> Value {
        let s = &self.workbook["sheets"][self.sheet];
        let id = s["id"].as_str().unwrap();
        let rows = (self.area.top..=self.area.bottom)
            .map(|r| {
                (self.area.left..=self.area.right)
                    .map(|c| {
                        let a = address(r, c);
                        formatted[id][&a]["text"]
                            .as_str()
                            .map(str::to_string)
                            .or_else(|| self.values[id].get(&a).map(plain))
                            .unwrap_or_else(|| {
                                s["cells"][a]
                                    .as_str()
                                    .unwrap_or("")
                                    .trim_start_matches('\'')
                                    .to_string()
                            })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        json!({"token":self.token,"text":encode(&rows),"rows":rows.len(),"columns":rows[0].len()})
    }
}

pub fn paste(
    w: &Value,
    r: &Value,
    snapshot: Option<&Snapshot>,
    mut pkg: Package,
) -> Result<(Value, Package), String> {
    let sheet = sheet_index(w, r)?;
    let requested_mode = r["mode"].as_str().unwrap_or("all");
    let merge_conditional = requested_mode == "allMergeConditional";
    let widths = matches!(requested_mode, "columnWidths" | "allColumnWidths");
    let number_formats = matches!(
        requested_mode,
        "valuesNumberFormats" | "formulasNumberFormats"
    );
    let mode = match requested_mode {
        "allMergeConditional" | "allColumnWidths" => "all",
        "valuesNumberFormats" => "values",
        "formulasNumberFormats" => "formulas",
        value => value,
    };
    if ![
        "all",
        "formulas",
        "values",
        "formats",
        "validation",
        "comments",
        "columnWidths",
    ]
    .contains(&mode)
    {
        return Err("Choose a paste option".into());
    }
    let internal = r.get("token").is_some();
    let source = if internal {
        Some(snapshot.filter(|s| r["token"] == s.token).ok_or(
            "The copied cells are no longer available. Copy them again, or use Paste text.",
        )?)
    } else {
        None
    };
    let text = if source.is_none() {
        if matches!(mode, "formats" | "validation" | "columnWidths" | "comments")
            || number_formats
            || widths
        {
            return Err(
                "Copy cells in Folio before pasting their formatting, validation or column widths"
                    .into(),
            );
        }
        Some(decode(
            r["text"]
                .as_str()
                .ok_or("There is no cell text on the clipboard")?,
        )?)
    } else {
        None
    };
    let (height, width) = source
        .map(|s| {
            (
                s.area.bottom - s.area.top + 1,
                s.area.right - s.area.left + 1,
            )
        })
        .unwrap_or_else(|| {
            let t = text.as_ref().unwrap();
            (t.len(), t[0].len())
        });
    let transpose = r["transpose"] == true;
    if widths && transpose {
        return Err("Turn off Transpose when pasting column widths. Nothing changed.".into());
    }
    let (ph, pw) = if transpose {
        (width, height)
    } else {
        (height, width)
    };
    let mut area = selection(r)?;
    if area.top == area.bottom && area.left == area.right {
        area.bottom += ph - 1;
        area.right += pw - 1;
    }
    if area.bottom >= ROWS || area.right >= COLS {
        return Err("The pasted range exceeds worksheet bounds. Nothing changed.".into());
    }
    if (mode != "columnWidths" && (area.bottom - area.top + 1) % ph != 0)
        || (area.right - area.left + 1) % pw != 0
    {
        return Err("Select one destination cell, or a range that is an exact multiple of the copied range.".into());
    }
    bounded(area)?;
    let projection = crate::xlsx::import(&pkg)?.0;
    let target_names = w["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    let path = projection[sheet]["officePath"]
        .as_str()
        .ok_or("Worksheet source missing")?;
    let metadata_only = matches!(mode, "validation" | "columnWidths" | "comments");
    guard(&pkg, path, area, true, !metadata_only)?;
    if metadata_only {
        let source = source.unwrap();
        let mut next = w.clone();
        if mode == "columnWidths" {
            copy_column_widths(source, &mut next["sheets"][sheet], area)?;
            crate::sheet_geometry::write(&mut pkg, &next["sheets"][sheet], path)?;
        } else {
            let mut copies = vec![];
            for row in area.top..=area.bottom {
                for col in area.left..=area.right {
                    let (dr, dc) = ((row - area.top) % ph, (col - area.left) % pw);
                    let (sr, sc) = if transpose { (dc, dr) } else { (dr, dc) };
                    let from = address(source.area.top + sr, source.area.left + sc);
                    if r["skipBlanks"] == true
                        && source.workbook["sheets"][source.sheet]["cells"][&from]
                            .as_str()
                            .unwrap_or("")
                            .is_empty()
                    {
                        continue;
                    }
                    copies.push((from, address(row, col)));
                }
            }
            if mode == "comments" {
                crate::sheet_reviews::copy_cells(
                    &source.workbook["sheets"][source.sheet],
                    &mut next["sheets"][sheet],
                    &copies,
                )?;
                crate::xlsx_reviews::write(
                    &mut pkg,
                    &next["sheets"][sheet],
                    Some(&projection[sheet]),
                    path,
                )?;
            } else {
                copy_validation(
                    source,
                    &projection[sheet],
                    &mut next["sheets"][sheet],
                    &copies,
                    (&target_names, sheet),
                    &mut pkg,
                    path,
                )?;
            }
        }
        pkg.original = pkg.bytes()?;
        return Ok((next, pkg));
    }
    let mut source_records = HashMap::new();
    let mut source_bases = None;
    let source_names = source
        .map(|s| {
            s.workbook["sheets"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["name"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(source) = source {
        let source_path = source.projection[source.sheet]["officePath"]
            .as_str()
            .unwrap();
        guard(&source.package, source_path, source.area, false, true)?;
        let xml = source.package.text(source_path)?;
        let d = parse(xml)?;
        for n in d.descendants().filter(|n| n.has_tag_name((S, "c"))) {
            let a = attr(n, "r").ok_or("Cell address missing")?;
            let (row, col) = parse_address(a)?;
            if source.area.contains(row, col) {
                source_records.insert(a.to_string(), scoped_record(xml, n));
            }
        }
        source_bases = Some(crate::xlsx_styles::SheetStyleBases::read(
            &source.package,
            Some(source_path),
        )?);
        if ["all", "formats"].contains(&mode) {
            // Theme/indexed-color IDs are workbook-relative. Until conversion
            // is implemented, never silently reinterpret a copied theme.
            for path in ["xl/theme/theme1.xml", "xl/metadata.xml"] {
                if source.package.entries.get(path) != pkg.entries.get(path) {
                    return Err("These workbooks use different themes or cell metadata. Use Paste values; source-format conversion needs additional support.".into());
                }
            }
        }
    }
    let target_xml = pkg.text(path)?.to_string();
    let td = parse(&target_xml)?;
    let data = child(td.root_element(), "sheetData").ok_or("Worksheet data missing")?;
    let mut records: BTreeMap<usize, BTreeMap<usize, String>> = BTreeMap::new();
    let mut original_rows = BTreeMap::new();
    for row in data.children().filter(|n| n.has_tag_name((S, "row"))) {
        let row_number = attr(row, "r")
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|r| *r > 0)
            .ok_or("Invalid worksheet row")?
            - 1;
        original_rows.insert(row_number, row);
        for c in row.children().filter(|n| n.has_tag_name((S, "c"))) {
            let (_, col) = parse_address(attr(c, "r").ok_or("Cell address missing")?)?;
            records
                .entry(row_number)
                .or_default()
                .insert(col, scoped_record(&target_xml, c));
        }
    }
    let mut next = w.clone();
    let mut edits = vec![];
    let mut style_map: HashMap<String, String> = HashMap::new();
    let target_bases = crate::xlsx_styles::SheetStyleBases::read(&pkg, Some(path))?;
    let mut number_style_map = HashMap::new();
    let mut copied_bytes = 0;
    let mut validation_copies = vec![];
    for row in area.top..=area.bottom {
        for col in area.left..=area.right {
            let (dr, dc) = ((row - area.top) % ph, (col - area.left) % pw);
            let (sr, sc) = if transpose { (dc, dr) } else { (dr, dc) };
            let to = address(row, col);
            let from = source.map(|s| address(s.area.top + sr, s.area.left + sc));
            let raw_value = source
                .map(|s| {
                    s.workbook["sheets"][s.sheet]["cells"][from.as_ref().unwrap()]
                        .as_str()
                        .unwrap_or("")
                        .to_string()
                })
                .unwrap_or_else(|| text.as_ref().unwrap()[sr][sc].clone());
            if r["skipBlanks"] == true && raw_value.is_empty() {
                continue;
            }
            if matches!(mode, "all" | "formats") {
                if let Some(from) = &from {
                    validation_copies.push((from.clone(), to.clone()));
                }
            }
            let value = if mode == "formats" {
                w["sheets"][sheet]["cells"][&to]
                    .as_str()
                    .unwrap_or("")
                    .to_string()
            } else if mode == "values" && source.is_some() && raw_value.starts_with('=') {
                let s = source.unwrap();
                let v = &s.values[s.workbook["sheets"][s.sheet]["id"].as_str().unwrap()]
                    [from.as_ref().unwrap()];
                let error = s.errors[s.workbook["sheets"][s.sheet]["id"].as_str().unwrap()]
                    [from.as_ref().unwrap()]
                    == true;
                if error {
                    plain(v)
                } else if let Some(t) = v.as_str() {
                    format!("'{t}")
                } else {
                    plain(v)
                }
            } else if mode == "values" && source.is_none() && raw_value.starts_with('=') {
                format!("'{raw_value}")
            } else if let (Some(source), Some(formula)) = (source, raw_value.strip_prefix('=')) {
                let current_names = w["sheets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s["name"].as_str().unwrap().to_string())
                    .collect::<Vec<_>>();
                if w["id"] != source.workbook["id"] || current_names != source_names {
                    return Err("The copied formulas belong to another workbook or its sheet names changed. Copy again or use Paste values.".into());
                }
                if !crate::table_refs::references(formula)?.is_empty() {
                    return Err("Pasting structured table references needs additional support. Use Paste values. Nothing changed.".into());
                }
                format!(
                    "={}",
                    crate::structure::copy_formula(
                        formula,
                        &source_names,
                        source.sheet,
                        from.as_ref().unwrap(),
                        &to
                    )?
                )
            } else {
                raw_value
            };
            let dest = records
                .get(&row)
                .and_then(|r| r.get(&col))
                .cloned()
                .unwrap_or_else(|| format!("<c xmlns=\"{S}\" r=\"{to}\"/>"));
            let mut cell = if mode == "formats" {
                dest.clone()
            } else if mode == "all" && source.is_some() {
                let s = source.unwrap();
                let cell = source_records
                    .get(from.as_ref().unwrap())
                    .cloned()
                    .unwrap_or_else(|| format!("<c xmlns=\"{S}\"/>"));
                let cell = inline_string(&s.package, &cell)?;
                let d = parse(&cell)?;
                let n = d.root_element();
                if value.starts_with('=') {
                    let base = crate::xlsx_tables::attributes(&cell, n, &[("t", None)]);
                    let extra = n
                        .children()
                        .filter(|n| {
                            n.is_element() && !matches!(n.tag_name().name(), "f" | "v" | "is")
                        })
                        .map(|n| scoped_record(&cell, n))
                        .collect::<String>();
                    crate::xlsx::fragment_inner(
                        &base,
                        &format!("<f xmlns=\"{S}\">{}</f>{extra}", esc(&value[1..])),
                    )?
                } else {
                    cell
                }
            } else {
                let generated = crate::xlsx::value_xml(&to, &value, None, &Value::Null, false);
                let gd = parse(&generated)?;
                let dd = parse(&dest)?;
                let base = crate::xlsx_tables::attributes(
                    &dest,
                    dd.root_element(),
                    &[("t", attr(gd.root_element(), "t").map(str::to_string))],
                );
                let inner = gd
                    .root_element()
                    .children()
                    .map(|n| raw(&generated, n))
                    .collect::<String>();
                let extras = dd
                    .root_element()
                    .children()
                    .filter(|n| n.is_element() && !matches!(n.tag_name().name(), "f" | "v" | "is"))
                    .map(|n| scoped_record(&dest, n))
                    .collect::<String>();
                crate::xlsx::fragment_inner(&base, &(inner + &extras))?
            };
            let mut attributes = vec![("r", Some(to.clone()))];
            if let Some(source) = source.filter(|_| ["all", "formats"].contains(&mode)) {
                let source_style = source_bases
                    .as_ref()
                    .unwrap()
                    .at(from.as_ref().unwrap())?
                    .to_string();
                let mapped = if let Some(id) = style_map.get(&source_style) {
                    id.clone()
                } else {
                    let id =
                        crate::xlsx_styles::transfer(&source.package, &mut pkg, &source_style)?;
                    style_map.insert(source_style, id.clone());
                    id
                };
                attributes.push(("s", Some(mapped)));
            } else if number_formats {
                let source = source.unwrap();
                let source_style = source_bases.as_ref().unwrap().at(from.as_ref().unwrap())?;
                let target_style = target_bases.at(&to)?;
                let key = (source_style.to_string(), target_style.to_string());
                let mapped = if let Some(id) = number_style_map.get(&key) {
                    String::clone(id)
                } else {
                    let id = crate::xlsx_styles::transfer_number_format(
                        &source.package,
                        &mut pkg,
                        source_style,
                        target_style,
                    )?;
                    number_style_map.insert(key, id.clone());
                    id
                };
                attributes.push(("s", Some(mapped)));
            }
            let d = parse(&cell)?;
            cell = crate::xlsx_tables::attributes(&cell, d.root_element(), &attributes);
            copied_bytes += cell.len();
            if copied_bytes > 32_000_000 {
                return Err(
                    "The pasted cells exceed the workspace size limit. Nothing changed.".into(),
                );
            }
            records.entry(row).or_default().insert(col, cell);
            if mode != "formats" {
                next["sheets"][sheet]["cells"][&to] = json!(value);
                edits.push(json!({"sheetId":w["sheets"][sheet]["id"],"address":to,"value":value}));
            }
        }
    }
    for r in original_rows.keys() {
        records.entry(*r).or_default();
    }
    let mut rows = String::new();
    for (r, cells) in records {
        let mut inner = cells.into_values().collect::<String>();
        if let Some(row) = original_rows.get(&r) {
            inner += &row
                .children()
                .filter(|n| n.is_element() && !n.has_tag_name((S, "c")))
                .map(|n| raw(&target_xml, n))
                .collect::<String>();
            rows += &crate::xlsx::fragment_inner(&scoped_record(&target_xml, *row), &inner)?;
        } else {
            rows += &format!("<row xmlns=\"{S}\" r=\"{}\">{inner}</row>", r + 1);
        }
    }
    pkg.set(
        path,
        replace_ranges(
            &target_xml,
            vec![(
                data.range(),
                crate::xlsx::fragment_inner(&scoped_record(&target_xml, data), &rows)?,
            )],
        ),
    );
    if !edits.is_empty() {
        crate::tables::after_cells(w, &mut next, &edits)?;
    }
    if let Some(source) = source.filter(|_| mode == "all") {
        crate::sheet_reviews::copy_cells(
            &source.workbook["sheets"][source.sheet],
            &mut next["sheets"][sheet],
            &validation_copies,
        )?;
        copy_validation(
            source,
            &projection[sheet],
            &mut next["sheets"][sheet],
            &validation_copies,
            (&target_names, sheet),
            &mut pkg,
            path,
        )?;
    }
    if let Some(source) = source.filter(|_| matches!(mode, "all" | "formats")) {
        let target_names = w["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["name"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        for key in [
            "conditionalFormats",
            "extendedConditionalFormats",
            "conditionalRightToLeft",
        ] {
            next["sheets"][sheet][key] = projection[sheet][key].clone();
        }
        crate::conditional_copy::copy_rules(
            &source.projection,
            source.sheet,
            &mut next["sheets"][sheet],
            &target_names,
            sheet,
            &validation_copies,
            merge_conditional,
        )?;
        crate::conditional_format::write(
            &mut pkg,
            &next["sheets"][sheet],
            Some(&projection[sheet]),
            path,
        )?;
    }
    if widths {
        copy_column_widths(source.unwrap(), &mut next["sheets"][sheet], area)?;
        crate::sheet_geometry::write(&mut pkg, &next["sheets"][sheet], path)?;
    }
    pkg.original = pkg.bytes()?;
    Ok((next, pkg))
}

fn copy_column_widths(source: &Snapshot, target: &mut Value, area: Area) -> Result<(), String> {
    if !target["columnWidths"].is_object() {
        target["columnWidths"] = json!({});
    }
    let source_sheet = &source.workbook["sheets"][source.sheet];
    let width = source.area.right - source.area.left + 1;
    for col in area.left..=area.right {
        let from = source.area.left + (col - area.left) % width + 1;
        let size = source_sheet["columnWidths"][from.to_string()].as_f64()
            .or(source.default_column_width)
            .ok_or("Copy these cells again in the worksheet to capture their default column width. Nothing changed.")?;
        target["columnWidths"][(col + 1).to_string()] = json!(size);
    }
    crate::sheet_geometry::validate(target)
}

fn copy_validation(
    source: &Snapshot,
    original: &Value,
    target: &mut Value,
    copies: &[(String, String)],
    context: (&[String], usize),
    pkg: &mut Package,
    path: &str,
) -> Result<(), String> {
    for key in ["dataValidations", "extendedDataValidations"] {
        if let Some(value) = original.get(key) {
            target[key] = value.clone();
        }
    }
    crate::data_validation::copy_rules(
        &source.projection,
        source.sheet,
        target,
        copies,
        Some(&source.package),
        Some(context),
    )?;
    crate::data_validation::write(pkg, target, Some(original), path)
}
fn inline_string(pkg: &Package, xml: &str) -> Result<String, String> {
    let d = parse(xml)?;
    let c = d.root_element();
    if attr(c, "t") != Some("s") {
        return Ok(xml.into());
    }
    let index = child(c, "v")
        .and_then(|n| n.text())
        .and_then(|v| v.parse::<usize>().ok())
        .ok_or("Invalid copied shared string")?;
    let strings = pkg.text("xl/sharedStrings.xml")?;
    let sd = parse(strings)?;
    let si = sd
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((S, "si")))
        .nth(index)
        .ok_or("Copied shared string missing")?;
    let inner = si
        .children()
        .filter(|n| n.is_element())
        .map(|n| scoped_record(strings, n))
        .collect::<String>();
    let extra = c
        .children()
        .filter(|n| n.is_element() && !n.has_tag_name((S, "v")))
        .map(|n| scoped_record(xml, n))
        .collect::<String>();
    let base = crate::xlsx_tables::attributes(xml, c, &[("t", Some("inlineStr".into()))]);
    crate::xlsx::fragment_inner(&base, &format!("<is xmlns=\"{S}\">{inner}</is>{extra}"))
}
