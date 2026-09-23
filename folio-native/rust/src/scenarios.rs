//! Named what-if inputs and lossless SpreadsheetML scenario metadata.
use crate::{
    formula::{address, parse_address},
    office::*,
    tables::Area,
    xlsx::fragment_inner,
};
use serde_json::{json, Value};
use std::collections::HashSet;
pub const KEY: &str = "scenarios";
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub fn items(sheet: &Value) -> impl Iterator<Item = &Value> {
    sheet[KEY]["items"].as_array().into_iter().flatten()
}
fn string<'a>(v: &'a Value, key: &str, max: usize) -> Result<&'a str, String> {
    let s = v[key]
        .as_str()
        .ok_or_else(|| format!("Invalid scenario {key}"))?;
    if s.encode_utf16().count() > max {
        return Err(format!("Scenario {key} exceeds {max} characters"));
    }
    Ok(s)
}
fn validate_item(v: &Value) -> Result<(), String> {
    if string(v, "name", 255)?.trim().is_empty() {
        return Err("Enter a scenario name".into());
    }
    for k in ["comment", "user"] {
        if v.get(k).is_some() {
            string(v, k, if k == "user" { 54 } else { 255 })?;
        }
    }
    for k in ["locked", "hidden"] {
        if v.get(k).is_some_and(|x| !x.is_boolean()) {
            return Err("Invalid scenario protection flag".into());
        }
    }
    let inputs = v["inputs"].as_array().ok_or("Invalid scenario inputs")?;
    if inputs.is_empty() || inputs.len() > 32 {
        return Err("A scenario needs 1–32 changing cells".into());
    }
    let mut seen = HashSet::new();
    for input in inputs {
        let a = string(input, "address", 16)?;
        parse_address(a)?;
        if input["deleted"] != true && !seen.insert(a) {
            return Err("Each changing cell must appear only once".into());
        }
        string(input, "value", 255)?;
        if let Some(format) = input.get("_numberFormat") {
            let doc = parse(
                format
                    .as_str()
                    .ok_or("Invalid scenario number format record")?,
            )?;
            if !doc.root_element().has_tag_name((S, "numFmt"))
                || attr(doc.root_element(), "formatCode").is_none()
            {
                return Err("Invalid scenario number format record".into());
            }
        }
        for k in ["deleted", "undone"] {
            if input.get(k).is_some_and(|x| !x.is_boolean()) {
                return Err("Invalid scenario input flag".into());
            }
        }
        if input
            .get("numFmtId")
            .is_some_and(|x| x.as_u64().is_none_or(|n| n > u32::MAX as u64))
        {
            return Err("Invalid scenario number format".into());
        }
    }
    Ok(())
}
pub fn validate(sheet: &Value) -> Result<(), String> {
    if let Some(group) = sheet.get(KEY) {
        let list = group["items"].as_array().ok_or("Invalid scenarios")?;
        let mut names = HashSet::new();
        for v in list {
            if v["unsupported"] == true {
                continue;
            }
            validate_item(v)?;
            if !names.insert(v["name"].as_str().unwrap().to_lowercase()) {
                return Err("Scenario names must be unique on this worksheet".into());
            }
        }
        for k in ["current", "show"] {
            if group
                .get(k)
                .is_some_and(|x| x.as_u64().is_none_or(|n| n >= list.len() as u64))
            {
                return Err("Invalid scenario selection".into());
            }
        }
        if let Some(refs) = group.get("resultCells") {
            addresses(
                refs.as_str().ok_or("Invalid scenario result cells")?,
                100000,
            )?;
        }
    }
    Ok(())
}
pub fn addresses(text: &str, limit: usize) -> Result<Vec<String>, String> {
    let mut out = vec![];
    let mut seen = HashSet::new();
    for range in text
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|s| !s.is_empty())
    {
        let a = Area::parse(range)?;
        if (a.bottom - a.top + 1) * (a.right - a.left + 1) > limit {
            return Err(format!("Choose at most {limit} cells"));
        }
        for r in a.top..=a.bottom {
            for c in a.left..=a.right {
                let s = address(r, c);
                if seen.insert(s.clone()) {
                    out.push(s);
                }
            }
        }
        if out.len() > limit {
            return Err(format!("Choose at most {limit} cells"));
        }
    }
    if out.is_empty() {
        return Err("Choose at least one cell".into());
    }
    Ok(out)
}
pub fn sheet_index(w: &Value, r: &Value) -> Result<usize, String> {
    w["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["id"] == r["sheetId"])
        .ok_or("Worksheet missing".into())
}
pub fn selected<'a>(sheet: &'a Value, r: &Value) -> Result<&'a Value, String> {
    let i = r["index"].as_u64().ok_or("Choose a scenario")? as usize;
    let v = sheet[KEY]["items"].get(i).ok_or("Scenario missing")?;
    if v["unsupported"] == true {
        return Err("This imported scenario contains unsupported input metadata".into());
    }
    Ok(v)
}
pub fn check_sheet(sheet: &Value, pkg: Option<&Package>) -> Result<(), String> {
    if let Some((p, path)) = pkg.zip(sheet["officePath"].as_str()) {
        let d = parse(p.text(path)?)?;
        if child(d.root_element(), "sheetProtection").is_some() {
            return Err("Scenario editing on protected worksheets needs additional support".into());
        }
    }
    if sheet[KEY]["unsupported"] == true {
        return Err("This worksheet has unsupported scenario collections".into());
    }
    Ok(())
}
pub fn change(w: &Value, r: &Value, pkg: Option<&Package>) -> Result<Value, String> {
    let si = sheet_index(w, r)?;
    check_sheet(&w["sheets"][si], pkg)?;
    let mut next = w.clone();
    let sheet = &mut next["sheets"][si];
    if sheet.get(KEY).is_none() {
        sheet[KEY] = json!({"items":[]});
    }
    match r["action"].as_str() {
        Some("add" | "edit") => {
            let edit = r["action"] == "edit";
            let old = if edit {
                Some(selected(sheet, r)?.clone())
            } else {
                None
            };
            let mut v = old
                .clone()
                .unwrap_or_else(|| json!({"locked":true,"hidden":false}));
            v["name"] = r["name"].clone();
            v["comment"] = r["comment"].as_str().unwrap_or("").into();
            if r.get("user").is_some() {
                v["user"] = r["user"].clone();
            }
            for key in ["locked", "hidden"] {
                if r.get(key).is_some() {
                    v[key] = r[key].clone();
                }
            }
            let requested = r["inputs"]
                .as_array()
                .ok_or("Enter the changing cells and values")?;
            let mut inputs = vec![];
            for input in requested {
                let a = input["address"]
                    .as_str()
                    .ok_or("Enter a changing cell")?
                    .trim()
                    .replace('$', "")
                    .to_uppercase();
                parse_address(&a)?;
                let prior = old
                    .as_ref()
                    .and_then(|v| v["inputs"].as_array())
                    .and_then(|v| v.iter().find(|v| v["address"] == a));
                let mut entry = prior.cloned().unwrap_or_else(|| json!({}));
                entry["address"] = json!(a);
                entry["value"] = input["value"].clone();
                inputs.push(entry);
            }
            v["inputs"] = json!(inputs);
            validate_item(&v)?;
            let list = sheet[KEY]["items"].as_array_mut().unwrap();
            if edit {
                list[r["index"].as_u64().unwrap() as usize] = v;
            } else {
                list.push(v);
            }
        }
        Some("delete") => {
            selected(sheet, r)?;
            let i = r["index"].as_u64().unwrap() as usize;
            sheet[KEY]["items"].as_array_mut().unwrap().remove(i);
            for k in ["current", "show"] {
                if let Some(n) = sheet[KEY][k].as_u64() {
                    if n as usize == i {
                        sheet[KEY].as_object_mut().unwrap().remove(k);
                    } else if n as usize > i {
                        sheet[KEY][k] = json!(n - 1);
                    }
                }
            }
        }
        Some("merge") => {
            let source = w["sheets"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["id"] == r["sourceSheetId"])
                .ok_or("Source worksheet missing")?;
            check_sheet(source, pkg)?;
            let selected = r["indices"]
                .as_array()
                .ok_or("Choose scenarios to copy")?
                .iter()
                .map(|index| selected(source, &json!({"index":index})).cloned())
                .collect::<Result<Vec<_>, _>>()?;
            return merge_items(w, r, pkg, selected);
        }
        _ => return Err("Choose Add, Edit, Delete or Merge scenarios".into()),
    }
    validate(sheet)?;
    Ok(next)
}
/// Copy definitions as one transaction without applying their input values.
pub fn merge_items(
    w: &Value,
    r: &Value,
    pkg: Option<&Package>,
    selected: Vec<Value>,
) -> Result<Value, String> {
    let si = sheet_index(w, r)?;
    check_sheet(&w["sheets"][si], pkg)?;
    if selected.is_empty() {
        return Err("Choose scenarios to copy".into());
    }
    let mut next = w.clone();
    let sheet = &mut next["sheets"][si];
    if sheet.get(KEY).is_none() {
        sheet[KEY] = json!({"items":[]});
    }
    let mut names: HashSet<_> = items(sheet)
        .filter_map(|s| s["name"].as_str().map(str::to_lowercase))
        .collect();
    for mut item in selected {
        validate_item(&item)?;
        let name = item["name"].as_str().unwrap();
        let mut unique = name.to_string();
        let mut suffix = 2;
        while names.contains(&unique.to_lowercase()) {
            let tail = format!(" ({suffix})");
            let mut units = 0;
            let prefix: String = name
                .chars()
                .take_while(|c| {
                    units += c.len_utf16();
                    units <= 255 - tail.len()
                })
                .collect();
            unique = format!("{prefix}{tail}");
            suffix += 1;
        }
        names.insert(unique.to_lowercase());
        item["name"] = json!(unique);
        sheet[KEY]["items"].as_array_mut().unwrap().push(item);
    }
    validate(sheet)?;
    Ok(next)
}
// Preserve namespaced extension attributes/children while replacing only known
// unqualified attributes. Values are already XML-escaped by callers.
fn element(
    template: Option<&str>,
    tag: &str,
    attrs: &[(&str, Option<String>)],
    inner: Option<&str>,
) -> Result<String, String> {
    let empty = format!("<{tag} xmlns=\"{S}\"/>");
    let xml = template.unwrap_or(&empty);
    let d = parse(xml)?;
    let n = d.root_element();
    if !n.has_tag_name((S, tag)) {
        return Err("Invalid scenario XML template".into());
    }
    let full = standalone_xml(xml, n);
    let d = parse(&full)?;
    let n = d.root_element();
    let mut edits = vec![];
    let mut add = String::new();
    for (key, value) in attrs {
        if let Some(a) = n
            .attributes()
            .find(|a| a.namespace().is_none() && a.name() == *key)
        {
            edits.push((
                a.range(),
                value
                    .as_ref()
                    .map(|v| format!("{key}=\"{v}\""))
                    .unwrap_or_default(),
            ));
        } else if let Some(value) = value {
            add.push_str(&format!(" {key}=\"{value}\""));
        }
    }
    let at = full.find('>').ok_or("Invalid scenario XML")?;
    edits.push((at..at, add));
    let result = replace_ranges(&full, edits);
    if let Some(inner) = inner {
        fragment_inner(&result, inner)
    } else {
        Ok(result)
    }
}
fn shell(xml: &str, n: roxmltree::Node, child_tag: &str) -> Result<String, String> {
    let full = standalone_xml(xml, n);
    let d = parse(&full)?;
    let inner = d
        .root_element()
        .children()
        .filter(|n| !n.has_tag_name((S, child_tag)))
        .map(|n| raw(&full, n))
        .collect::<String>();
    fragment_inner(&full, &inner)
}
pub fn read(pkg: &Package, path: &str, sheet: &mut Value) -> Result<(), String> {
    let xml = pkg.text(path)?;
    let d = parse(xml)?;
    let groups = d
        .root_element()
        .children()
        .filter(|n| n.has_tag_name((S, KEY)))
        .collect::<Vec<_>>();
    if groups.is_empty() {
        return Ok(());
    }
    let group = groups[0];
    let mut list = vec![];
    for n in group.children().filter(|n| n.has_tag_name((S, "scenario"))) {
        let mut v = json!({"name":crate::xlsx_strings::decode(attr(n,"name").unwrap_or("")),"inputs":[],"_xml":shell(xml,n,"inputCells")?});
        for k in ["comment", "user"] {
            if let Some(s) = attr(n, k) {
                v[k] = json!(crate::xlsx_strings::decode(s));
            }
        }
        for k in ["locked", "hidden"] {
            if let Some(s) = attr(n, k) {
                v[k] = json!(matches!(s, "true" | "1"));
            }
        }
        for input in n.children().filter(|n| n.has_tag_name((S, "inputCells"))) {
            let mut value = json!({"address":attr(input,"r").unwrap_or(""),"value":crate::xlsx_strings::decode(attr(input,"val").unwrap_or("")),"_xml":standalone_xml(xml,input)});
            for k in ["deleted", "undone"] {
                if let Some(s) = attr(input, k) {
                    value[k] = json!(matches!(s, "true" | "1"));
                }
            }
            if let Some(s) = attr(input, "numFmtId") {
                value["numFmtId"] = s.parse::<u32>().map(|n| json!(n)).unwrap_or(json!(s));
            }
            v["inputs"].as_array_mut().unwrap().push(value);
        }
        if validate_item(&v).is_err() {
            v["unsupported"] = json!(true);
            v["_xml"] = json!(standalone_xml(xml, n));
        }
        list.push(v);
    }
    let mut result = json!({"items":list,"_xml":shell(xml,group,"scenario")?});
    for k in ["current", "show"] {
        if let Some(n) = attr(group, k)
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n < list.len())
        {
            result[k] = json!(n);
        }
    }
    if let Some(s) = attr(group, "sqref") {
        if addresses(s, 100000).is_ok() {
            result["resultCells"] = json!(s);
        }
    }
    if groups.len() > 1 {
        result["unsupported"] = json!(true);
    }
    sheet[KEY] = result;
    if validate(sheet).is_err() {
        sheet[KEY]["unsupported"] = json!(true);
        for item in sheet[KEY]["items"].as_array_mut().unwrap() {
            item["unsupported"] = json!(true);
        }
    }
    Ok(())
}
fn inner_template(v: &Value) -> Result<String, String> {
    if let Some(xml) = v["_xml"].as_str() {
        let d = parse(xml)?;
        Ok(d.root_element().children().map(|n| raw(xml, n)).collect())
    } else {
        Ok(String::new())
    }
}
fn item_xml(v: &Value) -> Result<String, String> {
    if v["unsupported"] == true {
        return Ok(v["_xml"]
            .as_str()
            .ok_or("Missing preserved scenario")?
            .into());
    }
    let mut inner = inner_template(v)?;
    for input in v["inputs"].as_array().unwrap() {
        let mut attrs = vec![
            ("r", Some(esc(input["address"].as_str().unwrap()))),
            (
                "val",
                Some(crate::xlsx_strings::xml(
                    input["value"].as_str().unwrap(),
                    true,
                )),
            ),
        ];
        for k in ["deleted", "undone", "numFmtId"] {
            attrs.push((
                k,
                input.get(k).map(|v| {
                    if let Some(b) = v.as_bool() {
                        if b {
                            "1".into()
                        } else {
                            "0".into()
                        }
                    } else {
                        v.to_string()
                    }
                }),
            ));
        }
        inner.push_str(&element(
            input["_xml"].as_str(),
            "inputCells",
            &attrs,
            None,
        )?);
    }
    let mut attrs = vec![(
        "count",
        Some(v["inputs"].as_array().unwrap().len().to_string()),
    )];
    for k in ["name", "comment", "user"] {
        attrs.push((k, v[k].as_str().map(|s| crate::xlsx_strings::xml(s, true))));
    }
    for k in ["locked", "hidden"] {
        attrs.push((
            k,
            v[k].as_bool()
                .map(|b| if b { "1".into() } else { "0".into() }),
        ));
    }
    element(v["_xml"].as_str(), "scenario", &attrs, Some(&inner))
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
    let xml = pkg.text(path)?.to_string();
    let d = parse(&xml)?;
    let root = d.root_element();
    let existing = root
        .children()
        .filter(|n| n.has_tag_name((S, KEY)))
        .collect::<Vec<_>>();
    if existing.len() > 1 || sheet[KEY]["unsupported"] == true {
        return Err("This scenario collection needs additional file support".into());
    }
    let content = if items(sheet).next().is_none() {
        String::new()
    } else {
        let mut inner = inner_template(&sheet[KEY])?;
        for item in items(sheet) {
            let mut item = item.clone();
            if let Some(inputs) = item["inputs"].as_array_mut() {
                for input in inputs {
                    if let Some(format) = input["_numberFormat"].as_str() {
                        input["numFmtId"] = json!(crate::xlsx_styles::import_number_format_record(
                            pkg, format
                        )?);
                    }
                }
            }
            inner.push_str(&item_xml(&item)?);
        }
        let attrs = vec![
            (
                "current",
                sheet[KEY]["current"].as_u64().map(|n| n.to_string()),
            ),
            ("show", sheet[KEY]["show"].as_u64().map(|n| n.to_string())),
            ("sqref", sheet[KEY]["resultCells"].as_str().map(esc)),
        ];
        element(sheet[KEY]["_xml"].as_str(), KEY, &attrs, Some(&inner))?
    };
    if let Some(group) = existing.first() {
        pkg.set(path, replace_ranges(&xml, vec![(group.range(), content)]));
    } else if !content.is_empty() {
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
        ];
        let at = root
            .children()
            .find(|n| n.is_element() && !early.contains(&n.tag_name().name()))
            .map(|n| n.range().start)
            .or_else(|| xml.rfind("</"));
        let edit = if let Some(at) = at {
            (at..at, content)
        } else {
            (root.range(), fragment_inner(raw(&xml, root), &content)?)
        };
        pkg.set(path, replace_ranges(&xml, vec![edit]));
    }
    Ok(())
}
pub fn transform(sheet: &mut Value, change: &crate::structure::Change) -> Result<(), String> {
    if let Some(group) = sheet.get_mut(KEY) {
        for item in group["items"].as_array_mut().unwrap() {
            if item["unsupported"] == true {
                return Err("Cannot move unsupported scenario inputs".into());
            }
            for input in item["inputs"].as_array_mut().unwrap() {
                if input["undone"] == true || input["deleted"] == true {
                    continue;
                }
                if let Some(a) = change.cell(input["address"].as_str().unwrap())? {
                    input["address"] = json!(a);
                } else {
                    input["deleted"] = json!(true);
                }
            }
        }
        if let Some(s) = group["resultCells"].as_str() {
            let refs = s
                .split_whitespace()
                .map(|a| change.range(a))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ");
            if refs.is_empty() {
                group.as_object_mut().unwrap().remove("resultCells");
            } else {
                group["resultCells"] = json!(refs);
            }
        }
    }
    Ok(())
}
pub fn transform_input(
    xml: &str,
    n: roxmltree::Node,
    change: &crate::structure::Change,
) -> Result<String, String> {
    if matches!(attr(n, "deleted"), Some("1" | "true"))
        || matches!(attr(n, "undone"), Some("1" | "true"))
    {
        return Ok(raw(xml, n).into());
    }
    let original = standalone_xml(xml, n);
    let a = attr(n, "r").ok_or("Missing scenario input address")?;
    let attrs = if let Some(to) = change.cell(a)? {
        vec![("r", Some(esc(&to)))]
    } else {
        vec![("deleted", Some("1".into()))]
    };
    element(Some(&original), "inputCells", &attrs, None)
}
