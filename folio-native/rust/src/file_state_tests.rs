use crate::workspace::Core;
use serde_json::{json, Value};

#[test]
fn recovery_preview_is_read_only_and_restore_has_a_new_identity() {
    let mut c = Core::new();
    let directory = std::env::temp_dir().join(format!("folio-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("copy.folio");
    cell(&mut c, "B4", "4242");
    let original = c.workspace.clone();
    let saved = c
        .dispatch(json!({"op":"save","path":path,"describe":true}))
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    cell(&mut c, "B4", "5151");
    let live = c.workspace.clone();
    let preview = c
        .dispatch(json!({"op":"inspectWorkspace","path":path}))
        .unwrap();
    assert_eq!(preview, saved["description"]);
    assert_eq!(c.workspace, live);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], original["sheets"]);
    let malformed = directory.join("bad.folio");
    std::fs::write(&malformed, "{broken").unwrap();
    let live = c.workspace.clone();
    assert!(c
        .dispatch(json!({"op":"recoverWorkspace","path":malformed}))
        .is_err());
    assert_eq!(live, c.workspace);
    c.dispatch(json!({"op":"recoverWorkspace","path":path}))
        .unwrap();
    assert_ne!(c.workspace["id"], original["id"]);
    assert_eq!(c.workspace["document"], original["document"]);
    assert_eq!(c.workspace["sheets"], original["sheets"]);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let restored = c.workspace.clone();
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(restored, c.workspace);
    std::fs::remove_dir_all(directory).unwrap();
}

fn states(c: &mut Core) -> Value {
    c.snapshot()["fileStates"].clone()
}

#[test]
fn recovery_retains_both_office_packages_and_exports() {
    let mut c = Core::new();
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures");
    for name in ["word-outline-levels.docx", "structure.xlsx"] {
        c.dispatch(json!({"op":"importOffice","path":fixtures.join(name)}))
            .unwrap();
    }
    let directory =
        std::env::temp_dir().join(format!("folio-recovery-office-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("source.folio");
    let before = states(&mut c);
    for format in ["docx", "xlsx"] {
        c.dispatch(json!({"op":"exportOffice","format":format,"path":directory.join(format!("before.{format}"))})).unwrap();
    }
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c.dispatch(json!({"op":"inspectWorkspace","path":path}))
        .unwrap();
    assert!(c.workspace["officeDocument"].is_null());
    c.dispatch(json!({"op":"recoverWorkspace","path":path}))
        .unwrap();
    let after = states(&mut c);
    for format in ["docx", "xlsx"] {
        assert_eq!(before[format], after[format]);
        let output = directory.join(format!("after.{format}"));
        c.dispatch(json!({"op":"exportOffice","format":format,"path":output}))
            .unwrap();
        let before = crate::office::Package::read(
            directory.join(format!("before.{format}")).to_str().unwrap(),
        )
        .unwrap();
        let after = crate::office::Package::read(output.to_str().unwrap()).unwrap();
        assert_eq!(before.entries, after.entries);
    }
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_dir_all(directory).unwrap();
}
fn cell(c: &mut Core, address: &str, value: &str) {
    c.dispatch(
        json!({"op":"cells","cells":[{"sheetId":"budget","address":address,"value":value}]}),
    )
    .unwrap();
}
#[test]
fn saved_content_is_independent_of_revision_and_unrelated_edits() {
    let mut c = Core::new();
    let first = states(&mut c);
    cell(&mut c, "B4", "9876");
    let edited = states(&mut c);
    assert_eq!(first["docx"], edited["docx"]);
    assert_ne!(first["xlsx"], edited["xlsx"]);
    assert_ne!(first["folio"], edited["folio"]);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(first, states(&mut c));
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(edited, states(&mut c));
    let doc = json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"An independent edit"}]}]});
    c.dispatch(json!({"op":"document","document":doc})).unwrap();
    let document = states(&mut c);
    assert_eq!(document["xlsx"], edited["xlsx"]);
    assert_ne!(document["docx"], edited["docx"]);
    c.dispatch(json!({"op":"title","title":"New title"}))
        .unwrap();
    let renamed = states(&mut c);
    assert_eq!(document["docx"], renamed["docx"]);
    assert_eq!(document["xlsx"], renamed["xlsx"]);
    assert_ne!(document["folio"], renamed["folio"]);
}
#[test]
fn linked_document_tracks_visible_source_values_and_formatting() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"document","document":{"type":"doc","content":[{"type":"workbookBlock","attrs":{"sheetId":"budget","range":"B4:B4","title":"Budget","mode":"table"}}]}})).unwrap();
    let initial = states(&mut c);
    cell(&mut c, "A20", "Unrelated");
    assert_eq!(initial["docx"], states(&mut c)["docx"]);
    cell(&mut c, "B4", "1234");
    let changed = states(&mut c);
    assert_ne!(initial["docx"], changed["docx"]);
    c.dispatch(json!({"op":"formatCells","sheetId":"budget","addresses":["B4"],"style":{"numFmt":"$#,##0.00"}})).unwrap();
    assert_ne!(changed["docx"], states(&mut c)["docx"]);
}
#[test]
fn source_identity_survives_package_revisions_and_save_reopen() {
    let mut c = Core::new();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/structure.xlsx"
    );
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let before = c.workspace["officeWorkbook"].clone();
    let state = states(&mut c);
    c.dispatch(
        json!({"op":"structure","sheetId":"office-1","action":"insertRows","at":2,"count":1}),
    )
    .unwrap();
    assert_ne!(
        before["packageId"],
        c.workspace["officeWorkbook"]["packageId"]
    );
    assert_eq!(
        before["sourceId"],
        c.workspace["officeWorkbook"]["sourceId"]
    );
    let changed = states(&mut c);
    assert_ne!(state["xlsx"], changed["xlsx"]);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(state, states(&mut c));
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(changed, states(&mut c));
    let directory = std::env::temp_dir().join(format!("folio-file-state-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    for format in ["docx", "xlsx", "folio"] {
        let path = directory.join(format!("saved.{format}"));
        let result = c.dispatch(json!({"op":if format == "folio" {"save"} else {"exportOffice"},"format":format,"path":path})).unwrap();
        assert_eq!(result["contentSignature"], changed[format]);
    }
    let mut loaded = Core::new();
    loaded
        .dispatch(json!({"op":"load","path":directory.join("saved.folio")}))
        .unwrap();
    assert_eq!(changed, states(&mut loaded));
    assert_eq!(
        before["sourceId"],
        loaded.workspace["officeWorkbook"]["sourceId"]
    );
    loaded
        .dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    assert_ne!(
        before["sourceId"],
        loaded.workspace["officeWorkbook"]["sourceId"]
    );
}
#[test]
fn native_and_legacy_sources_gain_stable_identity_before_package_changes() {
    let mut native = json!({"id":"a"});
    crate::file_state::replace_package(&mut native, "officeDocument", "new-1");
    assert_eq!(native["officeDocument"]["sourceId"], "a:docx");
    crate::file_state::replace_package(&mut native, "officeDocument", "new-2");
    assert_eq!(native["officeDocument"]["sourceId"], "a:docx");
    let mut legacy = json!({"id":"a","officeWorkbook":{"packageId":"original"}});
    crate::file_state::replace_package(&mut legacy, "officeWorkbook", "edited");
    assert_eq!(legacy["officeWorkbook"]["sourceId"], "original");
}

#[test]
fn native_style_replacement_retains_file_lineage_and_undo_content() {
    let mut c = Core::new();
    let catalog = c.dispatch(json!({"op":"documentStyles"})).unwrap();
    let mut custom = catalog["catalog"]["styles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "Heading1")
        .unwrap()
        .clone();
    custom["id"] = json!("SavedCustom");
    custom["name"] = json!("Saved custom heading");
    fn style(c: &mut Core, mut request: Value) {
        request["op"] = json!("documentStyle");
        request["workspaceId"] = c.workspace["id"].clone();
        request["baseRevision"] = c.workspace["revision"].clone();
        c.dispatch(request).unwrap();
    }
    style(&mut c, json!({"action":"upsert","style":custom}));
    style(
        &mut c,
        json!({"action":"apply","styleId":"SavedCustom","paths":[[0]]}),
    );
    let before = states(&mut c);
    style(
        &mut c,
        json!({"action":"delete","styleId":"SavedCustom","replacementStyleId":"Normal"}),
    );
    assert_eq!(
        c.workspace["officeDocument"]["sourceId"],
        format!("{}:docx", c.workspace["id"].as_str().unwrap())
    );
    let after = states(&mut c);
    assert_ne!(before["docx"], after["docx"]);
    assert_eq!(before["xlsx"], after["xlsx"]);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(before, states(&mut c));
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(after, states(&mut c));
}

#[test]
fn discard_close_writes_blank_recovery_without_mutating_live_content_or_history() {
    let mut c = Core::new();
    cell(&mut c, "B4", "9999");
    let before = c.snapshot();
    let dir = std::env::temp_dir().join(format!("folio-discard-close-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let recovery = dir.join("workspace.folio");
    let backup = dir.join("discarded.folio");
    c.dispatch(json!({"op":"save","path":recovery})).unwrap();
    c.dispatch(json!({"op":"save","path":backup})).unwrap();
    let bytes = std::fs::read(&recovery).unwrap();
    assert!(c
        .dispatch(json!({"op":"saveEmptyWorkspace","path":dir}))
        .is_err());
    assert_eq!(c.snapshot(), before);
    assert_eq!(std::fs::read(&recovery).unwrap(), bytes);
    c.dispatch(json!({"op":"saveEmptyWorkspace","path":recovery}))
        .unwrap();
    assert_eq!(c.snapshot(), before);
    let mut restarted = Core::new();
    let fresh = restarted
        .dispatch(json!({"op":"load","path":recovery}))
        .unwrap();
    assert_eq!(fresh["blankWorkspace"], true);
    assert_ne!(fresh["workspace"]["id"], before["workspace"]["id"]);
    assert_eq!(fresh["workspace"]["sheets"][0]["cells"], json!({}));
    let restored = restarted
        .dispatch(json!({"op":"load","path":backup}))
        .unwrap();
    assert_eq!(restored["workspace"], before["workspace"]);
    assert_eq!(restored["blankWorkspace"], false);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["B4"], "2400");
}
