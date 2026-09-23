use crate::{
    calculation::WorkbookCalculator, calculation_clock::CalculationClock, conditional_dates,
    office::Package, workspace::Core,
};
use serde_json::{json, Value};
fn blank(clock: &str) -> Core {
    let mut c = Core::new();
    c.freeze_clock(clock);
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    c
}
fn id(c: &Core) -> String {
    c.workspace["sheets"][0]["id"].as_str().unwrap().to_owned()
}
fn cells(c: &mut Core, v: Value) {
    let sid = id(c);
    c.dispatch(json!({"op":"cells","cells":v.as_object().unwrap().iter().map(|(a,v)|json!({"sheetId":sid,"address":a,"value":v})).collect::<Vec<_>>()})).unwrap();
}
fn rule(period: &str) -> Value {
    json!({"type":"timePeriod","timePeriod":period,"sqref":"A1:A1048576","style":{"fill":"#ABCDEF","bold":true}})
}
fn command(c: &mut Core, mut request: Value) -> Result<Value, String> {
    request["op"] = json!("conditionalFormat");
    request["baseRevision"] = c.workspace["revision"].clone();
    request["sheetId"] = json!(id(c));
    c.dispatch(request)
}
fn add(c: &mut Core, r: Value) {
    command(c, json!({"action":"add","rule":r})).unwrap();
}
fn matches(c: &mut Core, addresses: &[&str]) -> Vec<String> {
    let v=c.dispatch(json!({"op":"conditionalRange","sheetId":id(c),"baseRevision":c.workspace["revision"],"cells":addresses})).unwrap();
    assert_eq!(v["warnings"], json!([]));
    v["styles"].as_object().unwrap().keys().cloned().collect()
}
fn dir() -> std::path::PathBuf {
    let d = std::env::temp_dir().join("folio-date-rule-tests");
    std::fs::create_dir_all(&d).unwrap();
    d
}
fn export(c: &mut Core, name: &str) -> Package {
    let path = dir().join(name);
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    Package::read(path.to_str().unwrap()).unwrap()
}
#[test]
fn local_clock_today_now_calendar_boundary_offset_and_explicit_recalculation() {
    let mut c = blank("2024-12-31T23:59:59-08:00");
    cells(
        &mut c,
        json!({"A1":"=TODAY()","A2":"=NOW()","A3":"=NOW()-NOW()","A4":"=TODAY(\"UTC\")","A5":"=NOW()-TODAY()"}),
    );
    let sid = id(&c);
    let before = c.snapshot();
    assert_eq!(before["calculated"][&sid]["A1"], 45657.0);
    assert_eq!(before["calculated"][&sid]["A4"], 45658.0);
    assert_eq!(before["calculated"][&sid]["A3"], 0.0);
    assert!((before["calculated"][&sid]["A5"].as_f64().unwrap() - 86399.0 / 86400.0).abs() < 1e-10);
    c.freeze_clock("2025-01-01T00:00:01-08:00");
    let next = c.dispatch(json!({"op":"refreshCalendar"})).unwrap();
    assert_eq!(next["calculated"][&sid]["A1"], 45658.0);
    assert_eq!(next["workspace"], before["workspace"]);
    assert_eq!(next["canUndo"], before["canUndo"]);
    assert_eq!(next["canRedo"], before["canRedo"]);
    c.freeze_clock("2025-01-01T12:00:00-08:00");
    assert_eq!(c.dispatch(json!({"op":"refreshCalendar"})).unwrap(), next); // NOW updates at calculation, not continuously.
    let recalculated = c.dispatch(json!({"op":"recalculate"})).unwrap();
    assert_eq!(recalculated["calculated"][&sid]["A2"], 45658.5);
    assert_eq!(recalculated["workspace"], before["workspace"]);
    c.freeze_clock("2025-01-01T14:00:00-06:00");
    let moved = c.dispatch(json!({"op":"refreshCalendar"})).unwrap();
    assert_eq!(moved["localUtcOffset"], -21600);
    assert!(
        (moved["calculated"][&sid]["A2"].as_f64().unwrap() - (45658.0 + 14.0 / 24.0)).abs() < 1e-10
    );
    c.freeze_clock("2024-12-31T20:00:00-08:00");
    assert_eq!(
        c.dispatch(json!({"op":"refreshCalendar"})).unwrap()["calculated"][&sid]["A1"],
        45657.0
    );
}
#[test]
fn calendar_models_remain_independent_and_both_date_systems_and_dst_use_one_instant() {
    let sheets = json!([{"id":"a","name":"Dates","cells":{"A1":"=TODAY()","A2":"=NOW()","A3":"=NOW()-TODAY()"}}]);
    let mut a = WorkbookCalculator::default();
    let mut b = WorkbookCalculator::default();
    a.clock_override = Some(CalculationClock::fixed("2024-02-29T06:00:00+05:45"));
    b.clock_override = Some(CalculationClock::fixed("2024-02-28T16:15:00-08:00"));
    a.sync_calendar(&sheets, false);
    b.sync_calendar(&sheets, true);
    assert_eq!(a.values["a"]["A1"], 45351.0);
    assert_eq!(b.values["a"]["A1"], 43888.0);
    assert_eq!(a.values["a"]["A3"], 0.25);
    a.clock_override = Some(CalculationClock::fixed("2024-03-10T01:59:59-08:00"));
    a.sync_calendar(&sheets, false);
    let before = a.values["a"]["A2"].as_f64().unwrap();
    a.clock_override = Some(CalculationClock::fixed("2024-03-10T03:00:00-07:00"));
    a.sync_calendar(&sheets, false);
    assert!((a.values["a"]["A2"].as_f64().unwrap() - before - 3601.0 / 86400.0).abs() < 1e-10);
    assert_eq!(b.values["a"]["A1"], 43888.0);
}
#[test]
fn every_date_period_weekend_leapday_and_year_rollover_with_fractional_times() {
    for (clock, period, yes, no) in [
        (
            "2025-01-01T12:00:00-08:00",
            "yesterday",
            "2024-12-31",
            "2024-12-30",
        ),
        (
            "2025-01-01T12:00:00-08:00",
            "today",
            "2025-01-01",
            "2025-01-02",
        ),
        (
            "2025-01-01T12:00:00-08:00",
            "tomorrow",
            "2025-01-02",
            "2025-01-03",
        ),
        (
            "2025-01-01T12:00:00-08:00",
            "last7Days",
            "2024-12-26",
            "2024-12-25",
        ),
        (
            "2025-01-05T12:00:00-08:00",
            "lastWeek",
            "2025-01-04",
            "2025-01-05",
        ),
        (
            "2025-01-05T12:00:00-08:00",
            "thisWeek",
            "2025-01-11",
            "2025-01-12",
        ),
        (
            "2025-01-04T12:00:00-08:00",
            "nextWeek",
            "2025-01-05",
            "2025-01-04",
        ),
        (
            "2025-01-01T12:00:00-08:00",
            "lastMonth",
            "2024-12-31",
            "2025-01-01",
        ),
        (
            "2024-02-29T12:00:00-08:00",
            "thisMonth",
            "2024-02-29",
            "2024-03-01",
        ),
        (
            "2024-12-31T12:00:00-08:00",
            "nextMonth",
            "2025-01-31",
            "2025-02-01",
        ),
    ] {
        let mut c = blank(clock);
        cells(
            &mut c,
            json!({"A1":yes,"A2":no,"A3":"=A1+0.99999","A4":"=A2+0.99999","A5":"=\"2025-01-01\"","A6":"=1/0","A7":"=TRUE()","A1048576":yes}),
        );
        add(&mut c, rule(period));
        assert_eq!(
            matches(
                &mut c,
                &["A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A1048576"]
            ),
            vec!["A1", "A1048576", "A3"],
            "{period}"
        );
    }
}
#[test]
fn date_rule_rollover_priority_formula_only_refresh_and_measured_edits_keep_clock() {
    let mut c = blank("2024-12-31T23:59:59-08:00");
    cells(&mut c, json!({"A1":"2024-12-31","A2":"2025-01-01"}));
    add(&mut c, rule("today"));
    let before = c.snapshot();
    assert_eq!(matches(&mut c, &["A1", "A2"]), vec!["A1"]);
    c.freeze_clock("2025-01-01T00:00:01-08:00");
    let state = c.dispatch(json!({"op":"refreshCalendar"})).unwrap();
    assert_eq!(state["calculated"], before["calculated"]);
    assert_eq!(matches(&mut c, &["A1", "A2"]), vec!["A2"]);
    assert_eq!(state["workspace"], before["workspace"]);
    assert!(state["formatted"][id(&c)]["A1"]["conditionalStyle"]["fill"].is_null());
    assert_eq!(
        state["formatted"][id(&c)]["A2"]["conditionalStyle"]["fill"],
        "#ABCDEF"
    );
    let prepared=c.dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"cells","cells":[{"sheetId":id(&c),"address":"B1","value":"=TODAY()"}]}})).unwrap();
    assert_eq!(prepared["calculated"][id(&c)]["B1"], 45658.0);
    let mut sizes = json!({});
    for (id, rows) in prepared["layoutRows"].as_object().unwrap() {
        sizes[id] = json!({});
        for row in rows.as_array().unwrap() {
            sizes[id][row.to_string()] = json!(30);
        }
    }
    c.dispatch(json!({"op":"commitLayoutEdit","token":prepared["layoutToken"],"sizes":sizes}))
        .unwrap();
    assert_eq!(c.snapshot()["calculated"][id(&c)]["B1"], 45658.0);
    add(
        &mut c,
        json!({"type":"expression","sqref":"A1:A2","style":{"fill":"#FF0000"},"formulas":["TRUE"],"stopIfTrue":true}),
    );
    assert_eq!(
        c.snapshot()["formatted"][id(&c)]["A2"]["conditionalStyle"]["fill"],
        "#FF0000"
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        c.snapshot()["formatted"][id(&c)]["A2"]["conditionalStyle"]["fill"],
        "#ABCDEF"
    );
}
#[test]
fn date_rules_ooxml_all_periods_legacy_recovery_structural_and_failed_changes() {
    let mut c = blank("2025-01-01T12:00:00-08:00");
    cells(&mut c, json!({"A1":"2025-01-01","B1":"=NOW()"}));
    for period in conditional_dates::PERIODS {
        let mut r = rule(period);
        r["sqref"] = json!("A1:A4 C1:C4");
        add(&mut c, r);
    }
    let pkg = export(&mut c, "date-periods.xlsx");
    let (s, _) = crate::xlsx::import(&pkg).unwrap();
    assert_eq!(s[0]["conditionalFormats"].as_array().unwrap().len(), 10);
    for r in s[0]["conditionalFormats"].as_array().unwrap() {
        assert_ne!(r["unsupported"], true);
        assert_eq!(r["formulas"].as_array().unwrap().len(), 1);
    }
    let mut legacy = s[0].clone();
    for r in legacy["conditionalFormats"].as_array_mut().unwrap() {
        r["unsupported"] = json!(true);
    }
    crate::conditional_format::hydrate(&pkg, s[0]["officePath"].as_str().unwrap(), &mut legacy)
        .unwrap();
    assert_eq!(legacy["conditionalFormats"], s[0]["conditionalFormats"]);
    let before = c.snapshot();
    let mut bad = rule("today");
    bad["timePeriod"] = json!("someday");
    assert!(command(&mut c, json!({"action":"add","rule":bad})).is_err());
    assert_eq!(c.snapshot(), before);
    c.dispatch(json!({"op":"structure","sheetId":id(&c),"action":"insertRows","at":1,"count":1}))
        .unwrap();
    let pkg = export(&mut c, "date-periods-shifted.xlsx");
    let (s, _) = crate::xlsx::import(&pkg).unwrap();
    for r in s[0]["conditionalFormats"].as_array().unwrap() {
        assert_eq!(r["sqref"], "A2:A5 C2:C5");
        assert!(r["formulas"][0].as_str().unwrap().contains("A2"));
    }
    let path = dir().join("date-rules.folio");
    c.dispatch(json!({"op":"save","path":path})).unwrap();
    c.freeze_clock("2025-01-01T15:00:00-08:00");
    let reload = c.dispatch(json!({"op":"load","path":path})).unwrap();
    assert_eq!(reload["calculated"][id(&c)]["B2"], 45658.625);
    export(&mut c, "date-periods-recovered.xlsx");
    std::fs::write(
        dir().join("date-periods-recovered.json"),
        serde_json::to_vec_pretty(&reload).unwrap(),
    )
    .unwrap();
}

#[test]
fn date_style_edits_preserve_imported_predicates_and_missing_formulas() {
    let mut c = blank("2025-01-01T12:00:00-08:00");
    cells(&mut c, json!({"A1":"2025-01-01","A2":"2025-01-02"}));
    add(&mut c, rule("today"));
    // An external writer is allowed to supply a more specific predicate.
    c.workspace["sheets"][0]["conditionalFormats"][0]["formulas"] =
        json!(["AND(A1=TODAY(),$B$1=1)"]);
    let mut edit = c.workspace["sheets"][0]["conditionalFormats"][0].clone();
    edit["style"] = json!({"fill":"#FF0000"});
    command(&mut c, json!({"action":"edit","index":0,"rule":edit})).unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["conditionalFormats"][0]["formulas"],
        json!(["AND(A1=TODAY(),$B$1=1)"])
    );
    assert!(matches(&mut c, &["A1", "A2"]).is_empty());
    edit["timePeriod"] = json!("tomorrow");
    command(&mut c, json!({"action":"edit","index":0,"rule":edit})).unwrap();
    assert_eq!(matches(&mut c, &["A1", "A2"]), vec!["A2"]);
    c.workspace["sheets"][0]["conditionalFormats"][0]["formulas"] = json!([]);
    c.dispatch(json!({"op":"recalculate"})).unwrap();
    assert_eq!(matches(&mut c, &["A1", "A2"]), vec!["A2"]);
}

#[test]
fn staged_recalculation_resizes_derived_rows_without_changing_history_or_revision() {
    let mut c = blank("2025-01-01T12:00:00-08:00");
    cells(
        &mut c,
        json!({"A1":"=IF(TODAY()=DATE(2025,1,1),\"short\",\"three\"&CHAR(10)&\"new\"&CHAR(10)&\"lines\")","A2":"manual","B1":"=NOW()"}),
    );
    let before = c.snapshot();
    c.freeze_clock("2025-01-02T12:00:00-08:00");
    let staged = c
        .dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"refreshCalendar"}}))
        .unwrap();
    assert_eq!(c.snapshot(), before, "Measuring must not publish results");
    assert_eq!(staged["layoutRows"][id(&c)], json!([1]));
    assert_eq!(staged["calculated"][id(&c)]["A1"], "three\nnew\nlines");
    let result=c.dispatch(json!({"op":"commitLayoutEdit","token":staged["layoutToken"],"sizes":{id(&c):{"1":75}}})).unwrap();
    assert_eq!(
        result["workspace"]["revision"],
        before["workspace"]["revision"]
    );
    assert_eq!(result["canUndo"], before["canUndo"]);
    assert_eq!(result["canRedo"], before["canRedo"]);
    assert_eq!(result["workspace"]["sheets"][0]["rowHeights"]["1"], 75.0);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        c.workspace["sheets"][0]["cells"],
        json!({}),
        "Undo still undoes the cell entry"
    );
    let redo_before = c.snapshot();
    let staged = c
        .dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"recalculate"}}))
        .unwrap();
    c.dispatch(json!({"op":"commitLayoutEdit","token":staged["layoutToken"],"sizes":{}}))
        .unwrap();
    assert_eq!(c.snapshot()["canRedo"], redo_before["canRedo"]);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(
        c.snapshot()["calculated"][id(&c)]["A1"],
        "three\nnew\nlines"
    );
}

#[test]
fn independent_date_rules_import_edit_export_in_both_calendars() {
    for calendar in [1900, 1904] {
        let mut c = blank("2025-01-01T12:00:00-08:00");
        let path = format!(
            "{}/tests/fixtures/date-periods-{calendar}.xlsx",
            env!("CARGO_MANIFEST_DIR")
        );
        c.dispatch(json!({"op":"importOffice","path":path}))
            .unwrap();
        assert_eq!(
            c.workspace["sheets"][0]["conditionalFormats"]
                .as_array()
                .unwrap()
                .len(),
            10
        );
        assert!(c.snapshot()["conditionalWarnings"]
            .as_array()
            .is_none_or(|w| w.is_empty()));
        let mut edit = c.workspace["sheets"][0]["conditionalFormats"][1].clone();
        edit["style"]["bold"] = json!(true);
        command(&mut c, json!({"action":"edit","index":1,"rule":edit})).unwrap();
        assert_eq!(
            c.workspace["sheets"][0]["conditionalFormats"][1]["formulas"],
            json!(["AND(A1=TODAY(),$B$1=1)"])
        );
        export(&mut c, &format!("date-independent-{calendar}.xlsx"));
        let prepared=c.dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"structure","sheetId":id(&c),"action":"insertRows","at":1,"count":1}})).unwrap();
        let mut sizes = json!({});
        for (sheet, rows) in prepared["layoutRows"].as_object().unwrap() {
            sizes[sheet] = json!({});
            for row in rows.as_array().unwrap() {
                sizes[sheet][row.to_string()] = json!(30);
            }
        }
        c.dispatch(json!({"op":"commitLayoutEdit","token":prepared["layoutToken"],"sizes":sizes}))
            .unwrap();
        let pkg = export(&mut c, &format!("date-independent-shifted-{calendar}.xlsx"));
        let doc = crate::office::parse(pkg.text("xl/worksheets/sheet1.xml").unwrap()).unwrap();
        let cell = doc
            .descendants()
            .find(|n| n.has_tag_name("c") && n.attribute("r") == Some("C2"))
            .unwrap();
        let cached = crate::office::child(cell, "v")
            .unwrap()
            .text()
            .unwrap()
            .parse::<f64>()
            .unwrap();
        assert_eq!(
            cached,
            if calendar == 1904 { 44196.5 } else { 45658.5 },
            "Staged structural exports must use the same captured clock"
        );
    }
}
