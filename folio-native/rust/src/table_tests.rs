use crate::{
    office::{attr, parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};

fn fixture() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/tables.xlsx")})).unwrap();
    c
}
fn table(c: &mut Core, action: &str, extra: Value) -> Value {
    let id = c.workspace["sheets"][0]["tables"][0]["id"].clone();
    let mut r = json!({"op":"table","sheetId":"office-1","tableId":id,"action":action});
    r.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    c.dispatch(r).unwrap()
}
fn output() -> std::path::PathBuf {
    let dir = std::env::var("FOLIO_TABLE_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("folio-table-tests"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = output().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn cell(c: &mut Core, a: &str, v: &str) {
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"office-1","address":a,"value":v}]}))
        .unwrap();
}

#[test]
fn resize_preserves_records_extends_formulas_moves_totals_and_roundtrips() {
    let mut c = fixture();
    for (a, v) in [
        ("E1", "Qty"),
        ("A6", "Pear"),
        ("B6", "2"),
        ("C6", "9"),
        ("A7", "Apple"),
        ("B7", "1"),
        ("C7", "2"),
        ("D7", "777"),
    ] {
        cell(&mut c, a, v);
    }
    table(
        &mut c,
        "filter",
        json!({"column":0,"rule":{"kind":"values","values":["Pear"],"blank":false}}),
    );
    table(&mut c, "resize", json!({"range":"$A$1:$E$8"}));
    let s = &c.workspace["sheets"][0];
    assert_eq!(s["tables"][0]["range"], "A1:E8");
    assert_eq!(s["tables"][0]["columns"][4]["name"], "Qty2");
    assert_eq!(s["tables"][0]["columns"][4]["id"], 5);
    assert_eq!(s["cells"]["D6"], "=[@Qty]*[@Price]");
    assert_eq!(s["cells"]["D7"], "777");
    assert!(s["cells"]["A5"].is_null());
    assert_eq!(s["cells"]["D8"], "=SUBTOTAL(109,Sales[Amount])");
    assert_eq!(s["tables"][0]["filteredRows"], json!([3, 5, 7]));
    assert_eq!(c.snapshot()["calculated"]["office-1"]["D8"], 41.0);
    let expanded = c.workspace.clone();
    let package = export(&mut c, "resized-expanded.xlsx");
    let saved = output().join("resized.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut reopened = Core::new();
    reopened
        .dispatch(json!({"op":"load","path":saved}))
        .unwrap();
    assert_eq!(reopened.snapshot()["calculated"]["office-1"]["D8"], 41.0);
    export(&mut reopened, "resized-reopened.xlsx");
    let tid = c.workspace["sheets"][0]["tables"][0]["id"].clone();
    assert!(c.dispatch(json!({"op":"table","sheetId":"office-1","tableId":tid,"action":"resize","range":"A1:E6"})).unwrap_err().contains("contains data"));
    assert_eq!(c.workspace, expanded);
    table(&mut c, "totals", json!({"enabled":false}));
    table(&mut c, "resize", json!({"range":"A1:C3"}));
    let s = &c.workspace["sheets"][0];
    assert_eq!(s["cells"]["D2"], "=B2*C2");
    assert_eq!(s["cells"]["D6"], "=B6*C6");
    assert_eq!(s["cells"]["D7"], "777");
    assert_eq!(s["tables"][0]["filteredRows"], json!([3]));
    assert_eq!(s["hiddenRows"], json!([9]));
    assert_eq!(c.snapshot()["calculated"]["office-1"]["D6"], 18.0);
    assert_eq!(c.workspace["sheets"][1]["cells"]["A1"], "=SUM(#REF!)");
    export(&mut c, "resized-shrunk.xlsx");
    c.dispatch(json!({"op":"undo"})).unwrap();
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], expanded["sheets"]);
    assert_eq!(
        export(&mut c, "resized-undone.xlsx").original,
        package.original
    );
}

#[test]
fn resize_reindexes_filters_and_sort_keys_and_rejects_invalid_ranges_atomically() {
    let mut c = fixture();
    table(
        &mut c,
        "filter",
        json!({"column":0,"rule":{"kind":"values","values":["Pear"],"blank":false}}),
    );
    table(
        &mut c,
        "filter",
        json!({"column":1,"rule":{"kind":"custom","join":"and","conditions":[{"op":"greaterThan","value":"1"}]}}),
    );
    table(
        &mut c,
        "sort",
        json!({"keys":[{"column":2,"descending":true},{"column":0},{"column":1}]}),
    );
    table(&mut c, "resize", json!({"range":"B1:D5"}));
    let t = &c.workspace["sheets"][0]["tables"][0];
    assert_eq!(t["columns"][0]["id"], 2);
    assert_eq!(
        t["sort"],
        json!([{"column":1,"descending":true},{"column":0}])
    );
    assert!(t["filters"].get("0").is_some());
    assert_eq!(t["filters"].as_object().unwrap().len(), 1);
    assert_eq!(t["filteredRows"], json!([2]));
    assert_eq!(c.snapshot()["calculated"]["office-1"]["D5"], 23.0);
    export(&mut c, "resized-left.xlsx");
    let before = c.workspace.clone();
    let tid = before["sheets"][0]["tables"][0]["id"].clone();
    for range in ["B2:D6", "F1:G4", "B1:D2", "B1:XFE9", "B1:D100003"] {
        assert!(c.dispatch(json!({"op":"table","sheetId":"office-1","tableId":tid,"action":"resize","range":range})).is_err(),"{range}");
        assert_eq!(c.workspace, before);
    }
    assert!(c.dispatch(json!({"op":"table","sheetId":"office-1","tableId":tid,"action":"resize","range":"A1:D5","baseRevision":0})).is_err());
    assert_eq!(c.workspace, before);
    table(&mut c, "resize", json!({"range":"A1:E5"}));
    assert_eq!(
        c.workspace["sheets"][0]["tables"][0]["columns"][0]["name"],
        "Product"
    );
    assert_eq!(c.workspace["sheets"][0]["tables"][0]["columns"][0]["id"], 5);
    assert_eq!(c.workspace["sheets"][0]["tables"][0]["columns"][4]["id"], 6);
    assert_eq!(c.snapshot()["calculated"]["office-1"]["D5"], 23.0);
}

#[test]
fn resize_retains_unmodeled_sort_choices_without_stale_ranges() {
    let mut pkg = Package::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/tables.xlsx"
    ))
    .unwrap();
    let xml=pkg.text("xl/tables/table1.xml").unwrap().replace("<tableColumns",
        "<sortState ref=\"A2:D4\"><sortCondition ref=\"A2:A4\" customList=\"Pear,Apple\"/><sortCondition ref=\"B2:B4\" customList=\"3,2,1\" descending=\"1\"/></sortState><tableColumns");
    pkg.set("xl/tables/table1.xml", xml);
    let input = output().join("resize-custom-sort-input.xlsx");
    std::fs::write(&input, pkg.bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":input}))
        .unwrap();
    assert!(c.workspace["sheets"][0]["tables"][0].get("sort").is_none());
    table(&mut c, "resize", json!({"range":"A1:E8"}));
    let expanded = export(&mut c, "resize-custom-sort-expanded.xlsx");
    let d = parse(expanded.text("xl/tables/table1.xml").unwrap()).unwrap();
    assert!(d
        .descendants()
        .any(|n| attr(n, "ref") == Some("A2:A7") && attr(n, "customList") == Some("Pear,Apple")));
    table(&mut c, "resize", json!({"range":"B1:D8"}));
    let narrowed = export(&mut c, "resize-custom-sort-narrowed.xlsx");
    let d = parse(narrowed.text("xl/tables/table1.xml").unwrap()).unwrap();
    assert_eq!(
        d.descendants()
            .filter(|n| n.tag_name().name() == "sortCondition")
            .count(),
        1
    );
    assert!(d.descendants().any(|n| attr(n, "ref") == Some("B2:B7")
        && attr(n, "customList") == Some("3,2,1")
        && attr(n, "descending") == Some("1")));
}

#[test]
fn resize_totals_keep_a1_operands_and_overlapping_tables_are_atomic() {
    let mut c = fixture();
    cell(&mut c, "D5", "=SUM($B$2:B4)");
    table(&mut c, "resize", json!({"range":"A1:D8"}));
    assert_eq!(c.workspace["sheets"][0]["cells"]["D8"], "=SUM($B$2:B4)");
    assert_eq!(c.snapshot()["calculated"]["office-1"]["D8"], 6.0);
    export(&mut c, "resize-a1-total.xlsx");
    c.dispatch(json!({"op":"table","sheetId":"office-1","action":"create","range":"G1:H4"}))
        .unwrap();
    let before = c.workspace.clone();
    let tid = before["sheets"][0]["tables"][0]["id"].clone();
    assert!(c.dispatch(json!({"op":"table","sheetId":"office-1","tableId":tid,"action":"resize","range":"A1:H8"})).unwrap_err().contains("overlap"));
    assert_eq!(c.workspace, before);
}

#[test]
fn custom_totals_follow_edits_renames_structure_hide_show_and_recovery() {
    let mut c = fixture();
    cell(&mut c, "D5", "=SUM(Sales[Amount])+B2");
    let column = &c.workspace["sheets"][0]["tables"][0]["columns"][3];
    assert_eq!(column["totalsFunction"], "custom");
    assert_eq!(column["totalsFormula"], "=SUM(Sales[Amount])+B2");
    table(&mut c, "totals", json!({"enabled":false}));
    table(&mut c, "rename", json!({"name":"OrdersTable"}));
    cell(&mut c, "D1", "Net amount");
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertColumns","at":2,"count":1}),
    )
    .unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["tables"][0]["columns"][4]["totalsFormula"],
        "=SUM(OrdersTable[Net amount])+C2"
    );
    table(&mut c, "totals", json!({"enabled":true}));
    assert_eq!(
        c.workspace["sheets"][0]["cells"]["E5"],
        "=SUM(OrdersTable[Net amount])+C2"
    );
    assert_eq!(c.snapshot()["calculated"]["office-1"]["E5"], 34.0);
    export(&mut c, "custom-totals.xlsx");
    let mut imported = Core::new();
    imported
        .dispatch(json!({"op":"importOffice","path":output().join("custom-totals.xlsx")}))
        .unwrap();
    assert_eq!(
        imported.workspace["sheets"][0]["tables"][0]["columns"][4]["totalsFormula"],
        "=SUM(OrdersTable[Net amount])+C2"
    );
    table(&mut imported, "totals", json!({"enabled":false}));
    table(&mut imported, "totals", json!({"enabled":true}));
    assert_eq!(imported.snapshot()["calculated"]["office-1"]["E5"], 34.0);
    let saved = output().join("custom-totals.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(loaded.snapshot()["calculated"]["office-1"]["E5"], 34.0);
    table(&mut loaded, "totals", json!({"enabled":false}));
    table(&mut loaded, "totals", json!({"enabled":true}));
    export(&mut loaded, "custom-totals-reopened.xlsx");
    table(
        &mut loaded,
        "totalFunction",
        json!({"column":4,"function":"average"}),
    );
    assert!(loaded.workspace["sheets"][0]["tables"][0]["columns"][4]
        .get("totalsFormula")
        .is_none());
    export(&mut loaded, "custom-totals-standard.xlsx");
    loaded.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(loaded.snapshot()["calculated"]["office-1"]["E5"], 34.0);
}

#[test]
fn total_formula_menu_labels_and_custom_definition_roundtrip() {
    let mut c = fixture();
    table(
        &mut c,
        "totalFunction",
        json!({"column":3,"function":"custom","formula":"=MAX(Sales[Amount])-MIN(Sales[Amount])"}),
    );
    assert_eq!(c.snapshot()["calculated"]["office-1"]["D5"], 7.0);
    cell(&mut c, "D5", "Range of amounts");
    table(&mut c, "totals", json!({"enabled":false}));
    table(&mut c, "totals", json!({"enabled":true}));
    assert_eq!(c.workspace["sheets"][0]["cells"]["D5"], "Range of amounts");
    assert_eq!(
        c.workspace["sheets"][0]["tables"][0]["columns"][3]["totalsLabel"],
        "Range of amounts"
    );
    export(&mut c, "custom-totals-label.xlsx");
    let before = c.workspace.clone();
    let tid = before["sheets"][0]["tables"][0]["id"].clone();
    assert!(c.dispatch(json!({"op":"table","sheetId":"office-1","tableId":tid,"action":"totalFunction","column":3,"function":"custom","formula":"no equals"})).is_err());
    assert_eq!(c.workspace, before);
}

#[test]
fn imported_tables_filter_sort_rename_and_recover() {
    let mut c = fixture();
    let original = c.workspace.clone();
    assert_eq!(c.snapshot()["calculated"]["office-2"]["A1"], 31.0);
    assert_eq!(c.workspace["sheets"][0]["hiddenRows"], json!([9]));
    table(
        &mut c,
        "filter",
        json!({"column":0,"rule":{"kind":"values","values":["Pear"],"blank":false}}),
    );
    assert_eq!(c.snapshot()["calculated"]["office-2"]["A2"], 23.0);
    export(&mut c, "filtered.xlsx");
    table(
        &mut c,
        "sort",
        json!({"keys":[{"column":0,"descending":false},{"column":1,"descending":false}]}),
    );
    let s = &c.workspace["sheets"][0];
    assert_eq!(s["cells"]["A2"], "Apple");
    assert_eq!(s["cells"]["B4"], "3");
    assert_eq!(s["tables"][0]["filteredRows"], json!([2]));
    let p = export(&mut c, "sorted.xlsx");
    let doc = parse(p.text("xl/worksheets/sheet1.xml").unwrap()).unwrap();
    assert!(doc
        .descendants()
        .any(|n| n.tag_name().name() == "hyperlink" && attr(n, "ref") == Some("A4")));
    assert_eq!(
        doc.descendants()
            .find(|n| n.tag_name().name() == "c" && attr(*n, "r") == Some("A4"))
            .unwrap()
            .descendants()
            .filter(|n| n.tag_name().name() == "r")
            .count(),
        2
    );
    table(&mut c, "rename", json!({"name":"OrdersTable"}));
    cell(&mut c, "D1", "Net amount");
    assert_eq!(
        c.workspace["sheets"][1]["cells"]["A1"],
        "=SUM(OrdersTable[Net amount])"
    );
    assert_eq!(c.snapshot()["calculated"]["office-2"]["A2"], 23.0);
    let p = export(&mut c, "renamed.xlsx");
    assert!(p
        .text("xl/workbook.xml")
        .unwrap()
        .contains("OrdersTable[Net amount]"));
    let saved = output().join("tables.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(loaded.snapshot()["calculated"]["office-2"]["A2"], 23.0);
    export(&mut loaded, "reopened.xlsx");
    for _ in 0..4 {
        c.dispatch(json!({"op":"undo"})).unwrap();
    }
    assert_eq!(c.workspace["sheets"], original["sheets"]);
    assert_eq!(
        export(&mut c, "undone.xlsx").original,
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../test/fixtures/tables.xlsx"
        ))
        .unwrap()
    );
}

#[test]
fn table_structural_edits_update_columns_formulas_filters_and_xml() {
    let mut c = fixture();
    table(
        &mut c,
        "filter",
        json!({"column":0,"rule":{"kind":"custom","join":"and","conditions":[{"op":"contains","value":"ea"}]}}),
    );
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertRows","at":3,"count":1}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["tables"][0]["range"], "A1:D6");
    assert_eq!(c.workspace["sheets"][0]["cells"]["D3"], "=[@Qty]*[@Price]");
    cell(&mut c, "A3", "Peach");
    cell(&mut c, "B3", "2");
    cell(&mut c, "C3", "10");
    assert_eq!(c.snapshot()["calculated"]["office-2"]["A1"], 51.0);
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertColumns","at":2,"count":1}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["tables"][0]["range"], "A1:E6");
    assert_eq!(
        c.workspace["sheets"][0]["tables"][0]["columns"][1]["name"],
        "Column2"
    );
    let p = export(&mut c, "structure.xlsx");
    let (s, _) = crate::xlsx::import(&p).unwrap();
    assert_eq!(s[0]["tables"][0]["columns"].as_array().unwrap().len(), 5);
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"deleteColumns","at":3,"count":1}),
    )
    .unwrap();
    assert!(c.workspace["sheets"][0]["cells"]["D2"]
        .as_str()
        .unwrap()
        .contains("#REF!"));
    assert!(
        c.workspace["sheets"][0]["tables"][0]["columns"][3]["formula"]
            .as_str()
            .unwrap()
            .contains("#REF!")
    );
    export(&mut c, "deleted-column.xlsx");
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.snapshot()["calculated"]["office-2"]["A1"], 51.0);
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"deleteRows","at":1,"count":6}),
    )
    .unwrap();
    assert!(c.workspace["sheets"][0]["tables"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(c.workspace["sheets"][1]["cells"]["A1"], "=SUM(#REF!)");
    assert!(crate::xlsx::import(&export(&mut c, "deleted-table.xlsx"))
        .unwrap()
        .0[0]["tables"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn native_table_expansion_column_rename_and_added_imported_sheet_export() {
    let mut c = fixture();
    let out = c.dispatch(json!({"op":"addSheet"})).unwrap();
    let id = out["workspace"]["sheets"][2]["id"].clone();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":id,"address":"A1","value":"Qty"},{"sheetId":id,"address":"B1","value":"Amount"},{"sheetId":id,"address":"A2","value":"5"}]})).unwrap();
    c.dispatch(
        json!({"op":"table","sheetId":id,"action":"create","range":"A1:B2","name":"NewSales"}),
    )
    .unwrap();
    let tid = c.workspace["sheets"][2]["tables"][0]["id"].clone();
    c.dispatch(json!({"op":"table","sheetId":id,"tableId":tid,"action":"columnFormula","column":1,"formula":"=[@Qty]*2"})).unwrap();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":id,"address":"A3","value":"7"},{"sheetId":id,"address":"A1","value":"Units"}]})).unwrap();
    assert_eq!(c.workspace["sheets"][2]["tables"][0]["range"], "A1:B3");
    assert_eq!(c.snapshot()["calculated"][id.as_str().unwrap()]["B3"], 14.0);
    assert!(c.workspace["sheets"][2]["cells"]["B3"]
        .as_str()
        .unwrap()
        .contains("Units"));
    let p = export(&mut c, "added-table.xlsx");
    let imported = crate::xlsx::import(&p).unwrap().0;
    assert_eq!(imported.as_array().unwrap().len(), 3);
    assert_eq!(imported[2]["tables"][0]["name"], "NewSales");
    assert_eq!(imported[2]["tables"][0]["range"], "A1:B3");
}

#[test]
fn custom_wildcards_and_stale_table_requests() {
    let mut c = fixture();
    let saved = c.workspace.clone();
    let tid = saved["sheets"][0]["tables"][0]["id"].clone();
    assert!(c.dispatch(json!({"op":"table","action":"sort","sheetId":"office-1","tableId":tid,"baseRevision":999,"keys":[{"column":0}]})).is_err());
    assert_eq!(c.workspace, saved);
    for (pattern, value, want) in [
        ("*ea*", "Pear", true),
        ("P?ar", "Pear", true),
        ("~*", "*", true),
        ("~*", "Pear", false),
        ("*a?", "Pear", true),
        ("*ea*", "Apple", false),
        ("*", "", true),
    ] {
        let rule =
            json!({"kind":"custom","join":"or","conditions":[{"op":"equal","value":pattern}]});
        assert_eq!(
            crate::tables::matches_filter(&json!(value), &rule).unwrap(),
            want,
            "{pattern}: {value}"
        );
    }
    table(
        &mut c,
        "filter",
        json!({"column":0,"rule":{"kind":"custom","join":"and","conditions":[{"op":"contains","value":"ea"}]}}),
    );
    let p = export(&mut c, "custom-filter.xlsx");
    let s = crate::xlsx::import(&p).unwrap().0;
    assert!(
        crate::tables::matches_filter(&json!("Pear"), &s[0]["tables"][0]["filters"]["0"]).unwrap()
    );
    assert!(
        !crate::tables::matches_filter(&json!("Apple"), &s[0]["tables"][0]["filters"]["0"])
            .unwrap()
    );
    table(&mut c, "clearFilters", json!({}));
    assert_eq!(c.snapshot()["calculated"]["office-2"]["A2"], 31.0);
    assert_eq!(c.workspace["sheets"][0]["hiddenRows"], json!([9]));
    export(&mut c, "cleared.xlsx");
}

#[test]
fn totals_and_numeric_filters_roundtrip_and_do_not_overwrite_data() {
    let mut c = fixture();
    table(&mut c, "totals", json!({"enabled":false}));
    assert_eq!(c.workspace["sheets"][0]["tables"][0]["range"], "A1:D4");
    let mut existing = c.workspace.clone();
    existing["sheets"][0]["cells"]["D5"] = json!("Keep this");
    c.dispatch(json!({"op":"replace","workspace":existing}))
        .unwrap();
    let old = c.workspace.clone();
    let id = old["sheets"][0]["tables"][0]["id"].clone();
    assert!(c
        .dispatch(
            json!({"op":"table","action":"totals","sheetId":"office-1","tableId":id,"enabled":true})
        )
        .is_err());
    assert_eq!(c.workspace, old);
    cell(&mut c, "D5", "");
    table(
        &mut c,
        "totalFunction",
        json!({"column":3,"function":"average"}),
    );
    let value = c.snapshot()["calculated"]["office-1"]["D5"]
        .as_f64()
        .unwrap();
    assert!((value - 31.0 / 3.0).abs() < 1e-10);
    table(
        &mut c,
        "filter",
        json!({"column":1,"rule":{"kind":"values","values":["3"],"blank":false}}),
    );
    assert_eq!(c.snapshot()["calculated"]["office-1"]["D5"], 15.0);
    export(&mut c, "totals.xlsx");
    table(
        &mut c,
        "totalFunction",
        json!({"column":1,"function":"sum"}),
    );
    table(&mut c, "totals", json!({"enabled":false}));
    table(&mut c, "totals", json!({"enabled":true}));
    assert_eq!(c.snapshot()["calculated"]["office-1"]["D5"], 15.0);
    assert_eq!(c.snapshot()["calculated"]["office-1"]["B5"], 3.0);

    table(&mut c, "clearFilters", json!({}));
    table(
        &mut c,
        "totalFunction",
        json!({"column":3,"function":"count"}),
    );
    assert_eq!(c.snapshot()["calculated"]["office-1"]["D5"], 3.0);
}

#[test]
fn prefixed_table_parts_and_old_workspaces_keep_editable_metadata() {
    let mut p = Package::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/tables.xlsx"
    ))
    .unwrap();
    let name = "xl/tables/table1.xml";
    let mut xml = p.text(name).unwrap().to_string();
    for tag in [
        "tableColumns",
        "tableColumn",
        "tableStyleInfo",
        "table",
        "autoFilter",
        "calculatedColumnFormula",
    ] {
        xml = xml
            .replace(&format!("<{tag}"), &format!("<x:{tag}"))
            .replace(&format!("</{tag}"), &format!("</x:{tag}"));
    }
    xml = xml.replace("xmlns=", "xmlns:x=");
    p.set(name, xml);
    let path = output().join("prefixed-input.xlsx");
    std::fs::write(&path, p.bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    table(&mut c, "rename", json!({"name":"PrefixedSales"}));
    cell(&mut c, "B1", "Units");
    assert_eq!(c.snapshot()["calculated"]["office-2"]["A1"], 31.0);
    export(&mut c, "prefixed.xlsx");
    let path = output().join("legacy-tables.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut old: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    // Older files lacked the table projection but retained original OOXML.
    for s in old["sheets"].as_array_mut().unwrap() {
        s.as_object_mut().unwrap().remove("tables");
        s.as_object_mut().unwrap().remove("hiddenRows");
    }
    for s in old["officeWorkbook"]["baseline"].as_array_mut().unwrap() {
        s.as_object_mut().unwrap().remove("tables");
        s.as_object_mut().unwrap().remove("hiddenRows");
    }
    // Return header inputs to the saved package baseline for the old-version scenario.
    old["sheets"][0]["cells"]["B1"] = json!("Qty");
    for a in ["D2", "D3", "D4"] {
        old["sheets"][0]["cells"][a] = json!("=[@Qty]*[@Price]");
    }
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        loaded.workspace["sheets"][0]["tables"][0]["name"],
        "PrefixedSales"
    );
    assert_eq!(loaded.workspace["sheets"][0]["hiddenRows"], json!([9]));
    assert_eq!(loaded.snapshot()["calculated"]["office-2"]["A1"], 31.0);
}

#[test]
fn deleting_table_sheet_invalidates_retained_names_and_undo_restores_it() {
    let mut c = fixture();
    let before = c.workspace.clone();
    c.dispatch(json!({"op":"structure","sheetId":"office-1","action":"delete"}))
        .unwrap();
    let p = export(&mut c, "deleted-table-sheet.xlsx");
    assert!(p
        .text("xl/workbook.xml")
        .unwrap()
        .contains(">#REF!</definedName>"));
    assert_eq!(c.workspace["sheets"][0]["cells"]["A1"], "=SUM(#REF!)");
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["sheets"]);
}

#[test]
fn deleting_a_filtered_column_reapplies_remaining_criteria_and_retains_sort_priority() {
    let mut c = fixture();
    table(
        &mut c,
        "filter",
        json!({"column":0,"rule":{"kind":"values","values":["Pear"],"blank":false}}),
    );
    table(
        &mut c,
        "filter",
        json!({"column":1,"rule":{"kind":"custom","join":"and","conditions":[{"op":"greaterThan","value":"1"}]}}),
    );
    assert_eq!(
        c.workspace["sheets"][0]["tables"][0]["filteredRows"],
        json!([3, 4])
    );
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"deleteColumns","at":1,"count":1}),
    )
    .unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["tables"][0]["filteredRows"],
        json!([4])
    );
    assert_eq!(c.snapshot()["calculated"]["office-2"]["A2"], 23.0);
    table(
        &mut c,
        "sort",
        json!({"keys":[{"column":1,"descending":true},{"column":0,"descending":false}]}),
    );
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertColumns","at":1,"count":1}),
    )
    .unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["tables"][0]["sort"],
        json!([{"column":1,"descending":true},{"column":0,"descending":false}])
    );
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertColumns","at":3,"count":1}),
    )
    .unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["tables"][0]["sort"],
        json!([{"column":2,"descending":true},{"column":0,"descending":false}])
    );
    export(&mut c, "filtered-column-deletion.xlsx");
}
