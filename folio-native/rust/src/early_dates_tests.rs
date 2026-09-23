use crate::workspace::Core;
use serde_json::{json, Value};

#[test]
fn early_excel_dates_retain_serial_fields_and_function_specific_month_ends() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":format!("{}/../test/fixtures/early-dates.xlsx",env!("CARGO_MANIFEST_DIR"))})).unwrap();
    let expected: Value =
        serde_json::from_str(include_str!("../../test/fixtures/early-dates.json")).unwrap();
    let s = c.snapshot();
    let values = &s["calculated"]["office-1"];
    let failures: Vec<_> = expected["expected"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(a, v)| match (values[*a].as_f64(), v.as_f64()) {
            (Some(a), Some(b)) => a != b,
            _ => values[*a] != **v,
        })
        .map(|(a, v)| format!("{a}: {} expected {v}", values[a]))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    for (a, v) in expected["explicitText"].as_object().unwrap() {
        assert_eq!(values[a], *v);
    }
    c.dispatch(json!({"op":"cells","cells":[
        {"sheetId":"office-1","address":"P2","value":"1900-01-01"},
        {"sheetId":"office-1","address":"P3","value":"1900-02-28"},
        {"sheetId":"office-1","address":"P4","value":"1900-02-29"},
        {"sheetId":"office-1","address":"Q2","value":"=COUNTIF(P2:P4,\">=1900-02-28\")"}
    ]}))
    .unwrap();
    for (a, v) in [("P2", 1.0), ("P3", 59.0), ("P4", 60.0), ("Q2", 2.0)] {
        assert_eq!(c.snapshot()["calculated"]["office-1"][a], v);
    }
    let folder = std::env::temp_dir().join("folio-early-date-tests");
    std::fs::create_dir_all(&folder).unwrap();
    c.dispatch(json!({"op":"save","path":folder.join("early.folio")}))
        .unwrap();
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":folder.join("early.xlsx")}))
        .unwrap();
    let before = c.snapshot();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"office-1","address":"A62","value":"59"}]}))
        .unwrap();
    assert_eq!(c.snapshot()["calculated"]["office-1"]["B62"], 28.0);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.snapshot()["calculated"], before["calculated"]);
    c.dispatch(json!({"op":"load","path":folder.join("early.folio")}))
        .unwrap();
    assert_eq!(c.snapshot()["calculated"], before["calculated"]);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":folder.join("reopened.xlsx")}))
        .unwrap();
    assert_eq!(
        std::fs::read(folder.join("early.xlsx")).unwrap(),
        std::fs::read(folder.join("reopened.xlsx")).unwrap()
    );
}
