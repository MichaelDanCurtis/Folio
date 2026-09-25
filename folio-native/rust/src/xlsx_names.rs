//! Defined names from a workbook package, for calculation. Structural edits
//! keep the package's `definedNames` current (`xlsx_structure`), so the
//! package is their source until Folio has a Name Manager (X09.05).
use crate::office::{attr, parse, Package};
use serde_json::Value;
use std::collections::HashMap;

const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// A defined name: its name, the workspace index of the sheet it is scoped
/// to (None for a workbook-level name), and its formula without `=`.
pub type Name = (String, Option<usize>, String);

/// The package's defined names that formulas can use. Sheet-scoped names
/// are matched to workspace sheets by part path, so sheet order doesn't
/// matter. Left out:
/// - built-in `_xlnm.` names (print areas, filter databases);
/// - names pointing at other workbooks, which X09.07 would need;
/// - names scoped to a sheet that isn't in the workspace.
pub fn read(pkg: &Package, sheets: &Value) -> Vec<Name> {
    let Ok(wb) = pkg.text("xl/workbook.xml").and_then(parse) else {
        return vec![];
    };
    let relations: HashMap<String, String> = pkg
        .text("xl/_rels/workbook.xml.rels")
        .and_then(parse)
        .map(|rels| {
            rels.descendants()
                .filter(|n| n.has_tag_name("Relationship"))
                .filter_map(|n| Some((attr(n, "Id")?.to_string(), attr(n, "Target")?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let sheet_paths: Vec<Option<String>> = wb
        .descendants()
        .filter(|n| n.has_tag_name((S, "sheet")))
        .map(|sheet| {
            attr(sheet, "id")
                .and_then(|id| relations.get(id))
                .and_then(|target| crate::xlsx::resolve(target).ok())
        })
        .collect();
    let workspace_index = |path: &str| {
        sheets
            .as_array()?
            .iter()
            .position(|s| s["officePath"].as_str() == Some(path))
    };
    let mut names = vec![];
    for defined in wb
        .descendants()
        .filter(|n| n.has_tag_name((S, "definedName")))
    {
        let Some(name) = attr(defined, "name") else {
            continue;
        };
        let formula = defined.text().unwrap_or("").trim();
        if name.starts_with("_xlnm.") || formula.is_empty() || refers_elsewhere(formula) {
            continue;
        }
        // A name whose target was deleted is `Sheet1!#REF!` in the file;
        // formulas using it show #REF!, as in Excel.
        let formula = if formula.contains("#REF!") {
            "#REF!"
        } else {
            formula
        };
        let scope = match attr(defined, "localSheetId") {
            None => None,
            Some(id) => {
                let Some(path) = id
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| sheet_paths.get(i).cloned().flatten())
                else {
                    continue;
                };
                match workspace_index(&path) {
                    Some(index) => Some(index),
                    None => continue,
                }
            }
        };
        names.push((
            name.to_string(),
            scope,
            formula.trim_start_matches('=').to_string(),
        ));
    }
    names
}

/// Whether a name formula points into another workbook: `[1]Sheet!A1` or
/// `'[Book.xlsx]Sheet'!A1`, a bracketed part followed by a sheet name and
/// `!`. Structured table references (`Table1[Amount]`) have no `!` after
/// the bracket, and stay.
pub(crate) fn refers_elsewhere(formula: &str) -> bool {
    let mut rest = formula;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']') else {
            return false;
        };
        let after = &rest[open + close + 1..];
        let sheet: String = after
            .chars()
            .take_while(|c| !matches!(c, '!' | '[' | ']' | '(' | ')' | ','))
            .collect();
        if after[sheet.len()..].starts_with('!') && !sheet.is_empty() {
            return true;
        }
        rest = after;
    }
    false
}
