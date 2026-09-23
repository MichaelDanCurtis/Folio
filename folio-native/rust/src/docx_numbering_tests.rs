use crate::{
    docx, docx_numbering,
    office::{attr, parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
fn output() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-numbering-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn fixture(name: &str) -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":format!("{}/../test/fixtures/{name}.docx",env!("CARGO_MANIFEST_DIR"))})).unwrap();
    c
}
fn export(c: &mut Core, name: &str) -> Package {
    let p = output().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":p}))
        .unwrap();
    Package::read(p.to_str().unwrap()).unwrap()
}
fn change(c: &mut Core, document: Value) {
    c.dispatch(json!({"op":"document","document":document,"baseRevision":c.workspace["revision"]}))
        .unwrap();
}
#[test]
fn docx_numbering_imports_instance_overrides_levels_styles_and_exact_undo() {
    let mut c = fixture("numbering");
    let nodes = &c.workspace["document"]["content"];
    assert_eq!(nodes[0]["attrs"]["numbering"]["numId"], "7");
    assert_eq!(nodes[0]["attrs"]["numbering"]["levels"][0]["start"], 5);
    assert_eq!(nodes[1]["attrs"]["numbering"]["level"], 1);
    assert_eq!(nodes[1]["attrs"]["numbering"]["levels"][1]["restart"], 0);
    assert_eq!(nodes[6]["attrs"]["numbering"]["levels"][0]["start"], 2);
    assert_eq!(
        nodes[7]["attrs"]["numbering"]["levels"][0]["format"],
        "upperRoman"
    );
    assert_eq!(nodes[8]["attrs"]["numbering"]["numId"], "8");
    assert!(nodes[9]["attrs"]["numbering"].is_null());
    c.dispatch(json!({"op":"title","title":"Word numbering verification sample"}))
        .unwrap();
    c.dispatch(json!({"op":"save","path":output().join("native-preview.folio")}))
        .unwrap();
    let original = export(&mut c, "original.docx");
    let mut next = c.workspace["document"].clone();
    next["content"][0]["content"][0]["text"] = json!("Edited project");
    change(&mut c, next);
    let edited = export(&mut c, "text-edited.docx");
    assert_eq!(
        edited.entries["word/numbering.xml"],
        original.entries["word/numbering.xml"]
    );
    assert_eq!(
        edited.entries["word/styles.xml"],
        original.entries["word/styles.xml"]
    );
    assert_eq!(
        docx::import(&edited).unwrap().0["content"][0]["content"][0]["marks"],
        json!([{"type":"bold"}])
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(export(&mut c, "undone.docx").original, original.original);
}
#[test]
fn docx_numbering_restarts_removes_inheritance_saves_reopens_and_hydrates() {
    let mut c = fixture("numbering");
    let original = c.workspace["document"].clone();
    let mut next = original.clone();
    for i in [4, 5] {
        next["content"][i]["attrs"]["numbering"]["numId"] = Value::Null;
        next["content"][i]["attrs"]["numbering"]["instance"] = json!("restart-test");
        next["content"][i]["attrs"]["numbering"]["levels"][0]["start"] = json!(10);
    }
    next["content"][8]["attrs"]["numbering"] = Value::Null;
    next["content"][8]["attrs"]["textAlign"] = json!("center");
    change(&mut c, next);
    let edited = export(&mut c, "restarted.docx");
    let re = docx::import(&edited).unwrap().0;
    assert_ne!(re["content"][4]["attrs"]["numbering"]["numId"], "7");
    assert_eq!(
        re["content"][4]["attrs"]["numbering"]["numId"],
        re["content"][5]["attrs"]["numbering"]["numId"]
    );
    assert_eq!(
        re["content"][4]["attrs"]["numbering"]["levels"][0]["start"],
        10
    );
    assert!(re["content"][8]["attrs"]["numbering"].is_null());
    let save = output().join("numbering.folio");
    c.dispatch(json!({"op":"save","path":save})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":save})).unwrap();
    assert_eq!(loaded.workspace["document"], c.workspace["document"]);
    assert_eq!(export(&mut loaded, "reopened.docx").entries, edited.entries);
    // Legacy workspaces gain only missing metadata; explicit removal stays removed.
    let mut disk: Value = serde_json::from_slice(&std::fs::read(&save).unwrap()).unwrap();
    disk["document"]["content"][0]["attrs"]
        .as_object_mut()
        .unwrap()
        .remove("numbering");
    for n in disk["officeDocument"]["baseline"]["content"]
        .as_array_mut()
        .unwrap()
    {
        n["attrs"].as_object_mut().unwrap().remove("numbering");
    }
    std::fs::write(&save, disk.to_string()).unwrap();
    loaded.dispatch(json!({"op":"load","path":save})).unwrap();
    assert_eq!(
        loaded.workspace["document"]["content"][0]["attrs"]["numbering"],
        original["content"][0]["attrs"]["numbering"]
    );
    assert!(loaded.workspace["document"]["content"][8]["attrs"]["numbering"].is_null());
    export(&mut loaded, "legacy-reopened.docx");
    let stable = loaded.workspace.clone();
    let mut invalid = stable["document"].clone();
    invalid["content"][0]["attrs"]["numbering"]["level"] = json!(9);
    assert!(loaded
        .dispatch(json!({"op":"document","document":invalid}))
        .is_err());
    assert_eq!(loaded.workspace, stable);
}
#[test]
fn docx_numbering_native_nested_lists_keep_marks_and_avoid_source_id_collisions() {
    let mut c = fixture("numbering");
    let mut next = c.workspace["document"].clone();
    let list = json!({"type":"orderedList","attrs":{"start":3},"content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Native bold","marks":[{"type":"bold"},{"type":"link","attrs":{"href":"https://example.com/lists"}}]}]},{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Nested bullet","marks":[{"type":"italic"}]}]}]}]}]},{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Next native"}]}]}]});
    next["content"].as_array_mut().unwrap().push(list.clone());
    change(&mut c, next);
    let pkg = export(&mut c, "appended.docx");
    let re = docx::import(&pkg).unwrap().0;
    let p = &re["content"][10];
    assert_eq!(p["content"][0]["text"], "Native bold");
    assert_eq!(p["attrs"]["numbering"]["levels"][0]["start"], 3);
    assert!(
        p["attrs"]["numbering"]["numId"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > 9
    );
    assert_eq!(re["content"][11]["attrs"]["numbering"]["level"], 1);
    assert_eq!(
        re["content"][11]["attrs"]["numbering"]["levels"][1]["format"],
        "bullet"
    );
    assert_eq!(
        re["content"][12]["attrs"]["numbering"]["numId"],
        p["attrs"]["numbering"]["numId"]
    );
    let native = json!({"type":"doc","content":[list]});
    let bytes = docx::export(&native, None, None).unwrap();
    std::fs::write(output().join("native.docx"), &bytes).unwrap();
    let pkg = Package::from_bytes(bytes).unwrap();
    assert!(pkg.text("word/numbering.xml").is_ok());
    let rels = parse(pkg.text("word/_rels/document.xml.rels").unwrap()).unwrap();
    assert!(rels
        .descendants()
        .any(|n| attr(n, "Type").is_some_and(|s| s.ends_with("/numbering"))));
}
#[test]
fn docx_numbering_prefixed_properties_remain_well_formed_and_ordered() {
    let mut c = fixture("numbering-prefixed");
    let mut n = c.workspace["document"].clone();
    n["content"][0]["attrs"]["numbering"] = Value::Null;
    n["content"][0]["attrs"]["textAlign"] = json!("center");
    n["content"][7]["attrs"]["numbering"]["instance"] = json!("roman-restart");
    n["content"][7]["attrs"]["numbering"]["numId"] = Value::Null;
    n["content"][7]["attrs"]["numbering"]["levels"][0]["start"] = json!(6);
    change(&mut c, n);
    let pkg = export(&mut c, "prefixed.docx");
    let re = docx::import(&pkg).unwrap().0;
    assert!(re["content"][0]["attrs"]["numbering"].is_null());
    assert_eq!(
        re["content"][7]["attrs"]["numbering"]["levels"][0]["start"],
        6
    );
    let d = parse(pkg.text("word/document.xml").unwrap()).unwrap();
    let p = d
        .descendants()
        .find(|n| n.has_tag_name((docx_numbering::W, "pPr")))
        .unwrap();
    assert_eq!(
        p.children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect::<Vec<_>>(),
        ["keepNext", "numPr", "spacing", "jc"]
    );
}
