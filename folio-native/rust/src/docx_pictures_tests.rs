use crate::{docx, docx_pictures, office::*};
use serde_json::{json, Value};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// A 2×1 PNG.
fn png() -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    b.extend(2u32.to_be_bytes());
    b.extend(1u32.to_be_bytes());
    b.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    b
}

fn drawing(rid: &str, anchor: bool) -> String {
    let kind = if anchor { "anchor" } else { "inline" };
    format!(
        concat!(
            "<w:drawing><wp:{k}><wp:extent cx=\"1905000\" cy=\"952500\"/>",
            "<wp:docPr id=\"4\" name=\"Picture 4\" descr=\"A chart\"/>",
            "<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">",
            "<pic:pic><pic:nvPicPr><pic:cNvPr id=\"0\" name=\"chart.png\"/><pic:cNvPicPr/></pic:nvPicPr>",
            "<pic:blipFill><a:blip r:embed=\"{rid}\"/></pic:blipFill><pic:spPr/></pic:pic>",
            "</a:graphicData></a:graphic></wp:{k}></w:drawing>"
        ),
        k = kind,
        rid = rid
    )
}

/// A Word package whose body holds `body`, with `png()` as image rId9.
fn package(body: &str) -> Package {
    let empty = json!({"type":"doc","content":[{"type":"paragraph"}]});
    let mut p = Package::from_bytes(docx::export(&empty, None, None).unwrap()).unwrap();
    let (part, rid) = docx_pictures::add(&mut p, &png()).unwrap();
    assert_eq!(part.split('.').next_back(), Some("png"));
    let body = body.replace("RID", &rid);
    p.set(
        "word/document.xml",
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"{W}\" xmlns:r=\"{R}\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><w:body>{body}<w:sectPr/></w:body></w:document>"
        ),
    );
    Package::from_bytes(p.bytes().unwrap()).unwrap()
}

fn images(doc: &Value) -> Vec<Value> {
    let mut out = vec![];
    fn walk(n: &Value, out: &mut Vec<Value>) {
        if n["type"] == "image" {
            out.push(n["attrs"].clone());
        }
        for c in n["content"].as_array().into_iter().flatten() {
            walk(c, out);
        }
    }
    walk(doc, &mut out);
    out
}

#[test]
fn inline_pictures_import_as_image_nodes_and_floating_ones_stay_preserved() {
    let pkg = package(&format!(
        "<w:p><w:r><w:t>Before </w:t></w:r><w:r>{}</w:r><w:r><w:t> after</w:t></w:r></w:p><w:p><w:r>{}</w:r></w:p>",
        drawing("RID", false),
        drawing("RID", true)
    ));
    let (doc, _) = docx::import(&pkg).unwrap();
    let content = doc["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "paragraph");
    assert_eq!(content[0]["content"][1]["type"], "image");
    let image = &images(&doc)[0];
    assert_eq!(image["width"], 200.0);
    assert_eq!(image["height"], 100.0);
    assert_eq!(image["alt"], "A chart");
    assert!(image["src"]
        .as_str()
        .unwrap()
        .starts_with("word/media/folio-"));
    assert_eq!(content[1]["type"], "officeBlock");
    assert_eq!(images(&doc).len(), 1);
}

#[test]
fn edited_paragraphs_keep_their_pictures_and_new_pictures_export() {
    let pkg = package(&format!(
        "<w:p><w:r><w:t>Before </w:t></w:r><w:r>{}</w:r></w:p>",
        drawing("RID", false)
    ));
    let (baseline, _) = docx::import(&pkg).unwrap();
    // Unchanged: the file itself.
    assert_eq!(
        docx::export(&baseline, Some(&pkg), Some(&baseline)).unwrap(),
        pkg.original
    );
    // Edited text: the picture's drawing is written back.
    let mut edited = baseline.clone();
    edited["content"][0]["content"][0]["text"] = json!("Changed ");
    let out =
        Package::from_bytes(docx::export(&edited, Some(&pkg), Some(&baseline)).unwrap()).unwrap();
    let (again, _) = docx::import(&out).unwrap();
    assert_eq!(again["content"][0]["content"][0]["text"], "Changed ");
    assert_eq!(images(&again)[0]["src"], images(&baseline)[0]["src"]);
    assert_eq!(images(&again)[0]["alt"], "A chart");

    // A new picture, added to the package and written as a new drawing.
    let mut pkg2 = pkg.clone();
    let mut gif = b"GIF89a".to_vec();
    gif.extend([3, 0, 5, 0, 0, 0, 0]);
    let (part, rid) = docx_pictures::add(&mut pkg2, &gif).unwrap();
    assert_eq!(docx_pictures::image_size(&gif), Some((3, 5)));
    let mut added = baseline.clone();
    added["content"][0]["content"].as_array_mut().unwrap().push(
        json!({"type":"image","attrs":{"src":part,"rId":rid,"width":30,"height":50,"alt":"New"}}),
    );
    let out =
        Package::from_bytes(docx::export(&added, Some(&pkg2), Some(&baseline)).unwrap()).unwrap();
    let (again, _) = docx::import(&out).unwrap();
    let found = images(&again);
    assert_eq!(found.len(), 2);
    assert_eq!(found[1]["src"], json!(part));
    assert_eq!(
        (found[1]["width"].as_f64(), found[1]["height"].as_f64()),
        (Some(30.0), Some(50.0))
    );
    assert_eq!(found[1]["alt"], "New");
    assert!(out
        .text("[Content_Types].xml")
        .unwrap()
        .contains("Extension=\"gif\""));
    assert!(out.entries.contains_key(&part));
}

#[test]
fn picture_kinds_and_sizes_from_headers() {
    assert_eq!(docx_pictures::image_size(&png()), Some((2, 1)));
    assert_eq!(docx_pictures::image_kind(b"<svg"), None);
    let jpeg = [
        0xff, 0xd8, 0xff, 0xe0, 0, 4, 0, 0, 0xff, 0xc0, 0, 11, 8, 0, 7, 0, 9, 3, 0, 0, 0,
    ];
    assert_eq!(docx_pictures::image_size(&jpeg), Some((9, 7)));
}
