use crate::{
    office::{parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
fn folder() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-alignment-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn fixture() -> Core {
    let mut c = Core::new();
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/alignment.xlsx");
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    c
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = folder().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn format(c: &mut Core, a: &str, style: Value) {
    c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":[a],"baseRevision":c.workspace["revision"],"style":style})).unwrap();
}
#[test]
fn alignment_import_inheritance_partial_edit_preservation_history_and_reopen() {
    let mut c = fixture();
    let original = export(&mut c, "original.xlsx");
    let sheet = &c.workspace["sheets"][0];
    let style = &sheet["cellStyles"]["A30"];
    assert_eq!(style["horizontalAlignment"], "right");
    assert_eq!(style["verticalAlignment"], "top");
    assert_eq!(style["textRotation"], -45);
    assert_eq!(style["wrapText"], true);
    assert_eq!(style["indent"], 2);
    assert_eq!(style["readingOrder"], 2);
    assert_eq!(sheet["cellStyles"]["C30"]["horizontalAlignment"], "left");
    assert_eq!(sheet["cellStyles"]["C30"]["textRotation"], 90);
    assert_eq!(sheet["cellStyles"]["D30"]["horizontalAlignment"], "general");
    assert_eq!(
        sheet["styleDefaults"]["rows"]["30"]["verticalAlignment"],
        "center"
    );
    assert_eq!(
        sheet["styleDefaults"]["columns"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["style"]["textRotation"],
        -45
    );
    format(
        &mut c,
        "A30",
        json!({"horizontalAlignment":"center","verticalAlignment":"center","wrapText":false,
      "shrinkToFit":true,"textRotation":-30,"indent":0,"readingOrder":0,"justifyLastLine":false}),
    );
    let edited = export(&mut c, "edited.xlsx");
    let projected = crate::xlsx::import(&edited).unwrap().0;
    for (key, value) in c.workspace["sheets"][0]["cellStyles"]["A30"]
        .as_object()
        .unwrap()
    {
        if !key.starts_with('_') {
            assert_eq!(&projected[0]["cellStyles"]["A30"][key], value, "{key}");
        }
    }
    let xml = parse(edited.text("xl/styles.xml").unwrap()).unwrap();
    let align = xml
        .descendants()
        .filter(|n| n.has_tag_name((S, "alignment")))
        .last()
        .unwrap();
    assert_eq!(align.attribute("textRotation"), Some("120"));
    assert_eq!(
        align.attribute(("urn:folio:alignment", "horizontal")),
        Some("keep-qualified")
    );
    assert_eq!(align.attribute("relativeIndent"), Some("-2"));
    assert_eq!(
        edited.entries["xl/sharedStrings.xml"],
        original.entries["xl/sharedStrings.xml"]
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(export(&mut c, "undone.xlsx").original, original.original);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(export(&mut c, "redone.xlsx").original, edited.original);
    let path = folder().join("alignment.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        export(&mut loaded, "reopened.xlsx").original,
        edited.original
    );
    format(&mut c, "C30", json!({"horizontalAlignment":"right"}));
    let inherited = export(&mut c, "inherited.xlsx");
    let projected = crate::xlsx::import(&inherited).unwrap().0;
    assert_eq!(
        projected[0]["cellStyles"]["C30"]["horizontalAlignment"],
        "right"
    );
    assert_eq!(
        projected[0]["cellStyles"]["C30"]["verticalAlignment"],
        "center"
    );
    assert_eq!(projected[0]["cellStyles"]["C30"]["textRotation"], 90);
}
#[test]
fn alignment_new_styles_rotation_limits_atomic_errors_and_legacy_defaults() {
    let mut c = fixture();
    for (i, rotation) in [-90, -1, 0, 1, 90, 255].iter().enumerate() {
        format(
            &mut c,
            &format!("{}40", (b'A' + i as u8) as char),
            json!({"textRotation":rotation,"verticalAlignment":"top"}),
        );
    }
    let pkg = export(&mut c, "rotations.xlsx");
    let projected = crate::xlsx::import(&pkg).unwrap().0;
    for (i, rotation) in [-90, -1, 0, 1, 90, 255].iter().enumerate() {
        assert_eq!(
            projected[0]["cellStyles"][format!("{}40", (b'A' + i as u8) as char)]["textRotation"],
            *rotation
        );
    }
    for style in [
        json!({"textRotation":91}),
        json!({"textRotation":-91}),
        json!({"textRotation":0.5}),
        json!({"indent":251}),
        json!({"indent":-1}),
        json!({"readingOrder":3}),
        json!({"wrapText":"true"}),
        json!({"horizontalAlignment":"middle"}),
        json!({"verticalAlignment":"general"}),
    ] {
        let before = c.workspace.clone();
        assert!(c
            .dispatch(
                json!({"op":"formatCells","sheetId":"office-1","addresses":["A30"],"style":style})
            )
            .is_err());
        assert_eq!(c.workspace, before);
    }
    let before = c.workspace.clone();
    assert!(c.dispatch(json!({"op":"formatCells","sheetId":"office-1","addresses":["A30"],"baseRevision":0,"style":{"wrapText":true}})).is_err());
    assert_eq!(c.workspace, before);
    let mut legacy = fixture();
    let source = export(&mut legacy, "legacy-source.xlsx");
    fn remove(value: &mut Value) {
        match value {
            Value::Object(map) => {
                for (key, _) in crate::xlsx_alignment::KEYS {
                    map.remove(*key);
                }
                for child in map.values_mut() {
                    remove(child);
                }
            }
            Value::Array(values) => {
                for child in values {
                    remove(child);
                }
            }
            _ => {}
        }
    }
    remove(&mut legacy.workspace["sheets"]);
    remove(&mut legacy.workspace["officeWorkbook"]["baseline"]);
    legacy.workspace["sheets"][0]["cellStyles"]["A30"]["horizontalAlignment"] = json!("center");
    crate::xlsx_styles::hydrate(&mut legacy.workspace, &source).unwrap();
    assert_eq!(
        legacy.workspace["sheets"][0]["cellStyles"]["A30"]["horizontalAlignment"],
        "center"
    );
    assert_eq!(
        legacy.workspace["sheets"][0]["cellStyles"]["A30"]["textRotation"],
        -45
    );
    assert_eq!(
        legacy.workspace["sheets"][0]["styleDefaults"]["rows"]["30"]["textRotation"],
        90
    );
    let once = legacy.workspace.clone();
    crate::xlsx_styles::hydrate(&mut legacy.workspace, &source).unwrap();
    assert_eq!(legacy.workspace, once);
    let mut blank = Core::new();
    blank.dispatch(json!({"op":"new","blank":true})).unwrap();
    blank.dispatch(json!({"op":"formatCells","sheetId":"sheet-1","addresses":["A1"],"style":{"horizontalAlignment":"center","wrapText":true}})).unwrap();
    let p = export(&mut blank, "new.xlsx");
    assert_eq!(
        crate::xlsx::import(&p).unwrap().0[0]["cellStyles"]["A1"]["horizontalAlignment"],
        "center"
    );
}
