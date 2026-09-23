use crate::{office::Package, workspace::Core};
use serde_json::{json, Value};
fn folder() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-column-fit-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn fixture() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/column-autofit.xlsx")})).unwrap();
    c
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = folder().join(format!("{name}.xlsx"));
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn command(c: &Core) -> Value {
    json!({"op":"sheetGeometry","sheetId":"office-1","axis":"columns","action":"autoFit","at":1,"count":4,"sizes":{"1":12.625,"2":40.25,"3":20.75},"baseRevision":c.workspace["revision"]})
}
#[test]
fn measured_columns_preserve_hidden_styles_numbers_and_exact_history() {
    let mut c = fixture();
    let original = export(&mut c, "original");
    let before = c.workspace.clone();
    c.dispatch(command(&c)).unwrap();
    assert_eq!(
        c.workspace["revision"].as_u64().unwrap(),
        before["revision"].as_u64().unwrap() + 1
    );
    assert_eq!(
        c.workspace["sheets"][0]["hiddenColumns"],
        before["sheets"][0]["hiddenColumns"]
    );
    assert_eq!(
        c.workspace["sheets"][0]["rowHeights"],
        before["sheets"][0]["rowHeights"]
    );
    assert_eq!(c.workspace["sheets"][0]["columnWidths"]["4"], 17.0);
    let edited = export(&mut c, "edited");
    assert_eq!(
        edited.entries["xl/styles.xml"],
        original.entries["xl/styles.xml"]
    );
    assert_eq!(
        crate::xlsx::import(&edited).unwrap().0[0]["columnWidths"]["2"],
        40.25
    );
    let after = c.workspace.clone();
    c.dispatch(command(&c)).unwrap();
    assert_eq!(c.workspace, after);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(export(&mut c, "undone").original, original.original);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(export(&mut c, "redone").original, edited.original);
    let saved = folder().join("columns.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut reopened = Core::new();
    reopened
        .dispatch(json!({"op":"load","path":saved}))
        .unwrap();
    assert_eq!(reopened.workspace["sheets"], c.workspace["sheets"]);
    assert_eq!(export(&mut reopened, "reopened").original, edited.original);
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertColumns","at":2,"count":2}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["columnWidths"]["4"], 40.25);
    assert_eq!(c.workspace["sheets"][0]["hiddenColumns"], json!([6]));
}
#[test]
fn column_measurements_and_localized_normal_font_validate_and_upgrade() {
    let mut c = fixture();
    let normal = &c.workspace["sheets"][0]["styleDefaults"]["normalFont"];
    assert_eq!(normal["fontName"], "Georgia");
    assert_eq!(normal["fontSize"], 14.0);
    assert_ne!(
        normal["fontName"],
        c.workspace["sheets"][0]["styleDefaults"]["default"]["fontName"]
    );
    for patch in [
        json!({"sizes":{"1":10,"2":20,"3":30,"4":40}}),
        json!({"sizes":{"1":256,"2":20,"3":30}}),
        json!({"sizes":{"1":10,"2":20}}),
        json!({"at":16384,"count":2}),
        json!({"baseRevision":null}),
    ] {
        let before = c.workspace.clone();
        let mut request = command(&c);
        request
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert!(c.dispatch(request).is_err());
        assert_eq!(c.workspace, before);
    }
    let expected = c.workspace["sheets"].clone();
    let path = folder().join("legacy.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    saved["sheets"][0]["styleDefaults"]
        .as_object_mut()
        .unwrap()
        .remove("normalFont");
    saved["officeWorkbook"]["baseline"][0]["styleDefaults"]
        .as_object_mut()
        .unwrap()
        .remove("normalFont");
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    c.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(c.workspace["sheets"], expected);
    assert_eq!(
        export(&mut c, "legacy-upgraded").original,
        export(&mut fixture(), "normal-source").original
    );
    let mut c = Core::new();
    let id = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"sheetGeometry","sheetId":id,"axis":"columns","action":"autoFit","at":16384,"count":1,"sizes":{"16384":255},"baseRevision":c.workspace["revision"]})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["columnWidths"]["16384"], 255);
    export(&mut c, "last-column");
}
