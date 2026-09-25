use crate::{office::Package, workspace::Core};
use serde_json::{json, Value};

fn fill(c: &mut Core, range: &str, direction: &str) -> Value {
    c.dispatch(json!({"op":"fillCells","sheetId":c.workspace["sheets"][0]["id"],"range":range,"direction":direction,"baseRevision":c.workspace["revision"]})).unwrap()
}
fn output() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-fill-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn export(c: &mut Core, name: &str) -> Package {
    let p = output().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":p}))
        .unwrap();
    Package::read(p.to_str().unwrap()).unwrap()
}

#[test]
fn directional_fill_translates_formulas_preserves_blanks_and_undo() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = c.workspace.clone();
    w["sheets"] = json!([
      {"id":"sheet-1","name":"Sheet 1","cells":{"A1":"2","B1":"=A1+$A1+A$1+$A$1+'Source data'!A1+LEN(\"A1\")","A2":"7","B2":"old","C2":"erase","B3":"old","C3":"erase","F1":"4","G1":"5"},"cellStyles":{"B1":{"bold":true,"numFmt":"0.00"},"C2":{"fill":"#FFFF00"}},"hiddenRows":[3]},
      {"id":"sheet-2","name":"Source data","cells":{"A1":"10","A2":"20","A3":"30"}}
    ]);
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    let before = c.workspace.clone();
    let result = fill(&mut c, "B1:C3", "down");
    assert_eq!(
        c.workspace["sheets"][0]["cells"]["B2"],
        "=A2+$A2+A$1+$A$1+'Source data'!A2+LEN(\"A1\")"
    );
    assert_eq!(result["calculated"]["sheet-1"]["B2"], 40.0);
    assert_eq!(c.workspace["sheets"][0]["cells"]["C2"], "");
    assert!(c.workspace["sheets"][0]["cellStyles"].get("C2").is_none());
    assert_eq!(c.workspace["sheets"][0]["cellStyles"]["B3"]["bold"], true);
    assert_eq!(c.workspace["sheets"][0]["hiddenRows"], json!([3]));
    assert_eq!(
        c.workspace["revision"].as_u64().unwrap(),
        before["revision"].as_u64().unwrap() + 1
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["sheets"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["C3"], "");
    fill(&mut c, "F1:H2", "right");
    assert_eq!(c.workspace["sheets"][0]["cells"]["H1"], "4");
    assert_eq!(c.workspace["sheets"][0]["cells"]["H2"], "");
    fill(&mut c, "A1:A2", "up");
    assert_eq!(c.workspace["sheets"][0]["cells"]["A1"], "7");
    fill(&mut c, "D1:F1", "left");
    assert_eq!(c.workspace["sheets"][0]["cells"]["D1"], "4");
    fill(&mut c, "F2", "down");
    assert_eq!(c.workspace["sheets"][0]["cells"]["F2"], "4");
    fill(&mut c, "G2", "right");
    assert_eq!(c.workspace["sheets"][0]["cells"]["G2"], "4");
}

#[test]
fn fill_rejects_invalid_stale_and_unsupported_formula_atomically() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    for (range, direction) in [
        ("A1", "down"),
        ("A1", "right"),
        ("XFD1", "left"),
        ("A1048576", "up"),
        ("A1:A100001", "down"),
        ("C3:A1", "right"),
        ("A1:A2", "sideways"),
    ] {
        let old = c.workspace.clone();
        assert!(c
            .dispatch(
                json!({"op":"fillCells","sheetId":"sheet-1","range":range,"direction":direction})
            )
            .is_err());
        assert_eq!(c.workspace, old);
    }
    assert!(c.dispatch(json!({"op":"fillCells","sheetId":"sheet-1","range":"A1:A2","direction":"down","baseRevision":999})).is_err());
    c.dispatch(
        json!({"op":"cells","cells":[{"sheetId":"sheet-1","address":"A1","value":"=SUM("}]}),
    )
    .unwrap();
    let old = c.workspace.clone();
    assert!(c
        .dispatch(json!({"op":"fillCells","sheetId":"sheet-1","range":"A1:A3","direction":"down"}))
        .is_err());
    assert_eq!(c.workspace, old);
}

#[test]
fn imported_fill_retains_complete_formats_rich_text_shared_formulas_and_recovery() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/fill.xlsx")})).unwrap();
    let original = export(&mut c, "original.xlsx");
    fill(&mut c, "B2:E5", "down");
    let current = c.workspace["sheets"].clone();
    assert_eq!(current[0]["cells"]["B5"], "=A5*$H$1");
    assert_eq!(current[0]["cells"]["C5"], "Rich title");
    assert_eq!(current[0]["cells"]["E5"], "");
    assert_eq!(c.snapshot()["calculated"]["office-1"]["B5"], 30.0);
    let edited = export(&mut c, "filled.xlsx");
    for part in ["xl/theme/theme1.xml", "docProps/core.xml"] {
        assert_eq!(edited.entries[part], original.entries[part]);
    }
    let saved = output().join("filled.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(loaded.workspace["sheets"], current);
    assert_eq!(
        export(&mut loaded, "reopened.xlsx").original,
        edited.original
    );
    let mut imported = Core::new();
    imported
        .dispatch(json!({"op":"importOffice","path":output().join("filled.xlsx")}))
        .unwrap();
    assert_eq!(
        export(&mut imported, "reimported.xlsx").original,
        edited.original
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(export(&mut c, "undone.xlsx").original, original.original);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(export(&mut c, "redone.xlsx").original, edited.original);
    for (range, expected) in [("J2:K3", "merged"), ("M2:M4", "array")] {
        let old = c.workspace.clone();
        let error = c
            .dispatch(
                json!({"op":"fillCells","sheetId":"office-1","range":range,"direction":"down"}),
            )
            .unwrap_err();
        assert!(error.contains(expected), "{error}");
        assert_eq!(c.workspace, old);
    }
}

#[test]
fn fill_copies_inherited_styles_and_prefixed_cells_and_rejects_protection() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/fill-prefixed.xlsx")})).unwrap();
    fill(&mut c, "B2:E5", "down");
    fill(&mut c, "P2:P4", "down");
    fill(&mut c, "R7:R9", "down");
    let inherited = export(&mut c, "inherited.xlsx");
    assert_eq!(c.workspace["sheets"][0]["cells"]["P4"], "Column style");
    assert_eq!(c.workspace["sheets"][0]["cellStyles"]["P4"]["italic"], true);
    assert_eq!(c.workspace["sheets"][0]["cellStyles"]["R9"]["bold"], true);
    let mut protected = inherited.clone();
    let path = "xl/worksheets/sheet1.xml";
    let xml = protected.text(path).unwrap().replace(
        "</x:sheetData>",
        "</x:sheetData><x:sheetProtection sheet=\"1\"/>",
    );
    protected.set(path, xml);
    std::fs::write(output().join("protected.xlsx"), protected.bytes().unwrap()).unwrap();
    c.dispatch(json!({"op":"importOffice","path":output().join("protected.xlsx")}))
        .unwrap();
    let old = c.workspace.clone();
    let err = c
        .dispatch(json!({"op":"fillCells","sheetId":"office-1","range":"A2:A4","direction":"down"}))
        .unwrap_err();
    assert!(err.contains("protected"), "{err}");
    assert_eq!(c.workspace, old);
}

#[test]
fn fill_formula_directions_boundaries_and_structured_rows() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = c.workspace.clone();
    w["sheets"][0]["cells"] =
        json!({"B2":"=A1+$A1+A$1+$A$1","B5":"=A5","D8":"=D9","XFC1048576":"=XFC1048575"});
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    fill(&mut c, "B2:D2", "right");
    assert_eq!(c.workspace["sheets"][0]["cells"]["D2"], "=C1+$A1+C$1+$A$1");
    fill(&mut c, "B3:B5", "up");
    assert_eq!(c.workspace["sheets"][0]["cells"]["B3"], "=A3");
    fill(&mut c, "B8:D8", "left");
    assert_eq!(c.workspace["sheets"][0]["cells"]["B8"], "=B9");
    fill(&mut c, "B1:B2", "up");
    assert!(c.workspace["sheets"][0]["cells"]["B1"]
        .as_str()
        .unwrap()
        .contains("#REF!"));
    fill(&mut c, "XFC1048576:XFD1048576", "right");
    assert_eq!(
        c.workspace["sheets"][0]["cells"]["XFD1048576"],
        "=XFD1048575"
    );
    let changes = [
        ("A10", "Qty"),
        ("B10", "Price"),
        ("C10", "Amount"),
        ("A11", "2"),
        ("B11", "3"),
        ("C11", "=[@Qty]*[@Price]"),
        ("A12", "4"),
        ("B12", "5"),
    ]
    .map(|(a, v)| json!({"sheetId":"sheet-1","address":a,"value":v}));
    c.dispatch(json!({"op":"cells","cells":changes})).unwrap();
    c.dispatch(json!({"op":"table","action":"create","sheetId":"sheet-1","range":"A10:C12","name":"Sales"})).unwrap();
    let result = fill(&mut c, "C11:C12", "down");
    assert_eq!(result["calculated"]["sheet-1"]["C12"], 20.0);
    let old = c.workspace.clone();
    assert!(c
        .dispatch(
            json!({"op":"fillCells","sheetId":"sheet-1","range":"C11:D11","direction":"right"})
        )
        .unwrap_err()
        .contains("table-column"));
    assert_eq!(c.workspace, old);
}

#[test]
fn the_fill_handle_repeats_the_source_block() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let id = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"cells","cells":[
        {"sheetId":id,"address":"A1","value":"1"},
        {"sheetId":id,"address":"A2","value":"x"},
        {"sheetId":id,"address":"B1","value":"=A1*2"},
        {"sheetId":id,"address":"B2","value":"=A2&\"!\""},
        {"sheetId":id,"address":"E5","value":"top"},
        {"sheetId":id,"address":"E6","value":"=E5"}
    ]}))
    .unwrap();
    let handle = |c: &mut Core, range: &str, source: &str, direction: &str| {
        c.dispatch(
            json!({"op":"fillCells","sheetId":id,"range":range,"source":source,
            "direction":direction,"baseRevision":c.workspace["revision"]}),
        )
    };
    handle(&mut c, "A1:B5", "A1:B2", "down").unwrap();
    let cells = c.workspace["sheets"][0]["cells"].clone();
    assert_eq!(cells["A3"], "1");
    assert_eq!(cells["A4"], "x");
    assert_eq!(cells["A5"], "1");
    assert_eq!(cells["B3"], "=A3*2", "formulas move with each copy");
    assert_eq!(cells["B4"], "=A4&\"!\"");

    // Up: the block stays anchored at the source.
    handle(&mut c, "E2:E6", "E5:E6", "up").unwrap();
    let cells = c.workspace["sheets"][0]["cells"].clone();
    assert_eq!(cells["E4"], "=E3");
    assert_eq!(cells["E3"], "top");
    assert_eq!(cells["E2"], "=E1");

    // Right, across columns.
    handle(&mut c, "A1:D1", "A1:B1", "right").unwrap();
    let cells = c.workspace["sheets"][0]["cells"].clone();
    assert_eq!(cells["C1"], "1");
    assert_eq!(cells["D1"], "=C1*2");

    let before = c.workspace.clone();
    for (range, source, direction) in [
        ("A1:B5", "A2:B3", "down"),
        ("A1:C5", "A1:B2", "down"),
        ("A1:B5", "A1:B2", "right"),
        ("A1:B5", "not a range", "down"),
    ] {
        assert!(
            handle(&mut c, range, source, direction).is_err(),
            "{range} {source}"
        );
        assert_eq!(c.workspace, before);
    }
}
