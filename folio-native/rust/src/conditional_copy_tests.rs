use crate::{
    conditional_copy, conditional_format as cf,
    office::{parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
fn core() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = c.workspace.clone();
    w["sheets"] = json!([
        {"id":"s","name":"Source","cells":{"A1":"0","A2":"25","A3":"75","A4":"100","B1":"3","D1":"5","D2":"9","D3":"99","Z1":"10"}},
        {"id":"t","name":"Target","cells":{"A1":"8","C1":"5","Z1":"10"}}
    ]);
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    c
}
fn add(c: &mut Core, sheet: &str, rule: Value) {
    c.dispatch(json!({"op":"conditionalFormat","sheetId":sheet,"baseRevision":c.workspace["revision"],"action":"add","rule":rule})).unwrap();
}
fn expression(range: &str, formula: &str, style: Value) -> Value {
    json!({"type":"expression","sqref":range,"formulas":[formula],"style":style})
}
fn copy(c: &mut Core, range: &str) -> Value {
    c.dispatch(json!({"op":"copyCells","sheetId":"s","range":range}))
        .unwrap()
}
fn paste(c: &mut Core, clip: &Value, range: &str, mode: &str) -> Value {
    c.dispatch(
        json!({"op":"pasteCells","sheetId":"s","range":range,"token":clip["token"],"mode":mode}),
    )
    .unwrap()
}
fn styles(c: &mut Core, sheet: &str, addresses: &[&str]) -> Value {
    let v=c.dispatch(json!({"op":"conditionalRange","sheetId":sheet,"baseRevision":c.workspace["revision"],"cells":addresses})).unwrap();
    assert_eq!(v["warnings"], json!([]));
    v["styles"].clone()
}
fn folder() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-conditional-copy-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn sheet_xml(pkg: &Package, index: usize) -> String {
    let sheets = crate::xlsx::import(pkg).unwrap().0;
    pkg.text(sheets[index]["officePath"].as_str().unwrap())
        .unwrap()
        .to_owned()
}
fn export(c: &mut Core, name: &str) -> Package {
    let p = folder().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":p}))
        .unwrap();
    Package::read(p.to_str().unwrap()).unwrap()
}
#[test]
fn conditional_copy_replaces_only_destinations_reanchors_formulas_and_preserves_history() {
    let mut c = core();
    add(
        &mut c,
        "s",
        expression(
            "D1:D1048576",
            "D1>=0",
            json!({"fill":"#0000FF","italic":true}),
        ),
    );
    add(
        &mut c,
        "s",
        expression("A1:A4", "A1>$Z$1", json!({"fill":"#FF0000"})),
    );
    let clip = copy(&mut c, "A2:A3");
    let before = c.workspace.clone();
    paste(&mut c, &clip, "D3", "all");
    let rules = &c.workspace["sheets"][0]["conditionalFormats"];
    assert_eq!(rules[0]["sqref"], "D3:D4");
    assert_eq!(rules[0]["formulas"], json!(["D3>$Z$1"]));
    assert_eq!(rules[2]["sqref"], "D1:D2 D5:D1048576");
    let s = styles(&mut c, "s", &["D2", "D3", "D4", "D5"]);
    assert_eq!(s["D3"]["fill"], "#FF0000");
    assert!(s["D3"]["italic"].is_null());
    assert_eq!(s["D2"]["fill"], "#0000FF");
    export(&mut c, "replace.xlsx");
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["sheets"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(styles(&mut c, "s", &["D3"])["D3"]["fill"], "#FF0000");
}
#[test]
fn conditional_paste_modes_merge_blanks_and_immutable_overlapping_rules() {
    for mode in [
        "all",
        "formats",
        "allMergeConditional",
        "values",
        "formulas",
    ] {
        let mut c = core();
        add(
            &mut c,
            "s",
            expression("D1:D4", "TRUE", json!({"italic":true,"fill":"#0000FF"})),
        );
        add(
            &mut c,
            "s",
            expression("A1:A4", "A1>=0", json!({"bold":true,"fill":"#FF0000"})),
        );
        let clip = copy(&mut c, "A2:A3");
        paste(&mut c, &clip, "D1", mode);
        let s = styles(&mut c, "s", &["D1"]);
        assert_eq!(
            s["D1"]["fill"],
            if mode == "values" || mode == "formulas" {
                "#0000FF"
            } else {
                "#FF0000"
            }
        );
        assert_eq!(
            s["D1"]["italic"],
            if matches!(mode, "allMergeConditional" | "values" | "formulas") {
                json!(true)
            } else {
                Value::Null
            }
        );
        assert_eq!(
            c.workspace["sheets"][0]["cells"]["D1"],
            if mode == "formats" { "5" } else { "25" }
        );
        export(&mut c, &format!("mode-{mode}.xlsx"));
    }
    let mut c = core();
    add(
        &mut c,
        "s",
        expression("A1:A4", "A1>10", json!({"fill":"#FF0000"})),
    );
    let clip = copy(&mut c, "A1:A3");
    let mut r = c.workspace["sheets"][0]["conditionalFormats"][0].clone();
    r["formulas"] = json!(["A1>90"]);
    c.dispatch(json!({"op":"conditionalFormat","sheetId":"s","baseRevision":c.workspace["revision"],"action":"edit","index":0,"rule":r})).unwrap();
    paste(&mut c, &clip, "A2", "all");
    assert_eq!(
        c.workspace["sheets"][0]["conditionalFormats"][0]["formulas"],
        json!(["A2>10"])
    );
    assert_eq!(
        c.workspace["sheets"][0]["conditionalFormats"][1]["sqref"],
        "A1:A1"
    );
    assert_eq!(
        styles(&mut c, "s", &["A2", "A3", "A4"])["A3"]["fill"],
        "#FF0000"
    );
    // Copying a blank cell with no rule replaces the destination rules too.
    let clip = copy(&mut c, "J10");
    paste(&mut c, &clip, "A3", "formats");
    assert!(styles(&mut c, "s", &["A3"])["A3"].is_null());
}
#[test]
fn conditional_transpose_tiling_skips_and_cross_sheet_context_follow_cell_mapping() {
    let mut c = core();
    add(
        &mut c,
        "s",
        expression("A1:A2", "A1>=0", json!({"bold":true})),
    );
    let clip = copy(&mut c, "A1:B2");
    c.dispatch(json!({"op":"pasteCells","sheetId":"s","range":"F5:I8","token":clip["token"],"transpose":true,"skipBlanks":true})).unwrap();
    let r = &c.workspace["sheets"][0]["conditionalFormats"][0];
    assert_eq!(r["sqref"], "F5:I5 F7:I7");
    assert_eq!(r["formulas"], json!(["F5>=0"]));
    let clip = copy(&mut c, "A2");
    c.dispatch(json!({"op":"pasteCells","sheetId":"t","range":"C1","token":clip["token"],"mode":"formats"})).unwrap();
    assert_eq!(styles(&mut c, "t", &["C1"])["C1"]["bold"], true);
    assert_eq!(
        c.workspace["sheets"][1]["conditionalFormats"][0]["formulas"],
        json!(["C1>=0"])
    );
    export(&mut c, "mapped.xlsx");
}
#[test]
fn conditional_graphics_copy_new_population_and_unique_extension_ids_through_recovery() {
    let mut c = core();
    add(
        &mut c,
        "s",
        json!({"type":"iconSet","sqref":"A1:A4","style":{},"iconSet":{"set":"3Stars","showValue":false,"thresholds":[{"type":"percent","value":"0"},{"type":"percent","value":"33"},{"type":"percent","value":"67"}]}}),
    );
    add(
        &mut c,
        "s",
        json!({"type":"dataBar","sqref":"A1:A4","style":{},"dataBar":{"extended":true,"color":"#638EC6","negativeFillColor":"#FF0000","axisColor":"#000000","minLength":0,"maxLength":100,"showValue":true,"gradient":true,"border":false,"axisPosition":"automatic","direction":"context","thresholds":[{"type":"autoMin","value":""},{"type":"autoMax","value":""}]}}),
    );
    add(
        &mut c,
        "s",
        json!({"type":"colorScale","sqref":"A1:A4","style":{},"scale":[{"type":"min","value":"","color":"#FF0000"},{"type":"max","value":"","color":"#00FF00"}]}),
    );
    let clip = copy(&mut c, "A2:A3");
    paste(&mut c, &clip, "D1", "all");
    let s = styles(&mut c, "s", &["D1", "D2"]);
    assert_eq!(s["D1"]["_iconSet"]["index"], 0);
    assert_eq!(s["D2"]["_iconSet"]["index"], 2);
    assert_eq!(s["D1"]["fill"], "#FF0000");
    assert_eq!(s["D2"]["fill"], "#00FF00");
    assert!(s["D1"]["_dataBar"].is_object());
    let pkg = export(&mut c, "graphics.xlsx");
    let xml = sheet_xml(&pkg, 0);
    let doc = parse(&xml).unwrap();
    let ids = doc
        .descendants()
        .filter(|n| n.tag_name().name() == "cfRule")
        .filter_map(|n| n.attribute("id"))
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 4);
    assert_eq!(
        ids.iter().collect::<std::collections::HashSet<_>>().len(),
        4
    );
    let path = folder().join("graphics.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut reopened = Core::new();
    reopened.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(styles(&mut reopened, "s", &["D1", "D2"]), s);
    export(&mut reopened, "graphics-recovered.xlsx");
    let clip = copy(&mut reopened, "A2:A3");
    reopened
        .dispatch(json!({"op":"pasteCells","sheetId":"t","range":"D1","token":clip["token"]}))
        .unwrap();
    let pkg = export(&mut reopened, "graphics-cross-sheet.xlsx");
    let mut all = vec![];
    for i in 0..2 {
        let xml = sheet_xml(&pkg, i);
        let doc = parse(&xml).unwrap();
        all.extend(
            doc.descendants()
                .filter(|n| n.tag_name().name() == "cfRule")
                .filter_map(|n| n.attribute("id"))
                .map(str::to_owned),
        );
    }
    assert_eq!(all.len(), 6);
    assert_eq!(
        all.iter().collect::<std::collections::HashSet<_>>().len(),
        6
    );
}
#[test]
fn conditional_directional_fill_compacts_rules_and_sparse_subtraction_reanchors_survivors() {
    for (direction, range, wanted) in [
        ("down", "A1:A1000", "A2:A1000"),
        ("right", "A1:E1", "B1:E1"),
        ("up", "A1:A4", "A1:A3"),
        ("left", "A1:D1", "A1:C1"),
    ] {
        let mut c = core();
        add(
            &mut c,
            "s",
            expression("A1:D4", "A1>=0", json!({"fill":"#00FF00"})),
        );
        c.dispatch(json!({"op":"fillCells","sheetId":"s","range":range,"direction":direction}))
            .unwrap();
        assert_eq!(
            c.workspace["sheets"][0]["conditionalFormats"][0]["sqref"],
            wanted
        );
        assert!(cf::rules(&c.workspace["sheets"][0]).count() <= 2);
        export(&mut c, &format!("fill-{direction}.xlsx"));
    }
    let mut c = core();
    add(
        &mut c,
        "s",
        expression("D1:D1048576", "D1>=0", json!({"italic":true})),
    );
    let clip = copy(&mut c, "J1");
    paste(&mut c, &clip, "D1:D3", "formats");
    let r = &c.workspace["sheets"][0]["conditionalFormats"][0];
    assert_eq!(r["sqref"], "D4:D1048576");
    assert_eq!(r["formulas"], json!(["D4>=0"]));
}
#[test]
fn conditional_unsupported_intersections_fail_atomically_but_unrelated_rules_survive() {
    let mut c = core();
    add(
        &mut c,
        "s",
        expression("A1:A4", "TRUE", json!({"bold":true})),
    );
    let mut w = c.workspace.clone();
    w["sheets"][0]["conditionalFormats"].as_array_mut().unwrap().push(json!({"type":"futureRule","sqref":"J1:J2","priority":2,"style":{},"unsupported":true,"officeXml":"<cfRule xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" type=\"futureRule\" priority=\"2\" future=\"keep\"/>"}));
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    let clip = copy(&mut c, "A1");
    paste(&mut c, &clip, "D1", "formats");
    let pkg = export(&mut c, "opaque-unrelated.xlsx");
    assert!(sheet_xml(&pkg, 0).contains("future=\"keep\""));
    let clip = copy(&mut c, "J1");
    let before = c.snapshot();
    assert!(c
        .dispatch(
            json!({"op":"pasteCells","sheetId":"s","range":"D1","token":clip["token"],"mode":"all"})
        )
        .is_err());
    assert_eq!(c.snapshot(), before);
    let clip = copy(&mut c, "A1");
    let before = c.snapshot();
    assert!(c.dispatch(json!({"op":"pasteCells","sheetId":"s","range":"J1","token":clip["token"],"mode":"formats"})).is_err());
    assert_eq!(c.snapshot(), before);
}
#[test]
fn conditional_threshold_mixed_references_cross_workbook_names_and_skipped_blanks() {
    let sheets = json!([{"id":"s","name":"Source","cells":{},"conditionalFormats":[expression("B2:B5","C2+$C2+C$2+$C$2>0",json!({"bold":true}))]}]);
    let mut target = json!({"id":"t","name":"Target","cells":{}});
    conditional_copy::copy_rules(
        &sheets,
        0,
        &mut target,
        &["Target".into()],
        0,
        &[("B5".into(), "E9".into())],
        false,
    )
    .unwrap();
    assert_eq!(
        target["conditionalFormats"][0]["formulas"],
        json!(["F9+$C9+F$2+$C$2>0"])
    );
    let mut foreign = sheets.clone();
    foreign[0]["conditionalFormats"][0]["formulas"] = json!(["Source!$A$1>0"]);
    let before = target.clone();
    assert!(conditional_copy::copy_rules(
        &foreign,
        0,
        &mut target,
        &["Target".into()],
        0,
        &[("B2".into(), "E9".into())],
        false
    )
    .is_err());
    assert_eq!(target, before);
    let mut c = core();
    add(
        &mut c,
        "s",
        expression("B1:B2", "TRUE", json!({"fill":"#FF0000"})),
    );
    add(
        &mut c,
        "s",
        expression("D1:D2", "TRUE", json!({"fill":"#0000FF"})),
    );
    let clip = copy(&mut c, "B1:B2");
    c.dispatch(json!({"op":"pasteCells","sheetId":"s","range":"D1","token":clip["token"],"skipBlanks":true})).unwrap();
    let s = styles(&mut c, "s", &["D1", "D2"]);
    assert_eq!(s["D1"]["fill"], "#FF0000");
    assert_eq!(s["D2"]["fill"], "#0000FF");
}

#[test]
fn conditional_copy_all_eighteen_types_from_an_independent_workbook() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/tests/fixtures/conditional-copy-openpyxl.xlsx")})).unwrap();
    let id = c.workspace["sheets"][0]["id"].clone();
    assert!(cf::rules(&c.workspace["sheets"][0]).all(|r| r["unsupported"] != true));
    let copied = c
        .dispatch(json!({"op":"copyCells","sheetId":id,"range":"A1:R4"}))
        .unwrap();
    c.dispatch(json!({"op":"pasteCells","sheetId":id,"range":"A10","token":copied["token"]}))
        .unwrap();
    assert_eq!(cf::rules(&c.workspace["sheets"][0]).count(), 36);
    for i in 0..18 {
        let a = crate::formula::address(9, i);
        let rule = &c.workspace["sheets"][0]["conditionalFormats"][i];
        assert_eq!(
            rule["sqref"],
            format!("{}:{}", a, crate::formula::address(12, i))
        );
    }
    export(&mut c, "independent-copied.xlsx");
}

#[test]
fn conditional_copied_threshold_formulas_recalculate_and_stop_if_true_survives_merge() {
    let mut c = core();
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"s","address":"Z2","value":"50"}]}))
        .unwrap();
    add(
        &mut c,
        "s",
        json!({"type":"iconSet","sqref":"A1:A4","style":{},"iconSet":{"set":"3Stars","thresholds":[{"type":"num","value":"0"},{"type":"formula","value":"$Z$1"},{"type":"formula","value":"$Z$2"}]}}),
    );
    add(
        &mut c,
        "s",
        json!({"type":"dataBar","sqref":"A1:A4","style":{},"dataBar":{"extended":true,"color":"#638EC6","negativeFillColor":"#FF0000","axisColor":"#000000","minLength":0,"maxLength":100,"showValue":true,"gradient":false,"border":false,"axisPosition":"automatic","direction":"context","thresholds":[{"type":"formula","value":"$Z$1"},{"type":"formula","value":"$Z$2"}]}}),
    );
    add(
        &mut c,
        "s",
        json!({"type":"colorScale","sqref":"A1:A4","style":{},"scale":[{"type":"formula","value":"A1","color":"#FF0000"},{"type":"formula","value":"$Z$2","color":"#00FF00"}]}),
    );
    let clip = copy(&mut c, "A2:A3");
    paste(&mut c, &clip, "D1", "all");
    assert_eq!(
        c.workspace["sheets"][0]["conditionalFormats"][0]["scale"][0]["value"],
        "D1"
    );
    let before = styles(&mut c, "s", &["D1", "D2"]);
    assert_eq!(before["D1"]["fill"], "#FF0000");
    assert_eq!(before["D2"]["fill"], "#00FF00");
    assert_eq!(before["D2"]["_iconSet"]["index"], 2);
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"s","address":"Z2","value":"100"}]}))
        .unwrap();
    let after = styles(&mut c, "s", &["D1", "D2"]);
    assert_eq!(after["D2"]["_iconSet"]["index"], 1);
    assert_ne!(after["D2"]["fill"], before["D2"]["fill"]);
    assert_ne!(after["D2"]["_dataBar"], before["D2"]["_dataBar"]);
    export(&mut c, "threshold-formulas.xlsx");
    let mut c = core();
    add(
        &mut c,
        "s",
        expression("D1:D4", "TRUE", json!({"italic":true})),
    );
    let mut r = expression("A1:A4", "TRUE", json!({"bold":true}));
    r["stopIfTrue"] = json!(true);
    add(&mut c, "s", r);
    let clip = copy(&mut c, "A2");
    paste(&mut c, &clip, "D1", "allMergeConditional");
    let s = styles(&mut c, "s", &["D1"]);
    assert_eq!(s["D1"]["bold"], true);
    assert!(s["D1"]["italic"].is_null());
}
