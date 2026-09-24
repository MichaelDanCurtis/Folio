//! Staged layout edits must publish exactly what the original staging did.
use crate::workspace::Core;
use serde_json::{json, Value};

fn pair() -> (Core, Core) {
    let mut seed = Core::new();
    seed.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = seed.workspace.clone();
    let mut a = serde_json::Map::new();
    for r in 1..=20 {
        a.insert(format!("A{r}"), json!(r.to_string()));
        a.insert(format!("B{r}"), json!(format!("=A{r}*3")));
    }
    a.insert("C1".into(), json!("=TODAY()"));
    a.insert("C2".into(), json!("2025-03-09"));
    w["sheets"] = json!([
        {"id":"a","name":"A","cells":a,"cellStyles":{"B2":{"numFmt":"0.00"}},
         "conditionalFormats":[{"sqref":"A1:A20","type":"cellIs","operator":"greaterThan","formulas":["10"],"priority":1,"style":{"bold":true}},
            {"sqref":"C1:C9","type":"timePeriod","timePeriod":"today","priority":2,"style":{"fill":"#ABCDEF"}}]},
        {"id":"b","name":"B","cells":{"A1":"='A'!B1+1"},"rowHeights":{"4":30},"rowHeightModes":{"4":"manual"}}
    ]);
    let [mut fast, mut slow] = [Core::new(), Core::new()];
    for c in [&mut fast, &mut slow] {
        c.freeze_clock("2025-03-04T05:06:07Z");
        c.workspace = w.clone();
        c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    }
    slow.slow_layout = true;
    (fast, slow)
}

fn outcome(r: Result<Value, String>) -> Result<Value, String> {
    r.map(|mut v| {
        // The token is a per-engine serial, not content.
        v.as_object_mut().map(|o| o.remove("layoutToken"));
        v
    })
}

fn same(fast: &mut Core, slow: &mut Core, request: Value, step: usize) -> Result<Value, String> {
    let a = outcome(fast.dispatch(request.clone()));
    let b = outcome(slow.dispatch(request.clone()));
    assert_eq!(a, b, "step {step}: {request}");
    assert_eq!(fast.workspace, slow.workspace, "step {step}: {request}");
    a
}

fn sizes(prepared: &Value, height: u64) -> Value {
    let mut sizes = json!({});
    for (id, rows) in prepared["layoutRows"].as_object().unwrap() {
        sizes[id] = json!({});
        for row in rows.as_array().unwrap() {
            sizes[id][row.to_string()] = json!(height + row.as_u64().unwrap() % 5);
        }
    }
    sizes
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
fn random_layout_edits_match_the_original_staging() {
    let (mut fast, mut slow) = pair();
    let values = [
        "",
        "12",
        "a longer line of text that wraps",
        "=1/0",
        "=A1+A2",
        "5",
        "=TODAY()+1",
    ];
    let mut r = Lcg(21);
    let mut committed = 0;
    for step in 0..250 {
        let sheet = ["a", "b"][r.below(2) as usize];
        let address = format!("{}{}", ["A", "B", "C"][r.below(3) as usize], 1 + r.below(6));
        let value = values[r.below(values.len() as u64) as usize];
        let edit = match r.below(8) {
            0 => {
                json!({"op":"formatCells","sheetId":sheet,"addresses":[address],"style":{"numFmt":"0.0"}})
            }
            1 => json!({"op":"enterCell","sheetId":sheet,"address":address,"value":value,
                        "baseRevision":fast.workspace["revision"]}),
            2 => json!({"op":"refreshCalendar"}),
            _ => json!({"op":"cells","cells":[{"sheetId":sheet,"address":address,"value":value}]}),
        };
        if edit["op"] == "refreshCalendar" {
            let day = format!("2025-03-{:02}T05:06:07Z", 5 + r.below(20));
            fast.freeze_clock(&day);
            slow.freeze_clock(&day);
        }
        let Ok(prepared) = same(
            &mut fast,
            &mut slow,
            json!({"op":"prepareLayoutEdit","edit":edit}),
            step,
        ) else {
            continue;
        };
        let token_fast = fast_token(&mut fast);
        let token_slow = fast_token(&mut slow);
        match r.below(10) {
            0 => {
                fast.dispatch(json!({"op":"cancelLayoutEdit","token":token_fast}))
                    .unwrap();
                slow.dispatch(json!({"op":"cancelLayoutEdit","token":token_slow}))
                    .unwrap();
            }
            1 => {
                // A write in between expires the measured edit.
                let _ = same(&mut fast, &mut slow, json!({"op":"undo"}), step);
                let s = sizes(&prepared, 20);
                let a =
                    fast.dispatch(json!({"op":"commitLayoutEdit","token":token_fast,"sizes":s}));
                let b =
                    slow.dispatch(json!({"op":"commitLayoutEdit","token":token_slow,"sizes":s}));
                assert_eq!(a, b, "step {step}: expired commit");
            }
            _ => {
                let s = sizes(&prepared, 18 + r.below(30));
                let a = outcome(
                    fast.dispatch(json!({"op":"commitLayoutEdit","token":token_fast,"sizes":s})),
                );
                let b = outcome(
                    slow.dispatch(json!({"op":"commitLayoutEdit","token":token_slow,"sizes":s})),
                );
                assert_eq!(a, b, "step {step}: commit");
                assert_eq!(
                    fast.workspace, slow.workspace,
                    "step {step}: commit workspace"
                );
                committed += a.is_ok() as usize;
            }
        }
        if r.below(6) == 0 {
            let op = ["undo", "redo"][r.below(2) as usize];
            let _ = same(&mut fast, &mut slow, json!({ "op": op }), step);
        }
        if r.below(7) == 0 {
            let _ = same(
                &mut fast,
                &mut slow,
                json!({"op":"cells","cells":[{"sheetId":"a","address":"A3","value":r.below(40).to_string()}]}),
                step,
            );
        }
    }
    assert!(committed > 100, "only {committed} layout edits committed");
}

/// The pending layout token of the last prepare.
fn fast_token(c: &mut Core) -> Value {
    c.layout_token()
}
