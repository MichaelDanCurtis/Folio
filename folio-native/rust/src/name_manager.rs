//! The Name Manager (X09.05, X09.04): create, edit and delete defined names,
//! including named LAMBDAs. Names live in the workbook package's
//! `definedNames`, which structural edits already keep current; a workbook
//! created in Folio gets a package on its first name.
use crate::office::{attr, child, parse, Package};
use crate::xlsx_names::Name;
use serde_json::{json, Value};

const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// One Name Manager change. `old` names the defined name an edit or
/// deletion applies to: its name and scope (a workspace sheet index).
pub struct NameEdit {
    pub old: Option<(String, Option<usize>)>,
    /// The new name, scope and formula (without `=`); None deletes.
    pub new: Option<Name>,
    /// The active sheet, whose name qualifies unqualified references.
    pub sheet: usize,
}

impl NameEdit {
    pub fn from_request(r: &Value, sheets: &Value) -> Result<Self, String> {
        let scope = |id: &Value| -> Result<Option<usize>, String> {
            match id {
                Value::Null => Ok(None),
                id => sheets
                    .as_array()
                    .and_then(|s| s.iter().position(|s| s["id"] == *id))
                    .map(Some)
                    .ok_or_else(|| "That worksheet no longer exists.".to_string()),
            }
        };
        let action = r["action"].as_str().unwrap_or("");
        let old = match action {
            "edit" | "delete" => Some((
                r["oldName"].as_str().ok_or("Choose a name")?.to_string(),
                scope(&r["oldSheetId"])?,
            )),
            "add" => None,
            _ => return Err("Unknown Name Manager action".into()),
        };
        let new = match action {
            "delete" => None,
            _ => {
                let name = r["name"].as_str().unwrap_or("").trim().to_string();
                let formula = r["refersTo"].as_str().unwrap_or("").trim();
                let formula = formula
                    .strip_prefix('=')
                    .unwrap_or(formula)
                    .trim()
                    .to_string();
                if name.is_empty() {
                    return Err("Enter a name.".into());
                }
                if formula.is_empty() {
                    return Err("Enter what the name refers to.".into());
                }
                Some((name, scope(&r["sheetId"])?, formula))
            }
        };
        let sheet = scope(&r["activeSheetId"])
            .ok()
            .flatten()
            .or(new.as_ref().and_then(|n| n.1))
            .unwrap_or(0);
        Ok(Self { old, new, sheet })
    }

    /// The names after this change, or why it can't be made: the name must
    /// be valid, unique in its scope, and its formula must parse.
    pub fn apply(&self, names: &[Name], sheet_names: &[String]) -> Result<Vec<Name>, String> {
        let same =
            |a: &(String, Option<usize>), n: &Name| a.0.eq_ignore_ascii_case(&n.0) && a.1 == n.1;
        let mut next: Vec<Name> = names.to_vec();
        if let Some(old) = &self.old {
            let at = next
                .iter()
                .position(|n| same(old, n))
                .ok_or_else(|| format!("The name {} no longer exists.", old.0))?;
            next.remove(at);
        }
        if let Some(new) = &self.new {
            // A trial model checks the name, duplicates and the formula
            // exactly as the calculation will.
            let mut model = ironcalc_base::Model::new_empty("Folio", "en", "UTC", "en")?;
            for (i, name) in sheet_names.iter().enumerate() {
                if i == 0 {
                    model.rename_sheet_by_index(0, name)?;
                } else {
                    model.add_sheet(name)?;
                }
            }
            for (name, scope, formula) in &next {
                let _ = model.new_defined_name(name, scope.map(|s| s as u32), formula);
            }
            model
                .new_defined_name(&new.0, new.1.map(|s| s as u32), &new.2)
                .map_err(|e| match e.as_str() {
                    "Name: Invalid defined name" => format!(
                        "“{}” isn't a valid name. Names start with a letter or underscore, contain no spaces, and can't look like a cell reference.",
                        new.0
                    ),
                    "Name: Defined name already exists" => {
                        format!("{} is already defined in that scope.", new.0)
                    }
                    _ => format!("The formula for {} isn't valid: {e}", new.0),
                })?;
            next.push(new.clone());
        }
        Ok(next)
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Writes `edit` into the package's `definedNames`, keeping every other
/// entry (built-in `_xlnm.` names, names Folio doesn't calculate) as it is.
/// Formulas are stored in the file's form (`_xlfn.` prefixes).
pub fn write(pkg: &mut Package, sheets: &Value, edit: &NameEdit) -> Result<(), String> {
    let xml = pkg.text("xl/workbook.xml")?.to_string();
    let d = parse(&xml)?;
    let root = d.root_element();
    let sheet_names: Vec<String> = sheets
        .as_array()
        .map(|s| {
            s.iter()
                .map(|s| s["name"].as_str().unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default();
    // localSheetId counts the workbook's <sheet> elements.
    let file_sheets: Vec<String> = child(root, "sheets")
        .ok_or("Missing worksheets")?
        .children()
        .filter(|n| n.is_element())
        .map(|n| attr(n, "name").unwrap_or("").to_string())
        .collect();
    let local_id = |scope: Option<usize>| -> Result<Option<usize>, String> {
        match scope {
            None => Ok(None),
            Some(i) => {
                let name = sheet_names.get(i).ok_or("Worksheet missing")?;
                file_sheets
                    .iter()
                    .position(|n| n == name)
                    .map(Some)
                    .ok_or_else(|| "Worksheet missing".to_string())
            }
        }
    };
    let old = child(root, "definedNames");
    let target = match &edit.old {
        Some((name, scope)) => {
            let id = local_id(*scope)?.map(|i| i.to_string());
            Some(
                old.into_iter()
                    .flat_map(|n| n.children())
                    .find(|n| {
                        n.is_element()
                            && attr(*n, "name").is_some_and(|v| v.eq_ignore_ascii_case(name))
                            && attr(*n, "localSheetId").map(str::to_string) == id
                    })
                    .ok_or_else(|| format!("The name {name} is not in the workbook."))?,
            )
        }
        None => None,
    };
    let entry = match &edit.new {
        Some((name, scope, formula)) => {
            let stored = crate::structure::name_formula(formula, &sheet_names, edit.sheet);
            let scope_attr = match local_id(*scope)? {
                Some(i) => format!(" localSheetId=\"{i}\""),
                None => String::new(),
            };
            format!(
                "<definedName name=\"{}\"{scope_attr}>{}</definedName>",
                esc(name),
                esc(&stored)
            )
        }
        None => String::new(),
    };
    let edited = match (old, target) {
        (_, Some(node)) => {
            let mut out = xml.clone();
            out.replace_range(node.range(), &entry);
            out
        }
        (Some(list), None) => {
            let end = list.range().end - "</definedNames>".len();
            if xml[list.range()].ends_with("/>") {
                let mut out = xml.clone();
                out.replace_range(
                    list.range(),
                    &format!("<definedNames>{entry}</definedNames>"),
                );
                out
            } else {
                format!("{}{entry}{}", &xml[..end], &xml[end..])
            }
        }
        (None, None) => {
            // <definedNames> follows <sheets> (and <functionGroups>,
            // <externalReferences>) in the schema's order.
            let after = ["externalReferences", "functionGroups", "sheets"]
                .iter()
                .find_map(|t| child(root, t))
                .ok_or("Missing worksheets")?;
            let at = after.range().end;
            format!(
                "{}<definedNames xmlns=\"{S}\">{entry}</definedNames>{}",
                &xml[..at],
                &xml[at..]
            )
        }
    };
    // An emptied list is removed; Excel rejects an empty <definedNames>.
    let edited = {
        let d = parse(&edited)?;
        match child(d.root_element(), "definedNames") {
            Some(list) if !list.children().any(|n| n.is_element()) => {
                let mut out = edited.clone();
                out.replace_range(list.range(), "");
                out
            }
            _ => edited,
        }
    };
    pkg.set("xl/workbook.xml", edited);
    Ok(())
}

/// A name formula as a person types it: without the file's prefixes.
pub fn display_formula(formula: &str) -> String {
    let mut out = String::with_capacity(formula.len());
    let mut quoted = false;
    let mut rest = formula;
    while let Some(c) = rest.chars().next() {
        if c == '"' {
            quoted = !quoted;
        }
        if !quoted {
            if let Some(p) = ["_xlfn._xlws.", "_xlfn.", "_xlpm.", "_xleta."]
                .iter()
                .find(|p| rest.starts_with(**p))
            {
                rest = &rest[p.len()..];
                continue;
            }
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// The Name Manager's rows: each name with its scope, formula and value.
pub fn rows(names: &[Name], sheets: &Value, values: &[Value]) -> Value {
    json!(names
        .iter()
        .zip(values)
        .map(|(n, value)| json!({
            "name": n.0,
            "sheetId": n.1.map(|i| sheets[i]["id"].clone()),
            "scope": n.1.map(|i| sheets[i]["name"].clone()).unwrap_or(json!("Workbook")),
            "refersTo": format!("={}", display_formula(&n.2)),
            "value": value,
        }))
        .collect::<Vec<_>>())
}
