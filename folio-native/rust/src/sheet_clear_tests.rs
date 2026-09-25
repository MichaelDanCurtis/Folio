//! Home > Clear (X01.17) and Delete (X01.18).
use crate::workspace::Core;
use serde_json::{json, Value};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

const SHEET: &str = concat!(
    r#"<sheetData>"#,
    r#"<row r="1"><c r="A1"><v>2</v></c><c r="B1" s="1"><v>5</v></c><c r="C1" s="1"/></row>"#,
    r#"<row r="3" s="1" customFormat="1"><c r="A3" s="1"><v>7</v></c></row>"#,
    r#"<row r="5"><c r="A5" t="inlineStr"><is><t>Home</t></is></c></row>"#,
    r#"</sheetData>"#,
    r#"<hyperlinks><hyperlink ref="B1" r:id="rId1"/><hyperlink ref="A5:C6" location="Sheet1!A1" display="Home"/></hyperlinks>"#,
);

/// `source.xlsx` whose second sheet has styled cells, a styled row and two
/// hyperlinks, one external.
fn workbook() -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/source.xlsx");
    let mut zip = zip::ZipArchive::new(Cursor::new(std::fs::read(fixture).unwrap())).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if file.name() == "xl/worksheets/sheet2.xml" {
            let xml = String::from_utf8(data).unwrap();
            let start = xml.find("<sheetData>").unwrap();
            let end = xml.find("</sheetData>").unwrap() + "</sheetData>".len();
            data = format!("{}{SHEET}{}", &xml[..start], &xml[end..])
                .replacen(
                    "<worksheet ",
                    r#"<worksheet xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" "#,
                    1,
                )
                .into_bytes();
        }
        out.start_file(file.name(), options).unwrap();
        out.write_all(&data).unwrap();
    }
    out.start_file("xl/worksheets/_rels/sheet2.xml.rels", options)
        .unwrap();
    out.write_all(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/" TargetMode="External"/>"#,
        r#"</Relationships>"#,
    ).as_bytes())
    .unwrap();
    let path = std::env::temp_dir().join(format!(
        "folio-clear-{}-{:?}.xlsx",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&path, out.finish().unwrap().into_inner()).unwrap();
    path
}

fn imported() -> (Core, String) {
    let mut core = Core::new();
    core.dispatch(json!({"op":"importOffice","path":workbook()}))
        .unwrap();
    let id = core.workspace["sheets"][1]["id"]
        .as_str()
        .unwrap()
        .to_string();
    (core, id)
}

fn clear(core: &mut Core, id: &str, range: &str, what: &str) {
    core.dispatch(
        json!({"op":"clearRange","sheetId":id,"range":range,"what":what,
        "baseRevision":core.workspace["revision"]}),
    )
    .unwrap();
}

fn sheet(core: &Core) -> Value {
    core.workspace["sheets"][1].clone()
}

fn part(core: &mut Core, name: &str) -> String {
    let out = std::env::temp_dir().join(format!(
        "folio-clear-out-{}-{:?}.xlsx",
        std::process::id(),
        std::thread::current().id()
    ));
    core.dispatch(json!({"op":"exportOffice","format":"xlsx","path":out}))
        .unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(std::fs::read(&out).unwrap())).unwrap();
    let mut xml = String::new();
    if let Ok(mut file) = zip.by_name(name) {
        file.read_to_string(&mut xml).unwrap();
    }
    xml
}

#[test]
fn clear_formats_falls_back_to_the_row_style_and_keeps_values() {
    let (mut core, id) = imported();
    assert_eq!(sheet(&core)["officeStyles"]["B1"], "1");
    let before = core.workspace.clone();
    clear(&mut core, &id, "A1:C3", "formats");
    let s = sheet(&core);
    assert_eq!(s["cells"]["B1"], "5", "values stay");
    assert_eq!(s["officeStyles"]["B1"], "0");
    assert!(s["cellStyles"]["B1"].is_null());
    assert!(
        s["cells"]["C1"].is_null(),
        "a blank kept only for its format"
    );
    // A3 is in a formatted row: it takes the row's style, as in Excel.
    assert_eq!(s["officeStyles"]["A3"], "1");
    assert_eq!(
        s["cellStyles"]["A3"],
        before["sheets"][1]["cellStyles"]["A3"]
    );
    // The hyperlink is not a format.
    let xml = part(&mut core, "xl/worksheets/sheet2.xml");
    assert!(xml.contains(r#"r="B1" s="0"><v>5</v></c>"#), "{xml}");
    assert!(xml.contains(r#"<c r="A3" s="1"><v>7</v></c>"#), "{xml}");
    assert!(xml.contains(r#"<hyperlink ref="B1""#));
    let styles = part(&mut core, "xl/styles.xml");
    assert!(
        styles.contains(r#"<cellXfs count="2">"#),
        "no new XF: {styles}"
    );

    core.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(core.workspace["sheets"], before["sheets"], "one undo step");
}

#[test]
fn clear_contents_keeps_formats_and_notes() {
    let (mut core, id) = imported();
    core.dispatch(
        json!({"op":"sheetReview","sheetId":id,"workspaceId":core.workspace["id"],
        "baseRevision":core.workspace["revision"],"action":"add","kind":"note","address":"A1",
        "author":"R","text":"Check"}),
    )
    .unwrap();
    clear(&mut core, &id, "A1:B1", "contents");
    let s = sheet(&core);
    assert!(s["cells"]["A1"].is_null());
    assert_eq!(s["cells"]["B1"], "", "the formatted cell stays, empty");
    assert_eq!(s["officeStyles"]["B1"], "1");
    assert_eq!(s["cellReviews"]["A1"]["kind"], "note");
    clear(&mut core, &id, "A1", "notes");
    assert!(sheet(&core)["cellReviews"]["A1"].is_null());
}

#[test]
fn clear_hyperlinks_splits_larger_links_and_drops_their_relationships() {
    let (mut core, id) = imported();
    clear(&mut core, &id, "B1:B6", "hyperlinks");
    let xml = part(&mut core, "xl/worksheets/sheet2.xml");
    assert!(!xml.contains(r#"ref="B1""#), "{xml}");
    assert!(xml.contains(r#"ref="A5:A6""#), "{xml}");
    assert!(xml.contains(r#"ref="C5:C6""#), "{xml}");
    assert!(xml.contains(r#"location="Sheet1!A1""#));
    assert!(
        xml.contains(r#"<c r="B1" s="1">"#),
        "the link's formatting stays"
    );
    let rels = part(&mut core, "xl/worksheets/_rels/sheet2.xml.rels");
    assert!(!rels.contains("example.com"), "{rels}");
    // Undo brings the links back.
    core.dispatch(json!({"op":"undo"})).unwrap();
    let xml = part(&mut core, "xl/worksheets/sheet2.xml");
    assert!(xml.contains(r#"<hyperlink ref="B1""#), "{xml}");
    assert!(xml.contains(r#"<hyperlink ref="A5:C6""#), "{xml}");
}

#[test]
fn clear_all_removes_everything_in_the_selection_only() {
    let (mut core, id) = imported();
    core.dispatch(
        json!({"op":"sheetReview","sheetId":id,"workspaceId":core.workspace["id"],
        "baseRevision":core.workspace["revision"],"action":"add","kind":"note","address":"B1",
        "author":"R","text":"Check"}),
    )
    .unwrap();
    clear(&mut core, &id, "B1:C6", "all");
    let s = sheet(&core);
    assert!(s["cellReviews"]["B1"].is_null());
    assert_eq!(s["cells"]["A1"], "2");
    assert_eq!(s["cells"]["A5"], "Home");
    assert!(s["cells"]["C1"].is_null());
    assert!(!s["cells"]["B1"].as_str().is_some_and(|v| !v.is_empty()));
    let xml = part(&mut core, "xl/worksheets/sheet2.xml");
    assert!(xml.contains(r#"ref="A5:A6""#), "{xml}");
    assert!(!xml.contains("<v>5</v>"), "{xml}");
    // Reopening the export gives the same cleared cells.
    let out = std::env::temp_dir().join(format!("folio-clear-all-{}.xlsx", std::process::id()));
    core.dispatch(json!({"op":"exportOffice","format":"xlsx","path":out}))
        .unwrap();
    let mut again = Core::new();
    again
        .dispatch(json!({"op":"importOffice","path":out}))
        .unwrap();
    let reopened = &again.workspace["sheets"][1];
    assert!(reopened["cells"]["B1"].as_str().is_none_or(str::is_empty));
    assert!(
        reopened["officeStyles"]["B1"]
            .as_str()
            .is_none_or(|s| s == "0"),
        "{reopened}\n{xml}"
    );
}

#[test]
fn clearing_whole_columns_visits_only_populated_cells() {
    let mut core = Core::new();
    core.dispatch(json!({"op":"new","blank":true})).unwrap();
    let id = core.workspace["sheets"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    core.dispatch(json!({"op":"cells","cells":[
        {"sheetId":id,"address":"A1","value":"1"},
        {"sheetId":id,"address":"A1048576","value":"2"},
        {"sheetId":id,"address":"B2","value":"keep"}
    ]}))
    .unwrap();
    core.dispatch(json!({"op":"formatCells","sheetId":id,"addresses":["A5"],
        "style":{"bold":true}}))
        .unwrap();
    let started = std::time::Instant::now();
    clear(&mut core, &id, "A1:A1048576", "all");
    assert!(started.elapsed() < crate::fuzz_tests::slow_limit());
    let s = &core.workspace["sheets"][0];
    assert!(s["cells"]["A1"].is_null() && s["cells"]["A1048576"].is_null());
    assert!(s["cells"]["A5"].is_null() && s["cellStyles"]["A5"].is_null());
    assert_eq!(s["cells"]["B2"], "keep");
}

#[test]
fn clear_rejects_bad_requests_without_changing_anything() {
    let (mut core, id) = imported();
    let before = core.workspace.clone();
    for request in [
        json!({"op":"clearRange","sheetId":id,"range":"A1","what":"everything"}),
        json!({"op":"clearRange","sheetId":id,"range":"not a range","what":"all"}),
        json!({"op":"clearRange","sheetId":"missing","range":"A1","what":"all"}),
        json!({"op":"clearRange","sheetId":id,"range":"A1","what":"all","baseRevision":-1}),
    ] {
        assert!(core.dispatch(request.clone()).is_err(), "{request}");
        assert_eq!(core.workspace, before);
    }
}

#[test]
fn clear_formats_removes_character_runs() {
    let mut core = Core::new();
    core.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/rich-text.xlsx")}))
        .unwrap();
    let id = core.workspace["sheets"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(core.workspace["sheets"][0]["cellStyles"]["A20"]["_richText"].is_array());
    core.dispatch(json!({"op":"clearRange","sheetId":id,"range":"A20","what":"formats"}))
        .unwrap();
    let out = std::env::temp_dir().join(format!("folio-clear-rich-{}.xlsx", std::process::id()));
    core.dispatch(json!({"op":"exportOffice","format":"xlsx","path":out}))
        .unwrap();
    let mut again = Core::new();
    again
        .dispatch(json!({"op":"importOffice","path":out}))
        .unwrap();
    let sheet = &again.workspace["sheets"][0];
    assert_eq!(sheet["cells"]["A20"], "Rich text 🌊");
    assert!(
        sheet["cellStyles"]["A20"]["_richText"].is_null(),
        "{}",
        sheet["cellStyles"]["A20"]
    );
    assert!(
        sheet["cellStyles"]["B20"]["_richText"].is_array(),
        "others keep theirs"
    );
}
