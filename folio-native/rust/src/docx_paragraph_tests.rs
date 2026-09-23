use crate::{
    docx,
    office::{parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn source() -> String {
    format!(
        "{}/../test/fixtures/word-paragraphs.docx",
        env!("CARGO_MANIFEST_DIR")
    )
}
fn fixture() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":source()}))
        .unwrap();
    c
}
fn output(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-paragraph-tests");
    std::fs::create_dir_all(&p).unwrap();
    p.join(name)
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = output(name);
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn change(c: &mut Core, document: Value) {
    c.dispatch(json!({"op":"document","document":document,"baseRevision":c.workspace["revision"]}))
        .unwrap();
}
#[test]
fn paragraph_inheritance_direct_units_and_nested_cells() {
    let c = fixture();
    let d = &c.workspace["document"];
    let p = &d["content"][0]["attrs"]["paragraphFormat"];
    assert_eq!(
        *p,
        json!({"styleId":"ParagraphDerived","inheritedRun":p["inheritedRun"],"direct":{"left":720,"after":240,"keepNext":true},
        "inherited":{"before":60,"after":120,"line":360,"lineRule":"auto","left":360,"right":180}})
    );
    assert_eq!(
        d["content"][1]["attrs"]["paragraphFormat"]["inherited"],
        p["inherited"]
    );
    assert_eq!(
        d["content"][2]["attrs"]["paragraphFormat"]["direct"],
        json!({"hanging":240,"line":400,"lineRule":"atLeast"})
    );
    let cell =
        &d["content"][3]["content"][0]["content"][0]["content"][0]["attrs"]["paragraphFormat"];
    assert_eq!(cell["direct"]["before"], 160);
    assert_eq!(cell["inherited"], p["inherited"]);
}
#[test]
fn paragraph_edit_roundtrip_undo_and_unknown_properties() {
    let mut c = fixture();
    let original = c.workspace["document"].clone();
    let source_pkg = Package::read(&source()).unwrap();
    assert_eq!(
        export(&mut c, "original.docx").original,
        source_pkg.original
    );
    let mut next = original.clone();
    next["content"][0]["attrs"]["paragraphFormat"]["direct"] = json!({"left":1080,"right":360,"before":180,"after":300,"line":480,"lineRule":"exact","keepNext":true});
    change(&mut c, next.clone());
    let edited = export(&mut c, "edited.docx");
    let imported = docx::import(&edited).unwrap().0;
    assert_eq!(imported, next);
    for (name, data) in &source_pkg.entries {
        if name != "word/document.xml" {
            assert_eq!(&edited.entries[name], data, "{name}");
        }
    }
    let xml = edited.text("word/document.xml").unwrap();
    let parsed = parse(xml).unwrap();
    let pr = parsed
        .descendants()
        .find(|n| n.has_tag_name((W, "pPr")))
        .unwrap();
    let names: Vec<_> = pr
        .children()
        .filter(|n| n.is_element())
        .map(|n| n.tag_name().name())
        .collect();
    assert_eq!(names, vec!["pStyle", "keepNext", "tabs", "spacing", "ind"]);
    for (tag, val) in [("ind", "indent"), ("spacing", "spacing")] {
        assert_eq!(
            pr.children()
                .find(|n| n.has_tag_name((W, tag)))
                .unwrap()
                .attribute(("urn:folio:paragraph-test", "keep")),
            Some(val)
        );
    }
    let path = output("edited.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut reopened = Core::new();
    reopened.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(reopened.workspace["document"], next);
    assert_eq!(
        docx::import(&export(&mut reopened, "reopened.docx"))
            .unwrap()
            .0,
        next
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], original);
    assert_eq!(export(&mut c, "undone.docx").original, source_pkg.original);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["document"], next);
    let mut round = Core::new();
    round
        .dispatch(json!({"op":"importOffice","path":output("edited.docx")}))
        .unwrap();
    assert_eq!(
        export(&mut round, "reimported.docx").original,
        edited.original
    );
}
#[test]
fn paragraph_clear_direct_and_inserted_style_survive_export() {
    let mut c = fixture();
    let mut d = c.workspace["document"].clone();
    let p = &mut d["content"][0];
    p["attrs"]["paragraphFormat"]["direct"] = json!({});
    change(&mut c, d);
    let pkg = export(&mut c, "cleared.docx");
    assert_eq!(
        docx::import(&pkg).unwrap().0["content"][0]["attrs"]["paragraphFormat"]["direct"],
        json!({})
    );
    let mut c = fixture();
    let mut d = c.workspace["document"].clone();
    let mut inserted = d["content"][0].clone();
    inserted["attrs"]
        .as_object_mut()
        .unwrap()
        .remove("officeId");
    inserted["content"] = json!([{"type":"text","text":"Inserted paragraph"}]);
    d["content"]
        .as_array_mut()
        .unwrap()
        .insert(1, inserted.clone());
    change(&mut c, d);
    let imported = docx::import(&export(&mut c, "inserted.docx")).unwrap().0;
    assert_eq!(
        imported["content"][1]["attrs"]["paragraphFormat"],
        inserted["attrs"]["paragraphFormat"]
    );
    // A newly-created heading must not be overridden by the inherited body style.
    let mut d = c.workspace["document"].clone();
    d["content"][1]["type"] = json!("heading");
    d["content"][1]["attrs"]["level"] = json!(2);
    change(&mut c, d);
    let imported = docx::import(&export(&mut c, "heading.docx")).unwrap().0;
    assert_eq!(imported["content"][1]["type"], "heading");
    assert_eq!(imported["content"][1]["attrs"]["level"], 2);
}
#[test]
fn paragraph_validation_rejects_invalid_edits_without_mutation() {
    let mut c = fixture();
    let original = c.workspace.clone();
    for (key, value) in [
        ("line", json!(-1)),
        ("left", json!(i64::MIN)),
        ("before", json!(1.2)),
        ("lineRule", json!("random")),
        ("fakeAutospacing", json!(true)),
        ("snapToGrid", json!(1)),
    ] {
        let mut d = original["document"].clone();
        d["content"][0]["attrs"]["paragraphFormat"]["direct"][key] = value;
        assert!(
            c.dispatch(json!({"op":"document","document":d})).is_err(),
            "{key}"
        );
        assert_eq!(c.workspace, original);
    }
}
#[test]
fn paragraph_legacy_hydration_preserves_explicit_overrides() {
    fn strip(n: &mut Value) {
        if let Some(a) = n.get_mut("attrs").and_then(Value::as_object_mut) {
            a.remove("paragraphFormat");
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
    let mut c = fixture();
    let expected = c.workspace["document"].clone();
    let path = output("legacy.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    strip(&mut saved["document"]);
    strip(&mut saved["officeDocument"]["baseline"]);
    let explicit = json!({"inherited":{},"direct":{"left":40}});
    saved["document"]["content"][0]["attrs"]["paragraphFormat"] = explicit.clone();
    std::fs::write(&path, saved.to_string()).unwrap();
    let mut opened = Core::new();
    opened.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        opened.workspace["document"]["content"][0]["attrs"]["paragraphFormat"],
        explicit
    );
    assert_eq!(
        opened.workspace["document"]["content"][3],
        expected["content"][3]
    );
    assert_eq!(opened.workspace["officeDocument"]["baseline"], expected);
}

#[test]
fn paragraph_numbering_defaults_style_cycles_and_native_direct_format() {
    let mut pkg = Package::read(&source()).unwrap();
    let xml = pkg.text("word/document.xml").unwrap().replace(
        "<wx:pStyle wx:val=\"ParagraphDerived\"/>",
        "<wx:pStyle wx:val=\"ParagraphDerived\"/><wx:numPr><wx:ilvl wx:val=\"0\"/><wx:numId wx:val=\"99\"/></wx:numPr>");
    pkg.set("word/document.xml", xml);
    pkg.set("word/styles.xml",format!("<w:styles xmlns:w=\"{W}\"><w:docDefaults><w:pPrDefault><w:pPr><w:spacing w:before=\"80\" w:after=\"100\"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type=\"paragraph\" w:styleId=\"ParagraphDerived\"><w:basedOn w:val=\"Cycle\"/><w:pPr><w:ind w:right=\"180\"/></w:pPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Cycle\"><w:basedOn w:val=\"ParagraphDerived\"/><w:pPr><w:spacing w:after=\"160\"/></w:pPr></w:style></w:styles>"));
    pkg.set("word/numbering.xml",format!("<w:numbering xmlns:w=\"{W}\"><w:abstractNum w:abstractNumId=\"99\"><w:lvl w:ilvl=\"0\"><w:start w:val=\"1\"/><w:numFmt w:val=\"decimal\"/><w:lvlText w:val=\"%1.\"/><w:pPr><w:ind w:left=\"900\"/><w:spacing w:before=\"140\" w:after=\"200\"/></w:pPr></w:lvl></w:abstractNum><w:num w:numId=\"99\"><w:abstractNumId w:val=\"99\"/></w:num></w:numbering>"));
    let imported = docx::import(&pkg).unwrap().0;
    assert_eq!(
        imported["content"][0]["attrs"]["paragraphFormat"]["inherited"],
        json!({"before":140,"after":160,"line":240,"lineRule":"auto","left":900,"right":180})
    );
    let mut c = Core::new();
    let direct = json!({"left":720,"before":0,"after":180,"line":336,"lineRule":"auto"});
    let d = json!({"type":"doc","content":[{"type":"paragraph","attrs":{"paragraphFormat":{"inherited":{},"direct":direct}},"content":[{"type":"text","text":"Native paragraph"}]}]});
    change(&mut c, d);
    let imported = docx::import(&export(&mut c, "native.docx")).unwrap().0;
    assert_eq!(
        imported["content"][0]["attrs"]["paragraphFormat"]["direct"],
        direct
    );
}
