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
    total_ms: Vec<f64>,
    response_bytes: usize,
    last: Value,
    last_response_text: String,
}

/// Edits s1!A1 `edits` times. Each edit is timed through the engine commit
/// (`edit_ms`) and, separately, through exactly the JSON response build that
/// `api::dispatch` performs — `json!({"ok":true,"value":v}).to_string()`,
/// which deep-copies the returned snapshot while wrapping it (`serialize_ms`).
/// `total_ms` is the per-edit sum of the two, i.e. the same engine+response
/// cost a real `api::dispatch` call pays end to end. The value inspected by
/// callers (`last`) is cloned *before* the timed response build so the clone
/// used for inspection never counts toward `serialize_ms`.
fn measure_edits(c: &mut Core, edits: usize) -> Samples {
    let mut s = Samples {
        edit_ms: vec![],
        serialize_ms: vec![],
        total_ms: vec![],
        response_bytes: 0,
        last: Value::Null,
        last_response_text: String::new(),
    };
    for i in 0..edits {
        let t = Instant::now();
        let v = c
            .dispatch(json!({"op":"cells","cells":[{"sheetId":"s1","address":"A1","value":(i + 1).to_string()}]}))
            .unwrap();
        let edit_ms = t.elapsed().as_secs_f64() * 1000.0;
        s.edit_ms.push(edit_ms);

        // Untimed: keep a copy for inspection before `v` is moved into the
        // timed, api::dispatch-equivalent response build below.
        let inspect = v.clone();

        let t = Instant::now();
        let text = json!({"ok":true,"value":v}).to_string();
        let serialize_ms = t.elapsed().as_secs_f64() * 1000.0;
        s.serialize_ms.push(serialize_ms);
        s.total_ms.push(edit_ms + serialize_ms);

        s.response_bytes = text.len();
        s.last = inspect;
        s.last_response_text = text;
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
    // The api-equivalent response text must actually be the wrapped
    // {"ok":true,"value":…} shape that api::dispatch returns, not just the
    // bare engine snapshot.
    let parsed: Value = serde_json::from_str(&s.last_response_text).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["value"]["calculated"]["s1"]["B1"], 10.0);
    // The combined total must never be less than the engine edit alone.
    for (edit, total) in s.edit_ms.iter().zip(s.total_ms.iter()) {
        assert!(total >= edit);
    }
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
        "totalMs": {"p50": percentile(&s.total_ms, 0.5), "p95": percentile(&s.total_ms, 0.95)},
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
