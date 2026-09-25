//! Defined names from imported workbooks evaluate in formulas (X09.08).
use crate::workspace::Core;
use serde_json::{json, Value};
use std::io::{Cursor, Read, Write};
use std::path::Path;

/// `source.xlsx` with extra `definedNames` in its workbook part.
fn workbook_with_names(names: &str) -> std::path::PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/source.xlsx");
    let mut zip = zip::ZipArchive::new(Cursor::new(std::fs::read(fixture).unwrap())).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if file.name() == "xl/workbook.xml" {
            let xml = String::from_utf8(data).unwrap();
            data = xml
                .replacen("<definedNames>", &format!("<definedNames>{names}"), 1)
                .into_bytes();
        }
        out.start_file(file.name(), zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(&data).unwrap();
    }
    let bytes = out.finish().unwrap().into_inner();
    let path = std::env::temp_dir().join(format!(
        "folio-names-{}-{}.xlsx",
        std::process::id(),
        names.len()
    ));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn value(core: &mut Core, sheet: &str, address: &str) -> Value {
    core.snapshot()["calculated"][sheet][address].clone()
}

#[test]
fn defined_names_evaluate_in_formulas() {
    let path = workbook_with_names(concat!(
        r#"<definedName name="Rate">Inputs!$A$1</definedName>"#,
        r#"<definedName name="Rows">'Inputs'!$A$1:$A$3</definedName>"#,
        r#"<definedName name="Twice">2*Inputs!$A$1</definedName>"#,
        r#"<definedName name="Local" localSheetId="1">Inputs!$B$1</definedName>"#,
        r#"<definedName name="TaxRate">0.07</definedName>"#,
        r#"<definedName name="Loop">Loop+1</definedName>"#,
    ));
    let mut core = Core::new();
    core.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let sheets = core.workspace["sheets"].clone();
    let budget = sheets[0]["id"].as_str().unwrap().to_string();
    let inputs = sheets[1]["id"].as_str().unwrap().to_string();
    let cells = json!([
        {"sheetId":inputs,"address":"A1","value":"5"},
        {"sheetId":inputs,"address":"A2","value":"6"},
        {"sheetId":inputs,"address":"A3","value":"7"},
        {"sheetId":inputs,"address":"B1","value":"100"},
        {"sheetId":budget,"address":"H1","value":"=Rate*2"},
        {"sheetId":budget,"address":"H2","value":"=SUM(Rows)"},
        {"sheetId":budget,"address":"H3","value":"=Twice"},
        {"sheetId":inputs,"address":"H4","value":"=Local+1"},
        {"sheetId":budget,"address":"H5","value":"=ISERROR(Local)"},
        {"sheetId":budget,"address":"H6","value":"=100*TaxRate"},
        {"sheetId":budget,"address":"H7","value":"=Loop"},
    ]);
    core.dispatch(json!({"op":"cells","cells":cells})).unwrap();
    assert_eq!(value(&mut core, &budget, "H1"), json!(10.0));
    assert_eq!(value(&mut core, &budget, "H2"), json!(18.0));
    assert_eq!(value(&mut core, &budget, "H3"), json!(10.0));
    assert_eq!(value(&mut core, &inputs, "H4"), json!(101.0), "sheet scope");
    assert_eq!(value(&mut core, &budget, "H5"), json!(true), "out of scope");
    assert_eq!(value(&mut core, &budget, "H6"), json!(7.000000000000001));
    assert_eq!(
        core.snapshot()["formatted"][&budget]["H7"]["text"],
        "#CIRC!",
        "a name that refers to itself"
    );

    // Inserting a row above the named cell moves the name with it.
    core.dispatch(
        json!({"op":"structure","sheetId":inputs,"action":"insertRows","at":1,"count":1}),
    )
    .unwrap();
    assert_eq!(core.workspace["sheets"][1]["cells"]["A2"], "5");
    assert_eq!(value(&mut core, &budget, "H1"), json!(10.0), "after insert");
    assert_eq!(value(&mut core, &budget, "H2"), json!(18.0), "after insert");

    // Undo and a change to the named cell recalculate through the name.
    core.dispatch(json!({"op":"undo"})).unwrap();
    core.dispatch(json!({"op":"cells","cells":[{"sheetId":inputs,"address":"A1","value":"50"}]}))
        .unwrap();
    assert_eq!(value(&mut core, &budget, "H1"), json!(100.0));
    let _ = std::fs::remove_file(path);
}

/// Names pointing at other workbooks are left out of calculation (X09.07),
/// and built-in `_xlnm.` names aren't offered to formulas.
#[test]
fn external_and_builtin_names_are_left_out() {
    let path = workbook_with_names(concat!(
        r#"<definedName name="Elsewhere">[1]Other!$A$1</definedName>"#,
        r#"<definedName name="Kept">Inputs!$A$1</definedName>"#,
    ));
    let mut core = Core::new();
    core.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let pkg = crate::office::Package::read(path.to_str().unwrap()).unwrap();
    let names = crate::xlsx_names::read(&pkg, &core.workspace["sheets"]);
    let listed: Vec<&str> = names.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(listed, ["Kept"]);
    let _ = std::fs::remove_file(path);
}

#[test]
fn external_references_are_told_apart_from_table_references() {
    use crate::xlsx_names::refers_elsewhere as elsewhere;
    assert!(elsewhere("[1]Other!$A$1"));
    assert!(elsewhere("'[Book 2.xlsx]Other sheet'!$A$1"));
    assert!(!elsewhere("SUM(Table1[Amount])"));
    assert!(!elsewhere("Table1[[#This Row],[Qty]]*2"));
    assert!(!elsewhere("Sheet1!$A$1:$B$2"));
}
