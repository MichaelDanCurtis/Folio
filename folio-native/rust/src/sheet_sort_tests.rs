use crate::{office::Package, workspace::Core};
use serde_json::{json, Value};
fn fixture() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/range-sort.xlsx")})).unwrap();
    c
}
fn output() -> std::path::PathBuf {
    let p = std::env::var("FOLIO_SORT_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or(std::env::temp_dir().join("folio-sort-tests"));
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn sort(c: &mut Core, extra: Value) {
    let mut r = json!({"op":"sortRange","sheetId":"office-1","range":"A1:D7","headers":true,"keys":[{"column":0},{"column":1}]});
    r.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    c.dispatch(r).unwrap();
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = output().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}

#[test]
fn range_sort_moves_visible_records_preserves_objects_and_undo_bytes() {
    let mut c = fixture();
    let original = c.workspace.clone();
    sort(&mut c, json!({}));
    let s = &c.workspace["sheets"][0];
    assert_eq!(s["tables"], json!([]));
    assert_eq!(s["cells"]["A2"], "Apple");
    assert_eq!(s["cells"]["A4"], "Hidden");
    assert_eq!(s["cells"]["B3"], "1");
    assert_eq!(s["cells"]["C3"], "8");
    assert_eq!(s["cells"]["C5"], "3");
    assert_eq!(s["cells"]["B6"], "3");
    assert_eq!(s["cells"]["D6"], "=B6*C6");
    assert_eq!(s["cells"]["E6"], "fixed-6");
    assert_eq!(s["cellStyles"]["B6"]["bold"], true);
    assert_eq!(c.snapshot()["calculated"]["office-1"]["D6"], 15.0);
    assert_eq!(c.snapshot()["calculated"]["office-2"]["A1"], 239.0);
    let sorted = export(&mut c, "range-sorted.xlsx");
    let saved = output().join("range-sorted.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(loaded.workspace["sheets"], c.workspace["sheets"]);
    assert_eq!(
        export(&mut loaded, "range-reopened.xlsx").original,
        sorted.original
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], original["sheets"]);
    assert_eq!(
        export(&mut c, "range-undone.xlsx").original,
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../test/fixtures/range-sort.xlsx"
        ))
        .unwrap()
    );
}

#[test]
fn headerless_case_sensitive_sort_and_structural_sort_metadata() {
    let mut c = fixture();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"office-1","address":"G1","value":"b"},{"sheetId":"office-1","address":"G2","value":"A"},{"sheetId":"office-1","address":"G3","value":"a"},{"sheetId":"office-1","address":"G5","value":"B"}]})).unwrap();
    sort(
        &mut c,
        json!({"range":"G1:G5","headers":false,"keys":[{"column":0,"descending":true}],"caseSensitive":true}),
    );
    assert_eq!(c.workspace["sheets"][0]["cells"]["G1"], "b");
    assert_eq!(c.workspace["sheets"][0]["cells"]["G2"], "a");
    assert_eq!(c.workspace["sheets"][0]["cells"]["G3"], "B");
    assert_eq!(c.workspace["sheets"][0]["cells"]["G5"], "A");
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertColumns","at":2,"count":1}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["rangeSort"]["range"], "H1:H5");
    export(&mut c, "range-headerless.xlsx");
    let before = c.workspace.clone();
    for r in [
        json!({"range":"A1:A1","headers":true}),
        json!({"keys":[{"column":40}]}),
        json!({"baseRevision":0}),
        json!({"range":"A1:XFE5"}),
    ] {
        let mut request = json!({"op":"sortRange","sheetId":"office-1","range":"A1:D7","headers":true,"keys":[{"column":0}]});
        request
            .as_object_mut()
            .unwrap()
            .extend(r.as_object().unwrap().clone());
        assert!(c.dispatch(request).is_err());
        assert_eq!(c.workspace, before);
    }
}

#[test]
fn range_sort_rejects_partial_tables_merged_cells_and_outline_groups_atomically() {
    let mut c = fixture();
    c.dispatch(json!({"op":"table","sheetId":"office-1","action":"create","range":"A1:D7"}))
        .unwrap();
    let before = c.workspace.clone();
    assert!(c.dispatch(json!({"op":"sortRange","sheetId":"office-1","range":"B1:C5","headers":true,"keys":[{"column":0}]})).unwrap_err().contains("includes an Excel table"));
    assert_eq!(c.workspace, before);
    for (name, edit) in [("merged", "merge"), ("outlined", "outline")] {
        let mut pkg = Package::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../test/fixtures/range-sort.xlsx"
        ))
        .unwrap();
        let xml = pkg.text("xl/worksheets/sheet1.xml").unwrap().to_string();
        let xml = if edit == "merge" {
            xml.replace(
                "</sheetData>",
                "</sheetData><mergeCells count=\"1\"><mergeCell ref=\"G1:H1\"/></mergeCells>",
            )
        } else {
            xml.replace("<row r=\"2\"", "<row r=\"2\" outlineLevel=\"1\"")
        };
        pkg.set("xl/worksheets/sheet1.xml", xml);
        let input = output().join(format!("{name}-input.xlsx"));
        std::fs::write(&input, pkg.bytes().unwrap()).unwrap();
        let mut c = Core::new();
        c.dispatch(json!({"op":"importOffice","path":input}))
            .unwrap();
        let before = c.workspace.clone();
        for action in ["sort", "resize"] {
            assert!(c.dispatch(json!({"op":"sortRange","action":action,"sheetId":"office-1","range":"A1:H7","headers":true,"keys":[{"column":0}]})).is_err());
        }
        assert_eq!(c.workspace, before);
        assert_eq!(
            export(&mut c, &format!("{name}-unchanged.xlsx")).original,
            std::fs::read(input).unwrap()
        );
    }
}
