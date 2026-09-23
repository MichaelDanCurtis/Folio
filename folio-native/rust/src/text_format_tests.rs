use crate::{number_format, workspace::Core};
use serde_json::{json, Value};
fn evaluate(cells: Value) -> Core {
    let mut core = Core::new();
    let mut w = core.workspace.clone();
    w["sheets"] = json!([{"id":"s","name":"Sheet","cells":cells}]);
    core.dispatch(json!({"op":"replace","workspace":w}))
        .unwrap();
    core
}
fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

#[test]
fn text_formulas_use_the_cell_formatter_without_changing_number_types() {
    let examples = [
        (6., "0.0000"),
        (1234.567, "$#,##0.00"),
        (0.285, "0.0%"),
        (2.125, "# ??/??"),
        (12345., "0.00E+00"),
        (123456789., "#,##0,,\"M\""),
        (60., "yyyy-mm-dd"),
        (0.4999999, "h:mm:ss AM/PM"),
        (2.9999999, "[h]:mm:ss"),
        (-1.25, "[>=100]0.0\" large\";[Red][<0](0.00);0.000"),
        (6., ";;;"),
        (12.5, "_(\"$\"* #,##0.00_)"),
    ];
    let mut cells = serde_json::Map::new();
    for (i, (value, code)) in examples.iter().enumerate() {
        cells.insert(format!("A{}", i + 1), json!(value.to_string()));
        cells.insert(
            format!("B{}", i + 1),
            json!(format!("=TEXT(A{},{})", i + 1, quote(code))),
        );
        cells.insert(format!("C{}", i + 1), json!(format!("=TYPE(B{})", i + 1)));
    }
    let mut c = evaluate(Value::Object(cells));
    let snapshot = c.snapshot();
    for (i, (value, code)) in examples.iter().enumerate() {
        assert_eq!(
            snapshot["calculated"]["s"][format!("B{}", i + 1)],
            number_format::render(&json!(value), code, false, &[])["text"],
            "{code}"
        );
        assert_eq!(snapshot["calculated"]["s"][format!("C{}", i + 1)], 2.0);
        assert_eq!(snapshot["calculated"]["s"][format!("A{}", i + 1)], *value);
    }
    let folder = std::env::temp_dir().join("folio-text-format-tests");
    std::fs::create_dir_all(&folder).unwrap();
    c.dispatch(
        json!({"op":"exportOffice","format":"xlsx","path":folder.join("text-formulas.xlsx")}),
    )
    .unwrap();
    std::fs::write(
        folder.join("text-formulas.json"),
        serde_json::to_vec_pretty(&snapshot).unwrap(),
    )
    .unwrap();
    c.dispatch(json!({"op":"save","path":folder.join("text-formulas.folio")}))
        .unwrap();
    let before = c.snapshot();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let reopened = c
        .dispatch(json!({"op":"load","path":folder.join("text-formulas.folio")}))
        .unwrap();
    assert_eq!(before["calculated"], reopened["calculated"]);
    c.dispatch(
        json!({"op":"exportOffice","format":"xlsx","path":folder.join("text-reopened.xlsx")}),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(folder.join("text-formulas.xlsx")).unwrap(),
        std::fs::read(folder.join("text-reopened.xlsx")).unwrap()
    );
}

#[test]
fn text_format_broadcasts_values_and_codes_and_propagates_errors() {
    let mut c = evaluate(json!({
        "A1":"=INDEX(TEXT({1.25;2.5},{\"0.00\",\"0%\"}),1,1)",
        "A2":"=INDEX(TEXT({1.25;2.5},{\"0.00\",\"0%\"}),2,2)",
        "A3":"=INDEX(TEXT({1;2;3},{\"0.0\";\"0.00\"}),3,1)",
        "A4":"=INDEX(TEXT({1;2},{\"0.0\";\"broken[\"}),1,1)",
        "A5":"=INDEX(TEXT({1;2},{\"0.0\";\"broken[\"}),2,1)",
        "A6":"=TEXT(1/0,\"0.00\")", "A7":"=TEXT(1,NA())", "A8":"=TEXT(1)",
        "A9":"=TEXT(1,\"0;0;0;0;0\")", "A10":"=TEXT(-1,\"yyyy-mm-dd\")",
        "A11":"=TEXT(1,REPT(\"0\",256))", "A12":"=TEXT(,\"0.00\")",
        "A13":"=TEXT(\"0012.5\",\"0.00\")", "A14":"=TEXT(\"hello\",\"0.00\")",
        "A15":"=TYPE(TEXT(TRUE,\"0.00\"))", "A16":"=TEXT(FALSE,\"0.00\")",
        "A17":"=TEXT(1.25,\"# ?/999999999999999999999999\")"
    }));
    let s = c.snapshot();
    let values = &s["calculated"]["s"];
    for (a, v) in [
        ("A1", "1.25"),
        ("A2", "250%"),
        ("A3", "#N/A"),
        ("A4", "1.0"),
        ("A5", "#VALUE!"),
        ("A6", "#DIV/0!"),
        ("A7", "#N/A"),
        ("A8", "#ERROR!"),
        ("A9", "#VALUE!"),
        ("A10", "#VALUE!"),
        ("A11", "#VALUE!"),
        ("A12", "0.00"),
        ("A13", "12.50"),
        ("A14", "hello"),
        ("A16", "FALSE"),
        ("A17", "#VALUE!"),
    ] {
        assert_eq!(values[a], v, "{a}");
    }
    assert_eq!(values["A15"], 2.0);
}

#[test]
fn formatted_filter_labels_preserve_keys_cascading_and_history() {
    let mut c = evaluate(
        json!({"A1":"Group","B1":"Rate","A2":"Keep","B2":"0.125","A3":"Keep","B3":"0.25","A4":"Other","B4":"0.5","A5":"Other","B5":"0.125"}),
    );
    c.dispatch(json!({"op":"formatCells","sheetId":"s","addresses":["B2","B3","B4","B5"],"style":{"numFmt":"0.0%"}})).unwrap();
    c.dispatch(json!({"op":"autoFilter","sheetId":"s","action":"enable","range":"A1:B5"}))
        .unwrap();
    let before = c.snapshot();
    let choices = c
        .dispatch(json!({"op":"filterChoices","sheetId":"s","column":1}))
        .unwrap();
    assert_eq!(c.snapshot(), before);
    let list = choices["values"].as_array().unwrap();
    assert_eq!(list.len(), 3);
    let selected = list.iter().find(|v| v["display"] == "12.5%").unwrap();
    assert_eq!(selected["key"], "0.125");
    assert_eq!(selected["value"], 0.125);
    assert_eq!(selected["sourceAddress"], "B2");
    c.dispatch(json!({"op":"autoFilter","sheetId":"s","action":"filter","column":1,"rule":{"kind":"values","values":[selected["key"].clone()],"blank":false}})).unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([3, 4])
    );
    c.dispatch(json!({"op":"autoFilter","sheetId":"s","action":"filter","column":0,"rule":{"kind":"values","values":["Keep"],"blank":false}})).unwrap();
    let choices = c
        .dispatch(json!({"op":"filterChoices","sheetId":"s","column":1}))
        .unwrap();
    assert_eq!(
        choices["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["display"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["12.5%", "25.0%"]
    );
    let folder = std::env::temp_dir().join("folio-text-format-tests");
    std::fs::create_dir_all(&folder).unwrap();
    let saved = c.snapshot();
    c.dispatch(
        json!({"op":"exportOffice","format":"xlsx","path":folder.join("formatted-filter.xlsx")}),
    )
    .unwrap();
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([3, 4])
    );
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.snapshot()["formatted"], saved["formatted"]);
    assert_eq!(c.workspace["sheets"], saved["workspace"]["sheets"]);
}
