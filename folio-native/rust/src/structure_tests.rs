use crate::{
    office::{attr, parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};

fn fixture() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/structure.xlsx")})).unwrap();
    c.dispatch(json!({"op":"document","document":{"type":"doc","content":[{"type":"workbookBlock","attrs":{"sheetId":"office-1","range":"A1:C4","title":"Budget","mode":"table"}}]}})).unwrap();
    c
}
fn edit(c: &mut Core, action: &str, extra: Value) -> Result<Value, String> {
    let mut request = json!({"op":"structure","sheetId":"office-1","action":action});
    request
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    c.dispatch(request)
}
fn export(c: &mut Core, name: &str) -> Package {
    let dir = std::env::var("FOLIO_STRUCTURE_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("folio-structure-tests"));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn formulas(p: &Package, path: &str, tag: &str) -> Vec<String> {
    parse(p.text(path).unwrap())
        .unwrap()
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == tag)
        .filter_map(|n| n.text().map(String::from))
        .collect()
}

#[test]
fn imported_structure_updates_references_objects_and_history() {
    let mut c = fixture();
    assert_eq!(c.workspace["sheets"][0]["cells"]["D3"], "=C3*2");
    let original = c.workspace.clone();
    let revision = c.workspace["revision"].as_u64().unwrap();
    let out = edit(&mut c, "insertRows", json!({"at":2,"count":2})).unwrap();
    assert_eq!(out["workspace"]["revision"], revision + 1);
    assert_eq!(out["workspace"]["sheets"][0]["cells"]["B4"], "2400");
    assert_eq!(out["workspace"]["sheets"][0]["cells"]["B6"], "=SUM(B4:B5)");
    assert_eq!(out["workspace"]["sheets"][0]["cells"]["D5"], "=C5*2");
    assert_eq!(
        out["workspace"]["sheets"][1]["cells"]["B1"],
        "=SUM(Budget!B4:B5)"
    );
    assert_eq!(out["calculated"]["office-1"]["B6"], 10000.0);
    assert_eq!(out["calculated"]["office-2"]["B1"], 10000.0);
    assert_eq!(
        out["workspace"]["document"]["content"][0]["attrs"]["range"],
        "A1:C6"
    );
    assert_eq!(
        out["workspace"]["sheets"][0]["cellStyles"]["B4"],
        original["sheets"][0]["cellStyles"]["B2"]
    );
    let p = export(&mut c, "rows.xlsx");
    let sheet = parse(p.text("xl/worksheets/sheet1.xml").unwrap()).unwrap();
    let ref_attr = |tag: &str, key: &str| {
        sheet
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == tag)
            .and_then(|n| attr(n, key))
            .unwrap()
            .to_string()
    };
    assert_eq!(ref_attr("autoFilter", "ref"), "A1:C6");
    assert_eq!(ref_attr("pane", "topLeftCell"), "B4");
    assert_eq!(ref_attr("pane", "ySplit"), "3");
    assert_eq!(ref_attr("dataValidation", "sqref"), "B4:C5");
    assert_eq!(ref_attr("conditionalFormatting", "sqref"), "B4:C5");
    assert_eq!(
        formulas(&p, "xl/worksheets/sheet1.xml", "formula"),
        vec!["$B$4", "B4>0"]
    );
    let comments = p
        .entries
        .keys()
        .find(|p| p.starts_with("xl/comments/") && p.ends_with(".xml"))
        .unwrap();
    assert!(p.text(comments).unwrap().contains("ref=\"A4\""));
    assert!(
        formulas(&p, "xl/charts/chart1.xml", "f")
            .iter()
            .any(|f| f.contains("$B$4:$B$5")),
        "{:?}",
        formulas(&p, "xl/charts/chart1.xml", "f")
    );
    assert!(
        formulas(&p, "xl/workbook.xml", "definedName").contains(&"Budget!$B$4:$C$5".to_string())
    );
    assert!(formulas(&p, "xl/drawings/drawing1.xml", "row").contains(&"3".into()));
    edit(&mut c, "rename", json!({"name":"O'Brien budget"})).unwrap();
    assert_eq!(
        c.workspace["sheets"][1]["cells"]["B1"],
        "=SUM('O''Brien budget'!B4:B5)"
    );
    edit(&mut c, "move", json!({"index":1})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["name"], "Inputs");
    let p = export(&mut c, "renamed-moved.xlsx");
    assert_eq!(
        crate::xlsx::import(&p).unwrap().0[1]["name"],
        "O'Brien budget"
    );
    let wb = parse(p.text("xl/workbook.xml").unwrap()).unwrap();
    assert_eq!(
        wb.descendants()
            .find(|n| attr(*n, "name") == Some("LocalCosts"))
            .and_then(|n| attr(n, "localSheetId")),
        Some("1")
    );
    edit(&mut c, "deleteRows", json!({"at":4,"count":1})).unwrap();
    assert_eq!(c.workspace["sheets"][1]["cells"]["B5"], "=SUM(B4:B4)");
    assert_eq!(c.snapshot()["calculated"]["office-1"]["B5"], 7600.0);
    export(&mut c, "deleted-row.xlsx");
    for _ in 0..4 {
        c.dispatch(json!({"op":"undo"})).unwrap();
    }
    assert_eq!(c.workspace["sheets"], original["sheets"]);
    assert_eq!(c.workspace["document"], original["document"]);
    assert_eq!(
        export(&mut c, "undone.xlsx").original,
        include_bytes!("../../test/fixtures/structure.xlsx")
    );
    for _ in 0..4 {
        c.dispatch(json!({"op":"redo"})).unwrap();
    }
    let save = std::env::temp_dir().join("folio-structural-roundtrip.folio");
    c.dispatch(json!({"op":"save","path":save})).unwrap();
    let mut restored = Core::new();
    restored.dispatch(json!({"op":"load","path":save})).unwrap();
    assert_eq!(restored.workspace, c.workspace);
    export(&mut restored, "reopened.xlsx");
}

#[test]
fn column_edits_sheet_deletion_and_failure_atomicity() {
    let mut c = fixture();
    edit(&mut c, "insertColumns", json!({"at":2,"count":2})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["D2"], "2400");
    assert_eq!(
        c.workspace["sheets"][0]["cells"]["AC1"],
        "=Budget!D2*Inputs!A1"
    );
    assert_eq!(
        c.workspace["document"]["content"][0]["attrs"]["range"],
        "A1:E4"
    );
    export(&mut c, "columns.xlsx");
    edit(&mut c, "deleteColumns", json!({"at":4,"count":1})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["AB1"], "=#REF!*Inputs!A1");
    export(&mut c, "deleted-column.xlsx");
    let before = c.workspace.clone();
    for action in [
        json!({"action":"rename","name":"Inputs"}),
        json!({"action":"rename","name":"Bad:Name"}),
        json!({"action":"insertRows","at":0}),
        json!({"action":"deleteRows","at":2,"count":-1}),
        json!({"action":"move","index":8}),
        json!({"action":"deleteRows","at":2,"baseRevision":0}),
    ] {
        assert!(edit(&mut c, "", action).is_err());
        assert_eq!(c.workspace, before);
    }
    edit(&mut c, "delete", json!({})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["name"], "Inputs");
    assert_eq!(c.workspace["sheets"][0]["cells"]["B1"], "=SUM(#REF!)");
    assert!(c.dispatch(json!({"op":"exportHtml"})).unwrap()["text"]
        .as_str()
        .unwrap()
        .contains("Workbook source unavailable"));
    let p = export(&mut c, "deleted-sheet.xlsx");
    assert_eq!(
        crate::xlsx::import(&p).unwrap().0.as_array().unwrap().len(),
        1
    );
    assert!(!formulas(&p, "xl/workbook.xml", "definedName")
        .iter()
        .any(|f| f.contains("Budget!")));
    let before = c.workspace.clone();
    assert!(c
        .dispatch(json!({"op":"structure","sheetId":"office-2","action":"delete"}))
        .is_err());
    assert_eq!(c.workspace, before);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"].as_array().unwrap().len(), 2);
}

#[test]
fn boundaries_deleted_links_and_unknown_formulas_do_not_lose_data() {
    let mut c = Core::new();
    c.workspace["sheets"] = json!([{"id":"s","name":"Data","cells":{"A1048576":"last","XFD1":"right","B1":"=1"},"cellStyles":{"B1048576":{"bold":true}}}]);
    c.workspace["document"] = json!({"type":"doc","content":[{"type":"workbookBlock","attrs":{"sheetId":"s","range":"A1:B2"}}]});
    let before = c.workspace.clone();
    for action in ["insertRows", "insertColumns"] {
        assert!(c
            .dispatch(json!({"op":"structure","action":action,"sheetId":"s","at":1}))
            .is_err());
        assert_eq!(c.workspace, before);
    }
    c.dispatch(json!({"op":"structure","action":"deleteRows","sheetId":"s","at":1,"count":2}))
        .unwrap();
    assert_eq!(
        c.workspace["document"]["content"][0]["attrs"]["range"],
        "#REF!"
    );
    assert!(c.dispatch(json!({"op":"exportHtml"})).unwrap()["text"]
        .as_str()
        .unwrap()
        .contains("Workbook source unavailable"));
    c.workspace["sheets"][0]["cells"]["A1"] = json!("=SUM(");
    let before = c.workspace.clone();
    assert!(c
        .dispatch(json!({"op":"structure","action":"rename","sheetId":"s","name":"Renamed"}))
        .is_err());
    assert_eq!(c.workspace, before);
}

#[test]
fn shared_formula_edits_and_reused_sheet_positions_export_safely() {
    let mut c = fixture();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"office-1","address":"D2","value":"7"}]}))
        .unwrap();
    let p = export(&mut c, "shared-edit.xlsx");
    let imported = crate::xlsx::import(&p).unwrap().0;
    assert_eq!(imported[0]["cells"]["D2"], "7");
    assert_eq!(imported[0]["cells"]["D3"], "=C3*2");
    c.dispatch(json!({"op":"addSheet"})).unwrap();
    let id = c.workspace["sheets"][2]["id"].clone();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":id,"address":"A1","value":"survives"}]}))
        .unwrap();
    c.dispatch(json!({"op":"structure","sheetId":id,"action":"move","index":0}))
        .unwrap();
    edit(&mut c, "delete", json!({})).unwrap();
    c.dispatch(json!({"op":"addSheet"})).unwrap();
    let last = c.workspace["sheets"][2]["id"].clone();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":last,"address":"B2","value":"new data"}]}))
        .unwrap();
    let p = export(&mut c, "added-after-delete.xlsx");
    let imported = crate::xlsx::import(&p).unwrap().0;
    assert_eq!(imported[0]["cells"]["A1"], "survives");
    assert_eq!(imported[2]["cells"]["B2"], "new data");
    let wb = parse(p.text("xl/workbook.xml").unwrap()).unwrap();
    let ids: Vec<_> = wb
        .descendants()
        .filter_map(|n| attr(n, "sheetId"))
        .collect();
    assert_eq!(
        ids.iter().collect::<std::collections::HashSet<_>>().len(),
        3
    );
}

#[test]
fn partial_array_deletion_is_rejected_before_dropping_its_master() {
    let mut p =
        Package::from_bytes(include_bytes!("../../test/fixtures/structure.xlsx").to_vec()).unwrap();
    let xml = p.text("xl/worksheets/sheet1.xml").unwrap().to_string();
    let xml = xml
        .replace(
            "t=\"shared\" si=\"0\" ref=\"D2:D3\"",
            "t=\"array\" ref=\"D2:D3\"",
        )
        .replace("<f t=\"shared\" si=\"0\"/>", "");
    p.set("xl/worksheets/sheet1.xml", xml);
    let change = crate::structure::Change {
        sheet: 0,
        names: vec!["Budget".into(), "Inputs".into()],
        edit: crate::structure::Edit::Axis {
            rows: true,
            insert: false,
            at: 2,
            count: 1,
        },
    };
    let error = crate::xlsx_structure::transform(&mut p, &change).unwrap_err();
    assert!(error.contains("part of an array"), "{error}");
}
