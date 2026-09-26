//! Captions, cross-references and tables of contents (W10.03–W10.05, W10.08).
use crate::docx_references::{update, Pages, MISSING};
use crate::{docx, office::*};
use serde_json::{json, Value};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn field(code: &str, text: &str) -> Value {
    let kind = code.split_whitespace().next().unwrap();
    json!({"type":"pageField","text":text,"attrs":{"kind":kind,"instruction":code,"locked":false,"dirty":true}})
}

fn text(t: &str) -> Value {
    json!({"type":"text","text":t})
}

fn fields(doc: &Value) -> Vec<String> {
    let mut out = vec![];
    fn walk(n: &Value, out: &mut Vec<String>) {
        if n["type"] == "pageField" {
            out.push(n["text"].as_str().unwrap().to_string());
        }
        for c in n["content"].as_array().into_iter().flatten() {
            walk(c, out);
        }
    }
    walk(doc, &mut out);
    out
}

#[test]
fn seq_fields_number_captions_in_their_formats_and_resets() {
    let mut doc = json!({"type":"doc","content":[
        {"type":"paragraph","content":[text("Figure "), field(" SEQ Figure \\* ARABIC ", "9")]},
        {"type":"paragraph","content":[text("Table "), field(" SEQ Table \\* ROMAN ", "?")]},
        {"type":"paragraph","content":[text("Figure "), field(" SEQ Figure ", "?")]},
        {"type":"paragraph","content":[text("Again "), field(" SEQ Figure \\c ", "?")]},
        {"type":"heading","attrs":{"level":1},"content":[text("Chapter")]},
        {"type":"paragraph","content":[text("Eq "), field(" SEQ Eq \\s 1 ", "?"), field(" SEQ Eq ", "?")]},
        {"type":"heading","attrs":{"level":1},"content":[text("Next")]},
        {"type":"paragraph","content":[field(" SEQ Eq ", "?"), field(" SEQ Table \\r 5 \\* alphabetic ", "?"), field(" SEQ Figure \\h ", "?")]},
    ]});
    assert!(update(&mut doc, &Pages::default()));
    assert_eq!(fields(&doc), ["1", "I", "2", "2", "1", "2", "1", "e", ""]);
    // Updating again changes nothing.
    assert!(!update(&mut doc, &Pages::default()));
    let mut count = 0;
    crate::workspace::document_validate(&doc, 0, &mut count).unwrap();
}

#[test]
fn cross_references_show_their_target_or_word_s_error() {
    let mut doc = json!({"type":"doc","attrs":{"bookmarks":[{"name":"_Ref1","start":0,"end":8}]},"content":[
        {"type":"paragraph","content":[text("Figure "), field(" SEQ Figure ", "?"), text(": A chart")]},
        {"type":"paragraph","content":[text("See "), field(" REF _Ref1 \\h ", "old"), text(" on page "), field(" PAGEREF _Ref1 \\h ", "?"), text(".")]},
        {"type":"paragraph","content":[field(" REF _Gone \\h ", "Figure 3"), field(" PAGEREF _Gone ", "4")]},
    ]});
    // "Figure ?" is 8 characters before the update; the field becomes "1",
    // and the bookmark moves with the text in the engine's rebase. Here the
    // update reads the bookmark over the numbered text.
    let mut pages = Pages::default();
    pages.bookmarks.insert("_ref1".into(), "3".into());
    update(&mut doc, &pages);
    let values = fields(&doc);
    assert_eq!(values[0], "1");
    assert!(values[1].starts_with("Figure 1"), "{}", values[1]);
    assert_eq!(values[2], "3");
    assert_eq!(values[3], MISSING);
    assert_eq!(values[4], MISSING);
    // A locked field keeps its result.
    let mut locked = json!({"type":"doc","content":[{"type":"paragraph","content":[field(" REF _Gone ", "kept")]}]});
    locked["content"][0]["content"][0]["attrs"]["locked"] = json!(true);
    assert!(!update(&mut locked, &Pages::default()));
}

#[test]
fn a_table_of_contents_lists_headings_and_round_trips_through_word() {
    let mut doc = json!({"type":"doc","content":[
        {"type":"tableOfContents","attrs":{"levels":2,"entries":[]}},
        {"type":"heading","attrs":{"level":1},"content":[text("Introduction")]},
        {"type":"paragraph","content":[text("Body")]},
        {"type":"heading","attrs":{"level":2},"content":[text("Scope & aims")]},
        {"type":"heading","attrs":{"level":3},"content":[text("Too deep")]},
        {"type":"heading","attrs":{"level":1},"content":[text("Results")]},
    ]});
    let pages = Pages {
        headings: vec!["1".into(), "1".into(), "2".into(), "3".into()],
        ..Default::default()
    };
    assert!(update(&mut doc, &pages));
    assert_eq!(
        doc["content"][0]["attrs"]["entries"],
        json!([
            {"text":"Introduction","level":1,"page":"1"},
            {"text":"Scope & aims","level":2,"page":"1"},
            {"text":"Results","level":1,"page":"3"},
        ])
    );
    // Without layout, known pages stay.
    let mut again = doc.clone();
    assert!(!update(&mut again, &Pages::default()));

    let out = Package::from_bytes(docx::export(&doc, None, None).unwrap()).unwrap();
    let xml = out.text("word/document.xml").unwrap();
    assert!(xml.contains("<w:docPartGallery w:val=\"Table of Contents\"/>"));
    assert!(xml.contains(" TOC \\o \"1-2\" \\u "));
    assert!(xml.contains("Scope &amp; aims"));
    assert!(xml.contains("<w:pStyle w:val=\"TOC2\"/>"));
    let (reopened, _) = docx::import(&out).unwrap();
    let toc = &reopened["content"][0];
    assert_eq!(toc["type"], "tableOfContents");
    assert_eq!(toc["attrs"]["levels"], 2);
    assert_eq!(
        toc["attrs"]["entries"],
        doc["content"][0]["attrs"]["entries"]
    );
    // Unchanged, it is written from its source.
    let out2 =
        Package::from_bytes(docx::export(&reopened, Some(&out), Some(&reopened)).unwrap()).unwrap();
    assert_eq!(out2.original, out.original);
}

#[test]
fn word_fields_for_captions_and_references_import_as_editable_fields() {
    let empty = json!({"type":"doc","content":[{"type":"paragraph"}]});
    let mut p = Package::from_bytes(docx::export(&empty, None, None).unwrap()).unwrap();
    p.set(
        "word/document.xml",
        format!(
            "<w:document xmlns:w=\"{W}\"><w:body>\
             <w:p><w:pPr><w:pStyle w:val=\"Caption\"/></w:pPr><w:bookmarkStart w:id=\"0\" w:name=\"_Ref9\"/><w:r><w:t xml:space=\"preserve\">Figure </w:t></w:r><w:fldSimple w:instr=\" SEQ Figure \\* ARABIC \"><w:r><w:t>1</w:t></w:r></w:fldSimple><w:bookmarkEnd w:id=\"0\"/><w:r><w:t>: Map</w:t></w:r></w:p>\
             <w:p><w:r><w:t xml:space=\"preserve\">As </w:t></w:r><w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> REF _Ref9 \\h </w:instrText></w:r><w:r><w:fldChar w:fldCharType=\"separate\"/></w:r><w:r><w:t>Figure 1</w:t></w:r><w:r><w:fldChar w:fldCharType=\"end\"/></w:r><w:r><w:t xml:space=\"preserve\"> shows.</w:t></w:r></w:p>\
             <w:sectPr/></w:body></w:document>"
        ),
    );
    let pkg = Package::from_bytes(p.bytes().unwrap()).unwrap();
    let (doc, _) = docx::import(&pkg).unwrap();
    assert_eq!(doc["content"][0]["type"], "paragraph");
    assert_eq!(doc["content"][1]["type"], "paragraph");
    let kinds: Vec<Value> = [
        &doc["content"][0]["content"][1],
        &doc["content"][1]["content"][1],
    ]
    .iter()
    .map(|n| n["attrs"]["kind"].clone())
    .collect();
    assert_eq!(kinds, [json!("SEQ"), json!("REF")]);
    // Unchanged, they are written from their source.
    let out = docx::export(&doc, Some(&pkg), Some(&doc)).unwrap();
    assert_eq!(out, pkg.original);
    // An update that changes nothing keeps them as they are.
    let mut updated = doc.clone();
    assert!(!update(&mut updated, &Pages::default()));
}

#[test]
fn update_fields_in_the_workspace_uses_layout_pages_and_declares_toc_styles() {
    let mut c = crate::workspace::Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let doc = json!({"type":"doc","content":[
        {"type":"tableOfContents","attrs":{"levels":3,"entries":[]}},
        {"type":"heading","attrs":{"level":1},"content":[text("One")]},
        {"type":"paragraph","content":[text("Figure "), field(" SEQ Figure ", "?")]},
        {"type":"heading","attrs":{"level":2},"content":[text("Two")]},
    ]});
    c.dispatch(json!({"op":"document","document":doc})).unwrap();
    let revision = c.workspace["revision"].clone();
    // The preview's blocks: the TOC is the first block, then the headings.
    c.dispatch(
        json!({"op":"documentUpdateFields","blockPages":["1","2","2","3"],"baseRevision":revision}),
    )
    .unwrap();
    let document = &c.workspace["document"];
    assert_eq!(fields(document), ["1"]);
    let entries = &document["content"][0]["attrs"]["entries"];
    assert_eq!(entries[0], json!({"text":"One","level":1,"page":"2"}));
    assert_eq!(entries[1]["page"], "3");
    // One undo step takes the whole update back.
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        c.workspace["document"]["content"][0]["attrs"]["entries"],
        json!([])
    );
    c.dispatch(json!({"op":"redo"})).unwrap();

    let path = std::env::temp_dir().join(format!("folio-toc-{}.docx", std::process::id()));
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":path}))
        .unwrap();
    let out = Package::read(path.to_str().unwrap()).unwrap();
    let styles = out.text("word/styles.xml").unwrap();
    assert!(styles.contains("w:styleId=\"TOC1\"") && styles.contains("w:styleId=\"TOC2\""));
    std::fs::remove_file(&path).ok();
    // A stale revision is refused.
    assert!(c
        .dispatch(json!({"op":"documentUpdateFields","baseRevision":-1}))
        .is_err());
}

fn core_with(doc: Value) -> crate::workspace::Core {
    let mut c = crate::workspace::Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c.dispatch(json!({"op":"document","document":doc})).unwrap();
    c
}

fn references(c: &mut crate::workspace::Core, mut request: Value) -> Result<Value, String> {
    request["op"] = json!("documentReferences");
    request["baseRevision"] = c.workspace["revision"].clone();
    c.dispatch(request)
}

#[test]
fn captions_insert_numbered_and_renumber_the_rest() {
    let mut c = core_with(json!({"type":"doc","content":[
        {"type":"paragraph","content":[text("First picture")]},
        {"type":"paragraph","content":[text("Figure "), field(" SEQ Figure ", "1"), text(": Later")]},
    ]}));
    // After "First picture" (offset 3 is inside it).
    references(
        &mut c,
        json!({"action":"caption","offset":3,"label":"Figure","text":"Early"}),
    )
    .unwrap();
    let doc = &c.workspace["document"];
    assert_eq!(
        crate::workspace::document_text(&doc["content"][1]),
        "Figure 1: Early"
    );
    assert_eq!(
        doc["content"][1]["attrs"]["paragraphFormat"]["styleId"],
        "Caption"
    );
    assert_eq!(
        crate::workspace::document_text(&doc["content"][2]),
        "Figure 2: Later"
    );
    // One undo step.
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        c.workspace["document"]["content"].as_array().unwrap().len(),
        2
    );
    assert!(references(
        &mut c,
        json!({"action":"caption","offset":3,"label":"Two words"})
    )
    .is_err());
}

#[test]
fn cross_references_bookmark_their_target_and_follow_it() {
    let mut c = core_with(json!({"type":"doc","content":[
        {"type":"heading","attrs":{"level":1},"content":[text("Method")]},
        {"type":"paragraph","content":[text("See .")]},
        {"type":"paragraph","content":[text("Figure "), field(" SEQ Figure ", "1"), text(": Map")]},
    ]}));
    let targets = references(&mut c, json!({"action":"targets"})).unwrap();
    assert_eq!(targets["headings"][0]["text"], "Method");
    assert_eq!(targets["captions"][0]["index"], 2);
    // "Method\n" is 7 units; "See " ends at 11.
    references(
        &mut c,
        json!({"action":"crossReference","offset":11,"target":{"kind":"heading","index":0}}),
    )
    .unwrap();
    let doc = c.workspace["document"].clone();
    assert_eq!(
        crate::workspace::document_text(&doc["content"][1]),
        "See Method."
    );
    let bookmark = &doc["attrs"]["bookmarks"][0];
    assert!(bookmark["name"].as_str().unwrap().starts_with("_Ref"));
    assert_eq!(
        (bookmark["start"].as_u64(), bookmark["end"].as_u64()),
        (Some(0), Some(6))
    );
    // A caption reference shows its label and number.
    let end = 7 + crate::workspace::document_text(&doc["content"][1]).len() - 1;
    references(
        &mut c,
        json!({"action":"crossReference","offset":end,"target":{"kind":"caption","index":2}}),
    )
    .unwrap();
    assert_eq!(
        crate::workspace::document_text(&c.workspace["document"]["content"][1]),
        "See MethodFigure 1."
    );
    // Editing the heading, then updating fields, follows it. (Text typed at
    // the start of a bookmark is inside it, as in Word.)
    let mut edited = c.workspace["document"].clone();
    edited["content"][0]["content"][0]["text"] = json!("Our Method");
    c.dispatch(json!({"op":"document","document":edited}))
        .unwrap();
    c.dispatch(json!({"op":"documentUpdateFields","baseRevision":c.workspace["revision"]}))
        .unwrap();
    assert_eq!(
        crate::workspace::document_text(&c.workspace["document"]["content"][1]),
        "See Our MethodFigure 1."
    );
    // Deleting the heading leaves Word's error on update.
    let mut gone = c.workspace["document"].clone();
    gone["content"].as_array_mut().unwrap().remove(0);
    c.dispatch(json!({"op":"document","document":gone}))
        .unwrap();
    c.dispatch(json!({"op":"documentUpdateFields","baseRevision":c.workspace["revision"]}))
        .unwrap();
    assert!(
        crate::workspace::document_text(&c.workspace["document"]["content"][0]).contains(MISSING)
    );

    // Export keeps the field and its bookmark.
    let path = std::env::temp_dir().join(format!("folio-xref-{}.docx", std::process::id()));
    c.dispatch(json!({"op":"undo"})).unwrap();
    c.dispatch(json!({"op":"undo"})).unwrap();
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":path}))
        .unwrap();
    let out = Package::read(path.to_str().unwrap()).unwrap();
    let xml = out.text("word/document.xml").unwrap();
    assert!(xml.contains("REF _Ref"));
    assert!(xml.contains("w:name=\"_Ref"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn a_table_of_contents_is_inserted_before_the_cursor_s_block() {
    let mut c = core_with(json!({"type":"doc","content":[
        {"type":"paragraph","content":[text("Intro")]},
        {"type":"heading","attrs":{"level":1},"content":[text("Part one")]},
    ]}));
    references(&mut c, json!({"action":"toc","offset":0,"levels":2})).unwrap();
    let toc = &c.workspace["document"]["content"][0];
    assert_eq!(toc["type"], "tableOfContents");
    assert_eq!(
        toc["attrs"]["entries"],
        json!([{"text":"Part one","level":1,"page":""}])
    );
}
