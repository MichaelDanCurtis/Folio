use crate::{
    office::{parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
fn core() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = c.workspace.clone();
    w["sheets"] = json!([
        {"id":"s","name":"Source","cells":{"A1":"0.25","A2":"=A1*2","A3":"'001","B1":"8","D1":"99","D2":"=D1+1","D3":"keep","D4":"preserve","E1":"21"},
        "cellStyles":{"A1":{"bold":true,"numFmt":"0.00%","fill":"#FF0000"},"A2":{"bold":true,"numFmt":"0.0000"},"A3":{"numFmt":"@"},"D1":{"italic":true,"numFmt":"0.0","fill":"#0000FF"},"D2":{"italic":true},"D3":{"italic":true}},
        "columnWidths":{"1":10.5,"4":30,"5":31},"hiddenColumns":[4],"defaultColumnWidth":16.25},
        {"id":"t","name":"Target","cells":{"B1":"5","B2":"6"},"columnWidths":{"2":50},"defaultColumnWidth":20}
    ]);
    for sheet in w["sheets"].as_array_mut().unwrap() {
        sheet["tables"] = json!([]);
        sheet["rowHeightModes"] = json!({});
        sheet["hiddenRows"] = json!([]);
        sheet["autoFilter"] = Value::Null;
    }
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    c
}
fn copy(c: &mut Core, range: &str) -> Value {
    c.dispatch(json!({"op":"copyCells","sheetId":"s","range":range}))
        .unwrap()
}
fn paste(c: &mut Core, clip: &Value, range: &str, mode: &str) -> Value {
    c.dispatch(
        json!({"op":"pasteCells","sheetId":"s","range":range,"mode":mode,"token":clip["token"]}),
    )
    .unwrap()
}
fn folder() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-paste-properties-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = folder().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn validation(c: &mut Core, range: &str, rule: Value) {
    c.dispatch(json!({"op":"dataValidation","sheetId":"s","baseRevision":c.workspace["revision"],"action":"set","range":range,"rule":rule})).unwrap();
}
#[test]
fn number_format_pastes_keep_destination_decoration_and_formula_value_types() {
    for mode in ["valuesNumberFormats", "formulasNumberFormats"] {
        let mut c = core();
        let clip = copy(&mut c, "A1:A3");
        let before = c.workspace.clone();
        let result = paste(&mut c, &clip, "D1", mode);
        let s = &c.workspace["sheets"][0];
        assert_eq!(s["cells"]["D1"], "0.25");
        assert_eq!(
            s["cells"]["D2"],
            if mode == "valuesNumberFormats" {
                "0.5"
            } else {
                "=D1*2"
            }
        );
        assert_eq!(result["calculated"]["s"]["D2"], 0.5);
        assert_eq!(s["cells"]["D3"], "'001");
        assert_eq!(s["cellStyles"]["D1"]["numFmt"], "0.00%");
        assert_eq!(s["cellStyles"]["D2"]["numFmt"], "0.0000");
        assert_eq!(s["cellStyles"]["D1"]["italic"], true);
        assert_eq!(s["cellStyles"]["D1"]["bold"], false);
        assert_eq!(s["cellStyles"]["D1"]["fill"], "#0000FF");
        assert_eq!(s["columnWidths"], before["sheets"][0]["columnWidths"]);
        export(&mut c, &format!("{mode}.xlsx"));
        let after = c.workspace.clone();
        c.dispatch(json!({"op":"undo"})).unwrap();
        assert_eq!(c.workspace["sheets"], before["sheets"]);
        c.dispatch(json!({"op":"redo"})).unwrap();
        assert_eq!(c.workspace["sheets"], after["sheets"]);
    }
}
#[test]
fn number_formats_preserve_snapshot_blanks_transpose_tiling_and_real_errors() {
    let mut c = core();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"s","address":"A3","value":"=1/0"},{"sheetId":"s","address":"B2","value":"=\"#DIV/0!\""}]})).unwrap();
    let clip = copy(&mut c, "A1:B3");
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"s","address":"A1","value":"100"}]}))
        .unwrap();
    let r=c.dispatch(json!({"op":"pasteCells","sheetId":"s","range":"D1:I2","mode":"valuesNumberFormats","token":clip["token"],"transpose":true,"skipBlanks":true})).unwrap();
    assert_eq!(r["calculated"]["s"]["D1"], 0.25);
    assert_eq!(r["calculated"]["s"]["E1"], 0.5);
    assert_eq!(r["calculated"]["s"]["F1"], "#DIV/0!");
    assert_eq!(c.workspace["sheets"][0]["cells"]["F1"], "#DIV/0!");
    assert_eq!(c.workspace["sheets"][0]["cells"]["E2"], "'#DIV/0!");
    assert_eq!(r["calculated"]["s"]["G1"], 0.25);
    export(&mut c, "number-transpose.xlsx");
    let clip = copy(&mut c, "B3");
    paste(&mut c, &clip, "D1", "valuesNumberFormats");
    assert_eq!(c.workspace["sheets"][0]["cells"]["D1"], "");
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"]["D1"]["numFmt"],
        "General"
    );
    assert_eq!(c.workspace["sheets"][0]["cellStyles"]["D1"]["italic"], true);
}
#[test]
fn validation_only_replaces_rules_without_cell_or_format_edits_and_reanchors_ranges() {
    let mut c = core();
    validation(
        &mut c,
        "A1:A4",
        json!({"type":"custom","formula1":"A1>0","allowBlank":true,"showErrorMessage":true,"errorStyle":"stop"}),
    );
    validation(
        &mut c,
        "D1:D1048576",
        json!({"type":"whole","formula1":"D1","formula2":"100"}),
    );
    let clip = copy(&mut c, "A2:A3");
    let before = c.workspace.clone();
    paste(&mut c, &clip, "D2", "validation");
    for key in ["cells", "columnWidths", "hiddenColumns"] {
        assert_eq!(c.workspace["sheets"][0][key], before["sheets"][0][key]);
    }
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"]["D1"]["fill"],
        "#0000FF"
    );
    let rule = crate::data_validation::at(&c.workspace["sheets"][0], "D2").unwrap();
    assert_eq!(rule["formula1"], "D2>0");
    assert_eq!(rule["sqref"], "D2:D3");
    assert_eq!(
        crate::data_validation::at(&c.workspace["sheets"][0], "D4").unwrap()["sqref"],
        "D1:D1 D4:D1048576"
    );
    assert_eq!(c.dispatch(json!({"op":"validationEntryCheck","sheetId":"s","address":"D2","value":"-1","baseRevision":c.workspace["revision"]})).unwrap()["valid"],false);
    export(&mut c, "validation.xlsx");
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["sheets"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    let clip = copy(&mut c, "B1:B2");
    c.dispatch(json!({"op":"pasteCells","sheetId":"s","range":"D2","mode":"validation","token":clip["token"],"skipBlanks":true})).unwrap();
    assert!(crate::data_validation::at(&c.workspace["sheets"][0], "D2").is_none());
    assert!(crate::data_validation::at(&c.workspace["sheets"][0], "D3").is_some());
    export(&mut c, "validation-skipped.xlsx");
}
#[test]
fn width_only_and_keep_widths_preserve_hidden_state_snapshot_tiling_and_recovery() {
    let mut c = core();
    let clip = copy(&mut c, "A1:B2");
    let before = c.workspace.clone();
    paste(&mut c, &clip, "D1:G4", "columnWidths");
    let s = &c.workspace["sheets"][0];
    assert_eq!(s["cells"], before["sheets"][0]["cells"]);
    assert_eq!(s["hiddenColumns"], before["sheets"][0]["hiddenColumns"]);
    for (col, width) in [("4", 10.5), ("5", 16.25), ("6", 10.5), ("7", 16.25)] {
        assert_eq!(s["columnWidths"][col], width);
    }
    export(&mut c, "widths.xlsx");
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["sheets"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    c.dispatch(json!({"op":"pasteCells","sheetId":"t","range":"B1","mode":"allColumnWidths","token":clip["token"]})).unwrap();
    assert_eq!(c.workspace["sheets"][1]["columnWidths"]["2"], 10.5);
    assert_eq!(c.workspace["sheets"][1]["columnWidths"]["3"], 16.25);
    assert_eq!(c.workspace["sheets"][1]["cells"]["B2"], "=B1*2");
    let path = folder().join("widths.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut reopened = Core::new();
    reopened.dispatch(json!({"op":"load","path":path})).unwrap();
    // Loading materializes optional null defaults (for example frozenPane).
    for (actual, expected) in reopened.workspace["sheets"]
        .as_array()
        .unwrap()
        .iter()
        .zip(c.workspace["sheets"].as_array().unwrap())
    {
        for key in actual
            .as_object()
            .unwrap()
            .keys()
            .chain(expected.as_object().unwrap().keys())
        {
            assert_eq!(actual[key], expected[key], "recovered {key}");
        }
    }
    export(&mut reopened, "widths-reopened.xlsx");
}
#[test]
fn number_format_transfer_resolves_inheritance_and_keeps_raw_builtin_ids_and_dependencies() {
    let mut c = core();
    let mut source = export(&mut c, "style-source.xlsx");
    source.set("xl/styles.xml",r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="1"><numFmt numFmtId="164" formatCode="0.00000&quot; units&quot;"/></numFmts><fonts count="1"><font/></fonts><fills count="1"><fill/></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf numFmtId="164"/></cellStyleXfs><cellXfs count="3"><xf numFmtId="165" xfId="0" applyNumberFormat="0"/><xf numFmtId="50"/><xf numFmtId="0"/></cellXfs></styleSheet>"#.into());
    let mut target = export(&mut c, "style-target.xlsx");
    let original = target.text("xl/styles.xml").unwrap().to_string();
    let projected = crate::xlsx_styles::read(&target);
    let base = projected
        .iter()
        .position(|p| p["italic"] == true && p["fill"] == "#0000FF")
        .unwrap()
        .to_string();
    let id = crate::xlsx_styles::transfer_number_format(&source, &mut target, "0", &base).unwrap();
    let style = &crate::xlsx_styles::read(&target)[id.parse::<usize>().unwrap()];
    assert_eq!(style["numFmt"], "0.00000\" units\"");
    assert_eq!(style["italic"], true);
    assert_eq!(style["fill"], "#0000FF");
    let id = crate::xlsx_styles::transfer_number_format(&source, &mut target, "1", &base).unwrap();
    let d = parse(target.text("xl/styles.xml").unwrap()).unwrap();
    let xf = d
        .descendants()
        .find(|n| n.tag_name().name() == "cellXfs")
        .unwrap()
        .children()
        .filter(|n| n.is_element())
        .nth(id.parse().unwrap())
        .unwrap();
    assert_eq!(xf.attribute("numFmtId"), Some("50"));
    for name in ["fonts", "fills", "borders", "cellStyleXfs"] {
        let old = parse(&original).unwrap();
        assert_eq!(
            old.descendants()
                .find(|n| n.tag_name().name() == name)
                .unwrap()
                .children()
                .filter(|n| n.is_element())
                .count(),
            d.descendants()
                .find(|n| n.tag_name().name() == name)
                .unwrap()
                .children()
                .filter(|n| n.is_element())
                .count()
        );
    }
    std::fs::write(
        folder().join("number-format-components.xlsx"),
        target.bytes().unwrap(),
    )
    .unwrap();
}
#[test]
fn metadata_paste_rejects_invalid_requests_atomically_and_captures_default_width() {
    let mut c = core();
    let clip = copy(&mut c, "A1:B2");
    let before = c.workspace.clone();
    for request in [
        json!({"mode":"columnWidths","transpose":true,"token":clip["token"]}),
        json!({"mode":"allColumnWidths","transpose":true,"token":clip["token"]}),
        json!({"mode":"validation","text":"hello"}),
        json!({"mode":"valuesNumberFormats","text":"2"}),
        json!({"mode":"formulasNumberFormats","token":"expired"}),
    ] {
        let mut r = request;
        r["op"] = json!("pasteCells");
        r["sheetId"] = json!("s");
        r["range"] = json!("D1");
        assert!(c.dispatch(r).is_err());
        assert_eq!(c.workspace, before);
    }
    let clip = c
        .dispatch(
            json!({"op":"copyCells","sheetId":"s","range":"B1","sourceDefaultColumnWidth":12.5}),
        )
        .unwrap();
    paste(&mut c, &clip, "H1", "columnWidths");
    assert_eq!(c.workspace["sheets"][0]["columnWidths"]["8"], 16.25);
    let mut w = c.workspace.clone();
    w["sheets"][0]
        .as_object_mut()
        .unwrap()
        .remove("defaultColumnWidth");
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    let clip = c
        .dispatch(
            json!({"op":"copyCells","sheetId":"s","range":"B1:B2","sourceDefaultColumnWidth":12.5}),
        )
        .unwrap();
    paste(&mut c, &clip, "I1:J1", "columnWidths");
    assert_eq!(c.workspace["sheets"][0]["columnWidths"]["9"], 12.5);
    assert_eq!(c.workspace["sheets"][0]["columnWidths"]["10"], 12.5);
    assert!(c
        .dispatch(
            json!({"op":"copyCells","sheetId":"s","range":"B1","sourceDefaultColumnWidth":999})
        )
        .is_err());
    let clip = c
        .dispatch(
            json!({"op":"copyCells","sheetId":"s","range":"A1:B2","sourceDefaultColumnWidth":12.5}),
        )
        .unwrap();
    let before = c.workspace.clone();
    let staged=c.dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"pasteCells","sheetId":"s","range":"D1","mode":"columnWidths","token":clip["token"]}})).unwrap();
    assert_eq!(c.workspace, before);
    assert!(!staged["layoutRows"]["s"].as_array().unwrap().is_empty());
}

#[test]
fn imported_paste_properties_keep_borders_validation_prompts_column_metadata_and_merges() {
    for mode in [
        "valuesNumberFormats",
        "formulasNumberFormats",
        "validation",
        "columnWidths",
        "allColumnWidths",
    ] {
        let mut c = Core::new();
        c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/tests/fixtures/paste-properties-openpyxl.xlsx")})).unwrap();
        let id = c.workspace["sheets"][0]["id"].clone();
        let clip = c
            .dispatch(json!({"op":"copyCells","sheetId":id,"range":"A1:B2"}))
            .unwrap();
        c.dispatch(
            json!({"op":"pasteCells","sheetId":id,"range":"D1","token":clip["token"],"mode":mode}),
        )
        .unwrap();
        export(&mut c, &format!("imported-{mode}.xlsx"));
        if mode == "validation" || mode == "columnWidths" {
            c.dispatch(json!({"op":"pasteCells","sheetId":id,"range":"H1","token":clip["token"],"mode":mode})).unwrap();
            assert_eq!(c.workspace["sheets"][0]["mergedRanges"], json!(["H1:I1"]));
            assert_eq!(c.workspace["sheets"][0]["cells"]["H1"], "Merged heading");
        }
    }
}
#[test]
fn validation_copy_uses_destination_sheet_catalog_and_rejects_missing_explicit_sheets() {
    let sheets = json!([{"name":"Source","dataValidations":[{"type":"custom","sqref":"A1:A3","formula1":"A1>0"}]}]);
    let mut target = json!({"dataValidations":[{"type":"custom","sqref":"D1:D4","formula1":"'Only target'!D1>0"}]});
    let names = vec!["First".to_string(), "Only target".to_string()];
    crate::data_validation::copy_rules(
        &sheets,
        0,
        &mut target,
        &[("A1".into(), "D1".into())],
        None,
        Some((&names, 1)),
    )
    .unwrap();
    assert_eq!(
        target["dataValidations"][0]["formula1"],
        "'Only target'!D2>0"
    );
    let mut missing = sheets.clone();
    missing[0]["dataValidations"][0]["formula1"] = json!("Source!A1>0");
    let before = target.clone();
    let error = crate::data_validation::copy_rules(
        &missing,
        0,
        &mut target,
        &[("A1".into(), "D1".into())],
        None,
        Some((&names, 1)),
    )
    .unwrap_err();
    assert!(error.contains("missing"));
    assert_eq!(target, before);
}
