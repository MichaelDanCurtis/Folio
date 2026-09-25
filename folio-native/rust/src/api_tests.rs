use crate::api::dispatch_on;
use crate::workspace::Core;
use serde_json::{json, Value};
use std::sync::Mutex;

fn call(core: &Mutex<Core>, request: Value) -> Value {
    serde_json::from_str(&dispatch_on(core, &request.to_string())).unwrap()
}

#[test]
fn a_panicking_request_returns_an_error_and_the_engine_keeps_working() {
    let core = Mutex::new(Core::new());
    call(&core, json!({"op":"new","blank":true}));
    let sheet = call(&core, json!({"op":"get"}))["value"]["workspace"]["sheets"][0]["id"].clone();
    let edited = call(
        &core,
        json!({"op":"cells","cells":[{"sheetId":sheet,"address":"A1","value":"=2*3"}]}),
    );
    assert_eq!(
        edited["value"]["calculated"][sheet.as_str().unwrap()]["A1"],
        6.0
    );
    let before = call(&core, json!({"op":"get"}))["value"]["workspace"].clone();

    let failed = call(&core, json!({"op":"panicForTest"}));
    assert_eq!(failed["ok"], false);
    assert!(failed["error"].as_str().unwrap().contains("internal error"));

    // The lock isn't poisoned for later calls, and recovery re-synced the engine.
    let after = call(&core, json!({"op":"get"}));
    assert_eq!(after["ok"], true);
    assert_eq!(
        after["value"]["calculated"][sheet.as_str().unwrap()]["A1"],
        6.0
    );
    // `panicForTest` mutated the title in place before panicking; everything
    // else is as it was.
    let mut expected = before;
    expected["title"] = after["value"]["workspace"]["title"].clone();
    assert_eq!(after["value"]["workspace"], expected);
    let edited = call(
        &core,
        json!({"op":"cells","cells":[{"sheetId":sheet,"address":"A2","value":"=A1+1"}]}),
    );
    assert_eq!(
        edited["value"]["calculated"][sheet.as_str().unwrap()]["A2"],
        7.0
    );
    let undone = call(&core, json!({"op":"undo"}));
    assert_eq!(undone["ok"], true);
    assert!(undone["value"]["calculated"][sheet.as_str().unwrap()]
        .get("A2")
        .is_none());
}

#[test]
fn malformed_requests_are_errors() {
    let core = Mutex::new(Core::new());
    let r: Value = serde_json::from_str(&dispatch_on(&core, "{not json")).unwrap();
    assert_eq!(r["ok"], false);
    assert_eq!(call(&core, json!({"op":"noSuchOp"}))["ok"], false);
    assert_eq!(call(&core, json!({"op":"get"}))["ok"], true);
}
