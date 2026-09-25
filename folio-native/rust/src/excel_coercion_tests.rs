//! Coercion and error cases from the Office corpus (X08.08), with Excel's
//! own cached results (POI's FormulaEvalTestData, saved by Excel).
use crate::workspace::Core;
use serde_json::{json, Value};

fn results(formulas: &[&str]) -> Vec<Value> {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let id = c.workspace["sheets"][0]["id"].as_str().unwrap().to_string();
    let cells: Vec<Value> = formulas
        .iter()
        .enumerate()
        .map(|(i, f)| json!({"sheetId":id,"address":format!("A{}", i + 1),"value":f}))
        .collect();
    c.dispatch(json!({"op":"cells","cells":cells})).unwrap();
    let snapshot = c.snapshot();
    (1..=formulas.len())
        .map(|i| {
            let a = format!("A{i}");
            let text = &snapshot["formatted"][&id][&a]["text"];
            if snapshot["formatted"][&id][&a]["isError"] == true {
                text.clone()
            } else {
                snapshot["calculated"][&id][&a].clone()
            }
        })
        .collect()
}

#[test]
fn typed_text_and_logicals_count_as_excel_counts_them() {
    let cases: &[(&str, Value)] = &[
        ("=RIGHT(\"a string\",\"6\")", json!("string")),
        ("=LEFT(\"a string\",\"6\")", json!("a stri")),
        ("=RIGHT(\"a string\",TRUE)", json!("g")),
        ("=LEFT(\"a string\",TRUE)", json!("a")),
        ("=VAR(\"1\",\"2\")", json!(0.5)),
        ("=VARP(\"1\",\"2\")", json!(0.25)),
        ("=AVEDEV(TRUE,FALSE)", json!(0.5)),
        ("=SUMSQ(TRUE,TRUE,FALSE)", json!(2.0)),
        ("=SUMSQ(\"2\",\"3\",4)", json!(29.0)),
        ("=MIN(\"-100\",5)", json!(-100.0)),
        ("=SUM(\"x\",1)", json!("#VALUE!")),
        ("=AND(\"1\",1)", json!("#VALUE!")),
        ("=OR(\"\",TRUE)", json!(true)),
        ("=MODE(\"1\",1,2,3)", json!("#VALUE!")),
    ];
    let formulas: Vec<&str> = cases.iter().map(|(f, _)| *f).collect();
    for ((f, want), got) in cases.iter().zip(results(&formulas)) {
        assert_eq!(&got, want, "{f}");
    }
    let close = results(&[
        "=STDEV(\"1\",\"2\")",
        "=STDEV(TRUE,FALSE,TRUE)",
        "=DEVSQ(\"1\",\"2\",2)",
    ]);
    for (got, want) in
        close
            .iter()
            .zip([0.7071067811865476, 0.5773502691896258, 0.6666666666666666])
    {
        assert!(
            (got.as_f64().unwrap() - want).abs() < 1e-12,
            "{got} vs {want}"
        );
    }
}

#[test]
fn paired_sums_and_empty_statistics_follow_excel() {
    let cases: &[(&str, Value)] = &[
        ("=SUMXMY2(2,4)", json!(4.0)),
        ("=SUMXMY2({1,2},{1,2,3})", json!("#N/A")),
        ("=SUMX2MY2({1,\"a\"},{2,3})", json!(-3.0)),
        ("=SUMX2PY2({\"a\"},{1})", json!("#DIV/0!")),
        ("=AVEDEV(Z1:Z3)", json!("#NUM!")),
        ("=DEVSQ(Z1:Z3)", json!("#NUM!")),
    ];
    let formulas: Vec<&str> = cases.iter().map(|(f, _)| *f).collect();
    for ((f, want), got) in cases.iter().zip(results(&formulas)) {
        assert_eq!(&got, want, "{f}");
    }
}
