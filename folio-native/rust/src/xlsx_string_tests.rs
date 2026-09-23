use crate::{office::Package, workspace::Core};
use serde_json::json;

fn output() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-string-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn fixture() -> Core {
    let mut core = Core::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/escaped-strings.xlsx");
    core.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    core
}
fn export(core: &mut Core, name: &str) -> Package {
    let path = output().join(name);
    core.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
const RICH: &str = "A\rB🌊 _x0041_ & end";

#[test]
fn escaped_text_import_keeps_literal_types_formula_source_and_unchanged_bytes() {
    let mut c = fixture();
    let sheet = &c.workspace["sheets"][0];
    for (a, value) in [
        ("A30", RICH),
        ("B30", RICH),
        ("C30", RICH),
        ("D30", "'001"),
        ("E30", "'=literal"),
        ("F30", "'TRUE"),
        ("G30", "a\0b\u{1}c\r\n\tend"),
        ("H30", "_xD800_X_xDC00_ _X0041_ _xZZZZ_"),
        ("I30", "_x0041_"),
        ("J30", "=\"_x0041_\""),
        ("K30", "=\"001\""),
        ("L30", "=\"🌊\""),
    ] {
        assert_eq!(sheet["cells"][a], value, "{a}");
    }
    assert_eq!(sheet["cached"]["J30"], "_x0041_");
    assert_eq!(sheet["cached"]["K30"], "001");
    assert_eq!(sheet["cached"]["L30"], "🌊");
    assert_eq!(sheet["cellStyles"]["A30"]["_richText"][0]["text"], "A\rB🌊");
    assert_eq!(
        sheet["cellStyles"]["A30"]["_richText"][1]["text"],
        " _x0041_ & end"
    );
    let original = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test/fixtures/escaped-strings.xlsx"),
    )
    .unwrap();
    assert_eq!(export(&mut c, "original.xlsx").original, original);
}

#[test]
fn escaped_character_ranges_new_text_caches_history_and_reopening_roundtrip() {
    let mut c = fixture();
    let original = export(&mut c, "before.xlsx");
    // Select B, the whole supplementary character and the first literal
    // underscore. Offsets refer to displayed UTF-16, never encoded XML bytes.
    c.dispatch(
        json!({"op":"formatCellText","sheetId":"office-1","address":"B30",
        "baseRevision":c.workspace["revision"],"expectedText":RICH,"start":2,"end":7,
        "style":{"bold":false,"fontSize":16.5}}),
    )
    .unwrap();
    let edited = export(&mut c, "characters.xlsx");
    let imported = crate::xlsx::import(&edited).unwrap().0;
    assert_eq!(imported[0]["cells"]["B30"], RICH);
    assert_eq!(
        imported[0]["cellStyles"]["B30"]["_richText"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["text"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["A\r", "B🌊", " _", "x0041_ & end"]
    );
    assert_eq!(
        edited.entries["xl/sharedStrings.xml"],
        original.entries["xl/sharedStrings.xml"]
    );
    assert_eq!(
        edited.entries["xl/styles.xml"],
        original.entries["xl/styles.xml"]
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(export(&mut c, "undone.xlsx").original, original.original);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(export(&mut c, "redone.xlsx").original, edited.original);
    let path = output().join("strings.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        export(&mut loaded, "reopened.xlsx").original,
        edited.original
    );
    let new = "_x0041_\r\n\t🌊\0&<>";
    c.dispatch(json!({"op":"cells","cells":[
        {"sheetId":"office-1","address":"M30","value":new},
        {"sheetId":"office-1","address":"N30","value":"'_x005F_x000D_"},
        {"sheetId":"office-1","address":"O30","value":"=\"_x0042_\""}
    ]}))
    .unwrap();
    c.dispatch(
        json!({"op":"formatCellText","sheetId":"office-1","address":"I30",
        "baseRevision":c.workspace["revision"],"expectedText":"_x0041_","start":0,"end":7,
        "style":{"italic":true}}),
    )
    .unwrap();
    let saved = export(&mut c, "values.xlsx");
    let sheets = crate::xlsx::import(&saved).unwrap().0;
    assert_eq!(sheets[0]["cells"]["M30"], new);
    assert_eq!(sheets[0]["cells"]["N30"], "_x005F_x000D_");
    assert_eq!(sheets[0]["cells"]["I30"], "_x0041_");
    assert_eq!(sheets[0]["cells"]["J30"], "=\"_x0041_\"");
    assert_eq!(sheets[0]["cached"]["J30"], "_x0041_");
    assert_eq!(sheets[0]["cached"]["K30"], "001");
    assert_eq!(sheets[0]["cached"]["O30"], "_x0042_");
}

#[test]
fn legacy_escaped_workspaces_upgrade_complete_character_spans_and_preserve_user_edits() {
    let mut c = fixture();
    let old = "A_x000D_B_xD83C__xDF0A_ _x005F_x0041_ & end";
    let originals = [
        ("A30", old),
        ("B30", old),
        ("C30", old),
        ("D30", "_x0030_01"),
    ];
    for (a, text) in originals {
        c.workspace["sheets"][0]["cells"][a] = json!(text);
        c.workspace["officeWorkbook"]["baseline"][0]["cells"][a] = json!(text);
        if a != "D30" {
            for key in ["sheets", "baseline"] {
                let sheets = if key == "sheets" {
                    &mut c.workspace["sheets"]
                } else {
                    &mut c.workspace["officeWorkbook"]["baseline"]
                };
                sheets[0]["cellStyles"][a]["_richText"][0]["text"] =
                    json!("A_x000D_B_xD83C__xDF0A_");
                sheets[0]["cellStyles"][a]["_richText"][1]["text"] = json!(" _x005F_x0041_ & end");
            }
        }
    }
    c.workspace["sheets"][0]["cached"]["J30"] = json!("_x005F_x0041_");
    c.workspace["officeWorkbook"]["baseline"][0]["cached"]["J30"] = json!("_x005F_x0041_");
    c.dispatch(
        json!({"op":"formatCellText","sheetId":"office-1","address":"B30",
        "baseRevision":c.workspace["revision"],"expectedText":old,"start":8,"end":31,
        "style":{"bold":false,"fontSize":16.5}}),
    )
    .unwrap();
    c.dispatch(
        json!({"op":"formatCells","sheetId":"office-1","addresses":["A30","B30"],"style":{"italic":true}}),
    )
    .unwrap();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"office-1","address":"C30","value":"User_x0041_"}]})).unwrap();
    let path = output().join("legacy.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    let sheet = &loaded.workspace["sheets"][0];
    assert_eq!(sheet["cells"]["A30"], RICH);
    assert_eq!(sheet["cells"]["B30"], RICH);
    assert_eq!(sheet["cells"]["C30"], "User_x0041_");
    assert_eq!(sheet["cells"]["D30"], "'001");
    assert_eq!(sheet["cached"]["J30"], "_x0041_");
    assert_eq!(sheet["cellStyles"]["A30"]["_richText"][0]["text"], "A\rB🌊");
    assert_eq!(sheet["cellStyles"]["A30"]["_fontOverrides"]["italic"], true);
    let ranges = &sheet["cellStyles"]["B30"]["_richTextEdits"];
    assert_eq!(ranges[0]["start"], 2);
    assert_eq!(ranges[0]["end"], 7);
    assert_eq!(sheet["cellStyles"]["B30"]["_fontOverrides"]["italic"], true);
    let exported = export(&mut loaded, "legacy.xlsx");
    let imported = crate::xlsx::import(&exported).unwrap().0;
    assert_eq!(imported[0]["cells"]["C30"], "User_x0041_");
    assert!(imported[0]["cellStyles"]["B30"]["_richText"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["style"]["italic"] == true));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let once = loaded.workspace.clone();
    let package = Package::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test/fixtures/escaped-strings.xlsx")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    crate::xlsx_styles::hydrate(&mut loaded.workspace, &package).unwrap();
    assert_eq!(loaded.workspace, once);

    // Old selections inside an encoded token cannot map to a character span.
    // Opening fails without replacing the active workspace or touching the file.
    let mut ambiguous: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    ambiguous["sheets"][0]["cellStyles"]["B30"]["_richTextEdits"] =
        json!([{"start":2,"end":3,"style":{"bold":true}}]);
    let bad_path = output().join("ambiguous.folio");
    let bad_bytes = serde_json::to_vec(&ambiguous).unwrap();
    std::fs::write(&bad_path, &bad_bytes).unwrap();
    let error = loaded
        .dispatch(json!({"op":"load","path":bad_path}))
        .unwrap_err();
    assert!(error.contains("inside an Excel escape sequence"), "{error}");
    assert_eq!(loaded.workspace, once);
    assert_eq!(std::fs::read(&bad_path).unwrap(), bad_bytes);
}
