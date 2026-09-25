//! TEXT against Excel's own results: every `TEXT(value, code)` row of Apache
//! POI's format test workbooks (tests/data/text_format_cases.tsv, generated
//! from the Office corpus). A ratchet: the number of matches may only rise.
//!
//!     cargo test --test text_format_cases -- --nocapture
use std::collections::BTreeMap;

const MATCHING: usize = 213;

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// TEXT as the engine evaluates it for one value and code.
fn text(kind: &str, value: &str, code: &str, date1904: bool) -> Result<String, String> {
    match kind {
        "b" => Ok(if value == "1" { "TRUE" } else { "FALSE" }.into()),
        "s" => match value.trim().parse::<f64>() {
            Ok(n) => ssf_rs::checked::number(code, n, date1904),
            Err(_) => ssf_rs::checked::text(code, value),
        },
        _ => ssf_rs::checked::number(code, value.parse().map_err(|_| "value")?, date1904),
    }
}

#[test]
fn text_format_cases() {
    let data = include_str!("data/text_format_cases.tsv");
    let mut totals: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut matching = 0;
    for line in data.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        let (file, row, dates, kind) = (f[0], f[1], f[2], f[3]);
        let (value, code, excel) = (unescape(f[4]), unescape(f[5]), unescape(f[6]));
        let got = text(kind, &value, &code, dates == "1904");
        let entry = totals.entry(file).or_default();
        entry.1 += 1;
        if got.as_deref() == Ok(excel.as_str()) {
            entry.0 += 1;
            matching += 1;
        } else if std::env::var("TEXT_CASES_VERBOSE").is_ok() {
            println!("{file}!A{row} {code:?} {value:?}: excel {excel:?}, folio {got:?}");
        }
    }
    for (file, (ok, all)) in &totals {
        println!("{file}: {ok}/{all}");
    }
    println!("TEXT cases matching Excel: {matching}");
    assert!(matching >= MATCHING, "{matching} < {MATCHING}");
}
