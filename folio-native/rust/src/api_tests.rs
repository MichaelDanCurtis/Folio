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

#[test]
fn sessions_are_independent_engines_and_close_cleanly() {
    let call = |request: serde_json::Value| -> serde_json::Value {
        serde_json::from_str(&crate::api::dispatch(request.to_string())).unwrap()
    };
    let a = format!("test-a-{}", std::process::id());
    let b = format!("test-b-{}", std::process::id());
    call(json!({"session":a,"op":"new","blank":true}));
    call(json!({"session":b,"op":"new","blank":true}));
    call(json!({"session":a,"op":"title","title":"First"}));
    call(json!({"session":b,"op":"title","title":"Second"}));
    let title =
        |s: &str| call(json!({"session":s,"op":"get"}))["value"]["workspace"]["title"].clone();
    assert_eq!(title(&a), "First");
    assert_eq!(title(&b), "Second");
    // Closing a session discards it; its name starts afresh.
    assert_eq!(
        call(json!({"session":a,"op":"closeSession"}))["value"]["closed"],
        true
    );
    call(json!({"session":a,"op":"new","blank":true}));
    assert_eq!(title(&a), "Untitled workspace");
    assert_eq!(title(&b), "Second");
    call(json!({"session":a,"op":"closeSession"}));
    call(json!({"session":b,"op":"closeSession"}));
}
