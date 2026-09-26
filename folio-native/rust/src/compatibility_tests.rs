use crate::{compatibility, docx, office::*};
use serde_json::{json, Value};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// A Word package whose body holds `body`.
fn package(body: &str) -> Package {
    let empty = json!({"type":"doc","content":[{"type":"paragraph"}]});
    let mut p = Package::from_bytes(docx::export(&empty, None, None).unwrap()).unwrap();
    p.set(
        "word/document.xml",
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"{W}\"><w:body>{body}<w:sectPr/></w:body></w:document>"
        ),
    );
    Package::from_bytes(p.bytes().unwrap()).unwrap()
}

/// Adds a digital signature the way Word stores one.
fn sign(p: &mut Package) {
    p.entries
        .insert("_xmlsignatures/origin.sigs".into(), vec![]);
    p.set(
        "_xmlsignatures/sig1.xml",
        "<Signature xmlns=\"http://www.w3.org/2000/09/xmldsig#\"/>".into(),
    );
    p.set(
        "_xmlsignatures/_rels/origin.sigs.rels",
        "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/digital-signature/signature\" Target=\"sig1.xml\"/></Relationships>".into(),
    );
    let rels = p.text("_rels/.rels").unwrap().replace(
        "</Relationships>",
        "<Relationship Id=\"rIdSig\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/digital-signature/origin\" Target=\"_xmlsignatures/origin.sigs\"/></Relationships>",
    );
    p.set("_rels/.rels", rels);
    let types = p.text("[Content_Types].xml").unwrap().replace(
        "</Types>",
        "<Default Extension=\"sigs\" ContentType=\"application/vnd.openxmlformats-package.digital-signature-origin\"/><Override PartName=\"/_xmlsignatures/sig1.xml\" ContentType=\"application/vnd.openxmlformats-package.digital-signature-xmlsignature+xml\"/></Types>",
    );
    p.set("[Content_Types].xml", types);
    p.original = p.bytes().unwrap();
}

fn feature<'a>(report: &'a Value, id: &str) -> Option<&'a Value> {
    report["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == id)
}

#[test]
fn the_report_lists_what_the_file_contains_with_its_states() {
    let pkg = package(
        "<w:p><w:r><w:t>One</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p/></w:tc></w:tr></w:tbl>\
         <w:p><w:fldSimple w:instr=\"DATE\"><w:r><w:t>today</w:t></w:r></w:fldSimple></w:p>",
    );
    let report = compatibility::report("docx", Some(&pkg), None, None, false);
    assert_eq!(report["format"], "docx");
    assert_eq!(feature(&report, "word.tables").unwrap()["count"], 1);
    let fields = feature(&report, "word.fields").unwrap();
    assert_eq!(fields["count"], 1);
    assert_eq!(fields["states"]["Edit"], "Partial");
    assert_eq!(fields["state"], "Partial");
    assert_eq!(
        feature(&report, "word.tables").unwrap()["state"],
        "Verified"
    );
    assert!(feature(&report, "word.equations").is_none());
    assert!(feature(&report, "excel.cells").is_none());
    assert!(feature(&report, "any.signatures").is_none());
    assert_eq!(report["impacts"], json!([]));
    // A new file has nothing to report.
    let created = compatibility::report("docx", None, None, None, true);
    assert_eq!(created["features"], json!([]));
}

#[test]
fn signatures_are_reported_and_removed_only_when_the_file_changed() {
    let mut pkg = package("<w:p><w:r><w:t>Signed</w:t></w:r></w:p>");
    sign(&mut pkg);
    let unchanged = compatibility::report("docx", Some(&pkg), None, None, false);
    let signatures = feature(&unchanged, "any.signatures").unwrap();
    assert_eq!(signatures["count"], 1);
    assert_eq!(signatures["state"], "Unsupported");
    assert_eq!(unchanged["impacts"], json!([]));
    let changed = compatibility::report("docx", Some(&pkg), None, None, true);
    assert_eq!(changed["impacts"][0]["id"], "any.signatures");
    assert_eq!(changed["impacts"][0]["effect"], "removed");

    // Exporting through the workspace: unchanged keeps the signature,
    // edited removes it along with its relationship and content type.
    let dir = std::env::temp_dir();
    let source = dir.join(format!("folio-signed-{}.docx", std::process::id()));
    std::fs::write(&source, &pkg.original).unwrap();
    let mut c = crate::workspace::Core::new();
    c.dispatch(json!({"op":"importOffice","path":source}))
        .unwrap();
    let report = c.dispatch(json!({"op":"compatibilityReport"})).unwrap();
    assert_eq!(report["reports"][0]["impacts"], json!([]));

    let out = dir.join(format!("folio-signed-out-{}.docx", std::process::id()));
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":out}))
        .unwrap();
    let kept = Package::read(out.to_str().unwrap()).unwrap();
    assert!(kept.entries.contains_key("_xmlsignatures/sig1.xml"));

    let mut doc = c.workspace["document"].clone();
    doc["content"][0]["content"][0]["text"] = json!("Changed");
    c.dispatch(json!({"op":"document","document":doc,"baseRevision":c.workspace["revision"]}))
        .unwrap();
    let report = c.dispatch(json!({"op":"compatibilityReport"})).unwrap();
    assert_eq!(report["reports"][0]["impacts"][0]["id"], "any.signatures");
    c.dispatch(json!({"op":"exportOffice","format":"docx","path":out}))
        .unwrap();
    let edited = Package::read(out.to_str().unwrap()).unwrap();
    assert!(!edited
        .entries
        .keys()
        .any(|k| k.starts_with("_xmlsignatures/")));
    assert!(!edited
        .text("_rels/.rels")
        .unwrap()
        .contains("digital-signature"));
    assert!(!edited
        .text("[Content_Types].xml")
        .unwrap()
        .contains("/_xmlsignatures/"));
    let (reopened, _) = docx::import(&edited).unwrap();
    assert_eq!(reopened["content"][0]["content"][0]["text"], "Changed");
    std::fs::remove_file(&source).ok();
    std::fs::remove_file(&out).ok();
}

#[test]
fn editing_a_paragraph_with_formatting_folio_does_not_edit_warns_about_it() {
    let pkg = package(
        "<w:p><w:r><w:rPr><w:smallCaps/><w:b/></w:rPr><w:t>Small caps</w:t></w:r></w:p>\
         <w:p><w:r><w:rPr><w:spacing w:val=\"20\"/></w:rPr><w:t>Spaced</w:t></w:r></w:p>\
         <w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Bold</w:t></w:r></w:p>",
    );
    let (baseline, _) = docx::import(&pkg).unwrap();
    let report = |doc: &Value| {
        compatibility::report(
            "docx",
            Some(&pkg),
            Some(doc),
            Some(&baseline),
            doc != &baseline,
        )
    };
    assert_eq!(report(&baseline)["impacts"], json!([]));

    // Editing only the bold paragraph loses nothing.
    let mut bold = baseline.clone();
    bold["content"][2]["content"][0]["text"] = json!("Bolder");
    assert_eq!(report(&bold)["impacts"], json!([]));

    let mut edited = baseline.clone();
    edited["content"][0]["content"][0]["text"] = json!("Small caps, edited");
    let impacts = report(&edited)["impacts"].clone();
    assert_eq!(impacts[0]["id"], "word.text");
    assert_eq!(impacts[0]["effect"], "changed");
    assert_eq!(impacts[0]["paragraphs"], json!(["word-0"]));
    assert!(impacts[0]["message"]
        .as_str()
        .unwrap()
        .contains("1 edited paragraph will lose formatting Folio doesn't edit: small caps."));
}

#[test]
fn every_feature_has_eight_states_and_a_unique_id() {
    let features = compatibility::features();
    let ids: std::collections::BTreeSet<_> = features.iter().map(|f| f.id).collect();
    assert_eq!(ids.len(), features.len());
    for f in &features {
        assert!(matches!(f.format, "docx" | "xlsx" | "both"), "{}", f.id);
        assert!(!f.evidence.is_empty() && !f.on_save.is_empty(), "{}", f.id);
    }
}

/// docs/status/COMPATIBILITY.md is generated from the table. Regenerate it
/// with `FOLIO_WRITE_COMPATIBILITY=1 cargo test compatibility_matrix`.
#[test]
fn compatibility_matrix_document_matches_the_table() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../docs/status/COMPATIBILITY.md"
    );
    let current = std::fs::read_to_string(path).unwrap_or_default();
    let (head, _) = current
        .split_once("<!-- generated -->")
        .unwrap_or((current.as_str(), ""));
    let expected = format!(
        "{head}<!-- generated -->\n\n{}",
        compatibility::matrix_markdown()
    );
    if std::env::var("FOLIO_WRITE_COMPATIBILITY").is_ok() {
        std::fs::write(path, &expected).unwrap();
        return;
    }
    assert!(
        current == expected,
        "docs/status/COMPATIBILITY.md differs from compatibility.rs; regenerate it with FOLIO_WRITE_COMPATIBILITY=1"
    );
}
