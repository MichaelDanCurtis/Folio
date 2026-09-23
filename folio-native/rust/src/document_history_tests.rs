use crate::{document_bookmarks, workspace::Core};
use serde_json::{json, Value};
fn doc(text: &str) -> Value {
    json!({"type":"doc","content":text.split('\n').map(|line| json!({"type":"paragraph","content":[{"type":"text","text":line}]})).collect::<Vec<_>>()})
}
fn core(text: &str) -> Core {
    let mut c = Core::default();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c.dispatch(json!({"op":"document","document":doc(text)}))
        .unwrap();
    c
}
#[test]
fn exact_history_uses_original_utf16_locations_and_repeats_both_directions() {
    for (before, after, edits, inverse) in [
        (
            "aaaa",
            "aaaaa",
            json!([{"start":0,"end":0,"text":"a"}]),
            json!([{"start":0,"end":1,"text":""}]),
        ),
        (
            "😀abab",
            "🚀😀xa",
            json!([{"start":0,"end":0,"text":"🚀"},{"start":2,"end":4,"text":"x"},{"start":5,"end":6,"text":""}]),
            json!([{"start":0,"end":2,"text":""},{"start":4,"end":5,"text":"ab"},{"start":6,"end":6,"text":"b"}]),
        ),
        (
            "abc",
            "c",
            json!([{"start":0,"end":1,"text":""},{"start":1,"end":2,"text":""}]),
            json!([{"start":0,"end":0,"text":"a"},{"start":0,"end":0,"text":"b"}]),
        ),
        (
            "Line\nLine",
            "Line\nLine\nLine",
            json!([{"start":0,"end":0,"text":"Line\n"}]),
            json!([{"start":0,"end":5,"text":""}]),
        ),
    ] {
        let mut c = core(before);
        c.dispatch(json!({"op":"document","document":doc(after),"textEdits":edits}))
            .unwrap();
        for _ in 0..3 {
            let revision = c.workspace["revision"].as_u64().unwrap();
            let undo = c.dispatch(json!({"op":"undo"})).unwrap();
            assert_eq!(
                undo["documentHistoryChange"],
                json!({"workspaceId":c.workspace["id"],"beforeRevision":revision,"afterRevision":revision+1,"textEdits":inverse})
            );
            assert_eq!(
                document_bookmarks::Index::new(&c.workspace["document"]).text,
                format!("{before}\n")
            );
            let redo = c.dispatch(json!({"op":"redo"})).unwrap();
            assert_eq!(redo["documentHistoryChange"]["textEdits"], edits);
            assert_eq!(
                document_bookmarks::Index::new(&c.workspace["document"]).text,
                format!("{after}\n")
            );
        }
        assert!(c
            .dispatch(json!({"op":"get"}))
            .unwrap()
            .get("documentHistoryChange")
            .is_none());
    }
}
#[test]
fn exact_history_rejects_invalid_journals_without_bookmarks_atomically() {
    let mut c = core("😀abc");
    let original = c.workspace.clone();
    let snapshot = c.snapshot();
    for edits in [
        json!([{"start":1,"end":2,"text":"X"}]),
        json!([{"start":4,"end":5,"text":"X"},{"start":2,"end":3,"text":"Y"}]),
        json!([{"start":0,"end":99,"text":"X"}]),
        json!([{"start":0,"end":0,"text":"wrong"}]),
        json!("invalid"),
    ] {
        assert!(c
            .dispatch(json!({"op":"document","document":doc("Xabc"),"textEdits":edits}))
            .is_err());
        assert_eq!(c.workspace, original);
        assert_eq!(c.snapshot()["canUndo"], snapshot["canUndo"]);
        assert_eq!(c.snapshot()["canRedo"], snapshot["canRedo"]);
    }
    c.dispatch(json!({"op":"document","document":doc("Xabc"),"textEdits":[{"start":0,"end":2,"text":"X"}]})).unwrap();
    let undo = c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        undo["documentHistoryChange"]["textEdits"],
        json!([{"start":0,"end":1,"text":"😀"}])
    );
    assert_eq!(c.workspace["document"], original["document"]);
}
#[test]
fn exact_history_survives_measured_proposals_and_mixed_workbook_commands() {
    let mut c = core("aaaa");
    let p = json!({"summary":"Insert leading a","baseRevision":c.workspace["revision"],"document":doc("aaaaa"),"textEdits":[{"start":0,"end":0,"text":"a"}]});
    let prepared = c
        .dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"apply","proposal":p}}))
        .unwrap();
    assert_eq!(prepared["layoutRows"], json!({}));
    c.dispatch(json!({"op":"commitLayoutEdit","token":prepared["layoutToken"],"sizes":{}}))
        .unwrap();
    let sheet = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":sheet,"address":"A1","value":"42"}]}))
        .unwrap();
    let workbook_undo = c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        workbook_undo["documentHistoryChange"]["textEdits"],
        json!([])
    );
    let undo = c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        undo["documentHistoryChange"]["textEdits"],
        json!([{"start":0,"end":1,"text":""}])
    );
    let redo = c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(redo["documentHistoryChange"]["textEdits"], p["textEdits"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["A1"], "42");
}
#[test]
fn history_journals_are_bounded_ephemeral_and_absent_for_untracked_replacements() {
    let mut c = core("aaaa");
    c.dispatch(json!({"op":"document","document":doc("aaaaa")}))
        .unwrap();
    assert!(c
        .dispatch(json!({"op":"undo"}))
        .unwrap()
        .get("documentHistoryChange")
        .is_none());
    c.dispatch(json!({"op":"document","document":doc("aaaaa"),"textEdits":[{"start":0,"end":0,"text":"a"}]})).unwrap();
    assert!(!c.snapshot()["canRedo"].as_bool().unwrap());
    for i in 0..101 {
        c.dispatch(json!({"op":"title","title":format!("Title {i}")}))
            .unwrap();
    }
    for _ in 0..100 {
        c.dispatch(json!({"op":"undo"})).unwrap();
    }
    assert_eq!(c.snapshot()["canUndo"], false);
    let path = std::env::temp_dir().join(format!("folio-history-{}.folio", std::process::id()));
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(!saved.contains("documentHistoryChange"));
    c.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(c.snapshot()["canUndo"], false);
    assert_eq!(c.snapshot()["canRedo"], false);
    assert!(c
        .dispatch(json!({"op":"undo"}))
        .unwrap()
        .get("documentHistoryChange")
        .is_none());
    std::fs::remove_file(path).unwrap();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    assert!(c
        .dispatch(json!({"op":"redo"}))
        .unwrap()
        .get("documentHistoryChange")
        .is_none());
}
