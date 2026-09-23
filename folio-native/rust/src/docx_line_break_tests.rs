use crate::{docx, office::*};
use serde_json::{json, Value};
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn package(inner: &str) -> Package {
    let mut p = docx::new_package();
    p.set("word/document.xml", format!("<w:document xmlns:w=\"{W}\"><w:body><w:p>{inner}</w:p><w:sectPr/></w:body></w:document>"));
    p.original = p.bytes().unwrap();
    p
}
fn breaks(document: &Value) -> Vec<&Value> {
    document["content"][0]["content"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["type"] == "hardBreak")
        .collect()
}

#[test]
fn line_breaks_keep_run_marks_types_and_clearing_when_surrounding_text_changes() {
    let p = package(
        r#"<w:r><w:rPr><w:b/><w:sz w:val="38"/></w:rPr><w:t>Before</w:t><w:br/><w:br w:type="textWrapping" w:clear="none"/><w:cr/><w:br w:type="page"/><w:br w:type="column"/><w:br w:clear="all"/><w:t>After</w:t></w:r>"#,
    );
    let (before, _) = docx::import(&p).unwrap();
    assert_eq!(breaks(&before).len(), 6);
    for br in breaks(&before) {
        assert!(br["marks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["type"] == "bold"));
        assert!(br["marks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["attrs"]["fontSize"].as_f64() == Some(19.0)));
    }
    assert_eq!(
        docx::export(&before, Some(&p), Some(&before)).unwrap(),
        p.original
    );
    let mut after = before.clone();
    after["content"][0]["content"][0]["text"] = json!("Edited before");
    let bytes = docx::export(&after, Some(&p), Some(&before)).unwrap();
    let out = Package::from_bytes(bytes.clone()).unwrap();
    let (reopened, _) = docx::import(&out).unwrap();
    assert_eq!(breaks(&reopened), breaks(&before));
    assert_eq!(
        reopened["content"][0]["content"][0]["text"],
        "Edited before"
    );
    let xml = out.text("word/document.xml").unwrap();
    let parsed = parse(&xml).unwrap();
    assert_eq!(
        parsed
            .descendants()
            .filter(|n| n.has_tag_name((W, "p")))
            .count(),
        1
    );
    assert_eq!(
        parsed
            .descendants()
            .filter(|n| n.has_tag_name((W, "br")))
            .count(),
        6
    );
    std::fs::write(
        std::env::temp_dir().join("folio-line-breaks-edited.docx"),
        bytes,
    )
    .unwrap();
}

#[test]
fn line_breaks_authored_breaks_and_literal_unicode_remain_distinct() {
    let doc = json!({"type":"doc","content":[{"type":"paragraph","content":[
        {"type":"hardBreak","marks":[{"type":"italic"},{"type":"link","attrs":{"href":"#Target"}}]},
        {"type":"text","text":"literal\u{2028}separator"},
        {"type":"hardBreak"}, {"type":"hardBreak"},
        {"type":"text","text":"Tail"}, {"type":"hardBreak"}
    ]}]});
    let bytes = docx::export(&doc, None, None).unwrap();
    let p = Package::from_bytes(bytes.clone()).unwrap();
    let xml = p.text("word/document.xml").unwrap();
    let parsed = parse(&xml).unwrap();
    assert_eq!(
        parsed
            .descendants()
            .filter(|n| n.has_tag_name((W, "p")))
            .count(),
        1
    );
    assert_eq!(
        parsed
            .descendants()
            .filter(|n| n.has_tag_name((W, "br")))
            .count(),
        4
    );
    assert!(xml.contains("literal\u{2028}separator"));
    let (again, _) = docx::import(&p).unwrap();
    assert_eq!(breaks(&again).len(), 4);
    assert_eq!(
        again["content"][0]["content"][1]["text"],
        "literal\u{2028}separator"
    );
    assert!(breaks(&again)[0]["marks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["type"] == "italic"));
    assert!(breaks(&again)[0]["marks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["attrs"]["href"] == "#Target"));
    std::fs::write(
        std::env::temp_dir().join("folio-line-breaks-authored.docx"),
        bytes,
    )
    .unwrap();
}
