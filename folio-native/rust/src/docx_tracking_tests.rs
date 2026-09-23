use crate::{
    document_bookmarks::Index, docx, docx_revisions as review, docx_track_edit::Author,
    docx_tracking as tracking, office::*,
};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn author() -> Author {
    Author {
        name: "Alex".into(),
        date: "2026-09-20T18:00:00Z".into(),
        enabled: true,
        formatting: true,
    }
}
fn paragraph(text: &str) -> String {
    format!(
        "<w:p><w:r><w:rPr><w:smallCaps/></w:rPr><w:t>{}</w:t></w:r></w:p>",
        esc(text)
    )
}
fn package(content: &str) -> Package {
    let mut p = docx::new_package();
    p.set(
        "word/document.xml",
        format!("<w:document xmlns:w=\"{W}\"><w:body>{content}<w:sectPr/></w:body></w:document>"),
    );
    p.entries
        .insert("word/media/preserved.bin".into(), vec![1, 2, 3, 4]);
    p.original = p.bytes().unwrap();
    p
}
fn save(p: &Package, name: &str) {
    std::fs::write(
        std::env::temp_dir().join(format!("folio-track-package-{name}.docx")),
        p.bytes().unwrap(),
    )
    .unwrap();
}
fn reject(p: &Package) -> Package {
    let list = review::list(p).unwrap();
    review::resolve(p,&json!({"action":"reject","keys":list["revisions"].as_array().unwrap().iter().map(|v|v["key"].clone()).collect::<Vec<_>>()})).unwrap()
}
fn edited(p: &Package, before: &Value, after: &Value, edits: Value) -> Package {
    tracking::edit(p, before, after, &edits, &author()).unwrap()
}
#[test]
fn ending_enter_and_typing_preserve_an_empty_final_paragraph() {
    for position in [4, 5] {
        let source = package(&paragraph("Body"));
        let before = tracking::import_current(&source).unwrap().0;
        let mut after = before.clone();
        let mut empty = after["content"][0].clone();
        empty["content"] = json!([]);
        after["content"].as_array_mut().unwrap().push(empty);
        let next = edited(
            &source,
            &before,
            &after,
            json!([{"start":position,"end":position,"text":"\n"}]),
        );
        assert_eq!(
            Index::new(&tracking::import_current(&next).unwrap().0).text,
            "Body\n\n"
        );
        let mut typed = after.clone();
        typed["content"][1]["content"] = json!([{"type":"text","text":"Next"}]);
        let result = edited(
            &next,
            &after,
            &typed,
            json!([{"start":5,"end":5,"text":"Next"}]),
        );
        assert_eq!(
            Index::new(&tracking::import_current(&result).unwrap().0).text,
            "Body\nNext\n"
        );
        assert_eq!(
            Index::new(&tracking::import_current(&reject(&result)).unwrap().0).text,
            "Body\n"
        );
    }
}
#[test]
fn native_appended_paragraph_continues_under_a_new_author() {
    let source =
        Package::from_bytes(include_bytes!("../../test/fixtures/tracked-recording.docx").to_vec())
            .unwrap();
    let fixture: Value = serde_json::from_str(include_str!(
        "../../test/fixtures/tracked-recording-enter.json"
    ))
    .unwrap();
    let reviewer = Author {
        name: "Morgan".into(),
        ..author()
    };
    let entered = tracking::edit(
        &source,
        &fixture["before"],
        &fixture["after"],
        &fixture["textEdits"],
        &reviewer,
    )
    .unwrap();
    let mut next = fixture["after"].clone();
    next["content"][4]["content"] = json!([{"type":"text","text":"Confirmed."}]);
    let result = tracking::edit(
        &entered,
        &fixture["after"],
        &next,
        &json!([{"start":108,"end":108,"text":"Confirmed."}]),
        &reviewer,
    )
    .unwrap();
    assert_eq!(
        Index::new(&tracking::import_current(&result).unwrap().0).text,
        Index::new(&next).text
    );
    let list = review::list(&result).unwrap();
    let keys = list["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["author"] == "Morgan")
        .map(|r| r["key"].clone())
        .collect::<Vec<_>>();
    assert_eq!(keys.len(), 2);
    let restored = review::resolve(&result, &json!({"action":"reject","keys":keys})).unwrap();
    assert_eq!(
        Index::new(&tracking::import_current(&restored).unwrap().0).text,
        Index::new(&fixture["before"]).text
    );
    assert_eq!(
        review::list(&restored).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    save(&result, "native-append");
}
#[test]
fn package_editor_bridge_handles_repeated_typing_stale_ids_and_rich_source() {
    let p = package(&paragraph("First second"));
    let before = tracking::import_current(&p).unwrap().0;
    let mut first = before["content"][0].clone();
    first["content"][0]["text"] = json!("First ");
    let mut second = first.clone();
    second["content"][0]["text"] = json!("second");
    let mut split = before.clone();
    split["content"] = json!([first, second]);
    // Both paragraphs retain the same old editor identity, as on Enter.
    assert_eq!(
        split["content"][0]["attrs"]["officeId"],
        split["content"][1]["attrs"]["officeId"]
    );
    let p1 = edited(
        &p,
        &before,
        &split,
        json!([{"start":6,"end":6,"text":"\n"}]),
    );
    assert_eq!(
        Index::new(&tracking::import_current(&p1).unwrap().0).text,
        "First \nsecond\n"
    );
    let mut after = split.clone();
    after["content"][1]["content"][0]["text"] = json!("last");
    let p2 = edited(
        &p1,
        &split,
        &after,
        json!([{"start":7,"end":13,"text":"last"}]),
    );
    let view = tracking::import_current(&p2).unwrap().0;
    assert_eq!(Index::new(&view).text, "First \nlast\n");
    assert_eq!(
        Index::new(&docx::import(&reject(&p2)).unwrap().0).text,
        "First second\n"
    );
    assert_eq!(
        p.entries["word/media/preserved.bin"],
        p2.entries["word/media/preserved.bin"]
    );
    let xml = p2.text("word/document.xml").unwrap();
    let d = parse(xml).unwrap();
    assert!(d.descendants().any(|n| n.has_tag_name((W, "smallCaps"))));
    save(&p2, "typing-split");
}
#[test]
fn package_editor_bridge_tracks_table_cell_edits_and_preserves_other_objects() {
    let table=format!("<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w=\"2400\"/><w:gridCol w:w=\"2400\"/></w:tblGrid><w:tr><w:tc>{}</w:tc><w:tc>{}</w:tc></w:tr></w:tbl>",paragraph("Cell one"),paragraph("Cell two"));
    let field="<w:p><w:r><w:fldChar w:fldCharType=\"begin\"/><w:instrText>DATE</w:instrText><w:fldChar w:fldCharType=\"separate\"/><w:t>Today</w:t><w:fldChar w:fldCharType=\"end\"/></w:r></w:p>";
    let p = package(&(paragraph("Title") + &table + field + &paragraph("Tail")));
    let before = tracking::import_current(&p).unwrap().0;
    assert_eq!(before["content"][2]["type"], "officeBlock");
    let mut after = before.clone();
    after["content"][1]["content"][0]["content"][0]["content"][0]["content"][0]["text"] =
        json!("Cell ONE");
    after["content"][3]["content"][0]["marks"] = json!([{"type":"bold"}]);
    let out = edited(
        &p,
        &before,
        &after,
        json!([{"start":11,"end":14,"text":"ONE"}]),
    );
    let view = tracking::import_current(&out).unwrap().0;
    assert_eq!(
        Index::new(&view).text,
        "Title\nCell ONE\nCell two\n\u{fffc}\nTail\n"
    );
    let reverted = reject(&out);
    assert_eq!(
        Index::new(&docx::import(&reverted).unwrap().0).text,
        Index::new(&before).text
    );
    let xml = out.text("word/document.xml").unwrap();
    let d = parse(xml).unwrap();
    let p_field = d
        .descendants()
        .find(|n| {
            n.has_tag_name((W, "p")) && n.descendants().any(|n| n.has_tag_name((W, "fldChar")))
        })
        .unwrap();
    assert_eq!(raw(xml, p_field), field);
    let ids = d
        .descendants()
        .filter(|n| {
            matches!(
                n.tag_name().name(),
                "ins" | "del" | "rPrChange" | "pPrChange"
            )
        })
        .filter_map(|n| n.attribute((W, "id")))
        .collect::<Vec<_>>();
    assert_eq!(
        ids.len(),
        ids.iter().collect::<std::collections::HashSet<_>>().len()
    );
    save(&out, "table-cell");
    save(&reverted, "table-cell-rejected");
}
#[test]
fn package_editor_bridge_rejects_cross_cell_edits_without_mutating_source() {
    let table=format!("<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w=\"2400\"/><w:gridCol w:w=\"2400\"/></w:tblGrid><w:tr><w:tc>{}</w:tc><w:tc>{}</w:tc></w:tr></w:tbl>",paragraph("One"),paragraph("Two"));
    let p = package(&table);
    let bytes = p.bytes().unwrap();
    let before = tracking::import_current(&p).unwrap().0;
    let mut after = before.clone();
    after["content"][0]["content"][0]["content"][0]["content"][0]["content"][0]["text"] =
        json!("OTwo");
    after["content"][0]["content"][0]["content"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    assert!(tracking::edit(
        &p,
        &before,
        &after,
        &json!([{"start":1,"end":4,"text":""}]),
        &author()
    )
    .is_err());
    assert_eq!(p.bytes().unwrap(), bytes);
}

fn mode(c: &mut crate::workspace::Core, enabled: bool, formatting: bool) -> Result<Value, String> {
    c.dispatch(json!({"op":"documentTracking","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"author":"Alex","enabled":enabled,"formatting":formatting}))
}
#[test]
fn tracking_core_typing_history_review_export_and_recovery() {
    let input = std::env::temp_dir().join("folio-track-core-input.docx");
    std::fs::write(&input, package(&paragraph("First second")).original).unwrap();
    let mut c = crate::workspace::Core::new();
    c.dispatch(json!({"op":"importOffice","path":input}))
        .unwrap();
    mode(&mut c, true, true).unwrap();
    let before = c.workspace.clone();
    let mut d = before["document"].clone();
    d["content"][0]["content"][0]["text"] = json!("First last");
    c.dispatch(
        json!({"op":"document","document":d,"textEdits":[{"start":6,"end":12,"text":"last"}]}),
    )
    .unwrap();
    assert_eq!(
        c.dispatch(json!({"op":"documentRevisions"})).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let changed = c.workspace.clone();
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], before["document"]);
    assert!(
        c.dispatch(json!({"op":"documentRevisions"})).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["document"], changed["document"]);
    assert_eq!(c.workspace["officeDocument"], changed["officeDocument"]);
    let output = std::env::temp_dir().join("folio-track-package-core-export.docx");
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":output}))
        .unwrap();
    let exported = Package::read(output.to_str().unwrap()).unwrap();
    assert!(parse(exported.text("word/settings.xml").unwrap())
        .unwrap()
        .descendants()
        .any(|n| n.has_tag_name((W, "trackRevisions"))));
    assert_eq!(
        Index::new(&docx::import(&reject(&exported)).unwrap().0).text,
        "First second\n"
    );
    let recovery = std::env::temp_dir().join("folio-track-core-recovery.folio");
    c.dispatch(json!({"op":"save","path":recovery})).unwrap();
    let mut reopened = crate::workspace::Core::new();
    reopened
        .dispatch(json!({"op":"load","path":recovery}))
        .unwrap();
    assert_eq!(reopened.workspace, c.workspace);
    let list = reopened
        .dispatch(json!({"op":"documentRevisions"}))
        .unwrap();
    reopened.dispatch(json!({"op":"documentRevision","workspaceId":reopened.workspace["id"],"baseRevision":reopened.workspace["revision"],"action":"reject","keys":list["revisions"].as_array().unwrap().iter().map(|v|v["key"].clone()).collect::<Vec<_>>()})).unwrap();
    assert_eq!(
        Index::new(&reopened.workspace["document"]).text,
        "First second\n"
    );
    assert_eq!(
        reopened.workspace["officeDocument"]["tracking"]["enabled"],
        true
    );
    let mut d = reopened.workspace["document"].clone();
    d["content"][0]["content"][0]["text"] = json!("First second!");
    reopened
        .dispatch(
            json!({"op":"document","document":d,"textEdits":[{"start":12,"end":12,"text":"!"}]}),
        )
        .unwrap();
    assert_eq!(
        reopened
            .dispatch(json!({"op":"documentRevisions"}))
            .unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn tracking_core_off_retains_history_and_formatting_switch_changes_behavior() {
    let mut c = crate::workspace::Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c.dispatch(json!({"op":"document","document":{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Base"}]}]}})).unwrap();
    mode(&mut c, true, false).unwrap();
    let mut d = c.workspace["document"].clone();
    d["content"][0]["content"][0]["text"] = json!("Base!");
    d["content"][0]["content"][0]["marks"] = json!([{"type":"bold"}]);
    c.dispatch(json!({"op":"document","document":d,"textEdits":[{"start":4,"end":4,"text":"!"}]}))
        .unwrap();
    let list = c.dispatch(json!({"op":"documentRevisions"})).unwrap();
    assert_eq!(list["revisions"].as_array().unwrap().len(), 1);
    mode(&mut c, false, false).unwrap();
    let mut d = c.workspace["document"].clone();
    d["content"][0]["content"][0]["text"] = json!("BASE!");
    c.dispatch(
        json!({"op":"document","document":d,"textEdits":[{"start":0,"end":4,"text":"BASE"}]}),
    )
    .unwrap();
    let list = c.dispatch(json!({"op":"documentRevisions"})).unwrap();
    assert_eq!(list["revisions"].as_array().unwrap().len(), 1);
    c.dispatch(json!({"op":"documentRevision","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"action":"reject","keys":[list["revisions"][0]["key"]]})).unwrap();
    assert_eq!(Index::new(&c.workspace["document"]).text, "BASE\n");
    assert_eq!(
        c.workspace["document"]["content"][0]["content"][0]["marks"][0]["type"],
        "bold"
    );
}
#[test]
fn tracking_core_stale_missing_journal_and_unsupported_changes_are_atomic() {
    let mut c = crate::workspace::Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let before = c.workspace.clone();
    assert!(c.dispatch(json!({"op":"documentTracking","workspaceId":before["id"],"baseRevision":before["revision"],"author":" ","enabled":true,"formatting":true})).is_err());
    assert_eq!(c.workspace, before);
    mode(&mut c, true, true).unwrap();
    let before = c.workspace.clone();
    let mut d = before["document"].clone();
    d["content"][0]["content"] = json!([{"type":"text","text":"new"}]);
    assert!(c
        .dispatch(json!({"op":"document","document":d}))
        .unwrap_err()
        .contains("journal"));
    assert_eq!(c.workspace, before);
    let mut d = before["document"].clone();
    d["content"] = json!([{"type":"workbookBlock","attrs":{"sheetId":"sheet-1","range":"A1","title":"Numbers"}}]);
    assert!(c.dispatch(json!({"op":"document","document":d})).is_err());
    assert_eq!(c.workspace, before);
}

#[test]
fn tracking_settings_preserve_custom_path_and_unrelated_settings() {
    let mut p = package(&paragraph("Settings"));
    let rels=p.text("word/_rels/document.xml.rels").unwrap().replace("</Relationships>","<Relationship Id=\"settingsSpecial\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings\" Target=\"preferences.xml\"/></Relationships>");
    p.set("word/_rels/document.xml.rels", rels);
    p.set("word/preferences.xml",format!("<x:settings xmlns:x=\"{W}\"><x:zoom x:percent=\"125\"/><x:revisionView x:markup=\"1\"/><x:defaultTabStop x:val=\"720\"/><x:compat/></x:settings>"));
    tracking::write_settings(
        &mut p,
        &json!({"version":1,"author":"Alex","enabled":true,"formatting":false}),
    )
    .unwrap();
    let xml = p.text("word/preferences.xml").unwrap();
    let d = parse(xml).unwrap();
    assert_eq!(
        d.root_element()
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect::<Vec<_>>(),
        vec![
            "zoom",
            "revisionView",
            "trackRevisions",
            "doNotTrackFormatting",
            "defaultTabStop",
            "compat"
        ]
    );
    assert!(xml.contains("<x:zoom x:percent=\"125\"/>"));
    save(&p, "settings-on");
    tracking::write_settings(
        &mut p,
        &json!({"version":1,"author":"Alex","enabled":false,"formatting":true}),
    )
    .unwrap();
    let d = parse(p.text("word/preferences.xml").unwrap()).unwrap();
    assert!(!d
        .descendants()
        .any(|n| n.has_tag_name((W, "trackRevisions"))
            || n.has_tag_name((W, "doNotTrackFormatting"))));
    save(&p, "settings-off");
}

#[test]
fn imported_recording_requires_explicit_author_or_recording_off_choice() {
    let mut p = package(&paragraph("Original"));
    tracking::write_settings(
        &mut p,
        &json!({"version":1,"author":"Source name","enabled":true,"formatting":false}),
    )
    .unwrap();
    let path = std::env::temp_dir().join("folio-track-requested.docx");
    std::fs::write(&path, &p.original).unwrap();
    let mut c = crate::workspace::Core::new();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let before = c.workspace.clone();
    assert_eq!(before["officeDocument"]["recordingRequested"], true);
    assert!(before["officeDocument"].get("tracking").is_none());
    let list = c.dispatch(json!({"op":"documentRevisions"})).unwrap();
    assert_eq!(list["tracking"]["author"], "");
    assert_eq!(
        list["sourceRecording"],
        json!({"enabled":true,"formatting":false})
    );
    let mut d = before["document"].clone();
    d["content"][0]["content"][0]["text"] = json!("Original!");
    assert!(c
        .dispatch(
            json!({"op":"document","document":d,"textEdits":[{"start":8,"end":8,"text":"!"}]})
        )
        .unwrap_err()
        .contains("Choose a review author"));
    assert_eq!(c.workspace, before);
    mode(&mut c, false, true).unwrap();
    assert_eq!(c.workspace["officeDocument"]["recordingRequested"], false);
    let mut d = c.workspace["document"].clone();
    d["content"][0]["content"][0]["text"] = json!("Original!");
    c.dispatch(json!({"op":"document","document":d,"textEdits":[{"start":8,"end":8,"text":"!"}]}))
        .unwrap();
    assert!(
        c.dispatch(json!({"op":"documentRevisions"})).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
