use crate::{docx_headers as headers, office::*};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
fn fixture() -> Package {
    let mut p =
        Package::from_bytes(include_bytes!("../../test/fixtures/section-breaks.docx").to_vec())
            .unwrap();
    p.set("word/document.xml",format!(r#"<w:document xmlns:w="{W}" xmlns:r="{R}"><w:body><w:p><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="head"/><w:footerReference w:type="default" r:id="foot"/></w:sectPr></w:pPr><w:r><w:t>Body one.</w:t></w:r></w:p><w:p><w:pPr><w:sectPr/></w:pPr><w:r><w:t>Body two.</w:t></w:r></w:p><w:p><w:r><w:t>Body three.</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="independent"/></w:sectPr></w:body></w:document>"#));
    p.set("word/nested/header.xml",format!(r#"<q:hdr xmlns:q="{W}" xmlns:r="{R}"><q:p><q:bookmarkStart q:id="20" q:name="HeaderName"/><q:r><q:rPr><q:b/></q:rPr><q:t>Shared 😀</q:t></q:r><q:hyperlink r:id="head"><q:r><q:t> link</q:t></q:r></q:hyperlink><q:bookmarkEnd q:id="20"/></q:p></q:hdr>"#));
    p.set("word/nested/_rels/header.xml.rels",format!(r#"<Relationships xmlns="{REL_NS}"><Relationship Id="head" Type="{R}/hyperlink" Target="https://example.org/header" TargetMode="External"/></Relationships>"#));
    p.set(
        "word/header2.xml",
        format!("<w:hdr xmlns:w=\"{W}\"><w:p><w:r><w:t>Independent</w:t></w:r></w:p></w:hdr>"),
    );
    let rels=p.text("word/_rels/document.xml.rels").unwrap().replace("Target=\"header1.xml\"","Target=\"nested/header.xml\"").replace("</Relationships>",&format!("<Relationship Id=\"independent\" Type=\"{R}/header\" Target=\"header2.xml\"/></Relationships>"));
    p.set("word/_rels/document.xml.rels", rels);
    let types=p.text("[Content_Types].xml").unwrap().replace("</Types>","<Override PartName=\"/word/nested/header.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/><Override PartName=\"/word/header2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/></Types>");
    p.set("[Content_Types].xml", types);
    p.original = p.bytes().unwrap();
    p
}
fn data(p: &Package, section: usize, kind: &str, variant: &str) -> Value {
    headers::read(p, &json!({"section":section,"kind":kind,"variant":variant})).unwrap()
}
fn request(d: &Value) -> Value {
    json!({"section":d["section"],"kind":d["kind"],"variant":d["variant"],"storyKey":d["storyKey"],"linked":d["linked"],"document":d["document"],"differentFirstPage":d["differentFirstPage"],"differentOddEven":d["differentOddEven"]})
}
fn label(d: &Value) -> String {
    crate::workspace::document_text(&d["document"])
}
#[test]
fn header_projection_separates_relationships_and_linked_owners() {
    let p = fixture();
    let d = data(&p, 1, "header", "default");
    assert_eq!(d["linked"], true);
    assert_eq!(d["ownerSection"], 0);
    assert_eq!(d["affectedSections"], json!([0, 1]));
    assert_eq!(label(&d), "Shared 😀 link");
    assert_eq!(
        d["document"]["content"][0]["content"][1]["marks"][0]["attrs"]["href"],
        "https://example.org/header"
    );
    assert_eq!(d["document"]["attrs"]["bookmarks"][0]["end"], 14);
    let before = p.bytes().unwrap();
    let same = headers::apply(&p, &request(&d), None).unwrap();
    assert_eq!(same.entries, p.entries);
    assert_eq!(p.bytes().unwrap(), before);
}
#[test]
fn linked_header_edits_update_chain_and_unlink_creates_independent_copy() {
    let p = fixture();
    let d = data(&p, 1, "header", "default");
    let mut r = request(&d);
    r["document"]["content"][0]["content"][0]["text"] = json!("Changed 😀");
    let edited = headers::apply(&p, &r, None).unwrap();
    assert_eq!(
        label(&data(&edited, 0, "header", "default")),
        "Changed 😀 link"
    );
    assert_eq!(
        label(&data(&edited, 1, "header", "default")),
        "Changed 😀 link"
    );
    assert_eq!(label(&data(&edited, 2, "header", "default")), "Independent");
    assert_eq!(
        edited.entries["word/document.xml"],
        p.entries["word/document.xml"]
    );
    let d = data(&edited, 1, "header", "default");
    let mut r = request(&d);
    r["linked"] = json!(false);
    r["document"]["content"][0]["content"][0]["text"] = json!("Only section two 😀");
    let unlinked = headers::apply(&edited, &r, None).unwrap();
    assert_eq!(
        label(&data(&unlinked, 0, "header", "default")),
        "Changed 😀 link"
    );
    assert_eq!(
        label(&data(&unlinked, 1, "header", "default")),
        "Only section two 😀 link"
    );
    assert_eq!(data(&unlinked, 1, "header", "default")["linked"], false);
    assert_eq!(
        data(&unlinked, 1, "header", "default")["document"]["content"][0]["content"][1]["marks"][0]
            ["attrs"]["href"],
        "https://example.org/header"
    );
    std::fs::write(
        std::env::temp_dir().join("folio-headers-unlinked.docx"),
        unlinked.bytes().unwrap(),
    )
    .unwrap();
    let d = data(&unlinked, 1, "header", "default");
    let mut r = request(&d);
    r["linked"] = json!(true);
    r["document"] = d["previousDocument"].clone();
    let linked = headers::apply(&unlinked, &r, None).unwrap();
    assert_eq!(data(&linked, 1, "header", "default")["linked"], true);
    assert_eq!(
        label(&data(&linked, 1, "header", "default")),
        "Changed 😀 link"
    );
    assert_eq!(
        unlinked.entries["word/folio-header-1.xml"],
        linked.entries["word/folio-header-1.xml"]
    );
    std::fs::write(
        std::env::temp_dir().join("folio-headers-linked.docx"),
        linked.bytes().unwrap(),
    )
    .unwrap();
}
#[test]
fn first_and_even_stories_and_settings_remain_independent() {
    let p = fixture();
    let d = data(&p, 1, "footer", "first");
    let mut r = request(&d);
    r["linked"] = json!(false);
    r["differentFirstPage"] = json!(true);
    r["differentOddEven"] = json!(true);
    r["document"]["content"] = json!([{"type":"paragraph","content":[{"type":"text","text":"Chapter opening","marks":[{"type":"italic"}]}]}]);
    let p = headers::apply(&p, &r, None).unwrap();
    assert_eq!(data(&p, 1, "footer", "first")["differentFirstPage"], true);
    assert_eq!(
        data(&p, 0, "header", "default")["differentFirstPage"],
        false
    );
    assert_eq!(data(&p, 2, "header", "even")["differentOddEven"], true);
    assert_eq!(label(&data(&p, 1, "footer", "first")), "Chapter opening");
    assert_eq!(label(&data(&p, 2, "footer", "first")), "Chapter opening");
    assert_eq!(label(&data(&p, 0, "footer", "first")), "");
    assert_eq!(label(&data(&p, 1, "footer", "default")), "Footer retained");
    let d = data(&p, 0, "header", "even");
    let mut r = request(&d);
    r["document"]["content"] = json!([{"type":"orderedList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Even list"}]}]}]}]);
    let p = headers::apply(&p, &r, None).unwrap();
    assert_eq!(label(&data(&p, 2, "header", "even")), "Even list");
    assert!(
        data(&p, 0, "header", "even")["document"]["content"][0]["attrs"]["numbering"].is_object()
    );
    let rels = p.text("word/_rels/document.xml.rels").unwrap();
    assert!(rels.contains("/numbering"));
    std::fs::write(
        std::env::temp_dir().join("folio-headers-variants.docx"),
        p.bytes().unwrap(),
    )
    .unwrap();
}
#[test]
fn headers_reject_stale_keys_recording_and_protection_without_mutation() {
    let p = fixture();
    let d = data(&p, 0, "header", "default");
    let mut r = request(&d);
    let before = p.bytes().unwrap();
    r["storyKey"] = json!("stale");
    assert!(headers::apply(&p, &r, None).is_err());
    r = request(&d);
    assert!(headers::apply(&p, &r, Some(&json!({"enabled":true})))
        .err()
        .unwrap()
        .contains("Recording"));
    r["document"]["content"] = json!([{"type":"workbookBlock","attrs":{"blockId":"missing"}}]);
    assert!(headers::apply(&p, &r, None).is_err());
    r = request(&d);
    assert_eq!(p.bytes().unwrap(), before);
    let mut protected = p.clone();
    let path = crate::docx_tabs::ensure_settings(&mut protected).unwrap();
    protected.set(&path,format!("<w:settings xmlns:w=\"{W}\"><w:documentProtection w:edit=\"readOnly\" w:enforcement=\"1\"/></w:settings>"));
    assert!(headers::apply(&protected, &r, None)
        .err()
        .unwrap()
        .contains("protection"));
}

#[test]
fn unlink_renames_bookmarks_and_retargets_simple_and_split_field_references() {
    let mut p = fixture();
    let x=p.text("word/nested/header.xml").unwrap().replace("</q:p>",r#"<q:hyperlink q:anchor="HeaderName"><q:r><q:t>jump</q:t></q:r></q:hyperlink><q:fldSimple q:instr=" REF HeaderName "><q:r><q:t>cached</q:t></q:r></q:fldSimple><q:r><q:fldChar q:fldCharType="begin"/></q:r><q:r><q:instrText xml:space="preserve"> PAGE</q:instrText></q:r><q:r><q:instrText xml:space="preserve">REF &quot;HeaderName&quot; \h </q:instrText></q:r><q:r><q:fldChar q:fldCharType="separate"/></q:r><q:r><q:t>1</q:t></q:r><q:r><q:fldChar q:fldCharType="end"/></q:r></q:p>"#);
    p.set("word/nested/header.xml", x.clone());
    let d = data(&p, 1, "header", "default");
    let mut r = request(&d);
    r["linked"] = json!(false);
    let out = headers::apply(&p, &r, None).unwrap();
    assert_eq!(out.text("word/nested/header.xml").unwrap(), x);
    let clone = out.text("word/folio-header-1.xml").unwrap();
    let d = parse(clone).unwrap();
    let bookmark = d
        .descendants()
        .find(|n| n.has_tag_name((W, "bookmarkStart")))
        .unwrap();
    assert_eq!(bookmark.attribute((W, "name")), Some("HeaderName_Copy1"));
    let link = d
        .descendants()
        .find(|n| n.has_tag_name((W, "hyperlink")) && n.attribute((W, "anchor")).is_some())
        .unwrap();
    assert_eq!(link.attribute((W, "anchor")), Some("HeaderName_Copy1"));
    let simple = d
        .descendants()
        .find(|n| n.has_tag_name((W, "fldSimple")))
        .unwrap();
    assert_eq!(
        simple.attribute((W, "instr")),
        Some(" REF HeaderName_Copy1 ")
    );
    let instruction = d
        .descendants()
        .filter(|n| n.has_tag_name((W, "instrText")))
        .map(|n| n.text().unwrap_or(""))
        .collect::<String>();
    assert_eq!(instruction, " PAGEREF \"HeaderName_Copy1\" \\h ");
    std::fs::write(
        std::env::temp_dir().join("folio-headers-bookmarks.docx"),
        out.bytes().unwrap(),
    )
    .unwrap();
}
#[test]
fn header_workspace_history_body_drafts_live_block_and_recovery() {
    use crate::workspace::Core;
    let source = std::env::temp_dir().join("folio-headers-source.docx");
    std::fs::write(&source, fixture().bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":source}))
        .unwrap();
    let mut doc = c.workspace["document"].clone();
    doc["content"][0]["content"][0]["text"] = json!("Draft body one.");
    let live = json!({"type":"workbookBlock","attrs":{"sheetId":c.workspace["sheets"][0]["id"],"range":"A1:B2","title":"Live data"}});
    doc["content"]
        .as_array_mut()
        .unwrap()
        .insert(0, live.clone());
    c.dispatch(json!({"op":"document","document":doc})).unwrap();
    c.dispatch(json!({"op":"bookmark","action":"set","name":"BodyDraft","start":2,"end":7,"baseRevision":c.workspace["revision"]})).unwrap();
    c.dispatch(json!({"op":"documentComment","action":"add","author":"Reviewer","text":"Body comment stays","start":2,"end":7,"workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"]})).unwrap();
    let before = c.workspace.clone();
    let d = c
        .dispatch(json!({"op":"documentHeaders","section":1,"kind":"header","variant":"default"}))
        .unwrap();
    assert_eq!(c.workspace, before);
    let mut r = request(&d);
    r["op"] = json!("documentHeadersApply");
    r["workspaceId"] = d["workspaceId"].clone();
    r["baseRevision"] = d["baseRevision"].clone();
    r["linked"] = json!(false);
    r["document"]["content"][0]["content"][0]["text"] = json!("Native chapter");
    c.dispatch(r.clone()).unwrap();
    let edited = c.workspace.clone();
    assert_eq!(edited["document"]["content"][0], live);
    let bookmark = crate::document_bookmarks::entries(&edited["document"])
        .iter()
        .find(|b| b["name"] == "BodyDraft")
        .unwrap();
    assert_eq!(bookmark["start"], 2);
    assert_eq!(bookmark["end"], 7);
    let comment = &crate::document_comments::entries(&edited["document"])[0];
    assert_eq!(comment["start"], 2);
    assert_eq!(comment["end"], 7);
    assert_eq!(comment["messages"][0]["text"], "Body comment stays");
    assert_eq!(
        crate::workspace::document_text(&edited["document"]["content"][1]),
        "Draft body one."
    );
    assert!(c.dispatch(r).err().unwrap().contains("changed"));
    assert_eq!(c.workspace, edited);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["document"], before["document"]);
    assert_eq!(c.workspace["officeDocument"], before["officeDocument"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["document"], edited["document"]);
    let path = std::env::temp_dir().join("folio-headers-recovery.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut fresh = Core::new();
    fresh.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(fresh.workspace["document"], edited["document"]);
    let d = fresh
        .dispatch(json!({"op":"documentHeaders","section":1,"kind":"header","variant":"default"}))
        .unwrap();
    assert_eq!(label(&d), "Native chapter link");
    let out = std::env::temp_dir().join("folio-headers-core.docx");
    fresh
        .dispatch(json!({"op":"exportOffice","format":"docx","path":out}))
        .unwrap();
}

#[test]
fn page_preview_snapshot_resolves_stories_numbering_and_preserves_workspace() {
    use crate::workspace::Core;
    let mut p = fixture();
    let xml = p.text("word/document.xml").unwrap().replace(
        "<w:sectPr/>",
        "<w:sectPr><w:pgNumType w:fmt=\"lowerRoman\" w:start=\"5\"/><w:titlePg/></w:sectPr>",
    );
    p.set("word/document.xml", xml);
    let before = p.entries.clone();
    let data = headers::page_preview(&p).unwrap();
    assert_eq!(p.entries, before);
    assert_eq!(data["sections"].as_array().unwrap().len(), 3);
    assert_eq!(
        data["sections"][1]["header"]["default"],
        data["sections"][0]["header"]["default"]
    );
    assert_eq!(
        crate::workspace::document_text(&data["sections"][2]["header"]["default"]),
        "Independent"
    );
    assert_eq!(data["sections"][1]["pageNumbering"]["start"], 5);
    assert_eq!(data["sections"][1]["differentFirstPage"], true);
    let source = std::env::temp_dir().join("folio-page-preview-source.docx");
    std::fs::write(&source, p.bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":source}))
        .unwrap();
    let mut doc = c.workspace["document"].clone();
    doc["content"][0]["content"][0]["text"] = json!("Pending page text");
    c.dispatch(json!({"op":"document","document":doc})).unwrap();
    let state = c.workspace.clone();
    let snapshot = c.dispatch(json!({"op":"documentPagePreview"})).unwrap();
    assert_eq!(c.workspace, state);
    assert_eq!(snapshot["baseRevision"], state["revision"]);
    assert!(crate::workspace::document_text(&snapshot["document"]).contains("Pending page text"));
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert!(
        !crate::workspace::document_text(&c.workspace["document"]).contains("Pending page text")
    );
}

#[test]
fn page_preview_fixture_contains_physical_sections_and_page_fields() {
    let mut p = fixture();
    let paragraphs = (0..36).map(|i|format!("<w:p><w:r><w:t>Paragraph {i:02}. This text flows across physical pages.</w:t></w:r></w:p>")).collect::<String>();
    let page = "<w:pgSz w:w=\"6120\" w:h=\"7920\"/><w:pgMar w:top=\"600\" w:right=\"600\" w:bottom=\"600\" w:left=\"600\" w:header=\"240\" w:footer=\"240\" w:gutter=\"0\"/>";
    let landscape = "<w:pgSz w:w=\"7920\" w:h=\"6120\" w:orient=\"landscape\"/><w:pgMar w:top=\"600\" w:right=\"600\" w:bottom=\"600\" w:left=\"600\" w:header=\"240\" w:footer=\"240\" w:gutter=\"0\"/>";
    p.set("word/document.xml", format!("<w:document xmlns:w=\"{W}\" xmlns:r=\"{R}\"><w:body>{paragraphs}<w:p><w:pPr><w:sectPr><w:headerReference w:type=\"default\" r:id=\"head\"/><w:footerReference w:type=\"default\" r:id=\"foot\"/>{page}</w:sectPr></w:pPr><w:r><w:t>End of section one.</w:t></w:r></w:p><w:p><w:r><w:t>Landscape section starts on an odd page.</w:t></w:r></w:p><w:sectPr><w:type w:val=\"oddPage\"/>{landscape}<w:pgNumType w:fmt=\"lowerRoman\" w:start=\"5\"/></w:sectPr></w:body></w:document>"));
    p.set("word/footer1.xml", format!("<w:ftr xmlns:w=\"{W}\"><w:p><w:r><w:t xml:space=\"preserve\">Page </w:t></w:r><w:fldSimple w:instr=\" PAGE \" w:dirty=\"1\"><w:r><w:t>99</w:t></w:r></w:fldSimple><w:r><w:t xml:space=\"preserve\"> of </w:t></w:r><w:fldSimple w:instr=\" NUMPAGES \" w:dirty=\"1\"><w:r><w:t>99</w:t></w:r></w:fldSimple></w:p></w:ftr>"));
    let snapshot = headers::page_preview(&p).unwrap();
    assert_eq!(
        snapshot["sections"][1]["geometry"]["orientation"],
        "landscape"
    );
    assert_eq!(
        snapshot["sections"][1]["footer"]["default"]["content"][0]["content"][1]["type"],
        "pageField"
    );
    let folder = std::env::temp_dir();
    std::fs::write(folder.join("folio-page-preview.docx"), p.bytes().unwrap()).unwrap();
    std::fs::write(
        folder.join("folio-page-preview.json"),
        serde_json::to_vec(&snapshot).unwrap(),
    )
    .unwrap();
}

#[test]
fn page_preview_retains_manual_break_types_and_flow_flags_on_roundtrip() {
    let mut p = fixture();
    p.set("word/document.xml", format!("<w:document xmlns:w=\"{W}\"><w:body><w:p><w:pPr><w:keepNext/><w:keepLines/><w:widowControl w:val=\"0\"/><w:pageBreakBefore/></w:pPr><w:r><w:t>Before</w:t><w:br w:type=\"page\"/><w:t>After</w:t><w:br w:type=\"column\"/><w:t>Column</w:t><w:br w:clear=\"all\"/><w:t>Last</w:t></w:r></w:p></w:body></w:document>"));
    let (mut doc, _) = crate::docx::import(&p).unwrap();
    let baseline = doc.clone();
    assert_eq!(
        doc["content"][0]["content"][1]["attrs"]["breakType"],
        "page"
    );
    assert_eq!(
        doc["content"][0]["content"][3]["attrs"]["breakType"],
        "column"
    );
    assert_eq!(doc["content"][0]["content"][5]["attrs"]["clear"], "all");
    let flags = &doc["content"][0]["attrs"]["paragraphFormat"]["direct"];
    assert_eq!(flags["keepNext"], true);
    assert_eq!(flags["keepLines"], true);
    assert_eq!(flags["widowControl"], false);
    assert_eq!(flags["pageBreakBefore"], true);
    doc["content"][0]["content"][0]["text"] = json!("Edited before");
    let exported = crate::docx::export_review(&doc, Some(&p), Some(&baseline)).unwrap();
    let imported = crate::docx::import(&exported).unwrap().0;
    assert_eq!(imported["content"][0], doc["content"][0]);
}
