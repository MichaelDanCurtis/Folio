//! Dynamic-array spills in the grid (X09.02) and the Name Manager (X09.04,
//! X09.05).
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

#[test]
fn spilled_values_reach_the_grid_and_shrink_and_collide() {
    let (mut c, s) = core();
    set(&mut c, &s, "A1", "=SEQUENCE(3)");
    assert_eq!(value(&mut c, &s, "A1"), json!(1.0));
    assert_eq!(value(&mut c, &s, "A2"), json!(2.0));
    assert_eq!(value(&mut c, &s, "A3"), json!(3.0));
    assert_eq!(c.snapshot()["spills"][&s]["A1"], "A1:A3");
    // Cells that read a spilled value see it.
    set(&mut c, &s, "C1", "=SUM(A1#)");
    assert_eq!(value(&mut c, &s, "C1"), json!(6.0));

    // A smaller spill clears the cells it no longer fills.
    set(&mut c, &s, "A1", "=SEQUENCE(2)");
    assert_eq!(value(&mut c, &s, "A2"), json!(2.0));
    assert_eq!(value(&mut c, &s, "A3"), Value::Null);
    assert_eq!(c.snapshot()["spills"][&s]["A1"], "A1:A2");

    // A value in the way is #SPILL!, and the spill is gone.
    set(&mut c, &s, "A2", "typed");
    assert_eq!(value(&mut c, &s, "A2"), json!("typed"));
    assert_eq!(c.snapshot()["formatted"][&s]["A1"]["text"], "#SPILL!");
    assert!(c.snapshot()["spills"][&s].get("A1").is_none());
    // Clearing it spills again.
    set(&mut c, &s, "A2", "");
    assert_eq!(value(&mut c, &s, "A2"), json!(2.0));
    // Undo and redo keep the spill in step.
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(value(&mut c, &s, "A2"), json!("typed"));
    c.dispatch(json!({"op":"redo"})).unwrap();
    assert_eq!(value(&mut c, &s, "A2"), json!(2.0));
}

fn names(c: &mut Core) -> Vec<Value> {
    c.dispatch(json!({"op":"nameManager","action":"list"}))
        .unwrap()["names"]
        .as_array()
        .unwrap()
        .clone()
}

fn edit(c: &mut Core, request: Value) -> Result<Value, String> {
    let mut request = request;
    request["op"] = json!("nameManager");
    request["baseRevision"] = c.workspace["revision"].clone();
    c.dispatch(request)
}

#[test]
fn names_are_added_edited_deleted_and_used_in_formulas() {
    let (mut c, s) = core();
    set(&mut c, &s, "B2", "8");
    set(&mut c, &s, "A1", "=Rate*B2");
    assert_eq!(c.snapshot()["formatted"][&s]["A1"]["text"], "#NAME?");

    // A new workbook gets a package on its first name.
    edit(
        &mut c,
        json!({"action":"add","name":"Rate","refersTo":"=0.25","sheetId":null,"activeSheetId":s}),
    )
    .unwrap();
    assert_eq!(value(&mut c, &s, "A1"), json!(2.0));
    // An unqualified reference gets the active sheet, as in Excel.
    edit(
        &mut c,
        json!({"action":"add","name":"Base","refersTo":"=$B$2","sheetId":null,"activeSheetId":s}),
    )
    .unwrap();
    // A named LAMBDA.
    edit(
        &mut c,
        json!({"action":"add","name":"Twice","refersTo":"=LAMBDA(x,x*2)","sheetId":null}),
    )
    .unwrap();
    set(&mut c, &s, "A2", "=Twice(Base)");
    assert_eq!(value(&mut c, &s, "A2"), json!(16.0));

    let list = names(&mut c);
    let find = |n: &str| list.iter().find(|r| r["name"] == n).unwrap().clone();
    assert_eq!(find("Rate")["refersTo"], "=0.25");
    assert_eq!(find("Rate")["value"], json!(0.25));
    assert_eq!(find("Rate")["scope"], "Workbook");
    assert_eq!(find("Base")["refersTo"], "='Sheet 1'!$B$2");
    assert_eq!(find("Twice")["refersTo"], "=LAMBDA(x,x*2)");

    // Rejected: invalid names, cell-like names, duplicates, bad formulas.
    for (name, formula, message) in [
        ("two words", "=1", "isn't a valid name"),
        ("A1", "=1", "isn't a valid name"),
        ("rate", "=1", "already defined"),
        ("Good", "=SUM(", "isn't valid"),
    ] {
        let e = edit(
            &mut c,
            json!({"action":"add","name":name,"refersTo":formula,"sheetId":null}),
        )
        .unwrap_err();
        assert!(e.contains(message), "{name}: {e}");
    }

    // Edit: rename and change the formula; formulas using the old name
    // show #NAME?, as in Excel.
    edit(&mut c, json!({"action":"edit","oldName":"Rate","oldSheetId":null,"name":"Tax","refersTo":"=0.5","sheetId":null}))
        .unwrap();
    assert_eq!(c.snapshot()["formatted"][&s]["A1"]["text"], "#NAME?");
    set(&mut c, &s, "A1", "=Tax*B2");
    assert_eq!(value(&mut c, &s, "A1"), json!(4.0));

    // A sheet-scoped name wins over a workbook one on its sheet.
    edit(
        &mut c,
        json!({"action":"add","name":"Tax","refersTo":"=0.1","sheetId":s}),
    )
    .unwrap();
    assert_eq!(value(&mut c, &s, "A1"), json!(0.8));

    // Undo takes back the last change; delete removes the name.
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(value(&mut c, &s, "A1"), json!(4.0));
    edit(
        &mut c,
        json!({"action":"delete","oldName":"Tax","oldSheetId":null}),
    )
    .unwrap();
    assert_eq!(c.snapshot()["formatted"][&s]["A1"]["text"], "#NAME?");
    assert!(names(&mut c).iter().all(|r| r["name"] != "Tax"));

    // Names survive export and reimport.
    let path = std::env::temp_dir().join(format!("folio-names-ui-{}.xlsx", std::process::id()));
    c.dispatch(json!({"op":"exportOffice","format":"xlsx","path":path}))
        .unwrap();
    let mut again = Core::new();
    again
        .dispatch(json!({"op":"importOffice","path":path}))
        .unwrap();
    let s2 = again.workspace["sheets"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(value(&mut again, &s2, "A2"), json!(16.0));
    let reopened = names(&mut again);
    assert!(reopened
        .iter()
        .any(|r| r["name"] == "Twice" && r["refersTo"] == "=LAMBDA(x,x*2)"));
    std::fs::remove_file(&path).ok();

    // A stale revision is refused.
    let e = c
        .dispatch(json!({"op":"nameManager","action":"add","name":"Late","refersTo":"=1","baseRevision":-1}))
        .unwrap_err();
    assert!(e.contains("changed"));
}

#[test]
fn names_follow_row_insertion() {
    let (mut c, s) = core();
    set(&mut c, &s, "B2", "5");
    edit(
        &mut c,
        json!({"action":"add","name":"Cell","refersTo":"=$B$2","sheetId":null,"activeSheetId":s}),
    )
    .unwrap();
    set(&mut c, &s, "A1", "=Cell");
    c.dispatch(json!({"op":"structure","sheetId":s,"action":"insertRows","at":1,"count":1,"baseRevision":c.workspace["revision"]}))
        .unwrap();
    // Row 1 moved down: the formula is in A2 now, and still reads B3.
    assert_eq!(value(&mut c, &s, "A2"), json!(5.0));
    let list = names(&mut c);
    assert_eq!(list[0]["refersTo"], "='Sheet 1'!$B$3");
}
