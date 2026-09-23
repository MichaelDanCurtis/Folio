use crate::{
    docx,
    office::{parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn fixture(name: &str) -> Core {
    let mut core = Core::new();
    core.dispatch(json!({"op":"importOffice","path":format!("{}/../test/fixtures/{name}.docx",env!("CARGO_MANIFEST_DIR"))})).unwrap();
    core
}
fn output() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-word-tabs-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = output().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn change(c: &mut Core, document: Value) {
    c.dispatch(json!({"op":"document","document":document,"baseRevision":c.workspace["revision"]}))
        .unwrap();
}
#[test]
fn tab_style_layers_direct_clears_and_nested_paragraphs_survive_import() {
    for name in ["word-tabs", "word-tabs-relocated"] {
        let c = fixture(name);
        let doc = &c.workspace["document"];
        assert_eq!(doc["attrs"]["defaultTabStop"], 600);
        let t = &doc["content"][0]["attrs"]["tabs"];
        assert_eq!(t["inherited"].as_array().unwrap().len(), 2);
        assert_eq!(t["inherited"][0]["position"], 720);
        assert_eq!(t["inherited"][1]["alignment"], "right");
        assert_eq!(t["direct"][0]["alignment"], "clear");
        assert_eq!(t["direct"][1]["leader"], "dot");
        assert_eq!(
            doc["content"][3]["content"][0]["content"][0]["content"][0]["attrs"]["tabs"]
                ["inherited"],
            t["inherited"]
        );
    }
}
#[test]
fn tabs_edit_default_export_reopen_reimport_and_exact_undo() {
    let mut c = fixture("word-tabs-relocated");
    let original_doc = c.workspace["document"].clone();
    let original = export(&mut c, "original.docx");
    let mut next = original_doc.clone();
    next["attrs"]["defaultTabStop"] = json!(900);
    next["content"][0]["attrs"]["tabs"]["direct"] = json!([
        {"position":720,"alignment":"clear","leader":"none"},
        {"position":1440,"alignment":"clear","leader":"none"},
        {"position":2880,"alignment":"right","leader":"hyphen"}]);
    next["content"][0]["content"][0]["text"] = json!("Revised");
    next["content"][0]["content"][2]["text"] = json!("24.50");
    change(&mut c, next);
    let edited = export(&mut c, "edited.docx");
    let imported = docx::import(&edited).unwrap().0;
    assert_eq!(imported["attrs"]["defaultTabStop"], 900);
    assert_eq!(
        imported["content"][0]["attrs"]["tabs"],
        c.workspace["document"]["content"][0]["attrs"]["tabs"]
    );
    for part in [
        "word/options/styles-custom.xml",
        "word/header1.xml",
        "word/_rels/document.xml.rels",
        "[Content_Types].xml",
    ] {
        assert_eq!(edited.entries[part], original.entries[part], "{part}");
    }
    let xml = edited.text("word/document.xml").unwrap();
    let d = parse(xml).unwrap();
    let p = d.descendants().find(|n| n.has_tag_name((W, "p"))).unwrap();
    assert!(p.descendants().any(|n| n.has_tag_name((W, "keepNext"))));
    assert!(p.descendants().any(|n| n.has_tag_name((W, "b"))));
    let path = output().join("tabs.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut reopened = Core::new();
    reopened.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(reopened.workspace["document"], c.workspace["document"]);
    let reopened_pkg = export(&mut reopened, "reopened.docx");
    assert_eq!(docx::import(&reopened_pkg).unwrap().0, imported);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], original_doc);
    assert_eq!(export(&mut c, "undone.docx").original, original.original);
    let mut round = Core::new();
    round
        .dispatch(json!({"op":"importOffice","path":output().join("edited.docx")}))
        .unwrap();
    assert_eq!(
        export(&mut round, "reimported.docx").original,
        edited.original
    );
}
#[test]
fn default_settings_survive_content_only_and_ai_edits_and_new_packages() {
    let mut c = Core::new();
    let mut document = c.workspace["document"].clone();
    document["attrs"]["defaultTabStop"] = json!(480);
    change(&mut c, document.clone());
    let package = export(&mut c, "new-default.docx");
    assert_eq!(
        docx::import(&package).unwrap().0["attrs"]["defaultTabStop"],
        480
    );
    assert!(package
        .text("word/_rels/document.xml.rels")
        .unwrap()
        .contains("/settings"));
    assert!(package
        .text("[Content_Types].xml")
        .unwrap()
        .contains("wordprocessingml.settings+xml"));
    document.as_object_mut().unwrap().remove("attrs");
    change(&mut c, document);
    assert_eq!(c.workspace["document"]["attrs"]["defaultTabStop"], 480);
    let p = c.dispatch(json!({"op":"propose","proposal":{"summary":"Replace prose","baseRevision":c.workspace["revision"],"replaceDocument":["New prose"]}})).unwrap();
    c.dispatch(json!({"op":"apply","proposal":p})).unwrap();
    assert_eq!(c.workspace["document"]["attrs"]["defaultTabStop"], 480);
    let before = c.workspace.clone();
    let mut bad = c.workspace["document"].clone();
    bad["attrs"]["defaultTabStop"] = json!(0);
    assert!(c.dispatch(json!({"op":"document","document":bad})).is_err());
    assert_eq!(c.workspace, before);
    assert!(c
        .dispatch(json!({"op":"document","document":before["document"],"baseRevision":0}))
        .is_err());
    assert_eq!(c.workspace, before);
}

#[test]
fn numbering_level_tabs_are_inherited_and_style_cycles_terminate() {
    let path = format!(
        "{}/../test/fixtures/word-tabs.docx",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut pkg = Package::read(&path).unwrap();
    let xml = pkg.text("word/document.xml").unwrap().replace(
        "<w:pStyle w:val=\"TabDerived\"/>",
        "<w:pStyle w:val=\"TabDerived\"/><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"99\"/></w:numPr>");
    pkg.set("word/document.xml", xml);
    pkg.set("word/numbering.xml", format!("<w:numbering xmlns:w=\"{W}\"><w:abstractNum w:abstractNumId=\"99\"><w:lvl w:ilvl=\"0\"><w:start w:val=\"1\"/><w:numFmt w:val=\"decimal\"/><w:lvlText w:val=\"%1.\"/><w:pPr><w:tabs><w:tab w:val=\"left\" w:pos=\"2880\"/></w:tabs></w:pPr></w:lvl></w:abstractNum><w:num w:numId=\"99\"><w:abstractNumId w:val=\"99\"/><w:lvlOverride w:ilvl=\"0\"><w:lvl w:ilvl=\"0\"><w:start w:val=\"1\"/><w:numFmt w:val=\"decimal\"/><w:lvlText w:val=\"%1.\"/><w:pPr><w:tabs><w:tab w:val=\"center\" w:pos=\"3600\"/></w:tabs></w:pPr></w:lvl></w:lvlOverride></w:num></w:numbering>"));
    let doc = docx::import(&pkg).unwrap().0;
    let inherited = doc["content"][0]["attrs"]["tabs"]["inherited"]
        .as_array()
        .unwrap();
    assert_eq!(
        inherited
            .iter()
            .map(|s| s["position"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        vec![720, 1440, 3600]
    );
    assert_eq!(inherited[2]["alignment"], "center");
    let styles = pkg.text("word/styles.xml").unwrap().replace(
        "<w:basedOn w:val=\"Normal\"/>",
        "<w:basedOn w:val=\"TabDerived\"/>",
    );
    pkg.set("word/styles.xml", styles);
    assert!(docx::import(&pkg).is_ok());
}

#[test]
fn reopening_legacy_workspaces_restores_settings_without_overwriting_explicit_tabs() {
    fn strip(n: &mut Value) {
        if let Some(attrs) = n.get_mut("attrs").and_then(Value::as_object_mut) {
            attrs.remove("tabs");
            attrs.remove("defaultTabStop");
        }
        for child in n
            .get_mut("content")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            strip(child);
        }
    }
    let mut c = fixture("word-tabs");
    let expected = c.workspace["document"].clone();
    let path = output().join("legacy-tabs.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    strip(&mut saved["document"]);
    strip(&mut saved["officeDocument"]["baseline"]);
    let explicit = json!({"inherited":[],"direct":[]});
    saved["document"]["content"][1]["attrs"]["tabs"] = explicit.clone();
    std::fs::write(&path, saved.to_string()).unwrap();
    let mut reopened = Core::new();
    reopened.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        reopened.workspace["document"]["attrs"]["defaultTabStop"],
        600
    );
    assert_eq!(
        reopened.workspace["document"]["content"][0]["attrs"]["tabs"],
        expected["content"][0]["attrs"]["tabs"]
    );
    assert_eq!(
        reopened.workspace["document"]["content"][1]["attrs"]["tabs"],
        explicit
    );
    assert_eq!(reopened.workspace["officeDocument"]["baseline"], expected);
}

#[test]
fn every_word_tab_leader_retains_a_single_character_through_save_and_export() {
    let leaders = ["none", "dot", "hyphen", "underscore", "heavy", "middleDot"];
    let mut c = Core::new();
    let document = json!({"type":"doc","content":leaders.iter().map(|leader|json!({
        "type":"paragraph","attrs":{"tabs":{"inherited":[],"direct":[{"position":4320,"alignment":"right","leader":leader}]}},
        "content":[{"type":"text","text":format!("{leader}\t123.45"),"marks":[{"type":"bold"}]}]
    })).collect::<Vec<_>>()});
    change(&mut c, document.clone());
    let pkg = export(&mut c, "all-leaders.docx");
    let xml = pkg.text("word/document.xml").unwrap();
    let d = parse(xml).unwrap();
    let body = d
        .descendants()
        .find(|n| n.has_tag_name((W, "body")))
        .unwrap();
    let paragraphs = body
        .children()
        .filter(|n| n.has_tag_name((W, "p")))
        .collect::<Vec<_>>();
    for (i, p) in paragraphs.into_iter().enumerate() {
        let field_tabs = p
            .descendants()
            .filter(|n| {
                n.has_tag_name((W, "tab")) && n.parent().is_some_and(|p| p.has_tag_name((W, "r")))
            })
            .count();
        assert_eq!(field_tabs, 1);
        assert!(p
            .descendants()
            .any(|n| n.has_tag_name((W, "tab")) && n.attribute((W, "leader")) == Some(leaders[i])));
    }
    let imported = docx::import(&pkg).unwrap().0;
    for (i, leader) in leaders.iter().enumerate() {
        assert_eq!(
            imported["content"][i]["attrs"]["tabs"]["direct"][0]["leader"],
            *leader
        );
    }
    let path = output().join("all-leaders.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut reopened = Core::new();
    reopened.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(reopened.workspace["document"], document);
    let restored = export(&mut reopened, "all-leaders-reopened.docx");
    assert_eq!(docx::import(&restored).unwrap().0, imported);
    let mut reimported = Core::new();
    reimported
        .dispatch(json!({"op":"importOffice","path":output().join("all-leaders.docx")}))
        .unwrap();
    assert_eq!(
        export(&mut reimported, "all-leaders-reimported.docx").original,
        pkg.original
    );
}
