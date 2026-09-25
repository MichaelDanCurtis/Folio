//! Formulas over huge ranges: aggregates skip the empty part of a range, and
//! any single formula that would read too many cells becomes `#NUM!` rather
//! than hanging the engine or exhausting memory.
use crate::workspace::Core;
use serde_json::{json, Value};
use std::time::Instant;

fn blank() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c
}

fn with(cells: &[(&str, &str)]) -> (Core, Value) {
    let mut c = blank();
    let sheet = c.workspace["sheets"][0]["id"].clone();
    let cells: Vec<Value> = cells
        .iter()
        .map(|(a, v)| json!({"sheetId":sheet,"address":a,"value":v}))
        .collect();
    let started = Instant::now();
    c.dispatch(json!({"op":"cells","cells":cells})).unwrap();
    assert!(
        started.elapsed() < crate::fuzz_tests::slow_limit(),
        "took {:?}",
        started.elapsed()
    );
    let formatted = c.snapshot()["formatted"][sheet.as_str().unwrap()].clone();
    (c, formatted)
}

fn text(formatted: &Value, address: &str) -> String {
    formatted[address]["text"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

#[test]
fn whole_sheet_aggregates_are_exact_and_fast() {
    let all = "A1:XFD1048576";
    let formulas = [
        ("B20", format!("=COUNTBLANK({all})"), "17179869181"),
        ("B21", format!("=COUNT({all})"), "2"),
        ("B22", format!("=COUNTA({all})"), "3"),
        ("B23", format!("=AVERAGE({all})"), "6"),
        ("B24", format!("=MAX({all})"), "7"),
        ("B25", format!("=MIN(A:XFD)"), "5"),
        ("B26", format!("=LARGE({all},2)"), "5"),
        ("B27", format!("=SMALL({all},1)"), "5"),
        ("B28", format!("=MEDIAN({all})"), "6"),
        ("B29", format!("=SUBTOTAL(9,{all})"), "12"),
        ("B30", format!("=SUBTOTAL(103,{all})"), "3"),
        ("B31", format!("=CONCAT({all})"), "57x"),
        ("B32", format!("=PERCENTILE({all},0.5)"), "6"),
        ("B33", format!("=MODE.SNGL(A1:XFD1048576,5,5)"), "5"),
        ("B34", format!("=AVERAGEA({all})"), "4"),
        ("B35", format!("=SUMSQ({all})"), "74"),
        ("B36", format!("=PRODUCT({all})"), "35"),
    ];
    // The formulas read a whole sheet, so they live on another one.
    let mut c = blank();
    c.dispatch(json!({"op":"addSheet"})).unwrap();
    let sheets = c.workspace["sheets"].clone();
    assert!(sheets.as_array().unwrap().len() >= 2, "{sheets}");
    let data = sheets[0]["id"].clone();
    let report = sheets[1]["id"].clone();
    let data_name = sheets[0]["name"].as_str().unwrap().to_string();
    let mut cells = vec![
        json!({"sheetId":data,"address":"A1","value":"5"}),
        json!({"sheetId":data,"address":"C3","value":"7"}),
        json!({"sheetId":data,"address":"C4","value":"x"}),
    ];
    for (address, formula, _) in &formulas {
        let formula = formula
            .replace("A1:XFD1048576", &format!("'{data_name}'!A1:XFD1048576"))
            .replace("A:XFD", &format!("'{data_name}'!A:XFD"));
        cells.push(json!({"sheetId":report,"address":address,"value":formula}));
    }
    let started = Instant::now();
    c.dispatch(json!({"op":"cells","cells":cells})).unwrap();
    let elapsed = started.elapsed();
    let formatted = c.snapshot()["formatted"][report.as_str().unwrap()].clone();
    for (address, formula, expected) in &formulas {
        assert_eq!(text(&formatted, address), *expected, "{formula}");
    }
    assert!(
        elapsed < crate::fuzz_tests::slow_limit(),
        "took {elapsed:?}"
    );
}

/// Data at opposite corners makes the populated extent the whole sheet, so
/// the skip-empty clip can't help; the read budget stops the formula in well
/// under a second instead of letting it visit 17 billion cells.
#[test]
fn data_spread_across_the_whole_sheet_hits_the_read_budget() {
    let (_, formatted) = with(&[
        ("A1", "5"),
        ("XFD1048576", "7"),
        ("B2", "=COUNTBLANK(C1:XFD1048576)"),
        ("B3", "=SUM(A1:XFD1048576)"),
    ]);
    assert_eq!(text(&formatted, "B2"), "#NUM!");
    assert_eq!(text(&formatted, "B3"), "#NUM!");
}

#[test]
fn a_formula_that_reads_too_many_cells_is_num_and_the_engine_carries_on() {
    let (mut c, formatted) = with(&[
        ("A1", "5"),
        ("B1", "=SUMPRODUCT(C1:XFD1048576*1)"),
        ("B2", "=ROWS(FILTER(C1:XFD1048576,C1:XFD1048576<>0))"),
        ("B3", "=IFERROR(B1,\"guarded\")"),
        ("B4", "=A1*2"),
    ]);
    assert_eq!(text(&formatted, "B1"), "#NUM!");
    assert_eq!(text(&formatted, "B2"), "#NUM!");
    assert_eq!(text(&formatted, "B3"), "guarded");
    assert_eq!(text(&formatted, "B4"), "10");
    // The engine keeps evaluating normally afterwards.
    let sheet = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":sheet,"address":"A1","value":"6"}]}))
        .unwrap();
    let formatted = c.snapshot()["formatted"][sheet.as_str().unwrap()].clone();
    assert_eq!(text(&formatted, "B4"), "12");
    assert_eq!(text(&formatted, "B1"), "#NUM!");
}

#[test]
fn whole_column_array_math_still_works() {
    let (_, formatted) = with(&[
        ("A1", "2"),
        ("A2", "3"),
        ("B1", "4"),
        ("B2", "5"),
        ("D1", "=SUM(A:A*B:B)"),
        ("D2", "=SUMPRODUCT(A:A,B:B)"),
        ("D3", "=COUNTBLANK(A:A)"),
    ]);
    assert_eq!(text(&formatted, "D1"), "23");
    assert_eq!(text(&formatted, "D2"), "23");
    assert_eq!(text(&formatted, "D3"), "1048574");
}

/// Conditional-format rules are evaluated per cell outside the formula
/// cells, so they carry the same read budget: a rule that reads too much is
/// false (Excel treats a formula error as false). It would go over for every
/// cell, so it is dropped after the first and the edit stays quick.
#[test]
fn a_conditional_format_rule_that_reads_too_many_cells_is_false() {
    let mut c = blank();
    let sheet = c.workspace["sheets"][0]["id"].as_str().unwrap().to_string();
    let rules = [
        ("A1:A200", "SUMPRODUCT($C$1:$XFD$1048576*1)=0"),
        ("B1:B3", "A1>0"),
    ];
    for (range, formula) in rules {
        let rule = json!({"type":"expression","sqref":range,"formulas":[formula],
            "style":{"fill":"#FFFF00"}});
        c.dispatch(json!({"op":"conditionalFormat","sheetId":sheet,
            "baseRevision":c.workspace["revision"],"action":"add","rule":rule}))
            .unwrap();
    }
    let mut cells: Vec<Value> = (1..=200)
        .map(|row| json!({"sheetId":sheet,"address":format!("A{row}"),"value":"1"}))
        .collect();
    cells.push(json!({"sheetId":sheet,"address":"B1","value":"x"}));
    let started = Instant::now();
    c.dispatch(json!({"op":"cells","cells":cells})).unwrap();
    let elapsed = started.elapsed();
    let snapshot = c.snapshot();
    let formatted = &snapshot["formatted"][&sheet];
    // One budget's worth of reads, not one per cell (200 would take minutes).
    assert!(
        elapsed < crate::fuzz_tests::slow_limit(),
        "took {elapsed:?}"
    );
    for row in [1, 200] {
        let cell = &formatted[format!("A{row}")];
        assert!(cell["conditionalStyle"]["fill"].is_null(), "{cell}");
    }
    assert_eq!(formatted["B1"]["conditionalStyle"]["fill"], "#FFFF00");
    // Rules are numbered by priority, and the rule added last comes first.
    assert_eq!(
        snapshot["conditionalWarnings"][&sheet],
        json!(["Rule 2 reads too many cells and is not displayed."])
    );
}

/// Whole-column lookups read only the populated rows: fast, and a sorted
/// search ends at the last value that is not greater (Excel), not in the
/// million blank rows below the data.
#[test]
fn whole_column_lookups_are_fast_and_match_excel() {
    let mut cells = vec![];
    for row in 1..=10 {
        cells.push((format!("A{row}"), format!("{}", row * 10)));
        cells.push((format!("B{row}"), format!("item {row}")));
    }
    let formulas = [
        ("D1", "=VLOOKUP(55,A:B,2)", "item 5"),
        ("D2", "=VLOOKUP(70,A:B,2,FALSE)", "item 7"),
        (
            "D3",
            "=IF(ISNA(VLOOKUP(71,A:B,2,FALSE)),\"none\",1)",
            "none",
        ),
        ("D4", "=MATCH(35,A:A)", "3"),
        ("D5", "=MATCH(90,A:A,0)", "9"),
        ("D6", "=LOOKUP(1000,A:A,B:B)", "item 10"),
        ("D7", "=HLOOKUP(\"x\",F1:XFD2,2,FALSE)", "found"),
        ("D8", "=VLOOKUP(5,A:B,2)", "#N/A"),
    ];
    cells.push(("F1".into(), "x".into()));
    cells.push(("F2".into(), "found".into()));
    let mut all: Vec<(&str, &str)> = cells
        .iter()
        .map(|(a, v)| (a.as_str(), v.as_str()))
        .collect();
    for (address, formula, _) in &formulas {
        all.push((address, formula));
    }
    // Many whole-column lookups, as in real workbooks.
    let many: Vec<(String, String)> = (1..=2000)
        .map(|r| {
            (
                format!("H{r}"),
                format!("=IF(ISNA(VLOOKUP({r},A:B,2,FALSE)),\" \",VLOOKUP({r},A:B,2,FALSE))"),
            )
        })
        .collect();
    all.extend(many.iter().map(|(a, v)| (a.as_str(), v.as_str())));
    let (_, formatted) = with(&all);
    for (address, formula, expected) in formulas {
        assert_eq!(text(&formatted, address), expected, "{formula}");
    }
    assert_eq!(text(&formatted, "H10"), "item 1");
}
