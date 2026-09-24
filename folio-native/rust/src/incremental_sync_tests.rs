//! Core::sync's incremental calculator, conditional-format and display paths
//! must publish exactly what a freshly loaded workspace does.
use crate::workspace::Core;
use serde_json::{json, Map, Value};

fn workspace(c: &Core) -> Value {
    let mut w = c.workspace.clone();
    let mut plain = Map::new();
    let mut ruled = Map::new();
    for r in 1..=12 {
        plain.insert(format!("A{r}"), json!(r.to_string()));
        plain.insert(format!("B{r}"), json!(format!("=A{r}/4")));
        ruled.insert(format!("A{r}"), json!((r * 3 % 7).to_string()));
        ruled.insert(format!("B{r}"), json!(format!("='Plain'!B{r}+A{r}")));
    }
    ruled.insert("C1".into(), json!("=1/0"));
    ruled.insert("C2".into(), json!("=TODAY()"));
    w["sheets"] = json!([
        {"id":"p","name":"Plain","cells":plain,
         "cellStyles":{"B1":{"numFmt":"0.00%"},"B2":{"numFmt":"$#,##0.00"},"C2":{"numFmt":"0.0"}}},
        {"id":"r","name":"Ruled","cells":ruled,
         "cellStyles":{"C2":{"numFmt":"yyyy-mm-dd"}},
         "conditionalFormats":[
            {"sqref":"A1:A12","type":"cellIs","operator":"greaterThan","formulas":["3"],"priority":1,"style":{"fill":"#FF0000","bold":true}},
            {"sqref":"B1:B12","type":"top10","rank":2,"priority":2,"style":{"italic":true}},
            {"sqref":"A1:B12","type":"colorScale","priority":3,"style":{},"scale":[{"type":"min","color":"#FFFFFF"},{"type":"max","color":"#000000"}]}
         ]}
    ]);
    w
}

fn published(c: &mut Core) -> Value {
    let s = c.snapshot();
    json!({"calculated":s["calculated"],"formatted":s["formatted"],
           "conditionalWarnings":s["conditionalWarnings"],"calculationError":s["calculationError"]})
}

fn fresh(w: &Value) -> Value {
    let mut c = Core::new();
    c.freeze_clock("2025-03-04T05:06:07Z");
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    published(&mut c)
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
fn cell_edits_undo_and_redo_publish_what_a_fresh_load_does() {
    let mut c = Core::new();
    c.freeze_clock("2025-03-04T05:06:07Z");
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let w = workspace(&c);
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    let values = [
        "0",
        "9",
        "-4",
        "",
        "=1/0",
        "=A1*3",
        "text",
        "=B2",
        "1e3",
        "=Plain!A3",
    ];
    let mut r = Lcg(5);
    for step in 0..120 {
        match r.below(6) {
            0 => {
                c.dispatch(json!({"op":"undo"})).unwrap();
            }
            1 => {
                c.dispatch(json!({"op":"redo"})).unwrap();
            }
            _ => {
                let sheet = ["p", "r"][r.below(2) as usize];
                let address = format!(
                    "{}{}",
                    ["A", "B", "C"][r.below(3) as usize],
                    1 + r.below(13)
                );
                let value = values[r.below(values.len() as u64) as usize];
                c.dispatch(json!({"op":"cells","cells":[{"sheetId":sheet,"address":address,"value":value}]}))
                    .unwrap();
            }
        }
        let expected = fresh(&c.workspace);
        assert_eq!(published(&mut c), expected, "step {step}");
    }
}
