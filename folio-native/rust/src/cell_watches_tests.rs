use crate::{cell_watches as watches, office::Package, workspace::Core};
use serde_json::{json, Value};
fn new() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c
}
fn id(c: &Core) -> String {
    c.workspace["sheets"][0]["id"].as_str().unwrap().into()
}
fn command(c: &mut Core, mut r: Value) -> Result<Value, String> {
    r["op"] = json!("cellWatches");
    r["workspaceId"] = c.workspace["id"].clone();
    r["baseRevision"] = c.workspace["revision"].clone();
    c.dispatch(r)
}
fn add(c: &mut Core, range: &str) {
    let sid = id(c);
    command(c, json!({"action":"add","sheetId":sid,"range":range})).unwrap();
}
fn export(c: &mut Core, name: &str) -> Package {
    let dir = std::env::temp_dir().join("folio-watch-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn imported(pkg: &Package) -> Core {
    let mut c = new();
    let path = std::env::temp_dir().join(format!(
        "folio-watch-{}.xlsx",
        crate::office::fingerprint(&pkg.bytes().unwrap())
    ));
    std::fs::write(&path, pkg.bytes().unwrap()).unwrap();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    c
}

#[test]
fn watches_are_unique_undoable_and_do_not_touch_cell_content_or_calculation() {
    let mut c = new();
    let sid = id(&c);
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":sid,"address":"A1","value":"=2+3"}]}))
        .unwrap();
    let before = c.snapshot();
    add(&mut c, "A1:B2");
    let watched = c.snapshot();
    assert_eq!(watches::entries(&c.workspace["sheets"][0]).count(), 4);
    assert_eq!(watched["calculated"], before["calculated"]);
    assert_eq!(watched["evaluatedCells"], before["evaluatedCells"]);
    assert_eq!(
        c.workspace["sheets"][0]["cells"],
        before["workspace"]["sheets"][0]["cells"]
    );
    add(&mut c, "$a$1:$b$2");
    assert_eq!(c.snapshot(), watched);
    command(&mut c,json!({"action":"remove","cells":[{"sheetId":sid,"address":"B2"},{"sheetId":sid,"address":"A1"}]})).unwrap();
    assert_eq!(watches::entries(&c.workspace["sheets"][0]).count(), 2);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], watched["workspace"]["sheets"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(watches::entries(&c.workspace["sheets"][0]).count(), 2);
}

#[test]
fn invalid_or_stale_watch_changes_are_atomic() {
    let mut c = new();
    let sid = id(&c);
    add(&mut c, "A1");
    let before = c.snapshot();
    for r in [
        json!({"action":"add","sheetId":sid,"range":"XFE1"}),
        json!({"action":"add","sheetId":sid,"range":"B2:A1"}),
        json!({"action":"add","sheetId":sid,"range":"A1:A1048576"}),
        json!({"action":"remove","cells":[{"sheetId":sid,"address":"A1"},{"sheetId":"missing","address":"A1"}]}),
    ] {
        assert!(command(&mut c, r).is_err());
        assert_eq!(c.snapshot(), before);
    }
    assert!(c.dispatch(json!({"op":"cellWatches","workspaceId":"other","baseRevision":c.workspace["revision"],"action":"add","sheetId":sid,"range":"B1"})).is_err());
    assert!(c.dispatch(json!({"op":"cellWatches","workspaceId":c.workspace["id"],"baseRevision":99999,"action":"add","sheetId":sid,"range":"B1"})).is_err());
    for v in [
        json!([]),
        json!({"A1":false}),
        json!({"a1":true}),
        json!({"XFE1":true}),
    ] {
        assert!(watches::validate(&json!({"cellWatches":v})).is_err());
    }
}

#[test]
fn watches_export_reimport_and_follow_axis_edits_in_new_and_imported_workbooks() {
    let mut original = new();
    add(&mut original, "B2:B3");
    let pkg = export(&mut original, "watch-source.xlsx");
    for (i, mut c) in [original, imported(&pkg)].into_iter().enumerate() {
        let sid = id(&c);
        c.dispatch(json!({"op":"structure","sheetId":sid,"action":"insertRows","at":2,"count":1}))
            .unwrap();
        assert_eq!(
            c.workspace["sheets"][0][watches::KEY],
            json!({"B3":true,"B4":true})
        );
        c.dispatch(
            json!({"op":"structure","sheetId":sid,"action":"insertColumns","at":2,"count":1}),
        )
        .unwrap();
        assert_eq!(
            c.workspace["sheets"][0][watches::KEY],
            json!({"C3":true,"C4":true})
        );
        c.dispatch(json!({"op":"structure","sheetId":sid,"action":"deleteRows","at":3,"count":1}))
            .unwrap();
        assert_eq!(c.workspace["sheets"][0][watches::KEY], json!({"C3":true}));
        let out = export(&mut c, &format!("watch-shifted-{i}.xlsx"));
        let re = crate::xlsx::import(&out).unwrap().0;
        assert_eq!(re[0][watches::KEY], json!({"C3":true}));
        c.dispatch(json!({"op":"undo"})).unwrap();
        assert_eq!(
            c.workspace["sheets"][0][watches::KEY],
            json!({"C3":true,"C4":true})
        );
    }
}

#[test]
fn watch_xml_preserves_unrelated_metadata_and_handles_empty_worksheets() {
    let mut pkg = Package::default();
    let path = "xl/worksheets/test.xml";
    pkg.set(path,"<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData/><cellWatches><!--kept--><cellWatch r=\"A1\"/><cellWatch r=\"XFE1\"/><x:future xmlns:x=\"urn:future\" data=\"keep\"/></cellWatches><ignoredErrors><ignoredError sqref=\"A1\" numberStoredAsText=\"1\"/></ignoredErrors></worksheet>".into());
    let mut sheet = json!({});
    watches::read(&pkg, path, &mut sheet).unwrap();
    let before = sheet.clone();
    assert_eq!(sheet[watches::KEY], json!({"A1":true}));
    assert_eq!(sheet["unsupportedCellWatches"], true);
    sheet[watches::KEY] = json!({"B2":true});
    watches::write(&mut pkg, &sheet, Some(&before), path).unwrap();
    let xml = pkg.text(path).unwrap();
    assert!(!xml.contains("r=\"A1\""));
    assert!(xml.contains("<!--kept-->"));
    assert!(xml.contains("r=\"XFE1\""));
    assert!(xml.contains("data=\"keep\""));
    assert!(xml.contains("numberStoredAsText=\"1\""));
    assert!(xml.find("cellWatches").unwrap() < xml.find("ignoredErrors").unwrap());
    pkg.set(
        path,
        "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"/>".into(),
    );
    watches::write(&mut pkg, &sheet, None, path).unwrap();
    assert!(pkg.text(path).unwrap().contains("r=\"B2\""));
}

#[test]
fn old_recovery_hydrates_watches_but_never_resurrects_intentionally_removed_ones() {
    let mut source = new();
    add(&mut source, "B2");
    let pkg = export(&mut source, "watch-hydrate-source.xlsx");
    let mut c = imported(&pkg);
    let unchanged = export(&mut c, "watch-unchanged.xlsx");
    assert_eq!(unchanged.original, pkg.original);
    let path = std::env::temp_dir().join("folio-watch-old-recovery.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for s in saved["sheets"].as_array_mut().unwrap() {
        s.as_object_mut().unwrap().remove(watches::KEY);
    }
    for s in saved["officeWorkbook"]["baseline"].as_array_mut().unwrap() {
        s.as_object_mut().unwrap().remove(watches::KEY);
    }
    std::fs::write(&path, saved.to_string()).unwrap();
    c.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(c.workspace["sheets"][0][watches::KEY], json!({"B2":true}));
    let sid = id(&c);
    command(
        &mut c,
        json!({"action":"remove","cells":[{"sheetId":sid,"address":"B2"}]}),
    )
    .unwrap();
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    c.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(c.workspace["sheets"][0][watches::KEY], json!({}));
    let removed = export(&mut c, "watch-removed.xlsx");
    let projected = crate::xlsx::import(&removed).unwrap().0;
    assert_eq!(watches::entries(&projected[0]).count(), 0);
    assert!(!removed
        .text(projected[0]["officePath"].as_str().unwrap())
        .unwrap()
        .contains("cellWatch"));
}

#[test]
fn blank_edge_watches_prevent_out_of_bounds_insertions_without_changing_history() {
    let mut c = new();
    add(&mut c, "XFD1048576");
    let before = c.snapshot();
    let sid = id(&c);
    for action in ["insertRows", "insertColumns"] {
        assert!(c
            .dispatch(json!({"op":"structure","sheetId":sid,"action":action,"at":1,"count":1}))
            .is_err());
        assert_eq!(c.snapshot(), before);
    }
}
