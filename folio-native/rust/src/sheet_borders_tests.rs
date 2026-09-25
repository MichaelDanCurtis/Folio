//! Borders and fill colour (X04.07, X04.08), and theme/indexed colours.
use crate::workspace::Core;
use serde_json::{json, Value};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

/// `source.xlsx` with a themed, tinted fill and a border using indexed and
/// theme colours (XF 2, on Inputs!B2).
fn workbook() -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/source.xlsx");
    let mut zip = zip::ZipArchive::new(Cursor::new(std::fs::read(fixture).unwrap())).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        let name = file.name().to_string();
        if name == "xl/styles.xml" || name == "xl/worksheets/sheet2.xml" {
            let mut xml = String::from_utf8(data).unwrap();
            if name == "xl/styles.xml" {
                xml = xml
                    .replace(
                        "<fills count=\"3\">",
                        "<fills count=\"4\">",
                    )
                    .replace(
                        "</fills>",
                        r#"<fill><patternFill patternType="solid"><fgColor theme="4" tint="0.7999816888943144"/><bgColor indexed="64"/></patternFill></fill></fills>"#,
                    )
                    .replace(
                        r#"<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>"#,
                        r#"<borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border><left style="thin"><color indexed="10"/></left><right/><top/><bottom style="medium"><color theme="1"/></bottom><diagonal/></border></borders>"#,
                    )
                    .replace("<cellXfs count=\"2\">", "<cellXfs count=\"3\">")
                    .replace(
                        "</cellXfs>",
                        r#"<xf numFmtId="0" fontId="0" fillId="3" borderId="1" applyFill="1" applyBorder="1" xfId="0"/></cellXfs>"#,
                    );
                assert!(xml.contains("fillId=\"3\" borderId=\"1\""), "{xml}");
            } else {
                xml = xml.replace(
                    "</sheetData>",
                    r#"<row r="2"><c r="B2" s="2"><v>1</v></c></row></sheetData>"#,
                );
            }
            data = xml.into_bytes();
        }
        out.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(&data).unwrap();
    }
    let path = std::env::temp_dir().join(format!(
        "folio-borders-{}-{:?}.xlsx",
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

fn reopened(core: &mut Core) -> Value {
    let out = std::env::temp_dir().join(format!(
        "folio-borders-out-{}-{:?}.xlsx",
        std::process::id(),
        std::thread::current().id()
    ));
    core.dispatch(json!({"op":"exportOffice","format":"xlsx","path":out}))
        .unwrap();
    let mut again = Core::new();
    again
        .dispatch(json!({"op":"importOffice","path":out}))
        .unwrap();
    again.workspace.clone()
}

fn borders(core: &mut Core, id: &str, range: &str, preset: &str) -> Result<Value, String> {
    core.dispatch(
        json!({"op":"borderCells","sheetId":id,"range":range,"preset":preset,
        "line":"thin","color":"#0070C0","baseRevision":core.workspace["revision"]}),
    )
}

#[test]
fn theme_and_indexed_colours_are_resolved_for_fills_and_borders() {
    let (core, _) = imported();
    let style = &core.workspace["sheets"][1]["cellStyles"]["B2"];
    // Accent 1 (#4F81BD), Lighter 80%.
    assert_eq!(style["fill"], "#DCE6F2");
    assert_eq!(
        style["border"]["left"],
        json!({"style":"thin","color":"#FF0000"})
    );
    assert_eq!(
        style["border"]["bottom"],
        json!({"style":"medium","color":"#000000"})
    );
    assert!(style["border"]["top"].is_null());
}

#[test]
fn presets_draw_the_lines_excel_draws() {
    let (mut core, id) = imported();
    let before = core.workspace.clone();
    borders(&mut core, &id, "C3:D4", "outside").unwrap();
    let styles = core.workspace["sheets"][1]["cellStyles"].clone();
    let line = json!({"style":"thin","color":"#0070C0"});
    assert_eq!(styles["C3"]["border"], json!({"top":line,"left":line}));
    assert_eq!(styles["D3"]["border"], json!({"top":line,"right":line}));
    assert_eq!(styles["C4"]["border"], json!({"bottom":line,"left":line}));
    assert_eq!(styles["D4"]["border"], json!({"bottom":line,"right":line}));
    core.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(core.workspace["sheets"], before["sheets"], "one undo step");

    borders(&mut core, &id, "C3:D3", "bottom").unwrap();
    let styles = core.workspace["sheets"][1]["cellStyles"].clone();
    assert_eq!(styles["C3"]["border"], json!({"bottom":line}));
    borders(&mut core, &id, "C3:C3", "all").unwrap();
    assert_eq!(
        core.workspace["sheets"][1]["cellStyles"]["C3"]["border"],
        json!({"top":line,"left":line,"bottom":line,"right":line})
    );
    borders(&mut core, &id, "B2:C3", "none").unwrap();
    let styles = core.workspace["sheets"][1]["cellStyles"].clone();
    assert_eq!(styles["B2"]["border"], json!({}));
    assert_eq!(styles["C3"]["border"], json!({}));
    assert_eq!(
        styles["D3"]["border"],
        json!({"bottom":line}),
        "outside the selection"
    );

    core.dispatch(
        json!({"op":"borderCells","sheetId":id,"range":"E5:F6","preset":"thickBox",
        "line":"thin"}),
    )
    .unwrap();
    assert_eq!(
        core.workspace["sheets"][1]["cellStyles"]["E5"]["border"]["top"],
        json!({"style":"thick"})
    );
}

#[test]
fn borders_and_fills_round_trip_through_xlsx() {
    let (mut core, id) = imported();
    borders(&mut core, &id, "C3:D4", "all").unwrap();
    borders(&mut core, &id, "B2", "none").unwrap();
    core.dispatch(json!({"op":"formatCells","sheetId":id,"addresses":["B2"],
        "style":{"fill":"none"}}))
        .unwrap();
    core.dispatch(json!({"op":"formatCells","sheetId":id,"addresses":["D4"],
        "style":{"fill":"#FFFF00"}}))
        .unwrap();
    let again = reopened(&mut core);
    let styles = &again["sheets"][1]["cellStyles"];
    let line = json!({"style":"thin","color":"#0070C0"});
    assert_eq!(
        styles["D4"]["border"],
        json!({"top":line,"left":line,"bottom":line,"right":line})
    );
    assert_eq!(styles["D4"]["fill"], "#FFFF00");
    assert!(styles["B2"]["border"].is_null(), "{}", styles["B2"]);
    assert!(styles["B2"]["fill"].is_null(), "{}", styles["B2"]);
}

#[test]
fn new_workbooks_write_borders_too() {
    let mut core = Core::new();
    core.dispatch(json!({"op":"new","blank":true})).unwrap();
    let id = core.workspace["sheets"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    borders(&mut core, &id, "A1:B2", "outside").unwrap();
    let again = reopened(&mut core);
    let styles = &again["sheets"][0]["cellStyles"];
    assert_eq!(styles["B2"]["border"]["right"]["style"], "thin");
    assert_eq!(styles["B2"]["border"]["bottom"]["color"], "#0070C0");
}

#[test]
fn bad_border_requests_change_nothing() {
    let (mut core, id) = imported();
    let before = core.workspace.clone();
    for request in [
        json!({"op":"borderCells","sheetId":id,"range":"A1","preset":"diagonal"}),
        json!({"op":"borderCells","sheetId":id,"range":"A1","preset":"all","line":"wavy"}),
        json!({"op":"borderCells","sheetId":id,"range":"A1","preset":"all","color":"red"}),
        json!({"op":"borderCells","sheetId":id,"range":"A1:A1048576","preset":"all"}),
        json!({"op":"borderCells","sheetId":"missing","range":"A1","preset":"all"}),
        json!({"op":"formatCells","sheetId":id,"addresses":["A1"],"style":{"border":{"middle":null}}}),
    ] {
        assert!(core.dispatch(request.clone()).is_err(), "{request}");
        assert_eq!(core.workspace, before);
    }
}
