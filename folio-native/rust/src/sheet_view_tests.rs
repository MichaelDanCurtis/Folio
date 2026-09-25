use crate::{
    office::{attr, parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
fn output() -> std::path::PathBuf {
    let p = std::env::var("FOLIO_VIEW_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or(std::env::temp_dir().join("folio-view-tests"));
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn fixture() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/views.xlsx")})).unwrap();
    c
}
fn change(c: &mut Core, action: &str, rows: usize, cols: usize) {
    c.dispatch(json!({"op":"sheetView","sheetId":"office-1","action":action,"rows":rows,"columns":cols,"baseRevision":c.workspace["revision"]})).unwrap();
}
fn export(c: &mut Core, name: &str) -> Package {
    let p = output().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":p}))
        .unwrap();
    Package::read(p.to_str().unwrap()).unwrap()
}
#[test]
fn frozen_panes_import_scroll_origins_edit_save_undo_and_export() {
    let mut c = fixture();
    assert_eq!(
        c.workspace["sheets"][0]["frozenPane"],
        json!({"rows":3,"columns":2,"state":"frozenSplit","topLeftCell":"G20"})
    );
    assert!(c.workspace["sheets"][1]["frozenPane"].is_null());
    let original = export(&mut c, "view-original.xlsx");
    change(&mut c, "freeze", 2, 4);
    let edited = export(&mut c, "view-edited.xlsx");
    let (s, _) = crate::xlsx::import(&edited).unwrap();
    assert_eq!(
        s[0]["frozenPane"],
        json!({"rows":2,"columns":4,"state":"frozen","topLeftCell":"E3"})
    );
    let save = output().join("views.folio");
    c.dispatch(json!({"op":"save","path":save})).unwrap();
    let mut reopened = Core::new();
    reopened.dispatch(json!({"op":"load","path":save})).unwrap();
    assert_eq!(reopened.workspace["sheets"], c.workspace["sheets"]);
    assert_eq!(
        export(&mut reopened, "view-reopened.xlsx").entries,
        edited.entries
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        export(&mut c, "view-undone.xlsx").original,
        original.original
    );
    c.dispatch(json!({"op":"redo"})).unwrap();
    change(&mut c, "unfreeze", 0, 0);
    export(&mut c, "view-unfrozen.xlsx");
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["frozenPane"]["columns"], 4);
}

#[test]
fn frozen_boundary_and_scrolled_origin_transform_independently() {
    let mut c = fixture();
    // Insert below the frozen prefix but before the scroll origin.
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertRows","at":6,"count":2}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["frozenPane"]["rows"], 3);
    assert_eq!(c.workspace["sheets"][0]["frozenPane"]["topLeftCell"], "G22");
    // Insert inside each frozen prefix; those prefixes grow.
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertRows","at":2,"count":2}),
    )
    .unwrap();
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertColumns","at":1,"count":1}),
    )
    .unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["frozenPane"],
        json!({"rows":5,"columns":3,"state":"frozenSplit","topLeftCell":"H24"})
    );
    export(&mut c, "view-structure.xlsx");
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"deleteRows","at":1,"count":5}),
    )
    .unwrap();
    let columns_only = export(&mut c, "view-columns-only.xlsx");
    let d = parse(columns_only.text("xl/worksheets/sheet1.xml").unwrap()).unwrap();
    let pane = d
        .descendants()
        .find(|n| n.tag_name().name() == "pane")
        .unwrap();
    assert_eq!(attr(pane, "activePane"), Some("topRight"));
    assert!(d
        .descendants()
        .filter(|n| n.tag_name().name() == "selection")
        .all(|n| !attr(n, "pane").unwrap_or("").starts_with("bottom")));
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"deleteColumns","at":1,"count":3}),
    )
    .unwrap();
    assert!(c.workspace["sheets"][0]["frozenPane"].is_null());
    export(&mut c, "view-deleted-prefix.xlsx");
}

#[test]
fn frozen_panes_reject_stale_invalid_requests_atomically_and_hydrate_legacy() {
    let mut c = fixture();
    let before = c.workspace.clone();
    for r in [
        json!({"rows":1048576,"columns":0}),
        json!({"rows":0,"columns":16384}),
        json!({"rows":-1,"columns":2}),
        json!({"rows":1.5,"columns":0}),
        json!({"rows":0,"columns":0}),
        json!({"rows":1,"columns":0,"baseRevision":0}),
    ] {
        let mut request = json!({"op":"sheetView","action":"freeze","sheetId":"office-1"});
        request
            .as_object_mut()
            .unwrap()
            .extend(r.as_object().unwrap().clone());
        assert!(c.dispatch(request).is_err());
        assert_eq!(c.workspace, before);
    }
    let path = output().join("legacy-view.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for sheet in saved["sheets"].as_array_mut().unwrap() {
        sheet.as_object_mut().unwrap().remove("frozenPane");
    }
    for sheet in saved["officeWorkbook"]["baseline"].as_array_mut().unwrap() {
        sheet.as_object_mut().unwrap().remove("frozenPane");
    }
    std::fs::write(&path, saved.to_string()).unwrap();
    c.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(c.workspace["sheets"], before["sheets"]);
    assert_eq!(
        export(&mut c, "view-legacy.xlsx").original,
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../test/fixtures/views.xlsx"
        ))
        .unwrap()
    );
}

#[test]
fn new_and_prefixed_views_keep_schema_order_and_namespaces() {
    let mut c = Core::new();
    let id = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"sheetView","action":"freeze","sheetId":id,"rows":1,"columns":0}))
        .unwrap();
    export(&mut c, "view-native.xlsx");
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/filter-prefixed.xlsx")})).unwrap();
    change(&mut c, "freeze", 0, 1);
    let pkg = export(&mut c, "view-prefixed.xlsx");
    let doc = parse(pkg.text("xl/worksheets/sheet1.xml").unwrap()).unwrap();
    for n in doc.descendants().filter(|n| {
        n.is_element()
            && matches!(
                n.tag_name().name(),
                "sheetView" | "sheetViews" | "pane" | "selection"
            )
    }) {
        assert_eq!(
            n.tag_name().namespace(),
            Some("http://schemas.openxmlformats.org/spreadsheetml/2006/main")
        );
    }
    assert_eq!(
        attr(
            doc.descendants()
                .find(|n| n.tag_name().name() == "pane")
                .unwrap(),
            "activePane"
        ),
        Some("topRight")
    );
}

fn sheet_xml(pkg: &Package, index: usize) -> String {
    let path = crate::xlsx::import(pkg).unwrap().0[index]["officePath"]
        .as_str()
        .unwrap()
        .to_string();
    pkg.text(&path).unwrap().to_string()
}

#[test]
fn zoom_is_kept_per_sheet_and_round_trips_as_zoom_scale() {
    let mut c = fixture();
    let zoom = |c: &mut Core, id: &str, z: Value| {
        c.dispatch(
            json!({"op":"sheetView","sheetId":id,"action":"zoom","zoom":z,
            "baseRevision":c.workspace["revision"]}),
        )
    };
    zoom(&mut c, "office-1", json!(150)).unwrap();
    zoom(&mut c, "office-2", json!(75)).unwrap();
    assert_eq!(c.workspace["sheets"][0]["zoom"], 150);
    let pkg = export(&mut c, "view-zoom.xlsx");
    let first = sheet_xml(&pkg, 0);
    assert!(first.contains("zoomScale=\"150\""), "{first}");
    assert!(first.contains("<pane"), "the frozen pane stays: {first}");
    assert!(sheet_xml(&pkg, 1).contains("zoomScale=\"75\""));
    let (sheets, _) = crate::xlsx::import(&pkg).unwrap();
    assert_eq!(sheets[0]["zoom"], 150);
    assert_eq!(sheets[1]["zoom"], 75);

    // 100% is the default: no attribute, no key.
    zoom(&mut c, "office-1", json!(100)).unwrap();
    assert!(c.workspace["sheets"][0].get("zoom").is_none());
    let first = sheet_xml(&export(&mut c, "view-zoom-100.xlsx"), 0);
    assert!(!first.contains("zoomScale=\"150\""), "{first}");
    assert!(
        first.contains("zoomScale=\"70\" workbookViewId=\"1\""),
        "other windows keep their zoom: {first}"
    );

    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["zoom"], 150);
    let before = c.workspace.clone();
    for bad in [json!(9), json!(401), json!(1.5), json!("200"), Value::Null] {
        assert!(zoom(&mut c, "office-1", bad.clone()).is_err(), "{bad}");
        assert_eq!(c.workspace, before);
    }
}

#[test]
fn new_workbooks_write_zoom_too() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let id = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"sheetView","sheetId":id,"action":"zoom","zoom":200}))
        .unwrap();
    let pkg = export(&mut c, "view-zoom-new.xlsx");
    assert_eq!(crate::xlsx::import(&pkg).unwrap().0[0]["zoom"], 200);
}
