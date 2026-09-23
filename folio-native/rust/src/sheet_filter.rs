//! Worksheet AutoFilter. Criteria and visibility are snapshots, reapplied explicitly.
use crate::{
    formula::address,
    office::*,
    tables::{self, Area},
    xlsx_tables::{attributes, filter_xml, inner, read_filters},
};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

pub fn validate(sheet: &Value) -> Result<(), String> {
    let Some(filter) = sheet.get("autoFilter").filter(|v| !v.is_null()) else {
        return Ok(());
    };
    let a = tables::area(filter)?;
    if a.top == a.bottom {
        return Err("A filter range needs a heading and at least one data row".into());
    }
    if tables::list(sheet).any(|t| tables::area(t).is_ok_and(|b| b.overlaps(a))) {
        return Err("A worksheet filter cannot overlap a table. Use the table's filters.".into());
    }
    for (col, rule) in filter["filters"]
        .as_object()
        .ok_or("Invalid worksheet filters")?
    {
        if col.parse::<usize>().is_err() || col.parse::<usize>().unwrap() > a.right - a.left {
            return Err("Invalid worksheet filter column".into());
        }
        if rule["kind"] != "unsupported" {
            tables::validate_filter(rule)?;
        }
    }
    for r in filter["filteredRows"]
        .as_array()
        .ok_or("Invalid filtered rows")?
    {
        if r.as_u64()
            .is_none_or(|r| r <= a.top as u64 + 1 || r > a.bottom as u64 + 1)
        {
            return Err("Invalid worksheet filtered row".into());
        }
    }
    Ok(())
}

fn apply(filter: &mut Value, values: &Value) -> Result<(), String> {
    let a = tables::area(filter)?;
    if a.bottom - a.top > 100000 {
        return Err("Filtering currently supports up to 100,000 data rows".into());
    }
    let mut hidden = vec![];
    for row in a.top + 1..=a.bottom {
        for (col, rule) in filter["filters"]
            .as_object()
            .ok_or("Invalid worksheet filters")?
        {
            let column = col.parse::<usize>().map_err(|_| "Invalid filter column")?;
            if !tables::matches_filter(&values[address(row, a.left + column)], rule)? {
                hidden.push(row + 1);
                break;
            }
        }
    }
    filter["filteredRows"] = json!(hidden);
    Ok(())
}

pub fn change(workspace: &Value, calculated: &Value, request: &Value) -> Result<Value, String> {
    let si = workspace["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["id"] == request["sheetId"])
        .ok_or("Worksheet missing")?;
    let mut next = workspace.clone();
    let sheet = &mut next["sheets"][si];
    match request["action"].as_str().ok_or("Choose a filter action")? {
        "enable" => {
            if sheet.get("autoFilter").is_some_and(|v| !v.is_null()) {
                return Err(
                    "Remove the existing worksheet filter before selecting another range".into(),
                );
            }
            let a = Area::parse(
                request["range"]
                    .as_str()
                    .ok_or("Choose a filter range")?
                    .trim(),
            )?;
            if a.bottom - a.top > 100000 {
                return Err("Filtering currently supports up to 100,000 data rows".into());
            }
            sheet["autoFilter"] = json!({"range":a.text(),"filters":{},"filteredRows":[]});
        }
        "remove" => {
            sheet["autoFilter"] = Value::Null;
        }
        action @ ("filter" | "reapply" | "clearFilters") => {
            let filter = &mut sheet["autoFilter"];
            let a = tables::area(filter).map_err(|_| "Enable a worksheet filter first")?;
            if action == "clearFilters" {
                filter["filters"] = json!({});
            }
            if action == "filter" {
                let col = request["column"]
                    .as_u64()
                    .filter(|c| *c <= (a.right - a.left) as u64)
                    .ok_or("Choose a filter column")?
                    .to_string();
                if request["rule"].is_null() {
                    filter["filters"]
                        .as_object_mut()
                        .ok_or("Invalid filters")?
                        .remove(&col);
                } else {
                    tables::validate_filter(&request["rule"])?;
                    filter["filters"][col] = request["rule"].clone();
                }
            }
            apply(
                filter,
                &calculated[workspace["sheets"][si]["id"].as_str().unwrap()],
            )?;
        }
        _ => return Err("Unknown worksheet filter action".into()),
    }
    tables::validate(&next["sheets"])?;
    Ok(next)
}

pub fn structural(
    before: &Value,
    next: &mut Value,
    change: &crate::structure::Change,
    date1904: bool,
) -> Result<(), String> {
    let crate::structure::Edit::Axis { rows, .. } = change.edit else {
        return Ok(());
    };
    let si = change.sheet;
    let Some(old) = before["sheets"][si]
        .get("autoFilter")
        .filter(|v| !v.is_null())
    else {
        return Ok(());
    };
    let a = tables::area(old)?;
    let Some(range) = change.range(&a.text())? else {
        next["sheets"][si]["autoFilter"] = Value::Null;
        return Ok(());
    };
    let b = Area::parse(&range)?;
    if b.top == b.bottom || (rows && change.point(a.top as i32 + 1).is_none()) {
        next["sheets"][si]["autoFilter"] = Value::Null;
        return Ok(());
    }
    let mut updated = old.clone();
    updated["range"] = json!(range);
    if rows {
        updated["filteredRows"] = json!(old["filteredRows"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|r| change.point(r.as_i64().unwrap() as i32))
            .collect::<Vec<_>>());
    } else {
        let mut filters = json!({});
        for (col, rule) in old["filters"].as_object().unwrap() {
            if let Some(c) = change.point((a.left + col.parse::<usize>().unwrap() + 1) as i32) {
                let index = c as usize - 1 - b.left;
                let mut rule = rule.clone();
                if rule["kind"] == "unsupported" {
                    let xml = rule["officeXml"]
                        .as_str()
                        .ok_or("Missing retained filter")?;
                    let d = parse(xml)?;
                    rule["officeXml"] = json!(attributes(
                        xml,
                        d.root_element(),
                        &[("colId", Some(index.to_string()))]
                    ));
                }
                filters[index.to_string()] = rule;
            }
        }
        updated["filters"] = filters;
        // Removing a criterion needs fresh results; surviving unsupported rules
        // cannot be silently interpreted as matching every record.
        if updated["filters"] != old["filters"] {
            let mut calc = crate::calculation::WorkbookCalculator::default();
            calc.sync_calendar(&next["sheets"], date1904);
            if let Some(e) = calc.error {
                return Err(e);
            }
            apply(
                &mut updated,
                &calc.values[next["sheets"][si]["id"].as_str().unwrap()],
            )?;
        }
    }
    next["sheets"][si]["autoFilter"] = updated;
    Ok(())
}

pub fn read(pkg: &Package, path: &str, sheet: &mut Value) -> Result<(), String> {
    let xml = pkg.text(path)?;
    let d = parse(xml)?;
    // Absence is explicit, so later exports can remove filters intentionally.
    sheet["autoFilter"] = Value::Null;
    if let Some(n) = child(d.root_element(), "autoFilter") {
        let a = Area::parse(attr(n, "ref").ok_or("Missing worksheet filter range")?)?;
        let filters = read_filters(xml, n);
        let hidden: Vec<_> = if filters.as_object().is_some_and(|f| !f.is_empty()) {
            sheet["hiddenRows"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_u64)
                .filter(|r| *r > a.top as u64 + 1 && *r <= a.bottom as u64 + 1)
                .collect()
        } else {
            vec![]
        };
        let filtered: HashSet<_> = hidden.iter().copied().collect();
        if let Some(rows) = sheet["hiddenRows"].as_array_mut() {
            rows.retain(|r| !filtered.contains(&r.as_u64().unwrap_or(0)));
        }
        // Some Excel sources retain a header-only range. Preserve it in the
        // package without projecting editable state until it has data.
        if a.top != a.bottom {
            sheet["autoFilter"] = json!({"range":a.text(),"filters":filters,"filteredRows":hidden});
        } else {
            sheet.as_object_mut().unwrap().remove("autoFilter");
        }
    }
    restore_visibility(xml, sheet)?;
    Ok(())
}

/// Insert an ordered worksheet child without disturbing unrelated XML.
fn set_filter(xml: &str, content: String) -> Result<String, String> {
    let d = parse(xml)?;
    let root = d.root_element();
    let change = if let Some(n) = child(root, "autoFilter") {
        (n.range(), content)
    } else {
        let early = [
            "sheetPr",
            "dimension",
            "sheetViews",
            "sheetFormatPr",
            "cols",
            "sheetData",
            "sheetCalcPr",
            "sheetProtection",
            "protectedRanges",
            "scenarios",
        ];
        let at = root
            .children()
            .find(|n| n.is_element() && !early.contains(&n.tag_name().name()))
            .map(|n| n.range().start)
            .unwrap_or_else(|| xml.rfind("</").unwrap());
        (at..at, content)
    };
    Ok(replace_ranges(xml, vec![change]))
}

pub fn write(pkg: &mut Package, sheet: &Value, path: &str) -> Result<(), String> {
    if sheet.get("autoFilter").is_some() {
        let xml = pkg.text(path)?.to_string();
        let d = parse(&xml)?;
        let f = &sheet["autoFilter"];
        let content = if f.is_null() {
            String::new()
        } else {
            let new_xml = filter_xml(f)?;
            let nd = parse(&new_xml)?;
            if let Some(old) = child(d.root_element(), "autoFilter") {
                let cols = nd
                    .root_element()
                    .children()
                    .map(|n| raw(&new_xml, n))
                    .collect::<String>();
                let retained = old
                    .children()
                    .filter(|n| {
                        n.is_element()
                            && n.tag_name().name() != "filterColumn"
                            && !(n.tag_name().name() == "sortState"
                                && sheet.get("rangeSort").is_some())
                    })
                    .map(|n| raw(&xml, n))
                    .collect::<String>();
                let base = attributes(&xml, old, &[("ref", Some(tables::area(f)?.text()))]);
                crate::xlsx::fragment_inner(&base, &(cols + &retained))?
            } else {
                new_xml
            }
        };
        pkg.set(path, set_filter(&xml, content)?);
        write_defined_name(pkg, sheet)?;
    }
    // filterMode reflects active criteria, including table filters.
    let active = tables::list(sheet)
        .chain(sheet.get("autoFilter").filter(|v| !v.is_null()))
        .any(|f| f["filters"].as_object().is_some_and(|f| !f.is_empty()));
    let xml = pkg.text(path)?.to_string();
    let d = parse(&xml)?;
    let root = d.root_element();
    let change = if let Some(pr) = child(root, "sheetPr") {
        (
            pr.range(),
            attributes(&xml, pr, &[("filterMode", active.then(|| "1".into()))]),
        )
    } else {
        let at = root
            .children()
            .find(|n| n.is_element())
            .map(|n| n.range().start)
            .unwrap_or_else(|| xml.rfind("</").unwrap());
        (
            at..at,
            if active {
                format!("<sheetPr xmlns=\"{S}\" filterMode=\"1\"/>")
            } else {
                String::new()
            },
        )
    };
    pkg.set(path, replace_ranges(&xml, vec![change]));
    write_visibility(pkg, sheet, path)
}

fn write_defined_name(pkg: &mut Package, sheet: &Value) -> Result<(), String> {
    let xml = pkg.text("xl/workbook.xml")?.to_string();
    let d = parse(&xml)?;
    let root = d.root_element();
    let si = child(root, "sheets")
        .ok_or("Missing worksheets")?
        .children()
        .filter(|n| n.is_element())
        .position(|n| attr(n, "name") == sheet["name"].as_str())
        .ok_or("Worksheet missing")?;
    let si_text = si.to_string();
    let old = child(root, "definedNames");
    let mut contents = old
        .into_iter()
        .flat_map(|n| n.children())
        .filter(|n| {
            !(n.is_element()
                && attr(*n, "name") == Some("_xlnm._FilterDatabase")
                && attr(*n, "localSheetId") == Some(si_text.as_str()))
        })
        .map(|n| raw(&xml, n))
        .collect::<String>();
    if !sheet["autoFilter"].is_null() {
        let a = tables::area(&sheet["autoFilter"])?;
        let abs = |r, c| {
            let cell = address(r, c);
            let at = cell.find(|c: char| c.is_ascii_digit()).unwrap();
            format!("${}${}", &cell[..at], &cell[at..])
        };
        let name = sheet["name"].as_str().unwrap().replace('\'', "''");
        contents.push_str(&format!("<definedName name=\"_xlnm._FilterDatabase\" localSheetId=\"{si}\" hidden=\"1\">{}</definedName>",esc(&format!("'{name}'!{}:{}",abs(a.top,a.left),abs(a.bottom,a.right)))));
    }
    if old.is_none() && contents.is_empty() {
        return Ok(());
    }
    let content = format!("<definedNames xmlns=\"{S}\">{contents}</definedNames>");
    let change = if let Some(old) = old {
        (old.range(), content)
    } else {
        let early = [
            "fileVersion",
            "fileSharing",
            "workbookPr",
            "workbookProtection",
            "bookViews",
            "sheets",
            "functionGroups",
            "externalReferences",
        ];
        let at = root
            .children()
            .find(|n| n.is_element() && !early.contains(&n.tag_name().name()))
            .map(|n| n.range().start)
            .unwrap_or_else(|| xml.rfind("</").unwrap());
        (at..at, content)
    };
    pkg.set("xl/workbook.xml", replace_ranges(&xml, vec![change]));
    Ok(())
}

// OOXML row.hidden does not record whether a user or a filter hid a row. A
// private extension preserves that distinction in Folio exports. Excel can
// ignore it; the standard autoFilter/row.hidden fields remain authoritative.
pub(crate) const VIS_URI: &str = "{F3E5A4B6-6332-473E-8BD0-7B9D07BDE231}";
const VIS_NS: &str = "urn:folio:worksheet-visibility:1";
fn descriptor(f: &Value) -> Value {
    if f.is_null() {
        return Value::Null;
    }
    // Normalize text convenience operators to their standard OOXML encoding.
    let xml = filter_xml(f).unwrap_or_default();
    let rules = parse(&xml)
        .map(|d| read_filters(&xml, d.root_element()))
        .unwrap_or(Value::Null);
    json!({"name":f.get("name"),"range":f["range"],"filters":rules})
}
fn visibility_state(sheet: &Value) -> Value {
    json!({"manual":sheet.get("hiddenRows").cloned().unwrap_or(json!([])),"range":{"descriptor":descriptor(&sheet["autoFilter"]),"rows":sheet["autoFilter"]["filteredRows"]},"tables":tables::list(sheet).map(|t|json!({"descriptor":descriptor(t),"rows":t["filteredRows"]})).collect::<Vec<_>>()})
}
fn write_visibility(pkg: &mut Package, sheet: &Value, path: &str) -> Result<(), String> {
    let xml = pkg.text(path)?.to_string();
    let d = parse(&xml)?;
    let root = d.root_element();
    let ext = format!(
        "<ext xmlns=\"{S}\" uri=\"{VIS_URI}\"><visibility xmlns=\"{VIS_NS}\">{}</visibility></ext>",
        esc(&visibility_state(sheet).to_string())
    );
    let existing = child(root, "extLst");
    let contents = existing
        .into_iter()
        .flat_map(|n| n.children())
        .filter(|n| attr(*n, "uri") != Some(VIS_URI))
        .map(|n| raw(&xml, n))
        .collect::<String>()
        + &ext;
    let fragment = if let Some(n) = existing {
        inner(&xml, n, &contents)
    } else {
        format!("<extLst xmlns=\"{S}\">{contents}</extLst>")
    };
    pkg.set(
        path,
        crate::xlsx_tables::replace_child(&xml, "extLst", &fragment)?,
    );
    Ok(())
}
fn restore_visibility(xml: &str, sheet: &mut Value) -> Result<(), String> {
    let d = parse(xml)?;
    let Some(node) = d
        .descendants()
        .find(|n| n.has_tag_name((VIS_NS, "visibility")))
    else {
        return Ok(());
    };
    let Ok(saved) = serde_json::from_str::<Value>(node.text().unwrap_or("")) else {
        return Ok(());
    };
    let current = visibility_state(sheet);
    let same = saved["range"]["descriptor"] == current["range"]["descriptor"]
        && saved["tables"]
            .as_array()
            .zip(current["tables"].as_array())
            .is_some_and(|(a, b)| {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|(a, b)| a["descriptor"] == b["descriptor"])
            });
    if !same {
        return Ok(());
    }
    let mut restored = sheet.clone();
    restored["hiddenRows"] = saved["manual"].clone();
    if !restored["autoFilter"].is_null() {
        restored["autoFilter"]["filteredRows"] = saved["range"]["rows"].clone();
    }
    for (i, t) in restored["tables"]
        .as_array_mut()
        .into_iter()
        .flatten()
        .enumerate()
    {
        t["filteredRows"] = saved["tables"][i]["rows"].clone();
    }
    let raw_hidden: BTreeSet<_> = d
        .descendants()
        .filter(|n| n.has_tag_name((S, "row")) && matches!(attr(*n, "hidden"), Some("1" | "true")))
        .filter_map(|n| attr(n, "r").and_then(|r| r.parse::<usize>().ok()))
        .collect();
    if tables::validate(&json!([restored.clone()])).is_ok()
        && tables::hidden_rows(&restored)
            .into_iter()
            .collect::<BTreeSet<_>>()
            == raw_hidden
    {
        *sheet = restored;
    }
    Ok(())
}

/// Choices honor the other column criteria, allowing this column's criterion
/// to be broadened. No visibility or workbook mutation occurs here.
pub fn choices(
    workspace: &Value,
    calculated: &Value,
    formatted: &Value,
    r: &Value,
) -> Result<Value, String> {
    let sheet = workspace["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == r["sheetId"])
        .ok_or("Worksheet missing")?;
    let f = if r.get("tableId").is_some() {
        tables::list(sheet)
            .find(|t| t["id"] == r["tableId"])
            .ok_or("Table missing")?
    } else {
        &sheet["autoFilter"]
    };
    let a = tables::area(f)?;
    let col = r["column"]
        .as_u64()
        .filter(|c| *c <= (a.right - a.left) as u64)
        .ok_or("Choose a filter column")? as usize;
    if a.bottom - a.top > 100000 {
        return Err("Filter choices currently support up to 100,000 data rows".into());
    }
    let values = &calculated[sheet["id"].as_str().unwrap()];
    let mut unique = std::collections::BTreeMap::new();
    let mut truncated = false;
    for row in a.top + f["headerRows"].as_u64().unwrap_or(1) as usize
        ..=a.bottom - f["totalsRows"].as_u64().unwrap_or(0) as usize
    {
        let mut visible = true;
        for (c, rule) in f["filters"].as_object().ok_or("Invalid filters")? {
            let c = c.parse::<usize>().map_err(|_| "Invalid filter column")?;
            if c != col && !tables::matches_filter(&values[address(row, a.left + c)], rule)? {
                visible = false;
                break;
            }
        }
        if !visible {
            continue;
        }
        let value = &values[address(row, a.left + col)];
        let text = if value.is_null() {
            String::new()
        } else {
            value
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| value.to_string())
        };
        let key = text.to_lowercase();
        if unique.contains_key(&key) {
            continue;
        }
        if unique.len() == 10000 {
            truncated = true;
            continue;
        }
        let address = address(row, a.left + col);
        let display = formatted[sheet["id"].as_str().unwrap()][&address]["text"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| crate::number_format::plain(value));
        unique.insert(
            key,
            json!({"key":text,"value":value,"display":display,"sourceAddress":address}),
        );
    }
    Ok(json!({"values":unique.into_values().collect::<Vec<_>>(),"truncated":truncated}))
}

pub(crate) fn is_visibility_extension(node: roxmltree::Node) -> bool {
    node.has_tag_name((S, "ext"))
        && attr(node, "uri") == Some(VIS_URI)
        && node.children().filter(|n| n.is_element()).count() == 1
        && node.children().filter(|n| n.is_element()).all(|n| {
            n.has_tag_name((VIS_NS, "visibility")) && !n.children().any(|n| n.is_element())
        })
}
