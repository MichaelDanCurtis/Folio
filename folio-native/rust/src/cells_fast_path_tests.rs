//! In-place cell edits must be indistinguishable from the general path.
use crate::workspace::Core;
use serde_json::{json, Value};

fn pair() -> (Core, Core) {
    let mut fast = Core::new();
    fast.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = fast.workspace.clone();
    w["sheets"] = json!([
        {"id":"a","name":"A","cells":{"A1":"1","B1":"=A1*2"},"cellStyles":{"B1":{"numFmt":"0.0"}},
         "conditionalFormats":[{"sqref":"A1:A9","type":"cellIs","operator":"greaterThan","formulas":["4"],"priority":1,"style":{"bold":true}}]},
        {"id":"b","name":"B","cells":{}}
    ]);
    w["document"] = json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Notes"}]}]});
    load(w)
}

fn load(w: Value) -> (Core, Core) {
    let [mut fast, mut slow] = [Core::new(), Core::new()];
    for c in [&mut fast, &mut slow] {
        c.freeze_clock("2025-03-04T05:06:07Z");
        // `replace` is undoable; start both from the same workspace so undo
        // can't reach two different random seeds.
        c.workspace = w.clone();
        c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    }
    slow.slow_cells = true;
    (fast, slow)
}

/// Epochs are process-wide identifiers, different in every engine.
fn content(r: Result<Value, String>) -> Result<Value, String> {
    r.map(|mut v| {
        v.as_object_mut().map(|o| o.remove("epoch"));
        v
    })
}

fn same(fast: &mut Core, slow: &mut Core, request: Value, step: usize) {
    let a = content(fast.dispatch(request.clone()));
    let b = content(slow.dispatch(request.clone()));
    match (a, b) {
        (Ok(a), Ok(b)) => assert_eq!(a, b, "step {step}: {request}"),
        (Err(a), Err(b)) => assert_eq!(a, b, "step {step}: {request}"),
        (a, b) => panic!("step {step}: {request}: {a:?} vs {b:?}"),
    }
    assert_eq!(fast.workspace, slow.workspace, "step {step}");
}

struct Lcg(u64);
impl Lcg {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

#[test]
fn random_cell_edits_match_the_general_path() {
    let (mut fast, mut slow) = pair();
    let values = [
        "",
        "7",
        "=A1+B1",
        "=1/0",
        "text \"quoted\"",
        "é✓",
        "=SUM(A1:A9)",
        "3",
    ];
    let mut r = Lcg(3);
    for step in 0..300 {
        let request = match r.below(11) {
            10 => {
                let text = format!("Notes {}", r.below(3));
                json!({"op":"document","document":{"type":"doc","content":[
                    {"type":"paragraph","content":[{"type":"text","text":text}]}]}})
            }
            0 => json!({"op":"undo"}),
            1 => json!({"op":"redo"}),
            2 => json!({"op":"cells","cells":[{"sheetId":"a","address":"zz1","value":"x"}]}),
            3 => json!({"op":"cells","cells":[{"sheetId":"missing","address":"A1","value":"x"}]}),
            4 => {
                json!({"op":"cells","cells":[{"sheetId":"a","address":"A1","value":"x".repeat(10001)}]})
            }
            5 => json!({"op":"cells","cells":"not a list"}),
            _ => {
                let cells: Vec<Value> = (0..=r.below(3))
                    .map(|_| {
                        let sheet = ["a", "b"][r.below(2) as usize];
                        let column = ["a", "A", "B", "C"][r.below(4) as usize];
                        let address = format!("{column}{}", 1 + r.below(9));
                        let value = values[r.below(values.len() as u64) as usize];
                        json!({"sheetId":sheet,"address":address,"value":value})
                    })
                    .collect();
                json!({"op":"cells","cells":cells})
            }
        };
        same(&mut fast, &mut slow, request, step);
    }
}

#[test]
fn workbooks_with_tables_take_the_general_path() {
    let (mut c, _) = pair();
    c.dispatch(json!({"op":"cells","cells":[
        {"sheetId":"b","address":"A1","value":"Name"},{"sheetId":"b","address":"B1","value":"Qty"},
        {"sheetId":"b","address":"A2","value":"x"},{"sheetId":"b","address":"B2","value":"1"}]}))
        .unwrap();
    c.dispatch(json!({"op":"table","action":"create","sheetId":"b","range":"A1:B2"}))
        .unwrap();
    assert!(c.workspace["sheets"][1]["tables"].is_array());
    let (mut fast, mut slow) = load(c.workspace.clone());
    for (step, (address, value)) in [("A1", "Item"), ("A3", "y"), ("B3", "2")]
        .into_iter()
        .enumerate()
    {
        same(
            &mut fast,
            &mut slow,
            json!({"op":"cells","cells":[{"sheetId":"b","address":address,"value":value}]}),
            step,
        );
    }
}

#[test]
fn one_request_editing_a_cell_twice_undoes_to_the_original() {
    let (mut fast, mut slow) = pair();
    let requests = [
        json!({"op":"cells","cells":[
            {"sheetId":"a","address":"A1","value":"x"},{"sheetId":"a","address":"a1","value":"y"},
            {"sheetId":"b","address":"C3","value":"new"},{"sheetId":"b","address":"C3","value":""}]}),
        json!({"op":"undo"}),
        json!({"op":"redo"}),
        json!({"op":"undo"}),
    ];
    for (step, request) in requests.into_iter().enumerate() {
        same(&mut fast, &mut slow, request, step);
    }
    assert_eq!(fast.workspace["sheets"][0]["cells"]["A1"], "1");
    assert!(fast.workspace["sheets"][1]["cells"].get("C3").is_none());
}

#[test]
fn validated_entries_match_the_general_path() {
    let (mut c, _) = pair();
    for (range, rule, style) in [
        (
            "C1:C9",
            json!({"type":"whole","formula1":"1","formula2":"10"}),
            "stop",
        ),
        (
            "D1:D9",
            json!({"type":"list","formula1":"\"Red,Green\"","error":"Choose a colour"}),
            "warning",
        ),
        (
            "E1:E9",
            json!({"type":"custom","formula1":"E1>A1"}),
            "information",
        ),
    ] {
        let mut rule = rule;
        rule["allowBlank"] = json!(true);
        rule["showErrorMessage"] = json!(true);
        rule["errorStyle"] = json!(style);
        c.dispatch(
            json!({"op":"dataValidation","sheetId":"a","baseRevision":c.workspace["revision"],
                          "range":range,"action":"set","rule":rule}),
        )
        .unwrap();
    }
    let (mut fast, mut slow) = load(c.workspace.clone());
    let mut r = Lcg(9);
    let values = ["5", "50", "Red", "Blue", "", "=A1+1", "0"];
    for step in 0..120 {
        let op = ["enterCell", "validationEntryCheck"][r.below(2) as usize];
        let address = format!(
            "{}{}",
            ["C", "D", "E", "F"][r.below(4) as usize],
            1 + r.below(3)
        );
        let value = values[r.below(values.len() as u64) as usize];
        let revision = if r.below(15) == 0 {
            json!(0)
        } else {
            fast.workspace["revision"].clone()
        };
        let request = json!({"op":op,"sheetId":"a","address":address,"value":value,
                             "allowInvalid":r.below(2) == 0,"baseRevision":revision});
        same(&mut fast, &mut slow, request, step);
        if r.below(8) == 0 {
            same(&mut fast, &mut slow, json!({"op":"undo"}), step);
        }
    }
}

#[test]
fn a_document_that_normalization_changes_takes_the_general_path() {
    let (c, _) = pair();
    let mut w = c.workspace.clone();
    w["document"] = json!({"type":"doc","content":[{"type":"paragraph","attrs":{"paragraphFormat":{
        "styleId":"Heading1","inheritedOutlineLevel":0,"inherited":{},"direct":{"outlineLevel":9},
        "inheritedRun":{"sz":48}}},"content":[{"type":"text","text":"Outline"}]}]});
    let (mut fast, mut slow) = pair();
    // Cache the stable document first, then swap in one that isn't.
    same(
        &mut fast,
        &mut slow,
        json!({"op":"cells","cells":[{"sheetId":"a","address":"A2","value":"0"}]}),
        0,
    );
    same(
        &mut fast,
        &mut slow,
        json!({"op":"replace","workspace":w}),
        1,
    );
    let before = fast.workspace["document"].clone();
    for (step, value) in ["1", "2"].into_iter().enumerate() {
        same(
            &mut fast,
            &mut slow,
            json!({"op":"cells","cells":[{"sheetId":"a","address":"A1","value":value}]}),
            step,
        );
    }
    assert_ne!(fast.workspace["document"], before, "normalization ran");
}
