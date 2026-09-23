//! Worksheet validation rules, distinct from cell data and calculation results.
use crate::{
    calculation::WorkbookCalculator,
    formula::{address, parse_address},
    office::*,
    tables::Area,
};
use serde_json::{json, Value};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const TYPES: &[&str] = &[
    "none",
    "whole",
    "decimal",
    "list",
    "date",
    "time",
    "textLength",
    "custom",
];
const OPS: &[&str] = &[
    "between",
    "notBetween",
    "equal",
    "notEqual",
    "lessThan",
    "lessThanOrEqual",
    "greaterThan",
    "greaterThanOrEqual",
];
const STRINGS: &[&str] = &[
    "type",
    "operator",
    "errorStyle",
    "promptTitle",
    "prompt",
    "errorTitle",
    "error",
];
const BOOLS: &[&str] = &[
    "allowBlank",
    "showInputMessage",
    "showErrorMessage",
    "showDropDown",
];

pub fn rules(sheet: &Value) -> impl Iterator<Item = &Value> {
    sheet
        .get("dataValidations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}
fn areas(rule: &Value) -> Result<Vec<Area>, String> {
    let refs = rule["sqref"].as_str().ok_or("Validation range missing")?;
    if refs.len() > 65536 {
        return Err("Validation range is too long".into());
    }
    let result: Vec<Area> = refs
        .split_whitespace()
        .map(Area::parse)
        .collect::<Result<_, _>>()?;
    if result.is_empty() {
        return Err("Validation range is empty".into());
    }
    Ok(result)
}
pub fn at<'a>(sheet: &'a Value, cell: &str) -> Option<&'a Value> {
    let (r, c) = parse_address(cell).ok()?;
    rules(sheet).find(|rule| areas(rule).is_ok_and(|a| a.iter().any(|a| a.contains(r, c))))
}
fn flag(rule: &Value, key: &str) -> bool {
    rule[key].as_bool().unwrap_or(false)
}

pub fn read(pkg: &Package, path: &str, sheet: &mut Value) -> Result<(), String> {
    let doc = parse(pkg.text(path)?)?;
    let mut result = vec![];
    if let Some(parent) = child(doc.root_element(), "dataValidations") {
        for (i, n) in parent
            .children()
            .filter(|n| n.has_tag_name((S, "dataValidation")))
            .enumerate()
        {
            let mut r = json!({"sqref":attr(n,"sqref").unwrap_or(""),"officeIndex":i,
                "type":attr(n,"type").unwrap_or("none"),"operator":attr(n,"operator").unwrap_or("between"),
                "errorStyle":attr(n,"errorStyle").unwrap_or("stop")});
            for key in STRINGS {
                if let Some(v) = attr(n, key) {
                    r[key] = json!(v);
                }
            }
            for key in BOOLS {
                r[key] = json!(matches!(attr(n, key), Some("1" | "true")));
            }
            for key in ["formula1", "formula2"] {
                if let Some(v) = child(n, key).and_then(|c| c.text()) {
                    r[key] = json!(v);
                }
            }
            if areas(&r).is_err()
                || !TYPES.contains(&r["type"].as_str().unwrap_or(""))
                || !OPS.contains(&r["operator"].as_str().unwrap_or(""))
                || !["stop", "warning", "information"]
                    .contains(&r["errorStyle"].as_str().unwrap_or(""))
            {
                r["unsupported"] = json!(true);
            }
            result.push(r);
        }
    }
    sheet["dataValidations"] = json!(result);
    // Extended rules must never silently disappear or be overridden by a new
    // standard rule. They remain in their source extension until supported.
    let extensions: Vec<_> = doc
        .descendants()
        .filter(|n| {
            n.is_element()
                && n.tag_name().name() == "dataValidations"
                && n.tag_name().namespace() != Some(S)
        })
        .collect();
    sheet["extendedDataValidations"] = json!(!extensions.is_empty());
    let mut extended_ranges = vec![];
    let mut unknown = false;
    for extension in extensions {
        for rule in extension
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "dataValidation")
        {
            if let Some(refs) = rule
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "sqref")
                .and_then(|n| n.text())
            {
                if areas(&json!({"sqref":refs})).is_ok() {
                    extended_ranges.push(refs.to_owned());
                } else {
                    unknown = true;
                }
            } else {
                unknown = true;
            }
        }
    }
    sheet["extendedValidationRanges"] = json!(extended_ranges);
    sheet["extendedValidationRangeUnknown"] = json!(unknown);
    Ok(())
}

pub fn validate(sheet: &Value) -> Result<(), String> {
    if let Some(list) = sheet.get("dataValidations") {
        let list = list.as_array().ok_or("Invalid validation rules")?;
        if list.len() > 65534 {
            return Err("Too many validation rules".into());
        }
        for rule in list {
            if rule["unsupported"] == true {
                continue;
            }
            areas(rule)?;
            if !TYPES.contains(&rule["type"].as_str().unwrap_or("none")) {
                return Err("Unknown validation type".into());
            }
            if !OPS.contains(&rule["operator"].as_str().unwrap_or("between")) {
                return Err("Unknown validation operator".into());
            }
            if !["stop", "warning", "information"]
                .contains(&rule["errorStyle"].as_str().unwrap_or("stop"))
            {
                return Err("Unknown validation error style".into());
            }
            for key in [
                "formula1",
                "formula2",
                "promptTitle",
                "prompt",
                "errorTitle",
                "error",
            ] {
                if rule
                    .get(key)
                    .is_some_and(|v| v.as_str().is_none_or(|v| v.len() > 32768))
                {
                    return Err("Invalid validation text".into());
                }
            }
        }
    }
    Ok(())
}

fn formula_for(
    rule: &Value,
    key: &str,
    workspace: &Value,
    sheet: usize,
    cell: &str,
) -> Result<String, String> {
    let f = rule[key].as_str().ok_or("Validation formula missing")?;
    let anchor = areas(rule)?[0];
    let names = workspace["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().into())
        .collect::<Vec<_>>();
    crate::structure::copy_formula(f, &names, sheet, &address(anchor.top, anchor.left), cell)
}

fn list_values(
    workspace: &Value,
    sheet: usize,
    cell: &str,
    rule: &Value,
    calc: &mut WorkbookCalculator,
) -> Result<Vec<Value>, String> {
    let f = rule["formula1"].as_str().ok_or("List source missing")?;
    if f.starts_with('"') && f.ends_with('"') && f.len() >= 2 {
        return Ok(f[1..f.len() - 1]
            .replace("\"\"", "\"")
            .split(',')
            .map(|s| json!(s))
            .collect());
    }
    let f = formula_for(rule, "formula1", workspace, sheet, cell)?;
    let rows = calc.expression(workspace["sheets"][sheet]["id"].as_str().unwrap(), cell, &f)?;
    let rows = rows.as_array().ok_or("Invalid list result")?;
    if rows.len() > 1
        && rows
            .iter()
            .any(|r| r.as_array().is_some_and(|r| r.len() > 1))
    {
        return Err("Use one row or one column for a validation list".into());
    }
    Ok(rows
        .iter()
        .flat_map(|r| r.as_array().into_iter().flatten())
        .cloned()
        .collect())
}
pub fn value_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.into(),
        _ => {
            let n = value.as_f64().unwrap_or(0.0);
            if n.fract() == 0.0 {
                format!("{n:.0}")
            } else {
                n.to_string()
            }
        }
    }
}
pub fn choices(
    workspace: &Value,
    request: &Value,
    calc: &mut WorkbookCalculator,
) -> Result<Value, String> {
    let sheet = workspace["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["id"] == request["sheetId"])
        .ok_or("Worksheet missing")?;
    let cell = request["address"].as_str().ok_or("Cell missing")?;
    parse_address(cell)?;
    let Some(rule) = at(&workspace["sheets"][sheet], cell) else {
        return Ok(json!({"rule":null,"values":[],"revision":workspace["revision"]}));
    };
    let values = if rule["type"] == "list" && request["settingsOnly"] != true {
        list_values(workspace, sheet, cell, rule, calc)?
    } else {
        vec![]
    };
    let mut display_rule = rule.clone();
    let anchor = areas(rule)?[0];
    let names = workspace["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().into())
        .collect::<Vec<_>>();
    reanchor(
        &mut display_rule,
        &names,
        sheet,
        &address(anchor.top, anchor.left),
        cell,
    )?;
    display_rule["sqref"] = json!(cell);
    Ok(
        json!({"rule":display_rule,"values":values.iter().map(|v|json!({"value":v,"label":value_text(v)})).collect::<Vec<_>>(),"revision":workspace["revision"]}),
    )
}

/// Check only the cell being entered. Existing invalid cells and later changes
/// to precedents remain editable; validation is not a recalculation constraint.
pub fn check(candidate: &Value, request: &Value, date1904: bool) -> Result<Value, String> {
    let sheet = candidate["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["id"] == request["sheetId"])
        .ok_or("Worksheet missing")?;
    let cell = request["address"].as_str().ok_or("Cell missing")?;
    parse_address(cell)?;
    let source = &candidate["sheets"][sheet];
    let (r, c) = parse_address(cell)?;
    if source["extendedDataValidations"] == true
        && (source["extendedValidationRangeUnknown"] != false
            || source["extendedValidationRanges"]
                .as_array()
                .is_none_or(|refs| {
                    refs.iter().any(|refs| {
                        areas(&json!({"sqref":refs}))
                            .is_ok_and(|areas| areas.iter().any(|a| a.contains(r, c)))
                    })
                }))
    {
        return Ok(
            json!({"valid":false,"shouldAlert":true,"canOverride":false,"reason":"This cell has extended Excel validation that Folio cannot check yet.","rule":null}),
        );
    }
    let Some(rule) = at(source, cell) else {
        return Ok(json!({"valid":true}));
    };
    if rule["unsupported"] == true {
        return Ok(
            json!({"valid":false,"shouldAlert":true,"canOverride":false,"reason":"This cell has an unsupported Excel validation rule.","rule":rule}),
        );
    }
    let id = source["id"].as_str().unwrap();
    let kind = rule["type"].as_str().unwrap_or("none");
    let mut calc = WorkbookCalculator::default();
    calc.sync_calendar(&candidate["sheets"], date1904);
    if let Some(error) = &calc.error {
        return Err(error.clone());
    }
    let value = calc.values[id][cell].clone();
    let evaluate = |calc: &mut WorkbookCalculator| -> Result<bool, String> {
        if kind == "none" || (flag(rule, "allowBlank") && (value.is_null() || value == "")) {
            return Ok(true);
        }
        if calc.cell_errors[id][cell] == true {
            return Ok(false);
        }
        if kind == "list" {
            let values = list_values(candidate, sheet, cell, rule, calc)?;
            return Ok(values
                .iter()
                .any(|v| value_text(v).eq_ignore_ascii_case(&value_text(&value))));
        }
        let scalar = |calc: &mut WorkbookCalculator, key: &str| -> Result<Value, String> {
            let f = formula_for(rule, key, candidate, sheet, cell)?;
            let v = calc.expression(id, cell, &f)?;
            if v.as_array().is_none_or(|r| r.len() != 1)
                || v[0].as_array().is_none_or(|r| r.len() != 1)
            {
                return Err("Use a single value for the validation formula".into());
            }
            Ok(v[0][0].clone())
        };
        if kind == "custom" {
            let v = scalar(calc, "formula1")?;
            return Ok(v
                .as_bool()
                .unwrap_or_else(|| v.as_f64().is_some_and(|n| n != 0.0)));
        }
        let number = if kind == "textLength" {
            calc.expression(id, cell, &format!("LEN({cell})"))?[0][0].as_f64()
        } else {
            value.as_f64()
        };
        let Some(n) = number else {
            return Ok(false);
        };
        if kind == "whole" && n.fract() != 0.0 {
            return Ok(false);
        }
        let min = scalar(calc, "formula1")?
            .as_f64()
            .ok_or("The validation boundary is not a number")?;
        let op = rule["operator"].as_str().unwrap_or("between");
        Ok(match op {
            "between" | "notBetween" => {
                let max = scalar(calc, "formula2")?
                    .as_f64()
                    .ok_or("The second validation boundary is not a number")?;
                let inside = n >= min && n <= max;
                if op == "between" {
                    inside
                } else {
                    !inside
                }
            }
            "equal" => n == min,
            "notEqual" => n != min,
            "lessThan" => n < min,
            "lessThanOrEqual" => n <= min,
            "greaterThan" => n > min,
            "greaterThanOrEqual" => n >= min,
            _ => return Err("Unknown validation comparison".into()),
        })
    };
    let (valid, reason) = match evaluate(&mut calc) {
        Ok(v) => (v, None),
        Err(e) => (
            false,
            Some(format!("The validation rule could not be checked: {e}")),
        ),
    };
    Ok(
        json!({"valid":valid,"rule":rule,"reason":reason,"shouldAlert":flag(rule,"showErrorMessage") || reason.is_some(),
        "canOverride":reason.is_none() && matches!(rule["errorStyle"].as_str(),Some("warning"|"information"))}),
    )
}

/// Rebind preserved-source indices whenever a transaction creates a new OOXML
/// baseline. Source order can change after a rule is split or replaced.
pub fn bind(sheets: &mut Value, projection: &Value) {
    if let Some(sheets) = sheets.as_array_mut() {
        for sheet in sheets {
            if let Some(p) = projection
                .as_array()
                .and_then(|all| all.iter().find(|s| s["name"] == sheet["name"]))
            {
                for key in [
                    "dataValidations",
                    "extendedDataValidations",
                    "extendedValidationRanges",
                    "extendedValidationRangeUnknown",
                ] {
                    sheet[key] = p[key].clone();
                }
            }
        }
    }
}

pub(crate) fn subtract(a: Area, b: Area) -> Vec<Area> {
    if !a.overlaps(b) {
        return vec![a];
    }
    let top = a.top.max(b.top);
    let bottom = a.bottom.min(b.bottom);
    let left = a.left.max(b.left);
    let right = a.right.min(b.right);
    let mut result = vec![];
    if a.top < top {
        result.push(Area {
            bottom: top - 1,
            ..a
        });
    }
    if bottom < a.bottom {
        result.push(Area {
            top: bottom + 1,
            ..a
        });
    }
    if a.left < left {
        result.push(Area {
            top,
            bottom,
            right: left - 1,
            ..a
        });
    }
    if right < a.right {
        result.push(Area {
            top,
            bottom,
            left: right + 1,
            ..a
        });
    }
    result.sort_by_key(|a| (a.top, a.left));
    result
}

fn reanchor(
    rule: &mut Value,
    names: &[String],
    sheet: usize,
    from: &str,
    to: &str,
) -> Result<(), String> {
    if from == to {
        return Ok(());
    }
    for key in ["formula1", "formula2"] {
        if let Some(f) = rule[key].as_str() {
            if !f.starts_with('"') {
                rule[key] = json!(crate::structure::copy_formula(f, names, sheet, from, to)?);
            }
        }
    }
    Ok(())
}

pub fn change(workspace: &Value, request: &Value) -> Result<Value, String> {
    if request["baseRevision"] != workspace["revision"] {
        return Err("The workbook changed. Open Data Validation again.".into());
    }
    let mut next = workspace.clone();
    let sheets = next["sheets"].as_array_mut().ok_or("Worksheets missing")?;
    let names: Vec<String> = sheets
        .iter()
        .map(|s| s["name"].as_str().unwrap_or("Sheet").into())
        .collect();
    let index = sheets
        .iter()
        .position(|s| s["id"] == request["sheetId"])
        .ok_or("Worksheet missing")?;
    let sheet = &mut sheets[index];
    if sheet["extendedDataValidations"] == true {
        return Err("This worksheet uses extended Excel validation rules. Their source is preserved, but editing those rules is not supported yet.".into());
    }
    let selected = Area::parse(request["range"].as_str().ok_or("Choose a cell range")?)?;
    let mut result = vec![];
    let inherited = at(sheet, &address(selected.top, selected.left))
        .and_then(|r| r.get("officeIndex"))
        .cloned();
    for rule in rules(sheet) {
        let old = areas(rule)?;
        let rest: Vec<_> = old.iter().flat_map(|a| subtract(*a, selected)).collect();
        if rest == old {
            result.push(rule.clone());
            continue;
        }
        if rule["unsupported"] == true {
            return Err(
                "This selection includes an unsupported validation rule. Nothing changed.".into(),
            );
        }
        if !rest.is_empty() {
            let mut rule = rule.clone();
            reanchor(
                &mut rule,
                &names,
                index,
                &address(old[0].top, old[0].left),
                &address(rest[0].top, rest[0].left),
            )?;
            rule["sqref"] = json!(rest.iter().map(|a| a.text()).collect::<Vec<_>>().join(" "));
            result.push(rule);
        }
    }
    match request["action"].as_str() {
        Some("clear") => (),
        Some("set") => {
            let input = &request["rule"];
            let kind = input["type"]
                .as_str()
                .ok_or("Choose an allowed value type")?;
            let mut rule = json!({"sqref":selected.text(),"type":kind,"operator":input["operator"].as_str().unwrap_or("between"),"errorStyle":input["errorStyle"].as_str().unwrap_or("stop")});
            for key in STRINGS {
                if let Some(v) = input.get(key) {
                    rule[key] = v.clone();
                }
            }
            for key in BOOLS {
                rule[key] = json!(flag(input, key));
            }
            for key in ["formula1", "formula2"] {
                if let Some(f) = input[key].as_str() {
                    rule[key] = json!(f.trim().strip_prefix('=').unwrap_or(f.trim()));
                }
            }
            if !TYPES.contains(&kind) {
                return Err("Choose a supported validation type".into());
            }
            if kind != "none" && rule["formula1"].as_str().is_none_or(str::is_empty) {
                return Err("Enter the validation value or formula".into());
            }
            if !["none", "list", "custom"].contains(&kind)
                && ["between", "notBetween"]
                    .contains(&rule["operator"].as_str().unwrap_or("between"))
                && rule["formula2"].as_str().is_none_or(str::is_empty)
            {
                return Err("Enter both validation boundaries".into());
            }
            for (key, max) in [
                ("promptTitle", 32),
                ("errorTitle", 32),
                ("prompt", 255),
                ("error", 255),
            ] {
                if rule[key]
                    .as_str()
                    .is_some_and(|v| v.encode_utf16().count() > max)
                {
                    return Err(format!("{key} exceeds {max} characters"));
                }
            }
            if kind == "list"
                && rule["formula1"]
                    .as_str()
                    .is_some_and(|v| v.starts_with('"') && v.encode_utf16().count() > 257)
            {
                return Err("A typed list supports up to 255 characters. Use a cell range for a longer list.".into());
            }
            let anchor = address(selected.top, selected.left);
            reanchor(&mut rule, &names, index, &anchor, &anchor)?;
            if let Some(i) = inherited {
                rule["officeIndex"] = i;
            }
            result.push(rule);
        }
        _ => return Err("Choose Set or Clear validation".into()),
    }
    sheet["dataValidations"] = json!(result);
    validate(sheet)?;
    Ok(next)
}

pub fn write(
    pkg: &mut Package,
    sheet: &Value,
    before: Option<&Value>,
    path: &str,
) -> Result<(), String> {
    if sheet.get("dataValidations").is_none()
        || before.is_some_and(|s| s["dataValidations"] == sheet["dataValidations"])
    {
        return Ok(());
    }
    validate(sheet)?;
    let xml = pkg.text(path)?.to_owned();
    let doc = parse(&xml)?;
    let root = doc.root_element();
    let parent = child(root, "dataValidations");
    let originals: Vec<_> = parent
        .into_iter()
        .flat_map(|p| p.children())
        .filter(|n| n.has_tag_name((S, "dataValidation")))
        .collect();
    let mut children = String::new();
    for rule in rules(sheet) {
        let portable = rule["officeXml"].as_str().map(parse).transpose()?;
        let source_xml = rule["officeXml"].as_str().unwrap_or(&xml);
        let source = portable.as_ref().map(|d| d.root_element()).or_else(|| {
            rule["officeIndex"]
                .as_u64()
                .and_then(|i| originals.get(i as usize))
                .copied()
        });
        if rule["unsupported"] == true {
            children.push_str(raw(
                source_xml,
                source.ok_or("Unsupported validation source missing")?,
            ));
            continue;
        }
        let mut attrs = vec![("sqref", Some(rule["sqref"].as_str().unwrap().to_owned()))];
        for key in STRINGS {
            attrs.push((key, rule[key].as_str().map(str::to_owned)));
        }
        for key in BOOLS {
            attrs.push((
                key,
                Some(if flag(rule, key) { "1" } else { "0" }.to_owned()),
            ));
        }
        let empty = format!("<dataValidation xmlns=\"{S}\"/>");
        let empty_doc = parse(&empty)?;
        let base = match source {
            Some(n) => crate::xlsx_tables::attributes(source_xml, n, &attrs),
            None => crate::xlsx_tables::attributes(&empty, empty_doc.root_element(), &attrs),
        };
        let mut contents = String::new();
        for key in ["formula1", "formula2"] {
            if let Some(f) = rule[key].as_str().filter(|s| !s.is_empty()) {
                contents.push_str(&format!("<{key} xmlns=\"{S}\">{}</{key}>", esc(f)));
            }
        }
        if let Some(n) = source {
            for child in n.children().filter(|n| {
                n.is_element() && !matches!(n.tag_name().name(), "formula1" | "formula2")
            }) {
                contents.push_str(raw(source_xml, child));
            }
        }
        // Reuse the source element's start/end tag without reparsing namespace
        // prefixes which are declared on the worksheet root.
        let start = opening_tag_end(&base).ok_or("Invalid validation XML")?;
        let built = if base[..start].ends_with('/') {
            let tag = base[1..].split([' ', '/', '>']).next().unwrap();
            format!("{}>{contents}</{tag}>", &base[..start - 1])
        } else {
            format!(
                "{}{contents}{}",
                &base[..=start],
                &base[base.rfind("</").ok_or("Invalid validation XML")?..]
            )
        };
        children.push_str(&built);
    }
    let count = rules(sheet).count();
    let replacement = if count == 0 {
        String::new()
    } else if let Some(p) = parent {
        // Parent has the same namespace context as the source root. Replace
        // count in the original first, then keep its opening tag and children.
        let counted =
            crate::xlsx_tables::attributes(&xml, p, &[("count", Some(count.to_string()))]);
        let end = opening_tag_end(&counted).unwrap();
        if counted[..end].ends_with('/') {
            let tag = counted[1..].split([' ', '/', '>']).next().unwrap();
            format!("{}>{children}</{tag}>", &counted[..end - 1])
        } else {
            format!(
                "{}{children}{}",
                &counted[..=end],
                &counted[counted.rfind("</").unwrap()..]
            )
        }
    } else {
        format!("<dataValidations xmlns=\"{S}\" count=\"{count}\">{children}</dataValidations>")
    };
    let edit = if let Some(p) = parent {
        (p.range(), replacement)
    } else {
        // SpreadsheetML's worksheet sequence places validation after conditional
        // formatting and before hyperlinks, print setup, drawings and extensions.
        let pos = root
            .children()
            .find(|n| {
                n.is_element()
                    && matches!(
                        n.tag_name().name(),
                        "hyperlinks"
                            | "printOptions"
                            | "pageMargins"
                            | "pageSetup"
                            | "headerFooter"
                            | "rowBreaks"
                            | "colBreaks"
                            | "customProperties"
                            | "cellWatches"
                            | "ignoredErrors"
                            | "smartTags"
                            | "drawing"
                            | "legacyDrawing"
                            | "legacyDrawingHF"
                            | "picture"
                            | "oleObjects"
                            | "controls"
                            | "webPublishItems"
                            | "tableParts"
                            | "extLst"
                    )
            })
            .map(|n| n.range().start)
            .unwrap_or(
                root.range().start
                    + raw(&xml, root)
                        .rfind("</")
                        .ok_or("Worksheet closing tag missing")?,
            );
        (pos..pos, replacement)
    };
    pkg.set(path, replace_ranges(&xml, vec![edit]));
    Ok(())
}

/// Compact cell destinations into row spans and then rectangles. Filling a
/// column must not create a rule per cell or enumerate a whole worksheet.
pub(crate) fn compact_cells(cells: &[String]) -> Result<Vec<Area>, String> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut rows: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for cell in cells {
        let (r, c) = parse_address(cell)?;
        rows.entry(r).or_default().insert(c);
    }
    let mut result: Vec<Area> = vec![];
    let mut prior: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for (r, cols) in rows {
        let mut spans = vec![];
        for c in cols {
            if let Some((_, right)) = spans.last_mut() {
                if *right + 1 == c {
                    *right = c;
                    continue;
                }
            }
            spans.push((c, c));
        }
        let mut current = BTreeMap::new();
        for (left, right) in spans {
            let index = if let Some(&i) = prior
                .get(&(left, right))
                .filter(|&&i| result[i].bottom + 1 == r)
            {
                result[i].bottom = r;
                i
            } else {
                result.push(Area {
                    top: r,
                    bottom: r,
                    left,
                    right,
                });
                result.len() - 1
            };
            current.insert((left, right), index);
        }
        prior = current;
    }
    Ok(result)
}

/// Copy rule semantics from an immutable source. Portable source fragments
/// retain unknown attributes and namespace declarations across worksheets.
pub fn copy_rules(
    sheets: &Value,
    index: usize,
    target: &mut Value,
    copies: &[(String, String)],
    pkg: Option<&Package>,
    target_context: Option<(&[String], usize)>,
) -> Result<(), String> {
    let source = &sheets[index];
    if copies.is_empty()
        || (rules(source).next().is_none()
            && rules(target).next().is_none()
            && source["extendedDataValidations"] != true
            && target["extendedDataValidations"] != true)
    {
        return Ok(());
    }
    if source["extendedDataValidations"] == true || target["extendedDataValidations"] == true {
        return Err(
            "Copying extended Excel validation rules needs additional support. Nothing changed."
                .into(),
        );
    }
    let names = sheets
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().into())
        .collect::<Vec<_>>();
    let (target_names, target_index) = target_context.unwrap_or((&names, index));
    let originals = rules(source).collect::<Vec<_>>();
    let mut grouped: std::collections::BTreeMap<usize, Vec<String>> =
        std::collections::BTreeMap::new();
    for (from, to) in copies {
        let (r, c) = parse_address(from)?;
        if let Some(i) = originals
            .iter()
            .position(|rule| areas(rule).is_ok_and(|a| a.iter().any(|a| a.contains(r, c))))
        {
            if originals[i]["unsupported"] == true {
                return Err("Copying this unsupported validation rule needs additional support. Nothing changed.".into());
            }
            grouped.entry(i).or_default().push(to.clone());
        }
    }
    let destinations = compact_cells(&copies.iter().map(|(_, to)| to.clone()).collect::<Vec<_>>())?;
    let mut next = vec![];
    for rule in rules(target) {
        let old = areas(rule)?;
        let mut rest = old.clone();
        for area in &destinations {
            rest = rest.into_iter().flat_map(|a| subtract(a, *area)).collect();
        }
        if old != rest && rule["unsupported"] == true {
            return Err(
                "The destination has an unsupported validation rule. Nothing changed.".into(),
            );
        }
        if rest.is_empty() {
            continue;
        }
        let mut rule = rule.clone();
        reanchor(
            &mut rule,
            target_names,
            target_index,
            &address(old[0].top, old[0].left),
            &address(rest[0].top, rest[0].left),
        )?;
        rule["sqref"] = json!(rest.iter().map(|a| a.text()).collect::<Vec<_>>().join(" "));
        next.push(rule);
    }
    let xml = pkg.and_then(|p| {
        source["officePath"]
            .as_str()
            .and_then(|path| p.text(path).ok())
    });
    let doc = xml.map(parse).transpose()?;
    let nodes: Vec<_> = doc
        .as_ref()
        .and_then(|d| child(d.root_element(), "dataValidations"))
        .into_iter()
        .flat_map(|p| p.children())
        .filter(|n| n.has_tag_name((S, "dataValidation")))
        .collect();
    for (i, cells) in grouped {
        let original = originals[i];
        let old = areas(original)?[0];
        let areas = compact_cells(&cells)?;
        let mut rule = original.clone();
        reanchor(
            &mut rule,
            &names,
            index,
            &address(old.top, old.left),
            &address(areas[0].top, areas[0].left),
        )?;
        rule["sqref"] = json!(areas.iter().map(|a| a.text()).collect::<Vec<_>>().join(" "));
        let anchor = address(areas[0].top, areas[0].left);
        for key in ["formula1", "formula2"] {
            if let Some(formula) = rule[key].as_str().filter(|f| !f.starts_with('"')) {
                if crate::structure::missing_formula_sheet(
                    formula,
                    target_names,
                    target_index,
                    &anchor,
                )? {
                    return Err("A copied validation rule refers to a worksheet missing from this workbook. Nothing changed.".into());
                }
            }
        }
        if let Some(node) = original["officeIndex"]
            .as_u64()
            .and_then(|i| nodes.get(i as usize))
        {
            rule["officeXml"] = json!(scoped_record(xml.unwrap(), *node));
        }
        // A source index has no meaning in another sheet's validation array.
        rule.as_object_mut().unwrap().remove("officeIndex");
        next.push(rule);
    }
    target["dataValidations"] = json!(next);
    validate(target)
}
