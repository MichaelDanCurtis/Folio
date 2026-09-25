//! Legacy (pre-dynamic-array) formulas from XLSX get Excel 365's automatic
//! `@`, and formulas are written back in the form worksheet parts store
//! (X08.14).
use crate::structure::{excel_formula, legacy_formula};
use crate::workspace::Core;
use serde_json::{json, Value};
use std::io::{Cursor, Read, Write};
use std::path::Path;

fn sheets() -> Vec<String> {
    vec!["Data".to_string(), "Other sheet".to_string()]
}

#[test]
fn legacy_formulas_gain_at_only_where_excel_intersects() {
    let names = sheets();
    let defined = vec![
        ("Rate".to_string(), None, "Data!$A$1".to_string()),
        ("Block".to_string(), None, "Data!$A$1:$A$9".to_string()),
    ];
    let legacy = |f: &str, at: &str| legacy_formula(f, &names, 0, at, &defined);
    // A range where one value is expected: intersected with the row.
    assert_eq!(legacy("=ABS(A1:A3)", "B2"), "=ABS(@A1:A3)");
    assert_eq!(legacy("=A:A*2", "C3"), "=@A:A*2");
    assert_eq!(legacy("=Block*2", "C3"), "=@Block*2");
    // Ranges where ranges are expected, single cells, and single-cell names
    // keep their exact text.
    for unchanged in [
        "=SUM(A1:A3)",
        "=B2*C2",
        "=VLOOKUP(A1,'Other sheet'!A:B,2,FALSE)",
        "=Rate*2",
        "=SUMPRODUCT(A1:A3,B1:B3)",
        "=Sales[@Qty]*Sales[@Price]",
        "=SUM(Sales[Amount])",
        "=\"a@b.com\"",
        "=NOT A FORMULA(",
    ] {
        assert_eq!(legacy(unchanged, "D4"), unchanged, "{unchanged}");
    }
}

#[test]
fn formulas_are_written_in_worksheet_form() {
    let names = sheets();
    let stored = |f: &str, at: &str| excel_formula(f, &names, 0, at, &[]);
    // The automatic `@` is implied by legacy semantics: dropped.
    assert_eq!(stored("=ABS(@A1:A3)", "B2"), "ABS(A1:A3)");
    assert_eq!(stored("=@A:A*2", "C3"), "A:A*2");
    // An `@` where legacy semantics would pass the whole range is kept, in
    // the file format's spelling.
    assert_eq!(stored("=SUM(@A1:A3)", "B2"), "SUM(_xlfn.SINGLE(A1:A3))");
    // Newer functions carry the prefix Excel expects in a file.
    assert_eq!(
        stored("=XLOOKUP(1,A1:A3,B1:B3)", "D1"),
        "_xlfn.XLOOKUP(1,A1:A3,B1:B3)"
    );
    assert_eq!(stored("=SUM(A1:A3)", "B2"), "SUM(A1:A3)");
    // Spacing and line breaks are the author's, and Excel keeps them.
    assert_eq!(stored("=SUM(A10,\nA10)", "B2"), "SUM(A10,\nA10)");
    assert_eq!(
        stored("=XLOOKUP(1, A1:A3, B1:B3)", "D1"),
        "_xlfn.XLOOKUP(1,A1:A3,B1:B3)"
    );
    assert_eq!(stored("=Sales[@Qty]*2", "D2"), "Sales[@Qty]*2");
}

/// `source.xlsx` with extra cells in the Inputs sheet (sheet2.xml).
fn workbook_with_cells(cells: &str) -> std::path::PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/source.xlsx");
    let mut zip = zip::ZipArchive::new(Cursor::new(std::fs::read(fixture).unwrap())).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if file.name() == "xl/worksheets/sheet2.xml" {
            let xml = String::from_utf8(data).unwrap();
            let start = xml.find("<sheetData>").unwrap() + "<sheetData>".len();
            let end = xml.find("</sheetData>").unwrap();
            data = format!("{}{cells}{}", &xml[..start], &xml[end..]).into_bytes();
        }
        out.start_file(file.name(), zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(&data).unwrap();
    }
    let path = std::env::temp_dir().join(format!("folio-legacy-{}.xlsx", std::process::id()));
    std::fs::write(&path, out.finish().unwrap().into_inner()).unwrap();
    path
}

fn sheet_xml(path: &Path) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(std::fs::read(path).unwrap())).unwrap();
    let mut xml = String::new();
    zip.by_name("xl/worksheets/sheet2.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

#[test]
fn imported_legacy_formulas_calculate_like_excel_and_round_trip() {
    let path = workbook_with_cells(concat!(
        r#"<row r="1"><c r="A1"><v>-1</v></c></row>"#,
        r#"<row r="2"><c r="A2"><v>-2</v></c><c r="B2"><f>ABS(A1:A3)</f><v>2</v></c></row>"#,
        r#"<row r="3"><c r="A3"><v>-3</v></c><c r="C3"><f>A:A*10</f><v>-30</v></c>"#,
        r#"<c r="D3"><f>SUM(A1:A3)</f><v>-6</v></c></row>"#,
        r#"<row r="9"><c r="B9"><f>ABS(A1:A3)</f><v>0</v></c></row>"#,
    ));
    let mut core = Core::new();
    core.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let inputs = core.workspace["sheets"][1].clone();
    let id = inputs["id"].as_str().unwrap().to_string();
    assert_eq!(inputs["cells"]["B2"], "=ABS(@A1:A3)");
    assert_eq!(inputs["cells"]["C3"], "=@A:A*10");
    assert_eq!(inputs["cells"]["D3"], "=SUM(A1:A3)");
    let snapshot = core.snapshot();
    let value = |a: &str| snapshot["calculated"][&id][a].clone();
    assert_eq!(value("B2"), json!(2.0), "intersects row 2");
    assert_eq!(value("C3"), json!(-30.0), "intersects row 3");
    assert_eq!(value("D3"), json!(-6.0));
    assert_eq!(
        snapshot["formatted"][&id]["B9"]["text"], "#VALUE!",
        "row 9 doesn't meet A1:A3, as in Excel"
    );

    // Unchanged formulas keep their original XML; an edited one is written
    // in worksheet form, and reimports with the same meaning.
    core.dispatch(json!({"op":"cells","cells":[
        {"sheetId":id,"address":"E1","value":"=XLOOKUP(-2,A1:A3,A1:A3)"}
    ]}))
    .unwrap();
    let out = std::env::temp_dir().join(format!("folio-legacy-out-{}.xlsx", std::process::id()));
    core.dispatch(json!({"op":"exportOffice","format":"xlsx","path":out}))
        .unwrap();
    let xml = sheet_xml(&out);
    assert!(xml.contains("<f>ABS(A1:A3)</f>"), "{xml}");
    assert!(xml.contains("<f>A:A*10</f>"));
    assert!(
        xml.contains("<f>_xlfn.XLOOKUP(-2,A1:A3,A1:A3)</f>"),
        "{xml}"
    );
    let mut again = Core::new();
    again
        .dispatch(json!({"op":"importOffice","path":out}))
        .unwrap();
    let reopened: Value = again.workspace["sheets"][1]["cells"].clone();
    assert_eq!(reopened["B2"], "=ABS(@A1:A3)");
    let e1 = again.snapshot()["calculated"][&id]["E1"].clone();
    assert_eq!(e1, json!(-2.0), "reimported XLOOKUP: {}", reopened["E1"]);
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(out);
}

#[test]
fn range_arguments_keep_their_ranges() {
    let names = sheets();
    // Arguments that take references or pass ranges through, as in legacy
    // Excel: no automatic `@` (found by the Office corpus: WORKDAY holidays).
    for unchanged in [
        "=WORKDAY(C2,6,D2:D5)",
        "=WORKDAY.INTL(A1,5,1,B1:B5)",
        "=NETWORKDAYS(A1,A2,B1:B5)",
        "=SUM(IF(1,A1:A5))",
        "=SUM(CHOOSE(2,A1:A5,B1:B5))",
        "=ISFORMULA(A1:A5)",
        "=FORMULATEXT(A1:A5)",
        "=SHEET(A1:A5)",
        "=SHEETS(A1:A5)",
    ] {
        assert_eq!(legacy_formula(unchanged, &names, 0, "H20", &[]), unchanged);
    }
    // Where legacy Excel intersects, the `@` is added: conditions, IFERROR's
    // value, scalar arguments, and a range-valued IF or CHOOSE on its own.
    for (formula, legacy) in [
        ("=IF(A1:A5,1,2)", "=IF(@A1:A5,1,2)"),
        ("=SUM(IFERROR(A1:A5,0))", "=SUM(IFERROR(@A1:A5,0))"),
        ("=NPER(0.1,1,A1:A5)", "=NPER(0.1,1,@A1:A5)"),
        ("=IF(1,A1:A5)", "=@IF(1,A1:A5)"),
        ("=CHOOSE(1,A1:A5,B1:B5)", "=@CHOOSE(1,A1:A5,B1:B5)"),
        // CELL's reference is kept; IronCalc treats CELL as dynamic, so the
        // call gets a harmless `@`, dropped again on export.
        ("=CELL(\"address\",A1:A5)", "=@CELL(\"address\",A1:A5)"),
    ] {
        assert_eq!(legacy_formula(formula, &names, 0, "H20", &[]), legacy);
    }
}
