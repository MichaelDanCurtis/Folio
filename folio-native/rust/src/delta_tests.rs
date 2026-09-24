//! Delta responses applied to the caller's state must equal full snapshots.
use crate::workspace::Core;
use serde_json::{json, Value};

/// Reference applier: what the Flutter client does with a `{"delta": …}`.
fn apply(base: &Value, response: &Value) -> Value {
    let d = &response["delta"];
    // Keys a full snapshot always has; anything else belonged to one response.
    const STANDARD: [&str; 16] = [
        "blankWorkspace",
        "fileStates",
        "workspace",
        "calculated",
        "formatted",
        "conditionalWarnings",
        "date1904",
        "calculationError",
        "canUndo",
        "canRedo",
        "evaluatedCells",
        "localCalendarDay",
        "localUtcOffset",
        "rows",
        "columns",
        "epoch",
    ];
    let mut next = json!({});
    for k in STANDARD {
        next[k] = base[k].clone();
    }
    for op in d["workspace"].as_array().unwrap() {
        let path = op["path"].as_array().unwrap();
        let (last, head) = path.split_last().unwrap();
        let mut node = &mut next["workspace"];
        for k in head {
            node = match k {
                Value::String(f) => node.get_mut(f.as_str()).unwrap(),
                Value::Number(i) => node.get_mut(i.as_u64().unwrap() as usize).unwrap(),
                _ => unreachable!(),
            };
        }
        match (last, op.get("value")) {
            (Value::String(f), Some(v)) => {
                node.as_object_mut().unwrap().insert(f.clone(), v.clone());
            }
            (Value::String(f), None) => {
                node.as_object_mut().unwrap().remove(f);
            }
            (Value::Number(i), Some(v)) => node[i.as_u64().unwrap() as usize] = v.clone(),
            _ => unreachable!(),
        }
    }
    for key in ["calculated", "formatted"] {
        for (id, cells) in d[key].as_object().unwrap() {
            let sheet = next[key][id].as_object_mut().unwrap();
            for (a, v) in cells.as_object().unwrap() {
                if v.is_null() {
                    sheet.remove(a);
                } else {
                    sheet.insert(a.clone(), v.clone());
                }
            }
        }
    }
    for (k, v) in d["set"].as_object().unwrap() {
        next[k] = v.clone();
    }
    for (k, v) in response.as_object().unwrap() {
        if k != "delta" {
            next[k] = v.clone();
        }
    }
    next
}

/// Identity and digests aside, the published content.
fn content(v: &Value) -> Value {
    let mut v = v.clone();
    for k in ["epoch", "fileStates", "layoutToken"] {
        v.as_object_mut().unwrap().remove(k);
    }
    v
}

/// The first path where two values differ, for readable failures.
fn first_difference(a: &Value, b: &Value, path: String) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => x.keys().chain(y.keys()).find_map(|k| {
            first_difference(
                &x.get(k).cloned().unwrap_or(Value::Null),
                &y.get(k).cloned().unwrap_or(Value::Null),
                format!("{path}/{k}"),
            )
        }),
        _ if a != b => Some(format!("{path}: {a} vs {b}")),
        _ => None,
    }
}

fn check(got: &Result<Value, String>, expected: &Result<Value, String>, what: String) {
    if let (Ok(g), Ok(e)) = (got, expected) {
        if let Some(d) = first_difference(&content(g), &content(e), String::new()) {
            panic!("{what}: {d}");
        }
    }
    assert_eq!(
        got.as_ref().map(content),
        expected.as_ref().map(content),
        "{what}"
    );
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

struct Client {
    core: Core,
    state: Value,
    deltas: usize,
}
impl Client {
    /// Sends `request` asking for a delta against the state it holds.
    fn send(&mut self, mut request: Value) -> Result<Value, String> {
        request["delta"] = self.state["epoch"].clone();
        let response = self.core.dispatch(request)?;
        Ok(if response.get("delta").is_some() {
            self.deltas += 1;
            assert_eq!(response["delta"]["base"], self.state["epoch"]);
            apply(&self.state, &response)
        } else {
            response
        })
    }
}

#[test]
fn delta_responses_reproduce_full_snapshots() {
    let mut seed = Core::new();
    seed.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = seed.workspace.clone();
    let mut a = serde_json::Map::new();
    for r in 1..=15 {
        a.insert(format!("A{r}"), json!(r.to_string()));
        a.insert(format!("B{r}"), json!(format!("=A{r}*2")));
    }
    w["sheets"] = json!([
        {"id":"a","name":"A","cells":a,
         "conditionalFormats":[{"sqref":"A1:B15","type":"cellIs","operator":"greaterThan","formulas":["12"],"priority":1,"style":{"bold":true}}]},
        {"id":"b","name":"B","cells":{"A1":"='A'!B2"}}
    ]);
    let mut engines = [Core::new(), Core::new()];
    for c in &mut engines {
        c.freeze_clock("2025-03-04T05:06:07Z");
        c.workspace = w.clone();
        c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    }
    let [mut delta_core, mut full] = engines;
    let state = delta_core.snapshot_for_test();
    let mut client = Client {
        core: delta_core,
        state,
        deltas: 0,
    };
    let values = [
        "",
        "3",
        "14",
        "=A1+A2",
        "=1/0",
        "long text that wraps across the cell",
        "=TODAY()",
    ];
    let mut r = Lcg(33);
    for step in 0..300 {
        let sheet = ["a", "b"][r.below(2) as usize];
        let address = format!("{}{}", ["A", "B", "C"][r.below(3) as usize], 1 + r.below(8));
        let value = values[r.below(values.len() as u64) as usize];
        let cells =
            json!({"op":"cells","cells":[{"sheetId":sheet,"address":address,"value":value}]});
        let (request, layout) = match r.below(9) {
            0 => (json!({"op":"undo"}), false),
            1 => (json!({"op":"redo"}), false),
            2 => (
                json!({"op":"formatCells","sheetId":sheet,"addresses":[address],"style":{"numFmt":"0.0"}}),
                false,
            ),
            3 => (json!({"op":"refreshCalendar"}), false),
            4 | 5 => (cells, true),
            _ => (cells, false),
        };
        if layout {
            let prepared_full =
                full.dispatch(json!({"op":"prepareLayoutEdit","edit":request.clone()}));
            let prepared = client.send(json!({"op":"prepareLayoutEdit","edit":request}));
            check(&prepared, &prepared_full, format!("step {step}: prepare"));
            let Ok(prepared) = prepared else { continue };
            let mut sizes = json!({});
            for (id, rows) in prepared["layoutRows"].as_object().unwrap() {
                sizes[id] = json!({});
                for row in rows.as_array().unwrap() {
                    sizes[id][row.to_string()] = json!(20 + row.as_u64().unwrap());
                }
            }
            let token = client.core.layout_token();
            let full_token = full.layout_token();
            let expected =
                full.dispatch(json!({"op":"commitLayoutEdit","token":full_token,"sizes":sizes}));
            let got = client.send(json!({"op":"commitLayoutEdit","token":token,"sizes":sizes}));
            check(&got, &expected, format!("step {step}: commit"));
            if let Ok(state) = got {
                client.state = state;
            }
        } else {
            let expected = full.dispatch(request.clone());
            let got = client.send(request.clone());
            check(&got, &expected, format!("step {step}: {request}"));
            if let Ok(state) = got {
                client.state = state;
            }
        }
        if r.below(10) == 0 {
            // A client that fell behind gets a full snapshot, never a wrong delta.
            client.state["epoch"] = json!(0);
        }
    }
    assert!(client.deltas > 150, "only {} deltas", client.deltas);
}

#[test]
fn file_states_op_matches_the_snapshot_and_follows_edits() {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let full = c.dispatch(json!({"op":"get"})).unwrap();
    let states = c.dispatch(json!({"op":"fileStates"})).unwrap();
    assert_eq!(states["fileStates"], full["fileStates"]);
    assert_eq!(states["epoch"], full["epoch"]);
    let id = c.workspace["sheets"][0]["id"].clone();
    let edited = c
        .dispatch(json!({"op":"cells","delta":full["epoch"],"cells":[{"sheetId":id,"address":"A1","value":"7"}]}))
        .unwrap();
    assert!(edited.get("delta").is_some() && edited["delta"]["set"].get("fileStates").is_none());
    let after = c.dispatch(json!({"op":"fileStates"})).unwrap();
    assert_eq!(after["epoch"], edited["delta"]["set"]["epoch"]);
    assert_ne!(after["fileStates"]["xlsx"], states["fileStates"]["xlsx"]);
    assert_eq!(
        after["fileStates"],
        c.dispatch(json!({"op":"get"})).unwrap()["fileStates"]
    );
    c.dispatch(json!({"op":"undo"})).unwrap();
    assert_eq!(
        c.dispatch(json!({"op":"fileStates"})).unwrap()["fileStates"],
        states["fileStates"]
    );
}
