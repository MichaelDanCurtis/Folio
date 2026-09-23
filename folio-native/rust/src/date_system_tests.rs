use crate::{calculation::WorkbookCalculator, workspace::Core};
use serde_json::{json, Value};

fn fixture(mac: bool) -> Core {
    let mut core = Core::new();
    core.dispatch(json!({"op":"importOffice","path":format!("{}/../test/fixtures/date-system-{}.xlsx", env!("CARGO_MANIFEST_DIR"), if mac {1904} else {1900})})).unwrap();
    core
}

#[test]
fn calendar_formulas_match_paired_gregorian_fixtures() {
    let expected: Value =
        serde_json::from_str(include_str!("../../test/fixtures/date-systems.json")).unwrap();
    let folder = std::env::temp_dir().join("folio-date-system-tests");
    std::fs::create_dir_all(&folder).unwrap();
    let mut results = vec![];
    let mut failures = vec![];
    for mac in [false, true] {
        let year = if mac { 1904 } else { 1900 };
        let e = &expected[format!("date-system-{year}")];
        let mut c = fixture(mac);
        let snapshot = c.snapshot();
        assert_eq!(snapshot["date1904"], mac);
        for (a, text) in [("M1", "1904-01-01"), ("M2", "25%"), ("M3", "$12.50")] {
            assert_eq!(snapshot["calculated"]["office-1"][a], text);
        }
        assert!(snapshot["calculationError"].is_null());
        let values = &snapshot["calculated"]["office-1"];
        for (a, expected) in e["expected"].as_object().unwrap() {
            let actual = &values[a];
            let matches = match (actual.as_f64(), expected.as_f64()) {
                (Some(a), Some(b)) => (a - b).abs() <= 1e-10 * b.abs().max(1.0),
                _ => actual == expected,
            };
            if !matches {
                failures.push(format!("{year} {a}: actual {actual}, expected {expected}"));
            }
        }
        for (a, n) in e["dateSerials"].as_object().unwrap() {
            assert_eq!(values[a], *n, "{year} {a}");
        }
        assert_eq!(
            snapshot["formatted"]["office-1"]["A2"]["text"],
            "1904-01-01"
        );
        let clock = values["L1"].as_f64().unwrap();
        assert!((0.0..1.0).contains(&clock));
        results.push(values.clone());
        c.dispatch(json!({"op":"cells","cells":[{"sheetId":"office-1","address":"D2","value":"1904-02-29"},{"sheetId":"office-1","address":"D3","value":"=DAY(D2)"},{"sheetId":"office-1","address":"D4","value":"25%"},{"sheetId":"office-1","address":"D5","value":"TRUE"}]})).unwrap();
        assert_eq!(
            c.snapshot()["calculated"]["office-1"]["D2"],
            e["dateSerials"]["A3"]
        );
        assert_eq!(c.snapshot()["calculated"]["office-1"]["D3"], 29.0);
        assert_eq!(
            c.snapshot()["formatted"]["office-1"]["D2"]["text"],
            "1904-02-29"
        );
        c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":folder.join(format!("calendar-{year}.xlsx"))})).unwrap();
        c.dispatch(json!({"op":"save","path":folder.join(format!("calendar-{year}.folio"))}))
            .unwrap();
        std::fs::write(
            folder.join(format!("calendar-{year}.json")),
            serde_json::to_vec_pretty(&c.snapshot()).unwrap(),
        )
        .unwrap();
        c.dispatch(json!({"op":"new","blank":true})).unwrap();
        let reopened = c
            .dispatch(json!({"op":"load","path":folder.join(format!("calendar-{year}.folio"))}))
            .unwrap();
        assert_eq!(reopened["date1904"], mac);
        assert_eq!(
            reopened["calculated"]["office-1"]["D2"],
            e["dateSerials"]["A3"]
        );
        // NOW is intentionally volatile; inspect deterministic formula results individually.
        for (a, v) in values
            .as_object()
            .unwrap()
            .iter()
            .filter(|(a, _)| a.starts_with('F'))
        {
            assert_eq!(reopened["calculated"]["office-1"][a], *v, "reopen {a}");
        }
        c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":folder.join(format!("calendar-{year}-reopened.xlsx"))})).unwrap();
    }
    for a in expected["date-system-1900"]["comparisons"]
        .as_array()
        .unwrap()
    {
        let a = a.as_str().unwrap();
        match (results[0][a].as_f64(), results[1][a].as_f64()) {
            (Some(x), Some(y))
                if x.is_finite() && y.is_finite() && (x - y).abs() <= 1e-9 * x.abs().max(1.0) => {}
            _ => failures.push(format!(
                "Financial calendar comparison {a}: {} vs {}",
                results[0][a], results[1][a]
            )),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn calendar_survives_table_resize_and_structural_filter_recalculation() {
    for table in [true, false] {
        let mut c = fixture(true);
        let inputs = [
            ("P1", "Group"),
            ("Q1", "Date"),
            ("R1", "Year"),
            ("P2", "Keep"),
            ("P3", "Keep"),
            ("P4", "Other"),
            ("P5", "Keep"),
            ("Q2", "0"),
            ("Q3", "366"),
            ("Q4", "731"),
            ("Q5", "1096"),
            ("R2", "=YEAR(Q2)"),
            ("R3", "=YEAR(Q3)"),
            ("R4", "=YEAR(Q4)"),
            ("R5", "=YEAR(Q5)"),
        ];
        c.dispatch(json!({"op":"cells","cells":inputs.iter().map(|(a,v)|json!({"sheetId":"office-1","address":a,"value":v})).collect::<Vec<_>>()})).unwrap();
        let operation = if table { "table" } else { "autoFilter" };
        let mut request = json!({"op":operation,"sheetId":"office-1","action":if table {"create"} else {"enable"},"name":"CalendarDates","range":if table {"P1:R4"} else {"P1:R5"}});
        c.dispatch(request.clone()).unwrap();
        if table {
            request["tableId"] = c.workspace["sheets"][0]["tables"][0]["id"].clone();
        }
        let filter_key = if table { "tables" } else { "autoFilter" };
        request["action"] = json!("filter");
        for (column, value) in [(0, "Keep"), (2, "1904")] {
            request["column"] = json!(column);
            request["rule"] = json!({"kind":"values","values":[value],"blank":false});
            c.dispatch(request.clone()).unwrap();
        }
        if table {
            request["action"] = json!("resize");
            request["range"] = json!("P1:R5");
            c.dispatch(request).unwrap();
        }
        let filter = if table {
            &c.workspace["sheets"][0][filter_key][0]
        } else {
            &c.workspace["sheets"][0][filter_key]
        };
        assert_eq!(
            filter["filteredRows"],
            json!([3, 4, 5]),
            "resize {operation}"
        );
        let before = c.workspace["sheets"].clone();
        c.dispatch(json!({"op":"structure","sheetId":"office-1","action":"deleteColumns","at":16,"count":1})).unwrap();
        let state = c.snapshot();
        assert_eq!(state["date1904"], true);
        assert_eq!(state["calculated"]["office-1"]["Q2"], 1904.0);
        let filter = if table {
            &c.workspace["sheets"][0][filter_key][0]
        } else {
            &c.workspace["sheets"][0][filter_key]
        };
        assert_eq!(
            filter["filteredRows"],
            json!([3, 4, 5]),
            "structure {operation}"
        );
        let after = c.workspace["sheets"].clone();
        c.dispatch(json!({"op":"undo"})).unwrap();
        assert_eq!(c.workspace["sheets"], before);
        c.dispatch(json!({"op":"redo"})).unwrap();
        assert_eq!(c.workspace["sheets"], after);
        let folder = std::env::temp_dir().join("folio-date-system-tests");
        std::fs::create_dir_all(&folder).unwrap();
        c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":folder.join(format!("calendar-{operation}-structure.xlsx"))})).unwrap();
    }
}

#[test]
fn older_workspaces_recover_explicit_date_text_and_keep_real_edits() {
    let mut c = fixture(true);
    let folder = std::env::temp_dir().join("folio-date-system-tests");
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("legacy.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for (a, original) in [("M1", "1904-01-01"), ("M2", "25%"), ("M3", "$12.50")] {
        saved["sheets"][0]["cells"][a] = json!(original);
        saved["officeWorkbook"]["baseline"][0]["cells"][a] = json!(original);
    }
    saved["sheets"][0]["cells"]["M3"] = json!("user edit");
    saved["officeWorkbook"]["report"]["warnings"] = json!([crate::xlsx::DATE1904_LEGACY_WARNING]);
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let s = c.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(s["calculated"]["office-1"]["M1"], "1904-01-01");
    assert_eq!(s["calculated"]["office-1"]["M2"], "25%");
    assert_eq!(s["calculated"]["office-1"]["M3"], "user edit");
    assert_eq!(
        s["workspace"]["officeWorkbook"]["report"]["warnings"][0],
        crate::xlsx::DATE1904_NOTICE
    );
}

#[test]
fn changing_calendars_invalidates_calculation_and_keeps_models_independent() {
    let sheets = json!([{"id":"s","name":"Dates","cells":{"A1":"=DATE(1904,1,1)","A2":"=TEXT(A1,\"yyyy-mm-dd\")","A3":"1904-01-01"}}]);
    let mut a = WorkbookCalculator::default();
    let mut b = WorkbookCalculator::default();
    a.sync_calendar(&sheets, false);
    b.sync_calendar(&sheets, true);
    assert_eq!(a.values["s"]["A1"], 1462.0);
    assert_eq!(b.values["s"]["A1"], 0.0);
    assert_eq!(b.values["s"]["A3"], 0.0);
    assert_eq!(a.values["s"]["A2"], b.values["s"]["A2"]);
    let old = a.generation;
    a.sync_calendar(&sheets, true);
    assert_eq!(a.generation, old + 1);
    assert_eq!(a.values, b.values);
    a.sync_calendar(&sheets, true);
    assert_eq!(a.generation, old + 1);
    b.sync_calendar(&sheets, false);
    assert_eq!(b.values["s"]["A1"], 1462.0);
    assert_eq!(a.values["s"]["A1"], 0.0);
}
