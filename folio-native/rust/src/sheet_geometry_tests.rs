use crate::{
    office::{attr, parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
fn output() -> std::path::PathBuf {
    let p = std::env::var("FOLIO_GEOMETRY_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or(std::env::temp_dir().join("folio-geometry-tests"));
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn fixture() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/geometry.xlsx")})).unwrap();
    c
}
fn change(c: &mut Core, axis: &str, action: &str, at: usize, count: usize, size: f64) -> Value {
    c.dispatch(json!({"op":"sheetGeometry","sheetId":"office-1","axis":axis,"action":action,"at":at,"count":count,"size":size,"baseRevision":c.workspace["revision"]})).unwrap()
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = output().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}

#[test]
fn measured_row_heights_are_one_transaction_with_exact_recovery() {
    let mut c = fixture();
    let original = export(&mut c, "autofit-original.xlsx");
    let before = c.workspace.clone();
    let command = json!({"op":"sheetGeometry","sheetId":"office-1","axis":"rows","action":"autoFit","at":2,"count":4,"sizes":{"2":78.75,"3":24,"5":90},"baseRevision":c.workspace["revision"]});
    c.dispatch(command).unwrap();
    assert_eq!(
        c.workspace["revision"].as_u64().unwrap(),
        before["revision"].as_u64().unwrap() + 1
    );
    assert_eq!(
        c.workspace["sheets"][0]["hiddenRows"],
        before["sheets"][0]["hiddenRows"]
    );
    assert_eq!(
        c.workspace["sheets"][0]["rowHeights"]["4"],
        before["sheets"][0]["rowHeights"]["4"]
    );
    let edited = export(&mut c, "autofit-edited.xlsx");
    assert_eq!(
        edited.text("xl/styles.xml").unwrap(),
        original.text("xl/styles.xml").unwrap()
    );
    assert_eq!(
        crate::xlsx::import(&edited).unwrap().0[0]["rowHeights"]["2"],
        78.75
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        export(&mut c, "autofit-undone.xlsx").original,
        original.original
    );
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(
        export(&mut c, "autofit-redone.xlsx").original,
        edited.original
    );
    let saved = output().join("autofit.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(
        export(&mut loaded, "autofit-reopened.xlsx").original,
        edited.original
    );
    // Repeated identical measurement must not create an empty undo step.
    let previous = c.workspace.clone();
    c.dispatch(json!({"op":"sheetGeometry","sheetId":"office-1","axis":"rows","action":"autoFit","at":2,"count":4,"sizes":{"2":78.75,"3":24,"5":90},"baseRevision":c.workspace["revision"]})).unwrap();
    assert_eq!(c.workspace, previous);
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertRows","at":2,"count":1}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["rowHeights"]["3"], 78.75);
}

#[test]
fn measured_rows_reject_stale_partial_hidden_invalid_and_merged_atomically() {
    let mut c = fixture();
    for patch in [
        json!({"sizes":{"2":40}}),
        json!({"sizes":{"2":40,"3":24,"4":20,"5":30}}),
        json!({"sizes":{"02":40,"3":24,"5":30}}),
        json!({"sizes":{"2":410,"3":24,"5":30}}),
        json!({"sizes":{"2":0,"3":24,"5":30}}),
        json!({"sizes":{"2":"40","3":24,"5":30}}),
        json!({"baseRevision":-1}),
        json!({"baseRevision":null}),
        json!({"axis":"columns"}),
        json!({"count":100001}),
        json!({"at":1048576,"count":2}),
    ] {
        let before = c.workspace.clone();
        let mut cmd = json!({"op":"sheetGeometry","sheetId":"office-1","axis":"rows","action":"autoFit","at":2,"count":4,"sizes":{"2":40,"3":24,"5":30},"baseRevision":c.workspace["revision"]});
        cmd.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert!(c.dispatch(cmd).is_err());
        assert_eq!(c.workspace, before);
    }
    c.dispatch(json!({"op":"autoFilter","action":"enable","sheetId":"office-1","range":"A1:D7"}))
        .unwrap();
    c.dispatch(json!({"op":"autoFilter","action":"filter","sheetId":"office-1","column":0,"rule":{"kind":"values","values":["Pear","Hidden"],"blank":false}})).unwrap();
    let before = c.workspace.clone();
    assert!(c.dispatch(json!({"op":"sheetGeometry","sheetId":"office-1","axis":"rows","action":"autoFit","at":3,"count":1,"sizes":{"3":55},"baseRevision":c.workspace["revision"]})).is_err());
    assert_eq!(c.workspace, before);
    c.dispatch(json!({"op":"sheetGeometry","sheetId":"office-1","axis":"rows","action":"autoFit","at":3,"count":1,"sizes":{},"baseRevision":c.workspace["revision"]})).unwrap();
    assert_eq!(c.workspace, before);
    // Exercise the real source-package guard using an independent merged row.
    let mut pkg = export(&mut c, "autofit-before-merge.xlsx");
    let xml = pkg.text("xl/worksheets/sheet1.xml").unwrap().to_string();
    let doc = parse(&xml).unwrap();
    let merges = format!("<mergeCells xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" count=\"1\"><mergeCell ref=\"F20:G20\"/></mergeCells>");
    let updated = if crate::office::child(doc.root_element(), "mergeCells").is_some() {
        crate::xlsx_tables::replace_child(&xml, "mergeCells", &merges).unwrap()
    } else {
        let at = xml.rfind("</").unwrap();
        crate::office::replace_ranges(&xml, vec![(at..at, merges)])
    };
    pkg.set("xl/worksheets/sheet1.xml", updated);
    let path = output().join("autofit-merged-input.xlsx");
    std::fs::write(&path, pkg.bytes().unwrap()).unwrap();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let before = c.workspace.clone();
    let error = c.dispatch(json!({"op":"sheetGeometry","sheetId":"office-1","axis":"rows","action":"autoFit","at":20,"count":1,"sizes":{"20":40},"baseRevision":c.workspace["revision"]})).unwrap_err();
    assert!(error.contains("merged cells"), "{error}");
    assert_eq!(c.workspace, before);
    for column in [6, 7] {
        let error = c.dispatch(json!({"op":"sheetGeometry","sheetId":"office-1","axis":"columns","action":"autoFit","at":column,"count":1,"sizes":{column.to_string():20},"baseRevision":c.workspace["revision"]})).unwrap_err();
        assert!(error.contains("merged cells"));
        assert_eq!(c.workspace, before);
    }
    // Last Excel row and a new workbook need no source package.
    let mut c = Core::new();
    let id = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"sheetGeometry","sheetId":id,"axis":"rows","action":"autoFit","at":1048576,"count":1,"sizes":{"1048576":409},"baseRevision":c.workspace["revision"]})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["rowHeights"]["1048576"], 409);
}
#[test]
fn dimensions_import_edit_split_source_columns_and_roundtrip() {
    let mut c = fixture();
    assert_eq!(c.workspace["sheets"][0]["hiddenColumns"], json!([2, 3, 4]));
    assert_eq!(c.workspace["sheets"][0]["columnWidths"]["3"], 18.0);
    assert_eq!(c.workspace["sheets"][0]["rowHeights"]["25"], 44.0);
    let original = export(&mut c, "geometry-original.xlsx");
    change(&mut c, "columns", "resize", 3, 1, 28.5);
    assert_eq!(c.workspace["sheets"][0]["hiddenColumns"], json!([2, 4]));
    change(&mut c, "rows", "resize", 2, 2, 36.0);
    change(&mut c, "rows", "hide", 5, 1, 0.0);
    let edited = export(&mut c, "geometry-edited.xlsx");
    let imported = crate::xlsx::import(&edited).unwrap().0;
    assert_eq!(imported[0]["columnWidths"]["2"], 18.0);
    assert_eq!(imported[0]["columnWidths"]["3"], 28.5);
    assert_eq!(imported[0]["columnWidths"]["4"], 18.0);
    assert_eq!(imported[0]["hiddenColumns"], json!([2, 4]));
    assert_eq!(imported[0]["hiddenRows"], json!([4, 5]));
    assert_eq!(imported[0]["rowHeights"]["2"], 36.0);
    let saved = output().join("geometry.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(loaded.workspace["sheets"], c.workspace["sheets"]);
    assert_eq!(
        export(&mut loaded, "geometry-reopened.xlsx").entries,
        edited.entries
    );
    for _ in 0..3 {
        c.dispatch(json!({"op":"undo"})).unwrap();
    }
    assert_eq!(
        export(&mut c, "geometry-undone.xlsx").original,
        original.original
    );
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["columnWidths"]["3"], 28.5);
}

#[test]
fn dimensions_visibility_subtotals_zero_and_atomic_validation() {
    let mut c = fixture();
    c.dispatch(json!({"op":"cells","cells":[
        {"sheetId":"office-2","address":"C1","value":"=SUBTOTAL(9,Records!D2:D7)"},
        {"sheetId":"office-2","address":"D1","value":"=SUBTOTAL(109,Records!D2:D7)"}]}))
        .unwrap();
    c.dispatch(json!({"op":"autoFilter","action":"enable","sheetId":"office-1","range":"A1:D7"}))
        .unwrap();
    c.dispatch(json!({"op":"autoFilter","action":"filter","sheetId":"office-1","column":0,"rule":{"kind":"values","values":["Pear","Hidden"],"blank":false}})).unwrap();
    let v = change(&mut c, "rows", "hide", 2, 1, 0.0);
    assert_eq!(v["calculated"]["office-2"]["C1"], 224.0);
    assert_eq!(v["calculated"]["office-2"]["D1"], 11.0);
    let v = change(&mut c, "rows", "unhideAll", 0, 0, 0.0);
    assert_eq!(v["calculated"]["office-2"]["D1"], 224.0);
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([3, 6])
    );
    change(&mut c, "rows", "resize", 2, 1, 0.0);
    assert_eq!(c.workspace["sheets"][0]["rowHeights"]["2"], 30.0);
    change(&mut c, "rows", "unhide", 2, 1, 0.0);
    assert_eq!(c.workspace["sheets"][0]["hiddenRows"], json!([]));
    change(&mut c, "columns", "unhideAll", 0, 0, 0.0);
    let before = c.workspace.clone();
    for request in [
        json!({"axis":"rows","action":"resize","at":1,"count":1,"size":410}),
        json!({"axis":"columns","action":"resize","at":1,"count":1,"size":-1}),
        json!({"axis":"columns","action":"hide","at":1,"count":16384}),
        json!({"axis":"rows","action":"hide","at":1048576,"count":2}),
        json!({"axis":"rows","action":"hide","at":0,"count":1}),
        json!({"axis":"columns","action":"hide","at":2,"count":1,"baseRevision":0}),
    ] {
        let mut r = json!({"op":"sheetGeometry","sheetId":"office-1"});
        r.as_object_mut()
            .unwrap()
            .extend(request.as_object().unwrap().clone());
        assert!(c.dispatch(r).is_err());
        assert_eq!(c.workspace, before);
    }
    export(&mut c, "geometry-filtered.xlsx");
}

#[test]
fn dimensions_move_with_structural_edits_and_work_without_source() {
    let mut c = fixture();
    c.dispatch(
        json!({"op":"structure","action":"insertColumns","sheetId":"office-1","at":2,"count":2}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["columnWidths"]["4"], 18.0);
    assert_eq!(c.workspace["sheets"][0]["hiddenColumns"], json!([4, 5, 6]));
    c.dispatch(
        json!({"op":"structure","action":"insertRows","sheetId":"office-1","at":2,"count":2}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["rowHeights"]["4"], 30.0);
    assert_eq!(c.workspace["sheets"][0]["rowHeights"]["27"], 44.0);
    export(&mut c, "geometry-structure.xlsx");
    c.dispatch(
        json!({"op":"structure","action":"deleteColumns","sheetId":"office-1","at":5,"count":1}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["hiddenColumns"], json!([4, 5]));
    let mut c = Core::new();
    let id = c.workspace["sheets"][0]["id"].clone();
    for (axis, at, size) in [("rows", 30, 50.0), ("columns", 8, 22.0)] {
        c.dispatch(json!({"op":"sheetGeometry","sheetId":id,"axis":axis,"action":"resize","at":at,"count":1,"size":size})).unwrap();
    }
    let native = export(&mut c, "geometry-native.xlsx");
    let sheet = crate::xlsx::import(&native).unwrap().0[0].clone();
    assert_eq!(sheet["rowHeights"]["30"], 50.0);
    assert_eq!(sheet["columnWidths"]["8"], 22.0);
    let xml = native.text(sheet["officePath"].as_str().unwrap()).unwrap();
    let doc = parse(xml).unwrap();
    let tags: Vec<_> = doc
        .root_element()
        .children()
        .filter(|n| n.is_element())
        .map(|n| n.tag_name().name())
        .collect();
    assert!(
        tags.iter().position(|s| *s == "cols").unwrap()
            < tags.iter().position(|s| *s == "sheetData").unwrap()
    );
    assert_eq!(
        attr(
            doc.descendants()
                .find(|n| n.tag_name().name() == "col")
                .unwrap(),
            "width"
        ),
        Some("22")
    );
}

#[test]
fn dimension_edits_preserve_prefixed_xml_and_unhide_explicit_zero_sizes() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/filter-prefixed.xlsx")})).unwrap();
    change(&mut c, "rows", "resize", 30, 1, 25.0);
    change(&mut c, "columns", "resize", 3, 1, 21.0);
    let prefixed = export(&mut c, "geometry-prefixed.xlsx");
    let xml = prefixed.text("xl/worksheets/sheet1.xml").unwrap();
    for n in parse(xml)
        .unwrap()
        .descendants()
        .filter(|n| n.is_element() && matches!(n.tag_name().name(), "row" | "c" | "cols" | "col"))
    {
        assert_eq!(
            n.tag_name().namespace(),
            Some("http://schemas.openxmlformats.org/spreadsheetml/2006/main")
        );
    }
    let mut source = Package::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/geometry.xlsx"
    ))
    .unwrap();
    let xml = source.text("xl/worksheets/sheet1.xml").unwrap().to_string();
    let doc = parse(&xml).unwrap();
    let row = doc
        .descendants()
        .find(|n| n.tag_name().name() == "row" && attr(*n, "r") == Some("2"))
        .unwrap();
    let edited = crate::office::replace_ranges(
        &xml,
        vec![(
            row.range(),
            crate::xlsx_tables::attributes(
                &xml,
                row,
                &[("ht", Some("0".into())), ("hidden", None)],
            ),
        )],
    );
    source.set("xl/worksheets/sheet1.xml", edited);
    let path = output().join("geometry-zero-input.xlsx");
    std::fs::write(&path, source.bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    assert_eq!(c.workspace["sheets"][0]["rowHeights"]["2"], 0.0);
    assert!(c.workspace["sheets"][0]["hiddenRows"]
        .as_array()
        .unwrap()
        .contains(&json!(2)));
    change(&mut c, "rows", "unhide", 2, 1, 0.0);
    assert!(c.workspace["sheets"][0]["rowHeights"].get("2").is_none());
    let unhidden = export(&mut c, "geometry-zero-unhidden.xlsx");
    let projected = crate::xlsx::import(&unhidden).unwrap().0;
    assert_eq!(projected[0]["hiddenRows"], json!([4]));
    assert!(projected[0]["rowHeights"].get("2").is_none());
}

#[test]
fn merged_display_barriers_follow_structure_history_and_legacy_recovery() {
    let mut pkg = Package::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/geometry.xlsx"
    ))
    .unwrap();
    let xml = pkg.text("xl/worksheets/sheet1.xml").unwrap().replace(
        "</sheetData>",
        "</sheetData><mergeCells count=\"1\"><mergeCell ref=\"G20:H21\"/></mergeCells>",
    );
    pkg.set("xl/worksheets/sheet1.xml", xml);
    let path = output().join("overflow-merged-input.xlsx");
    std::fs::write(&path, pkg.bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    assert_eq!(c.workspace["sheets"][0]["mergedRanges"], json!(["G20:H21"]));
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertColumns","at":7,"count":2}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["mergedRanges"], json!(["I20:J21"]));
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["mergedRanges"], json!(["G20:H21"]));
    c.dispatch(json!({"op":"redo"})).unwrap();
    let saved = output().join("overflow-merged-legacy.folio");
    c.workspace["sheets"][0]
        .as_object_mut()
        .unwrap()
        .remove("mergedRanges");
    c.workspace["officeWorkbook"]["baseline"][0]
        .as_object_mut()
        .unwrap()
        .remove("mergedRanges");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut reopened = Core::new();
    reopened
        .dispatch(json!({"op":"load","path":saved}))
        .unwrap();
    assert_eq!(
        reopened.workspace["sheets"][0]["mergedRanges"],
        json!(["I20:J21"])
    );
    let exported = export(&mut reopened, "overflow-merged-roundtrip.xlsx");
    assert_eq!(
        crate::xlsx::import(&exported).unwrap().0[0]["mergedRanges"],
        json!(["I20:J21"])
    );
}
