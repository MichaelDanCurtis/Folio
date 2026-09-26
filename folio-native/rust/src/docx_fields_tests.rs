use crate::{
    docx, docx_fields as fields, docx_headers as headers, docx_page_numbers as numbers, office::*,
};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
fn simple(code: &str, result: &str) -> String {
    format!("<w:fldSimple w:instr=\"{}\"><w:r><w:rPr><w:b/></w:rPr><w:t>{result}</w:t></w:r></w:fldSimple>",esc(code))
}
fn package(inner: &str) -> Package {
    let mut p = docx::new_package();
    p.set("word/document.xml",format!("<w:document xmlns:w=\"{W}\"><w:body><w:p>{inner}</w:p><w:sectPr/></w:body></w:document>"));
    p.original = p.bytes().unwrap();
    p
}
fn new_field(code: &str) -> Value {
    json!({"type":"pageField","text":"?","attrs":{"kind":code,"instruction":format!(" {code} "),"dirty":true,"locked":false}})
}
#[test]
fn page_fields_import_simple_complex_and_keep_source_while_editing_text() {
    let complex = r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText>NUM</w:instrText></w:r><w:r><w:instrText>PAGES \* Arabic</w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>123</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r>"#;
    let p=package(&format!("<w:bookmarkStart w:id=\"1\" w:name=\"Around\"/><w:r><w:t>Page </w:t></w:r>{}<w:r><w:t> of </w:t></w:r>{complex}<w:bookmarkEnd w:id=\"1\"/><w:r><w:t> 😀 end</w:t></w:r>",simple(" PAGE ","7")));
    let (before, _) = docx::import(&p).unwrap();
    let content = &before["content"][0]["content"];
    assert_eq!(content[1]["type"], "pageField");
    assert_eq!(content[3]["attrs"]["kind"], "NUMPAGES");
    assert_eq!(content[3]["text"], "123");
    assert_eq!(before["attrs"]["bookmarks"][0]["end"], 13);
    assert_eq!(
        docx::export(&before, Some(&p), Some(&before)).unwrap(),
        p.original
    );
    let mut edited = before.clone();
    edited["content"][0]["content"][0]["text"] = json!("Report page ");
    crate::document_bookmarks::rebase(
        &before,
        &mut edited,
        Some(&json!([{"start":0,"end":5,"text":"Report page "}])),
    )
    .unwrap();
    let bytes = docx::export(&edited, Some(&p), Some(&before)).unwrap();
    let out = Package::from_bytes(bytes.clone()).unwrap();
    let (again, _) = docx::import(&out).unwrap();
    assert_eq!(
        again["content"][0]["content"][1]["attrs"]["instruction"],
        " PAGE "
    );
    assert_eq!(
        again["content"][0]["content"][3]["attrs"]["instruction"],
        "NUMPAGES \\* Arabic"
    );
    assert_eq!(again["attrs"]["bookmarks"][0]["end"], 20);
    std::fs::write(
        std::env::temp_dir().join("folio-page-fields-edited.docx"),
        bytes,
    )
    .unwrap();
}
#[test]
fn unsupported_simple_nested_and_malformed_fields_remain_preserved() {
    for x in [
        simple(" MERGEFIELD Named ", "cached"),
        format!(
            "<w:fldSimple w:instr=\" PAGE \">{}</w:fldSimple>",
            simple(" PAGE ", "1")
        ),
        "<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:t>broken</w:t></w:r>".into(),
    ] {
        let p = package(&format!("<w:r><w:t>Keep </w:t></w:r>{x}"));
        let (before, _) = docx::import(&p).unwrap();
        assert_eq!(before["content"][0]["type"], "officeBlock");
        let mut edited = before.clone();
        edited["content"]
            .as_array_mut()
            .unwrap()
            .push(json!({"type":"paragraph","content":[{"type":"text","text":"After"}]}));
        let out =
            Package::from_bytes(docx::export(&edited, Some(&p), Some(&before)).unwrap()).unwrap();
        assert!(out.text("word/document.xml").unwrap().contains(&x));
    }
}
#[test]
fn authored_fields_roundtrip_all_page_kinds_and_reject_forged_sources() {
    let mut content = vec![];
    for code in ["PAGE", "NUMPAGES", "SECTIONPAGES", "SECTION"] {
        let mut f = new_field(code);
        f["marks"] = json!([{"type":"italic"}]);
        fields::validate(&f).unwrap();
        content.push(f);
    }
    let doc = json!({"type":"doc","content":[{"type":"paragraph","content":content}]});
    let bytes = docx::export(&doc, None, None).unwrap();
    let p = Package::from_bytes(bytes.clone()).unwrap();
    let (again, _) = docx::import(&p).unwrap();
    assert_eq!(again["content"][0]["content"].as_array().unwrap().len(), 4);
    for (i, kind) in ["PAGE", "NUMPAGES", "SECTIONPAGES", "SECTION"]
        .iter()
        .enumerate()
    {
        assert_eq!(again["content"][0]["content"][i]["attrs"]["kind"], *kind);
        assert_eq!(again["content"][0]["content"][i]["attrs"]["dirty"], true);
    }
    let mut forged = again["content"][0]["content"][0].clone();
    forged["attrs"]["instruction"] = json!(" NUMPAGES ");
    assert!(fields::validate(&forged).is_err());
    let mut unsupported = new_field("DDEAUTO");
    assert!(fields::validate(&unsupported).is_err());
    unsupported["attrs"]["instruction"] = json!(" PAGE \\unknown value");
    assert!(fields::validate(&unsupported).is_err());
    std::fs::write(
        std::env::temp_dir().join("folio-page-fields-new.docx"),
        bytes,
    )
    .unwrap();
}
#[test]
fn page_number_format_restart_chapter_and_custom_attributes() {
    let x=format!("<q:sectPr xmlns:q=\"{W}\" xmlns:ext=\"urn:keep\"><q:pgSz q:w=\"12240\" q:h=\"15840\"/><q:pgNumType q:fmt=\"lowerRoman\" q:start=\"7\" ext:kept=\"yes\"/><q:cols q:space=\"720\"/></q:sectPr>");
    let d = parse(&x).unwrap();
    let mut v = numbers::read(Some(d.root_element()));
    assert_eq!(numbers::patch(&x, &v).unwrap(), x);
    v["start"] = Value::Null;
    v["chapterStyle"] = json!(2);
    v["chapterSeparator"] = json!("emDash");
    let out = numbers::patch(&x, &v).unwrap();
    let d = parse(&out).unwrap();
    let n = child(d.root_element(), "pgNumType").unwrap();
    assert_eq!(n.attribute(("urn:keep", "kept")), Some("yes"));
    assert_eq!(n.attribute((W, "start")), None);
    assert_eq!(n.attribute((W, "chapStyle")), Some("2"));
    assert_eq!(n.attribute((W, "chapSep")), Some("emDash"));
    v["format"] = json!("upperLetter");
    v["start"] = json!(0);
    v["chapterStyle"] = Value::Null;
    let out = numbers::patch(&out, &v).unwrap();
    let d = parse(&out).unwrap();
    let n = child(d.root_element(), "pgNumType").unwrap();
    assert_eq!(n.attribute((W, "start")), Some("0"));
    assert_eq!(n.attribute((W, "chapSep")), None);
    for invalid in [json!(-1), json!(2147483648_i64), json!(1.5), json!("abc")] {
        v["start"] = invalid;
        assert!(numbers::patch(&x, &v).is_err());
    }
}
#[test]
fn page_fields_header_numbering_history_body_and_recovery() {
    use crate::workspace::Core;
    let source = std::env::temp_dir().join("folio-page-numbers-source.docx");
    std::fs::write(
        &source,
        include_bytes!("../../test/fixtures/headers-sections.docx"),
    )
    .unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":source}))
        .unwrap();
    let before = c.workspace.clone();
    let data = c
        .dispatch(json!({"op":"documentHeaders","section":1,"kind":"footer","variant":"default"}))
        .unwrap();
    let mut doc = data["document"].clone();
    let content = doc["content"][0]["content"].as_array_mut().unwrap();
    content.push(json!({"type":"text","text":" Page "}));
    content.push(new_field("PAGE"));
    content.push(json!({"type":"text","text":" of "}));
    content.push(new_field("NUMPAGES"));
    let mut r = data.clone();
    r["op"] = json!("documentHeadersApply");
    r["document"] = doc;
    r["pageNumbering"] =
        json!({"format":"lowerRoman","start":0,"chapterStyle":2,"chapterSeparator":"hyphen"});
    c.dispatch(r.clone()).unwrap();
    let after = c.workspace.clone();
    assert_eq!(before["document"]["content"], after["document"]["content"]);
    assert!(c.dispatch(r).is_err());
    assert_eq!(c.workspace, after);
    let first = c
        .dispatch(json!({"op":"documentHeaders","section":0,"kind":"footer","variant":"default"}))
        .unwrap();
    let second = c
        .dispatch(json!({"op":"documentHeaders","section":1,"kind":"footer","variant":"default"}))
        .unwrap();
    assert_eq!(first["pageNumbering"]["start"], Value::Null);
    assert_eq!(second["pageNumbering"]["start"], 0);
    assert_eq!(first["document"]["content"], second["document"]["content"]);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["officeDocument"], before["officeDocument"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["officeDocument"], after["officeDocument"]);
    let recovery = std::env::temp_dir().join("folio-page-numbers.folio");
    c.dispatch(json!({"op":"save","path":recovery})).unwrap();
    let mut fresh = Core::new();
    fresh
        .dispatch(json!({"op":"load","path":recovery}))
        .unwrap();
    assert_eq!(fresh.workspace["officeDocument"], after["officeDocument"]);
    fresh.dispatch(json!({"op":"exportOffice","format":"docx","path":std::env::temp_dir().join("folio-page-numbers-core.docx")})).unwrap();
    // A format-only request cannot bypass recording or protection through the story API.
    let mut request = second;
    request["pageNumbering"]["start"] = json!(4);
    let p = Package::from_bytes(std::fs::read(source).unwrap()).unwrap();
    assert!(headers::apply(&p, &request, Some(&json!({"enabled":true}))).is_err());
}

#[test]
fn comments_around_page_fields_and_table_fields_survive_surrounding_edits() {
    use crate::workspace::Core;
    let mut p = package(&format!(
        "<w:r><w:t>Before </w:t></w:r>{}<w:r><w:t> after</w:t></w:r>",
        simple(" NUMPAGES ", "123")
    ));
    let x=p.text("word/document.xml").unwrap().replace("<w:sectPr/>",&format!("<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w=\"4000\"/></w:tblGrid><w:tr><w:tc><w:tcPr><w:tcW w:w=\"4000\" w:type=\"dxa\"/></w:tcPr><w:p>{}</w:p></w:tc></w:tr></w:tbl><w:sectPr/>",simple(" PAGE ","7")));
    p.set("word/document.xml", x);
    let path = std::env::temp_dir().join("folio-page-fields-comments-source.docx");
    std::fs::write(&path, p.bytes().unwrap()).unwrap();
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    c.dispatch(json!({"op":"documentComment","action":"add","author":"Reviewer","text":"Keep the count","start":7,"end":10,"workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"]})).unwrap();
    let mut doc = c.workspace["document"].clone();
    doc["content"][0]["content"][0]["text"] = json!("New Before ");
    c.dispatch(
        json!({"op":"document","document":doc,"textEdits":[{"start":0,"end":0,"text":"New "}]}),
    )
    .unwrap();
    let out = std::env::temp_dir().join("folio-page-fields-comments.docx");
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":out}))
        .unwrap();
    let p = Package::from_bytes(std::fs::read(out).unwrap()).unwrap();
    let (doc, _) = docx::import(&p).unwrap();
    assert_eq!(doc["content"][0]["content"][1]["attrs"]["kind"], "NUMPAGES");
    assert_eq!(
        doc["content"][1]["content"][0]["content"][0]["content"][0]["content"][0]["attrs"]["kind"],
        "PAGE"
    );
    let comments = crate::document_comments::entries(&doc);
    assert_eq!(comments[0]["start"], 11);
    assert_eq!(comments[0]["end"], 14);
}
