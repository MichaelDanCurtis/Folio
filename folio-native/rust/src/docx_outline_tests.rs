use crate::{docx, office::Package, workspace::Core};
use serde_json::{json, Value};
fn path(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-outline-tests");
    std::fs::create_dir_all(&p).unwrap();
    p.join(name)
}
fn setup() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":format!("{}/../test/fixtures/word-outline-levels.docx",env!("CARGO_MANIFEST_DIR"))})).unwrap();
    c
}
fn change(c: &mut Core, d: Value) {
    c.dispatch(json!({"op":"document","document":d,"baseRevision":c.workspace["revision"]}))
        .unwrap();
}
fn export(c: &mut Core, name: &str) -> Package {
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":path(name)}))
        .unwrap();
    Package::read(path(name).to_str().unwrap()).unwrap()
}
#[test]
fn direct_outline_precedence_all_levels_clear_history_and_recovery() {
    let mut c = setup();
    let original = c.workspace["document"].clone();
    for (i, level) in [(0, 1), (2, 9), (3, 2), (4, 4)] {
        assert_eq!(original["content"][i]["attrs"]["level"], level);
    }
    assert_eq!(original["content"][1]["type"], "paragraph");
    assert_eq!(
        original["content"][1]["attrs"]["paragraphFormat"]["styleId"],
        "Heading1"
    );
    assert_eq!(
        original["content"][5]["content"][0]["content"][0]["content"][0]["attrs"]["level"],
        8
    );
    let before = export(&mut c, "original.docx");
    for level in 0..10 {
        let mut d = c.workspace["document"].clone();
        d["content"][0]["attrs"]["paragraphFormat"]["direct"]["outlineLevel"] = json!(level);
        change(&mut c, d);
        let after = c.workspace["document"].clone();
        assert_eq!(
            after["content"][0]["type"],
            if level == 9 { "paragraph" } else { "heading" }
        );
        assert_eq!(
            after["content"][0]["attrs"]["level"],
            if level == 9 {
                Value::Null
            } else {
                json!(level + 1)
            }
        );
        assert_eq!(
            after["content"][0]["content"],
            original["content"][0]["content"]
        );
        let p = export(&mut c, &format!("level-{level}.docx"));
        assert_eq!(docx::import(&p).unwrap().0, after);
        for (name, data) in &before.entries {
            if name != "word/document.xml" {
                assert_eq!(&p.entries[name], data, "{name}");
            }
        }
    }
    let last = c.workspace["document"].clone();
    let mut d = last.clone();
    d["content"][0]["attrs"]["paragraphFormat"]["direct"]
        .as_object_mut()
        .unwrap()
        .remove("outlineLevel");
    change(&mut c, d);
    let cleared = c.workspace["document"].clone();
    assert_eq!(cleared["content"][0]["type"], "paragraph");
    assert_eq!(
        docx::import(&export(&mut c, "cleared.docx")).unwrap().0,
        cleared
    );
    c.dispatch(json!({"op":"save","path":path("cleared.folio")}))
        .unwrap();
    let mut reopen = Core::new();
    reopen
        .dispatch(json!({"op":"load","path":path("cleared.folio")}))
        .unwrap();
    assert_eq!(reopen.workspace["document"], cleared);
    assert_eq!(
        docx::import(&export(&mut reopen, "cleared-reopened.docx"))
            .unwrap()
            .0,
        cleared
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], last);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["document"], cleared);
    for bad in [json!(-1), json!(10), json!(1.5), json!("2"), Value::Null] {
        let prior = c.workspace.clone();
        let mut d = cleared.clone();
        d["content"][0]["attrs"]["paragraphFormat"]["direct"]["outlineLevel"] = bad;
        assert!(c.dispatch(json!({"op":"document","document":d})).is_err());
        assert_eq!(c.workspace, prior);
    }
}
#[test]
fn direct_outline_wins_over_style_apply_modify_and_removal() {
    let mut c = setup();
    let original = c.workspace["document"].clone();
    c.dispatch(json!({"op":"documentStyle","workspaceId":c.workspace["id"],"action":"apply","styleId":"Heading2","paths":[[0],[1]],"baseRevision":c.workspace["revision"]})).unwrap();
    assert_eq!(c.workspace["document"]["content"][0]["attrs"]["level"], 1);
    assert_eq!(c.workspace["document"]["content"][1]["type"], "paragraph");
    let catalog = c.dispatch(json!({"op":"documentStyles"})).unwrap();
    let mut s = catalog["catalog"]["styles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "Structureonly")
        .unwrap()
        .clone();
    s["properties"]["outlineLevel"] = json!(4);
    c.dispatch(json!({"op":"documentStyle","workspaceId":c.workspace["id"],"action":"upsert","style":s,"baseRevision":c.workspace["revision"]})).unwrap();
    assert_eq!(c.workspace["document"]["content"][2]["attrs"]["level"], 9);
    assert_eq!(c.workspace["document"]["content"][3]["attrs"]["level"], 5);
    c.dispatch(json!({"op":"documentStyle","workspaceId":c.workspace["id"],"action":"delete","styleId":"Structureonly","replacementStyleId":"Normal","baseRevision":c.workspace["revision"]})).unwrap();
    assert_eq!(c.workspace["document"]["content"][2]["attrs"]["level"], 9);
    assert_eq!(c.workspace["document"]["content"][3]["type"], "paragraph");
    for i in 0..5 {
        assert_eq!(
            c.workspace["document"]["content"][i]["content"],
            original["content"][i]["content"]
        );
    }
    assert_eq!(
        docx::import(&export(&mut c, "style-changes.docx"))
            .unwrap()
            .0,
        c.workspace["document"]
    );
}
#[test]
fn legacy_outline_hydration_preserves_edits_and_upgrades_baseline_together() {
    let mut c = setup();
    c.dispatch(json!({"op":"save","path":path("legacy.folio")}))
        .unwrap();
    let mut w: Value =
        serde_json::from_slice(&std::fs::read(path("legacy.folio")).unwrap()).unwrap();
    fn strip(n: &mut Value) {
        if let Some(p) = n
            .get_mut("attrs")
            .and_then(|a| a.get_mut("paragraphFormat"))
            .and_then(|f| f.get_mut("direct"))
            .and_then(Value::as_object_mut)
        {
            p.remove("outlineLevel");
        }
        for c in n
            .get_mut("content")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            strip(c);
        }
    }
    strip(&mut w["document"]);
    strip(&mut w["officeDocument"]["baseline"]);
    w["document"]["content"][0]["content"][0]["text"] = json!("Retained local text");
    w["document"]["content"][0]["attrs"]["paragraphFormat"]["direct"]["left"] = json!(480);
    std::fs::write(path("legacy.folio"), serde_json::to_vec(&w).unwrap()).unwrap();
    let mut reopen = Core::new();
    reopen
        .dispatch(json!({"op":"load","path":path("legacy.folio")}))
        .unwrap();
    assert_eq!(
        reopen.workspace["document"]["content"][0]["content"][0]["text"],
        "Retained local text"
    );
    assert_eq!(
        reopen.workspace["document"]["content"][0]["attrs"]["paragraphFormat"]["direct"]["left"],
        480
    );
    assert_eq!(
        reopen.workspace["document"]["content"][0]["attrs"]["level"],
        1
    );
    assert_eq!(
        reopen.workspace["document"]["content"][1]["type"],
        "paragraph"
    );
    assert_eq!(
        reopen.workspace["officeDocument"]["baseline"],
        c.workspace["officeDocument"]["baseline"]
    );
}

#[test]
fn native_outline_materializes_style_identity_and_retains_subsequent_style_commands() {
    let mut c = Core::new();
    let before = c.workspace["document"].clone();
    let mut d = before.clone();
    d["content"][0] = json!({"type":"paragraph","attrs":{"paragraphFormat":{
        "styleId":"Heading1","inheritedOutlineLevel":0,
        "inherited":{},"direct":{"outlineLevel":9},
        "inheritedRun":{"sz":48}}},"content":[{"type":"text","text":"Native heading appearance"}]});
    change(&mut c, d);
    assert!(c.workspace["document"]["attrs"]["wordStyles"].is_object());
    assert_eq!(c.workspace["document"]["content"][0]["type"], "paragraph");
    let mut cleared = c.workspace["document"].clone();
    cleared["content"][0]["attrs"]["paragraphFormat"]["direct"]
        .as_object_mut()
        .unwrap()
        .remove("outlineLevel");
    change(&mut c, cleared);
    assert_eq!(c.workspace["document"]["content"][0]["attrs"]["level"], 1);
    c.dispatch(json!({"op":"documentStyle","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"action":"apply","styleId":"Heading3","paths":[[0]]})).unwrap();
    assert_eq!(c.workspace["document"]["content"][0]["attrs"]["level"], 3);
    assert_eq!(
        docx::import(&export(&mut c, "native-outline.docx"))
            .unwrap()
            .0["content"][0]["attrs"]["level"],
        3
    );
    for _ in 0..3 {
        c.dispatch(json!({"op":"undo"})).unwrap();
    }
    assert_eq!(c.workspace["document"], before);
}
