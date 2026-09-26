//! Footnotes and endnotes (W11.01).
use crate::{docx, office::*};
use serde_json::{json, Value};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// A Word package with two footnotes and an endnote.
fn package() -> Package {
    let empty = json!({"type":"doc","content":[{"type":"paragraph"}]});
    let mut p = Package::from_bytes(docx::export(&empty, None, None).unwrap()).unwrap();
    p.set(
        "word/document.xml",
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"{W}\"><w:body>\
             <w:p><w:r><w:t>Claim</w:t></w:r><w:r><w:rPr><w:rStyle w:val=\"FootnoteReference\"/></w:rPr><w:footnoteReference w:id=\"1\"/></w:r><w:r><w:t> and more</w:t></w:r><w:r><w:rPr><w:vertAlign w:val=\"superscript\"/></w:rPr><w:footnoteReference w:id=\"2\"/></w:r></w:p>\
             <w:p><w:r><w:t>End</w:t></w:r><w:r><w:endnoteReference w:id=\"1\"/></w:r></w:p>\
             <w:sectPr/></w:body></w:document>"
        ),
    );
    p.set(
        "word/footnotes.xml",
        format!(
            "<w:footnotes xmlns:w=\"{W}\"><w:footnote w:type=\"separator\" w:id=\"-1\"><w:p><w:r><w:separator/></w:r></w:p></w:footnote>\
             <w:footnote w:id=\"1\"><w:p><w:pPr><w:pStyle w:val=\"FootnoteText\"/></w:pPr><w:r><w:footnoteRef/></w:r><w:r><w:t xml:space=\"preserve\"> Source one.</w:t></w:r></w:p></w:footnote>\
             <w:footnote w:id=\"2\"><w:p><w:r><w:footnoteRef/></w:r><w:r><w:rPr><w:i/></w:rPr><w:t xml:space=\"preserve\"> Italic source.</w:t></w:r></w:p><w:p><w:r><w:t>Second line</w:t></w:r></w:p></w:footnote></w:footnotes>"
        ),
    );
    p.set(
        "word/endnotes.xml",
        format!(
            "<w:endnotes xmlns:w=\"{W}\"><w:endnote w:id=\"1\"><w:p><w:r><w:endnoteRef/></w:r><w:r><w:t xml:space=\"preserve\"> The end.</w:t></w:r></w:p></w:endnote></w:endnotes>"
        ),
    );
    Package::from_bytes(p.bytes().unwrap()).unwrap()
}

fn notes(doc: &Value) -> Vec<Value> {
    let mut out = vec![];
    fn walk(n: &Value, out: &mut Vec<Value>) {
        if n["type"] == "noteRef" {
            out.push(n["attrs"].clone());
        }
        for c in n["content"].as_array().into_iter().flatten() {
            walk(c, out);
        }
    }
    walk(doc, &mut out);
    out
}

fn notes_mut(doc: &mut Value, f: &mut impl FnMut(&mut Value)) {
    if doc["type"] == "noteRef" {
        f(doc);
    }
    for c in doc["content"].as_array_mut().into_iter().flatten() {
        notes_mut(c, f);
    }
}

#[test]
fn references_import_as_inline_notes_with_their_text() {
    let pkg = package();
    let (doc, report) = docx::import(&pkg).unwrap();
    let found = notes(&doc);
    assert_eq!(found.len(), 3);
    assert_eq!(
        (found[0]["kind"].as_str(), found[0]["id"].as_str()),
        (Some("footnote"), Some("1"))
    );
    assert_eq!(found[0]["text"], "Source one.");
    assert_eq!(found[1]["text"], "Italic source.\nSecond line");
    assert_eq!(found[2]["kind"], "endnote");
    assert_eq!(found[2]["text"], "The end.");
    // The paragraph stays editable, with its text around the reference.
    assert_eq!(doc["content"][0]["type"], "paragraph");
    assert_eq!(
        crate::workspace::document_text(&doc["content"][0]),
        "Claim and more"
    );
    assert!(!report
        .to_string()
        .contains("Footnotes and endnotes are preserved"));
    let mut count = 0;
    crate::workspace::document_validate(&doc, 0, &mut count).unwrap();
}

#[test]
fn editing_the_paragraph_or_a_note_keeps_every_reference() {
    let pkg = package();
    let (baseline, _) = docx::import(&pkg).unwrap();
    // Edit the paragraph's text: both references survive.
    let mut doc = baseline.clone();
    doc["content"][0]["content"][0]["text"] = json!("Edited claim");
    let out =
        Package::from_bytes(docx::export(&doc, Some(&pkg), Some(&baseline)).unwrap()).unwrap();
    let xml = out.text("word/document.xml").unwrap();
    assert!(xml.contains("Edited claim"));
    assert!(xml.contains("<w:footnoteReference w:id=\"1\"/>"));
    assert!(xml.contains("<w:footnoteReference w:id=\"2\"/>"));
    assert_eq!(
        out.text("word/footnotes.xml").unwrap(),
        pkg.text("word/footnotes.xml").unwrap()
    );

    // Edit a note: only that note is rewritten, keeping its paragraph style.
    let mut doc = baseline.clone();
    notes_mut(&mut doc, &mut |n| {
        if n["attrs"]["id"] == "1" && n["attrs"]["kind"] == "footnote" {
            n["attrs"]["text"] = json!("Revised source & more.\nNew line");
        }
    });
    let out =
        Package::from_bytes(docx::export(&doc, Some(&pkg), Some(&baseline)).unwrap()).unwrap();
    let foot = out.text("word/footnotes.xml").unwrap();
    assert!(foot.contains("Revised source &amp; more."));
    assert!(foot.contains("<w:pStyle w:val=\"FootnoteText\"/>"));
    assert!(
        foot.contains("Italic source."),
        "the other note is unchanged"
    );
    let (again, _) = docx::import(&out).unwrap();
    assert_eq!(notes(&again)[0]["text"], "Revised source & more.\nNew line");
    assert_eq!(notes(&again)[1]["text"], "Italic source.\nSecond line");
}

#[test]
fn new_notes_get_ids_and_a_notes_part_when_there_is_none() {
    let doc = json!({"type":"doc","content":[{"type":"paragraph","content":[
        {"type":"text","text":"New claim"},
        {"type":"noteRef","attrs":{"kind":"footnote","id":null,"text":"A new source."}},
        {"type":"text","text":" then"},
        {"type":"noteRef","attrs":{"kind":"endnote","id":null,"text":"Closing note"}},
        {"type":"noteRef","attrs":{"kind":"footnote","id":null,"text":"Another"}},
    ]}]});
    let mut count = 0;
    crate::workspace::document_validate(&doc, 0, &mut count).unwrap();
    let out = Package::from_bytes(docx::export(&doc, None, None).unwrap()).unwrap();
    let foot = out.text("word/footnotes.xml").unwrap();
    assert!(foot.contains("w:type=\"separator\""));
    assert!(foot.contains("<w:footnote w:id=\"1\">"));
    assert!(foot.contains("<w:footnote w:id=\"2\">"));
    assert!(out
        .text("word/endnotes.xml")
        .unwrap()
        .contains("Closing note"));
    let rels = out.text("word/_rels/document.xml.rels").unwrap();
    assert!(rels.contains("Target=\"footnotes.xml\"") && rels.contains("Target=\"endnotes.xml\""));
    let types = out.text("[Content_Types].xml").unwrap();
    assert!(types.contains("/word/footnotes.xml") && types.contains("/word/endnotes.xml"));
    let (again, _) = docx::import(&out).unwrap();
    let found = notes(&again);
    assert_eq!(
        found
            .iter()
            .map(|n| (
                n["kind"].as_str().unwrap(),
                n["id"].as_str().unwrap(),
                n["text"].as_str().unwrap()
            ))
            .collect::<Vec<_>>(),
        [
            ("footnote", "1", "A new source."),
            ("endnote", "1", "Closing note"),
            ("footnote", "2", "Another")
        ]
    );

    // Adding a note to a document that has notes continues the numbering.
    let pkg = package();
    let (baseline, _) = docx::import(&pkg).unwrap();
    let mut doc = baseline.clone();
    doc["content"][1]["content"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"noteRef","attrs":{"kind":"footnote","id":null,"text":"Third"}}));
    let out =
        Package::from_bytes(docx::export(&doc, Some(&pkg), Some(&baseline)).unwrap()).unwrap();
    assert!(out
        .text("word/footnotes.xml")
        .unwrap()
        .contains("<w:footnote w:id=\"3\">"));
}

#[test]
fn invalid_notes_are_refused() {
    let check = |attrs: Value| {
        let mut count = 0;
        crate::workspace::document_validate(&json!({"type":"noteRef","attrs":attrs}), 0, &mut count)
    };
    assert!(check(json!({"kind":"footnote","id":"1","text":"ok"})).is_ok());
    assert!(check(json!({"kind":"sidenote","id":"1","text":"x"})).is_err());
    assert!(check(json!({"kind":"footnote","id":"1\"/><w:x","text":"x"})).is_err());
    // A source run must be exactly the reference it claims.
    let run = |xml: &str| json!({"kind":"footnote","id":"1","text":"x","sourceXml":xml});
    assert!(check(run("<w:r><w:footnoteReference w:id=\"1\"/></w:r>")).is_ok());
    assert!(check(run("<w:r><w:footnoteReference w:id=\"2\"/></w:r>")).is_err());
    assert!(check(run(
        "<w:r><w:t>injected</w:t><w:footnoteReference w:id=\"1\"/></w:r>"
    ))
    .is_err());
    assert!(check(run("<w:r><w:endnoteReference w:id=\"1\"/></w:r>")).is_err());
}
