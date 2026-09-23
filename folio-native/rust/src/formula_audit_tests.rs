use crate::{calculation::WorkbookCalculator, workspace::Core};
use serde_json::{json, Value};

#[test]
fn structured_columns_and_this_row_follow_table_resize() {
    let mut w = crate::workspace::seed();
    w["sheets"] = json!([{"id":"s","name":"Orders","cells":{"A1":"Qty","B1":"Price","C1":"Amount","A2":"3","B2":"5","A3":"2","B3":"4","F1":"=SUM(Sales[Amount])"}}]);
    let mut c = WorkbookCalculator::default();
    c.sync(&w["sheets"]);
    w = crate::tables::change(
        &w,
        &c.values,
        &json!({"action":"create","sheetId":"s","range":"A1:C3","name":"Sales"}),
    )
    .unwrap()
    .0;
    let id = w["sheets"][0]["tables"][0]["id"].clone();
    c.sync(&w["sheets"]);
    w = crate::tables::change(&w,&c.values,&json!({"action":"columnFormula","sheetId":"s","tableId":id,"column":2,"formula":"=[@Qty]*[@Price]"})).unwrap().0;
    c.sync(&w["sheets"]);
    assert_eq!(
        ranges(&c.audit("s", "F1", 0, 100).unwrap(), "precedents"),
        vec!["Orders!C2:C3"]
    );
    assert_eq!(
        ranges(&c.audit("s", "C3", 0, 100).unwrap(), "precedents"),
        vec!["Orders!A3", "Orders!B3"]
    );
    assert_eq!(c.audit("s", "C3", 0, 100).unwrap()["value"], 8.0);
    assert_eq!(
        c.audit("s", "F1", 0, 100).unwrap()["unresolvedFormulaCount"],
        0
    );
    // Keep the name and cell formulas unchanged; the table extent alone changes.
    w["sheets"][0]["tables"][0]["range"] = json!("A1:C4");
    c.sync(&w["sheets"]);
    assert_eq!(
        ranges(&c.audit("s", "F1", 0, 100).unwrap(), "precedents"),
        vec!["Orders!C2:C4"]
    );
    assert_eq!(
        ranges(&c.audit("s", "C4", 0, 100).unwrap(), "dependents"),
        vec!["Orders!F1"]
    );
}

#[test]
fn inspecting_does_not_cancel_a_staged_layout_transaction() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let sid = c.workspace["sheets"][0]["id"].clone();
    let p=c.dispatch(json!({"op":"prepareLayoutEdit","edit":{"op":"cells","cells":[{"sheetId":sid,"address":"A1","value":"=B1*2"}]}})).unwrap();
    query(&mut c, "B1");
    let mut sizes = json!({});
    for (id, rows) in p["layoutRows"].as_object().unwrap() {
        sizes[id] = json!({});
        for row in rows.as_array().unwrap() {
            sizes[id][row.to_string()] = json!(20.0);
        }
    }
    c.dispatch(json!({"op":"commitLayoutEdit","token":p["layoutToken"],"sizes":sizes}))
        .unwrap();
    assert_eq!(query(&mut c, "B1")["dependentCount"], 1);
}

fn ranges(v: &Value, key: &str) -> Vec<String> {
    v[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            format!(
                "{}!{}",
                a["sheetName"].as_str().unwrap(),
                a["range"].as_str().unwrap()
            )
        })
        .collect()
}
fn query(c: &mut Core, at: &str) -> Value {
    let s = c.snapshot();
    c.dispatch(json!({"op":"formulaAudit","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"calculationGeneration":s["evaluatedCells"],"sheetId":c.workspace["sheets"][0]["id"],"address":at})).unwrap()
}
fn cells(c: &mut Core, values: Value) {
    let id = c.workspace["sheets"][0]["id"].clone();
    c.dispatch(json!({"op":"cells","cells":values.as_object().unwrap().iter().map(|(a,v)|json!({"sheetId":id,"address":a,"value":v})).collect::<Vec<_>>()})).unwrap();
}

#[test]
fn mixed_absolute_relative_cross_sheet_ranges_and_strings_use_calculation_ast() {
    let mut c = WorkbookCalculator::default();
    c.sync(&json!([
        {"id":"a","name":"First","cells":{"D8":"=SUM($A1,B$2,$C$3,A1,'O''Brien data'!$B4:$C5,A:A,2:2)+IF(FALSE,D1,E1)+N(\"Z99\")","F1":"=D8","F2":"=D8+D8"}},
        {"id":"b","name":"O'Brien data","cells":{"B4":"7","E9":"=First!D8"}}
    ]));
    assert!(c.error.is_none());
    let q = c.audit("a", "D8", 0, 100).unwrap();
    assert_eq!(
        ranges(&q, "precedents"),
        vec![
            "First!A1",
            "First!A1:A1048576",
            "First!D1",
            "First!E1",
            "First!A2:XFD2",
            "First!B2",
            "First!C3",
            "O'Brien data!B4:C5"
        ]
    );
    assert_eq!(
        ranges(&q, "dependents"),
        vec!["First!F1", "First!F2", "O'Brien data!E9"]
    );
    assert_eq!(q["warnings"], json!([]));
    assert_eq!(q["unresolvedFormulaCount"], 0);
    assert_eq!(
        ranges(&c.audit("a", "A1048576", 0, 100).unwrap(), "dependents"),
        vec!["First!D8"]
    );
    assert_eq!(
        ranges(&c.audit("a", "XFD2", 0, 100).unwrap(), "dependents"),
        vec!["First!D8"]
    );
    assert_eq!(
        ranges(&c.audit("b", "C5", 0, 100).unwrap(), "dependents"),
        vec!["First!D8"]
    );
    assert_eq!(c.audit("a", "Z99", 0, 100).unwrap()["dependentCount"], 0);
}

#[test]
fn dynamic_broken_and_spilled_references_report_incomplete_dependents() {
    let mut c = WorkbookCalculator::default();
    c.sync(&json!([{"id":"a","name":"First","cells":{
        "A1":"2","B1":"=INDIRECT(\"A1\")","B2":"=OFFSET(A1,1,0)",
        "B3":"=SUM(A1:INDEX(A1:A5,3))","B4":"=Missing!A1","B5":"=MissingName+1",
        "B6":"=SUM(A1#)","C1":"=A1+A1","D1":"'=A1","E1":"=1/0"
    }}]));
    assert!(c.error.is_none());
    let q = c.audit("a", "A1", 0, 100).unwrap();
    assert_eq!(q["unresolvedFormulaCount"], 6);
    assert!(!ranges(&q, "dependents").contains(&"First!B1".into()));
    assert!(!ranges(&q, "dependents").contains(&"First!D1".into()));
    assert_eq!(c.audit("a", "C1", 0, 100).unwrap()["precedentCount"], 1);
    assert!(c.audit("a", "B1", 0, 100).unwrap()["warnings"][0]
        .as_str()
        .unwrap()
        .contains("INDIRECT"));
    assert_eq!(c.audit("a", "E1", 0, 100).unwrap()["isError"], true);
}

#[test]
fn trace_pages_are_stable_sparse_and_include_self_references() {
    let mut c = WorkbookCalculator::default();
    c.sync(&json!([{"id":"a","name":"First","cells":{"A1":"=A1+SUM(B1:B3)","B1":"=A1","B2":"=A1","B3":"=A1","B4":"=A1"}}]));
    let first = c.audit("a", "A1", 0, 2).unwrap();
    assert_eq!(first["dependentCount"], 5);
    assert_eq!(ranges(&first, "dependents"), vec!["First!A1", "First!B1"]);
    assert_eq!(
        ranges(&c.audit("a", "A1", 2, 2).unwrap(), "dependents"),
        vec!["First!B2", "First!B3"]
    );
    assert_eq!(
        ranges(&c.audit("a", "A1", 4, 2).unwrap(), "dependents"),
        vec!["First!B4"]
    );
    assert_eq!(c.audit("a", "A1", 6, 2).unwrap()["dependents"], json!([]));
    assert!(c.audit("missing", "A1", 0, 100).is_err());
    assert!(c.audit("a", "XFE1", 0, 100).is_err());
}

#[test]
fn audit_is_read_only_and_invalidates_after_edit_undo_redo_and_new_workbook() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    cells(&mut c, json!({"A1":"4","B1":"=A1*2"}));
    let before = c.snapshot();
    assert_eq!(query(&mut c, "A1")["dependentCount"], 1);
    assert_eq!(query(&mut c, "B1")["value"], 8.0);
    assert_eq!(c.snapshot(), before);
    cells(&mut c, json!({"B1":"=C1+1"}));
    assert_eq!(query(&mut c, "A1")["dependentCount"], 0);
    assert_eq!(query(&mut c, "C1")["dependentCount"], 1);
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(query(&mut c, "A1")["dependentCount"], 1);
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(query(&mut c, "A1")["dependentCount"], 0);
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    assert_eq!(query(&mut c, "C1")["dependentCount"], 0);
}

#[test]
fn audit_rejects_stale_identity_revision_calculation_and_invalid_paging() {
    let mut c = Core::new();
    let s = c.snapshot();
    let r = json!({"op":"formulaAudit","workspaceId":c.workspace["id"],"baseRevision":c.workspace["revision"],"calculationGeneration":s["evaluatedCells"],"sheetId":c.workspace["sheets"][0]["id"],"address":"A1"});
    for (key, value) in [
        ("workspaceId", json!("other")),
        ("baseRevision", json!(99999)),
        ("calculationGeneration", json!(99999)),
        ("limit", json!(501)),
        ("limit", json!(0)),
        ("offset", json!(-1)),
    ] {
        let mut bad = r.clone();
        bad[key] = value;
        assert!(c.dispatch(bad).is_err());
    }
    assert_eq!(c.snapshot(), s);
    c.dispatch(json!({"op":"recalculate"})).unwrap();
    assert!(c.dispatch(r).unwrap_err().contains("workbook changed"));
}
