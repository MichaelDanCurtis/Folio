//! Preserve table-reference syntax when moving ordinary A1 references. The
//! calculation adapter qualifies local references without changing saved input.
use ironcalc_base::{
    expressions::{
        lexer::{Lexer, LexerMode},
        token::{TableReference, TableSpecifier, TokenType},
    },
    language::get_default_language,
    locale::get_default_locale,
};
use std::ops::Range;

#[derive(Debug)]
pub struct Reference<'a> {
    pub range: Range<usize>,
    pub table: &'a str,
    pub body: &'a str,
}
fn identifier(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '.' | '\\')
}

pub fn references(formula: &str) -> Result<Vec<Reference<'_>>, String> {
    let chars: Vec<(usize, char)> = formula.char_indices().collect();
    let mut i = 0;
    let mut refs = vec![];
    while i < chars.len() {
        let (start, c) = chars[i];
        if matches!(c, '"' | '\'') {
            i += 1;
            while i < chars.len() {
                if chars[i].1 == c {
                    i += 1;
                    if i < chars.len() && chars[i].1 == c {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            continue;
        }
        let mut bracket = i;
        if identifier(c) {
            while bracket < chars.len() && identifier(chars[bracket].1) {
                bracket += 1;
            }
        }
        if bracket >= chars.len() || chars[bracket].1 != '[' {
            i = bracket.max(i + 1);
            continue;
        }
        let body_start = chars[bracket].0;
        let mut depth = 0;
        i = bracket;
        while i < chars.len() {
            match chars[i].1 {
                '\'' => {
                    i += 2;
                    continue;
                }
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if depth != 0 {
            return Err("Unclosed table reference".into());
        }
        let end = chars.get(i).map(|p| p.0).unwrap_or(formula.len());
        if body_start == start && chars.get(i).is_some_and(|(_, c)| identifier(*c)) {
            return Err("External workbook references cannot be transformed yet".into());
        }
        refs.push(Reference {
            range: start..end,
            table: &formula[start..body_start],
            body: &formula[body_start..end],
        });
    }
    Ok(refs)
}

pub fn replace(formula: &str, edits: Vec<(Range<usize>, String)>) -> String {
    crate::office::replace_ranges(formula, edits)
}

pub fn protect(formula: &str) -> Result<(String, Vec<(String, String)>), String> {
    let refs = references(formula)?;
    let mut prefix = "FolioProtectedTableRef_".to_string();
    while formula.contains(&prefix) {
        prefix.push('_');
    }
    let mut restore = vec![];
    let edits = refs
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            let name = format!("{prefix}{i}");
            restore.push((name.clone(), formula[r.range.clone()].to_string()));
            (r.range, name)
        })
        .collect();
    Ok((replace(formula, edits), restore))
}
pub fn restore(mut formula: String, refs: Vec<(String, String)>) -> String {
    // Identifiers were chosen not to occur anywhere in the source, including
    // literals. Longest first prevents suffix 1 from matching suffix 10.
    let mut refs = refs;
    refs.sort_by_key(|(name, _)| std::cmp::Reverse(name.len()));
    for (name, source) in refs {
        formula = formula.replace(&name, &source);
    }
    formula
}

pub fn qualify(
    formula: &str,
    context_table: Option<&str>,
    table_names: &[String],
) -> Result<String, String> {
    let mut edits = vec![];
    for r in references(formula)? {
        let name = if r.table.is_empty() {
            context_table.ok_or("A local table reference must be inside a table")?
        } else {
            r.table
        };
        let name = table_names
            .iter()
            .find(|n| n.eq_ignore_ascii_case(name))
            .map(String::as_str)
            .unwrap_or(name);
        let inner = r.body[1..r.body.len() - 1].trim();
        let body = if let Some(row) = inner.strip_prefix('@') {
            let row = row.trim();
            if row.starts_with('[') {
                format!("[[#This Row],{row}]")
            } else {
                format!("[[#This Row],[{row}]]")
            }
        } else {
            r.body.to_string()
        };
        edits.push((r.range, format!("{name}{body}")));
    }
    let qualified = replace(formula, edits);
    // A bare table name denotes its data body, excluding headers and totals.
    let (masked, refs) = protect(&qualified)?;
    let chars: Vec<usize> = masked
        .char_indices()
        .map(|p| p.0)
        .chain(std::iter::once(masked.len()))
        .collect();
    let mut lexer = Lexer::new(
        &masked,
        LexerMode::A1,
        get_default_locale(),
        get_default_language(),
    );
    let mut edits = vec![];
    loop {
        let start = lexer.get_position() as usize;
        let token = lexer.next_token();
        let end = lexer.get_position() as usize;
        if let TokenType::Ident(name) = &token {
            if let Some(actual) = table_names.iter().find(|n| n.eq_ignore_ascii_case(name)) {
                let leading = masked[chars[start]..chars[end]].len()
                    - masked[chars[start]..chars[end]].trim_start().len();
                edits.push((
                    chars[start] + leading..chars[end],
                    format!("{actual}[#Data]"),
                ));
            }
        }
        if token == TokenType::EOF || end <= start {
            break;
        }
    }
    Ok(restore(replace(&masked, edits), refs))
}

pub fn rename_table(formula: &str, from: &str, to: &str) -> Result<String, String> {
    let edits = references(formula)?
        .into_iter()
        .filter(|r| r.table.eq_ignore_ascii_case(from))
        .map(|r| (r.range, format!("{to}{}", r.body)))
        .collect();
    let rewritten = replace(formula, edits);
    let (masked, refs) = protect(&rewritten)?;
    let chars: Vec<usize> = masked
        .char_indices()
        .map(|p| p.0)
        .chain(std::iter::once(masked.len()))
        .collect();
    let mut lexer = Lexer::new(
        &masked,
        LexerMode::A1,
        get_default_locale(),
        get_default_language(),
    );
    let mut edits = vec![];
    loop {
        let start = lexer.get_position() as usize;
        let token = lexer.next_token();
        let end = lexer.get_position() as usize;
        if matches!(&token,TokenType::Ident(name) if name.eq_ignore_ascii_case(from)) {
            let leading = masked[chars[start]..chars[end]].len()
                - masked[chars[start]..chars[end]].trim_start().len();
            edits.push((chars[start] + leading..chars[end], to.into()));
        }
        if token == TokenType::EOF || end <= start {
            break;
        }
    }
    Ok(restore(replace(&masked, edits), refs))
}

pub fn escaped(name: &str) -> String {
    name.replace('\'', "''")
        .replace('[', "'[")
        .replace(']', "']")
        .replace('#', "'#")
        .replace('@', "'@")
}

/// Keep formulas in cells leaving a table meaningful after their local table
/// context disappears. Explicit references to named tables remain live.
pub fn materialize_local(
    formula: &str,
    table: &serde_json::Value,
    row: usize,
) -> Result<String, String> {
    let area = crate::tables::area(table)?;
    let header = table["headerRows"].as_u64().unwrap_or(1) as usize;
    let totals = table["totalsRows"].as_u64().unwrap_or(0) as usize;
    let mut edits = vec![];
    for reference in references(formula)?
        .into_iter()
        .filter(|r| r.table.is_empty())
    {
        let qualified = qualify(reference.body, Some("FolioTable"), &["FolioTable".into()])?;
        let mut lexer = Lexer::new(
            &qualified,
            LexerMode::A1,
            get_default_locale(),
            get_default_language(),
        );
        let TokenType::StructuredReference {
            specifier,
            table_reference,
            ..
        } = lexer.next_token()
        else {
            return Err(
                "This local table formula cannot yet be converted to cell references".into(),
            );
        };
        if lexer.next_token() != TokenType::EOF {
            return Err("This local table formula needs additional reference support".into());
        }
        let column = |label: &str| -> Result<usize, String> {
            table["columns"]
                .as_array()
                .unwrap()
                .iter()
                .position(|c| {
                    c["name"]
                        .as_str()
                        .is_some_and(|n| n.eq_ignore_ascii_case(label))
                })
                .map(|ci| area.left + ci)
                .ok_or("Table formula refers to an unknown column".into())
        };
        let (left, right) = match table_reference {
            Some(TableReference::ColumnReference(c)) => {
                let c = column(&c)?;
                (c, c)
            }
            Some(TableReference::RangeReference((a, b))) => (column(&a)?, column(&b)?),
            None => (area.left, area.right),
        };
        let relative = specifier == Some(TableSpecifier::ThisRow);
        let (first, end) = match specifier {
            Some(TableSpecifier::ThisRow) => (row, row + 1),
            Some(TableSpecifier::Headers) => (area.top, area.top + header),
            Some(TableSpecifier::Totals) => (area.bottom + 1 - totals, area.bottom + 1),
            Some(TableSpecifier::All) => (area.top, area.bottom + 1),
            _ => (area.top + header, area.bottom + 1 - totals),
        };
        let cell = |r, c| {
            let a = crate::formula::address(r, c);
            if relative {
                a
            } else {
                let at = a.find(|c: char| c.is_ascii_digit()).unwrap();
                format!("${}${}", &a[..at], &a[at..])
            }
        };
        let replacement = if first >= end {
            "#REF!".into()
        } else if first + 1 == end && left == right {
            cell(first, left)
        } else {
            format!("{}:{}", cell(first, left), cell(end - 1, right))
        };
        edits.push((reference.range, replacement));
    }
    Ok(replace(formula, edits))
}
pub fn rename_columns(
    formula: &str,
    context: Option<&str>,
    table: &str,
    changes: &std::collections::HashMap<String, Option<String>>,
) -> Result<String, String> {
    let mut edits = vec![];
    for r in references(formula)? {
        let owner = if r.table.is_empty() {
            context.unwrap_or("")
        } else {
            r.table
        };
        if !owner.eq_ignore_ascii_case(table) {
            continue;
        }
        let qualified = qualify(&format!("{table}{}", r.body), Some(table), &[table.into()])?;
        let mut lexer = Lexer::new(
            &qualified,
            LexerMode::A1,
            get_default_locale(),
            get_default_language(),
        );
        let TokenType::StructuredReference {
            specifier,
            table_reference,
            ..
        } = lexer.next_token()
        else {
            return Err("This table reference syntax cannot yet be renamed safely".into());
        };
        let mut changed = false;
        let mut missing = false;
        let mut label = |name: String| {
            if let Some(replacement) = changes.get(&name.to_lowercase()) {
                changed = true;
                match replacement {
                    Some(n) => n.clone(),
                    None => {
                        missing = true;
                        name
                    }
                }
            } else {
                name
            }
        };
        let columns = match table_reference {
            Some(TableReference::ColumnReference(c)) => Some(format!("[{}]", escaped(&label(c)))),
            Some(TableReference::RangeReference((a, b))) => {
                Some(format!("[{}]:[{}]", escaped(&label(a)), escaped(&label(b))))
            }
            None => None,
        };
        if !changed {
            continue;
        }
        let result = if missing {
            "#REF!".into()
        } else {
            let spec = specifier.map(|s| match s {
                TableSpecifier::ThisRow => "#This Row",
                TableSpecifier::Totals => "#Totals",
                TableSpecifier::Headers => "#Headers",
                TableSpecifier::Data => "#Data",
                TableSpecifier::All => "#All",
            });
            let body = match (spec, columns) {
                (Some(s), Some(c)) => format!("[[{s}],{c}]"),
                (None, Some(c)) => {
                    if c.contains("]:[") {
                        format!("[{c}]")
                    } else {
                        c
                    }
                }
                (Some(s), None) => format!("[{s}]"),
                (None, None) => "[]".into(),
            };
            format!("{}{body}", r.table)
        };
        edits.push((r.range, result));
    }
    Ok(replace(formula, edits))
}
pub fn invalidate_table(formula: &str, name: &str) -> Result<String, String> {
    let edits = references(formula)?
        .into_iter()
        .filter(|r| r.table.eq_ignore_ascii_case(name))
        .map(|r| (r.range, "#REF!".into()))
        .collect();
    rename_table(&replace(formula, edits), name, "#REF!")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detached_table_cells_resolve_row_range_headers_and_escaped_names() {
        let table = serde_json::json!({"range":"B3:D7","headerRows":1,"totalsRows":1,
            "columns":[{"name":"Qty"},{"name":"Unit price"},{"name":"Net ] value"}]});
        let f = "=SUM([Qty])+[@[Unit price]]+SUM([[#All],[Qty]:[Unit price]])+SUM([[#Totals],[Qty]])+IF(A1=\"[@Qty]\",1,0)+Sales[Qty]";
        assert_eq!(
            materialize_local(f, &table, 4).unwrap(),
            "=SUM($B$4:$B$6)+C5+SUM($B$3:$C$7)+SUM($B$7)+IF(A1=\"[@Qty]\",1,0)+Sales[Qty]"
        );
        assert_eq!(
            materialize_local("=[@[Net '] value]]", &table, 5).unwrap(),
            "=D6"
        );
    }
    #[test]
    fn local_and_named_references_keep_literals_and_unicode() {
        let f = "=SUM(sales[Amount])+[@[Unit price]]*[@Qty]+IF(A1=\"[@Qty]\",1,0)";
        assert_eq!(qualify(f,Some("Sales"),&["Sales".into()]).unwrap(),"=SUM(Sales[Amount])+Sales[[#This Row],[Unit price]]*Sales[[#This Row],[Qty]]+IF(A1=\"[@Qty]\",1,0)");
        assert_eq!(
            qualify("=SUM(sales)", None, &["Sales".into()]).unwrap(),
            "=SUM(Sales[#Data])"
        );
        let (masked, refs) = protect("=SUM(Δεδομένα[Net '] value])+A1+\"Sales[A1]\"").unwrap();
        assert_eq!(
            restore(masked, refs),
            "=SUM(Δεδομένα[Net '] value])+A1+\"Sales[A1]\""
        );
        assert_eq!(
            rename_table(
                "=SUM(Sales)+Sales[Amount]+\"Sales[Amount]\"",
                "Sales",
                "Costs"
            )
            .unwrap(),
            "=SUM(Costs)+Costs[Amount]+\"Sales[Amount]\""
        );
    }
}
