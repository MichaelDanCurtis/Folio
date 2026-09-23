use crate::{
    conditional_icons as icons,
    office::{parse, Package},
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
fn change(c: &mut Core, mut r: Value) -> Result<Value, String> {
    r["op"] = json!("conditionalFormat");
    r["sheetId"] = json!(id(c));
    r["baseRevision"] = c.workspace["revision"].clone();
    c.dispatch(r)
}
fn add(c: &mut Core, r: Value) {
    change(c, json!({"action":"add","rule":r})).unwrap();
}
fn view(c: &mut Core, addresses: &[&str]) -> Value {
    c.dispatch(json!({"op":"conditionalRange","sheetId":id(c),"baseRevision":c.workspace["revision"],"cells":addresses})).unwrap()
}
fn rule(set: &str) -> Value {
    let count = icons::count(set).unwrap();
    json!({"type":"iconSet","style":{},"sqref":"A1:A1048576","iconSet":{"set":set,"thresholds":(0..count).map(|i|json!({"type":"percent","value":((i as f64*100.0/count as f64).round() as usize).to_string(),"gte":true})).collect::<Vec<_>>()}})
}
fn folder() -> std::path::PathBuf {
    let p = std::env::temp_dir().join("folio-icon-tests");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = folder().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn index(c: &mut Core, a: &str) -> Value {
    view(c, &[a])["styles"][a]["_iconSet"]["index"].clone()
}
#[test]
fn icon_sets_catalog_categories_and_standard_extended_roundtrip() {
    let mut c = blank();
    cells(
        &mut c,
        json!({"A1":"0","A2":"25","A3":"50","A4":"75","A5":"100"}),
    );
    for (ordinal, set) in icons::SETS.iter().enumerate() {
        let mut r = rule(set);
        r["sqref"] = json!(format!(
            "{}1:{}5",
            crate::formula::address(0, ordinal)[..crate::formula::address(0, ordinal).len() - 1]
                .to_owned(),
            crate::formula::address(0, ordinal)[..crate::formula::address(0, ordinal).len() - 1]
                .to_owned()
        ));
        add(&mut c, r);
    }
    let pkg = export(&mut c, "all-icon-sets.xlsx");
    let (s, _) = crate::xlsx::import(&pkg).unwrap();
    assert_eq!(s[0]["extendedConditionalFormats"], false);
    let rules = s[0]["conditionalFormats"].as_array().unwrap();
    assert_eq!(rules.len(), 20);
    for r in rules {
        assert_ne!(r["unsupported"], true, "{r}");
        icons::validate(&r["iconSet"]).unwrap();
    }
    let xml = pkg.text(s[0]["officePath"].as_str().unwrap()).unwrap();
    let doc = parse(xml).unwrap();
    assert_eq!(
        doc.descendants()
            .filter(|n| n.has_tag_name((crate::conditional_data_bar::S, "iconSet")))
            .count(),
        17
    );
    assert_eq!(
        doc.descendants()
            .filter(|n| n.has_tag_name((crate::conditional_data_bar::X14, "iconSet")))
            .count(),
        3
    );
    for set in icons::SETS {
        let r = rule(set);
        let v = &r["iconSet"];
        let n = icons::count(set).unwrap();
        for i in 0..n {
            let number = 100.0 * (i as f64 + 0.4) / n as f64;
            let icon = icons::evaluate(v, &[0.0, 100.0], number, |_| unreachable!())
                .unwrap()
                .unwrap();
            assert_eq!(icon["index"], i, "{set}");
        }
    }
}
#[test]
fn icon_thresholds_gte_percent_percentile_formula_population_and_errors() {
    let mut c = blank();
    cells(
        &mut c,
        json!({"A1":"-100","A2":"0","A3":"5","A4":"10","A5":"100","A6":"text","A7":"=1/0","A8":"=\"10\"","A9":"=TRUE()","A1048576":"1000","B1":"5","B2":"10"}),
    );
    let mut r = rule("3Arrows");
    r["sqref"] = json!("A1:A9 A1048576");
    add(&mut c, r.clone());
    assert_eq!(index(&mut c, "A5"), 0); // Offscreen maximum counts, not just the viewport.
    let before = c.snapshot();
    let v = view(&mut c, &["A6", "A7", "A8", "A9", "A10"]);
    assert_eq!(v["styles"], json!({}));
    assert_eq!(c.snapshot(), before);
    r["iconSet"]["thresholds"] = json!([{"type":"num","value":"999"},{"type":"num","value":"5","gte":false},{"type":"num","value":"10"}]);
    change(&mut c, json!({"action":"edit","index":0,"rule":r})).unwrap();
    assert_eq!(index(&mut c, "A3"), 0);
    assert_eq!(index(&mut c, "A4"), 2);
    r["iconSet"]["thresholds"][1] = json!({"type":"formula","value":"$B$1","gte":true});
    r["iconSet"]["thresholds"][2] = json!({"type":"formula","value":"$B$2"});
    change(&mut c, json!({"action":"edit","index":0,"rule":r})).unwrap();
    assert_eq!(index(&mut c, "A3"), 1);
    cells(&mut c, json!({"B1":"6"}));
    assert_eq!(index(&mut c, "A3"), 0);
    cells(&mut c, json!({"B1":"=1/0"}));
    let v = view(&mut c, &["A3"]);
    assert!(v["styles"]["A3"].is_null());
    assert_eq!(v["warnings"].as_array().unwrap().len(), 1);
    r["sqref"] = json!("A1:A5");
    r["iconSet"]["thresholds"] = json!([{"type":"num","value":"0"},{"type":"percentile","value":"25"},{"type":"percentile","value":"75"}]);
    change(&mut c, json!({"action":"edit","index":0,"rule":r})).unwrap();
    assert_eq!(index(&mut c, "A2"), 1);
    assert_eq!(index(&mut c, "A4"), 2);
    let r = rule("3Stars");
    let v = icons::evaluate(&r["iconSet"], &[-1e308, 1e308], 0.0, |_| unreachable!())
        .unwrap()
        .unwrap();
    assert_eq!(v["index"], 1);
    let v = icons::evaluate(&r["iconSet"], &[7.0, 7.0], 7.0, |_| unreachable!())
        .unwrap()
        .unwrap();
    assert_eq!(v["index"], 2);
}
#[test]
fn icon_custom_no_icon_reverse_priority_and_one_history_entry() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"0","A2":"50","A3":"100"}));
    add(&mut c, rule("3Flags"));
    let mut r = rule("3Arrows");
    r["iconSet"]["reverse"] = json!(true);
    r["iconSet"]["showValue"] = json!(false);
    add(&mut c, r.clone());
    assert_eq!(index(&mut c, "A1"), 2);
    assert_eq!(index(&mut c, "A3"), 0);
    let before = c.snapshot();
    r["iconSet"]["icons"] = json!([{"set":"NoIcons","index":0},{"set":"5Quarters","index":2},{"set":"3Stars","index":2}]);
    change(&mut c, json!({"action":"edit","index":0,"rule":r})).unwrap();
    let v = view(&mut c, &["A1", "A2", "A3"]);
    assert_eq!(v["styles"]["A1"]["_iconSet"]["set"], "NoIcons");
    assert_eq!(v["styles"]["A2"]["_iconSet"]["set"], "5Quarters");
    assert_eq!(v["styles"]["A3"]["_iconSet"]["showValue"], false);
    assert_eq!(
        c.workspace["sheets"][0]["cells"],
        before["workspace"]["sheets"][0]["cells"]
    );
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"],
        before["workspace"]["sheets"][0]["cellStyles"]
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.snapshot()["formatted"], before["formatted"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(view(&mut c, &["A1", "A2", "A3"]), v);
    add(
        &mut c,
        json!({"type":"expression","sqref":"A1:A3","formulas":["A1=0"],"style":{"fill":"#ABCDEF"},"stopIfTrue":true}),
    );
    assert!(view(&mut c, &["A1"])["styles"]["A1"]["_iconSet"].is_null());
    assert_eq!(index(&mut c, "A2"), 2);
    export(&mut c, "custom-icons.xlsx");
}
#[test]
fn icon_structural_references_recovery_edit_delete_and_legacy_upgrade() {
    let mut c = blank();
    cells(
        &mut c,
        json!({"A1":"0","A2":"50","A3":"100","B1":"25","B2":"75"}),
    );
    for set in ["3Arrows", "3Stars"] {
        let mut r = rule(set);
        r["sqref"] = json!("A1:A3 C1:C3");
        r["iconSet"]["thresholds"][1] = json!({"type":"formula","value":"$B$1"});
        r["iconSet"]["thresholds"][2] = json!({"type":"formula","value":"$B$2"});
        add(&mut c, r);
    }
    let pkg = export(&mut c, "icon-formulas.xlsx");
    let (s, _) = crate::xlsx::import(&pkg).unwrap();
    let path = s[0]["officePath"].as_str().unwrap();
    let mut old = s[0].clone();
    old["conditionalFormats"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r.get("officeIconSetId").is_none());
    old["conditionalFormats"][0]
        .as_object_mut()
        .unwrap()
        .remove("iconSet");
    old["conditionalFormats"][0]["unsupported"] = json!(true);
    old["extendedConditionalFormats"] = json!(true);
    crate::conditional_format::hydrate(&pkg, path, &mut old).unwrap();
    assert_eq!(old["conditionalFormats"], s[0]["conditionalFormats"]);
    assert_eq!(old["extendedConditionalFormats"], false);
    old["conditionalFormats"] = json!([]);
    crate::conditional_format::hydrate(&pkg, path, &mut old).unwrap();
    assert_eq!(old["conditionalFormats"], json!([]));
    c.dispatch(json!({"op":"structure","action":"insertRows","sheetId":id(&c),"at":1,"count":1}))
        .unwrap();
    for r in c.workspace["sheets"][0]["conditionalFormats"]
        .as_array()
        .unwrap()
    {
        assert_eq!(r["sqref"], "A2:A4 C2:C4");
        assert_eq!(r["iconSet"]["thresholds"][1]["value"], "$B$2");
        assert_eq!(r["iconSet"]["thresholds"][2]["value"], "$B$3");
    }
    export(&mut c, "icon-formulas-shifted.xlsx");
    let temp = folder().join("icon-workspace.folio");
    c.dispatch(json!({"op":"save","path":temp})).unwrap();
    let before = c.snapshot();
    let mut other = blank();
    other.dispatch(json!({"op":"load","path":temp})).unwrap();
    assert_eq!(other.snapshot()["formatted"], before["formatted"]);
    let mut r = c.workspace["sheets"][0]["conditionalFormats"][0].clone();
    r["iconSet"]["showValue"] = json!(false);
    change(&mut c, json!({"action":"edit","index":0,"rule":r})).unwrap();
    export(&mut c, "icon-formulas-edited.xlsx");
    change(&mut c, json!({"action":"delete","index":0})).unwrap();
    change(&mut c, json!({"action":"delete","index":0})).unwrap();
    let pkg = export(&mut c, "icon-deleted.xlsx");
    assert!(!pkg.text(path).unwrap().contains("iconSet"));
}
#[test]
fn icon_invalid_authoring_and_unmodeled_xml_preserve_state() {
    let mut c = blank();
    let before = c.snapshot();
    for mutation in [
        json!({"set":"Bogus"}),
        json!({"reverse":"true"}),
        json!({"thresholds":[]}),
        json!({"icons":[{"set":"3Stars","index":9}]}),
    ] {
        let mut r = rule("3Arrows");
        for (k, v) in mutation.as_object().unwrap() {
            r["iconSet"][k] = v.clone();
        }
        assert!(change(&mut c, json!({"action":"add","rule":r})).is_err());
        assert_eq!(c.snapshot(), before);
    }
    for f in [
        "A1",
        "$A1",
        "SUM($A$1:$A$3)",
        "{1,2}",
        "SUM(",
        "Table1[Amount]",
    ] {
        let mut r = rule("3Stars");
        r["iconSet"]["thresholds"][1] = json!({"type":"formula","value":f});
        assert!(
            change(&mut c, json!({"action":"add","rule":r})).is_err(),
            "{f}"
        );
        assert_eq!(c.snapshot(), before);
    }
    add(&mut c, rule("3Stars"));
    let mut pkg = export(&mut c, "icon-extension-source.xlsx");
    let sheets = crate::xlsx::import(&pkg).unwrap().0;
    let path = sheets[0]["officePath"].as_str().unwrap();
    let xml = pkg
        .text(path)
        .unwrap()
        .replacen("custom=\"0\"", "custom=\"0\" future=\"keep\"", 1);
    assert!(xml.contains("future="));
    pkg.set(path, xml.clone());
    let temp = folder().join("icon-unknown.xlsx");
    std::fs::write(&temp, pkg.bytes().unwrap()).unwrap();
    let mut imported = blank();
    imported
        .dispatch(json!({"op":"importOffice","path":temp}))
        .unwrap();
    assert_eq!(
        imported.workspace["sheets"][0]["extendedConditionalFormats"],
        true
    );
    let before = imported.snapshot();
    assert!(change(&mut imported, json!({"action":"add","rule":rule("3Flags")})).is_err());
    assert_eq!(imported.snapshot(), before);
    cells(&mut imported, json!({"Z1":"unrelated"}));
    let out = export(&mut imported, "icon-unknown-retained.xlsx");
    let d = parse(&xml).unwrap();
    let outxml = out.text(path).unwrap();
    let od = parse(outxml).unwrap();
    let ns = crate::conditional_data_bar::X14;
    let a = d
        .descendants()
        .find(|n| n.has_tag_name((ns, "conditionalFormattings")))
        .unwrap();
    let b = od
        .descendants()
        .find(|n| n.has_tag_name((ns, "conditionalFormattings")))
        .unwrap();
    assert_eq!(crate::office::raw(&xml, a), crate::office::raw(outxml, b));
}

#[test]
fn icon_openpyxl_import_reverse_strict_boundary_and_edit_roundtrip() {
    let pkg =
        Package::from_bytes(include_bytes!("../tests/fixtures/icon-sets-openpyxl.xlsx").to_vec())
            .unwrap();
    let (s, _) = crate::xlsx::import(&pkg).unwrap();
    let rules = s[0]["conditionalFormats"].as_array().unwrap();
    assert_eq!(rules.len(), 17);
    assert_eq!(s[0]["extendedConditionalFormats"], false);
    for (i, r) in rules.iter().enumerate() {
        assert_ne!(r["unsupported"], true, "{r}");
        assert_eq!(r["iconSet"]["set"], icons::SETS[i]);
        assert_eq!(r["iconSet"]["reverse"], true);
        assert_eq!(r["iconSet"]["showValue"], false);
        assert_eq!(r["iconSet"]["thresholds"][1]["gte"], false);
    }
    let mut calc = crate::calculation::WorkbookCalculator::default();
    calc.sync(&s);
    let v = crate::conditional_format::evaluate(
        &s,
        0,
        &["A1".into(), "A2".into(), "A3".into()],
        &mut calc,
    )
    .unwrap();
    assert_eq!(v["styles"]["A1"]["_iconSet"]["index"], 2);
    assert_eq!(v["styles"]["A2"]["_iconSet"]["index"], 1);
    assert_eq!(v["styles"]["A3"]["_iconSet"]["index"], 0);
    let mut after = s[0].clone();
    after["conditionalFormats"][0]["iconSet"]["showValue"] = json!(true);
    let mut out = pkg.clone();
    crate::conditional_format::write(
        &mut out,
        &after,
        Some(&s[0]),
        s[0]["officePath"].as_str().unwrap(),
    )
    .unwrap();
    let reopened = crate::xlsx::import(&out).unwrap().0;
    assert_eq!(
        reopened[0]["conditionalFormats"][0]["iconSet"]["showValue"],
        true
    );
    for i in 1..17 {
        for key in ["type", "sqref", "priority", "iconSet", "style"] {
            assert_eq!(
                reopened[0]["conditionalFormats"][i][key],
                s[0]["conditionalFormats"][i][key]
            );
        }
    }
    std::fs::write(
        folder().join("icon-openpyxl-edited.xlsx"),
        out.bytes().unwrap(),
    )
    .unwrap();
}

#[test]
fn icon_custom_category_count_ignores_unused_base_set_and_keeps_extension_metadata_safe() {
    let ns = crate::conditional_data_bar::X14;
    let xm = crate::conditional_data_bar::XM;
    for base in ["3TrafficLights1", "NoIcons"] {
        let xml = format!(
            r#"<x14:iconSet xmlns:x14="{ns}" xmlns:xm="{xm}" iconSet="{base}" custom="1" reverse="1"><x14:cfvo type="percent"><xm:f>0</xm:f></x14:cfvo><x14:cfvo type="num"><xm:f>20</xm:f></x14:cfvo><x14:cfvo type="num"><xm:f>40</xm:f></x14:cfvo><x14:cfvo type="num"><xm:f>60</xm:f></x14:cfvo><x14:cfIcon iconSet="NoIcons" iconId="0"/><x14:cfIcon iconSet="4Rating" iconId="1"/><x14:cfIcon iconSet="3Triangles" iconId="2"/><x14:cfIcon iconSet="5Boxes" iconId="4"/></x14:iconSet>"#
        );
        let doc = parse(&xml).unwrap();
        let v = icons::read(doc.root_element()).unwrap();
        let out = icons::evaluate(&v, &[0.0, 100.0], 80.0, |_| unreachable!())
            .unwrap()
            .unwrap();
        assert_eq!(out["count"], 4);
        assert_eq!(out["set"], "5Boxes");
        assert_eq!(out["index"], 4);
        let mut c = blank();
        let mut r = rule("3TrafficLights1");
        r["iconSet"] = v;
        add(&mut c, r);
        let p = export(&mut c, &format!("icon-custom-{base}.xlsx"));
        let sheets = crate::xlsx::import(&p).unwrap().0;
        assert_ne!(sheets[0]["conditionalFormats"][0]["unsupported"], true);
        let xml = xml.replace("<xm:f>20", "<xm:f future=\"keep\">20");
        assert!(icons::read(parse(&xml).unwrap().root_element()).is_err());
    }
    let mut c = blank();
    let mut r = rule("3Arrows");
    r["priority"] = json!(1);
    r["officeXml"] = json!(
        r#"<cfRule xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" type="iconSet" priority="1" xmlns:v="urn:vendor" v:owner="retained"><iconSet iconSet="3Arrows"><cfvo type="percent" val="0"/><cfvo type="percent" val="33"/><cfvo type="percent" val="67"/></iconSet></cfRule>"#
    );
    c.workspace["sheets"][0]["conditionalFormats"] = json!([r]);
    let before = c.snapshot();
    r["iconSet"]["set"] = json!("3Stars");
    assert!(change(&mut c, json!({"action":"edit","index":0,"rule":r}))
        .unwrap_err()
        .contains("metadata"));
    assert_eq!(c.snapshot(), before);
}
