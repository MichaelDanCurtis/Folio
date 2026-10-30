//! Exported dynamic arrays (X09.02) and formula errors after a defined name
//! is deleted (X09.05), checked in the saved XML and after reopening.
use crate::office::{attr, parse, raw, Package};
use crate::workspace::Core;
use serde_json::{json, Value};

fn core() -> (Core, String) {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let sheet = c.workspace["sheets"][0]["id"].as_str().unwrap().to_string();
    (c, sheet)
}

fn set(c: &mut Core, sheet: &str, address: &str, value: &str) {
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":sheet,"address":address,"value":value}]}))
        .unwrap();
}

fn value(c: &mut Core, sheet: &str, address: &str) -> Value {
    c.snapshot()["calculated"][sheet][address].clone()
}

/// Saves the workbook and returns its package.
fn save(c: &mut Core, name: &str) -> (Package, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("folio-{name}-{}.xlsx", std::process::id()));
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    (Package::read(path.to_str().unwrap()).unwrap(), path)
}

fn reopen(path: &std::path::Path) -> (Core, String) {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let sheet = c.workspace["sheets"][0]["id"].as_str().unwrap().to_string();
    (c, sheet)
}

fn sheet_xml(p: &Package) -> String {
    let path = p
        .entries
        .keys()
        .find(|k| k.starts_with("xl/worksheets/") && k.ends_with(".xml"))
        .unwrap();
    p.text(path).unwrap().to_string()
}

/// The `<c>` record of a cell, or None when the sheet has none.
fn cell(xml: &str, a: &str) -> Option<String> {
    let d = parse(xml).unwrap();
    let found = d
        .descendants()
        .find(|n| n.tag_name().name() == "c" && attr(*n, "r") == Some(a))
        .map(|n| raw(xml, n).to_string());
    found
}

#[test]
fn spilled_formula_is_saved_as_a_dynamic_array_with_cached_values() {
    let (mut c, s) = core();
    set(&mut c, &s, "A1", "=SEQUENCE(3,3)");
    assert_eq!(value(&mut c, &s, "C3"), json!(9.0));
    let (p, path) = save(&mut c, "spill");
    let xml = sheet_xml(&p);

    // The anchor: an array formula over the spilled range, marked dynamic.
    let anchor = cell(&xml, "A1").unwrap();
    assert!(anchor.contains(r#"cm="1""#), "{anchor}");
    assert!(
        anchor.contains(r#"<f t="array" ref="A1:C3">_xlfn.SEQUENCE(3,3)</f>"#),
        "{anchor}"
    );
    assert!(anchor.contains("<v>1.0</v>"), "{anchor}");
    // Every spilled cell holds its value, and no formula.
    for (a, n) in [("B1", 2), ("C1", 3), ("A2", 4), ("B3", 8), ("C3", 9)] {
        let record = cell(&xml, a).unwrap_or_else(|| panic!("{a} missing"));
        assert!(record.contains(&format!("<v>{n}.0</v>")), "{a}: {record}");
        assert!(!record.contains("<f"), "{a}: {record}");
    }

    // The XLDAPR metadata, its content type and its relationship.
    let metadata = p.text("xl/metadata.xml").unwrap();
    assert!(metadata.contains(r#"<metadataType name="XLDAPR""#));
    assert!(metadata.contains(r#"fDynamic="1""#));
    assert!(metadata.contains(r#"<cellMetadata count="1"><bk><rc t="1" v="0"/></bk>"#));
    assert!(p
        .text("[Content_Types].xml")
        .unwrap()
        .contains(r#"PartName="/xl/metadata.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheetMetadata+xml""#));
    assert!(p
        .text("xl/_rels/workbook.xml.rels")
        .unwrap()
        .contains(r#"relationships/sheetMetadata" Target="metadata.xml""#));

    // Reopened, it spills again rather than holding fixed values.
    let (mut again, s2) = reopen(&path);
    // (Imported formulas keep the file's `_xlfn.` spelling.)
    let formula = again.workspace["sheets"][0]["cells"]["A1"].clone();
    assert!(
        formula.as_str().unwrap().ends_with("SEQUENCE(3,3)"),
        "{formula}"
    );
    assert!(again.workspace["sheets"][0]["cells"].get("B2").is_none());
    assert!(again.workspace["sheets"][0].get("arrayFormulas").is_none());
    assert_eq!(again.snapshot()["spills"][&s2]["A1"], "A1:C3");
    assert_eq!(value(&mut again, &s2, "B2"), json!(5.0));

    // Edited in the reopened file, the range and values follow the spill;
    // cells it no longer fills lose their old values.
    set(&mut again, &s2, "A1", "=SEQUENCE(2,2)");
    let (p2, path2) = save(&mut again, "spill-shrunk");
    let xml = sheet_xml(&p2);
    let anchor = cell(&xml, "A1").unwrap();
    assert!(
        anchor.contains(r#"t="array" ref="A1:B2">_xlfn.SEQUENCE(2,2)</f>"#),
        "{anchor}"
    );
    assert!(cell(&xml, "B2").unwrap().contains("<v>4.0</v>"));
    assert!(cell(&xml, "C3").is_none_or(|c| !c.contains("<v>")));
    assert!(cell(&xml, "C1").is_none_or(|c| !c.contains("<v>")));
    // The existing metadata entry is reused.
    assert_eq!(p2.text("xl/metadata.xml").unwrap(), metadata);
    std::fs::remove_file(&path).ok();
    std::fs::remove_file(&path2).ok();
}

#[test]
fn groupby_spills_text_and_blocked_spill_stays_dynamic() {
    let (mut c, s) = core();
    for (a, v) in [
        ("A1", "East"),
        ("A2", "West"),
        ("A3", "East"),
        ("B1", "1"),
        ("B2", "2"),
        ("B3", "3"),
        ("D1", "=GROUPBY(A1:A3,B1:B3,SUM)"),
        ("G1", "=SEQUENCE(2)"),
        ("G2", "x"),
    ] {
        set(&mut c, &s, a, v);
    }
    let spill = c.snapshot()["spills"][&s]["D1"].clone();
    let range = spill.as_str().expect("GROUPBY spills").to_string();
    let (p, path) = save(&mut c, "groupby");
    let xml = sheet_xml(&p);
    let anchor = cell(&xml, "D1").unwrap();
    assert!(anchor.contains(r#"cm="1""#), "{anchor}");
    assert!(
        anchor.contains(&format!(r#"<f t="array" ref="{range}">_xlfn.GROUPBY("#)),
        "{anchor}"
    );
    // Text results are cached as formula strings, numbers as numbers.
    assert!(
        anchor.contains(r#"t="str""#) && anchor.contains("<v>East</v>"),
        "{anchor}"
    );
    let total = cell(&xml, "E1").unwrap();
    assert!(total.contains("<v>4.0</v>"), "{total}");
    let label = cell(&xml, "D2").unwrap();
    assert!(
        label.contains(r#"t="str""#) && label.contains("<v>West</v>"),
        "{label}"
    );

    // Blocked by a value, the formula is still dynamic: Excel shows #SPILL!
    // rather than the first value of a legacy formula.
    let blocked = cell(&xml, "G1").unwrap();
    assert!(blocked.contains(r#"cm="1""#), "{blocked}");
    assert!(blocked.contains(r#"<f t="array" ref="G1">"#), "{blocked}");
    assert!(
        blocked.contains(r#"t="e""#) && blocked.contains("<v>#SPILL!</v>"),
        "{blocked}"
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn formula_whose_name_was_deleted_saves_its_error() {
    let (mut c, s) = core();
    set(&mut c, &s, "B2", "8");
    c.dispatch(json!({"op":"nameManager","action":"add","name":"Rate","refersTo":"=5","sheetId":null,"baseRevision":c.workspace["revision"]}))
        .unwrap();
    set(&mut c, &s, "A1", "=Rate*B2");
    assert_eq!(value(&mut c, &s, "A1"), json!(40.0));
    let (p, path) = save(&mut c, "name-before");
    assert!(cell(&sheet_xml(&p), "A1").unwrap().contains("<v>40.0</v>"));

    c.dispatch(json!({"op":"nameManager","action":"delete","oldName":"Rate","oldSheetId":null,"baseRevision":c.workspace["revision"]}))
        .unwrap();
    assert_eq!(c.snapshot()["formatted"][&s]["A1"]["text"], "#NAME?");
    let (p, path2) = save(&mut c, "name-deleted");
    let record = cell(&sheet_xml(&p), "A1").unwrap();
    assert!(record.contains(r#"t="e""#), "{record}");
    assert!(record.contains("<f>Rate*B2</f><v"), "{record}");
    assert!(record.contains(">#NAME?</v>"), "{record}");
    assert!(!record.contains("40"), "{record}");
    assert!(!p.text("xl/workbook.xml").unwrap().contains("Rate"));

    // Reopened, it is still the formula, still #NAME?.
    let (mut again, s2) = reopen(&path2);
    // (A legacy formula with an unknown name reopens with Excel's `@`.)
    let formula = again.workspace["sheets"][0]["cells"]["A1"].clone();
    assert!(formula.as_str().unwrap().ends_with("Rate*B2"), "{formula}");
    assert_eq!(again.snapshot()["formatted"][&s2]["A1"]["text"], "#NAME?");
    // Undo brings the name, and the value, back to the saved file.
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(value(&mut c, &s, "A1"), json!(40.0));
    let (p, path3) = save(&mut c, "name-undone");
    assert!(cell(&sheet_xml(&p), "A1").unwrap().contains("<v>40.0</v>"));
    for p in [path, path2, path3] {
        std::fs::remove_file(p).ok();
    }
}

#[test]
fn existing_metadata_part_gains_the_dynamic_array_entry() {
    let mut p = Package::from_bytes(
        crate::xlsx::export(
            &json!([{"id":"s","name":"S","cells":{"A1":"1"}}]),
            &json!({}),
            &json!({}),
            None,
            None,
        )
        .unwrap(),
    )
    .unwrap();
    // A rich-value type and cell entry Excel wrote for something else.
    p.set(
        "xl/metadata.xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><metadata xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><metadataTypes count="1"><metadataType name="XLRICHVALUE" minSupportedVersion="120000"/></metadataTypes><futureMetadata name="XLRICHVALUE" count="1"><bk><extLst/></bk></futureMetadata><valueMetadata count="1"><bk><rc t="1" v="0"/></bk></valueMetadata></metadata>"#
            .to_string(),
    );
    assert_eq!(crate::xlsx_dynamic::metadata_index(&mut p).unwrap(), "1");
    let xml = p.text("xl/metadata.xml").unwrap().to_string();
    let d = parse(&xml).unwrap();
    let names: Vec<_> = d
        .root_element()
        .children()
        .filter(|n| n.is_element())
        .map(|n| n.tag_name().name())
        .collect();
    assert_eq!(
        names,
        [
            "metadataTypes",
            "futureMetadata",
            "futureMetadata",
            "cellMetadata",
            "valueMetadata"
        ]
    );
    assert!(xml.contains(r#"<metadataTypes count="2">"#), "{xml}");
    assert!(xml.contains(r#"<rc t="2" v="0"/>"#), "{xml}");
    // Asked again, the same entry is found rather than added.
    assert_eq!(crate::xlsx_dynamic::metadata_index(&mut p).unwrap(), "1");
    assert_eq!(p.text("xl/metadata.xml").unwrap(), xml);
}
