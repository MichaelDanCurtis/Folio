//! Edit-path performance baseline. The smoke test runs in the normal suite so
//! the harness keeps compiling; the full baseline is opt-in:
//! `cargo test --release perf_baseline -- --ignored --nocapture`.
use crate::workspace::Core;
use serde_json::{json, Value};
use std::time::Instant;

/// `sheets` sheets, each with numbers in A1:A{rows} and `=A{r}*2` in B1:B{rows}.
fn workbook(sheets: usize, rows: usize) -> Value {
    let list: Vec<Value> = (1..=sheets)
        .map(|s| {
            let mut cells = serde_json::Map::new();
            for r in 1..=rows {
                cells.insert(format!("A{r}"), json!(r.to_string()));
                cells.insert(format!("B{r}"), json!(format!("=A{r}*2")));
            }
            json!({"id": format!("s{s}"), "name": format!("Sheet {s}"), "cells": cells})
        })
        .collect();
    Value::Array(list)
}

fn loaded(sheets: usize, rows: usize) -> (Core, f64) {
    let mut c = Core::new();
    c.dispatch(json!({"op":"new","blank":true})).unwrap();
    let mut w = c.workspace.clone();
    w["sheets"] = workbook(sheets, rows);
    let t = Instant::now();
    c.dispatch(json!({"op":"replace","workspace":w})).unwrap();
    (c, t.elapsed().as_secs_f64() * 1000.0)
}

struct Samples {
    edit_ms: Vec<f64>,
    serialize_ms: Vec<f64>,
    response_bytes: usize,
    last: Value,
}

/// Edits s1!A1 `edits` times. Each edit is timed through the engine commit and,
/// separately, through the JSON serialization `api::dispatch` performs.
fn measure_edits(c: &mut Core, edits: usize) -> Samples {
    let mut s = Samples {
        edit_ms: vec![],
        serialize_ms: vec![],
        response_bytes: 0,
        last: Value::Null,
    };
    for i in 0..edits {
        let t = Instant::now();
        let v = c
            .dispatch(json!({"op":"cells","cells":[{"sheetId":"s1","address":"A1","value":(i + 1).to_string()}]}))
            .unwrap();
        s.edit_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        let text = v.to_string();
        s.serialize_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        s.response_bytes = text.len();
        s.last = v;
    }
    s
}

fn percentile(values: &[f64], p: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

#[test]
fn perf_harness_smoke() {
    let (mut c, _) = loaded(2, 10);
    let s = measure_edits(&mut c, 5);
    assert_eq!(s.edit_ms.len(), 5);
    assert_eq!(s.last["calculated"]["s1"]["B1"], 10.0);
    assert!(s.response_bytes > 0);
    assert!(percentile(&s.edit_ms, 0.5) <= percentile(&s.edit_ms, 0.95));
}

#[test]
#[ignore = "baseline measurement; run with --release -- --ignored"]
fn perf_baseline() {
    let (sheets, rows, edits) = (20, 2_500, 200);
    let (mut c, load_ms) = loaded(sheets, rows);
    let s = measure_edits(&mut c, edits);
    let report = json!({
        "workload": {"sheets": sheets, "populatedCells": sheets * rows * 2, "edits": edits},
        "loadMs": load_ms,
        "editMs": {"p50": percentile(&s.edit_ms, 0.5), "p95": percentile(&s.edit_ms, 0.95),
                   "first10Avg": s.edit_ms[..10].iter().sum::<f64>() / 10.0,
                   "last10Avg": s.edit_ms[edits - 10..].iter().sum::<f64>() / 10.0},
        "serializeMs": {"p50": percentile(&s.serialize_ms, 0.5), "p95": percentile(&s.serialize_ms, 0.95)},
        "responseBytes": s.response_bytes,
    });
    let out = std::env::var("FOLIO_PERF_OUT").unwrap_or_else(|_| {
        std::env::temp_dir()
            .join("folio-perf-baseline.json")
            .to_string_lossy()
            .into_owned()
    });
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!(
        "{}\nwritten to {out}",
        serde_json::to_string_pretty(&report).unwrap()
    );
}
