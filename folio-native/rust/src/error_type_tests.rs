use crate::{office::Package, workspace::Core};
use serde_json::{json, Value};

const ERRORS: [&str; 7] = [
    "#NULL!", "#DIV/0!", "#VALUE!", "#REF!", "#NAME?", "#NUM!", "#N/A",
];
fn folder() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-error-type-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn fixture() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":format!("{}/../test/fixtures/error-types.xlsx",env!("CARGO_MANIFEST_DIR"))})).unwrap();
    c
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = folder().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn edit(c: &mut Core, address: &str, value: &str) {
    c.dispatch(
        json!({"op":"cells","cells":[{"sheetId":"office-1","address":address,"value":value}]}),
    )
    .unwrap();
}
fn assert_types(c: &mut Core) {
    let snapshot = c.snapshot();
    let values = &snapshot["calculated"]["office-1"];
    for (index, error) in ERRORS.iter().enumerate() {
        let row = index + 2;
        for col in ["B", "C", "D", "E", "K"] {
            assert_eq!(values[format!("{col}{row}")], *error, "{col}{row}");
        }
        for (col, expected) in [("F", true), ("G", false), ("H", true), ("I", false)] {
            assert_eq!(values[format!("{col}{row}")], expected, "{col}{row}");
        }
        assert_eq!(values[format!("J{row}")], "handled");
        assert_eq!(values[format!("L{row}")], (index + 1) as f64);
        assert_eq!(
            snapshot["formatted"]["office-1"][format!("B{row}")]["text"],
            *error
        );
        assert_eq!(
            snapshot["formatted"]["office-1"][format!("C{row}")]["text"],
            format!("TEXT: {error}")
        );
    }
    for a in ["C11", "C12"] {
        assert_eq!(c.workspace["sheets"][0]["cells"][a], "'#N/A");
    }
    assert_eq!(c.workspace["sheets"][0]["cells"]["C13"], "'#n/a");
}
fn cell_type(p: &Package, a: &str) -> String {
    let xml = p.text("xl/worksheets/sheet1.xml").unwrap();
    let d = crate::office::parse(xml).unwrap();
    let c = d
        .descendants()
        .find(|n| n.has_tag_name("c") && n.attribute("r") == Some(a))
        .unwrap();
    c.attribute("t").unwrap_or("n").into()
}

#[test]
fn imported_errors_and_error_shaped_text_keep_distinct_semantics() {
    let mut c = fixture();
    assert_types(&mut c);
    let original = export(&mut c, "unchanged.xlsx");
    assert_eq!(
        original.original,
        std::fs::read(format!(
            "{}/../test/fixtures/error-types.xlsx",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    );
    edit(&mut c, "P2", "1");
    let p = export(&mut c, "edited.xlsx");
    for row in 2..=8 {
        assert_eq!(cell_type(&p, &format!("D{row}")), "e");
        assert_eq!(cell_type(&p, &format!("E{row}")), "str");
    }
    assert_eq!(cell_type(&p, "N2"), "str");
    edit(&mut c, "P2", "2");
    assert_eq!(cell_type(&export(&mut c, "numeric.xlsx"), "N2"), "n");
    edit(&mut c, "P2", "0");
    let p = export(&mut c, "error.xlsx");
    assert_eq!(cell_type(&p, "N2"), "e");
    assert_types(&mut c);
    let path = folder().join("errors.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut reopened = Core::new();
    reopened.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_types(&mut reopened);
    assert_eq!(export(&mut reopened, "reopened.xlsx").original, p.original);
    let mut imported = Core::new();
    imported
        .dispatch(json!({"op":"importOffice","path":folder().join("edited.xlsx")}))
        .unwrap();
    assert_types(&mut imported);
}

#[test]
fn new_workbook_errors_export_as_errors_and_literal_text_stays_text() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let id = c.workspace["sheets"][0]["id"].clone();
    let mut cells = vec![];
    for (i, error) in ERRORS.iter().enumerate() {
        for (col, value) in [
            ("A", error.to_string()),
            ("B", format!("'{error}")),
            ("C", format!("={error}")),
            ("D", format!("=\"{error}\"")),
        ] {
            cells.push(json!({"sheetId":id,"address":format!("{col}{}",i+1),"value":value}));
        }
    }
    c.dispatch(json!({"op":"cells","cells":cells})).unwrap();
    export(&mut c, "new.xlsx");
    c.dispatch(json!({"op":"importOffice","path":folder().join("new.xlsx")}))
        .unwrap();
    for i in 1..=7 {
        assert_eq!(
            c.workspace["sheets"][0]["cells"][format!("B{i}")],
            format!("'{}", ERRORS[i - 1])
        );
        c.dispatch(json!({"op":"cells","cells":[{"sheetId":"office-1","address":format!("E{i}"),"value":format!("=AND(ISERROR(A{i}),ISTEXT(B{i}),ISERROR(C{i}),ISTEXT(D{i}))")}]})).unwrap();
        assert_eq!(
            c.snapshot()["calculated"]["office-1"][format!("E{i}")],
            true
        );
    }
}

#[test]
fn paste_values_retains_error_types_history_and_package_recalculation() {
    let mut c = fixture();
    c.dispatch(json!({"op":"importOffice","path":format!("{}/../test/fixtures/error-types-structure.xlsx",env!("CARGO_MANIFEST_DIR"))})).unwrap();
    let copy = c
        .dispatch(json!({"op":"copyCells","sheetId":"office-1","range":"D2:E8"}))
        .unwrap();
    let before = c.workspace.clone();
    c.dispatch(json!({"op":"pasteCells","sheetId":"office-1","range":"R2","mode":"values","token":copy["token"]})).unwrap();
    let after = c.workspace["sheets"].clone();
    for (i, error) in ERRORS.iter().enumerate() {
        assert_eq!(after[0]["cells"][format!("R{}", i + 2)], *error);
        assert_eq!(
            after[0]["cells"][format!("S{}", i + 2)],
            format!("'{error}")
        );
    }
    let p = export(&mut c, "pasted.xlsx");
    for row in 2..=8 {
        assert_eq!(cell_type(&p, &format!("R{row}")), "e");
        assert_eq!(cell_type(&p, &format!("S{row}")), "inlineStr");
    }
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["sheets"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["sheets"], after);
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertRows","at":2,"count":1}),
    )
    .unwrap();
    let p = export(&mut c, "shifted.xlsx");
    assert_eq!(cell_type(&p, "R3"), "e");
    assert_eq!(cell_type(&p, "D3"), "e");
    assert_eq!(cell_type(&p, "E3"), "str");
    c.dispatch(json!({"op":"fillCells","sheetId":"office-1","range":"D3:E10","direction":"down"}))
        .unwrap();
    let p = export(&mut c, "filled.xlsx");
    for row in 3..=10 {
        assert_eq!(cell_type(&p, &format!("D{row}")), "e");
        assert_eq!(cell_type(&p, &format!("E{row}")), "str");
    }
}

#[test]
fn formula_cache_types_change_with_same_visible_result_and_keep_metadata() {
    let mut c = fixture();
    assert_eq!(c.snapshot()["formatted"]["office-1"]["N2"]["text"], "#N/A");
    edit(&mut c, "P2", "1");
    assert_eq!(
        c.snapshot()["formatted"]["office-1"]["N2"]["text"],
        "TEXT: #N/A"
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.snapshot()["formatted"]["office-1"]["N2"]["text"], "#N/A");
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(
        c.snapshot()["formatted"]["office-1"]["N2"]["text"],
        "TEXT: #N/A"
    );
    // A changed formula uses the cell patch path, not the cache-refresh path.
    edit(&mut c, "N2", "=1/0");
    let p = export(&mut c, "changed-formula.xlsx");
    assert_eq!(cell_type(&p, "N2"), "e");
    edit(&mut c, "N2", "=\"#DIV/0!\"");
    assert_eq!(cell_type(&export(&mut c, "changed-text.xlsx"), "N2"), "str");
    edit(&mut c, "N2", "=N2");
    let p = export(&mut c, "engine-diagnostic.xlsx");
    let d = crate::office::parse(p.text("xl/worksheets/sheet1.xml").unwrap()).unwrap();
    let n = d
        .descendants()
        .find(|n| n.attribute("r") == Some("N2"))
        .unwrap();
    assert!(n.attribute("t").is_none());
    assert!(n.children().all(|n| !n.has_tag_name("v")));
    assert_eq!(
        n.children().find(|n| n.has_tag_name("f")).unwrap().text(),
        Some("N2")
    );
}

#[test]
fn legacy_imported_error_shaped_text_is_repaired_without_replacing_edits() {
    let mut c = fixture();
    let path = folder().join("legacy.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for a in ["C2", "C11", "C12"] {
        let text = saved["sheets"][0]["cells"][a]
            .as_str()
            .unwrap()
            .trim_start_matches('\'')
            .to_string();
        saved["sheets"][0]["cells"][a] = json!(text);
        saved["officeWorkbook"]["baseline"][0]["cells"][a] = json!(text);
    }
    saved["sheets"][0]["cells"]["C12"] = json!("actual edit");
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    c.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["C2"], "'#NULL!");
    assert_eq!(c.workspace["sheets"][0]["cells"]["C11"], "'#N/A");
    assert_eq!(c.workspace["sheets"][0]["cells"]["C12"], "actual edit");
    export(&mut c, "legacy.xlsx");
}
