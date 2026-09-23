use crate::{document_bookmarks as bm, document_comments as c, docx, office::*, workspace::Core};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn doc(text: &str) -> Value {
    json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":text}]}]})
}
fn add(d: &Value, start: usize, end: usize) -> Value {
    c::command(d,&json!({"action":"add","author":"Reviewer","text":"Please check this.\nA second line.","start":start,"end":end})).unwrap()
}
fn action(d: &Value, action: &str, more: Value) -> Value {
    let mut r = more;
    r["action"] = json!(action);
    r["id"] = c::entries(d)[0]["id"].clone();
    c::command(d, &r).unwrap()
}
#[test]
fn comment_commands_replies_resolution_edits_and_validation() {
    let d = add(&doc("A😀 target"), 4, 10);
    assert!(c::command(
        &d,
        &json!({"action":"add","author":"R","text":"a","start":2,"end":3})
    )
    .is_err());
    assert!(c::command(
        &d,
        &json!({"action":"add","author":" ","text":"a","start":0,"end":0})
    )
    .is_err());
    let d = action(
        &d,
        "reply",
        json!({"author":"Second reviewer","text":"Agreed"}),
    );
    let mid = c::entries(&d)[0]["messages"][1]["id"].clone();
    let d = action(&d, "resolve", json!({}));
    assert!(c::command(
        &d,
        &json!({"action":"reply","id":c::entries(&d)[0]["id"],"author":"R","text":"No"})
    )
    .is_err());
    let d = action(&d, "reopen", json!({}));
    let d = action(&d, "edit", json!({"messageId":mid,"text":"Confirmed"}));
    assert_eq!(
        c::entries(&d)[0]["messages"][1]["author"],
        "Second reviewer"
    );
    let d = action(&d, "deleteReply", json!({"messageId":mid}));
    assert_eq!(c::entries(&d)[0]["messages"].as_array().unwrap().len(), 1);
    let d = action(&d, "delete", json!({}));
    assert!(c::entries(&d).is_empty());
}
#[test]
fn comment_ranges_exact_edits_metadata_injection_and_history() {
    let mut core = Core::new();
    core.dispatch(json!({"op":"document","document":doc("aaaaaa")}))
        .unwrap();
    let r = json!({"op":"documentComment","workspaceId":core.workspace["id"],"baseRevision":core.workspace["revision"],"action":"add","author":"Reviewer","text":"Check","start":2,"end":4});
    core.dispatch(r.clone()).unwrap();
    let before = core.workspace.clone();
    assert!(core.dispatch(r).is_err());
    assert_eq!(before, core.workspace);
    let mut next = doc("aaZaaaa");
    next["attrs"]["comments"] = json!([]);
    core.dispatch(
        json!({"op":"document","document":next,"textEdits":[{"start":2,"end":2,"text":"Z"}]}),
    )
    .unwrap();
    assert_eq!(c::entries(&core.workspace["document"])[0]["end"], 5);
    core.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(core.workspace["document"], before["document"]);
    core.dispatch(json!({"op":"redo"})).unwrap();
    core.dispatch(
        json!({"op":"document","document":doc("aaaa"),"textEdits":[{"start":2,"end":5,"text":""}]}),
    )
    .unwrap();
    assert!(c::entries(&core.workspace["document"]).is_empty());
    core.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c::entries(&core.workspace["document"]).len(), 1);
    let path = std::env::temp_dir().join(format!("folio-comments-{}.folio", std::process::id()));
    core.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut reopened = Core::new();
    reopened.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(reopened.workspace["document"], core.workspace["document"]);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn word_comments_cross_paragraph_table_unicode_bookmarks_replies_roundtrip() {
    let mut d = doc("A😀 target");
    d["content"].as_array_mut().unwrap().push(json!({"type":"table","content":[{"type":"tableRow","content":[{"type":"tableCell","content":[{"type":"paragraph","content":[{"type":"text","text":"Cell text"}]}]}]}]}));
    let d = add(&d, 4, 15);
    let d = action(
        &d,
        "reply",
        json!({"author":"Alex","text":"Verified & checked <all>"}),
    );
    let d = action(&d, "resolve", json!({}));
    let d = bm::command(
        &d,
        &json!({"action":"set","name":"Target","start":4,"end":10}),
    )
    .unwrap();
    let p = Package::from_bytes(docx::export(&d, None, None).unwrap()).unwrap();
    let round = docx::import(&p).unwrap().0;
    let t = &c::entries(&round)[0];
    assert_eq!(t["start"], 4);
    assert_eq!(t["end"], 15);
    assert_eq!(t["resolved"], true);
    assert_eq!(t["messages"][1]["text"], "Verified & checked <all>");
    assert_eq!(bm::entries(&round).len(), 1);
    assert_eq!(
        docx::export(&round, Some(&p), Some(&round)).unwrap(),
        p.original
    );
    let updated = action(&round, "reopen", json!({}));
    let bytes = docx::export(&updated, Some(&p), Some(&round)).unwrap();
    let updated = docx::import(&Package::from_bytes(bytes).unwrap())
        .unwrap()
        .0;
    assert_eq!(c::entries(&updated)[0]["resolved"], false);
    let out = std::env::temp_dir().join("folio-comments-independent.docx");
    std::fs::write(&out, &p.original).unwrap();
}
#[test]
fn word_comments_preserve_rich_source_runs_and_remove_deleted_threads() {
    let base = add(&doc("before target after"), 7, 13);
    let mut p = Package::from_bytes(docx::export(&base, None, None).unwrap()).unwrap();
    let xml = p
        .text("word/comments.xml")
        .unwrap()
        .replace("<w:r>", "<w:r><w:rPr><w:b/></w:rPr>");
    p.set("word/comments.xml", xml);
    let xml = p
        .text("word/document.xml")
        .unwrap()
        .replace("<w:r>", "<w:r><w:rPr><w:smallCaps/></w:rPr>");
    p.set("word/document.xml", xml);
    p.set("custom/preserved.xml", "<unknown>kept</unknown>".into());
    p = Package::from_bytes(p.bytes().unwrap()).unwrap();
    let d = docx::import(&p).unwrap().0;
    assert_eq!(c::entries(&d)[0]["messages"][0]["readOnly"], true);
    let edited = action(&d, "reply", json!({"author":"R","text":"A reply"}));
    let out = Package::from_bytes(docx::export(&edited, Some(&p), Some(&d)).unwrap()).unwrap();
    assert!(out.text("word/comments.xml").unwrap().contains("<w:b/>"));
    assert!(out
        .text("word/document.xml")
        .unwrap()
        .contains("<w:smallCaps/>"));
    assert_eq!(
        out.entries["custom/preserved.xml"],
        p.entries["custom/preserved.xml"]
    );
    let deleted = action(&d, "delete", json!({}));
    let out = Package::from_bytes(docx::export(&deleted, Some(&p), Some(&d)).unwrap()).unwrap();
    assert!(c::entries(&docx::import(&out).unwrap().0).is_empty());
    assert!(!out
        .text("word/document.xml")
        .unwrap()
        .contains("<w:comment"));
    assert!(!out.text("word/comments.xml").unwrap().contains("w:id="));
}
#[test]
fn unmanaged_comment_anchor_cannot_silently_disappear() {
    let mut p = Package::from_bytes(docx::export(&doc("target"), None, None).unwrap()).unwrap();
    p.set("word/document.xml",format!("<w:document xmlns:w=\"{W}\"><w:body><w:p><w:commentRangeStart w:id=\"99\"/><w:r><w:t>target</w:t></w:r><w:commentRangeEnd w:id=\"99\"/><w:r><w:commentReference w:id=\"99\"/></w:r></w:p></w:body></w:document>"));
    p = Package::from_bytes(p.bytes().unwrap()).unwrap();
    let d = docx::import(&p).unwrap().0;
    let mut changed = d.clone();
    changed["content"][0]["content"][0]["text"] = json!("Changed");
    assert!(docx::export(&changed, Some(&p), Some(&d))
        .unwrap_err()
        .contains("preserved Word comment"));
    let added = add(&d, 0, 2);
    let out = Package::from_bytes(docx::export(&added, Some(&p), Some(&d)).unwrap()).unwrap();
    assert!(out
        .text("word/document.xml")
        .unwrap()
        .contains("w:id=\"99\""));
}

#[test]
fn word_comment_nonstandard_parts_prefixes_extensions_and_reply_deletion() {
    let d = add(&doc("before target after"), 7, 13);
    let d = action(&d, "reply", json!({"author":"Reply author","text":"Reply"}));
    let mut p = Package::from_bytes(docx::export(&d, None, None).unwrap()).unwrap();
    let comment_xml = p
        .text("word/comments.xml")
        .unwrap()
        .replace("w:", "z:")
        .replace("xmlns:w=", "xmlns:z=");
    p.entries.remove("word/comments.xml");
    p.set("word/review/commentsA.xml", comment_xml);
    let mut rels = p
        .text("word/_rels/document.xml.rels")
        .unwrap()
        .replace("/word/comments.xml", "review/commentsA.xml");
    rels=rels.replace("</Relationships>","<Relationship Id=\"ids\" Type=\"http://schemas.microsoft.com/office/2016/09/relationships/commentsIds\" Target=\"review/ids.xml\"/><Relationship Id=\"cex\" Type=\"http://schemas.microsoft.com/office/2018/08/relationships/commentsExtensible\" Target=\"review/extra.xml\"/></Relationships>");
    p.set("word/_rels/document.xml.rels", rels);
    let ct = p
        .text("[Content_Types].xml")
        .unwrap()
        .replace("/word/comments.xml", "/word/review/commentsA.xml");
    p.set("[Content_Types].xml", ct);
    let ex = p.text("word/commentsExtended.xml").unwrap().replace(
        "w15:done=\"0\"",
        "w15:done=\"0\" xmlns:custom=\"urn:custom\" custom:flag=\"retain me\"",
    );
    p.set("word/commentsExtended.xml", ex);
    let before = docx::import(&p).unwrap().0;
    let root_para = c::entries(&before)[0]["messages"][0]["paraId"]
        .as_str()
        .unwrap();
    let reply_para = c::entries(&before)[0]["messages"][1]["paraId"]
        .as_str()
        .unwrap();
    p.set("word/review/ids.xml",format!("<c:commentsIds xmlns:c=\"http://schemas.microsoft.com/office/word/2016/wordml/cid\"><c:commentId c:paraId=\"{root_para}\" c:durableId=\"00000001\"/><c:commentId c:paraId=\"{reply_para}\" c:durableId=\"00000002\"/><c:commentId c:paraId=\"76543210\" c:durableId=\"00000003\"/></c:commentsIds>"));
    p.set("word/review/extra.xml","<c:commentsExtensible xmlns:c=\"http://schemas.microsoft.com/office/word/2018/wordml/cex\"><c:commentExtensible c:durableId=\"00000001\"/><c:commentExtensible c:durableId=\"00000002\"/><c:commentExtensible c:durableId=\"00000003\"/></c:commentsExtensible>".into());
    p = Package::from_bytes(p.bytes().unwrap()).unwrap();
    let resolved = action(&before, "resolve", json!({}));
    let out =
        Package::from_bytes(docx::export(&resolved, Some(&p), Some(&before)).unwrap()).unwrap();
    assert!(!out.entries.contains_key("word/comments.xml"));
    assert_eq!(
        c::entries(&docx::import(&out).unwrap().0)[0]["resolved"],
        true
    );
    assert!(out
        .text("word/commentsExtended.xml")
        .unwrap()
        .contains("retain me"));
    assert_eq!(
        out.entries["word/review/ids.xml"],
        p.entries["word/review/ids.xml"]
    );
    let deleted = action(
        &before,
        "deleteReply",
        json!({"messageId":c::entries(&before)[0]["messages"][1]["id"]}),
    );
    let out =
        Package::from_bytes(docx::export(&deleted, Some(&p), Some(&before)).unwrap()).unwrap();
    assert_eq!(
        c::entries(&docx::import(&out).unwrap().0)[0]["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(!out
        .text("word/review/ids.xml")
        .unwrap()
        .contains("00000002"));
    assert!(!out
        .text("word/review/extra.xml")
        .unwrap()
        .contains("00000002"));
    assert!(out
        .text("word/review/extra.xml")
        .unwrap()
        .contains("00000003"));
}

#[test]
fn comment_legacy_hydration_and_ai_replace_are_explicit() {
    let source = add(&doc("before target after"), 7, 13);
    let mut legacy = doc("New before target after");
    c::hydrate(&mut legacy, &source).unwrap();
    assert_eq!(c::entries(&legacy)[0]["start"], 11);
    let mut explicit = doc("New before target after");
    explicit["attrs"]["comments"] = json!([]);
    c::hydrate(&mut explicit, &source).unwrap();
    assert!(c::entries(&explicit).is_empty());
    let mut core = Core::new();
    core.dispatch(json!({"op":"document","document":doc("before target after")}))
        .unwrap();
    core.dispatch(json!({"op":"documentComment","action":"add","workspaceId":core.workspace["id"],"baseRevision":core.workspace["revision"],"author":"R","text":"Check","start":7,"end":13})).unwrap();
    let proposal=core.dispatch(json!({"op":"propose","proposal":{"summary":"Replace","baseRevision":core.workspace["revision"],"replaceDocument":["before target after"]}})).unwrap();
    core.dispatch(json!({"op":"apply","proposal":proposal}))
        .unwrap();
    assert!(c::entries(&core.workspace["document"]).is_empty());
    core.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c::entries(&core.workspace["document"]).len(), 1);
}

#[test]
fn editing_plain_imported_comment_keeps_attribution_style_and_unknown_attributes() {
    let d = add(&doc("target"), 0, 6);
    let mut p = Package::from_bytes(docx::export(&d, None, None).unwrap()).unwrap();
    let xml=p.text("word/comments.xml").unwrap().replace("w:author=", "xmlns:custom=\"urn:test\" custom:origin=\"retained\" w:author=").replace("<w:r>","<w:pPr><w:pStyle w:val=\"CommentText\"/></w:pPr><w:r><w:rPr><w:rStyle w:val=\"CommentTextChar\"/></w:rPr>");
    p.set("word/comments.xml", xml);
    p = Package::from_bytes(p.bytes().unwrap()).unwrap();
    let d = docx::import(&p).unwrap().0;
    assert_eq!(c::entries(&d)[0]["messages"][0]["readOnly"], false);
    let edited = action(
        &d,
        "edit",
        json!({"messageId":c::entries(&d)[0]["messages"][0]["id"],"text":"New text\nAnd a new second line\nThird"}),
    );
    let out = Package::from_bytes(docx::export(&edited, Some(&p), Some(&d)).unwrap()).unwrap();
    let xml = out.text("word/comments.xml").unwrap();
    assert!(xml.contains("custom:origin=\"retained\""));
    assert!(xml.contains("CommentTextChar"));
    assert!(xml.contains("CommentText\""));
    let round = docx::import(&out).unwrap().0;
    assert_eq!(
        c::entries(&round)[0]["messages"][0]["text"],
        "New text\nAnd a new second line\nThird"
    );
    assert_eq!(c::entries(&round)[0]["messages"][0]["author"], "Reviewer");
}
