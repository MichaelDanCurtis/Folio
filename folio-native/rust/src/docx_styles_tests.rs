use crate::{
    docx, docx_styles as styles,
    office::{parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn output(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join("folio-style-tests");
    std::fs::create_dir_all(&d).unwrap();
    d.join(name)
}
fn package() -> Package {
    let mut p =
        Package::from_bytes(include_bytes!("../../test/fixtures/word-paragraphs.docx").to_vec())
            .unwrap();
    // A relationship-resolved alternate-prefix part with unsupported properties,
    // foreign attributes, a table style and character-style toggle inheritance.
    let source = p.text("word/styles.xml").unwrap().to_string();
    let extra = r#"<wx:style wx:type="paragraph" wx:styleId="StyleBase" keep:flag="style"><wx:name wx:val="Body base"/><wx:basedOn wx:val="Normal"/><wx:qFormat/><wx:pPr keep:flag="paragraph"><wx:keepNext/><wx:spacing wx:after="180" keep:flag="spacing"/><wx:ind wx:left="120"/><wx:jc wx:val="center"/></wx:pPr><wx:rPr keep:flag="run"><wx:rFonts wx:ascii="Georgia" wx:hAnsi="Georgia" keep:flag="fonts"/><wx:b/><wx:color wx:val="314159"/><wx:sz wx:val="28"/><wx:lang wx:val="en-NZ"/></wx:rPr><keep:payload>retain me</keep:payload></wx:style><wx:style wx:type="paragraph" wx:styleId="StyleChild"><wx:name wx:val="Body child"/><wx:basedOn wx:val="StyleBase"/><wx:next wx:val="Normal"/><wx:pPr><wx:spacing wx:before="120"/></wx:pPr><wx:rPr><wx:b/><wx:i/></wx:rPr></wx:style><wx:style wx:type="character" wx:styleId="CharBase"><wx:name wx:val="Character base"/><wx:rPr><wx:b/></wx:rPr></wx:style><wx:style wx:type="character" wx:styleId="CharChild"><wx:name wx:val="Character child"/><wx:basedOn wx:val="CharBase"/><wx:rPr><wx:b wx:val="0"/></wx:rPr></wx:style>"#;
    let extra = extra.replace("<wx:style ", "<wx:style wx:customStyle=\"1\" ");
    let source = source
        .replace("<w:", "<wx:")
        .replace("</w:", "</wx:")
        .replace(" w:", " wx:")
        .replace("xmlns:w=", "xmlns:wx=");
    let source = source
        .replace(
            "<wx:styles ",
            "<wx:styles xmlns:keep=\"urn:folio:styles-test\" ",
        )
        .replace("</wx:styles>", &format!("{extra}</wx:styles>"));
    p.set("word/styles/named.xml", source);
    p.entries.remove("word/styles.xml");
    p.set(
        "word/_rels/document.xml.rels",
        p.text("word/_rels/document.xml.rels")
            .unwrap()
            .replace("Target=\"styles.xml\"", "Target=\"styles/named.xml\""),
    );
    p.set(
        "[Content_Types].xml",
        p.text("[Content_Types].xml")
            .unwrap()
            .replace("/word/styles.xml", "/word/styles/named.xml"),
    );
    Package::from_bytes(p.bytes().unwrap()).unwrap()
}
fn setup(name: &str) -> Core {
    let p = package();
    let file = output(name);
    std::fs::write(&file, &p.original).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":file}))
        .unwrap();
    c
}
fn catalog(c: &Core) -> Value {
    c.workspace["document"]["attrs"][styles::KEY].clone()
}
fn find(c: &Value, id: &str) -> Value {
    c["styles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
        .unwrap()
        .clone()
}
fn command(c: &mut Core, mut r: Value) -> Result<Value, String> {
    r["op"] = json!("documentStyle");
    r["baseRevision"] = c.workspace["revision"].clone();
    r["workspaceId"] = c.workspace["id"].clone();
    c.dispatch(r)
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = output(name);
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
#[test]
fn named_styles_resolve_inheritance_toggles_fonts_and_relationship_paths() {
    let p = package();
    let c = styles::read(&p).unwrap();
    styles::validate(&c).unwrap();
    let child = styles::resolve(&c, Some("StyleChild"));
    assert_eq!(child["paragraph"]["after"], 180);
    assert_eq!(child["paragraph"]["before"], 120);
    assert_eq!(child["alignment"], "center");
    assert_eq!(child["run"]["bold"], false);
    assert_eq!(child["run"]["italic"], true);
    assert_eq!(child["run"]["rFonts"]["ascii"], "Georgia");
    assert_eq!(child["run"]["sz"], 28);
    assert_eq!(
        styles::resolve(&c, Some("CharChild"))["run"]["bold"],
        true,
        "false in a style must not cancel inherited bold"
    );
    let mut bad = find(&c, "StyleBase");
    bad["basedOn"] = json!("CharBase");
    let mut imported = c.clone();
    let i = imported["styles"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["id"] == "StyleBase")
        .unwrap();
    imported["styles"][i] = bad;
    assert_eq!(
        styles::resolve(&imported, Some("StyleChild"))["run"]["bold"],
        false,
        "cross-type base ignored"
    );
    let (doc, _) = docx::import(&p).unwrap();
    assert_eq!(
        docx::export(&doc, Some(&p), Some(&doc)).unwrap(),
        p.original
    );
}
#[test]
fn style_apply_edit_undo_recovery_and_source_preservation() {
    let mut c = setup("source-edit.docx");
    let original = c.workspace["document"].clone();
    command(
        &mut c,
        json!({"action":"apply","styleId":"StyleChild","paths":[[0],[1],[3,0,0,0]]}),
    )
    .unwrap();
    let applied = c.workspace["document"].clone();
    assert_eq!(
        applied["content"][0]["attrs"]["paragraphFormat"]["direct"],
        original["content"][0]["attrs"]["paragraphFormat"]["direct"]
    );
    assert_eq!(
        applied["content"][0]["attrs"]["paragraphFormat"]["inherited"]["after"],
        180
    );
    let mut base = find(&catalog(&c), "StyleBase");
    base["name"] = json!("Body base renamed");
    base["properties"]["paragraph"]["after"] = json!(360);
    base["properties"]["paragraph"]["right"] = json!(90);
    base["properties"]["run"]["sz"] = json!(32);
    base["properties"]["run"]["rFonts"]["ascii"] = json!("Arial");
    base["properties"]["alignment"] = json!("right");
    command(&mut c, json!({"action":"upsert","style":base})).unwrap();
    let edited = c.workspace["document"].clone();
    for p in [
        &edited["content"][0],
        &edited["content"][1],
        &edited["content"][3]["content"][0]["content"][0]["content"][0],
    ] {
        assert_eq!(p["attrs"]["paragraphFormat"]["inherited"]["after"], 360);
        assert_eq!(p["attrs"]["paragraphFormat"]["inheritedRun"]["sz"], 32);
        assert_eq!(p["attrs"]["paragraphFormat"]["inheritedRun"]["bold"], false);
    }
    let out = export(&mut c, "edited.docx");
    let imported = docx::import(&out).unwrap().0;
    assert_eq!(imported, edited);
    let xml = out.text("word/styles/named.xml").unwrap();
    let d = parse(xml).unwrap();
    let n = d
        .descendants()
        .find(|n| n.attribute((W, "styleId")) == Some("StyleBase"))
        .unwrap();
    assert_eq!(
        n.attribute(("urn:folio:styles-test", "flag")),
        Some("style")
    );
    for (tag, flag) in [
        ("pPr", "paragraph"),
        ("spacing", "spacing"),
        ("rPr", "run"),
        ("rFonts", "fonts"),
    ] {
        assert_eq!(
            n.descendants()
                .find(|n| n.has_tag_name((W, tag)))
                .unwrap()
                .attribute(("urn:folio:styles-test", "flag")),
            Some(flag),
            "{tag}"
        );
    }
    assert!(xml.contains("retain me"));
    assert!(n
        .descendants()
        .any(|n| n.has_tag_name((W, "lang")) && n.attribute((W, "val")) == Some("en-NZ")));
    let source = package();
    for (name, b) in &source.entries {
        if name != "word/document.xml" && name != "word/styles/named.xml" {
            assert_eq!(&out.entries[name], b, "{name}");
        }
    }
    let srcdoc = parse(source.text("word/document.xml").unwrap()).unwrap();
    let destdoc = parse(out.text("word/document.xml").unwrap()).unwrap();
    let runs = |d: &roxmltree::Document| {
        d.descendants()
            .filter(|n| n.has_tag_name((W, "r")))
            .map(|n| d.input_text()[n.range()].to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        runs(&srcdoc),
        runs(&destdoc),
        "style edits preserve full run XML"
    );
    let path = output("edited.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut re = Core::new();
    re.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(re.workspace["document"], edited);
    assert_eq!(
        docx::import(&export(&mut re, "recovered.docx")).unwrap().0,
        edited
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], applied);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["document"], edited);
    c.dispatch(json!({"op":"undo"})).unwrap();
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], original);
    assert_eq!(export(&mut c, "undone.docx").original, source.original);
}
#[test]
fn style_creation_custom_heading_and_deletion_roundtrip() {
    let mut c = setup("source-create.docx");
    let mut s = find(&catalog(&c), "StyleChild");
    s["id"] = json!("ProjectHeading");
    s["name"] = json!("Project heading");
    s["properties"]["outlineLevel"] = json!(1);
    command(&mut c, json!({"action":"upsert","style":s})).unwrap();
    command(
        &mut c,
        json!({"action":"apply","styleId":"ProjectHeading","paths":[[1]]}),
    )
    .unwrap();
    assert_eq!(c.workspace["document"]["content"][1]["type"], "heading");
    assert_eq!(c.workspace["document"]["content"][1]["attrs"]["level"], 2);
    let edited = c.workspace["document"].clone();
    assert_eq!(
        docx::import(&export(&mut c, "created.docx")).unwrap().0,
        edited
    );
    assert!(command(
        &mut c,
        json!({"action":"delete","styleId":"ProjectHeading"})
    )
    .is_err());
    command(
        &mut c,
        json!({"action":"apply","styleId":"Normal","paths":[[1]]}),
    )
    .unwrap();
    command(
        &mut c,
        json!({"action":"delete","styleId":"ProjectHeading"}),
    )
    .unwrap();
    let out = export(&mut c, "deleted.docx");
    assert!(!out
        .text("word/styles/named.xml")
        .unwrap()
        .contains("ProjectHeading"));
}
#[test]
fn invalid_style_commands_are_atomic_and_imported_cycles_terminate() {
    let mut c = setup("source-invalid.docx");
    let original = c.workspace.clone();
    let mut s = find(&catalog(&c), "StyleBase");
    for target in ["StyleBase", "StyleChild", "CharBase", "Missing"] {
        s["basedOn"] = json!(target);
        assert!(command(&mut c, json!({"action":"upsert","style":s})).is_err());
        assert_eq!(c.workspace, original);
    }
    for r in [
        json!({"action":"delete","styleId":"Normal"}),
        json!({"action":"delete","styleId":"StyleBase"}),
        json!({"action":"apply","styleId":"CharBase","paths":[[0]]}),
        json!({"action":"apply","styleId":"StyleChild","paths":[[0],[999]]}),
        json!({"action":"apply","styleId":"StyleChild","paths":[[0],[0]]}),
    ] {
        assert!(command(&mut c, r).is_err());
        assert_eq!(c.workspace, original);
    }
    for (rev, id) in [
        (json!(0), original["id"].clone()),
        (original["revision"].clone(), json!("other-workspace")),
    ] {
        assert!(c.dispatch(json!({"op":"documentStyle","action":"apply","styleId":"StyleChild","paths":[[0]],"baseRevision":rev,"workspaceId":id})).is_err());
        assert_eq!(c.workspace, original);
    }
    let mut catalog = catalog(&c);
    let i = catalog["styles"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["id"] == "StyleBase")
        .unwrap();
    catalog["styles"][i]["basedOn"] = json!("StyleChild");
    assert!(styles::resolve(&catalog, Some("StyleChild")).is_object());
}

#[test]
fn native_styles_apply_custom_heading_and_noops_without_history_changes() {
    let mut c = Core::new();
    let available = c.dispatch(json!({"op":"documentStyles"})).unwrap();
    let mut s = find(&available["catalog"], "Heading2");
    s["id"] = json!("NativeHeading");
    s["name"] = json!("Native heading");
    command(&mut c, json!({"action":"upsert","style":s})).unwrap();
    command(
        &mut c,
        json!({"action":"apply","styleId":"NativeHeading","paths":[[0]]}),
    )
    .unwrap();
    let out = export(&mut c, "native-heading.docx");
    let doc = docx::import(&out).unwrap().0;
    assert_eq!(
        doc["content"][0]["attrs"]["paragraphFormat"]["styleId"],
        "NativeHeading"
    );
    assert_eq!(doc["content"][0]["attrs"]["level"], 2);
    let same = c.workspace.clone();
    s = find(&catalog(&c), "NativeHeading");
    command(&mut c, json!({"action":"upsert","style":s})).unwrap();
    assert_eq!(c.workspace, same);
    command(
        &mut c,
        json!({"action":"apply","styleId":"NativeHeading","paths":[[0]]}),
    )
    .unwrap();
    assert_eq!(c.workspace, same);
    s["properties"]["outlineLevel"] = json!(2);
    command(&mut c, json!({"action":"upsert","style":s})).unwrap();
    assert_eq!(c.workspace["document"]["content"][0]["attrs"]["level"], 3);
    assert_eq!(
        docx::import(&export(&mut c, "native-heading-updated.docx"))
            .unwrap()
            .0["content"][0]["attrs"]["paragraphFormat"]["styleId"],
        "NativeHeading"
    );
}
#[test]
fn absent_style_parts_are_created_with_relationships_and_content_types() {
    let mut p = docx::new_package();
    p.entries.remove("word/styles.xml");
    p.set(
        "word/_rels/document.xml.rels",
        "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>"
            .into(),
    );
    p.set("[Content_Types].xml",p.text("[Content_Types].xml").unwrap().replace("<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>",""));
    let p = Package::from_bytes(p.bytes().unwrap()).unwrap();
    let original = docx::import(&p).unwrap().0;
    let mut changed = p.clone();
    let doc=styles::command(&original,&json!({"action":"upsert","style":{"id":"FirstStyle","name":"First style","type":"paragraph","properties":{"paragraph":{"after":200},"tabs":[],"run":{"bold":true}}}}),&mut changed,Some(&original)).unwrap();
    let out = Package::from_bytes(docx::export(&doc, Some(&p), Some(&original)).unwrap()).unwrap();
    assert_eq!(
        find(&styles::read(&out).unwrap(), "FirstStyle")["properties"]["paragraph"]["after"],
        200
    );
    assert!(out
        .text("word/_rels/document.xml.rels")
        .unwrap()
        .contains("/relationships/styles"));
    assert!(out
        .text("[Content_Types].xml")
        .unwrap()
        .contains("wordprocessingml.styles+xml"));
    std::fs::write(output("created-style-part.docx"), out.original).unwrap();
}
#[test]
fn legacy_styles_hydrate_missing_catalog_and_metadata_without_overwriting_edits() {
    let mut c = setup("source-legacy.docx");
    let expected = c.workspace["document"].clone();
    let path = output("legacy.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut old: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    fn strip(n: &mut Value) {
        if let Some(a) = n.get_mut("attrs").and_then(Value::as_object_mut) {
            a.remove(styles::KEY);
            if let Some(p) = a.get_mut("paragraphFormat").and_then(Value::as_object_mut) {
                p.remove("inheritedRun");
                p.remove("inheritedAlignment");
                p.remove("inheritedOutlineLevel");
            }
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
    strip(&mut old["document"]);
    strip(&mut old["officeDocument"]["baseline"]);
    std::fs::write(&path, old.to_string()).unwrap();
    let mut re = Core::new();
    re.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(re.workspace["document"], expected);
    assert_eq!(
        export(&mut re, "legacy-recovered.docx").original,
        package().original
    );
    old["document"]["content"][0]["attrs"]["paragraphFormat"]["styleId"] = json!("StyleChild");
    old["document"]["content"][0]["attrs"]["paragraphFormat"]["inheritedRun"] = json!({"sz":40});
    std::fs::write(&path, old.to_string()).unwrap();
    re.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        re.workspace["document"]["content"][0]["attrs"]["paragraphFormat"]["inheritedRun"],
        json!({"sz":40})
    );
}

#[test]
fn imported_custom_heading_keeps_its_style_when_outline_level_changes() {
    let mut c = setup("source-outline.docx");
    let mut s = find(&catalog(&c), "StyleChild");
    s["properties"]["outlineLevel"] = json!(1);
    command(&mut c, json!({"action":"upsert","style":s})).unwrap();
    command(
        &mut c,
        json!({"action":"apply","styleId":"StyleChild","paths":[[1]]}),
    )
    .unwrap();
    export(&mut c, "outline-before.docx");
    let mut imported = Core::new();
    imported
        .dispatch(json!({"op":"importOffice","path":output("outline-before.docx")}))
        .unwrap();
    s["properties"]["outlineLevel"] = json!(2);
    command(&mut imported, json!({"action":"upsert","style":s})).unwrap();
    let result = docx::import(&export(&mut imported, "outline-after.docx"))
        .unwrap()
        .0;
    assert_eq!(
        result["content"][1]["attrs"]["paragraphFormat"]["styleId"],
        "StyleChild"
    );
    assert_eq!(result["content"][1]["attrs"]["level"], 3);
    assert_eq!(result, imported.workspace["document"]);
    s["properties"]
        .as_object_mut()
        .unwrap()
        .remove("outlineLevel");
    command(&mut imported, json!({"action":"upsert","style":s})).unwrap();
    let body = docx::import(&export(&mut imported, "outline-removed.docx"))
        .unwrap()
        .0;
    assert_eq!(body["content"][1]["type"], "paragraph");
    assert_eq!(
        body["content"][1]["attrs"]["paragraphFormat"]["styleId"],
        "StyleChild"
    );
}
#[test]
fn style_tab_clears_and_removed_declarations_recompute_dependents() {
    let mut c = setup("source-tabs.docx");
    let mut base = find(&catalog(&c), "StyleBase");
    base["properties"]["tabs"] = json!([{"position":1440,"alignment":"center","leader":"dot"}]);
    command(&mut c, json!({"action":"upsert","style":base})).unwrap();
    let mut child = find(&catalog(&c), "StyleChild");
    child["properties"]["tabs"] = json!([{"position":1440,"alignment":"clear","leader":"none"},{"position":2880,"alignment":"right","leader":"hyphen"}]);
    command(&mut c, json!({"action":"upsert","style":child})).unwrap();
    command(
        &mut c,
        json!({"action":"apply","styleId":"StyleChild","paths":[[0],[1]]}),
    )
    .unwrap();
    assert_eq!(
        c.workspace["document"]["content"][0]["attrs"]["tabs"]["inherited"],
        json!([{"position":2880,"alignment":"right","leader":"hyphen"}])
    );
    assert_eq!(
        c.workspace["document"]["content"][0]["attrs"]["tabs"]["direct"][0]["position"],
        1440
    );
    base["properties"]["run"]
        .as_object_mut()
        .unwrap()
        .remove("sz");
    base["properties"]["paragraph"]
        .as_object_mut()
        .unwrap()
        .remove("after");
    command(&mut c, json!({"action":"upsert","style":base})).unwrap();
    assert_eq!(
        c.workspace["document"]["content"][1]["attrs"]["paragraphFormat"]["inheritedRun"]["sz"],
        22
    );
    assert_eq!(
        c.workspace["document"]["content"][1]["attrs"]["paragraphFormat"]["inherited"]["after"],
        120
    );
    assert_eq!(
        docx::import(&export(&mut c, "tab-clears.docx")).unwrap().0,
        c.workspace["document"]
    );
}

#[test]
fn style_font_edits_keep_leaf_local_namespaces_and_unknown_xml() {
    let mut pkg = package();
    let path = "word/styles/named.xml";
    let xml=pkg.text(path).unwrap().replace("keep:flag=\"fonts\"/>","keep:flag=\"fonts\" xmlns:leaf = \"urn:folio:leaf\" leaf:value=\"retained\" wx:future=\"retained\"><leaf:child/></wx:rFonts>");
    pkg.set(path, xml);
    let before = styles::read(&pkg).unwrap();
    let mut edited = find(&before, "StyleBase");
    edited["properties"]["run"]["rFonts"]["ascii"] = json!("Arial");
    let catalog = styles::edit(&before, &json!({"action":"upsert","style":edited})).unwrap();
    styles::write(&json!({"attrs":{styles::KEY:catalog}}), &mut pkg).unwrap();
    let d = parse(pkg.text(path).unwrap()).unwrap();
    let font = d
        .descendants()
        .find(|n| n.attribute(("urn:folio:leaf", "value")) == Some("retained"))
        .unwrap();
    assert_eq!(font.attribute((W, "ascii")), Some("Arial"));
    assert_eq!(font.attribute((W, "future")), Some("retained"));
    assert!(font
        .children()
        .any(|n| n.has_tag_name(("urn:folio:leaf", "child"))));
}

#[test]
fn ordinary_document_and_ai_edits_cannot_bypass_style_transactions() {
    let mut c = setup("source-bypass.docx");
    let original = c.workspace.clone();
    let mut d = original["document"].clone();
    d["attrs"][styles::KEY]["styles"][0]["name"] = json!("Changed outside styles");
    assert!(c.dispatch(json!({"op":"document","document":d})).is_err());
    assert_eq!(c.workspace, original);
    assert!(c.dispatch(json!({"op":"apply","proposal":{"baseRevision":original["revision"],"summary":"Change style","document":d}})).is_err());
    assert_eq!(c.workspace, original);
    let queried = c.dispatch(json!({"op":"documentStyles"})).unwrap();
    assert_eq!(queried["catalog"], catalog(&c));
    assert_eq!(c.workspace, original);
}

#[test]
fn explicit_off_run_marks_survive_text_edit_and_word_export() {
    let mut pkg = docx::new_package();
    let xml=format!("<w:document xmlns:w=\"{W}\"><w:body><w:p><w:r><w:rPr><w:b w:val=\"0\"/><w:i w:val=\"false\"/><w:u w:val=\"none\"/><w:strike w:val=\"off\"/></w:rPr><w:t>Plain override</w:t></w:r></w:p></w:body></w:document>");
    pkg.set("word/document.xml", xml);
    let pkg = Package::from_bytes(pkg.bytes().unwrap()).unwrap();
    let original = docx::import(&pkg).unwrap().0;
    let marks = &original["content"][0]["content"][0]["marks"];
    assert_eq!(
        *marks,
        json!([{"type":"bold","attrs":{"value":false}},{"type":"italic","attrs":{"value":false}},{"type":"underline","attrs":{"value":false}},{"type":"strike","attrs":{"value":false}}])
    );
    let mut edited = original.clone();
    edited["content"][0]["content"][0]["text"] = json!("Edited plain override");
    let out =
        Package::from_bytes(docx::export(&edited, Some(&pkg), Some(&original)).unwrap()).unwrap();
    assert_eq!(docx::import(&out).unwrap().0, edited);
    std::fs::write(output("explicit-off.docx"), out.original).unwrap();
}
#[test]
fn theme_fonts_colors_and_later_literal_declarations_resolve_in_order() {
    let pkg = package();
    let catalog = styles::read(&pkg).unwrap();
    assert_eq!(catalog["theme"]["fonts"]["minorFont"]["latin"], "Cambria");
    let normal = styles::resolve(&catalog, Some("Normal"));
    assert_eq!(normal["run"]["rFonts"]["ascii"], "Cambria");
    let child = styles::resolve(&catalog, Some("StyleChild"));
    assert_eq!(child["run"]["rFonts"]["ascii"], "Georgia");
    assert!(child["run"]["rFonts"]["asciiTheme"].is_null());
    let mut s = find(&catalog, "StyleChild");
    s["properties"]["run"]["rFonts"] =
        json!({"ascii":"Ignored within this element","asciiTheme":"majorHAnsi"});
    s["properties"]["run"]["color"] = json!({"val":"000000","themeColor":"accent1"});
    let updated = styles::edit(&catalog, &json!({"action":"upsert","style":s})).unwrap();
    let resolved = styles::resolve(&updated, Some("StyleChild"));
    assert_eq!(
        resolved["run"]["rFonts"]["ascii"],
        catalog["theme"]["fonts"]["majorFont"]["latin"]
    );
    assert_eq!(
        resolved["run"]["color"]["val"],
        catalog["theme"]["colors"]["accent1"]
    );
    s["properties"]["run"]["color"] = json!({"val":"112233"});
    let updated = styles::edit(&catalog, &json!({"action":"upsert","style":s})).unwrap();
    assert_eq!(
        styles::resolve(&updated, Some("StyleChild"))["run"]["color"],
        json!({"val":"112233"})
    );
}
#[test]
fn cell_style_preview_keeps_workspace_and_history_unchanged_and_rejects_stale_drafts() {
    let mut c = setup("source-preview.docx");
    let before = c.workspace.clone();
    let request = json!({"op":"documentStylePreview","action":"apply","workspaceId":before["id"],"baseRevision":before["revision"],"document":{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Unsaved cell draft"}]}]},"styleId":"StyleBase","paths":[[0]]});
    let draft = c.dispatch(request.clone()).unwrap();
    assert_eq!(c.workspace, before);
    assert_eq!(
        draft["document"]["content"][0]["attrs"]["paragraphFormat"]["styleId"],
        "StyleBase"
    );
    assert_eq!(
        draft["document"]["content"][0]["content"][0]["text"],
        "Unsaved cell draft"
    );
    c.dispatch(json!({"op":"title","title":"Another revision"}))
        .unwrap();
    let current = c.workspace.clone();
    assert!(c.dispatch(request).is_err());
    assert_eq!(c.workspace, current);
}

#[test]
fn replacing_used_style_updates_body_dependents_and_preserved_parts_with_history() {
    let mut p = package();
    p.set("word/header-style-test.xml", format!(r#"<x:hdr xmlns:x="{W}" xmlns:k="urn:keep"><x:p k:flag="keep"><x:pPr><x:pStyle k:val="StyleBase" x:val='StyleBase'/></x:pPr><x:r><x:rPr><x:color x:val="ABCDEF"/></x:rPr><x:t>StyleBase header</x:t></x:r></x:p></x:hdr>"#));
    p.set("word/numbering-style-test.xml", format!(r#"<w:numbering xmlns:w="{W}"><w:abstractNum w:abstractNumId="99"><w:styleLink w:val="StyleBase"/><w:lvl w:ilvl="0"><w:pStyle w:val="StyleBase"/></w:lvl></w:abstractNum></w:numbering>"#));
    let source = output("replace-style-source.docx");
    std::fs::write(&source, p.bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":source}))
        .unwrap();
    command(
        &mut c,
        json!({"action":"apply","styleId":"StyleBase","paths":[[0],[3,0,0,0]]}),
    )
    .unwrap();
    command(
        &mut c,
        json!({"action":"apply","styleId":"StyleChild","paths":[[1]]}),
    )
    .unwrap();
    // Changes not yet in the preserved package must survive replacement.
    let mut doc = c.workspace["document"].clone();
    doc["content"][0]["content"][0]["text"] = json!("Changed before style deletion");
    c.dispatch(json!({"op":"document","document":doc})).unwrap();
    let before = c.workspace.clone();
    let before_export = export(&mut c, "replace-style-before.docx");
    command(
        &mut c,
        json!({"action":"delete","styleId":"StyleBase","replacementStyleId":"Normal"}),
    )
    .unwrap();
    let after = c.workspace.clone();
    for (a, b) in [
        (
            &after["document"]["content"][0],
            &before["document"]["content"][0],
        ),
        (
            &after["document"]["content"][3]["content"][0]["content"][0]["content"][0],
            &before["document"]["content"][3]["content"][0]["content"][0]["content"][0],
        ),
    ] {
        assert_eq!(a["attrs"]["paragraphFormat"]["styleId"], "Normal");
        assert_eq!(
            a["attrs"]["paragraphFormat"]["direct"],
            b["attrs"]["paragraphFormat"]["direct"]
        );
        assert_eq!(a["content"], b["content"]);
    }
    assert_eq!(find(&catalog(&c), "StyleChild")["basedOn"], "Normal");
    assert_eq!(after["sheets"], before["sheets"]);
    let out = export(&mut c, "replace-style-after.docx");
    let expected_header = p
        .text("word/header-style-test.xml")
        .unwrap()
        .replace("x:val='StyleBase'", "x:val='Normal'");
    assert_eq!(
        out.text("word/header-style-test.xml").unwrap(),
        expected_header
    );
    assert!(!out
        .text("word/numbering-style-test.xml")
        .unwrap()
        .contains("StyleBase"));
    assert!(!out
        .text("word/styles/named.xml")
        .unwrap()
        .contains("StyleBase"));
    for (name, bytes) in &before_export.entries {
        if ![
            "word/document.xml",
            "word/styles/named.xml",
            "word/header-style-test.xml",
            "word/numbering-style-test.xml",
        ]
        .contains(&name.as_str())
        {
            assert_eq!(&out.entries[name], bytes, "unrelated part {name}");
        }
    }
    assert_eq!(docx::import(&out).unwrap().0, after["document"]);
    let path = output("replace-style.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(
        export(&mut loaded, "replace-style-recovered.docx").entries,
        out.entries
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], before["document"]);
    assert_eq!(
        export(&mut c, "replace-style-undone.docx").entries,
        before_export.entries
    );
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["document"], after["document"]);
    assert_eq!(
        export(&mut c, "replace-style-redone.docx").entries,
        out.entries
    );
}

#[test]
fn invalid_style_replacements_are_atomic_and_native_heading_replacement_reverts_outline() {
    let mut c = setup("replace-style-invalid.docx");
    let before = c.workspace.clone();
    for replacement in [
        json!("StyleBase"),
        json!("StyleChild"),
        json!("Missing"),
        json!("CharBase"),
        Value::Null,
        json!(3),
    ] {
        assert!(command(
            &mut c,
            json!({"action":"delete","styleId":"StyleBase","replacementStyleId":replacement})
        )
        .is_err());
        assert_eq!(c.workspace, before);
    }
    assert!(command(
        &mut c,
        json!({"action":"delete","styleId":"Normal","replacementStyleId":"StyleBase"})
    )
    .is_err());
    assert_eq!(c.workspace, before);
    let mut native = Core::new();
    let available = native.dispatch(json!({"op":"documentStyles"})).unwrap();
    let mut custom = find(&available["catalog"], "Heading1");
    custom["id"] = json!("CustomHeading");
    custom["name"] = json!("Custom heading");
    command(&mut native, json!({"action":"upsert","style":custom})).unwrap();
    command(
        &mut native,
        json!({"action":"apply","styleId":"CustomHeading","paths":[[0]]}),
    )
    .unwrap();
    let before = native.workspace["document"].clone();
    command(
        &mut native,
        json!({"action":"delete","styleId":"CustomHeading","replacementStyleId":"Normal"}),
    )
    .unwrap();
    assert_eq!(
        native.workspace["document"]["content"][0]["type"],
        "paragraph"
    );
    let out = export(&mut native, "replace-native-heading.docx");
    assert!(!out
        .text("word/styles.xml")
        .unwrap()
        .contains("styleId=\"CustomHeading\""));
    assert!(docx::import(&out).is_ok());
    native.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(native.workspace["document"], before);
}

fn missing_headings_package() -> Package {
    let mut p = docx::new_package();
    let xml = p.text("word/styles.xml").unwrap().to_string();
    let dom = parse(&xml).unwrap();
    let ranges = dom
        .descendants()
        .filter(|n| {
            n.has_tag_name((W, "style"))
                && n.attribute((W, "styleId"))
                    .unwrap_or("")
                    .starts_with("Heading")
        })
        .map(|n| (n.range(), String::new()))
        .collect();
    p.set(
        "word/styles.xml",
        crate::office::replace_ranges(&xml, ranges),
    );
    let xml=p.text("word/document.xml").unwrap().replace("<w:sectPr>",
        "<w:p><w:pPr><w:ind w:left=\"200\"/></w:pPr><w:r><w:rPr><w:b w:val=\"0\"/></w:rPr><w:t>Heading candidate</w:t></w:r></w:p><w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w=\"2500\"/></w:tblGrid><w:tr><w:tc><w:tcPr/><w:p><w:r><w:t>Cell candidate</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:sectPr>");
    p.set("word/document.xml", xml);
    Package::from_bytes(p.bytes().unwrap()).unwrap()
}
fn import_builtin_fixture(name: &str, p: Package) -> Core {
    let file = output(name);
    std::fs::write(&file, p.original).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":file}))
        .unwrap();
    c
}
#[test]
fn missing_heading_gallery_materializes_each_level_with_roundtrip_and_single_history() {
    let mut c = import_builtin_fixture("missing-headings.docx", missing_headings_package());
    let original = c.workspace.clone();
    let data = c.dispatch(json!({"op":"documentStyles"})).unwrap();
    assert_eq!(
        c.workspace, original,
        "reading the gallery must not create styles"
    );
    // Nine headings, Caption and TOC 1–3.
    assert_eq!(data["availableStyles"].as_array().unwrap().len(), 13);
    for level in 1..=9 {
        let before = c.workspace.clone();
        command(
            &mut c,
            json!({"action":"apply","styleId":format!("Heading{level}"),"paths":[[0]]}),
        )
        .unwrap();
        let doc = &c.workspace["document"];
        assert_eq!(doc["content"][0]["attrs"]["level"], level);
        assert_eq!(
            doc["content"][0]["content"],
            original["document"]["content"][0]["content"]
        );
        assert_eq!(
            doc["content"][0]["attrs"]["paragraphFormat"]["direct"],
            original["document"]["content"][0]["attrs"]["paragraphFormat"]["direct"]
        );
        assert_eq!(
            find(&catalog(&c), &format!("Heading{level}"))["customStyle"],
            false
        );
        let changed = c.workspace["document"].clone();
        let out = export(&mut c, &format!("builtin-heading-{level}.docx"));
        assert_eq!(docx::import(&out).unwrap().0, changed);
        c.dispatch(json!({"op":"undo"})).unwrap();
        assert_eq!(c.workspace["document"], before["document"]);
        c.dispatch(json!({"op":"redo"})).unwrap();
        assert_eq!(c.workspace["document"], changed);
    }
    let saved = output("builtin-headings.folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":saved})).unwrap();
    assert_eq!(loaded.workspace["document"], c.workspace["document"]);
    assert_eq!(
        docx::import(&export(&mut loaded, "builtin-headings-reopened.docx"))
            .unwrap()
            .0,
        c.workspace["document"]
    );
}
#[test]
fn missing_heading_in_table_draft_waits_for_apply_and_collision_is_not_overwritten() {
    let mut p = missing_headings_package();
    let xml=p.text("word/styles.xml").unwrap().replace("</w:styles>","<w:style w:type=\"character\" w:customStyle=\"1\" w:styleId=\"Heading1\"><w:name w:val=\"Custom character\"/><w:rPr><w:i/></w:rPr></w:style></w:styles>");
    p.set("word/styles.xml", xml);
    let mut c = import_builtin_fixture(
        "heading-collision.docx",
        Package::from_bytes(p.bytes().unwrap()).unwrap(),
    );
    let original = c.workspace.clone();
    let data = c.dispatch(json!({"op":"documentStyles"})).unwrap();
    let candidate = data["availableStyles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Heading 1")
        .unwrap()["id"]
        .clone();
    assert_ne!(candidate, "Heading1");
    let preview=c.dispatch(json!({"op":"documentStylePreview","action":"apply","styleId":candidate,"paths":[[1,0,0,0]],"document":c.workspace["document"],"baseRevision":c.workspace["revision"],"workspaceId":c.workspace["id"]})).unwrap();
    assert_eq!(c.workspace, original);
    let mut draft = preview["document"].clone();
    // The table dialog retains paragraph tokens, not a mutable global catalog.
    draft["attrs"][styles::KEY] = original["document"]["attrs"][styles::KEY].clone();
    c.dispatch(json!({"op":"document","document":draft}))
        .unwrap();
    assert_eq!(find(&catalog(&c), "Heading1")["type"], "character");
    assert_eq!(
        find(&catalog(&c), candidate.as_str().unwrap())["customStyle"],
        false
    );
    let changed = c.workspace["document"].clone();
    assert_eq!(
        changed["content"][1]["content"][0]["content"][0]["content"][0]["attrs"]["level"],
        1
    );
    assert_eq!(
        docx::import(&export(&mut c, "builtin-cell-collision.docx"))
            .unwrap()
            .0,
        changed
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], original["document"]);
}
#[test]
fn builtin_origin_survives_rename_and_legacy_origin_lookup_without_allowing_deletion() {
    let mut c = Core::new();
    let data = c.dispatch(json!({"op":"documentStyles"})).unwrap();
    let mut heading = find(&data["catalog"], "Heading1");
    heading["name"] = json!("Renamed section title");
    command(&mut c, json!({"action":"upsert","style":heading})).unwrap();
    let before = c.workspace.clone();
    for request in [
        json!({"action":"delete","styleId":"Heading1"}),
        json!({"action":"delete","styleId":"Heading1","replacementStyleId":"Normal"}),
    ] {
        assert!(command(&mut c, request).is_err());
        assert_eq!(c.workspace, before);
    }
    heading["customStyle"] = json!(true);
    assert!(command(&mut c, json!({"action":"upsert","style":heading})).is_err());
    assert_eq!(c.workspace, before);
    heading["id"] = json!("CustomHeading");
    heading["name"] = json!("My heading");
    heading["customStyle"] = json!(false);
    command(&mut c, json!({"action":"upsert","style":heading})).unwrap();
    assert_eq!(find(&catalog(&c), "CustomHeading")["customStyle"], true);
    // Legacy native catalogs have no origins and no Office source package.
    for s in c.workspace["document"]["attrs"][styles::KEY]["styles"]
        .as_array_mut()
        .unwrap()
    {
        s.as_object_mut().unwrap().remove("customStyle");
    }
    let old = c.workspace.clone();
    let data = c.dispatch(json!({"op":"documentStyles"})).unwrap();
    assert_eq!(c.workspace, old);
    assert_eq!(find(&data["catalog"], "Heading1")["customStyle"], false);
    assert_eq!(find(&data["catalog"], "CustomHeading")["customStyle"], true);
    assert!(command(
        &mut c,
        json!({"action":"delete","styleId":"Heading1","replacementStyleId":"Normal"})
    )
    .is_err());
    assert_eq!(c.workspace, old);
    command(
        &mut c,
        json!({"action":"delete","styleId":"CustomHeading","replacementStyleId":"Normal"}),
    )
    .unwrap();
}
#[test]
fn heading_creation_without_any_style_part_keeps_document_defaults_and_following_style() {
    let mut p = missing_headings_package();
    p.entries.remove("word/styles.xml");
    let mut c = import_builtin_fixture(
        "no-style-headings.docx",
        Package::from_bytes(p.bytes().unwrap()).unwrap(),
    );
    let before = c.workspace["document"].clone();
    command(
        &mut c,
        json!({"action":"apply","styleId":"Heading2","paths":[[0]]}),
    )
    .unwrap();
    assert_eq!(
        catalog(&c)["defaults"],
        before["attrs"][styles::KEY]["defaults"]
    );
    assert_eq!(catalog(&c)["defaultStyle"], Value::Null);
    assert_eq!(find(&catalog(&c), "Heading2")["next"], "Normal");
    assert_eq!(find(&catalog(&c), "Normal")["customStyle"], false);
    let doc = c.workspace["document"].clone();
    assert_eq!(
        docx::import(&export(&mut c, "builtin-no-style-part.docx"))
            .unwrap()
            .0,
        doc
    );
}
#[test]
fn queued_editor_style_inheritance_keeps_engine_catalog_and_rejects_catalog_injection() {
    let mut c = import_builtin_fixture("queued-heading.docx", missing_headings_package());
    let mut old = c.workspace["document"].clone();
    old["content"][0]["type"] = json!("heading");
    old["content"][0]["attrs"]["level"] = json!(3);
    old["content"][0]["attrs"]["paragraphFormat"]["styleId"] = json!("Heading3");
    c.dispatch(json!({"op":"document","document":old})).unwrap();
    let declared = catalog(&c);
    let state = c.workspace.clone();
    for invalid in [
        Value::Null,
        json!(3),
        json!({"type":"doc","attrs":"invalid","content":[]}),
    ] {
        assert!(c
            .dispatch(json!({"op":"document","inheritStyleCatalog":true,"document":invalid}))
            .is_err());
        assert_eq!(c.workspace, state);
    }
    old["content"][0]["content"][0]["text"] = json!("Typed while the heading was saving");
    assert!(c.dispatch(json!({"op":"document","document":old})).is_err());
    old["attrs"][styles::KEY]["defaults"]["run"]["sz"] = json!(999);
    c.dispatch(json!({"op":"document","inheritStyleCatalog":true,"document":old}))
        .unwrap();
    assert_eq!(catalog(&c), declared);
    assert_eq!(
        c.workspace["document"]["content"][0]["content"][0]["text"],
        "Typed while the heading was saving"
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(catalog(&c), declared);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert!(
        c.dispatch(json!({"op":"documentStyles"})).unwrap()["availableStyles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == "Heading3")
    );
}
