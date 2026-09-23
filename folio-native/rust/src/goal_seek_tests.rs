use crate::workspace::Core;
use serde_json::{json, Value};

fn core(sheets: Value) -> Core {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = c.workspace.clone();
    w["sheets"] = sheets;
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    c
}
fn setup(input: &str, formula: &str) -> Core {
    core(json!([{"id":"s","name":"Sheet1","cells":{"A1":input,"B1":formula}}]))
}
fn request(c: &mut Core, target: Value, changing: Value, goal: f64) -> Value {
    let generation = c.snapshot()["evaluatedCells"].clone();
    json!({"op":"goalSeekStart","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],
        "calculationGeneration":generation,"target":target,"changing":changing,"goal":goal,"tolerance":1e-8,"maxTrials":200})
}
fn start(c: &mut Core, goal: f64) -> Value {
    let r = request(
        c,
        json!({"sheetId":"s","address":"B1"}),
        json!({"sheetId":"s","address":"A1"}),
        goal,
    );
    c.dispatch(r).unwrap()
}
fn finish(c: &mut Core, mut result: Value) -> Value {
    for _ in 0..10002 {
        if result["status"] != "running" {
            return result;
        }
        result = c
            .dispatch(json!({"op":"goalSeekStep","token":result["token"]}))
            .unwrap();
    }
    panic!("Goal Seek did not stop at its trial limit")
}
fn accept(c: &mut Core, result: &Value, best: bool) -> Result<Value, String> {
    let p=c.dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"goalSeekAccept","token":result["token"],"acceptBest":best}}))?;
    let mut sizes = json!({});
    for (id, rows) in p["layoutRows"].as_object().unwrap() {
        sizes[id] = json!({});
        for row in rows.as_array().unwrap() {
            sizes[id][row.to_string()] = json!(20.0);
        }
    }
    c.dispatch(json!({"op":"commitLayoutEdit","token":p["layoutToken"],"sizes":sizes}))
}

#[test]
fn numerical_linear_nonlinear_flat_negative_and_domain_cases() {
    for (initial, formula, wanted) in [
        ("2", "=A1*3+7", 100.0),
        ("2", "=A1^2", 49.0),
        ("-2", "=A1^2", 49.0),
        ("1", "=A1^2", 0.0),
        ("0", "=LN(A1)", 2.0),
        ("0", "=EXP(A1)", 100.0),
        ("4", "=ABS(A1)", 1.0),
        ("1", "=A1^3-A1", 4.0),
        // Convergence means within the requested residual tolerance, not an
        // assertion that the equation has an exact finite root.
        ("2", "=1/A1", 0.0),
    ] {
        let mut c = setup(initial, formula);
        let before = c.snapshot();
        let started = start(&mut c, wanted);
        let result = finish(&mut c, started);
        assert_eq!(result["status"], "converged", "{formula}: {result}");
        assert!(
            (result["result"].as_f64().unwrap() - wanted).abs() <= 1e-8,
            "{result}"
        );
        assert_eq!(c.snapshot(), before, "Trials changed {formula}'s workspace");
        let accepted = accept(&mut c, &result, false).unwrap();
        assert!((accepted["calculated"]["s"]["B1"].as_f64().unwrap() - wanted).abs() <= 1e-8);
        c.dispatch(json!({"op":"undo"})).unwrap();
        assert_eq!(c.workspace["sheets"], before["workspace"]["sheets"]);
        c.dispatch(json!({"op":"redo"})).unwrap();
        assert!((c.snapshot()["calculated"]["s"]["B1"].as_f64().unwrap() - wanted).abs() <= 1e-8);
    }
}

#[test]
fn discontinuous_constant_and_impossible_results_never_claim_convergence() {
    for formula in ["=IF(A1<0,-1,1)", "=A1^2+1", "=5"] {
        let mut c = setup("2", formula);
        let before = c.snapshot();
        let s = start(&mut c, 0.0);
        let r = finish(&mut c, s);
        assert_ne!(r["status"], "converged", "{formula}: {r}");
        assert!(r["trials"].as_u64().unwrap() <= 200);
        assert_eq!(c.snapshot(), before);
        if r["canApply"] == true {
            assert!(accept(&mut c, &r, false).is_err());
        }
    }
}

#[test]
fn cross_sheet_loan_search_preserves_unrelated_inputs_and_formatting() {
    let mut c = core(json!([
        {"id":"s","name":"Inputs","cells":{"B1":"100000","B2":"180","B3":"0"},
          "cellWatches":{"B3":true},"cellStyles":{"B3":{"numFmt":"0.00%"}}},
        {"id":"o","name":"Summary","cells":{"D5":"=PMT(Inputs!B3/12,Inputs!B2,Inputs!B1)","F1":"=Inputs!B3*100"}}
    ]));
    let original = c.workspace.clone();
    let req = request(
        &mut c,
        json!({"sheetId":"o","address":"$D$5"}),
        json!({"sheetId":"s","address":"b3"}),
        -900.0,
    );
    let s = c.dispatch(req).unwrap();
    let r = finish(&mut c, s);
    assert_eq!(r["status"], "converged", "{r}");
    let rate = r["input"].as_f64().unwrap();
    let monthly = rate / 12.0;
    let independently_computed = -100000.0 * monthly / (1.0 - (1.0 + monthly).powf(-180.0));
    assert!((independently_computed + 900.0).abs() < 1e-7);
    accept(&mut c, &r, false).unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["B1"], "100000");
    assert_eq!(c.workspace["sheets"][0]["cells"]["B2"], "180");
    assert_eq!(
        c.workspace["sheets"][0]["cellStyles"],
        original["sheets"][0]["cellStyles"]
    );
    assert_eq!(
        c.workspace["sheets"][0]["cellWatches"],
        original["sheets"][0]["cellWatches"]
    );
    assert_eq!(
        c.workspace["sheets"][1]["cells"],
        original["sheets"][1]["cells"]
    );
}

#[test]
fn cancellation_stop_best_acceptance_and_tokens_are_isolated() {
    let mut c = setup("2", "=A1*3");
    let before = c.snapshot();
    let s = start(&mut c, 100.0);
    c.dispatch(json!({"op":"goalSeekStep","token":s["token"]}))
        .unwrap();
    let stopped = c
        .dispatch(json!({"op":"goalSeekStop","token":s["token"]}))
        .unwrap();
    assert_eq!(stopped["status"], "stopped");
    assert!(accept(&mut c, &stopped, false).is_err());
    // Keeping an unchanged best trial does not create an edit/history record.
    accept(&mut c, &stopped, true).unwrap();
    assert_eq!(c.snapshot(), before);
    let newer = start(&mut c, 20.0);
    c.dispatch(json!({"op":"goalSeekCancel","token":s["token"]}))
        .unwrap();
    let r = finish(&mut c, newer);
    assert_eq!(r["status"], "converged");
    c.dispatch(json!({"op":"goalSeekCancel","token":r["token"]}))
        .unwrap();
    assert!(accept(&mut c, &r, false).is_err());
    assert_eq!(c.snapshot(), before);
}

#[test]
fn invalid_stale_and_validation_blocked_requests_preserve_work() {
    let mut c = setup("2", "=A1*3");
    let valid = request(
        &mut c,
        json!({"sheetId":"s","address":"B1"}),
        json!({"sheetId":"s","address":"A1"}),
        100.0,
    );
    for patch in [
        json!({"goal":"oops"}),
        json!({"tolerance":0}),
        json!({"tolerance":"x"}),
        json!({"maxTrials":1}),
        json!({"maxTrials":1.5}),
        json!({"target":{"sheetId":"s","address":"A1"}}),
        json!({"changing":{"sheetId":"s","address":"B1"}}),
        json!({"changing":{"sheetId":"s","address":"XFE1"}}),
        json!({"workspaceId":"other"}),
        json!({"calculationGeneration":0}),
    ] {
        let mut r = valid.clone();
        for (k, v) in patch.as_object().unwrap() {
            r[k] = v.clone();
        }
        let before = c.snapshot();
        assert!(c.dispatch(r).is_err(), "{patch}");
        assert_eq!(c.snapshot(), before);
    }
    let s = start(&mut c, 100.0);
    let done = finish(&mut c, s);
    c.dispatch(json!({"op":"cells","cells":[{"sheetId":"s","address":"D1","value":"changed"}]}))
        .unwrap();
    let before = c.snapshot();
    assert!(accept(&mut c, &done, false).is_err());
    assert_eq!(c.snapshot(), before);
    let mut w = c.workspace.clone();
    w["sheets"][0]["dataValidations"] = json!([{"id":"v","sqref":"A1","type":"decimal","operator":"between", "formula1":"0","formula2":"10","errorStyle":"stop","showErrorMessage":true}]);
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    let s = start(&mut c, 100.0);
    let done = finish(&mut c, s);
    let before = c.snapshot();
    assert!(accept(&mut c, &done, false).is_err());
    assert_eq!(c.snapshot(), before);
}

#[test]
fn search_reads_preserve_pending_layout_and_blank_noop() {
    let mut c = setup("", "=A1*2");
    let before = c.snapshot();
    let s = start(&mut c, 0.0);
    let r = finish(&mut c, s);
    accept(&mut c, &r, false).unwrap();
    assert_eq!(c.snapshot(), before);
    let p=c.dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"cells","cells":[{"sheetId":"s","address":"C1","value":"3"}]}})).unwrap();
    let s = start(&mut c, 20.0);
    let r = finish(&mut c, s);
    c.dispatch(json!({"op":"goalSeekCancel","token":r["token"]}))
        .unwrap();
    let mut sizes = json!({});
    for (id, rows) in p["layoutRows"].as_object().unwrap() {
        sizes[id] = json!({});
        for row in rows.as_array().unwrap() {
            sizes[id][row.to_string()] = json!(20.0);
        }
    }
    c.dispatch(json!({"op":"commitLayoutEdit","token":p["layoutToken"],"sizes":sizes}))
        .unwrap();
    assert_eq!(c.workspace["sheets"][0]["cells"]["C1"], "3");
}

#[test]
fn source_protection_merges_and_array_members_reject_before_trials() {
    use crate::office::Package;
    let dir = std::env::temp_dir().join("folio-goal-seek-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let mut base = setup("2", "=A1*3");
    let path = dir.join("source.xlsx");
    base.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    let package = Package::read(path.to_str().unwrap()).unwrap();
    let sheets = crate::xlsx::import(&package).unwrap().0;
    let sheet_path = sheets[0]["officePath"].as_str().unwrap();
    for (label, extra, formula) in [
        ("protected", "<sheetProtection sheet=\"1\"/>", false),
        (
            "merged",
            "<mergeCells count=\"1\"><mergeCell ref=\"A1:A2\"/></mergeCells>",
            false,
        ),
        ("array", "", true),
    ] {
        let mut pkg = package.clone();
        let mut xml = pkg
            .text(sheet_path)
            .unwrap()
            .replace("</sheetData>", &format!("</sheetData>{extra}"));
        if formula {
            xml = xml.replace("<f>", "<f t=\"array\" ref=\"A1:B1\">");
        }
        pkg.set(sheet_path, xml);
        let path = dir.join(format!("{label}.xlsx"));
        std::fs::write(&path, pkg.bytes().unwrap()).unwrap();
        let mut c = Core::new();
        c.dispatch(json!({"op":"importOffice","path":path}))
            .unwrap();
        let sid = c.workspace["sheets"][0]["id"].clone();
        let req = request(
            &mut c,
            json!({"sheetId":sid,"address":"B1"}),
            json!({"sheetId":sid,"address":"A1"}),
            30.0,
        );
        let before = c.snapshot();
        let err = c.dispatch(req).unwrap_err();
        assert!(
            err.contains(label) || label == "array" && err.contains("formula"),
            "{label}: {err}"
        );
        assert_eq!(c.snapshot(), before);
    }
}

#[test]
fn numerical_search_retains_filtered_and_hidden_row_calculation() {
    let mut c = core(json!([{"id":"s","name":"Sheet1","hiddenRows":[2],"cells":{
        "A1":"Amount","A2":"10","A3":"20","B1":"=SUBTOTAL(109,A2:A3)+C1","C1":"0"}}]));
    let req = request(
        &mut c,
        json!({"sheetId":"s","address":"B1"}),
        json!({"sheetId":"s","address":"C1"}),
        70.0,
    );
    let s = c.dispatch(req).unwrap();
    let r = finish(&mut c, s);
    assert_eq!(r["status"], "converged");
    assert!((r["input"].as_f64().unwrap() - 50.0).abs() < 1e-8);
    accept(&mut c, &r, false).unwrap();
    assert_eq!(c.workspace["sheets"][0]["hiddenRows"], json!([2]));
    assert!((c.snapshot()["calculated"]["s"]["B1"].as_f64().unwrap() - 70.0).abs() < 1e-8);
    let mut w = c.workspace.clone();
    w["sheets"][0]["autoFilter"] = json!({"range":"A1:A3","filteredRows":[3],"filters":{}});
    w["sheets"][0]["cells"]["C1"] = json!("0");
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    let req = request(
        &mut c,
        json!({"sheetId":"s","address":"B1"}),
        json!({"sheetId":"s","address":"C1"}),
        70.0,
    );
    let s = c.dispatch(req).unwrap();
    let r = finish(&mut c, s);
    assert_eq!(r["status"], "converged");
    assert!((r["input"].as_f64().unwrap() - 70.0).abs() < 1e-8);
    accept(&mut c, &r, false).unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["autoFilter"]["filteredRows"],
        json!([3])
    );
}

#[test]
fn date1904_search_and_export_retain_the_workbook_calendar() {
    use crate::office::Package;
    let dir = std::env::temp_dir().join("folio-goal-seek-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("date1904-base.xlsx");
    let mut c = setup("0", "=DATE(1904,1,2)+A1");
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    let mut pkg = Package::read(path.to_str().unwrap()).unwrap();
    let mut xml = pkg.text("xl/workbook.xml").unwrap().to_owned();
    let range = {
        let d = crate::office::parse(&xml).unwrap();
        crate::office::child(d.root_element(), "workbookPr")
            .map(|n| n.range())
            .unwrap_or_else(|| {
                let start = crate::office::child(d.root_element(), "sheets")
                    .unwrap()
                    .range()
                    .start;
                start..start
            })
    };
    xml.replace_range(range, "<workbookPr date1904=\"1\"/>");
    pkg.set("xl/workbook.xml", xml);
    let path = dir.join("date1904-before.xlsx");
    std::fs::write(&path, pkg.bytes().unwrap()).unwrap();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    assert_eq!(c.snapshot()["date1904"], true);
    let sid = c.workspace["sheets"][0]["id"].clone();
    let req = request(
        &mut c,
        json!({"sheetId":sid,"address":"B1"}),
        json!({"sheetId":sid,"address":"A1"}),
        11.0,
    );
    let s = c.dispatch(req).unwrap();
    let r = finish(&mut c, s);
    assert_eq!(r["status"], "converged");
    assert!((r["input"].as_f64().unwrap() - 10.0).abs() < 1e-8);
    accept(&mut c, &r, false).unwrap();
    let path = dir.join("date1904-goal.xlsx");
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    c.dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    assert_eq!(c.snapshot()["date1904"], true);
    assert!(
        (c.snapshot()["calculated"][sid.as_str().unwrap()]["B1"]
            .as_f64()
            .unwrap()
            - 11.0)
            .abs()
            < 1e-8
    );
}

#[test]
fn rejected_measurement_can_retry_without_consuming_goal_result() {
    let mut c = setup("2", "=A1*3");
    let before = c.snapshot();
    let started = start(&mut c, 30.0);
    let r = finish(&mut c, started);
    let p = c
        .dispatch(
            json!({"op":"prepareLayoutEdit","edit":{"op":"goalSeekAccept","token":r["token"]}}),
        )
        .unwrap();
    c.dispatch(json!({"op":"cancelLayoutEdit","token":p["layoutToken"]}))
        .unwrap();
    assert_eq!(c.snapshot(), before);
    accept(&mut c, &r, false).unwrap();
    assert!((c.snapshot()["calculated"]["s"]["B1"].as_f64().unwrap() - 30.0).abs() < 1e-8);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(c.workspace["sheets"], before["workspace"]["sheets"]);
}
