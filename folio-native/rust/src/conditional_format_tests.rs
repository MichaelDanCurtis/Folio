use crate::{
    office::{child, parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
fn blank() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c
}
fn id(c: &Core) -> String {
    c.workspace["sheets"][0]["id"].as_str().unwrap().into()
}
fn cells(c: &mut Core, v: Value) {
    let sid = id(c);
    c.dispatch(json!({"op":"cells","cells":v.as_object().unwrap().iter().map(|(a,v)|json!({"sheetId":sid,"address":a,"value":v})).collect::<Vec<_>>()})).unwrap();
}
fn command(c: &mut Core, r: Value) -> Result<Value, String> {
    let mut r = r;
    r["op"] = json!("conditionalFormat");
    r["sheetId"] = json!(id(c));
    r["baseRevision"] = c.workspace["revision"].clone();
    c.dispatch(r)
}
fn add(c: &mut Core, r: Value) {
    command(c, json!({"action":"add","rule":r})).unwrap();
}
fn view(c: &mut Core, addresses: &[&str]) -> Value {
    c.dispatch(json!({"op":"conditionalRange","sheetId":id(c),"baseRevision":c.workspace["revision"],"cells":addresses})).unwrap()
}
fn export(c: &mut Core, name: &str) -> Package {
    let dir = std::env::temp_dir().join("folio-conditional-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
#[test]
fn conditional_priority_property_merging_stop_and_atomic_history() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"3","A2":"12","A3":"=A1*2"}));
    add(
        &mut c,
        json!({"sqref":"A1:A3","type":"cellIs","operator":"greaterThan","formulas":["5"],"style":{"fill":"#FF0000","bold":true}}),
    );
    add(
        &mut c,
        json!({"sqref":"A1:A3","type":"expression","formulas":["A1>0"],"style":{"fill":"#00FF00","numFmt":"0.00"}}),
    );
    let before = c.snapshot();
    let sid = id(&c);
    assert_eq!(
        before["formatted"][&sid]["A2"]["conditionalStyle"],
        json!({"fill":"#00FF00","bold":true,"numFmt":"0.00"})
    );
    assert_eq!(before["formatted"][&sid]["A2"]["text"], "12.00");
    assert!(c.workspace["sheets"][0]["cellStyles"]["A2"].is_null());
    let mut rule = c.workspace["sheets"][0]["conditionalFormats"][0].clone();
    rule["stopIfTrue"] = json!(true);
    command(&mut c, json!({"action":"edit","index":0,"rule":rule})).unwrap();
    assert!(c.snapshot()["formatted"][&sid]["A2"]["conditionalStyle"]["bold"].is_null());
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        c.snapshot()["workspace"]["sheets"],
        before["workspace"]["sheets"]
    );
    command(&mut c, json!({"action":"move","index":1,"to":0})).unwrap();
    assert_eq!(view(&mut c, &["A2"])["styles"]["A2"]["fill"], "#FF0000");
    cells(&mut c, json!({"A2":"-4"}));
    assert!(view(&mut c, &["A2"])["styles"]["A2"].is_null());
    assert_eq!(c.snapshot()["formatted"][&sid]["A2"]["text"], "-4");
}
#[test]
fn conditional_relative_formulas_blanks_errors_and_full_column_are_read_only() {
    let mut c = blank();
    cells(
        &mut c,
        json!({"A2":"9","B2":"8","A3":"3","B3":"4","A4":"=1/0","B4":"#DIV/0!","C2":"'😀 cat"}),
    );
    add(
        &mut c,
        json!({"sqref":"A2:A1048576","type":"expression","formulas":["AND(A2>$B2,ROW()>1)"],"style":{"italic":true}}),
    );
    let before = c.snapshot();
    let result = view(&mut c, &["A2", "A3", "A4", "A1048576"]);
    assert_eq!(result["styles"], json!({"A2":{"italic":true}}));
    assert_eq!(c.snapshot(), before);
    add(
        &mut c,
        json!({"sqref":"A2:B5","type":"containsErrors","style":{"fill":"#FF0000"}}),
    );
    assert_eq!(
        view(&mut c, &["A4", "B4"])["styles"]["A4"]["fill"],
        "#FF0000"
    );
    add(
        &mut c,
        json!({"sqref":"C2:C3","type":"containsText","text":"CAT","style":{"bold":true}}),
    );
    assert_eq!(view(&mut c, &["C2"])["styles"]["C2"]["bold"], true);
    add(
        &mut c,
        json!({"sqref":"Z1:Z1048576","type":"containsBlanks","style":{"fill":"#EEEEEE"}}),
    );
    assert_eq!(
        view(&mut c, &["Z1048576"])["styles"]["Z1048576"]["fill"],
        "#EEEEEE"
    );
}
#[test]
fn conditional_comparison_operators_and_boolean_formula_results() {
    for (op, f, yes, no) in [
        ("equal", vec!["5"], "5", "4"),
        ("notEqual", vec!["5"], "4", "5"),
        ("greaterThan", vec!["5"], "6", "5"),
        ("greaterThanOrEqual", vec!["5"], "5", "4"),
        ("lessThan", vec!["5"], "4", "5"),
        ("lessThanOrEqual", vec!["5"], "5", "6"),
        ("between", vec!["4", "6"], "5", "7"),
        ("notBetween", vec!["4", "6"], "7", "5"),
    ] {
        let mut c = blank();
        cells(&mut c, json!({"A1":yes,"A2":no}));
        add(
            &mut c,
            json!({"sqref":"A1:A2","type":"cellIs","operator":op,"formulas":f,"style":{"bold":true}}),
        );
        assert_eq!(
            view(&mut c, &["A1", "A2"])["styles"],
            json!({"A1":{"bold":true}}),
            "{op}"
        );
    }
    let mut c = blank();
    cells(&mut c, json!({"A1":"1","A2":"0","A3":"-1"}));
    add(
        &mut c,
        json!({"sqref":"A1:A3","type":"expression","formulas":["A1"],"style":{"bold":false}}),
    );
    assert_eq!(
        view(&mut c, &["A1", "A2", "A3"])["styles"],
        json!({"A1":{"bold":false},"A3":{"bold":false}})
    );
}
#[test]
fn conditional_rank_average_duplicate_and_scale_use_whole_rule_population() {
    let mut c = blank();
    cells(
        &mut c,
        json!({"A1":"10","A2":"20","A3":"20","A4":"40","A5":"'20","A6":"text","A7":"TEXT"}),
    );
    add(
        &mut c,
        json!({"sqref":"A1:A7","type":"top10","rank":2,"style":{"bold":true}}),
    );
    assert_eq!(view(&mut c, &["A2"])["styles"]["A2"]["bold"], true);
    add(
        &mut c,
        json!({"sqref":"A1:A7","type":"duplicateValues","style":{"italic":true}}),
    );
    let v = view(&mut c, &["A2", "A5", "A6"]);
    assert_eq!(v["styles"]["A2"]["italic"], true);
    assert!(v["styles"]["A5"].is_null());
    assert_eq!(v["styles"]["A6"]["italic"], true);
    add(
        &mut c,
        json!({"sqref":"A1:A7","type":"aboveAverage","style":{"strike":true}}),
    );
    assert_eq!(view(&mut c, &["A2", "A4"])["styles"]["A4"]["strike"], true);
    assert!(view(&mut c, &["A2"])["styles"]["A2"]["strike"].is_null());
    add(
        &mut c,
        json!({"sqref":"A1:A4","type":"colorScale","style":{},"scale":[{"type":"min","color":"#FFFFFF"},{"type":"max","color":"#000000"}]}),
    );
    let v = view(&mut c, &["A1", "A2", "A4"]);
    assert_eq!(v["styles"]["A1"]["fill"], "#FFFFFF");
    assert_eq!(v["styles"]["A2"]["fill"], "#AAAAAA");
    assert_eq!(v["styles"]["A4"]["fill"], "#000000");
    cells(&mut c, json!({"A4":"30"}));
    assert_eq!(view(&mut c, &["A2"])["styles"]["A2"]["fill"], "#808080");
}
#[test]
fn conditional_invalid_or_stale_changes_are_atomic() {
    let mut c = blank();
    let before = c.snapshot();
    for r in [
        json!({"sqref":"A1","type":"expression","formulas":["SUM("],"style":{"bold":true}}),
        json!({"sqref":"A1","type":"cellIs","operator":"equal","formulas":[],"style":{}}),
        json!({"sqref":"A1","type":"top10","rank":0,"style":{}}),
        json!({"sqref":"A1","type":"expression","formulas":["TRUE"],"style":{"fill":"bad"}}),
    ] {
        assert!(command(&mut c, json!({"action":"add","rule":r})).is_err());
        assert_eq!(c.snapshot(), before);
    }
    assert!(c.dispatch(json!({"op":"conditionalFormat","sheetId":id(&c),"baseRevision":900,"action":"delete","index":0})).is_err());
    assert_eq!(c.snapshot(), before);
}
#[test]
fn conditional_xlsx_roundtrip_differential_styles_recovery_and_structure() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"5","A2":"20"}));
    add(
        &mut c,
        json!({"sqref":"A1:A2 C1:C2","type":"expression","formulas":["A1>10"],"style":{"fill":"#FFC7CE","color":"#9C0006","bold":false,"numFmt":"0.00%"},"stopIfTrue":true}),
    );
    let pkg = export(&mut c, "rules.xlsx");
    let (sheets, _) = crate::xlsx::import(&pkg).unwrap();
    let r = &sheets[0]["conditionalFormats"][0];
    assert_eq!(
        r["style"],
        c.workspace["sheets"][0]["conditionalFormats"][0]["style"]
    );
    assert_ne!(r["unsupported"], true);
    let path = sheets[0]["officePath"].as_str().unwrap();
    let doc = parse(pkg.text(path).unwrap()).unwrap();
    assert!(child(doc.root_element(), "conditionalFormatting").is_some());
    let temp = std::env::temp_dir().join("folio-conditional-tests/recovery.folio");
    c.dispatch(json!({"op":"save","path":temp})).unwrap();
    let before = c.snapshot();
    let mut other = blank();
    other.dispatch(json!({"op":"load","path":temp})).unwrap();
    assert_eq!(other.workspace["sheets"], before["workspace"]["sheets"]);
    assert_eq!(other.snapshot()["formatted"], before["formatted"]);
    c.dispatch(json!({"op":"structure","action":"insertRows","sheetId":id(&c),"at":1,"count":1}))
        .unwrap();
    let r = &c.workspace["sheets"][0]["conditionalFormats"][0];
    assert_eq!(r["sqref"], "A2:A3 C2:C3");
    assert_eq!(r["formulas"][0], "A2>10");
    export(&mut c, "rules-shifted.xlsx");
}
#[test]
fn conditional_source_extensions_and_unknown_rules_survive_unrelated_edits() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"1"}));
    let mut pkg = export(&mut c, "source-base.xlsx");
    let sheets = crate::xlsx::import(&pkg).unwrap().0;
    let path = sheets[0]["officePath"].as_str().unwrap().to_string();
    let xml=pkg.text(&path).unwrap().replace("</worksheet>","<conditionalFormatting xmlns:z=\"urn:future\" sqref=\"A1:A20\" z:label=\"Keep &gt; 1\"><cfRule type=\"dataBar\" priority=\"1\" z:token=\"keep\"><dataBar><cfvo type=\"min\"/><cfvo type=\"max\"/><color rgb=\"FF638EC6\"/></dataBar><extLst><ext uri=\"future\"><z:thing/></ext></extLst></cfRule></conditionalFormatting></worksheet>");
    pkg.set(&path, xml);
    let mut sheets = crate::xlsx::import(&pkg).unwrap().0;
    assert_eq!(sheets[0]["conditionalFormats"][0]["unsupported"], true);
    let baseline = sheets.clone();
    sheets[0]["conditionalFormats"][0]["priority"] = json!(2);
    crate::conditional_format::write(&mut pkg, &sheets[0], Some(&baseline[0]), &path).unwrap();
    let xml = pkg.text(&path).unwrap();
    assert!(xml.contains("dataBar"));
    assert!(xml.contains("z:thing"));
    assert!(xml.contains("Keep &gt; 1"));
    assert!(xml.contains("z:token=\"keep\""));
    assert!(xml.contains("priority=\"2\""));
    parse(xml).unwrap();
}

#[test]
fn conditional_batch_is_one_atomic_edit_and_failed_batch_keeps_state() {
    let mut c = blank();
    let before = c.snapshot();
    let r = json!({"sqref":"A1:A3","type":"containsText","text":"a\"b","style":{"bold":true}});
    command(&mut c,json!({"action":"batch","changes":[{"action":"add","rule":r},{"action":"add","rule":{"sqref":"B1:B5","type":"containsBlanks","style":{"fill":"#EEEEEE"}}},{"action":"move","index":1,"to":0}]})).unwrap();
    assert_eq!(
        c.workspace["revision"].as_u64(),
        Some(before["workspace"]["revision"].as_u64().unwrap() + 1)
    );
    assert_eq!(
        c.workspace["sheets"][0]["conditionalFormats"][0]["formulas"][0],
        "NOT(ISERROR(SEARCH(\"a\"\"b\",A1)))"
    );
    assert_eq!(
        c.workspace["sheets"][0]["conditionalFormats"][0]["operator"],
        "containsText"
    );
    let stable = c.snapshot();
    assert!(command(&mut c,json!({"action":"batch","changes":[{"action":"delete","index":0},{"action":"delete","index":50}]})).is_err());
    assert_eq!(c.snapshot(), stable);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["workspace"]["sheets"]);
}
#[test]
fn conditional_three_color_formula_thresholds_and_priority_over_scale() {
    let mut c = blank();
    cells(
        &mut c,
        json!({"A1":"0","A2":"25","A3":"50","A4":"75","A5":"100","B1":"50"}),
    );
    add(
        &mut c,
        json!({"sqref":"A1:A5","type":"colorScale","style":{},"scale":[{"type":"min","color":"#FF0000"},{"type":"formula","value":"$B$1","color":"#FFFFFF"},{"type":"max","color":"#00FF00"}]}),
    );
    let v = view(&mut c, &["A2", "A3", "A4"]);
    assert_eq!(v["styles"]["A2"]["fill"], "#FF8080");
    assert_eq!(v["styles"]["A3"]["fill"], "#FFFFFF");
    assert_eq!(v["styles"]["A4"]["fill"], "#80FF80");
    add(
        &mut c,
        json!({"sqref":"A1:A5","type":"cellIs","operator":"equal","formulas":["50"],"style":{"fill":"#0000FF"}}),
    );
    assert_eq!(view(&mut c, &["A3"])["styles"]["A3"]["fill"], "#0000FF");
}
#[test]
fn conditional_protected_and_extended_sheets_reject_rule_edits() {
    let mut c = blank();
    let base = export(&mut c, "guard-base.xlsx");
    let sheets = crate::xlsx::import(&base).unwrap().0;
    let path = sheets[0]["officePath"].as_str().unwrap();
    for (name,tag) in [("protected","<sheetProtection sheet=\"1\"/>"),("extended","<extLst><ext uri=\"future\"><x:conditionalFormatting xmlns:x=\"urn:x14\"/></ext></extLst>")] {
  let mut pkg=base.clone();pkg.set(path,pkg.text(path).unwrap().replace("</worksheet>",&format!("{tag}</worksheet>")));
  let file=std::env::temp_dir().join(format!("folio-conditional-tests/{name}.xlsx"));std::fs::write(&file,pkg.bytes().unwrap()).unwrap();
  c.dispatch(json!({"op":"importOffice","path":file})).unwrap();let before=c.snapshot();assert!(command(&mut c,json!({"action":"add","rule":{"sqref":"A1","type":"expression","formulas":["TRUE"],"style":{"bold":true}}})).is_err());assert_eq!(c.snapshot(),before);
 }
}

#[test]
fn conditional_replacing_reviewed_rules_validates_final_draft_and_read_only_check() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"7"}));
    add(
        &mut c,
        json!({"sqref":"A1:A2","type":"containsErrors","style":{"fill":"#FF0000"}}),
    );
    let original = c.workspace["sheets"][0]["conditionalFormats"][0].clone();
    let proposed = json!([{"origin":null,"rule":{"sqref":"A1:A2","type":"cellIs","operator":"greaterThan","formulas":["5"],"style":{"fill":"#00FF00"}}},{"origin":0,"rule":original}]);
    let before = c.snapshot();
    let request = json!({"op":"conditionalCheck","sheetId":id(&c),"baseRevision":c.workspace["revision"],"action":"replace","rules":proposed});
    assert_eq!(c.dispatch(request).unwrap(), json!({"valid":true}));
    assert_eq!(c.snapshot(), before);
    command(&mut c, json!({"action":"replace","rules":proposed})).unwrap();
    assert_eq!(view(&mut c, &["A1"])["styles"]["A1"]["fill"], "#00FF00");
    let stable = c.snapshot();
    assert!(command(&mut c,json!({"action":"replace","rules":[{"origin":0,"rule":original},{"origin":0,"rule":original}]})).is_err());
    assert_eq!(c.snapshot(), stable);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["workspace"]["sheets"]);
}

#[test]
fn conditional_import_edit_preserves_vendor_style_metadata_and_rebinds_structure() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"5","A2":"20"}));
    add(
        &mut c,
        json!({"sqref":"A1:A2","type":"cellIs","operator":"greaterThan","formulas":["10"],"style":{"fill":"#FF0000","bold":true}}),
    );
    let mut pkg = export(&mut c, "metadata-base.xlsx");
    let xml = pkg
        .text("xl/styles.xml")
        .unwrap()
        .replace(
            "<dxf xmlns=",
            "<dxf xmlns:z=\"urn:folio:future\" z:keep=\"root\" xmlns=",
        )
        .replace(
            "<b val=\"1\"/>",
            "<b xmlns:z=\"urn:folio:future\" z:keep=\"font\" val=\"1\"/>",
        )
        .replace(
            "<fgColor rgb=\"FFFF0000\"/>",
            "<fgColor xmlns:z=\"urn:folio:future\" z:keep=\"fill\" rgb=\"FFFF0000\"/>",
        );
    pkg.set("xl/styles.xml", xml);
    let path = std::env::temp_dir().join("folio-conditional-tests/metadata-source.xlsx");
    std::fs::write(&path, pkg.bytes().unwrap()).unwrap();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let mut rule = c.workspace["sheets"][0]["conditionalFormats"][0].clone();
    assert_ne!(rule["unsupported"], true);
    rule["style"]["fill"] = json!("#00FF00");
    rule["style"]["bold"] = json!(false);
    command(&mut c, json!({"action":"edit","index":0,"rule":rule})).unwrap();
    let edited = export(&mut c, "metadata-edited.xlsx");
    let styles = edited.text("xl/styles.xml").unwrap();
    let d = parse(styles).unwrap();
    let ds = child(d.root_element(), "dxfs").unwrap();
    let last = ds.children().filter(|n| n.is_element()).last().unwrap();
    assert_eq!(last.attribute(("urn:folio:future", "keep")), Some("root"));
    let font = child(last, "font").unwrap();
    assert_eq!(
        child(font, "b")
            .unwrap()
            .attribute(("urn:folio:future", "keep")),
        Some("font")
    );
    let color = child(
        child(child(last, "fill").unwrap(), "patternFill").unwrap(),
        "fgColor",
    )
    .unwrap();
    assert_eq!(color.attribute(("urn:folio:future", "keep")), Some("fill"));
    assert_eq!(color.attribute("rgb"), Some("FF00FF00"));
    c.dispatch(json!({"op":"structure","sheetId":id(&c),"action":"insertRows","at":1,"count":1}))
        .unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["conditionalFormats"][0]["sqref"],
        "A2:A3"
    );
    assert_eq!(view(&mut c, &["A3"])["styles"]["A3"]["fill"], "#00FF00");
    export(&mut c, "metadata-shifted.xlsx");
}

fn data_bar(extended: bool) -> Value {
    json!({"sqref":"A1:A1048576","type":"dataBar","style":{},"dataBar":{
        "extended":extended,"color":"#638EC6","negativeFillColor":"#FF0000","axisColor":"#000000",
        "minLength":if extended {0} else {10},"maxLength":if extended {100} else {90},
        "showValue":true,"gradient":true,"border":false,"axisPosition":if extended {"automatic"} else {"none"},"direction":"context",
        "thresholds":[{"type":if extended {"autoMin"} else {"min"},"value":""},{"type":if extended {"autoMax"} else {"max"},"value":""}]}})
}
fn near(value: &Value, expected: f64) {
    assert!(
        (value.as_f64().unwrap() - expected).abs() < 1e-10,
        "{value} != {expected}"
    );
}
#[test]
fn data_bars_classic_lengths_numeric_types_and_no_viewport_side_effects() {
    let mut c = blank();
    cells(
        &mut c,
        json!({"A1":"-20","A2":"0","A3":"40","A4":"80","A5":"'40","A6":"TRUE","A7":"=1/0","A8":"=A3*2"}),
    );
    add(&mut c, data_bar(false));
    let before = c.snapshot();
    let v = view(
        &mut c,
        &["A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A1048576"],
    );
    for (a, n) in [
        ("A1", 0.1),
        ("A2", 0.26),
        ("A3", 0.58),
        ("A4", 0.9),
        ("A8", 0.9),
    ] {
        near(&v["styles"][a]["_dataBar"]["end"], n);
        assert!(v["styles"][a]["_dataBar"]["axis"].is_null());
    }
    for a in ["A5", "A6", "A7", "A1048576"] {
        assert!(v["styles"][a].is_null());
    }
    assert_eq!(c.snapshot(), before);
    assert!(c.workspace["sheets"][0]["cellStyles"]["A1"].is_null());
}
#[test]
fn data_bars_mixed_sign_axis_recalculation_priority_and_history() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"-20","A2":"0","A3":"40","A4":"80"}));
    add(&mut c, data_bar(true));
    let v = view(&mut c, &["A1", "A2", "A3", "A4"]);
    for a in ["A1", "A2", "A3", "A4"] {
        near(&v["styles"][a]["_dataBar"]["axis"], 0.2);
    }
    near(&v["styles"]["A1"]["_dataBar"]["end"], 0.0);
    near(&v["styles"]["A4"]["_dataBar"]["end"], 1.0);
    near(&v["styles"]["A3"]["_dataBar"]["end"], 0.6);
    near(&v["styles"]["A2"]["_dataBar"]["end"], 0.2);
    assert_eq!(v["styles"]["A1"]["_dataBar"]["color"], "#FF0000");
    cells(&mut c, json!({"A4":"160"}));
    near(
        &view(&mut c, &["A3"])["styles"]["A3"]["_dataBar"]["axis"],
        1.0 / 9.0,
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(view(&mut c, &["A1", "A2", "A3", "A4"]), v);
    add(
        &mut c,
        json!({"type":"expression","sqref":"A1:A4","formulas":["A1>0"],"style":{"fill":"#EEEEEE"},"stopIfTrue":true}),
    );
    let v = view(&mut c, &["A1", "A3"]);
    assert!(v["styles"]["A3"]["_dataBar"].is_null());
    assert!(v["styles"]["A1"]["_dataBar"].is_object());
}
#[test]
fn data_bars_direction_midpoint_bar_only_and_negative_border() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"-10","A2":"10"}));
    let mut r = data_bar(true);
    r["dataBar"]["axisPosition"] = json!("middle");
    r["dataBar"]["direction"] = json!("rightToLeft");
    r["dataBar"]["showValue"] = json!(false);
    r["dataBar"]["border"] = json!(true);
    r["dataBar"]["negativeBarBorderColorSameAsPositive"] = json!(false);
    r["dataBar"]["negativeBorderColor"] = json!("#800000");
    add(&mut c, r);
    let v = view(&mut c, &["A1", "A2"]);
    let b = &v["styles"]["A1"]["_dataBar"];
    near(&b["axis"], 0.5);
    near(&b["end"], 1.0);
    assert_eq!(b["borderColor"], "#800000");
    assert_eq!(b["showValue"], false);
    near(&v["styles"]["A2"]["_dataBar"]["end"], 0.0);
    assert_eq!(v["formatted"]["A1"]["text"], "-10");
    assert_eq!(c.workspace["sheets"][0]["cells"]["A1"], "-10");
}
#[test]
fn data_bars_all_negative_constant_zero_percentile_and_formula_threshold_errors() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"-10","A2":"-5"}));
    add(&mut c, data_bar(true));
    let v = view(&mut c, &["A1", "A2"]);
    near(&v["styles"]["A1"]["_dataBar"]["axis"], 1.0);
    near(&v["styles"]["A2"]["_dataBar"]["end"], 0.5);
    cells(&mut c, json!({"A1":"0","A2":"0"}));
    let v = view(&mut c, &["A1"]);
    near(&v["styles"]["A1"]["_dataBar"]["end"], 0.0);
    cells(&mut c, json!({"A1":"10","A2":"10"}));
    near(
        &view(&mut c, &["A1"])["styles"]["A1"]["_dataBar"]["end"],
        1.0,
    );
    let mut r = data_bar(true);
    r["dataBar"]["thresholds"] =
        json!([{"type":"formula","value":"$B$1"},{"type":"formula","value":"$B$2"}]);
    cells(&mut c, json!({"A1":"25","A2":"75","B1":"0","B2":"100"}));
    command(&mut c, json!({"action":"edit","index":0,"rule":r})).unwrap();
    near(
        &view(&mut c, &["A1"])["styles"]["A1"]["_dataBar"]["end"],
        0.25,
    );
    cells(&mut c, json!({"B2":"=1/0"}));
    let v = view(&mut c, &["A1", "A2"]);
    assert_eq!(v["styles"], json!({}));
    assert_eq!(v["warnings"].as_array().unwrap().len(), 1);
    let mut r = data_bar(true);
    r["dataBar"]["axisPosition"] = json!("none");
    r["dataBar"]["thresholds"] =
        json!([{"type":"percentile","value":"25"},{"type":"percent","value":"75"}]);
    cells(
        &mut c,
        json!({"A1":"0","A2":"25","A3":"50","A4":"75","A5":"100"}),
    );
    command(&mut c, json!({"action":"edit","index":0,"rule":r})).unwrap();
    near(
        &view(&mut c, &["A3"])["styles"]["A3"]["_dataBar"]["end"],
        0.5,
    );
}
#[test]
fn data_bars_invalid_settings_rejected_atomically() {
    let mut c = blank();
    let before = c.snapshot();
    for (key, v) in [
        ("minLength", json!(-1)),
        ("maxLength", json!(101)),
        ("maxLength", json!(1.5)),
        ("direction", json!("sideways")),
        ("color", json!("automatic")),
        ("border", json!("yes")),
        ("axisPosition", json!("edge")),
    ] {
        let mut r = data_bar(true);
        r["dataBar"][key] = v;
        assert!(
            command(&mut c, json!({"action":"add","rule":r})).is_err(),
            "{key}"
        );
        assert_eq!(c.snapshot(), before);
    }
    for f in ["A1", "SUM($A$1:$A$3)", "{1,2}", "SUM(", "Table1[Amount]"] {
        let mut r = data_bar(true);
        r["dataBar"]["thresholds"][0] = json!({"type":"formula","value":f});
        assert!(
            command(&mut c, json!({"action":"add","rule":r})).is_err(),
            "{f}"
        );
        assert_eq!(c.snapshot(), before);
    }
}
#[test]
fn data_bars_ooxml_edit_structure_delete_and_recovery() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"-20","A2":"80","B1":"-40","B2":"100"}));
    let mut r = data_bar(true);
    r["sqref"] = json!("A1:A2 C1:C2");
    r["dataBar"]["thresholds"] =
        json!([{"type":"formula","value":"$B$1"},{"type":"formula","value":"$B$2"}]);
    add(&mut c, r);
    let pkg = export(&mut c, "data-bars.xlsx");
    let (s, _) = crate::xlsx::import(&pkg).unwrap();
    let r = &s[0]["conditionalFormats"][0];
    assert_ne!(s[0]["extendedConditionalFormats"], true);
    assert_ne!(r["unsupported"], true);
    assert_eq!(r["dataBar"]["minLength"], 0);
    assert_eq!(r["dataBar"]["maxLength"], 100);
    let xml = pkg.text(s[0]["officePath"].as_str().unwrap()).unwrap();
    let d = parse(xml).unwrap();
    let standard = d
        .descendants()
        .find(|n| n.has_tag_name((crate::conditional_data_bar::S, "dataBar")))
        .unwrap();
    assert_eq!(standard.attribute("minLength"), Some("10"));
    let temp = std::env::temp_dir().join("folio-conditional-tests/data-bars.folio");
    c.dispatch(json!({"op":"save","path":temp})).unwrap();
    let before = c.snapshot();
    let mut other = blank();
    other.dispatch(json!({"op":"load","path":temp})).unwrap();
    assert_eq!(other.snapshot()["formatted"], before["formatted"]);
    c.dispatch(json!({"op":"structure","action":"insertRows","sheetId":id(&c),"at":1,"count":1}))
        .unwrap();
    let r = &c.workspace["sheets"][0]["conditionalFormats"][0];
    assert_eq!(r["sqref"], "A2:A3 C2:C3");
    assert_eq!(r["dataBar"]["thresholds"][0]["value"], "$B$2");
    assert_eq!(r["dataBar"]["thresholds"][1]["value"], "$B$3");
    let mut r = r.clone();
    export(&mut c, "data-bars-shifted.xlsx");
    r["dataBar"]["showValue"] = json!(false);
    r["dataBar"]["direction"] = json!("rightToLeft");
    command(&mut c, json!({"action":"edit","index":0,"rule":r})).unwrap();
    export(&mut c, "data-bars-edited.xlsx");
    command(&mut c, json!({"action":"delete","index":0})).unwrap();
    let pkg = export(&mut c, "data-bars-deleted.xlsx");
    let deleted = crate::xlsx::import(&pkg).unwrap().0;
    let xml = pkg
        .text(deleted[0]["officePath"].as_str().unwrap())
        .unwrap();
    assert!(!xml.contains("dataBar") && !xml.contains("x14:id"));
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["conditionalFormats"][0]["dataBar"]["showValue"],
        false
    );
}
#[test]
fn data_bars_xlsxwriter_import_linked_settings_and_read_only_display() {
    let pkg =
        Package::from_bytes(include_bytes!("../tests/fixtures/data-bars.xlsx").to_vec()).unwrap();
    let (s, _) = crate::xlsx::import(&pkg).unwrap();
    assert_eq!(s[0]["conditionalRightToLeft"], true);
    assert_eq!(s[0]["extendedConditionalFormats"], false);
    let rules = s[0]["conditionalFormats"].as_array().unwrap();
    assert_eq!(rules.len(), 6);
    for r in rules {
        assert_ne!(r["unsupported"], true, "{r}");
    }
    assert_eq!(rules[0]["dataBar"]["extended"], false);
    let b = &rules[1]["dataBar"];
    assert_eq!(b["showValue"], false);
    assert_eq!(b["gradient"], false);
    assert_eq!(b["negativeFillColor"], "#C00000");
    assert_eq!(b["negativeBorderColor"], "#800000");
    assert_eq!(rules[4]["dataBar"]["thresholds"][0]["value"], "$H$1");
    let mut calc = crate::calculation::WorkbookCalculator::default();
    calc.sync(&s);
    let v =
        crate::conditional_format::evaluate(&s, 0, &["B1".into(), "B4".into()], &mut calc).unwrap();
    near(&v["styles"]["B1"]["_dataBar"]["axis"], 0.8);
    near(&v["styles"]["B1"]["_dataBar"]["end"], 1.0);
    let mut after = s[0].clone();
    after["conditionalFormats"][1]["dataBar"]["negativeFillColor"] = json!("#8800AA");
    let mut out = pkg.clone();
    crate::conditional_format::write(
        &mut out,
        &after,
        Some(&s[0]),
        s[0]["officePath"].as_str().unwrap(),
    )
    .unwrap();
    let s2 = crate::xlsx::import(&out).unwrap().0;
    assert_eq!(
        s2[0]["conditionalFormats"][1]["dataBar"]["negativeFillColor"],
        "#8800AA"
    );
    assert_eq!(s2[0]["conditionalFormats"].as_array().unwrap().len(), 6);
    let path =
        std::env::temp_dir().join("folio-conditional-tests/data-bars-xlsxwriter-edited.xlsx");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    crate::office::write_bytes(path.to_str().unwrap(), &out.bytes().unwrap()).unwrap();
}

#[test]
fn data_bars_standalone_extension_and_legacy_recovery_upgrade() {
    use crate::conditional_data_bar::{S, X14};
    let mut pkg =
        Package::from_bytes(include_bytes!("../tests/fixtures/data-bars.xlsx").to_vec()).unwrap();
    let fresh = crate::xlsx::import(&pkg).unwrap().0;
    let path = fresh[0]["officePath"].as_str().unwrap();
    let mut old = fresh[0].clone();
    old["conditionalFormats"].as_array_mut().unwrap().remove(5);
    for r in old["conditionalFormats"].as_array_mut().unwrap() {
        r.as_object_mut().unwrap().remove("dataBar");
        r.as_object_mut().unwrap().remove("officeDataBarId");
        r["unsupported"] = json!(true);
    }
    old["extendedConditionalFormats"] = json!(true);
    crate::conditional_format::hydrate(&pkg, path, &mut old).unwrap();
    assert_eq!(old["conditionalFormats"].as_array().unwrap().len(), 5);
    assert_eq!(old["extendedConditionalFormats"], false);
    assert!(old["conditionalFormats"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["unsupported"] != true));
    let xml = pkg.text(path).unwrap().to_owned();
    let d = parse(&xml).unwrap();
    let base = d
        .descendants()
        .find(|n| {
            n.has_tag_name((S, "conditionalFormatting")) && n.attribute("sqref") == Some("B1:B4")
        })
        .unwrap();
    let rule = d
        .descendants()
        .find(|n| n.has_tag_name((X14, "cfRule")))
        .unwrap();
    let raw = crate::office::scoped_record(&xml, rule);
    let raw = raw
        .replacen("type=\"dataBar\"", "type=\"dataBar\" priority=\"2\"", 1)
        .replacen("<x14:dataBar", "<x14:dataBar showValue=\"0\"", 1)
        .replacen(
            "<x14:borderColor",
            "<x14:fillColor rgb=\"FF63BE7B\"/><x14:borderColor",
            1,
        );
    pkg.set(
        path,
        crate::office::replace_ranges(
            &xml,
            vec![(base.range(), String::new()), (rule.range(), raw)],
        ),
    );
    let s = crate::xlsx::import(&pkg).unwrap().0;
    assert_eq!(s[0]["conditionalFormats"].as_array().unwrap().len(), 6);
    assert_eq!(s[0]["extendedConditionalFormats"], false);
    assert_ne!(s[0]["conditionalFormats"][1]["unsupported"], true);
    let mut changed = s[0].clone();
    changed["conditionalFormats"][1]["dataBar"]["showValue"] = json!(true);
    crate::conditional_format::write(&mut pkg, &changed, Some(&s[0]), path).unwrap();
    let reopened = crate::xlsx::import(&pkg).unwrap().0;
    assert_eq!(
        reopened[0]["conditionalFormats"].as_array().unwrap().len(),
        6
    );
    assert_eq!(
        reopened[0]["conditionalFormats"][1]["dataBar"]["showValue"],
        true
    );
}
#[test]
fn data_bars_unmodeled_extensions_remain_read_only_and_source_preserved() {
    let base =
        Package::from_bytes(include_bytes!("../tests/fixtures/data-bars.xlsx").to_vec()).unwrap();
    for (name, from, to) in [
        (
            "theme",
            "<x14:negativeFillColor rgb=\"FFC00000\"/>",
            "<x14:negativeFillColor theme=\"1\"/>",
        ),
        (
            "metadata",
            "<x14:conditionalFormattings>",
            "<x14:conditionalFormattings xmlns:z=\"urn:future\" z:keep=\"yes\">",
        ),
        (
            "unknown",
            "<x14:cfRule type=\"dataBar\"",
            "<x14:cfRule type=\"futureBar\"",
        ),
    ] {
        let mut pkg = base.clone();
        let path = "xl/worksheets/sheet1.xml";
        let xml = pkg.text(path).unwrap().replacen(from, to, 1);
        assert!(xml.contains(to));
        pkg.set(path, xml.clone());
        let temp =
            std::env::temp_dir().join(format!("folio-conditional-tests/data-bars-{name}.xlsx"));
        std::fs::create_dir_all(temp.parent().unwrap()).unwrap();
        std::fs::write(&temp, pkg.bytes().unwrap()).unwrap();
        let mut c = blank();
        c.dispatch(json!({"op":"importOffice","path":temp}))
            .unwrap();
        assert_eq!(c.workspace["sheets"][0]["extendedConditionalFormats"], true);
        let before = c.snapshot();
        assert!(command(&mut c, json!({"action":"delete","index":0})).is_err());
        assert_eq!(c.snapshot(), before);
        assert!(c
            .dispatch(
                json!({"op":"structure","action":"insertRows","sheetId":id(&c),"at":1,"count":1})
            )
            .is_err());
        assert_eq!(c.snapshot(), before);
        cells(&mut c, json!({"J1":"unrelated edit"}));
        let exported = export(&mut c, &format!("data-bars-{name}-retained.xlsx"));
        let out = exported.text(path).unwrap();
        assert!(out.contains(to));
        let d1 = parse(&xml).unwrap();
        let d2 = parse(out).unwrap();
        let before = child(d1.root_element(), "extLst")
            .unwrap()
            .children()
            .find(|n| n.is_element())
            .unwrap();
        let after = child(d2.root_element(), "extLst")
            .unwrap()
            .children()
            .find(|n| n.attribute("uri") == before.attribute("uri"))
            .unwrap();
        assert_eq!(
            crate::office::raw(&xml, before),
            crate::office::raw(out, after)
        );
    }
}

#[test]
fn data_bars_x14_literal_formula_threshold_and_finite_geometry() {
    let mut rule = data_bar(true);
    rule["dataBar"]["thresholds"] =
        json!([{"type":"formula","value":"50"},{"type":"num","value":"100"}]);
    let bar = crate::conditional_data_bar::geometry(
        &rule["dataBar"],
        &[25.0, 100.0],
        25.0,
        false,
        |_| panic!("A bare x14 numerical constant must be zero"),
    )
    .unwrap()
    .unwrap();
    near(&bar["end"], 0.25);
    rule["dataBar"]["thresholds"] =
        json!([{"type":"num","value":"-1e308"},{"type":"num","value":"1e308"}]);
    assert!(crate::conditional_data_bar::geometry(
        &rule["dataBar"],
        &[25.0, 100.0],
        25.0,
        false,
        |_| unreachable!()
    )
    .is_err());
}

#[test]
fn data_bars_midpoint_uses_one_magnitude_scale_and_axis_requires_negative_values() {
    let mut r = data_bar(true);
    r["dataBar"]["axisPosition"] = json!("middle");
    let geometry = |b: &Value, n: f64, values: &[f64]| {
        crate::conditional_data_bar::geometry(b, values, n, false, |_| unreachable!())
            .unwrap()
            .unwrap()
    };
    let minus = geometry(&r["dataBar"], -20.0, &[-20.0, 40.0, 80.0]);
    near(&minus["axis"], 0.5);
    near(&minus["end"], 0.375);
    let plus = geometry(&r["dataBar"], 80.0, &[-20.0, 40.0, 80.0]);
    near(&plus["end"], 1.0);
    let smaller = geometry(&r["dataBar"], 40.0, &[-20.0, 40.0, 80.0]);
    near(&smaller["end"], 0.75);
    let positive = geometry(&r["dataBar"], 40.0, &[40.0, 80.0]);
    assert!(positive["axis"].is_null());
}
