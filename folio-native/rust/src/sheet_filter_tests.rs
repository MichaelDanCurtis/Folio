use crate::{
    office::{attr, parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
fn output() -> std::path::PathBuf {
    let p = std::env::var("FOLIO_FILTER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or(std::env::temp_dir().join("folio-filter-tests"));
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn fixture() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/range-sort.xlsx")})).unwrap();
    c
}
fn action(c: &mut Core, a: &str, extra: Value) -> Value {
    let mut r = json!({"op":"autoFilter","action":a,"sheetId":"office-1","baseRevision":c.workspace["revision"]});
    r.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    c.dispatch(r).unwrap()
}
fn cells(c: &mut Core, si: &str, items: &[(&str, &str)]) {
    c.dispatch(json!({"op":"cells","cells":items.iter().map(|(a,v)|json!({"sheetId":si,"address":a,"value":v})).collect::<Vec<_>>()})).unwrap();
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = output().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn pear(c: &mut Core) {
    action(
        c,
        "filter",
        json!({"column":0,"rule":{"kind":"values","values":["Pear","Hidden"],"blank":false}}),
    );
}
fn totals(c: &mut Core) {
    cells(
        c,
        "office-2",
        &[
            ("C1", "=SUBTOTAL(9,Records!D2:D7)"),
            ("D1", "=SUBTOTAL(109,Records!D2:D7)"),
            ("E1", "=SUBTOTAL(9,Records!F2:F7)"),
            ("F1", "=SUBTOTAL(109,Records!F2:F7)"),
        ],
    );
    cells(
        c,
        "office-1",
        &[
            ("F2", "10"),
            ("F3", "20"),
            ("F4", "30"),
            ("F5", "40"),
            ("F6", "50"),
            ("F7", "60"),
        ],
    );
}

#[test]
fn worksheet_filters_subtotals_cross_columns_recovery_and_roundtrip() {
    let mut c = fixture();
    totals(&mut c);
    let before = c.workspace.clone();
    let before_pkg = export(&mut c, "filter-before.xlsx");
    action(&mut c, "enable", json!({"range":"$A$1:$D$7"}));
    pear(&mut c);
    let s = &c.workspace["sheets"][0];
    assert_eq!(s["tables"], json!([]));
    assert_eq!(s["autoFilter"]["filteredRows"], json!([3, 6]));
    assert_eq!(s["hiddenRows"], json!([4]));
    let calc = c.snapshot()["calculated"]["office-2"].clone();
    for (a, v) in [
        ("A1", 239.0),
        ("C1", 224.0),
        ("D1", 26.0),
        ("E1", 140.0),
        ("F1", 110.0),
    ] {
        assert_eq!(calc[a], v, "{a}");
    }
    let filtered = export(&mut c, "filtered-range.xlsx");
    let imported = crate::xlsx::import(&filtered).unwrap().0;
    assert_eq!(imported[0]["hiddenRows"], json!([4]));
    assert_eq!(imported[0]["autoFilter"]["filteredRows"], json!([3, 6]));
    let mut calc = crate::calculation::WorkbookCalculator::default();
    calc.sync(&imported);
    assert_eq!(calc.error, None);
    assert_eq!(calc.values["office-2"]["C1"], 224.0);
    let saved = output().join("filtered-range.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(loaded.workspace["sheets"], c.workspace["sheets"]);
    assert_eq!(
        export(&mut loaded, "filtered-reopened.xlsx").original,
        filtered.original
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["sheets"]);
    assert_eq!(
        export(&mut c, "filter-undone.xlsx").original,
        before_pkg.original
    );
}

#[test]
fn refilter_clear_remove_conditions_blanks_and_choices() {
    let mut c = fixture();
    totals(&mut c);
    action(&mut c, "enable", json!({"range":"A1:D7"}));
    pear(&mut c);
    let options = c
        .dispatch(json!({"op":"filterChoices","sheetId":"office-1","column":1}))
        .unwrap();
    assert_eq!(options["values"].as_array().unwrap().len(), 3);
    assert!(!options["values"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["value"] == 7.0));
    let options = c
        .dispatch(json!({"op":"filterChoices","sheetId":"office-1","column":0}))
        .unwrap();
    assert!(options["values"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["value"] == "Apple"));
    cells(&mut c, "office-1", &[("A3", "Pear")]);
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([3, 6])
    );
    action(&mut c, "reapply", json!({}));
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([6])
    );
    action(
        &mut c,
        "filter",
        json!({"column":1,"rule":{"kind":"custom","join":"and","conditions":[{"op":"greaterThan","value":"1"},{"op":"lessThan","value":"90"}]}}),
    );
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([4, 5, 6, 7])
    );
    export(&mut c, "filtered-conditions.xlsx");
    action(&mut c, "clearFilters", json!({}));
    assert_eq!(c.workspace["sheets"][0]["autoFilter"]["filters"], json!({}));
    assert_eq!(c.workspace["sheets"][0]["hiddenRows"], json!([4]));
    assert_eq!(c.snapshot()["calculated"]["office-2"]["C1"], 239.0);
    assert_eq!(c.snapshot()["calculated"]["office-2"]["D1"], 41.0);
    export(&mut c, "filtered-cleared.xlsx");
    action(
        &mut c,
        "filter",
        json!({"column":0,"rule":{"kind":"values","values":[],"blank":true}}),
    );
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([2, 3, 4, 5, 7])
    );
    action(&mut c, "remove", json!({}));
    assert!(c.workspace["sheets"][0]["autoFilter"].is_null());
    export(&mut c, "filtered-removed.xlsx");
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([2, 3, 4, 5, 7])
    );
}

#[test]
fn filters_follow_structure_reject_invalid_changes_and_sort_visible_records() {
    let mut c = fixture();
    action(&mut c, "enable", json!({"range":"A1:D7"}));
    pear(&mut c);
    let before = c.workspace.clone();
    for r in [
        json!({"action":"enable","range":"G1:H5"}),
        json!({"action":"filter","column":4,"rule":null}),
        json!({"action":"reapply","baseRevision":0}),
        json!({"action":"filter","column":0,"rule":{"kind":"custom","conditions":[]}}),
    ] {
        let mut request = json!({"op":"autoFilter","sheetId":"office-1"});
        request
            .as_object_mut()
            .unwrap()
            .extend(r.as_object().unwrap().clone());
        assert!(c.dispatch(request).is_err());
        assert_eq!(c.workspace, before);
    }
    assert!(c
        .dispatch(json!({"op":"table","sheetId":"office-1","action":"create","range":"B1:E5"}))
        .is_err());
    assert_eq!(c.workspace, before);
    c.dispatch(json!({"op":"sortRange","sheetId":"office-1","range":"A1:D7","headers":true,"keys":[{"column":1}]})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["B2"], "1");
    assert_eq!(c.workspace["sheets"][0]["cells"]["B3"], "2");
    assert_eq!(c.workspace["sheets"][0]["cells"]["B4"], "99");
    export(&mut c, "filtered-sorted.xlsx");
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertRows","at":2,"count":1}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["autoFilter"]["range"], "A1:D8");
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([4, 7])
    );
    assert_eq!(c.workspace["sheets"][0]["hiddenRows"], json!([5]));
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertColumns","at":1,"count":1}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["autoFilter"]["range"], "B1:E8");
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"rename","name":"Order's data"}),
    )
    .unwrap();
    export(&mut c, "filtered-structure.xlsx");
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"deleteColumns","at":2,"count":1}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["autoFilter"]["filters"], json!({}));
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([])
    );
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"deleteRows","at":1,"count":1}),
    )
    .unwrap();
    assert!(c.workspace["sheets"][0]["autoFilter"].is_null());
    export(&mut c, "filtered-header-deleted.xlsx");
}

#[test]
fn table_manual_hidden_rows_and_external_visibility_edits() {
    let mut c = fixture();
    totals(&mut c);
    c.dispatch(json!({"op":"table","sheetId":"office-1","action":"create","range":"A1:D7","name":"RecordsTable"})).unwrap();
    let id = c.workspace["sheets"][0]["tables"][0]["id"].clone();
    c.dispatch(json!({"op":"table","sheetId":"office-1","tableId":id,"action":"filter","column":0,"rule":{"kind":"values","values":["Pear","Hidden"],"blank":false}})).unwrap();
    assert_eq!(c.snapshot()["calculated"]["office-2"]["C1"], 224.0);
    assert_eq!(c.snapshot()["calculated"]["office-2"]["F1"], 110.0);
    let mut pkg = export(&mut c, "filtered-table-manual.xlsx");
    let imported = crate::xlsx::import(&pkg).unwrap().0;
    assert_eq!(imported[0]["hiddenRows"], json!([4]));
    // Excel or another editor can change row visibility while retaining unknown
    // extensions. A stale extension must not override the standard row flags.
    let xml = pkg.text("xl/worksheets/sheet1.xml").unwrap().to_string();
    let d = parse(&xml).unwrap();
    let row = d
        .descendants()
        .find(|n| n.tag_name().name() == "row" && attr(*n, "r") == Some("2"))
        .unwrap();
    let edited = crate::xlsx_tables::attributes(&xml, row, &[("hidden", Some("1".into()))]);
    let xml = crate::office::replace_ranges(&xml, vec![(row.range(), edited)]);
    pkg.set("xl/worksheets/sheet1.xml", xml);
    let imported = crate::xlsx::import(&pkg).unwrap().0;
    assert!(crate::tables::hidden_rows(&imported[0]).contains(&2));
    assert_ne!(imported[0]["tables"][0]["filteredRows"], json!([3, 6]));
}

#[test]
fn native_filters_export_without_tables_and_imported_unsupported_filters_are_preserved() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    cells(
        &mut c,
        "sheet-1",
        &[
            ("A1", "Item"),
            ("A2", "One"),
            ("A3", "Two"),
            ("B1", "Value"),
            ("B2", "1"),
            ("B3", "2"),
        ],
    );
    c.dispatch(json!({"op":"autoFilter","sheetId":"sheet-1","action":"enable","range":"A1:B3"}))
        .unwrap();
    c.dispatch(json!({"op":"autoFilter","sheetId":"sheet-1","action":"filter","column":0,"rule":{"kind":"values","values":["One"],"blank":false}})).unwrap();
    let pkg = export(&mut c, "filter-native.xlsx");
    let imported = crate::xlsx::import(&pkg).unwrap().0;
    assert_eq!(imported[0]["autoFilter"]["range"], "A1:B3");
    assert_eq!(imported[0]["autoFilter"]["filteredRows"], json!([3]));
    assert_eq!(imported[0]["tables"], json!([]));
    let mut source = Package::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/range-sort.xlsx"
    ))
    .unwrap();
    let xml=source.text("xl/worksheets/sheet1.xml").unwrap().to_string().replace("</sheetData>","</sheetData><autoFilter ref=\"A1:D7\"><filterColumn colId=\"1\" hiddenButton=\"0\"><top10 top=\"1\" percent=\"0\" val=\"2\"/></filterColumn></autoFilter>");
    source.set("xl/worksheets/sheet1.xml", xml);
    let path = output().join("filter-unsupported-input.xlsx");
    std::fs::write(&path, source.bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filters"]["1"]["kind"],
        "unsupported"
    );
    let before = c.workspace.clone();
    assert!(c
        .dispatch(json!({"op":"autoFilter","sheetId":"office-1","action":"reapply"}))
        .is_err());
    assert_eq!(c.workspace, before);
    cells(&mut c, "office-1", &[("H20", "Unrelated edit")]);
    let pkg = export(&mut c, "filter-unsupported-retained.xlsx");
    assert!(pkg
        .text("xl/worksheets/sheet1.xml")
        .unwrap()
        .contains("<top10 top=\"1\" percent=\"0\" val=\"2\"/>"));
    action(&mut c, "clearFilters", json!({}));
    export(&mut c, "filter-unsupported-cleared.xlsx");
}

#[test]
fn prefixed_worksheets_preserve_cells_formula_metadata_and_filter_namespaces() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/filter-prefixed.xlsx")})).unwrap();
    totals(&mut c);
    pear(&mut c);
    cells(&mut c, "office-1", &[("G15", "New cell")]);
    let pkg = export(&mut c, "filter-prefixed.xlsx");
    let imported = crate::xlsx::import(&pkg).unwrap().0;
    assert_eq!(imported[0]["cells"]["A2"], "Pear");
    assert_eq!(imported[0]["cells"]["D2"], "=B2*C2");
    assert_eq!(imported[0]["cells"]["G15"], "New cell");
    assert_eq!(imported[0]["autoFilter"]["filteredRows"], json!([3, 6]));
    assert_eq!(imported[1]["cached"]["C1"], 224.0);
    for path in ["xl/worksheets/sheet1.xml", "xl/worksheets/sheet2.xml"] {
        let d = parse(pkg.text(path).unwrap()).unwrap();
        assert!(d
            .descendants()
            .filter(|n| n.is_element()
                && ["c", "f", "v", "row", "autoFilter", "filterColumn"]
                    .contains(&n.tag_name().name()))
            .all(|n| n.tag_name().namespace()
                == Some("http://schemas.openxmlformats.org/spreadsheetml/2006/main")));
    }
}
