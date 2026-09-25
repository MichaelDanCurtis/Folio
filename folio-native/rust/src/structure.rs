//! Workbook structural edits. Coordinates are one-based; formula references are
//! transformed as syntax nodes, never by replacing text inside a formula.
use crate::formula::{address, parse_address, COLS, ROWS};
use ironcalc_base::expressions::{
    parser::{
        new_parser_english,
        static_analysis::{add_implicit_intersection, remove_redundant_implicit_intersection},
        stringify::{to_english_string, to_excel_string},
        Node,
    },
    token::Error,
    types::CellReferenceRC,
};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Edit {
    Axis {
        rows: bool,
        insert: bool,
        at: i32,
        count: i32,
    },
    Rename(String),
    Move(usize),
    Delete,
}

#[derive(Clone, Debug)]
pub struct Change {
    pub sheet: usize,
    pub names: Vec<String>,
    pub edit: Edit,
}

pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.encode_utf16().count() <= 31
        && !name.contains(['/', '\\', '?', '*', ':', '[', ']'])
        && !name.chars().any(char::is_control)
        && !name.starts_with('\'')
        && !name.ends_with('\'')
        && !name.eq_ignore_ascii_case("History")
}

pub fn copy_formula(
    formula: &str,
    names: &[String],
    sheet: usize,
    from: &str,
    to: &str,
) -> Result<String, String> {
    let context = |a: &str| -> Result<CellReferenceRC, String> {
        let (r, c) = parse_address(a)?;
        Ok(CellReferenceRC {
            sheet: names[sheet].clone(),
            row: r as i32 + 1,
            column: c as i32 + 1,
        })
    };
    let mut parser = new_parser_english(names.to_vec(), vec![], HashMap::new());
    let (masked, refs) = crate::table_refs::protect(formula)?;
    let node = parser.parse(&masked, &context(from)?);
    if matches!(node, Node::ParseErrorKind { .. }) {
        return Err(format!("This formula cannot be copied yet: {formula}"));
    }
    Ok(crate::table_refs::restore(
        to_english_string(&node, &context(to)?),
        refs,
    ))
}

/// Parses a formula (with or without `=`) at `at` on sheet `sheet`, with
/// structured table references masked. None if it doesn't parse.
fn parse_at(
    formula: &str,
    names: &[String],
    sheet: usize,
    at: &str,
    defined: &[(String, Option<u32>, String)],
) -> Option<(Node, CellReferenceRC, Vec<(String, String)>)> {
    let (r, c) = parse_address(at).ok()?;
    let context = CellReferenceRC {
        sheet: names.get(sheet)?.clone(),
        row: r as i32 + 1,
        column: c as i32 + 1,
    };
    let body = formula.strip_prefix('=').unwrap_or(formula);
    let (masked, refs) = crate::table_refs::protect(body).ok()?;
    // The parser must know the workbook's names, or it reads them as bare
    // identifiers. Masked table references are single cells or ranges it
    // can't see, so they are declared as names that need no `@`.
    let mut known = defined.to_vec();
    known.extend(
        refs.iter()
            .map(|(placeholder, _)| (placeholder.clone(), None, "$A$1".to_string())),
    );
    let mut parser = new_parser_english(names.to_vec(), known, HashMap::new());
    let node = parser.parse(&masked, &context);
    (!matches!(node, Node::ParseErrorKind { .. })).then_some((node, context, refs))
}

/// A legacy (pre-dynamic-array) Excel formula as Excel 365 shows it, with
/// `@` wherever the old engine intersected a range with the formula's row
/// or column (X08.14): `=ABS(A1:A9)` in row 5 means `=ABS(@A1:A9)`, the
/// value in A5. Formulas that need no `@`, or don't parse, are returned
/// unchanged, so most imported formulas keep their exact text.
pub fn legacy_formula(
    formula: &str,
    names: &[String],
    sheet: usize,
    at: &str,
    defined: &[(String, Option<u32>, String)],
) -> String {
    let Some((mut node, context, refs)) = parse_at(formula, names, sheet, at, defined) else {
        return formula.to_string();
    };
    let plain = to_english_string(&node, &context);
    // Rewrite only a formula the parser read completely: printed back, it
    // must match the original apart from case and spacing. Anything else
    // keeps its text rather than risk changing its meaning.
    let squash = |text: &str| {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase()
    };
    let original = formula.strip_prefix('=').unwrap_or(formula);
    if squash(&crate::table_refs::restore(plain.clone(), refs.clone())) != squash(original) {
        return formula.to_string();
    }
    add_implicit_intersection(&mut node, true);
    let intersected = to_english_string(&node, &context);
    if intersected == plain {
        return formula.to_string();
    }
    format!("={}", crate::table_refs::restore(intersected, refs))
}

/// A formula in the form a worksheet part stores it. Newer functions get
/// their `_xlfn.` prefixes, so Excel doesn't read them as unknown names.
/// `@` operators that legacy semantics already imply are dropped, and any
/// other `@` is written as `_xlfn.SINGLE(...)`. Without the `=`; formulas
/// that don't parse are written as they are.
pub fn excel_formula(
    formula: &str,
    names: &[String],
    sheet: usize,
    at: &str,
    defined: &[(String, Option<u32>, String)],
) -> String {
    let body = formula.strip_prefix('=').unwrap_or(formula);
    let Some((mut node, context, refs)) = parse_at(body, names, sheet, at, defined) else {
        return body.to_string();
    };
    remove_redundant_implicit_intersection(&mut node, true);
    crate::table_refs::restore(to_excel_string(&node, &context), refs)
}

/// Check reference nodes, rather than guessing sheet qualifiers from text.
/// Quoted strings containing exclamation marks are ordinary text.
pub(crate) fn missing_formula_sheet(
    formula: &str,
    names: &[String],
    sheet: usize,
    at: &str,
) -> Result<bool, String> {
    fn missing(n: &Node) -> bool {
        use Node::*;
        match n {
            WrongReferenceKind { .. } | WrongRangeKind { .. } | ParseErrorKind { .. } => true,
            OpRangeKind { left, right }
            | OpConcatenateKind { left, right }
            | OpSumKind { left, right, .. }
            | OpProductKind { left, right, .. }
            | OpPowerKind { left, right }
            | CompareKind { left, right, .. } => missing(left) || missing(right),
            FunctionKind { args, .. } | NamedFunctionKind { args, .. } => args.iter().any(missing),
            LambdaDefKind { body, .. } => missing(body),
            LambdaCallKind { lambda, args } => missing(lambda) || args.iter().any(missing),
            ImplicitIntersection { child, .. } | SpillRangeOperator { child } => missing(child),
            UnaryKind { right, .. } => missing(right),
            _ => false,
        }
    }
    let (row, column) = parse_address(at)?;
    let mut parser = new_parser_english(names.to_vec(), vec![], HashMap::new());
    let (masked, _) = crate::table_refs::protect(formula)?;
    Ok(missing(&parser.parse(
        &masked,
        &CellReferenceRC {
            sheet: names[sheet].clone(),
            row: row as i32 + 1,
            column: column as i32 + 1,
        },
    )))
}

impl Change {
    pub fn from_request(w: &Value, r: &Value) -> Result<Self, String> {
        let sheets = w["sheets"].as_array().ok_or("Missing sheets")?;
        let sheet = sheets
            .iter()
            .position(|s| s["id"] == r["sheetId"])
            .ok_or("Worksheet no longer exists")?;
        let names: Vec<String> = sheets
            .iter()
            .map(|s| s["name"].as_str().unwrap().into())
            .collect();
        let edit = match r["action"].as_str().ok_or("Missing structural action")? {
            "insertRows" | "deleteRows" | "insertColumns" | "deleteColumns" => {
                let action = r["action"].as_str().unwrap();
                let rows = action.ends_with("Rows");
                let limit = if rows { ROWS } else { COLS } as u64;
                let at = r["at"].as_u64().ok_or("Choose a row or column")?;
                let count = match r.get("count") {
                    Some(v) => v.as_u64().ok_or("Invalid row or column count")?,
                    None => 1,
                };
                if at == 0 || count == 0 || at > limit || count > limit || at + count - 1 > limit {
                    return Err("The operation exceeds worksheet bounds".into());
                }
                Edit::Axis {
                    rows,
                    insert: action.starts_with("insert"),
                    at: at as i32,
                    count: count as i32,
                }
            }
            "rename" => {
                let name = r["name"].as_str().ok_or("Enter a worksheet name")?;
                if !valid_name(name) {
                    return Err("Use 1–31 characters, without / \\ ? * : [ ], leading/trailing apostrophes, or the reserved name History.".into());
                }
                if names
                    .iter()
                    .enumerate()
                    .any(|(i, n)| i != sheet && n.to_lowercase() == name.to_lowercase())
                {
                    return Err("Another worksheet already has that name".into());
                }
                Edit::Rename(name.into())
            }
            "move" => {
                let index = r["index"].as_u64().ok_or("Choose a sheet position")? as usize;
                if index >= sheets.len() {
                    return Err("Invalid sheet position".into());
                }
                Edit::Move(index)
            }
            "delete" => {
                if sheets.len() == 1 {
                    return Err("Keep at least one worksheet".into());
                }
                Edit::Delete
            }
            _ => return Err("Unknown structural action".into()),
        };
        Ok(Self { sheet, names, edit })
    }

    /// A removed coordinate has no image. Insertion must not discard data at the
    /// bottom/right edge; callers reject any populated cell mapped out of bounds.
    pub fn point(&self, p: i32) -> Option<i32> {
        match self.edit {
            Edit::Axis {
                insert: true,
                at,
                count,
                ..
            } => Some(if p >= at { p + count } else { p }),
            Edit::Axis {
                insert: false,
                at,
                count,
                ..
            } => {
                if p < at {
                    Some(p)
                } else if p >= at + count {
                    Some(p - count)
                } else {
                    None
                }
            }
            _ => Some(p),
        }
    }

    /// Excel shrinks ranges when an endpoint is deleted. IronCalc 0.8.3's
    /// displacement helper instead invalidates that endpoint, so use interval
    /// intersection here and invalidate only when no referenced cells survive.
    pub fn interval(&self, a: i32, b: i32) -> Option<(i32, i32)> {
        let (lo, hi) = (a.min(b), a.max(b));
        let (x, y) = match self.edit {
            Edit::Axis {
                insert: false,
                at,
                count,
                ..
            } => {
                let end = at + count - 1;
                if lo >= at && hi <= end {
                    return None;
                }
                let first = if lo >= at && lo <= end { end + 1 } else { lo };
                let last = if hi >= at && hi <= end { at - 1 } else { hi };
                (self.point(first)?, self.point(last)?)
            }
            _ => (self.point(lo)?, self.point(hi)?),
        };
        Some(if a <= b { (x, y) } else { (y, x) })
    }

    pub fn cell(&self, a: &str) -> Result<Option<String>, String> {
        let (r, c) = parse_address(a)?;
        let (r, c) = match self.edit {
            Edit::Axis { rows: true, .. } => (self.point(r as i32 + 1), Some(c as i32 + 1)),
            Edit::Axis { rows: false, .. } => (Some(r as i32 + 1), self.point(c as i32 + 1)),
            _ => return Ok(Some(a.into())),
        };
        match (r, c) {
            (Some(r), Some(c)) if r > 0 && r <= ROWS as i32 && c > 0 && c <= COLS as i32 => {
                Ok(Some(address(r as usize - 1, c as usize - 1)))
            }
            (None, _) | (_, None) => Ok(None),
            _ => {
                Err("This would push existing cells or formatting beyond the worksheet edge".into())
            }
        }
    }

    pub fn range(&self, range: &str) -> Result<Option<String>, String> {
        if range == "#REF!" {
            return Ok(None);
        }
        let ends: Vec<_> = range.split(':').collect();
        if ends.len() > 2 {
            return Err("Invalid range".into());
        }
        let (r1, c1) = parse_address(&ends[0].replace('$', ""))?;
        let (r2, c2) = parse_address(&ends[ends.len() - 1].replace('$', ""))?;
        let mut rows = (r1 as i32 + 1, r2 as i32 + 1);
        let mut cols = (c1 as i32 + 1, c2 as i32 + 1);
        match self.edit {
            Edit::Axis { rows: true, .. } => match self.interval(rows.0, rows.1) {
                Some(v) => rows = v,
                None => return Ok(None),
            },
            Edit::Axis { rows: false, .. } => match self.interval(cols.0, cols.1) {
                Some(v) => cols = v,
                None => return Ok(None),
            },
            _ => {}
        }
        if rows.1 > ROWS as i32 || cols.1 > COLS as i32 {
            return Err("A range would exceed worksheet bounds".into());
        }
        let first = address(rows.0 as usize - 1, cols.0 as usize - 1);
        Ok(Some(if ends.len() == 1 {
            first
        } else {
            format!(
                "{first}:{}",
                address(rows.1 as usize - 1, cols.1 as usize - 1)
            )
        }))
    }

    pub fn formula(&self, f: &str, sheet: usize, a: &str) -> Result<String, String> {
        if matches!(self.edit, Edit::Move(_)) {
            return Ok(f.into());
        }
        let (row, column) = parse_address(a)?;
        let context = CellReferenceRC {
            sheet: self.names[sheet].clone(),
            row: row as i32 + 1,
            column: column as i32 + 1,
        };
        let mut parser = new_parser_english(self.names.clone(), vec![], HashMap::new());
        let (masked, refs) = crate::table_refs::protect(f.strip_prefix('=').unwrap_or(f))?;
        let mut node = parser.parse(&masked, &context);
        self.walk(&mut node, &context)?;
        Ok(format!(
            "{}{}",
            if f.starts_with('=') { "=" } else { "" },
            crate::table_refs::restore(to_english_string(&node, &context), refs)
        ))
    }

    /// Defined names (notably print areas/titles) can be unions. Split only
    /// top-level separators outside quoted sheet names and formula strings.
    pub fn defined_formula(&self, f: &str, sheet: usize) -> Result<String, String> {
        let mut quote = None;
        let mut depth = 0;
        let mut start = 0;
        let mut parts = vec![];
        let mut chars = f.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            if let Some(q) = quote {
                if c == q {
                    if chars.peek().is_some_and(|(_, next)| *next == q) {
                        chars.next();
                    } else {
                        quote = None;
                    }
                }
            } else {
                match c {
                    '\'' | '"' => quote = Some(c),
                    '(' | '{' | '[' => depth += 1,
                    ')' | '}' | ']' => depth -= 1,
                    ',' if depth == 0 => {
                        parts.push(self.formula(&f[start..i], sheet, "A1")?);
                        start = i + 1;
                    }
                    _ => {}
                }
            }
        }
        parts.push(self.formula(&f[start..], sheet, "A1")?);
        Ok(parts.join(","))
    }

    fn walk(&self, n: &mut Node, context: &CellReferenceRC) -> Result<(), String> {
        use Node::*;
        match n {
            ReferenceKind {
                sheet_name,
                sheet_index,
                absolute_row,
                absolute_column,
                row,
                column,
            } if *sheet_index as usize == self.sheet => match &self.edit {
                Edit::Rename(name) => {
                    if sheet_name.is_some() {
                        *sheet_name = Some(name.clone());
                    }
                }
                Edit::Delete => *n = ErrorKind(Error::REF),
                Edit::Axis { rows, .. } => {
                    let (v, absolute, origin, limit) = if *rows {
                        (row, *absolute_row, context.row, ROWS as i32)
                    } else {
                        (column, *absolute_column, context.column, COLS as i32)
                    };
                    let p = *v + if absolute { 0 } else { origin };
                    if let Some(p) = self.point(p).filter(|p| *p > 0 && *p <= limit) {
                        *v = p - if absolute { 0 } else { origin };
                    } else {
                        *n = ErrorKind(Error::REF);
                    }
                }
                _ => {}
            },
            RangeKind {
                sheet_name,
                sheet_index,
                absolute_row1,
                absolute_column1,
                row1,
                column1,
                absolute_row2,
                absolute_column2,
                row2,
                column2,
            } if *sheet_index as usize == self.sheet => {
                match &self.edit {
                    Edit::Rename(name) => {
                        if sheet_name.is_some() {
                            *sheet_name = Some(name.clone());
                        }
                    }
                    Edit::Delete => *n = ErrorKind(Error::REF),
                    Edit::Axis { rows, .. } => {
                        let (v1, v2, abs1, abs2, origin, limit) = if *rows {
                            (
                                row1,
                                row2,
                                *absolute_row1,
                                *absolute_row2,
                                context.row,
                                ROWS as i32,
                            )
                        } else {
                            (
                                column1,
                                column2,
                                *absolute_column1,
                                *absolute_column2,
                                context.column,
                                COLS as i32,
                            )
                        };
                        let a = *v1 + if abs1 { 0 } else { origin };
                        let b = *v2 + if abs2 { 0 } else { origin };
                        // Full-column/row ranges keep spanning the entire axis.
                        if a.min(b) == 1 && a.max(b) == limit {
                            return Ok(());
                        }
                        if let Some((a, b)) = self
                            .interval(a, b)
                            .filter(|(a, b)| *a > 0 && *b > 0 && *a <= limit && *b <= limit)
                        {
                            *v1 = a - if abs1 { 0 } else { origin };
                            *v2 = b - if abs2 { 0 } else { origin };
                        } else {
                            *n = ErrorKind(Error::REF);
                        }
                    }
                    _ => {}
                }
            }
            OpRangeKind { left, right }
            | OpConcatenateKind { left, right }
            | OpSumKind { left, right, .. }
            | OpProductKind { left, right, .. }
            | OpPowerKind { left, right }
            | CompareKind { left, right, .. } => {
                self.walk(left, context)?;
                self.walk(right, context)?;
            }
            FunctionKind { args, .. } | NamedFunctionKind { args, .. } => {
                for arg in args {
                    self.walk(arg, context)?;
                }
            }
            LambdaDefKind { body, .. } => self.walk(body, context)?,
            LambdaCallKind { lambda, args } => {
                self.walk(lambda, context)?;
                for arg in args {
                    self.walk(arg, context)?;
                }
            }
            ImplicitIntersection { child, .. } | SpillRangeOperator { child } => {
                self.walk(child, context)?
            }
            UnaryKind { right, .. } => self.walk(right, context)?,
            ParseErrorKind { formula, .. } => {
                return Err(format!(
                    "Cannot safely update this formula yet: {formula}. Nothing changed."
                ))
            }
            _ => {}
        }
        Ok(())
    }

    pub fn apply_calendar(&self, w: &Value, date1904: bool) -> Result<Value, String> {
        let mut next = w.clone();
        let sheets = next["sheets"].as_array_mut().unwrap();
        let target_id = sheets[self.sheet]["id"].as_str().unwrap().to_string();
        for (i, sheet) in sheets.iter_mut().enumerate() {
            if matches!(self.edit, Edit::Delete) && i == self.sheet {
                continue;
            }
            if i == self.sheet {
                crate::scenarios::transform(sheet, self)?;
            }
            for key in [
                "cells",
                "cellStyles",
                "officeStyles",
                "cached",
                "cellReviews",
                "cellWatches",
            ] {
                if let Some(map) = sheet[key].as_object() {
                    let mut moved = serde_json::Map::new();
                    for (a, value) in map {
                        let to = if i == self.sheet {
                            self.cell(a)?
                        } else {
                            Some(a.clone())
                        };
                        if let Some(to) = to {
                            let v = if key == "cells"
                                && value.as_str().is_some_and(|v| v.starts_with('='))
                            {
                                json!(self.formula(value.as_str().unwrap(), i, a)?)
                            } else {
                                value.clone()
                            };
                            moved.insert(to, v);
                        }
                    }
                    sheet[key] = json!(moved);
                }
            }
            let positions: Vec<_> = sheet["cells"]
                .as_object()
                .unwrap()
                .keys()
                .filter_map(|a| parse_address(a).ok())
                .collect();
            sheet["usedRows"] = json!(positions.iter().map(|p| p.0 + 1).max().unwrap_or(1));
            sheet["usedColumns"] = json!(positions.iter().map(|p| p.1 + 1).max().unwrap_or(1));
        }
        crate::tables::structural(w, &mut next, self, date1904)?;
        let sheets = next["sheets"].as_array_mut().unwrap();
        match &self.edit {
            Edit::Rename(name) => sheets[self.sheet]["name"] = json!(name),
            Edit::Move(to) => {
                let sheet = sheets.remove(self.sheet);
                sheets.insert(*to, sheet);
            }
            Edit::Delete => {
                sheets.remove(self.sheet);
            }
            _ => {}
        }
        fn links(change: &Change, node: &mut Value, id: &str) -> Result<(), String> {
            if node["type"] == "workbookBlock" && node["attrs"]["sheetId"] == id {
                if matches!(change.edit, Edit::Axis { .. }) {
                    let range = change.range(
                        node["attrs"]["range"]
                            .as_str()
                            .ok_or("Missing linked range")?,
                    )?;
                    node["attrs"]["range"] = json!(range.unwrap_or("#REF!".into()));
                }
            }
            if let Some(children) = node.get_mut("content").and_then(Value::as_array_mut) {
                for child in children {
                    links(change, child, id)?;
                }
            }
            Ok(())
        }
        links(self, &mut next["document"], &target_id)?;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows(insert: bool, at: i32, count: i32) -> Change {
        Change {
            sheet: 0,
            names: vec!["Budget".into(), "Inputs".into()],
            edit: Edit::Axis {
                rows: true,
                insert,
                at,
                count,
            },
        }
    }
    #[test]
    fn formulas_transform_syntax_and_ranges() {
        let c = rows(true, 3, 2);
        assert_eq!(
            c.formula("=SUM($B$2:B4)+Inputs!B3+IF(A3=\"A3\",1,0)", 0, "C9")
                .unwrap(),
            "=SUM($B$2:B6)+Inputs!B3+IF(A5=\"A3\",1,0)"
        );
        assert_eq!(
            c.formula("=Budget!B3+$B$3", 1, "C9").unwrap(),
            "=Budget!B5+$B$3"
        );
        assert_eq!(
            c.formula("=SUM(B:B)+SUM(2:4)", 0, "C9").unwrap(),
            "=SUM(B:B)+SUM(2:6)"
        );
        let d = rows(false, 2, 2);
        assert_eq!(
            d.formula("=SUM(B2:B8)+SUM(B1:B3)+SUM(B2:B3)+B3+B4", 0, "C9")
                .unwrap(),
            "=SUM(B2:B6)+SUM(B1:B1)+SUM(#REF!)+#REF!+B2"
        );
        assert_eq!(d.interval(8, 2), Some((6, 2)));
        assert!(d.formula("=SUM(", 0, "A1").is_err());
    }
    #[test]
    fn rename_and_delete_are_reference_aware() {
        let mut c = rows(true, 1, 1);
        c.edit = Edit::Rename("O'Brien data".into());
        assert_eq!(
            c.formula("=Budget!$A$1+IF(A1=\"Budget!A1\",2,3)", 1, "B1")
                .unwrap(),
            "='O''Brien data'!$A$1+IF(A1=\"Budget!A1\",2,3)"
        );
        c.edit = Edit::Delete;
        assert_eq!(
            c.formula("=Budget!A1+SUM(Budget!B2:C9)+A1", 1, "B1")
                .unwrap(),
            "=#REF!+SUM(#REF!)+A1"
        );
    }
}
