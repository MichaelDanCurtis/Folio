use crate::{docx, docx_revisions as rev, office::*, workspace::Core};
use serde_json::json;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn package(body: &str) -> Package {
    let mut p = docx::new_package();
    p.set(
        "word/document.xml",
        format!("<q:document xmlns:q=\"{W}\"><q:body>{body}<q:sectPr/></q:body></q:document>"),
    );
    p.original = p.bytes().unwrap();
    p
}
fn meta(id: u32) -> String {
    format!("q:id=\"{id}\" q:author=\"Alex\" q:date=\"2026-09-20T12:00:00Z\"")
}
fn all(p: &Package, action: &str) -> Package {
    let list = rev::list(p).unwrap();
    rev::resolve(p,&json!({"action":action,"keys":list["revisions"].as_array().unwrap().iter().map(|r|r["key"].clone()).collect::<Vec<_>>()})).unwrap()
}
fn visible(p: &Package) -> String {
    let xml = p.text("word/document.xml").unwrap();
    let d = parse(xml).unwrap();
    rev::preview(d.root_element())
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["text"].as_str())
        .collect()
}
fn save(p: &Package, name: &str) {
    std::fs::write(
        std::env::temp_dir().join(format!("folio-revisions-{name}.docx")),
        p.bytes().unwrap(),
    )
    .unwrap();
}
#[test]
fn revisions_text_namespace_formatting_import_export_and_nested_bulk() {
    let p=package(&format!("<q:p><q:r><q:t xml:space=\"preserve\">Keep </q:t></q:r><q:del {}><q:r><q:rPr><q:b/></q:rPr><q:delText>old😀</q:delText></q:r></q:del><q:ins {}><q:r><q:rPr><q:i/></q:rPr><q:t>new</q:t></q:r></q:ins></q:p>",meta(1),meta(2)));
    let (d, _) = docx::import(&p).unwrap();
    assert_eq!(d["content"][0]["attrs"]["label"], "Tracked changes");
    assert_eq!(
        d["content"][0]["attrs"]["revisionPreview"][1]["kind"],
        "delete"
    );
    assert_eq!(docx::export(&d, Some(&p), Some(&d)).unwrap(), p.original);
    let accepted = all(&p, "accept");
    let rejected = all(&p, "reject");
    assert_eq!(visible(&accepted), "Keep new");
    assert_eq!(visible(&rejected), "Keep old😀");
    assert!(rejected
        .text("word/document.xml")
        .unwrap()
        .contains("<q:b/>"));
    assert!(!rejected
        .text("word/document.xml")
        .unwrap()
        .contains("delText"));
    save(&p, "input");
    save(&accepted, "accepted");
    save(&rejected, "rejected");
    let nested=package(&format!("<q:p><q:ins {}><q:r><q:t>A</q:t></q:r><q:ins {}><q:r><q:t>B</q:t></q:r></q:ins><q:del {}><q:r><q:delText>C</q:delText></q:r></q:del></q:ins></q:p>",meta(3),meta(4),meta(5)));
    let a = all(&nested, "accept");
    assert_eq!(visible(&a), "AB");
    assert!(rev::list(&a).unwrap()["revisions"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(visible(&all(&nested, "reject")).is_empty());
    let nested=package(&format!("<q:p><q:del {}><q:r><q:delText>A</q:delText></q:r><q:del {}><q:r><q:delText>B</q:delText></q:r></q:del></q:del></q:p>",meta(6),meta(7)));
    let r = rev::list(&nested).unwrap();
    let outer = rev::resolve(
        &nested,
        &json!({"action":"reject","keys":[r["revisions"][0]["key"]]}),
    )
    .unwrap();
    assert_eq!(
        rev::list(&outer).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(outer
        .text("word/document.xml")
        .unwrap()
        .contains("delText>B"));
    assert_eq!(visible(&all(&nested, "reject")), "AB");
    save(&all(&nested, "reject"), "nested");
}
#[test]
fn revisions_paragraph_breaks_join_without_losing_following_content() {
    for (tag, action) in [("del", "accept"), ("ins", "reject")] {
        let p=package(&format!("<q:p><q:pPr><q:rPr><q:{tag} {}/></q:rPr></q:pPr><q:r><q:t>First</q:t></q:r></q:p><q:p><q:pPr><q:jc q:val=\"right\"/></q:pPr><q:r><q:t>Second</q:t></q:r></q:p><q:p><q:r><q:t>Third</q:t></q:r></q:p>",meta(1)));
        let a = all(&p, action);
        let x = a.text("word/document.xml").unwrap();
        let dom = parse(x).unwrap();
        assert_eq!(
            dom.descendants()
                .filter(|n| n.has_tag_name((W, "p")))
                .count(),
            2
        );
        assert_eq!(visible(&a), "FirstSecond\nThird");
        assert_eq!(
            dom.descendants()
                .find(|n| n.has_tag_name((W, "jc")))
                .unwrap()
                .attribute((W, "val")),
            Some("right")
        );
        save(&a, &format!("break-{tag}"));
    }
    let p = package(&format!(
        "<q:p><q:pPr><q:rPr><q:ins {}/></q:rPr></q:pPr><q:r><q:t>Last paragraph</q:t></q:r></q:p>",
        meta(1)
    ));
    let r = rev::list(&p).unwrap();
    assert_eq!(r["revisions"][0]["canAccept"], true);
    assert_eq!(r["revisions"][0]["canReject"], false);
    assert_eq!(visible(&all(&p, "accept")), "Last paragraph");
}
#[test]
fn revisions_property_snapshots_and_row_lifetimes() {
    let p=package(&format!("<q:p><q:pPr><q:jc q:val=\"right\"/><q:pPrChange {}><q:pPr><q:jc q:val=\"left\"/></q:pPr></q:pPrChange></q:pPr><q:r><q:rPr><q:b/><q:rPrChange {}><q:rPr><q:i/></q:rPr></q:rPrChange></q:rPr><q:t>Text</q:t></q:r></q:p>",meta(1),meta(2)));
    let listed = rev::list(&p).unwrap();
    assert_eq!(
        listed["revisions"][0]["formatChanges"][0]["property"],
        "Alignment"
    );
    assert_eq!(listed["revisions"][0]["formatChanges"][0]["before"], "left");
    assert_eq!(listed["revisions"][0]["formatChanges"][0]["after"], "right");
    let a = all(&p, "accept");
    let b = all(&p, "reject");
    assert!(a.text("word/document.xml").unwrap().contains("<q:b/>"));
    assert!(b.text("word/document.xml").unwrap().contains("<q:i/>"));
    assert!(!b.text("word/document.xml").unwrap().contains("<q:b/>"));
    assert!(b.text("word/document.xml").unwrap().contains("left"));
    save(&a, "format-accepted");
    save(&b, "format-rejected");
    let p=package(&format!("<q:tbl><q:tblPr/><q:tblGrid><q:gridCol q:w=\"1000\"/></q:tblGrid><q:tr><q:trPr><q:ins {}/></q:trPr><q:tc><q:p><q:ins {}><q:r><q:t>New row</q:t></q:r></q:ins></q:p></q:tc></q:tr></q:tbl>",meta(3),meta(4)));
    assert_eq!(visible(&all(&p, "accept")), "New row");
    assert_eq!(visible(&all(&p, "reject")), "");
    save(&all(&p, "accept"), "row-accepted");
    save(&all(&p, "reject"), "row-rejected");
}
#[test]
fn revisions_story_parts_protection_stale_and_unsupported_are_atomic() {
    let mut p = package("<q:p/>");
    let content=p.text("[Content_Types].xml").unwrap().replace("</Types>","<Override PartName=\"/word/header7.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/></Types>");
    p.set("[Content_Types].xml", content);
    p.set("word/header7.xml",format!("<q:hdr xmlns:q=\"{W}\"><q:p><q:ins {}><q:r><q:t>Header</q:t></q:r></q:ins></q:p></q:hdr>",meta(8)));
    p.original = p.bytes().unwrap();
    let list = rev::list(&p).unwrap();
    assert_eq!(list["revisions"][0]["story"], "Header");
    assert!(rev::list(&all(&p, "accept")).unwrap()["revisions"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(rev::resolve(&p, &json!({"action":"accept","keys":["stale"]})).is_err());
    let types=p.text("[Content_Types].xml").unwrap().replace("</Types>","<Override PartName=\"/word/custom-settings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml\"/></Types>");
    p.set("[Content_Types].xml", types);
    p.set("word/custom-settings.xml",format!("<q:settings xmlns:q=\"{W}\"><q:documentProtection q:edit=\"trackedChanges\" q:enforcement=\"1\"/></q:settings>"));
    let before = p.bytes().unwrap();
    let list = rev::list(&p).unwrap();
    assert_eq!(list["protected"], true);
    assert_eq!(list["revisions"][0]["canAccept"], false);
    assert!(rev::resolve(
        &p,
        &json!({"action":"accept","keys":[list["revisions"][0]["key"]]})
    )
    .is_err());
    assert_eq!(before, p.bytes().unwrap());
    let p=package(&format!("<q:p><q:moveFrom {}><q:r><q:t>Moved</q:t></q:r></q:moveFrom><q:ins {}><q:r><q:t>Added</q:t></q:r></q:ins></q:p>",meta(1),meta(2)));
    let l = rev::list(&p).unwrap();
    assert_eq!(l["revisions"][0]["canAccept"], false);
    assert!(rev::resolve(&p,&json!({"action":"accept","keys":l["revisions"].as_array().unwrap().iter().map(|r|r["key"].clone()).collect::<Vec<_>>()})).is_err());
}
#[test]
fn revisions_crossing_bookmark_range_is_rejected_without_partial_output() {
    let p=package(&format!("<q:p><q:bookmarkStart q:id=\"9\" q:name=\"KeepRange\"/><q:r><q:t>Before</q:t></q:r><q:del {}><q:r><q:delText>Old</q:delText></q:r><q:bookmarkEnd q:id=\"9\"/></q:del></q:p>",meta(1)));
    let before = p.bytes().unwrap();
    let list = rev::list(&p).unwrap();
    let error = rev::resolve(
        &p,
        &json!({"action":"accept","keys":[list["revisions"][0]["key"]]}),
    )
    .err()
    .unwrap();
    assert!(error.contains("crosses a bookmark"));
    assert_eq!(p.bytes().unwrap(), before);
}

#[test]
fn revisions_workspace_history_pending_edits_recovery_and_export_guards() {
    let p=package(&format!("<q:p><q:ins {}><q:r><q:t>Added</q:t></q:r></q:ins></q:p><q:p><q:r><q:t>Editable</q:t></q:r></q:p>",meta(1)));
    let path =
        std::env::temp_dir().join(format!("folio-revisions-core-{}.docx", std::process::id()));
    std::fs::write(&path, &p.original).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let mut d = c.workspace["document"].clone();
    d["content"][1]["content"][0]["text"] = json!("Pending edit");
    c.dispatch(json!({"op":"document","document":d})).unwrap();
    let before = c.workspace.clone();
    let list = c.dispatch(json!({"op":"documentRevisions"})).unwrap();
    let r = json!({"op":"documentRevision","workspaceId":before["id"],"baseRevision":before["revision"],"action":"accept","keys":[list["revisions"][0]["key"]]});
    c.dispatch(r.clone()).unwrap();
    let after = c.workspace.clone();
    assert_eq!(
        after["document"]["content"][1]["content"][0]["text"],
        "Pending edit"
    );
    assert_eq!(
        after["officeDocument"]["sourceId"],
        before["officeDocument"]["sourceId"]
    );
    assert!(c.dispatch(r).is_err());
    assert_eq!(c.workspace, after);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], before["document"]);
    let mut removed = c.workspace["document"].clone();
    removed["content"].as_array_mut().unwrap().remove(0);
    assert!(c
        .dispatch(json!({"op":"document","document":removed}))
        .is_err());
    assert!(docx::export(&removed, Some(&p), Some(&docx::import(&p).unwrap().0)).is_err());
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["document"], after["document"]);
    let saved = path.with_extension("folio");
    c.dispatch(json!({"op":"save","path":saved})).unwrap();
    let mut reopened = Core::new();
    reopened
        .dispatch(json!({"op":"load","path":saved}))
        .unwrap();
    assert_eq!(reopened.workspace["document"], after["document"]);
    let out = std::env::temp_dir().join("folio-revisions-core-reviewed.docx");
    reopened
        .dispatch(json!({"op":"exportOffice","format":"docx","path":out}))
        .unwrap();
}
#[test]
fn revisions_keep_live_workbook_blocks_and_reject_cross_boundary_join_atomically() {
    let p = package(&format!(
        "<q:p><q:ins {}><q:r><q:t>Added</q:t></q:r></q:ins></q:p>",
        meta(1)
    ));
    let path = std::env::temp_dir().join("folio-revisions-linked-input.docx");
    std::fs::write(&path, &p.original).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let sheet = c.workspace["sheets"][0]["id"].clone();
    let mut d = c.workspace["document"].clone();
    let linked = json!({"type":"workbookBlock","attrs":{"sheetId":sheet,"range":"A1:B2","title":"Live figures"}});
    d["content"].as_array_mut().unwrap().push(linked.clone());
    c.dispatch(json!({"op":"document","document":d})).unwrap();
    let l = c.dispatch(json!({"op":"documentRevisions"})).unwrap();
    c.dispatch(json!({"op":"documentRevision","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"action":"accept","keys":[l["revisions"][0]["key"]]})).unwrap();
    assert_eq!(c.workspace["document"]["content"][1], linked);
    assert!(
        c.dispatch(json!({"op":"documentRevisions"})).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    c.dispatch(
        json!({"op":"cells","cells":[{"sheetId":sheet,"address":"A1","value":"New live value"}]}),
    )
    .unwrap();
    let out = std::env::temp_dir().join("folio-revisions-linked.docx");
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":out}))
        .unwrap();
    assert!(visible(&Package::read(out.to_str().unwrap()).unwrap()).contains("New live value"));
    let p = package(&format!(
        "<q:p><q:pPr><q:rPr><q:del {}/></q:rPr></q:pPr><q:r><q:t>Adjacent</q:t></q:r></q:p>",
        meta(2)
    ));
    std::fs::write(&path, &p.original).unwrap();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let mut d = c.workspace["document"].clone();
    d["content"].as_array_mut().unwrap().push(linked);
    c.dispatch(json!({"op":"document","document":d})).unwrap();
    let before = c.workspace.clone();
    let l = c.dispatch(json!({"op":"documentRevisions"})).unwrap();
    let err=c.dispatch(json!({"op":"documentRevision","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"action":"accept","keys":[l["revisions"][0]["key"]]})).unwrap_err();
    assert!(err.contains("workbook block boundary"), "{err}");
    assert_eq!(before, c.workspace);
}
#[test]
fn revisions_multiple_breaks_default_namespace_and_legacy_hydration() {
    let p=package(&format!("<q:p><q:pPr><q:rPr><q:del {}/></q:rPr></q:pPr><q:r><q:t>A</q:t></q:r></q:p><q:p><q:pPr><q:rPr><q:del {}/></q:rPr></q:pPr><q:r><q:t>B</q:t></q:r></q:p><q:p><q:r><q:t>C</q:t></q:r></q:p>",meta(1),meta(2)));
    assert_eq!(visible(&all(&p, "accept")), "ABC");
    let mut p = package("");
    p.set("word/document.xml",format!("<document xmlns=\"{W}\" xmlns:a=\"{W}\"><body><p><del a:id=\"3\" a:author=\"Alex\"><r><delText xml:space=\"preserve\"> kept &amp; restored </delText></r></del></p><sectPr/></body></document>"));
    p.original = p.bytes().unwrap();
    let r = all(&p, "reject");
    assert_eq!(visible(&r), " kept & restored ");
    save(&r, "default-namespace");
    let source = docx::import(&p).unwrap().0;
    let mut old = source.clone();
    old["content"][0]["attrs"]
        .as_object_mut()
        .unwrap()
        .remove("revisionPreview");
    let mut unchanged = old.clone();
    rev::hydrate(&mut unchanged, &source, &old);
    assert_eq!(unchanged, source);
    let mut edited = old.clone();
    edited["content"][0]["content"][0]["content"][0]["text"] = json!("User edit");
    let before = edited.clone();
    rev::hydrate(&mut edited, &source, &old);
    assert_eq!(edited, before);
}

#[test]
fn review_display_original_current_and_markup_are_non_mutating() {
    fn text(n: &serde_json::Value) -> String {
        n["text"].as_str().map(String::from).unwrap_or_else(|| {
            n["content"]
                .as_array()
                .map(|a| a.iter().map(text).collect())
                .unwrap_or_default()
        })
    }
    let pkg =
        Package::from_bytes(include_bytes!("../../test/fixtures/tracked-recording.docx").to_vec())
            .unwrap();
    let bytes = pkg.bytes().unwrap();
    let records = rev::list(&pkg).unwrap();
    let mut results = json!({});
    for mode in ["markup", "current", "original"] {
        let view = rev::display(&pkg, mode).unwrap();
        results[mode] = view.clone();
        assert_eq!(pkg.bytes().unwrap(), bytes);
        assert_eq!(rev::list(&pkg).unwrap(), records);
        assert_eq!(view["mode"], mode);
        let blocks = view["document"]["content"].as_array().unwrap();
        assert_eq!(blocks.len(), if mode == "original" { 3 } else { 4 });
        if mode == "original" {
            assert_eq!(
                blocks[2]["attrs"]["reviewBlocks"],
                json!(["word-2", "word-3"])
            );
            assert!(text(&view["document"]).contains("125000"));
            assert!(!text(&view["document"]).contains("Review due Friday"));
        } else if mode == "current" {
            assert!(text(&view["document"]).contains("141000"));
            assert!(text(&view["document"]).contains("Review due Friday"));
        }
    }
    std::fs::write(
        std::env::temp_dir().join("folio-review-display.json"),
        serde_json::to_vec_pretty(&results).unwrap(),
    )
    .unwrap();
}

#[test]
fn review_display_join_anchors_previous_formats_and_table_rows() {
    let pkg = package(&format!("<q:p><q:pPr><q:rPr><q:ins {}/></q:rPr></q:pPr><q:r><q:rPr><q:b/><q:rPrChange {}><q:rPr><q:i/></q:rPr></q:rPrChange></q:rPr><q:t>A😀</q:t></q:r></q:p><q:p><q:r><q:t>B</q:t></q:r></q:p><q:tbl><q:tblPr/><q:tblGrid><q:gridCol q:w=\"2000\"/></q:tblGrid><q:tr><q:trPr><q:del {}/></q:trPr><q:tc><q:p><q:r><q:t>Old row</q:t></q:r></q:p></q:tc></q:tr><q:tr><q:trPr><q:ins {}/></q:trPr><q:tc><q:p><q:r><q:t>New row</q:t></q:r></q:p></q:tc></q:tr></q:tbl>", meta(1), meta(2), meta(3), meta(4)));
    let original = rev::display(&pkg, "original").unwrap();
    let current = rev::display(&pkg, "current").unwrap();
    assert_eq!(
        original["document"]["content"][0]["attrs"]["reviewBlocks"],
        json!(["word-0", "word-1"])
    );
    assert_eq!(
        original["document"]["content"][1]["attrs"]["reviewBlocks"],
        json!(["word-2"])
    );
    assert!(original.to_string().contains("Old row"));
    assert!(!original.to_string().contains("New row"));
    assert!(current.to_string().contains("New row"));
    assert!(!current.to_string().contains("Old row"));
    let marks = &original["document"]["content"][0]["content"][0]["marks"];
    assert!(marks
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["type"] == "italic"));
    assert!(!marks
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["type"] == "bold"));
    assert_eq!(
        rev::list(&pkg).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn review_display_protected_source_keeps_actions_locked_and_unsupported_is_explicit() {
    let mut pkg = package(&format!("<q:p><q:del {}><q:r><q:delText>old</q:delText></q:r></q:del><q:ins {}><q:r><q:t>new</q:t></q:r></q:ins></q:p>",meta(1),meta(2)));
    pkg.set("word/settings.xml",format!("<w:settings xmlns:w=\"{W}\"><w:documentProtection w:edit=\"readOnly\" w:enforcement=\"1\"/></w:settings>"));
    // Ensure the synthetic settings part is declared to the package reader.
    let types=pkg.text("[Content_Types].xml").unwrap().replace("</Types>","<Override PartName=\"/word/settings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml\"/></Types>");
    pkg.set("[Content_Types].xml", types);
    let before = pkg.bytes().unwrap();
    assert_eq!(rev::list(&pkg).unwrap()["protected"], true);
    assert!(rev::display(&pkg, "current")
        .unwrap()
        .to_string()
        .contains("new"));
    assert!(rev::display(&pkg, "original")
        .unwrap()
        .to_string()
        .contains("old"));
    assert!(rev::list(&pkg).unwrap()["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["canAccept"] == false && r["canReject"] == false));
    assert_eq!(pkg.bytes().unwrap(), before);
    let unsupported = package(&format!(
        "<q:p><q:moveFrom {}><q:r><q:t>moved</q:t></q:r></q:moveFrom></q:p>",
        meta(3)
    ));
    assert!(rev::display(&unsupported, "markup").is_ok());
    assert!(rev::display(&unsupported, "original")
        .unwrap_err()
        .contains("view cannot yet display"));
    assert!(rev::display(&pkg, "invalid").is_err());
}

#[test]
fn review_display_core_does_not_change_revision_history_source_or_export() {
    let path = std::env::temp_dir().join("folio-review-display-source.docx");
    let output = std::env::temp_dir().join("folio-review-display-after.docx");
    std::fs::write(
        &path,
        include_bytes!("../../test/fixtures/tracked-recording.docx"),
    )
    .unwrap();
    let mut core = Core::default();
    core.dispatch(json!({"op":"new","blank":true})).unwrap();
    core.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let before = core.dispatch(json!({"op":"get"})).unwrap();
    let list = core.dispatch(json!({"op":"documentRevisions"})).unwrap();
    for mode in ["original", "current", "markup", "original"] {
        let result = core
            .dispatch(json!({"op":"documentReviewView","mode":mode}))
            .unwrap();
        assert_eq!(result["revisions"], list["revisions"]);
        assert_eq!(result["baseRevision"], list["baseRevision"]);
        assert_eq!(core.dispatch(json!({"op":"get"})).unwrap(), before);
    }
    core.dispatch(json!({"op":"exportOffice","format":"docx","path":output}))
        .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), std::fs::read(output).unwrap());
}
