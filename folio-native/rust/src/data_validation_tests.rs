use crate::{
    data_validation,
    office::{attr, child, parse, Package},
    workspace::Core,
};
use serde_json::{json, Value};
fn blank() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c
}
fn sid(c: &Core) -> String {
    c.workspace["sheets"][0]["id"].as_str().unwrap().into()
}
fn rule(c: &mut Core, range: &str, mut r: Value) {
    for (k, v) in [
        ("allowBlank", json!(true)),
        ("showErrorMessage", json!(true)),
        ("errorStyle", json!("stop")),
    ] {
        if r.get(k).is_none() {
            r[k] = v;
        }
    }
    let req = json!({"op":"dataValidation","sheetId":sid(c),"baseRevision":c.workspace["revision"],"range":range,"action":"set","rule":r});
    c.dispatch(req).unwrap();
}
fn check(c: &mut Core, a: &str, v: &str) -> Value {
    c.dispatch(json!({"op":"validationEntryCheck","sheetId":sid(c),"address":a,"value":v,"baseRevision":c.workspace["revision"]})).unwrap()
}
fn enter(c: &mut Core, a: &str, v: &str, allow: bool) -> Result<Value, String> {
    c.dispatch(json!({"op":"enterCell","sheetId":sid(c),"address":a,"value":v,"allowInvalid":allow,"baseRevision":c.workspace["revision"]}))
}
fn cells(c: &mut Core, values: Value) {
    let id = sid(c);
    let items: Vec<_> = values
        .as_object()
        .unwrap()
        .iter()
        .map(|(a, v)| json!({"sheetId":id,"address":a,"value":v}))
        .collect();
    c.dispatch(json!({"op":"cells","cells":items})).unwrap();
}
fn export(c: &mut Core, name: &str) -> Package {
    let dir = std::env::temp_dir().join("folio-validation-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}

#[test]
fn validation_types_operators_and_formula_context() {
    let mut c = blank();
    rule(
        &mut c,
        "A1:A10",
        json!({"type":"whole","formula1":"1","formula2":"10"}),
    );
    for (v, valid) in [
        ("1", true),
        ("10", true),
        ("11", false),
        ("1.5", false),
        ("text", false),
        ("", true),
        ("'1", false),
    ] {
        assert_eq!(check(&mut c, "A1", v)["valid"], valid, "{v}");
    }
    let before = c.snapshot();
    assert!(enter(&mut c, "A1", "11", true).is_err());
    assert_eq!(before, c.snapshot());
    enter(&mut c, "A1", "8", false).unwrap();
    assert_eq!(c.snapshot()["calculated"][sid(&c)]["A1"], 8.0);
    for (op, pass, fail) in [
        ("equal", "5", "4"),
        ("notEqual", "4", "5"),
        ("lessThan", "4", "5"),
        ("lessThanOrEqual", "5", "6"),
        ("greaterThan", "6", "5"),
        ("greaterThanOrEqual", "5", "4"),
        ("notBetween", "11", "5"),
    ] {
        rule(
            &mut c,
            "B1",
            json!({"type":"decimal","operator":op,"formula1":"5","formula2":"10"}),
        );
        assert_eq!(check(&mut c, "B1", pass)["valid"], true, "{op}");
        assert_eq!(check(&mut c, "B1", fail)["valid"], false, "{op}");
    }
    rule(
        &mut c,
        "B1",
        json!({"type":"decimal","formula1":"0.1","formula2":"0.8"}),
    );
    assert_eq!(check(&mut c, "B1", "0.5")["valid"], true);
    rule(
        &mut c,
        "C1",
        json!({"type":"date","formula1":"DATE(2026,9,1)","formula2":"DATE(2026,9,30)"}),
    );
    assert_eq!(check(&mut c, "C1", "2026-09-19")["valid"], true);
    assert_eq!(check(&mut c, "C1", "2026-10-01")["valid"], false);
    rule(
        &mut c,
        "D1",
        json!({"type":"time","formula1":"TIME(8,0,0)","formula2":"TIME(17,0,0)"}),
    );
    assert_eq!(check(&mut c, "D1", "12:30")["valid"], true);
    assert_eq!(check(&mut c, "D1", "18:00")["valid"], false);
    rule(
        &mut c,
        "E1",
        json!({"type":"textLength","operator":"lessThanOrEqual","formula1":"4"}),
    );
    assert_eq!(check(&mut c, "E1", "four")["valid"], true);
    assert_eq!(check(&mut c, "E1", "hello")["valid"], false);
    rule(
        &mut c,
        "F2:F8",
        json!({"type":"custom","formula1":"AND(F2>0,F2=ROW())"}),
    );
    assert_eq!(check(&mut c, "F5", "5")["valid"], true);
    assert_eq!(check(&mut c, "F5", "2")["valid"], false);
    cells(&mut c, json!({"G1":"3"}));
    rule(
        &mut c,
        "H1:H5",
        json!({"type":"whole","operator":"lessThanOrEqual","formula1":"$G$1"}),
    );
    assert_eq!(check(&mut c, "H4", "4")["valid"], false);
    cells(&mut c, json!({"G1":"6"}));
    assert_eq!(check(&mut c, "H4", "4")["valid"], true);
}

#[test]
fn validation_override_history_and_stale_revision_are_atomic() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"existing"}));
    for style in ["stop", "warning", "information"] {
        rule(
            &mut c,
            "A1",
            json!({"type":"list","formula1":"\"Red,Green,Blue\"","errorStyle":style,"error":"Choose a colour"}),
        );
        assert_eq!(c.workspace["sheets"][0]["cells"]["A1"], "existing");
        let before = c.snapshot();
        let checked = check(&mut c, "A1", "purple");
        assert_eq!(before, c.snapshot());
        assert_eq!(checked["valid"], false);
        assert_eq!(checked["canOverride"], style != "stop");
        assert!(enter(&mut c, "A1", "purple", false).is_err());
        assert_eq!(before, c.snapshot());
        if style != "stop" {
            enter(&mut c, "A1", "purple", true).unwrap();
            c.dispatch(json!({"op":"undo"})).unwrap();
            assert_eq!(before["workspace"]["sheets"], c.workspace["sheets"]);
        }
    }
    let stale = c.workspace["revision"].clone();
    enter(&mut c, "A1", "green", false).unwrap();
    assert!(c.dispatch(json!({"op":"enterCell","sheetId":sid(&c),"address":"A1","value":"red","baseRevision":stale})).is_err());
    rule(
        &mut c,
        "A1",
        json!({"type":"whole","formula1":"0","formula2":"2","showErrorMessage":false}),
    );
    enter(&mut c, "A1", "anything", false).unwrap();
}

#[test]
fn validation_lists_reanchor_and_partial_clear_without_cell_expansion() {
    let mut c = blank();
    cells(&mut c, json!({"D1":"Red","D2":"Green","D3":"Blue"}));
    rule(
        &mut c,
        "A1:A4",
        json!({"type":"list","formula1":"$D$1:$D$3"}),
    );
    let choices = c
        .dispatch(json!({"op":"validationChoices","sheetId":sid(&c),"address":"A3"}))
        .unwrap();
    assert_eq!(choices["values"][1]["label"], "Green");
    assert_eq!(check(&mut c, "A3", "green")["valid"], true);
    rule(
        &mut c,
        "B1:B5",
        json!({"type":"custom","formula1":"B1=ROW()"}),
    );
    let settings = c
        .dispatch(
            json!({"op":"validationChoices","sheetId":sid(&c),"address":"B3","settingsOnly":true}),
        )
        .unwrap();
    assert_eq!(settings["rule"]["formula1"], "B3=ROW()");
    c.dispatch(json!({"op":"dataValidation","sheetId":sid(&c),"baseRevision":c.workspace["revision"],"range":"B1:B2","action":"clear"})).unwrap();
    let r = data_validation::at(&c.workspace["sheets"][0], "B3").unwrap();
    assert_eq!(r["formula1"], "B3=ROW()");
    assert!(data_validation::at(&c.workspace["sheets"][0], "B2").is_none());
    assert_eq!(check(&mut c, "B5", "5")["valid"], true);
    rule(
        &mut c,
        "J1:J1048576",
        json!({"type":"whole","formula1":"0","formula2":"100"}),
    );
    c.dispatch(json!({"op":"dataValidation","sheetId":sid(&c),"baseRevision":c.workspace["revision"],"range":"J3:J4","action":"clear"})).unwrap();
    let r = data_validation::at(&c.workspace["sheets"][0], "J1048576").unwrap();
    assert_eq!(r["sqref"], "J1:J2 J5:J1048576");
    assert!(data_validation::at(&c.workspace["sheets"][0], "J3").is_none());
}

#[test]
fn validation_export_reopen_structure_and_source_metadata() {
    let mut c = blank();
    rule(
        &mut c,
        "A2:A8",
        json!({"type":"custom","formula1":"A2=ROW()","showInputMessage":true,"promptTitle":"Row number","prompt":"Use this row's number"}),
    );
    rule(
        &mut c,
        "B2:B8",
        json!({"type":"list","formula1":"\"Red,Green,Blue\"","showDropDown":false}),
    );
    let mut pkg = export(&mut c, "authored.xlsx");
    let projection = crate::xlsx::import(&pkg).unwrap().0;
    let path_string = projection[0]["officePath"].as_str().unwrap().to_owned();
    let path = path_string.as_str();
    let xml = pkg.text(path).unwrap().replace(
        "<dataValidation xmlns=",
        "<dataValidation imeMode=\"disabled\" xmlns=",
    );
    pkg.set(path, xml);
    let imported = std::env::temp_dir().join("folio-validation-tests/imported.xlsx");
    std::fs::write(&imported, pkg.bytes().unwrap()).unwrap();
    c.dispatch(json!({"op":"importOffice","path":imported}))
        .unwrap();
    rule(
        &mut c,
        "A3:A4",
        json!({"type":"whole","formula1":"2","formula2":"10"}),
    );
    // New package baselines must not attach preserved attributes to another rule.
    c.dispatch(json!({"op":"fillCells","sheetId":sid(&c),"range":"D2:D3","direction":"down"}))
        .unwrap();
    c.dispatch(json!({"op":"structure","sheetId":sid(&c),"action":"insertRows","at":2,"count":1}))
        .unwrap();
    assert_eq!(check(&mut c, "A6", "6")["valid"], true);
    let p = export(&mut c, "edited.xlsx");
    let d = parse(p.text(path).unwrap()).unwrap();
    let parent = child(d.root_element(), "dataValidations").unwrap();
    assert_eq!(attr(parent, "count"), Some("3"));
    assert!(parent
        .children()
        .filter(|n| n.is_element())
        .all(|n| attr(n, "imeMode") == Some("disabled")));
    let dir = std::env::temp_dir().join("folio-validation-tests");
    c.dispatch(json!({"op":"save","path":dir.join("validation.folio")}))
        .unwrap();
    let before = c.workspace["sheets"].clone();
    c.dispatch(json!({"op":"load","path":dir.join("validation.folio")}))
        .unwrap();
    assert_eq!(c.workspace["sheets"], before);
    let reopened = export(&mut c, "reopened.xlsx");
    assert_eq!(p.bytes().unwrap(), reopened.bytes().unwrap());
    c.dispatch(json!({"op":"importOffice","path":dir.join("edited.xlsx")}))
        .unwrap();
    assert_eq!(check(&mut c, "A6", "6")["valid"], true);
    // The same transformation also works for workbooks with no source package.
    let mut n = blank();
    rule(
        &mut n,
        "A2:A8",
        json!({"type":"custom","formula1":"A2=ROW()"}),
    );
    n.dispatch(json!({"op":"structure","sheetId":sid(&n),"action":"deleteRows","at":2,"count":1}))
        .unwrap();
    assert_eq!(check(&mut n, "A3", "3")["valid"], true);
}

#[test]
fn validation_error_sources_and_extended_rules_are_explicit() {
    let mut c = blank();
    rule(
        &mut c,
        "A1",
        json!({"type":"list","formula1":"MissingName"}),
    );
    let v = check(&mut c, "A1", "hello");
    assert_eq!(v["valid"], false);
    assert_eq!(v["canOverride"], false);
    assert!(v["reason"]
        .as_str()
        .unwrap()
        .contains("could not be checked"));
    let before = c.snapshot();
    assert!(enter(&mut c, "A1", "hello", true).is_err());
    assert_eq!(before, c.snapshot());
}

#[test]
fn validation_copy_fill_paste_modes_and_cross_sheet_keep_rules() {
    let mut c = blank();
    cells(&mut c, json!({"A1":"1","B1":"Keep"}));
    rule(
        &mut c,
        "A1",
        json!({"type":"custom","formula1":"A1=ROW()","prompt":"row number","showInputMessage":true}),
    );
    rule(
        &mut c,
        "B1:B5",
        json!({"type":"list","formula1":"\"Keep,Other\""}),
    );
    let copied = c
        .dispatch(json!({"op":"copyCells","sheetId":sid(&c),"range":"A1"}))
        .unwrap();
    c.dispatch(json!({"op":"pasteCells","sheetId":sid(&c),"range":"C3:C5","token":copied["token"],"mode":"all"})).unwrap();
    assert_eq!(check(&mut c, "C5", "5")["valid"], true);
    assert_eq!(check(&mut c, "C5", "1")["valid"], false);
    assert_eq!(
        data_validation::at(&c.workspace["sheets"][0], "C5").unwrap()["formula1"],
        "C3=ROW()"
    );
    c.dispatch(json!({"op":"fillCells","sheetId":sid(&c),"range":"A1:A5","direction":"down"}))
        .unwrap();
    assert_eq!(check(&mut c, "A5", "5")["valid"], true);
    // Paste values bypasses entry checks but preserves the destination rule.
    c.dispatch(json!({"op":"pasteCells","sheetId":sid(&c),"range":"B2","token":copied["token"],"mode":"values"})).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["B2"], "1");
    assert_eq!(
        data_validation::at(&c.workspace["sheets"][0], "B2").unwrap()["type"],
        "list"
    );
    c.dispatch(json!({"op":"addSheet"})).unwrap();
    let second = c.workspace["sheets"][1]["id"].clone();
    c.dispatch(json!({"op":"pasteCells","sheetId":second,"range":"D4","token":copied["token"],"mode":"all"})).unwrap();
    assert_eq!(
        data_validation::at(&c.workspace["sheets"][1], "D4").unwrap()["formula1"],
        "D4=ROW()"
    );
    let empty = c
        .dispatch(json!({"op":"copyCells","sheetId":sid(&c),"range":"K1"}))
        .unwrap();
    c.dispatch(json!({"op":"pasteCells","sheetId":sid(&c),"range":"B1","token":empty["token"],"mode":"all"})).unwrap();
    assert!(data_validation::at(&c.workspace["sheets"][0], "B1").is_none());
    assert!(data_validation::at(&c.workspace["sheets"][0], "B2").is_some());
    export(&mut c, "copied.xlsx");
}

#[test]
fn validation_clock_and_1904_calendar_and_protection_guards() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"importOffice","path":concat!(env!("CARGO_MANIFEST_DIR"),"/../test/fixtures/date-system-1904.xlsx")})).unwrap();
    rule(
        &mut c,
        "Z1",
        json!({"type":"date","formula1":"DATE(1904,1,1)","formula2":"DATE(1904,1,2)"}),
    );
    assert_eq!(check(&mut c, "Z1", "1904-01-01")["valid"], true);
    assert_eq!(check(&mut c, "Z1", "1904-01-03")["valid"], false);
    rule(
        &mut c,
        "Z2",
        json!({"type":"time","formula1":"0","formula2":"0.5"}),
    );
    for (v, yes) in [
        ("12:00 AM", true),
        ("12:00 PM", true),
        ("1:00 pm", false),
        ("00:00:00.5", true),
        ("25:00", false),
        ("'10:00", false),
    ] {
        assert_eq!(check(&mut c, "Z2", v)["valid"], yes, "{v}");
    }
    let mut p = export(&mut c, "calendar.xlsx");
    let sheets = crate::xlsx::import(&p).unwrap().0;
    let path = sheets[0]["officePath"].as_str().unwrap().to_owned();
    let xml=p.text(&path).unwrap().replace("</worksheet>","<extLst><ext uri=\"test\"><x14:dataValidations xmlns:x14=\"http://schemas.microsoft.com/office/spreadsheetml/2009/9/main\" xmlns:xm=\"http://schemas.microsoft.com/office/excel/2006/main\" count=\"1\"><x14:dataValidation type=\"list\"><xm:sqref>Y1:Y2</xm:sqref></x14:dataValidation></x14:dataValidations></ext></extLst></worksheet>");
    p.set(&path, xml);
    let dir = std::env::temp_dir().join("folio-validation-tests");
    std::fs::write(dir.join("extended.xlsx"), p.bytes().unwrap()).unwrap();
    c.dispatch(json!({"op":"importOffice","path":dir.join("extended.xlsx")}))
        .unwrap();
    assert_eq!(check(&mut c, "Y1", "anything")["valid"], false);
    assert_eq!(check(&mut c, "X1", "anything")["valid"], true);
    let before = c.snapshot();
    assert!(c.dispatch(json!({"op":"dataValidation","sheetId":sid(&c),"range":"X1","action":"clear","baseRevision":c.workspace["revision"]})).is_err());
    assert_eq!(before, c.snapshot());
    // A standard protected worksheet also rejects rule edits atomically.
    let mut n = blank();
    let mut p = export(&mut n, "protection-base.xlsx");
    let projection = crate::xlsx::import(&p).unwrap().0;
    let path = projection[0]["officePath"].as_str().unwrap().to_owned();
    let xml = p
        .text(&path)
        .unwrap()
        .replace("</sheetData>", "</sheetData><sheetProtection sheet=\"1\"/>");
    p.set(&path, xml);
    std::fs::write(dir.join("protected.xlsx"), p.bytes().unwrap()).unwrap();
    n.dispatch(json!({"op":"importOffice","path":dir.join("protected.xlsx")}))
        .unwrap();
    assert!(n.dispatch(json!({"op":"dataValidation","sheetId":sid(&n),"range":"A1","action":"clear","baseRevision":n.workspace["revision"]})).unwrap_err().contains("protection"));
}

#[test]
fn validation_xml_attributes_with_literal_greater_than_keep_tag_boundaries() {
    let mut c = blank();
    rule(
        &mut c,
        "A1",
        json!({"type":"whole","formula1":"1","formula2":"5"}),
    );
    let mut p = export(&mut c, "attribute-base.xlsx");
    let sheets = crate::xlsx::import(&p).unwrap().0;
    let path = sheets[0]["officePath"].as_str().unwrap().to_owned();
    let xml = p
        .text(&path)
        .unwrap()
        .replace(
            "<dataValidations ",
            "<dataValidations custom=\"one > zero\" ",
        )
        .replace(
            "<dataValidation xmlns=",
            "<dataValidation prompt=\"value > zero\" custom=\"a > b\" xmlns=",
        );
    p.set(&path, xml);
    let mut s = crate::xlsx::import(&p).unwrap().0[0].clone();
    s["dataValidations"][0]["formula2"] = json!("10");
    data_validation::write(&mut p, &s, None, &path).unwrap();
    let d = parse(p.text(&path).unwrap()).unwrap();
    let parent = child(d.root_element(), "dataValidations").unwrap();
    let rule = child(parent, "dataValidation").unwrap();
    assert_eq!(attr(parent, "custom"), Some("one > zero"));
    assert_eq!(attr(rule, "custom"), Some("a > b"));
    assert_eq!(attr(rule, "prompt"), Some("value > zero"));
    assert_eq!(child(rule, "formula2").unwrap().text(), Some("10"));
}
