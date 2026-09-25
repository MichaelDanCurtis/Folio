//! 3-D references (X08.15): `Sheet1:Sheet4!C1` names the same cell on every
//! sheet from Sheet1 to Sheet4, in the workbook's current order. Excel
//! accepts them only in multi-argument aggregates (SUM, AVERAGE, COUNT, MAX,
//! MIN, PRODUCT, STDEV, VAR and their A/P/S variants), where they mean the
//! list of per-sheet references. The calculator gets that list; the formula
//! as written is unchanged.

/// A sheet span in a formula: its two end sheets and the reference.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub first: String,
    pub last: String,
    pub reference: String,
}

impl Span {
    /// The span as a formula writes it.
    pub fn text(&self) -> String {
        if self.first.eq_ignore_ascii_case(&self.last) {
            return format!("{}!{}", quoted(&self.first), self.reference);
        }
        format!(
            "'{}:{}'!{}",
            self.first.replace('\'', "''"),
            self.last.replace('\'', "''"),
            self.reference
        )
    }
}

/// The sheet spans in `formula`, with their character ranges, outside
/// strings.
fn scan(formula: &str) -> Vec<(std::ops::Range<usize>, Span)> {
    let mut found = vec![];
    if !formula.contains(':') || !formula.contains('!') {
        return found;
    }
    let chars: Vec<char> = formula.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' {
            i += 1;
            while i < chars.len() {
                if chars[i] == '"' {
                    if chars.get(i + 1) == Some(&'"') {
                        i += 2;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if i == 0 || !is_name_char(chars[i - 1]) {
            if let Some((first, last, after)) = span_at(&chars, i) {
                if let Some((reference, end)) = reference_at(&chars, after) {
                    found.push((
                        i..end,
                        Span {
                            first,
                            last,
                            reference,
                        },
                    ));
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
    found
}

/// `formula` with each span replaced by `with(span)`, or None when it has
/// no spans.
fn map(formula: &str, mut with: impl FnMut(&Span) -> String) -> Option<String> {
    let spans = scan(formula);
    if spans.is_empty() {
        return None;
    }
    let chars: Vec<char> = formula.chars().collect();
    let mut out = String::with_capacity(formula.len());
    let mut at = 0;
    for (range, span) in spans {
        out.extend(&chars[at..range.start]);
        out.push_str(&with(&span));
        at = range.end;
    }
    out.extend(&chars[at..]);
    Some(out)
}

/// `formula` with each sheet span expanded, or None when it has none. A
/// span naming a sheet that doesn't exist becomes `#REF!`.
pub fn expand(formula: &str, sheets: &[String]) -> Option<String> {
    map(formula, |s| {
        expanded(&s.first, &s.last, &s.reference, sheets)
    })
}

/// Replaces spans with placeholder names, so a parser that doesn't know
/// them passes them through. Each comes back with its span and its text as
/// written.
pub fn protect(formula: &str) -> (String, Vec<(String, Span, String)>) {
    let mut prefix = "FolioSheetSpan_".to_string();
    while formula.contains(&prefix) {
        prefix.push('_');
    }
    let spans = scan(formula);
    if spans.is_empty() {
        return (formula.to_string(), vec![]);
    }
    let chars: Vec<char> = formula.chars().collect();
    let mut out = String::with_capacity(formula.len());
    let mut found = vec![];
    let mut at = 0;
    for (range, span) in spans {
        out.extend(&chars[at..range.start]);
        let name = format!("{prefix}{}", found.len());
        out.push_str(&name);
        found.push((name, span, chars[range.clone()].iter().collect()));
        at = range.end;
    }
    out.extend(&chars[at..]);
    (out, found)
}

/// The placeholder-to-text pairs that put the spans back as written.
pub fn as_written(spans: Vec<(String, Span, String)>) -> Vec<(String, String)> {
    spans
        .into_iter()
        .map(|(name, _, text)| (name, text))
        .collect()
}

/// A span after sheet `sheet` of `names` is renamed to `name`.
pub fn renamed(span: &Span, names: &[String], sheet: usize, name: &str) -> Span {
    let old = &names[sheet];
    let rename = |end: &str| {
        if end.eq_ignore_ascii_case(old) {
            name.to_string()
        } else {
            end.to_string()
        }
    };
    Span {
        first: rename(&span.first),
        last: rename(&span.last),
        reference: span.reference.clone(),
    }
}

/// A span after sheet `sheet` of `names` is deleted: a deleted end moves
/// one sheet inward, as in Excel. None when the span had only that sheet.
pub fn after_delete(span: &Span, names: &[String], sheet: usize) -> Option<Span> {
    let find = |n: &str| names.iter().position(|s| s.eq_ignore_ascii_case(n));
    let (Some(a), Some(b)) = (find(&span.first), find(&span.last)) else {
        return Some(span.clone());
    };
    let (lo, hi) = (a.min(b), a.max(b));
    if sheet != lo && sheet != hi {
        return Some(span.clone());
    }
    if lo == hi {
        return None;
    }
    let (lo, hi) = if sheet == lo {
        (lo + 1, hi)
    } else {
        (lo, hi - 1)
    };
    Some(Span {
        first: names[lo].clone(),
        last: names[hi].clone(),
        reference: span.reference.clone(),
    })
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '.' | '\\')
}

/// A sheet span starting at `i` (`Name1:Name2!` or `'Name 1:Name 2'!`):
/// the two names and the index after the `!`.
fn span_at(chars: &[char], i: usize) -> Option<(String, String, usize)> {
    if chars[i] == '\'' {
        let mut text = String::new();
        let mut j = i + 1;
        loop {
            match chars.get(j)? {
                '\'' if chars.get(j + 1) == Some(&'\'') => {
                    text.push('\'');
                    j += 2;
                }
                '\'' => break,
                c => {
                    text.push(*c);
                    j += 1;
                }
            }
        }
        if chars.get(j + 1) != Some(&'!') {
            return None;
        }
        // Sheet names can't contain ':', so it separates the two ends.
        let (first, last) = text.split_once(':')?;
        return Some((first.to_string(), last.to_string(), j + 2));
    }
    let word = |from: usize| {
        let mut j = from;
        while j < chars.len() && is_name_char(chars[j]) {
            j += 1;
        }
        (j > from && !chars[from].is_ascii_digit())
            .then(|| (chars[from..j].iter().collect::<String>(), j))
    };
    let (first, j) = word(i)?;
    if chars.get(j) != Some(&':') {
        return None;
    }
    let (last, k) = word(j + 1)?;
    (chars.get(k) == Some(&'!')).then(|| (first, last, k + 1))
}

/// A cell, range, or whole row/column reference starting at `i`.
fn reference_at(chars: &[char], i: usize) -> Option<(String, usize)> {
    let mut j = i;
    let end_of = |mut j: usize| {
        while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '$') {
            j += 1;
        }
        j
    };
    j = end_of(j);
    if j == i {
        return None;
    }
    if chars.get(j) == Some(&':') {
        let k = end_of(j + 1);
        if k > j + 1 {
            j = k;
        }
    }
    let text: String = chars[i..j].iter().collect();
    let valid = text.split(':').all(|part| {
        let part = part.replace('$', "");
        let letters = part.chars().take_while(char::is_ascii_alphabetic).count();
        let digits = &part[letters..];
        (letters > 0 || !digits.is_empty())
            && letters <= 3
            && digits.chars().all(|c| c.is_ascii_digit())
    });
    valid.then_some((text, j))
}

fn quoted(name: &str) -> String {
    format!("'{}'", name.replace('\'', "''"))
}

fn expanded(first: &str, last: &str, reference: &str, sheets: &[String]) -> String {
    let find = |name: &str| sheets.iter().position(|s| s.eq_ignore_ascii_case(name));
    let (Some(a), Some(b)) = (find(first), find(last)) else {
        return "#REF!".into();
    };
    let (from, to) = if a <= b { (a, b) } else { (b, a) };
    sheets[from..=to]
        .iter()
        .map(|name| format!("{}!{reference}", quoted(name)))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::expand;

    fn sheets() -> Vec<String> {
        ["Jan", "Feb", "Mar data", "Apr"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn spans_become_per_sheet_lists() {
        let s = sheets();
        assert_eq!(
            expand("=SUM(Jan:Feb!C1)", &s).unwrap(),
            "=SUM('Jan'!C1,'Feb'!C1)"
        );
        assert_eq!(
            expand("=SUM('Feb:Mar data'!$A$1:B2)+1", &s).unwrap(),
            "=SUM('Feb'!$A$1:B2,'Mar data'!$A$1:B2)+1"
        );
        assert_eq!(
            expand("=MAX(Apr:Feb!A:A)", &s).unwrap(),
            "=MAX('Feb'!A:A,'Mar data'!A:A,'Apr'!A:A)",
            "either order"
        );
        assert_eq!(expand("=SUM(Jan:Nope!A1)", &s).unwrap(), "=SUM(#REF!)");
    }

    #[test]
    fn other_colons_are_left_alone() {
        let s = sheets();
        for formula in [
            "=SUM(A1:B2)",
            "=SUM(Jan!A1:B2)",
            "=\"Jan:Feb!A1\"",
            "=Sales[[#This Row],[Qty]:[Price]]",
            "=TIME(1,2,3)",
        ] {
            assert_eq!(expand(formula, &s), None, "{formula}");
        }
    }
}

#[cfg(test)]
mod workbook_tests {
    use crate::workspace::Core;
    use serde_json::json;

    #[test]
    fn spans_follow_the_sheet_order() {
        let mut c = Core::new();
        c.dispatch(json!({"op":"new","blank":true})).unwrap();
        let mut w = c.workspace.clone();
        w["sheets"] = json!([
            {"id":"s1","name":"Jan","cells":{"A1":"1"}},
            {"id":"s2","name":"Feb","cells":{"A1":"10"}},
            {"id":"s3","name":"Mar","cells":{"A1":"100"}},
            {"id":"t","name":"Total","cells":{
                "A1":"=SUM(Jan:Mar!A1)",
                "A2":"=AVERAGE('Jan:Feb'!A1:A1)",
                "A3":"=SUM(Jan:Gone!A1)"
            }}
        ]);
        c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
        let total = |c: &mut Core| c.snapshot()["calculated"]["t"].clone();
        assert_eq!(total(&mut c)["A1"], json!(111.0));
        assert_eq!(total(&mut c)["A2"], json!(5.5));
        assert_eq!(c.snapshot()["formatted"]["t"]["A3"]["text"], "#REF!");
        // The formula keeps its text.
        assert_eq!(c.workspace["sheets"][3]["cells"]["A1"], "=SUM(Jan:Mar!A1)");
        // Moving Feb after Total takes it out of the span, as in Excel.
        c.dispatch(json!({"op":"structure","sheetId":"s2","action":"move","index":3}))
            .unwrap();
        assert_eq!(c.snapshot()["calculated"]["t"]["A1"], json!(101.0));
        // An edit on a sheet inside the span recalculates the total.
        c.dispatch(json!({"op":"cells","cells":[{"sheetId":"s3","address":"A1","value":"200"}]}))
            .unwrap();
        assert_eq!(c.snapshot()["calculated"]["t"]["A1"], json!(201.0));
    }

    #[test]
    fn renames_deletes_and_row_edits_keep_spans_right() {
        let mut c = Core::new();
        c.dispatch(json!({"op":"new","blank":true})).unwrap();
        let mut w = c.workspace.clone();
        w["sheets"] = json!([
            {"id":"s1","name":"Jan","cells":{"A1":"1"}},
            {"id":"s2","name":"Feb","cells":{"A1":"10"}},
            {"id":"s3","name":"Mar","cells":{"A1":"100"}},
            {"id":"t","name":"Total","cells":{"B2":"=SUM(Jan:Mar!A1)+B1"}}
        ]);
        c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
        let formula = |c: &Core| {
            c.workspace["sheets"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["id"] == "t")
                .unwrap()["cells"]
                .as_object()
                .unwrap()
                .values()
                .next()
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        };
        c.dispatch(json!({"op":"structure","sheetId":"t","action":"insertRows","at":1,"count":1}))
            .unwrap();
        assert_eq!(
            formula(&c),
            "=SUM(Jan:Mar!A1)+B2",
            "the span is untouched, B1 moves"
        );
        c.dispatch(json!({"op":"structure","sheetId":"s1","action":"rename","name":"January"}))
            .unwrap();
        assert_eq!(formula(&c), "=SUM('January:Mar'!A1)+B2");
        assert_eq!(c.snapshot()["calculated"]["t"]["B3"], json!(111.0));
        c.dispatch(json!({"op":"structure","sheetId":"s3","action":"delete"}))
            .unwrap();
        assert_eq!(
            formula(&c),
            "=SUM('January:Feb'!A1)+B2",
            "a deleted end moves inward"
        );
        assert_eq!(c.snapshot()["calculated"]["t"]["B3"], json!(11.0));

        // Written to XLSX and read back with the same text and result.
        let out = std::env::temp_dir().join(format!("folio-spans-{}.xlsx", std::process::id()));
        c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":out}))
            .unwrap();
        let mut again = Core::new();
        again
            .dispatch(json!({"op":"importOffice","path":out}))
            .unwrap();
        let sheet = again.workspace["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == "Total")
            .unwrap()
            .clone();
        assert_eq!(sheet["cells"]["B3"], "=SUM('January:Feb'!A1)+B2");
        let id = sheet["id"].as_str().unwrap().to_string();
        assert_eq!(again.snapshot()["calculated"][&id]["B3"], json!(11.0));
    }
}
