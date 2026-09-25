//! Excel function-catalog coverage. `docs/status/excel-functions.tsv` lists
//! every worksheet function in Microsoft's alphabetical catalog with its
//! status in this engine. This test recomputes each status by entering
//! `=NAME()` and reading the result: `#NAME?` means the engine doesn't know
//! the function, `#N/IMPL` means it parses but isn't implemented, and any
//! other result (including an argument-count error) means it is implemented.
//! A mismatch fails, so coverage can't drop unnoticed and the register can't
//! overstate it. Run with `FOLIO_WRITE_CATALOG=1` to rewrite the statuses.
use crate::workspace::Core;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

const CATALOG: &str = "../docs/status/excel-functions.tsv";

fn status(display: &str) -> &'static str {
    match display {
        "#NAME?" => "unknown",
        "#N/IMPL" | "#N/IMPL!" => "not-implemented",
        _ => "implemented",
    }
}

#[test]
fn excel_function_catalog_statuses_are_current() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(CATALOG);
    let text = std::fs::read_to_string(&path).unwrap();
    let rows: Vec<(String, String, String)> = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut cells = l.split('\t').map(String::from);
            let row = (
                cells.next().unwrap(),
                cells.next().unwrap(),
                cells.next().unwrap(),
            );
            assert!(cells.next().is_none(), "extra column: {l}");
            row
        })
        .collect();
    assert!(rows.len() > 500, "catalog has {} rows", rows.len());

    let mut core = Core::new();
    core.dispatch(json!({"op":"new","blank":true})).unwrap();
    let sheet = core.workspace["sheets"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // One function per row of column A.
    let cells: Vec<Value> = rows
        .iter()
        .enumerate()
        .map(|(i, (name, _, _))| {
            json!({"sheetId":sheet,"address":format!("A{}", i + 1),"value":format!("={name}()")})
        })
        .collect();
    core.dispatch(json!({"op":"cells","cells":cells})).unwrap();
    let formatted = core.snapshot()["formatted"][&sheet].clone();

    let mut measured = Vec::new();
    let mut mismatches = Vec::new();
    for (i, (name, category, recorded)) in rows.iter().enumerate() {
        let display = formatted[format!("A{}", i + 1)]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let actual = status(&display);
        if actual != recorded {
            mismatches.push(format!("{name}: recorded {recorded}, measured {actual}"));
        }
        measured.push((name, category, actual));
    }

    if std::env::var("FOLIO_WRITE_CATALOG").is_ok() {
        let header: Vec<&str> = text.lines().take_while(|l| l.starts_with('#')).collect();
        let mut out = header.join("\n") + "\n";
        for (name, category, actual) in &measured {
            out += &format!("{name}\t{category}\t{actual}\n");
        }
        std::fs::write(&path, out).unwrap();
        let mut totals: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, _, actual) in &measured {
            *totals.entry(actual).or_default() += 1;
        }
        println!("{totals:?}");
        return;
    }
    assert!(
        mismatches.is_empty(),
        "function catalog out of date (rerun with FOLIO_WRITE_CATALOG=1 and update \
         docs/STATUS.md and X08):\n{}",
        mismatches.join("\n")
    );
}
