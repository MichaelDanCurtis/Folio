use crate::{office::Package, scenarios, workspace::Core};
use serde_json::{json, Value};
fn new() -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = c.workspace.clone();
    w["sheets"] = json!([{"id":"inputs","name":"Inputs","cells":{"A1":"10","B1":"20","C1":"=A1*B1"}},{"id":"result","name":"Result","cells":{"A1":"=Inputs!C1"}}]);
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    c
}
fn call(c: &mut Core, mut r: Value) -> Result<Value, String> {
    r["workspaceId"] = c.workspace["id"].clone();
    r["baseRevision"] = c.workspace["revision"].clone();
    if r.get("sheetId").is_none() {
        r["sheetId"] = json!("inputs");
    }
    c.dispatch(r)
}
fn add(c: &mut Core, name: &str, a: &str, b: &str) {
    call(c,json!({"op":"scenarios","action":"add","name":name,"comment":"sample","inputs":[{"address":"A1","value":a},{"address":"B1","value":b}]})).unwrap();
}
fn export(c: &mut Core, name: &str) -> Package {
    let dir = std::env::temp_dir().join("folio-scenarios-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
fn import(p: &Package) -> Core {
    let path = std::env::temp_dir().join(format!(
        "folio-scenarios-{}.xlsx",
        crate::office::fingerprint(&p.bytes().unwrap())
    ));
    std::fs::write(&path, p.bytes().unwrap()).unwrap();
    let mut c = new();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    c
}
#[test]
fn scenarios_named_inputs_show_history_edit_merge_delete_and_recovery() {
    let mut c = new();
    let original = c.snapshot();
    add(&mut c, "Base", "10", "20");
    add(&mut c, "Growth", "15", "30");
    assert_eq!(c.snapshot()["calculated"], original["calculated"]);
    assert_eq!(c.snapshot()["evaluatedCells"], original["evaluatedCells"]);
    call(&mut c, json!({"op":"scenarioShow","index":1})).unwrap();
    assert_eq!(c.snapshot()["calculated"]["result"]["A1"], 450.0);
    let shown = c.workspace["sheets"].clone();
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.snapshot()["calculated"]["result"]["A1"], 200.0);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(c.workspace["sheets"], shown);
    call(&mut c,json!({"op":"scenarios","action":"edit","index":1,"name":"Upside","comment":"new","inputs":[{"address":"$a$1","value":"25"},{"address":"B1","value":"40"}]})).unwrap();
    assert_eq!(c.snapshot()["calculated"]["result"]["A1"], 450.0);
    call(&mut c,json!({"op":"scenarios","action":"merge","sheetId":"result","sourceSheetId":"inputs","indices":[0,1]})).unwrap();
    assert_eq!(scenarios::items(&c.workspace["sheets"][1]).count(), 2);
    call(
        &mut c,
        json!({"op":"scenarios","action":"delete","index":0}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["scenarios"]["show"], 0);
    let path = std::env::temp_dir().join("folio-scenarios-recovery.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut loaded = Core::new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(loaded.workspace, c.workspace);
}
#[test]
fn scenarios_validate_limits_duplicates_stale_and_cancelled_measurement() {
    let mut c = new();
    add(&mut c, "Base", "10", "20");
    let before = c.snapshot();
    for r in [
        json!({"op":"scenarios","action":"add","name":"base","inputs":[{"address":"A1","value":"2"}]}),
        json!({"op":"scenarios","action":"add","name":"Bad","inputs":[{"address":"XFE1","value":"2"}]}),
        json!({"op":"scenarios","action":"add","name":"Bad","inputs":[{"address":"A1","value":"x".repeat(256)}]}),
        json!({"op":"scenarios","action":"delete","index":99}),
        json!({"op":"scenarioShow","index":99}),
    ] {
        assert!(call(&mut c, r).is_err());
        assert_eq!(c.snapshot(), before);
    }
    assert!(c.dispatch(json!({"op":"scenarioShow","workspaceId":c.workspace["id"],"baseRevision":9999,"sheetId":"inputs","index":0})).is_err());
    let refs = scenarios::addresses("A1:P2,A1", 32).unwrap();
    assert_eq!(refs.len(), 32);
    assert!(scenarios::addresses("A1:Q2", 32).is_err());
    add(&mut c, "Changed", "12", "24");
    let before = c.snapshot();
    let prepared=c.dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"scenarioShow","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"sheetId":"inputs","index":1}})).unwrap();
    assert_eq!(c.snapshot(), before);
    call(&mut c, json!({"op":"scenarioCapture","range":"A1:B1"})).unwrap();
    c.dispatch(json!({"op":"cancelLayoutEdit","token":prepared["layoutToken"]}))
        .unwrap();
    assert_eq!(c.snapshot(), before);
}
#[test]
fn scenarios_apply_validation_and_formula_replacement_are_atomic() {
    let mut c = new();
    add(&mut c, "Invalid", "-2", "30");
    let mut w = c.workspace.clone();
    w["sheets"][0]["dataValidations"] = json!([{"id":"positive","type":"whole","operator":"greaterThan","formula1":"0","sqref":"A1","showErrorMessage":true,"errorStyle":"stop"}]);
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    let before = c.snapshot();
    assert!(call(&mut c, json!({"op":"scenarioShow","index":0})).is_err());
    assert_eq!(c.snapshot(), before);
    call(&mut c,json!({"op":"scenarios","action":"add","name":"Formula replacement","inputs":[{"address":"C1","value":"50"}]})).unwrap();
    let before = c.snapshot();
    assert!(call(&mut c, json!({"op":"scenarioShow","index":1})).is_err());
    assert_eq!(c.snapshot(), before);
    call(
        &mut c,
        json!({"op":"scenarioShow","index":1,"replaceFormulas":true}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["C1"], "50");
}
#[test]
fn scenarios_summary_is_static_independent_and_undoable() {
    let mut c = new();
    add(&mut c, "Base", "10", "20");
    add(&mut c, "Growth", "15", "30");
    let source = c.workspace["sheets"].clone();
    call(&mut c, json!({"op":"scenarioSummary","resultCells":"C1"})).unwrap();
    assert_eq!(c.workspace["sheets"][2]["name"], "Scenario Summary");
    let summary = &c.snapshot()["calculated"]["scenario-summary-missing"];
    assert!(summary.is_null());
    let snap = c.snapshot();
    let sid = c.workspace["sheets"][2]["id"].as_str().unwrap();
    assert_eq!(snap["calculated"][sid]["B9"], 200.0);
    assert_eq!(snap["calculated"][sid]["C9"], 200.0);
    assert_eq!(snap["calculated"][sid]["D9"], 450.0);
    assert_eq!(c.workspace["sheets"][0]["cells"], source[0]["cells"]);
    assert_eq!(c.workspace["sheets"][1], source[1]);
    export(&mut c, "summary.xlsx");
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], source);
}
#[test]
fn scenarios_xlsx_roundtrip_preserves_unknown_metadata_and_strings() {
    let mut c = new();
    add(&mut c, "Base", "10", "20");
    let mut p = export(&mut c, "new.xlsx");
    let path = crate::xlsx::import(&p).unwrap().0[0]["officePath"]
        .as_str()
        .unwrap()
        .to_string();
    let xml = p
        .text(&path)
        .unwrap()
        .replace(
            "<scenarios ",
            "<scenarios xmlns:z=\"urn:test\" z:future=\"keep\" ",
        )
        .replace("<scenario ", "<scenario z:color=\"green\" ")
        .replace("<inputCells ", "<inputCells z:future=\"input\" ");
    p.set(&path, xml);
    let mut c = import(&p);
    let sid = c.workspace["sheets"][0]["id"].clone();
    let same = export(&mut c, "unchanged.xlsx");
    assert_eq!(same.bytes().unwrap(), p.bytes().unwrap());
    call(&mut c,json!({"op":"scenarios","action":"edit","sheetId":sid,"index":0,"name":"literal _x0041_ & 😀","comment":"first\nsecond\tthird","inputs":[{"address":"A1","value":"_x0041_"},{"address":"B1","value":"line\nnext"}]})).unwrap();
    let out = export(&mut c, "metadata.xlsx");
    let xml = out.text(&path).unwrap();
    assert!(xml.contains("z:future=\"keep\""));
    assert!(xml.contains("z:color=\"green\""));
    assert!(xml.contains("z:future=\"input\""));
    let again = import(&out);
    let v = &again.workspace["sheets"][0]["scenarios"]["items"][0];
    assert_eq!(v["name"], "literal _x0041_ & 😀");
    assert_eq!(v["comment"], "first\nsecond\tthird");
    assert_eq!(v["inputs"][0]["value"], "_x0041_");
    assert_eq!(v["inputs"][1]["value"], "line\nnext");
}
#[test]
fn scenarios_structural_references_and_removed_inputs_survive_export() {
    for imported in [false, true] {
        let mut c = new();
        add(&mut c, "Base", "10", "20");
        if imported {
            c = import(&export(&mut c, "structure-before.xlsx"));
        }
        let sid = c.workspace["sheets"][0]["id"].clone();
        call(
            &mut c,
            json!({"op":"structure","sheetId":sid,"action":"insertRows","at":1,"count":1}),
        )
        .unwrap();
        let inputs = &c.workspace["sheets"][0]["scenarios"]["items"][0]["inputs"];
        assert_eq!(inputs[0]["address"], "A2");
        assert_eq!(inputs[1]["address"], "B2");
        let out = export(
            &mut c,
            if imported {
                "shift-imported.xlsx"
            } else {
                "shift-new.xlsx"
            },
        );
        let again = import(&out);
        assert_eq!(
            again.workspace["sheets"][0]["scenarios"]["items"][0]["inputs"][0]["address"],
            "A2"
        );
        c.dispatch(json!({"op":"undo"})).unwrap();
        assert_eq!(
            c.workspace["sheets"][0]["scenarios"]["items"][0]["inputs"][0]["address"],
            "A1"
        );
    }
}

#[test]
fn scenarios_deleted_cells_do_not_retarget_or_break_surviving_inputs() {
    for imported in [false, true] {
        let mut c = new();
        call(&mut c,json!({"op":"scenarios","action":"add","name":"Rows","inputs":[{"address":"A1","value":"111"},{"address":"A2","value":"222"}]})).unwrap();
        if imported {
            c = import(&export(&mut c, "deleted-before.xlsx"));
        }
        let sid = c.workspace["sheets"][0]["id"].clone();
        call(
            &mut c,
            json!({"op":"structure","sheetId":sid,"action":"deleteRows","at":1,"count":1}),
        )
        .unwrap();
        let list = &c.workspace["sheets"][0]["scenarios"]["items"][0]["inputs"];
        assert_eq!(list[0]["deleted"], true);
        assert_eq!(list[0]["address"], "A1");
        assert_eq!(list[1]["address"], "A1");
        call(&mut c, json!({"op":"scenarioShow","sheetId":sid,"index":0})).unwrap();
        assert_eq!(c.workspace["sheets"][0]["cells"]["A1"], "222");
        let out = export(
            &mut c,
            if imported {
                "deleted-imported.xlsx"
            } else {
                "deleted-new.xlsx"
            },
        );
        let again = import(&out);
        assert_eq!(
            again.workspace["sheets"][0]["scenarios"]["items"][0]["inputs"][0]["deleted"],
            true
        );
    }
}
#[test]
fn scenarios_capture_formula_values_and_summary_errors_are_typed() {
    let mut c = new();
    let values = call(&mut c, json!({"op":"scenarioCapture","range":"A1:C1"})).unwrap();
    assert_eq!(values["inputs"][2]["value"], "200.0");
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"inputs","address":"D1","value":"=1/0"},{"sheetId":"inputs","address":"E1","value":"'#DIV/0!"}]})).unwrap();
    add(&mut c, "Base", "10", "20");
    let before = c.workspace["sheets"].clone();
    call(
        &mut c,
        json!({"op":"scenarioSummary","resultCells":"D1:E1"}),
    )
    .unwrap();
    assert_eq!(c.workspace["sheets"][2]["cells"]["B9"], "#DIV/0!");
    assert_eq!(c.workspace["sheets"][2]["cells"]["B10"], "'#DIV/0!");
    assert_eq!(c.workspace["sheets"][0]["cells"], before[0]["cells"]);
    let p = export(&mut c, "summary-errors.xlsx");
    let projection = crate::xlsx::import(&p).unwrap().0;
    let xml = p
        .text(projection[2]["officePath"].as_str().unwrap())
        .unwrap();
    let d = crate::office::parse(xml).unwrap();
    let cell = |a| {
        d.descendants()
            .find(|n| n.tag_name().name() == "c" && n.attribute("r") == Some(a))
            .unwrap()
    };
    assert_eq!(cell("B9").attribute("t"), Some("e"));
    assert_ne!(cell("B10").attribute("t"), Some("e"));
}
#[test]
fn scenarios_protection_and_legacy_hydration_preserve_intentional_deletion() {
    let mut c = new();
    add(&mut c, "Base", "10", "20");
    let p = export(&mut c, "hydrate-before.xlsx");
    let mut c = import(&p);
    let sid = c.workspace["sheets"][0]["id"].clone();
    call(
        &mut c,
        json!({"op":"scenarios","action":"delete","sheetId":sid,"index":0}),
    )
    .unwrap();
    let path = std::env::temp_dir().join("folio-scenarios-empty.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut loaded = new();
    loaded.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(scenarios::items(&loaded.workspace["sheets"][0]).count(), 0);
    let mut p = p.clone();
    let path = crate::xlsx::import(&p).unwrap().0[0]["officePath"]
        .as_str()
        .unwrap()
        .to_string();
    let xml = p
        .text(&path)
        .unwrap()
        .replace("<scenarios ", "<sheetProtection sheet=\"1\"/><scenarios ");
    p.set(&path, xml);
    let mut c = import(&p);
    let sid = c.workspace["sheets"][0]["id"].clone();
    let before = c.snapshot();
    assert!(call(&mut c, json!({"op":"scenarioShow","sheetId":sid,"index":0})).is_err());
    assert!(call(
        &mut c,
        json!({"op":"scenarios","action":"delete","sheetId":sid,"index":0})
    )
    .is_err());
    assert_eq!(c.snapshot(), before);
}

fn source_file(package: &Package, name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("folio-scenarios-merge-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, package.bytes().unwrap()).unwrap();
    path
}
#[test]
fn scenarios_external_source_is_frozen_read_only_and_merge_history_roundtrips() {
    let mut source = new();
    add(&mut source, "From sales", "7", "9");
    let p = export(&mut source, "merge-source.xlsx");
    let path = source_file(&p, "frozen-source.xlsx");
    let bytes = std::fs::read(&path).unwrap();
    let mut target = import(&p);
    let sid = target.workspace["sheets"][0]["id"].clone();
    let before = target.snapshot();
    let preview = call(
        &mut target,
        json!({"op":"inspectScenarioSource","sheetId":sid,"path":path}),
    )
    .unwrap();
    assert_eq!(target.snapshot(), before);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(preview["sheets"][0]["name"], "Inputs");
    // IDs collide between imported workbooks; their identities remain separate.
    assert_eq!(preview["sheets"][0]["id"], sid);
    std::fs::write(&path, b"changed after preview").unwrap();
    let request = json!({"op":"scenarios","action":"mergeExternal","sheetId":sid,"sourceSheetId":sid,"sourceToken":preview["token"],"indices":[0]});
    call(&mut target, request).unwrap();
    assert_eq!(
        target.workspace["sheets"][0]["scenarios"]["items"][1]["name"],
        "From sales (2)"
    );
    assert_eq!(target.snapshot()["calculated"], before["calculated"]);
    assert_eq!(
        target.workspace["officePackages"],
        before["workspace"]["officePackages"]
    );
    assert_eq!(
        target.workspace["document"],
        before["workspace"]["document"]
    );
    let merged = target.workspace["sheets"].clone();
    target.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(target.workspace["sheets"], before["workspace"]["sheets"]);
    target.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(target.workspace["sheets"], merged);
    let recovery = path.with_extension("folio");
    target
        .dispatch(json!({"op":"save","path":recovery}))
        .unwrap();
    let mut loaded = Core::new();
    loaded
        .dispatch(json!({"op":"load","path":recovery}))
        .unwrap();
    assert_eq!(loaded.workspace, target.workspace);
    let out = export(&mut target, "merge-roundtrip.xlsx");
    let mut again = import(&out);
    call(
        &mut again,
        json!({"op":"scenarioShow","sheetId":sid,"index":1}),
    )
    .unwrap();
    assert_eq!(
        again.snapshot()["calculated"][sid.as_str().unwrap()]["C1"],
        63.0
    );
}
#[test]
fn scenarios_external_invalid_closed_or_stale_sources_fail_without_mutation() {
    let mut source = new();
    add(&mut source, "Source", "1", "2");
    let package = export(&mut source, "merge-errors-source.xlsx");
    let path = source_file(&package, "errors-source.xlsx");
    let mut c = new();
    let before = c.snapshot();
    let preview = call(&mut c, json!({"op":"inspectScenarioSource","path":path})).unwrap();
    for (token, indices) in [
        (json!(99999), json!([0])),
        (preview["token"].clone(), json!([99])),
        (preview["token"].clone(), json!([])),
    ] {
        assert!(call(&mut c, json!({"op":"scenarios","action":"mergeExternal","sourceToken":token,"sourceSheetId":"office-1","indices":indices})).is_err());
        assert_eq!(c.snapshot(), before);
    }
    call(
        &mut c,
        json!({"op":"closeScenarioSource","sourceToken":preview["token"]}),
    )
    .unwrap();
    let request = json!({"op":"scenarios","action":"mergeExternal","sourceToken":preview["token"],"sourceSheetId":"office-1","indices":[0]});
    assert!(call(&mut c, request).is_err());
    assert_eq!(c.snapshot(), before);
    let preview = call(&mut c, json!({"op":"inspectScenarioSource","path":path})).unwrap();
    // Even replacement with the same identity/revision invalidates the source.
    c.dispatch(json!({"op":"replace","workspace":c.workspace}))
        .unwrap();
    let before = c.snapshot();
    assert!(call(&mut c,json!({"op":"scenarios","action":"mergeExternal","sourceToken":preview["token"],"sourceSheetId":"office-1","indices":[0]})).is_err());
    assert_eq!(c.snapshot(), before);
    let mut protected = package.clone();
    let part = crate::xlsx::import(&protected).unwrap().0[0]["officePath"]
        .as_str()
        .unwrap()
        .to_string();
    let xml = protected
        .text(&part)
        .unwrap()
        .replace("<scenarios", "<sheetProtection sheet=\"1\"/><scenarios");
    protected.set(&part, xml);
    let protected_path = source_file(&protected, "protected-source.xlsx");
    let preview = call(
        &mut c,
        json!({"op":"inspectScenarioSource","path":protected_path}),
    )
    .unwrap();
    assert!(preview["sheets"][0]["unavailableReason"].is_string());
    assert!(call(&mut c,json!({"op":"scenarios","action":"mergeExternal","sourceToken":preview["token"],"sourceSheetId":"office-1","indices":[0]})).is_err());
    assert_eq!(c.snapshot(), before);
}
#[test]
fn scenarios_external_custom_formats_remap_collisions_and_reuse_records() {
    let mut source = new();
    add(&mut source, "Formatted", "7", "9");
    let mut w = source.workspace.clone();
    w["sheets"][0]["cellStyles"] = json!({"A1":{"numFmt":"0.0000"}});
    w["sheets"][0]["scenarios"]["items"][0]["inputs"][0]["numFmtId"] = json!(164);
    source
        .dispatch(json!({"op":"replace","workspace":w}))
        .unwrap();
    let source_pkg = export(&mut source, "merge-format-source.xlsx");
    let path = source_file(&source_pkg, "format-source.xlsx");
    let mut c = new();
    let mut w = c.workspace.clone();
    w["sheets"][0]["cellStyles"] = json!({"B1":{"numFmt":"0.00000"}});
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    let p = export(&mut c, "merge-format-target.xlsx");
    let mut c = import(&p);
    let before = c.workspace.clone();
    let preview = call(
        &mut c,
        json!({"op":"inspectScenarioSource","sheetId":"office-1","path":path}),
    )
    .unwrap();
    for _ in 0..2 {
        call(&mut c,json!({"op":"scenarios","action":"mergeExternal","sheetId":"office-1","sourceSheetId":"office-1","sourceToken":preview["token"],"indices":[0]})).unwrap();
    }
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"],
        before["sheets"][0]["cellStyles"]
    );
    assert_eq!(c.workspace["officePackages"], before["officePackages"]);
    let p = export(&mut c, "merge-format-result.xlsx");
    let xml = p.text("xl/styles.xml").unwrap();
    let d = crate::office::parse(xml).unwrap();
    let formats: Vec<_> = d
        .descendants()
        .filter(|n| n.tag_name().name() == "numFmt")
        .collect();
    assert_eq!(formats.len(), 2);
    assert!(formats
        .iter()
        .any(|n| n.attribute("numFmtId") == Some("164")
            && n.attribute("formatCode") == Some("0.00000")));
    assert!(formats
        .iter()
        .any(|n| n.attribute("numFmtId") == Some("165")
            && n.attribute("formatCode") == Some("0.0000")));
    let again = import(&p);
    for item in scenarios::items(&again.workspace["sheets"][0]) {
        assert_eq!(item["inputs"][0]["numFmtId"], 165);
    }
    let mut blank = new();
    let preview = call(
        &mut blank,
        json!({"op":"inspectScenarioSource","path":path}),
    )
    .unwrap();
    call(&mut blank,json!({"op":"scenarios","action":"mergeExternal","sourceSheetId":"office-1","sourceToken":preview["token"],"indices":[0]})).unwrap();
    let blank = import(&export(&mut blank, "merge-format-new.xlsx"));
    assert_eq!(
        blank.workspace["sheets"][0]["scenarios"]["items"][0]["inputs"][0]["numFmtId"],
        164
    );
}
#[test]
fn scenarios_merge_unicode_and_maximum_length_names_receive_valid_suffixes() {
    let mut c = new();
    let name = format!("{}a", "😀".repeat(127));
    add(&mut c, &name, "1", "2");
    add(&mut c, "ÅNGSTRÖM", "3", "4");
    call(
        &mut c,
        json!({"op":"scenarios","action":"merge","sourceSheetId":"inputs","indices":[0,1,0]}),
    )
    .unwrap();
    let items: Vec<_> = scenarios::items(&c.workspace["sheets"][0]).collect();
    assert_eq!(items.len(), 5);
    assert!(items[2]["name"].as_str().unwrap().ends_with(" (2)"));
    assert_eq!(items[3]["name"], "ÅNGSTRÖM (2)");
    assert!(items[4]["name"].as_str().unwrap().ends_with(" (3)"));
    assert!(items
        .iter()
        .all(|i| i["name"].as_str().unwrap().encode_utf16().count() <= 255));
    let before = c.workspace.clone();
    assert!(call(
        &mut c,
        json!({"op":"scenarios","action":"merge","sourceSheetId":"inputs","indices":[]})
    )
    .is_err());
    assert_eq!(c.workspace, before);
}
