use crate::{office::Package, workspace::Core};
use serde_json::{json, Value};
fn fixture() -> Core {
    let mut core = Core::new();
    core.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/row-reflow.xlsx")})).unwrap();
    core
}
fn prepare(c: &mut Core, edit: Value) -> Value {
    c.dispatch(json!({"op":"prepareLayoutEdit","edit":edit}))
        .unwrap()
}
fn measurements(p: &Value, height: f64) -> Value {
    let mut sizes = json!({});
    for (id, rows) in p["layoutRows"].as_object().unwrap() {
        sizes[id] = json!({});
        for r in rows.as_array().unwrap() {
            sizes[id][r.to_string()] = json!(height);
        }
    }
    sizes
}
fn commit(c: &mut Core, p: &Value) {
    c.dispatch(
        json!({"op":"commitLayoutEdit","token":p["layoutToken"],"sizes":measurements(p, 60.0)}),
    )
    .unwrap();
}

#[test]
fn automatic_rows_include_cross_sheet_results_and_exclude_manual_hidden_merged() {
    let mut c = fixture();
    let before = c.workspace.clone();
    let p = prepare(
        &mut c,
        json!({"op":"cells","cells":[
        {"sheetId":"office-1","address":"B2","value":"One\nTwo\nThree"},
        {"sheetId":"office-1","address":"B3","value":"Manual\nlines"},
        {"sheetId":"office-1","address":"B5","value":"Hidden\nlines"},
        {"sheetId":"office-1","address":"B8","value":"Merged\nlines"},
        {"sheetId":"office-2","address":"A1","value":"Formula\nlines"}]}),
    );
    assert_eq!(p["layoutRows"], json!({"office-1":[2,7],"office-2":[1]}));
    assert_eq!(p["calculated"]["office-1"]["B7"], "Formula\nlines");
    assert_eq!(c.workspace, before);
    commit(&mut c, &p);
    assert_eq!(
        c.workspace["revision"],
        before["revision"].as_u64().unwrap() + 1
    );
    assert_eq!(
        c.workspace["sheets"][0]["rowHeights"]["2"].as_f64(),
        Some(60.0)
    );
    for (r, h) in [(3, 18), (5, 22), (8, 20)] {
        assert_eq!(
            c.workspace["sheets"][0]["rowHeights"][r.to_string()].as_f64(),
            Some(h as f64)
        );
    }
    let after = c.workspace.clone();
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["sheets"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["sheets"], after["sheets"]);
}

#[test]
fn incomplete_invalid_cancelled_and_stale_layouts_never_publish_edits() {
    let mut c = fixture();
    let before = c.workspace.clone();
    let edit = json!({"op":"cells","cells":[{"sheetId":"office-1","address":"B2","value":"new"}]});
    for sizes in [
        json!({}),
        json!({"office-1":{}}),
        json!({"office-1":{"2":0}}),
        json!({"office-1":{"2":410}}),
        json!({"office-1":{"02":30}}),
        json!({"office-1":{"2":30,"3":40}}),
        json!({"office-2":{"2":30}}),
    ] {
        let p = prepare(&mut c, edit.clone());
        assert!(c
            .dispatch(json!({"op":"commitLayoutEdit","token":p["layoutToken"],"sizes":sizes}))
            .is_err());
        assert_eq!(c.workspace, before);
    }
    let p = prepare(&mut c, edit.clone());
    c.dispatch(json!({"op":"cancelLayoutEdit","token":p["layoutToken"]}))
        .unwrap();
    assert!(c
        .dispatch(
            json!({"op":"commitLayoutEdit","token":p["layoutToken"],"sizes":measurements(&p,30.0)})
        )
        .is_err());
    assert_eq!(c.workspace, before);
    let p = prepare(&mut c, edit);
    c.dispatch(json!({"op":"title","title":"Concurrent"}))
        .unwrap();
    let current = c.workspace.clone();
    assert!(c
        .dispatch(
            json!({"op":"commitLayoutEdit","token":p["layoutToken"],"sizes":measurements(&p,30.0)})
        )
        .is_err());
    assert_eq!(c.workspace, current);
    for op in [
        "save",
        "exportOffice",
        "load",
        "new",
        "replace",
        "prepareLayoutEdit",
    ] {
        assert!(c
            .dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":op}}))
            .is_err());
    }
}

#[test]
fn row_modes_follow_explicit_fit_resize_structure_save_and_legacy_loading() {
    let mut c = fixture();
    assert_eq!(c.workspace["sheets"][0]["rowHeightModes"]["3"], "manual");
    assert_eq!(c.workspace["sheets"][0]["rowHeightModes"]["5"], "auto");
    c.dispatch(json!({"op":"sheetGeometry","axis":"rows","action":"autoFit","sheetId":"office-1","at":3,"count":1,"sizes":{"3":18},"baseRevision":c.workspace["revision"]})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["rowHeightModes"]["3"], "auto");
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"deleteRows","at":2,"count":1}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["rowHeightModes"]["2"], "auto");
    c.dispatch(json!({"op":"sheetGeometry","axis":"rows","action":"resize","sheetId":"office-1","at":2,"count":1,"size":18})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["rowHeightModes"]["2"], "manual");
    let dir = std::env::temp_dir().join("folio-row-modes-tests");
    std::fs::create_dir_all(&dir).unwrap();
    c.dispatch(json!({"op":"save","path":dir.join("modes.folio")}))
        .unwrap();
    let mut loaded = Core::new();
    loaded
        .dispatch(json!({"op":"load","path":dir.join("modes.folio")}))
        .unwrap();
    assert_eq!(loaded.workspace, c.workspace);
    let mut saved: Value =
        serde_json::from_slice(&std::fs::read(dir.join("modes.folio")).unwrap()).unwrap();
    for key in ["sheets", "baseline"] {
        let sheets = if key == "sheets" {
            &mut saved["sheets"]
        } else {
            &mut saved["officeWorkbook"]["baseline"]
        };
        for sheet in sheets.as_array_mut().unwrap() {
            sheet.as_object_mut().unwrap().remove("rowHeightModes");
        }
    }
    // Preserve a user's changed legacy height; infer automatic only from an
    // unchanged source height, never merely from a coincidental numeric value.
    saved["sheets"][0]["rowHeights"]["4"] = json!(77);
    std::fs::write(dir.join("legacy.folio"), saved.to_string()).unwrap();
    loaded
        .dispatch(json!({"op":"load","path":dir.join("legacy.folio")}))
        .unwrap();
    assert_eq!(
        loaded.workspace["sheets"][0]["rowHeightModes"]["4"],
        "manual"
    );
    assert_eq!(loaded.workspace["sheets"][0]["rowHeights"]["4"], 77);
    assert_eq!(loaded.workspace["sheets"][0]["rowHeightModes"]["7"], "auto");
}

#[test]
fn automatic_height_export_keeps_source_parts_and_restores_exact_original() {
    let mut c = fixture();
    let dir = std::env::temp_dir().join("folio-row-reflow-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let p = prepare(
        &mut c,
        json!({"op":"cells","cells":[{"sheetId":"office-1","address":"B2","value":"One\nTwo\nThree"}]}),
    );
    commit(&mut c, &p);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":dir.join("edited.xlsx")}))
        .unwrap();
    let pkg = Package::read(dir.join("edited.xlsx").to_str().unwrap()).unwrap();
    let sheets = crate::xlsx::import(&pkg).unwrap().0;
    assert_eq!(sheets[0]["rowHeightModes"]["2"], "auto");
    assert_eq!(sheets[0]["rowHeights"]["2"].as_f64(), Some(60.0));
    assert_eq!(sheets[0]["rowHeightModes"]["3"], "manual");
    c.dispatch(json!({"op":"undo"})).unwrap();
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":dir.join("undone.xlsx")}))
        .unwrap();
    assert_eq!(
        std::fs::read(dir.join("undone.xlsx")).unwrap(),
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../test/fixtures/row-reflow.xlsx"
        ))
        .unwrap()
    );
    let p = prepare(
        &mut c,
        json!({"op":"sheetGeometry","axis":"columns","action":"resize","sheetId":"office-1","at":2,"count":1,"size":10}),
    );
    assert!(p["layoutRows"]["office-1"]
        .as_array()
        .unwrap()
        .contains(&json!(1048576)));
    assert!(!p["layoutRows"]["office-1"]
        .as_array()
        .unwrap()
        .contains(&json!(8)));
}

#[test]
fn filtering_and_default_manual_height_preserve_automatic_policy() {
    let mut c = fixture();
    c.workspace["sheets"][0]["autoFilter"] =
        json!({"range":"A1:B7","filters":{},"filteredRows":[4]});
    let p = prepare(
        &mut c,
        json!({"op":"sheetGeometry","axis":"columns","action":"resize","sheetId":"office-1","at":2,"count":1,"size":10}),
    );
    assert!(!p["layoutRows"]["office-1"]
        .as_array()
        .unwrap()
        .contains(&json!(4)));
    assert!(!p["layoutRows"]["office-1"]
        .as_array()
        .unwrap()
        .contains(&json!(5)));
    c.dispatch(json!({"op":"cancelLayoutEdit","token":p["layoutToken"]}))
        .unwrap();
    let mut pkg = Package::default();
    pkg.set("sheet.xml", "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetFormatPr defaultRowHeight=\"28\" customHeight=\"1\"/><sheetData><row r=\"2\" ht=\"22\" customHeight=\"0\"/><row r=\"3\" customHeight=\"1\"/></sheetData></worksheet>".into());
    let mut sheet = json!({"id":"s","name":"Sheet","cells":{}});
    crate::sheet_geometry::read(&pkg, "sheet.xml", &mut sheet).unwrap();
    assert!(!crate::sheet_geometry::automatic_row(&sheet, "1"));
    assert!(crate::sheet_geometry::automatic_row(&sheet, "2"));
    assert!(!crate::sheet_geometry::automatic_row(&sheet, "3"));
    for modes in [
        json!({"0":"auto"}),
        json!({"02":"auto"}),
        json!({"1048577":"auto"}),
        json!({"2":true}),
        json!({"2":"unknown"}),
    ] {
        let mut invalid = sheet.clone();
        invalid["rowHeightModes"] = modes;
        assert!(crate::sheet_geometry::validate(&invalid).is_err());
    }
}
