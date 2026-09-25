use crate::{
    formula::{address, parse_address},
    office::*,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub(crate) fn resolve(target: &str) -> Result<String, String> {
    let p = if target.starts_with('/') {
        target.trim_start_matches('/').to_string()
    } else {
        format!("xl/{target}")
    };
    let mut parts = vec![];
    for part in p.split('/') {
        match part {
            ".." => {
                parts.pop().ok_or("Invalid workbook relationship")?;
            }
            "." => {}
            _ => parts.push(part),
        }
    }
    Ok(parts.join("/"))
}
/// The workbook's defined names as the formula parser takes them: name,
/// sheet index for a sheet-scoped name, and formula.
pub(crate) fn defined_names(wb: &roxmltree::Document) -> Vec<(String, Option<u32>, String)> {
    wb.descendants()
        .filter(|n| n.has_tag_name((S, "definedName")))
        .filter_map(|n| {
            Some((
                attr(n, "name")?.to_string(),
                attr(n, "localSheetId").and_then(|i| i.parse().ok()),
                n.text().unwrap_or("").trim().to_string(),
            ))
        })
        .collect()
}
pub fn import(pkg: &Package) -> Result<(Value, Value), String> {
    let input_parser = ironcalc_base::Model::new_empty("Import", "en", "UTC", "en")?;
    let wb = parse(pkg.text("xl/workbook.xml")?)?;
    let rel = parse(pkg.text("xl/_rels/workbook.xml.rels")?)?;
    let relations: HashMap<_, _> = rel
        .descendants()
        .filter(|n| n.has_tag_name("Relationship"))
        .filter_map(|n| Some((attr(n, "Id")?, attr(n, "Target")?)))
        .collect();
    let strings = if let Ok(xml) = pkg.text("xl/sharedStrings.xml") {
        let d = parse(xml)?;
        d.descendants()
            .filter(|n| n.has_tag_name((S, "si")))
            .map(crate::xlsx_styles::string_content)
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    let styles_catalog = crate::xlsx_styles::read(pkg);
    let normal_font = crate::xlsx_styles::normal_font(pkg);
    let mut sheets = vec![];
    let names: Vec<String> = wb
        .descendants()
        .filter(|n| n.has_tag_name((S, "sheet")))
        .map(|n| attr(n, "name").unwrap_or("Sheet").into())
        .collect();
    let defined = defined_names(&wb);
    let mut warnings=vec!["Charts, pivot tables, macros and other advanced objects are not editable in Folio. Unmodified package parts are retained on export.".to_string()];
    if crate::number_format::settings(Some(pkg)).0 {
        warnings.push(DATE1904_NOTICE.into());
    }
    for sh in wb.descendants().filter(|n| n.has_tag_name((S, "sheet"))) {
        let rid = attr(sh, "id").ok_or("Sheet relationship missing")?;
        let path = resolve(relations.get(rid).ok_or("Sheet target missing")?)?;
        let xml = pkg.text(&path)?;
        let d = parse(xml)?;
        let shared = shared_formulas(&d);
        let inherited_styles = crate::xlsx_styles::SheetStyleBases::read(pkg, Some(&path))?;
        let mut cells = serde_json::Map::new();
        let mut cached = serde_json::Map::new();
        let mut styles = serde_json::Map::new();
        let mut cell_styles = serde_json::Map::new();
        // Array (CSE) formulas: the anchor holds the formula; the other cells
        // of its range hold its values, so they aren't inputs.
        let mut arrays = serde_json::Map::new();
        let mut array_areas: Vec<(crate::tables::Area, String)> = vec![];
        for c in d.descendants().filter(|n| n.has_tag_name((S, "c"))) {
            let a = attr(c, "r").ok_or("Cell address missing")?;
            let (row, column) = parse_address(a)?;
            if let Some(range) = child(c, "f")
                .filter(|f| attr(*f, "t") == Some("array"))
                .and_then(|f| attr(f, "ref"))
            {
                if let Ok(area) = crate::tables::Area::parse(range) {
                    arrays.insert(a.into(), json!(area.text()));
                    array_areas.push((area, a.to_string()));
                }
            }
            let follower = array_areas.iter().any(|(area, anchor)| {
                anchor != a
                    && (area.top..=area.bottom).contains(&row)
                    && (area.left..=area.right).contains(&column)
            });
            let value = child(c, "v").and_then(|n| n.text()).unwrap_or("");
            let t = attr(c, "t").unwrap_or("");
            let content = if child(c, "f").is_some() {
                None
            } else if t == "s" {
                Some(
                    strings
                        .get(
                            value
                                .parse::<usize>()
                                .map_err(|_| "Invalid shared string")?,
                        )
                        .ok_or("Shared string missing")?
                        .clone(),
                )
            } else if t == "inlineStr" {
                child(c, "is").map(crate::xlsx_styles::string_content)
            } else {
                None
            };
            let mut raw = if let Some(f) = child(c, "f") {
                if attr(f, "t") == Some("shared") {
                    let (origin, expr) = shared
                        .get(attr(f, "si").ok_or("Shared formula index missing")?)
                        .ok_or("Shared formula master missing")?;
                    format!(
                        "={}",
                        crate::structure::copy_formula(expr, &names, sheets.len(), origin, a)?
                    )
                } else if let Some(expr) = f.text() {
                    format!("={expr}")
                } else {
                    warnings.push(format!("{a}: shared or array formula is retained in the source file but displayed from its cached value."));
                    value.to_string()
                }
            } else {
                match t {
                    "s" | "inlineStr" => content.as_ref().map(|v| v.0.clone()).unwrap_or_default(),
                    "str" => crate::xlsx_strings::decode(value),
                    "b" => {
                        if value == "1" {
                            "TRUE".into()
                        } else {
                            "FALSE".into()
                        }
                    }
                    _ => value.into(),
                }
            };
            // A formula saved without dynamic-array metadata follows Excel's
            // legacy rules: ranges passed where one value is expected are
            // intersected with the formula's row or column. Show and
            // evaluate it with Excel 365's automatic `@` (X08.14). Array
            // (CSE) formulas and dynamic formulas (`cm`) keep their text.
            if let Some(f) = child(c, "f") {
                if raw.starts_with('=') && attr(f, "t") != Some("array") && attr(f, "cm").is_none()
                {
                    raw = crate::structure::legacy_formula(&raw, &names, sheets.len(), a, &defined);
                }
            }
            if child(c, "f").is_none()
                && matches!(t, "s" | "inlineStr" | "str")
                && (input_parser.input_parses_as_number(&raw)
                    || input_is_error(&raw)
                    || raw.starts_with(['=', '\''])
                    || matches!(raw.to_uppercase().as_str(), "TRUE" | "FALSE"))
            {
                raw = format!("'{raw}");
            }
            if !follower {
                cells.insert(a.into(), json!(raw));
            }
            cached.insert(
                a.into(),
                if t == "str" {
                    json!(crate::xlsx_strings::decode(value))
                } else {
                    value
                        .parse::<f64>()
                        .map(|n| json!(n))
                        .unwrap_or(json!(value))
                },
            );
            if let Some(s) = attr(c, "s") {
                styles.insert(a.into(), json!(s));
            }
            let resolved_style = inherited_styles.at(a)?;
            if attr(c, "s").is_some() || resolved_style != "0" {
                if let Some(spec) = resolved_style
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| styles_catalog.get(i))
                {
                    cell_styles.insert(a.into(), spec.clone());
                }
            }
            if let Some((_, Some(runs))) = content {
                let spec = cell_styles
                    .entry(a.to_string())
                    .or_insert_with(|| json!({}));
                spec["_richText"] = runs;
            }
        }
        let row_count = cells
            .keys()
            .filter_map(|a| parse_address(a).ok())
            .map(|p| p.0 + 1)
            .max()
            .unwrap_or(1);
        let col_count = cells
            .keys()
            .filter_map(|a| parse_address(a).ok())
            .map(|p| p.1 + 1)
            .max()
            .unwrap_or(1);
        sheets.push(json!({"id":format!("office-{}",sheets.len()+1),"name":attr(sh,"name").unwrap_or("Sheet"),"cells":cells,"cached":cached,"officePath":path,"officeStyles":styles,"cellStyles":cell_styles,"usedRows":row_count,"usedColumns":col_count}));
        if !arrays.is_empty() {
            sheets.last_mut().unwrap()["arrayFormulas"] = Value::Object(arrays);
        }
        sheets.last_mut().unwrap()["styleDefaults"] = inherited_styles.projection(&styles_catalog);
        sheets.last_mut().unwrap()["styleDefaults"]["normalFont"] = normal_font.clone();
        let (tables, hidden) = crate::xlsx_tables::read(pkg, &path)?;
        let sheet = sheets.last_mut().unwrap();
        sheet["tables"] = json!(tables);
        sheet["hiddenRows"] = json!(hidden);
        crate::sheet_filter::read(pkg, &path, sheet)?;
        crate::sheet_geometry::read(pkg, &path, sheet)?;
        crate::sheet_view::read(pkg, &path, sheet)?;
        crate::cell_watches::read(pkg, &path, sheet)?;
        crate::scenarios::read(pkg, &path, sheet)?;
        crate::data_validation::read(pkg, &path, sheet)?;
        crate::conditional_format::read(pkg, &path, sheet)?;
        if crate::xlsx_reviews::read(pkg, &path, sheet)? {
            warnings.push("Some worksheet annotations use unsupported or malformed metadata and remain preserved in the source.".into());
        }
        if sheet["extendedConditionalFormats"] == true
            || crate::conditional_format::rules(sheet).any(|r| r["unsupported"] == true)
        {
            warnings.push(format!("{}: some conditional-format rules are preserved but not displayed or editable yet.",sheet["name"].as_str().unwrap_or("Worksheet")));
        }
        if let Some(sort) = crate::sheet_sort::read(pkg, &path)? {
            sheet["rangeSort"] = sort;
        }
    }
    if sheets.is_empty() {
        return Err("Workbook has no supported worksheets".into());
    }
    warnings.sort();
    warnings.dedup();
    warnings.truncate(30);
    Ok((json!(sheets), report("xlsx", warnings, pkg.entries.len())))
}
pub(crate) fn shared_formulas<'a>(
    d: &'a roxmltree::Document<'a>,
) -> HashMap<&'a str, (&'a str, &'a str)> {
    d.descendants()
        .filter(|n| n.has_tag_name((S, "f")) && attr(*n, "t") == Some("shared"))
        .filter_map(|f| Some((attr(f, "si")?, (attr(f.parent()?, "r")?, f.text()?))))
        .collect()
}
/// Mirrors the calculation model's English input parsing, not a substring test.
/// Formula results use the model's separate type flag instead.
pub(crate) fn input_is_error(text: &str) -> bool {
    ironcalc_base::expressions::token::is_english_error_string(&text.to_uppercase())
}
fn excel_error(text: &str) -> bool {
    matches!(
        text,
        "#NULL!"
            | "#DIV/0!"
            | "#VALUE!"
            | "#REF!"
            | "#NAME?"
            | "#NUM!"
            | "#N/A"
            | "#SPILL!"
            | "#CALC!"
    )
}
pub(crate) fn value_xml(
    a: &str,
    value: &str,
    style: Option<&str>,
    calculated: &Value,
    is_error: bool,
) -> String {
    let style = style
        .map(|s| format!(" s=\"{}\"", esc(s)))
        .unwrap_or_default();
    let start = format!("<c xmlns=\"{S}\" r=\"{}\"{style}", esc(a));
    if let Some(text) = value.strip_prefix('\'') {
        return format!(
            "{start} t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
            crate::xlsx_strings::xml(text, false)
        );
    }
    if let Some(f) = value.strip_prefix('=') {
        // Engine diagnostics such as #N/IMPL! have no Excel cache equivalent.
        // Leave the cache absent so Excel can calculate the retained formula.
        if calculated.is_null() || (is_error && !calculated.as_str().is_some_and(excel_error)) {
            return format!("{start}><f>{}</f></c>", esc(f));
        }
        let (t, v) = if is_error {
            (" t=\"e\"", calculated.as_str().unwrap_or("").into())
        } else if calculated.is_number() {
            ("", calculated.to_string())
        } else if calculated.is_boolean() {
            (
                " t=\"b\"",
                if calculated == true {
                    "1".into()
                } else {
                    "0".into()
                },
            )
        } else {
            (" t=\"str\"", calculated.as_str().unwrap_or("").into())
        };
        let cache = if t == " t=\"str\"" {
            crate::xlsx_strings::xml(&v, false)
        } else {
            esc(&v)
        };
        format!("{start}{t}><f>{}</f><v>{cache}</v></c>", esc(f))
    } else if is_error || input_is_error(value) {
        format!(
            "{start} t=\"e\"><v>{}</v></c>",
            esc(calculated.as_str().unwrap_or(value).to_uppercase().as_str())
        )
    } else if value.is_empty() {
        format!("{start}/>")
    } else if value.parse::<f64>().is_ok_and(|n| n.is_finite()) {
        format!("{start}><v>{}</v></c>", esc(value))
    } else if calculated.is_number() {
        // User-entered dates, percentages and currencies have a numeric value.
        // Explicit source strings are protected by the apostrophe branch above.
        format!("{start}><v>{calculated}</v></c>")
    } else if let Some(boolean) = calculated.as_bool() {
        format!("{start} t=\"b\"><v>{}</v></c>", if boolean { 1 } else { 0 })
    } else {
        format!(
            "{start} t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
            crate::xlsx_strings::xml(value, false)
        )
    }
}
fn patched_cell(
    old: roxmltree::Node<'_, '_>,
    a: &str,
    value: &str,
    calculated: &Value,
    is_error: bool,
    xml: &str,
    style: Option<&str>,
) -> String {
    // Keep the original cell's namespace-qualified metadata and extension
    // payloads while replacing its value and direct style id.
    let fresh = value_xml(a, value, style.or(attr(old, "s")), calculated, is_error);
    let d = parse(&fresh).unwrap();
    let node = d.root_element();
    let scoped = scoped_record(xml, old);
    let original = parse(&scoped).unwrap();
    let output = crate::xlsx_tables::attributes(
        &scoped,
        original.root_element(),
        &[
            ("r", Some(a.into())),
            ("s", attr(node, "s").map(String::from)),
            ("t", attr(node, "t").map(String::from)),
        ],
    );
    let inner = node
        .children()
        .filter(|n| n.is_element())
        .map(|n| scoped_record(&fresh, n))
        .collect::<String>()
        + &old
            .children()
            .filter(|n| n.is_element() && !matches!(n.tag_name().name(), "f" | "v" | "is"))
            .map(|n| scoped_record(xml, n))
            .collect::<String>();
    fragment_inner(&output, &inner).unwrap()
}
/// Refresh a formula's cached result without replacing its formula attributes,
/// styles, metadata or retained extension children.
fn refresh_formula_cache(
    xml: &str,
    cell: roxmltree::Node,
    a: &str,
    value: &str,
    calc: &Value,
    is_error: bool,
) -> String {
    let formula = child(cell, "f").unwrap();
    let unsupported = matches!(attr(formula, "t"), Some("array" | "dataTable"));
    let fresh = value_xml(a, value, None, calc, is_error);
    let fd = parse(&fresh).unwrap();
    let fresh_cell = fd.root_element();
    let value_type = if unsupported || calc.is_null() {
        None
    } else {
        attr(fresh_cell, "t").map(String::from)
    };
    let base = crate::xlsx_tables::attributes(xml, cell, &[("t", value_type)]);
    let cached = if unsupported || calc.is_null() {
        String::new()
    } else {
        child(fresh_cell, "v")
            .map(|n| raw(&fresh, n).replacen("<v>", &format!("<v xmlns=\"{S}\">"), 1))
            .unwrap_or_default()
    };
    let extra = cell
        .children()
        .filter(|n| n.is_element() && !matches!(n.tag_name().name(), "f" | "v" | "is"))
        .map(|n| raw(xml, n))
        .collect::<String>();
    fragment_inner(&base, &(raw(xml, formula).to_string() + &cached + &extra))
        .expect("The source cell was parsed before refreshing its cache")
}
fn sheet_xml(sheet: &Value, calculated: &Value, errors: &Value) -> String {
    let mut rows: BTreeMap<usize, Vec<(usize, String)>> = BTreeMap::new();
    for (a, v) in sheet["cells"].as_object().unwrap() {
        let (r, c) = parse_address(a).unwrap();
        rows.entry(r).or_default().push((
            c,
            value_xml(
                a,
                v.as_str().unwrap(),
                sheet["officeStyles"][a].as_str(),
                &calculated[a],
                errors[a] == true,
            ),
        ));
    }
    let body = rows
        .into_iter()
        .map(|(r, mut cells)| {
            cells.sort_by_key(|c| c.0);
            format!(
                "<row xmlns=\"{S}\" r=\"{}\">{}</row>",
                r + 1,
                cells.into_iter().map(|c| c.1).collect::<String>()
            )
        })
        .collect::<String>();
    format!("{XML_HEAD}<worksheet xmlns=\"{S}\"><sheetData>{body}</sheetData></worksheet>")
}
fn patch_sheet(
    xml: &str,
    sheet: &Value,
    before: &Value,
    calc: &Value,
    errors: &Value,
    pkg: &Package,
) -> Result<String, String> {
    // Once any cell in a shared group changes, followers must no longer depend
    // on an old master that may be replaced or deleted. The imported projection
    // already contains each follower's expanded formula.
    let source = parse(xml)?;
    let mut expanded = vec![];
    for f in source
        .descendants()
        .filter(|n| n.has_tag_name((S, "f")) && attr(*n, "t") == Some("shared"))
    {
        let a = attr(f.parent().ok_or("Shared formula cell missing")?, "r")
            .ok_or("Shared formula address missing")?;
        let value = before["cells"][a]
            .as_str()
            .and_then(|v| v.strip_prefix('='))
            .ok_or("Shared formula projection missing")?;
        let tag = raw(xml, f)[1..].split([' ', '/', '>']).next().unwrap();
        expanded.push((f.range(), format!("<{tag}>{}</{tag}>", esc(value))));
    }
    let normalized = replace_ranges(xml, expanded);
    let xml = normalized.as_str();
    let d = parse(xml)?;
    let data = d
        .descendants()
        .find(|n| n.has_tag_name((S, "sheetData")))
        .ok_or("Worksheet data missing")?;
    let mut rows: BTreeMap<usize, String> = BTreeMap::new();
    let mut wanted: BTreeMap<usize, Vec<(usize, String, String)>> = BTreeMap::new();
    for (a, v) in sheet["cells"].as_object().ok_or("Cells missing")? {
        let (r, c) = parse_address(a)?;
        wanted
            .entry(r)
            .or_default()
            .push((c, a.clone(), v.as_str().unwrap_or("").into()));
    }
    for row in data.children().filter(|n| n.is_element()) {
        if row.tag_name().name() != "row" {
            continue;
        }
        let r = attr(row, "r")
            .ok_or("Row index missing")?
            .parse::<usize>()
            .map_err(|_| "Invalid row index")?
            .checked_sub(1)
            .ok_or("Invalid row index")?;
        let mut cells: BTreeMap<usize, String> = BTreeMap::new();
        let mut changed = false;
        for cell in row
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "c")
        {
            let a = attr(cell, "r").ok_or("Cell address missing")?;
            let (_, c) = parse_address(a)?;
            // A cell cleared of its value and format (Clear All) goes.
            if sheet["cells"].get(a).is_none()
                && sheet["officeStyles"].get(a).is_none()
                && sheet["cellStyles"].get(a).is_none()
                && before["cells"].get(a).is_some()
            {
                changed = true;
                continue;
            }
            let value = sheet["cells"][a].as_str().unwrap_or("");
            let was = before["cells"][a].as_str().unwrap_or("");
            // Clear Formats removes character runs: the text is rewritten.
            let runs_cleared = before["cellStyles"][a].get("_richText").is_some()
                && sheet["cellStyles"][a].get("_richText").is_none();
            if value == was && !runs_cleared {
                let font_edits = sheet["cellStyles"][a].get("_fontOverrides");
                let style_changed = sheet["officeStyles"][a] != before["officeStyles"][a];
                let mut updated = raw(xml, cell).to_string();
                if value.starts_with('=') && child(cell, "f").is_some() {
                    updated =
                        refresh_formula_cache(xml, cell, a, value, &calc[a], errors[a] == true);
                }
                if style_changed || font_edits.is_some() {
                    // Parsing a copied record needs namespaces declared on
                    // the worksheet/row, including prefixed extension data.
                    let scoped = scoped_record(xml, cell);
                    let d = parse(&scoped)?;
                    updated = if value.starts_with('=') && child(d.root_element(), "f").is_some() {
                        refresh_formula_cache(
                            &scoped,
                            d.root_element(),
                            a,
                            value,
                            &calc[a],
                            errors[a] == true,
                        )
                    } else {
                        scoped
                    };
                    let d = parse(&updated)?;
                    updated = crate::xlsx_tables::attributes(
                        &updated,
                        d.root_element(),
                        &[("s", sheet["officeStyles"][a].as_str().map(String::from))],
                    );
                    if let Some(spec) = font_edits {
                        updated = crate::xlsx_styles::format_rich_cell(pkg, &updated, spec)?;
                    }
                }
                changed |= updated != raw(xml, cell);
                cells.insert(c, updated);
            } else {
                changed = true;
                cells.insert(
                    c,
                    patched_cell(
                        cell,
                        a,
                        value,
                        &calc[a],
                        errors[a] == true,
                        xml,
                        sheet["officeStyles"][a].as_str(),
                    ),
                );
            }
        }
        for (c, a, v) in wanted.remove(&r).unwrap_or_default() {
            if let std::collections::btree_map::Entry::Vacant(e) = cells.entry(c) {
                changed = true;
                e.insert(value_xml(
                    &a,
                    &v,
                    sheet["officeStyles"][&a].as_str(),
                    &calc[&a],
                    errors[&a] == true,
                ));
            }
        }
        if changed {
            let extras = row
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() != "c")
                .map(|n| raw(xml, n))
                .collect::<String>();
            rows.insert(
                r,
                fragment_inner(
                    raw(xml, row),
                    &(cells.values().cloned().collect::<String>() + &extras),
                )?,
            );
        } else {
            rows.insert(r, raw(xml, row).into());
        }
    }
    for (r, mut cells) in wanted {
        cells.sort_by_key(|c| c.0);
        rows.insert(
            r,
            format!(
                "<row xmlns=\"{S}\" r=\"{}\">{}</row>",
                r + 1,
                cells
                    .iter()
                    .map(|(_, a, v)| value_xml(
                        a,
                        v,
                        sheet["officeStyles"][a].as_str(),
                        &calc[a],
                        errors[a] == true
                    ))
                    .collect::<String>()
            ),
        );
    }
    let new = fragment_inner(raw(xml, data), &rows.values().cloned().collect::<String>())?;
    let mut edits = vec![(data.range(), new)];
    if let Some(dim) = d.descendants().find(|n| n.has_tag_name((S, "dimension"))) {
        let used = sheet["cells"]
            .as_object()
            .unwrap()
            .keys()
            .filter_map(|a| parse_address(a).ok())
            .collect::<Vec<_>>();
        let r = used.iter().map(|p| p.0).max().unwrap_or(0);
        let c = used.iter().map(|p| p.1).max().unwrap_or(0);
        edits.push((
            dim.range(),
            format!("<dimension xmlns=\"{S}\" ref=\"A1:{}\"/>", address(r, c)),
        ));
    }
    Ok(replace_ranges(xml, edits))
}
pub fn export(
    sheets: &Value,
    calculated: &Value,
    errors: &Value,
    pkg: Option<&Package>,
    baseline: Option<&Value>,
) -> Result<Vec<u8>, String> {
    export_impl(sheets, calculated, errors, pkg, baseline, false)
}

/// Package-level edits have already changed the baseline XML. Refresh every
/// formula cache even when the editable projection compares equal.
pub(crate) fn export_recalculated(
    sheets: &Value,
    calculated: &Value,
    errors: &Value,
    pkg: &Package,
    baseline: &Value,
) -> Result<Vec<u8>, String> {
    export_impl(sheets, calculated, errors, Some(pkg), Some(baseline), true)
}

fn export_impl(
    sheets: &Value,
    calculated: &Value,
    errors: &Value,
    pkg: Option<&Package>,
    baseline: Option<&Value>,
    refresh: bool,
) -> Result<Vec<u8>, String> {
    if let (false, Some(p), Some(b)) = (refresh, pkg, baseline) {
        if sheets == b {
            return Ok(p.original.clone());
        }
    }
    let mut p = pkg.cloned().unwrap_or_default();
    let list = sheets.as_array().ok_or("Sheets missing")?;
    if pkg.is_none() {
        p.set("_rels/.rels",format!("{XML_HEAD}<Relationships xmlns=\"{REL_NS}\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>"));
        p.set("[Content_Types].xml",format!("{XML_HEAD}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/></Types>"));
        p.set("xl/workbook.xml",format!("{XML_HEAD}<workbook xmlns=\"{S}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets/></workbook>"));
        p.set(
            "xl/_rels/workbook.xml.rels",
            format!("{XML_HEAD}<Relationships xmlns=\"{REL_NS}\"/>"),
        );
    }
    if let Some(before) = baseline {
        crate::xlsx_tables::rewrite_references(&mut p, before, sheets)?;
    }
    let mut style_cache = HashMap::<(String, String), String>::new();
    let sheet_names: Vec<String> = list
        .iter()
        .map(|s| s["name"].as_str().unwrap_or("").to_string())
        .collect();
    let defined = p
        .text("xl/workbook.xml")
        .and_then(parse)
        .map(|wb| defined_names(&wb))
        .unwrap_or_default();
    for (index, original_sheet) in list.iter().enumerate() {
        let mut prepared = original_sheet.clone();
        let baseline_sheet = baseline
            .and_then(|b| b.as_array())
            .and_then(|a| a.iter().find(|v| v["id"] == prepared["id"]));
        // Formulas entered or changed in Folio are written the way a
        // worksheet part stores them (`_xlfn.` prefixes, `@` resolved).
        // Unchanged formulas keep their original XML.
        if let Some(cells) = prepared["cells"].as_object_mut() {
            for (a, value) in cells.iter_mut() {
                let Some(formula) = value.as_str().filter(|v| v.starts_with('=')) else {
                    continue;
                };
                if baseline_sheet.is_some_and(|b| b["cells"][a.as_str()] == *value) {
                    continue;
                }
                let stored =
                    crate::structure::excel_formula(formula, &sheet_names, index, a, &defined);
                *value = json!(format!("={stored}"));
            }
        }
        let style_bases =
            crate::xlsx_styles::SheetStyleBases::read(&p, original_sheet["officePath"].as_str())?;
        if let Some(styles) = original_sheet["cellStyles"].as_object() {
            for (a, spec) in styles {
                if baseline_sheet.is_none_or(|b| b["cellStyles"][a] != *spec) {
                    let base = original_sheet["officeStyles"][a]
                        .as_str()
                        .unwrap_or(style_bases.at(a)?);
                    // Only the actual edits override the original XF. An old
                    // projection might contain an incomplete format label.
                    let mut delta = if let Some(before) = baseline_sheet
                        .filter(|b| b["officeStyles"][a] == original_sheet["officeStyles"][a])
                    {
                        Value::Object(
                            spec.as_object()
                                .ok_or("Invalid cell style")?
                                .iter()
                                .filter(|(k, v)| before["cellStyles"][a].get(*k) != Some(*v))
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect(),
                        )
                    } else {
                        spec.clone()
                    };
                    // Character metadata does not change the cell's XF. Its
                    // run edits are applied to worksheet text below.
                    if let Some(properties) = delta.as_object_mut() {
                        properties.retain(|key, _| !key.starts_with('_'));
                        if properties.is_empty() {
                            continue;
                        }
                    }
                    let key = (base.to_string(), delta.to_string());
                    let id = if let Some(id) = style_cache.get(&key) {
                        id.clone()
                    } else {
                        let id = crate::xlsx_styles::add(&mut p, &delta, Some(base))?;
                        style_cache.insert(key, id.clone());
                        id
                    };
                    if !prepared["officeStyles"].is_object() {
                        prepared["officeStyles"] = json!({});
                    }
                    prepared["officeStyles"][a] = json!(id);
                }
            }
        }
        let sheet = &prepared;
        let path = sheet["officePath"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| {
                (1..)
                    .map(|n| format!("xl/worksheets/folio-{n}.xml"))
                    .find(|candidate| !p.entries.contains_key(candidate))
                    .unwrap()
            });
        let calc = &calculated[sheet["id"].as_str().unwrap()];
        let sheet_errors = &errors[sheet["id"].as_str().unwrap()];
        let before = baseline
            .and_then(|b| b.as_array())
            .and_then(|a| a.iter().find(|v| v["id"] == sheet["id"]));
        if let (Some(b), Ok(xml)) = (before, p.text(&path)) {
            let result = patch_sheet(xml, sheet, b, calc, sheet_errors, &p)?;
            p.set(
                &path,
                crate::xlsx_styles::patch_character_formats(&p, &result, sheet)?,
            );
        } else {
            p.set(
                &path,
                crate::xlsx_styles::patch_character_formats(
                    &p,
                    &sheet_xml(sheet, calc, sheet_errors),
                    sheet,
                )?,
            );
            let rels = parse(p.text("xl/_rels/workbook.xml.rels")?)?;
            let used: std::collections::HashSet<_> =
                rels.descendants().filter_map(|n| attr(n, "Id")).collect();
            let rid = (1..)
                .map(|n| format!("folioSheet{n}"))
                .find(|id| !used.contains(id.as_str()))
                .unwrap();
            let workbook = p.text("xl/workbook.xml")?.to_string();
            let d = parse(&workbook)?;
            let new_id = d
                .descendants()
                .filter_map(|n| attr(n, "sheetId").and_then(|v| v.parse::<u32>().ok()))
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or("Worksheet id limit reached")?;
            let node = d
                .descendants()
                .find(|n| n.has_tag_name((S, "sheets")))
                .ok_or("Sheets missing")?;
            let inner = node
                .children()
                .filter(|n| n.is_element())
                .map(|n| raw(&workbook, n))
                .collect::<String>();
            let next = fragment_inner(
                raw(&workbook, node),
                &format!(
                    "{inner}<sheet xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" name=\"{}\" sheetId=\"{}\" r:id=\"{rid}\"/>",
                    esc(sheet["name"].as_str().unwrap()),
                    new_id
                ),
            )?;
            p.set(
                "xl/workbook.xml",
                replace_ranges(&workbook, vec![(node.range(), next)]),
            );
            append_root(&mut p,"xl/_rels/workbook.xml.rels",&format!("<Relationship Id=\"{rid}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"{}\"/>",esc(path.trim_start_matches("xl/"))))?;
            append_root(&mut p,"[Content_Types].xml",&format!("<Override PartName=\"/{path}\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"))?;
        }
        crate::xlsx_tables::write(&mut p, sheet, &path)?;
        crate::sheet_geometry::write(&mut p, sheet, &path)?;
        crate::sheet_view::write(&mut p, sheet, before, &path)?;
        crate::sheet_view::write_zoom(&mut p, sheet, before, &path)?;
        crate::cell_watches::write(&mut p, sheet, before, &path)?;
        crate::scenarios::write(&mut p, sheet, before, &path)?;
        crate::data_validation::write(&mut p, sheet, before, &path)?;
        crate::conditional_format::write(&mut p, sheet, before, &path)?;
        crate::xlsx_reviews::write(&mut p, sheet, before, &path)?;
    }
    force_recalculation(&mut p)?;
    p.bytes()
}

pub(crate) fn force_recalculation(p: &mut Package) -> Result<(), String> {
    // Preserve calculation preferences (including iteration/precision) while
    // invalidating caches. Structural exports need this even if no cell edits
    // preceded the operation and export returned the original package bytes.
    let wb = p.text("xl/workbook.xml")?.to_string();
    let d = parse(&wb)?;
    let calc = "<calcPr calcId=\"0\" fullCalcOnLoad=\"1\" forceFullCalc=\"1\"/>";
    let out = if let Some(n) = d.descendants().find(|n| n.has_tag_name((S, "calcPr"))) {
        let mut edits = vec![];
        let mut missing = String::new();
        for (name, value) in [
            ("calcId", "0"),
            ("fullCalcOnLoad", "1"),
            ("forceFullCalc", "1"),
        ] {
            if let Some(a) = n.attributes().find(|a| a.name() == name) {
                edits.push((a.range_value(), value.into()));
            } else {
                missing.push_str(&format!(" {name}=\"{value}\""));
            }
        }
        let end = n.range().start
            + raw(&wb, n)
                .find('>')
                .ok_or("Invalid calculation properties")?;
        let at = if wb.as_bytes()[end - 1] == b'/' {
            end - 1
        } else {
            end
        };
        edits.push((at..at, missing));
        replace_ranges(&wb, edits)
    } else {
        let root = d.root_element();
        let content = root.children().map(|n| raw(&wb, n)).collect::<String>();
        replace_ranges(
            &wb,
            vec![(
                root.range(),
                fragment_inner(raw(&wb, root), &(content + calc))?,
            )],
        )
    };
    p.set("xl/workbook.xml", out);
    Ok(())
}
fn append_root(p: &mut Package, path: &str, extra: &str) -> Result<(), String> {
    let xml = p.text(path)?.to_string();
    let d = parse(&xml)?;
    let root = d.root_element();
    let inner = root.children().map(|n| raw(&xml, n)).collect::<String>();
    p.set(
        path,
        replace_ranges(
            &xml,
            vec![(
                root.range(),
                fragment_inner(raw(&xml, root), &(inner + extra))?,
            )],
        ),
    );
    Ok(())
}

pub(crate) fn fragment_inner(fragment: &str, content: &str) -> Result<String, String> {
    let end = fragment.find('>').ok_or("Invalid XML element")?;
    if fragment[..end].ends_with('/') {
        let tag = fragment[1..].split([' ', '/', '>']).next().unwrap();
        Ok(format!("{}>{content}</{tag}>", &fragment[..end - 1]))
    } else {
        let close = fragment.rfind("</").ok_or("Invalid XML element")?;
        Ok(format!(
            "{}{content}{}",
            &fragment[..=end],
            &fragment[close..]
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workbook_preserves_objects_and_formulas() {
        let p = Package::from_bytes(include_bytes!("../../test/fixtures/source.xlsx").to_vec())
            .unwrap();
        let (sheets, _) = import(&p).unwrap();
        assert_eq!(sheets[0]["cells"]["B2"], "2400");
        assert_eq!(
            export(&sheets, &json!({}), &json!({}), Some(&p), Some(&sheets)).unwrap(),
            p.original
        );
        let mut edited = sheets.clone();
        edited[0]["cells"]["B2"] = json!("3000");
        edited[0]["cells"]["AB10"] = json!("New cell");
        let mut calc = crate::calculation::WorkbookCalculator::default();
        calc.sync(&edited);
        assert_eq!(calc.values["office-1"]["B4"], 10600.0);
        let bytes = export(
            &edited,
            &calc.values,
            &calc.cell_errors,
            Some(&p),
            Some(&sheets),
        )
        .unwrap();
        let out = Package::from_bytes(bytes.clone()).unwrap();
        for (name, old) in &p.entries {
            if name != "xl/workbook.xml" && !name.starts_with("xl/worksheets/sheet") {
                assert_eq!(&out.entries[name], old, "changed {name}");
            }
        }
        let imported = import(&out).unwrap().0;
        assert_eq!(imported[0]["cells"]["B2"], "3000");
        assert_eq!(imported[0]["cells"]["B4"], "=SUM(B2:B3)");
        assert_eq!(imported[0]["cells"]["AB10"], "New cell");
        std::fs::write(
            std::env::temp_dir().join("folio-verified-edited.xlsx"),
            bytes,
        )
        .unwrap();
    }
    #[test]
    fn new_workbook_opens() {
        let sheets = json!([{"id":"s","name":"New sheet","cells":{"A1":"Label","B1":"Value","A2":"Sales","B2":"1200","B3":"=SUM(B2:B2)"}}]);
        let bytes = export(&sheets, &json!({"s":{"B3":1200}}), &json!({}), None, None).unwrap();
        let p = Package::from_bytes(bytes.clone()).unwrap();
        assert_eq!(import(&p).unwrap().0[0]["cells"]["B3"], "=SUM(B2:B2)");
        std::fs::write(std::env::temp_dir().join("folio-verified-new.xlsx"), bytes).unwrap();
    }
}
#[cfg(test)]
mod text_tests {
    use super::*;
    #[test]
    fn literal_formula_and_numeric_text_stay_text() {
        let sheets = json!([{"id":"s","name":"Text","cells":{"A1":"'00123","A2":"'=SUM(B1:B2)","A3":"'TRUE","B1":"2"}}]);
        let data = export(&sheets, &json!({}), &json!({}), None, None).unwrap();
        let imported = import(&Package::from_bytes(data).unwrap()).unwrap().0;
        assert_eq!(imported[0]["cells"]["A1"], "'00123");
        assert_eq!(imported[0]["cells"]["A2"], "'=SUM(B1:B2)");
        assert_eq!(imported[0]["cells"]["A3"], "'TRUE");
    }
}

pub(crate) const DATE1904_LEGACY_WARNING: &str = "This workbook uses the 1904 date system. Date display honors that setting; calendar-based formula evaluation still uses the 1900 system and needs qualification.";
pub(crate) const DATE1904_NOTICE: &str =
    "Dates and supported date formulas use this workbook's 1904 date system.";
