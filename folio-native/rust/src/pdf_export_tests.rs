//! PDF export (D05.04, D05.05).
use crate::pdf_export::write;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

fn text(x: f64, y: f64, width: f64, t: &str, extra: Value) -> Value {
    let mut op = json!({"op":"text","x":x,"y":y,"width":width,"text":t,"size":16,
        "family":"Liberation Sans","bold":false,"italic":false,"color":"#222222"});
    for (k, v) in extra.as_object().unwrap() {
        op[k] = v.clone();
    }
    op
}

/// A two-page document: a heading, a paragraph with a link, a picture, a
/// header artifact, rules and a cell border.
pub(crate) fn sample() -> Value {
    let pixel = STANDARD.encode([
        255u8, 0, 0, 255, 0, 0, 255, 128, 0, 255, 0, 255, 255, 255, 255, 0,
    ]);
    json!({
        "title": "Quarterly report",
        "lang": "en-US",
        "pages": [
            {"width":816,"height":1056,"ops":[
                {"op":"begin","tag":"Artifact"},
                text(96.0, 60.0, 120.0, "Header text", json!({"size":10})),
                {"op":"end"},
                {"op":"begin","tag":"H1"},
                text(96.0, 120.0, 300.0, "Quarterly report", json!({"size":32,"bold":true})),
                {"op":"end"},
                {"op":"begin","tag":"P"},
                {"op":"save"},
                {"op":"clip","rect":[96,130,624,60]},
                text(96.0, 160.0, 250.0, "Revenue rose in every region.", json!({})),
                text(350.0, 160.0, 90.0, "Details", json!({"link":"https://example.com/q3","color":"#185abd"})),
                {"op":"line","from":[350,163],"to":[440,163],"color":"#185abd","width":1},
                {"op":"restore"},
                {"op":"end"},
                {"op":"begin","tag":"Figure","alt":"A red and blue square"},
                {"op":"image","image":"word/media/a.png","rect":[96,200,96,96]},
                {"op":"end"},
                {"op":"rect","rect":[96,320,200,40],"stroke":"#b9c1cc","width":0.5},
                {"op":"rect","rect":[96,320,100,40],"fill":"#eef2f7"},
            ]},
            {"width":816,"height":1056,"ops":[
                {"op":"begin","tag":"H2"},
                text(96.0, 120.0, 200.0, "Next steps — naïve café 東京", json!({"size":24,"italic":true})),
                {"op":"end"},
            ]},
        ],
        "images": {"word/media/a.png": {"width":2,"height":2,"rgba":pixel}},
        "outline": [
            {"title":"Quarterly report","level":1,"page":0,"y":96},
            {"title":"Next steps","level":2,"page":1,"y":96},
        ],
    })
}

fn has(pdf: &[u8], needle: &str) -> bool {
    pdf.windows(needle.len()).any(|w| w == needle.as_bytes())
}

#[test]
fn a_document_becomes_a_tagged_pdf_with_fonts_pictures_links_and_an_outline() {
    let pdf = write(&sample()).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    for needle in [
        "/Type /Catalog",
        "/StructTreeRoot",
        "/MarkInfo",
        "/Lang (en-US)",
        "/Outlines",
        "/FontFile2",
        "/Identity-H",
        "/ToUnicode",
        "/S /URI",
        "(https://example.com/q3)",
        "/Alt (A red and blue square)",
        "/SMask",
        "/Title (Quarterly report)",
        "/S /H1",
        "/S /H2",
        "/S /Figure",
    ] {
        assert!(has(&pdf, needle), "missing {needle}");
    }
    // Two pages.
    assert_eq!(
        pdf.windows(b"/Type /Page\n".len())
            .filter(|w| *w == b"/Type /Page\n")
            .count()
            + pdf
                .windows(b"/Type /Page ".len())
                .filter(|w| *w == b"/Type /Page ")
                .count(),
        2
    );
    std::fs::write(std::env::temp_dir().join("folio-sample.pdf"), &pdf).ok();
}

#[test]
fn empty_or_invalid_requests_are_refused() {
    assert!(write(&json!({"pages":[]})).is_err());
    assert!(write(&json!({})).is_err());
    // Unknown operations and bad pictures are skipped, not fatal.
    let pdf = write(&json!({"pages":[{"width":100,"height":100,"ops":[
        {"op":"mystery"},
        {"op":"image","image":"missing","rect":[0,0,10,10]},
        {"op":"text","x":1,"y":20,"width":0,"text":"   ","size":10},
    ]}],"images":{"bad":{"width":5,"height":5,"rgba":"AAAA"}}}))
    .unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
}
