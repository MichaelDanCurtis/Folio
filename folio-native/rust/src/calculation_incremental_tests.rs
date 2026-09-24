//! The incremental calculator must match a freshly built one after every edit.
use crate::calculation::{Changed, WorkbookCalculator};
use crate::calculation_clock::CalculationClock;
use serde_json::{json, Map, Value};

fn workbook() -> Value {
    let sheets: Vec<Value> = (1..=3)
        .map(|s| {
            let mut cells = Map::new();
            for r in 1..=60 {
                cells.insert(format!("A{r}"), json!(r.to_string()));
                cells.insert(format!("B{r}"), json!(format!("=A{r}*2")));
            }
            json!({"id": format!("s{s}"), "name": format!("Sheet {s}"), "cells": cells})
        })
        .collect();
    let mut w = Value::Array(sheets);
    let extra = [
        (1, "C1", "='Sheet 1'!A1+1"),
        (1, "C2", "=SUM('Sheet 1'!B1:B60)"),
        (0, "D1", "=SEQUENCE(3)"),
        (0, "E1", "=1/0"),
        (0, "E2", "=IFERROR(E1,\"fixed\")"),
        (0, "F1", "plain text"),
        (2, "G1", "=NOW()"),
        (2, "G2", "=TODAY()+A1"),
    ];
    for (s, a, v) in extra {
        w[s]["cells"][a] = json!(v);
    }
    w
}

fn calculator() -> WorkbookCalculator {
    let mut c = WorkbookCalculator::default();
    c.clock_override = Some(CalculationClock::fixed("2025-03-04T05:06:07Z"));
    c
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
fn incremental_updates_match_a_fresh_calculator() {
    let mut sheets = workbook();
    let mut incremental = calculator();
    incremental.sync(&sheets);
    assert_eq!(incremental.changed, Changed::All);
    let mut r = Lcg(11);
    let inputs = [
        "5",
        "-2.5",
        "",
        "=A1+A2",
        "=1/0",
        "=D2*10",
        "text",
        "=SEQUENCE(2)",
        "=E1",
        "=NOW()",
        "TRUE",
        "#N/A",
    ];
    for step in 0..200 {
        let before_values = incremental.values.clone();
        let before_errors = incremental.cell_errors.clone();
        let sheet = r.below(3) as usize;
        let address = format!(
            "{}{}",
            ["A", "B", "C", "D", "E", "H"][r.below(6) as usize],
            1 + r.below(8)
        );
        if r.below(5) == 0 {
            sheets[sheet]["cells"]
                .as_object_mut()
                .unwrap()
                .remove(&address);
        } else {
            sheets[sheet]["cells"][&address] = json!(inputs[r.below(inputs.len() as u64) as usize]);
        }
        let hint = [(sheet, address.clone())];
        if step % 3 == 0 {
            incremental.sync_changes(&sheets, false, None);
        } else {
            incremental.sync_changes(&sheets, false, Some(&hint));
        }
        let mut fresh = calculator();
        fresh.sync(&sheets);
        assert_eq!(incremental.error, fresh.error, "step {step}: {address}");
        assert_eq!(incremental.values, fresh.values, "step {step}: {address}");
        assert_eq!(
            incremental.cell_errors, fresh.cell_errors,
            "step {step}: {address}"
        );
        let Changed::Cells(changed) = &incremental.changed else {
            panic!("step {step}: a cell edit must not rebuild");
        };
        for (i, s) in sheets.as_array().unwrap().iter().enumerate() {
            let id = s["id"].as_str().unwrap();
            let addresses = before_values[id]
                .as_object()
                .unwrap()
                .keys()
                .chain(incremental.values[id].as_object().unwrap().keys());
            for a in addresses {
                let differs = before_values[id].get(a) != incremental.values[id].get(a)
                    || before_errors[id].get(a) != incremental.cell_errors[id].get(a);
                if differs {
                    assert!(
                        changed.contains(&(i, a.clone())),
                        "step {step}: {id}!{a} changed but was not reported"
                    );
                }
            }
        }
    }
}

#[test]
fn clock_day_change_recalculates_volatile_cells_without_input_changes() {
    let sheets = workbook();
    let mut c = calculator();
    c.sync(&sheets);
    let today = c.values["s3"]["G2"].clone();
    c.clock_override = Some(CalculationClock::fixed("2025-03-05T05:06:07Z"));
    c.sync_changes(&sheets, false, Some(&[]));
    assert_ne!(c.values["s3"]["G2"], today);
    assert!(matches!(&c.changed, Changed::Cells(cells) if cells.contains(&(2, "G2".into()))));
}
