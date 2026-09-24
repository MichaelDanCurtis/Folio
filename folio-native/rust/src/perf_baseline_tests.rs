//! Edit-path performance baseline. The smoke test runs in the normal suite so
//! the harness keeps compiling; the full baseline is opt-in:
//! `cargo test --release perf_baseline -- --ignored --nocapture`.
use crate::workspace::Core;
use serde_json::{json, Value};
use std::collections::BTreeMap;
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
    /// Per-stage durations across all measured edits, from `perf_stages`.
    stages: BTreeMap<&'static str, Vec<f64>>,
}

/// The epoch a response publishes, whether full or a delta.
fn epoch(response: &Value) -> Value {
    response
        .get("delta")
        .map_or(&response["epoch"], |d| &d["set"]["epoch"])
        .clone()
}

/// Edits s1!A1 `edits` times. Each edit is timed through the engine commit
/// (`edit_ms`) and, separately, through exactly the JSON response build that
/// `api::dispatch` performs — `json!({"ok":true,"value":v}).to_string()`,
/// which deep-copies the returned value while wrapping it (`serialize_ms`).
/// `total_ms` is the per-edit sum of the two, i.e. the same engine+response
/// cost a real `api::dispatch` call pays end to end. With `delta`, each
/// request asks for a delta against the previous response, as the app does.
/// The value inspected by callers (`last`) is cloned *before* the timed
/// response build so the clone never counts toward `serialize_ms`.
fn measure_edits(c: &mut Core, edits: usize, delta: bool) -> Samples {
    let mut s = Samples {
        edit_ms: vec![],
        serialize_ms: vec![],
        total_ms: vec![],
        response_bytes: 0,
        last: Value::Null,
        last_response_text: String::new(),
        stages: BTreeMap::new(),
    };
    let mut base = epoch(&c.dispatch(json!({"op":"get"})).unwrap());
    crate::perf_stages::enable(true);
    for i in 0..edits {
        let mut request = json!({"op":"cells","cells":[{"sheetId":"s1","address":"A1","value":(i + 1).to_string()}]});
        if delta {
            request["delta"] = base.clone();
        }
        let t = Instant::now();
        let v = c.dispatch(request).unwrap();
        let edit_ms = t.elapsed().as_secs_f64() * 1000.0;
        s.edit_ms.push(edit_ms);
        record_stages(&mut s.stages);
        base = epoch(&v);

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
    crate::perf_stages::enable(false);
    s
}

fn record_stages(stages: &mut BTreeMap<&'static str, Vec<f64>>) {
    let mut edit: BTreeMap<&'static str, f64> = BTreeMap::new();
    for (label, ms) in crate::perf_stages::take() {
        *edit.entry(label).or_default() += ms;
    }
    for (label, ms) in edit {
        stages.entry(label).or_default().push(ms);
    }
}

/// Undoes `count` edits, timing engine + api-equivalent response like
/// `total_ms`, asking for deltas.
fn measure_undo(c: &mut Core, count: usize) -> Vec<f64> {
    let mut base = epoch(&c.dispatch(json!({"op":"get"})).unwrap());
    (0..count)
        .map(|_| {
            let t = Instant::now();
            let v = c.dispatch(json!({"op":"undo","delta":base})).unwrap();
            base = epoch(&v);
            let text = json!({"ok":true,"value":v}).to_string();
            assert!(!text.is_empty());
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect()
}

/// The app sends cell edits through `prepareLayoutEdit` (staged engine that
/// returns rows to measure) and `commitLayoutEdit` (applies row sizes and
/// commits), asking for deltas. Times both dispatches plus both
/// api-equivalent response builds; stages go to `stages`.
fn measure_layout_edits(
    c: &mut Core,
    edits: usize,
    stages: &mut BTreeMap<&'static str, Vec<f64>>,
) -> (Vec<f64>, usize) {
    let mut base = epoch(&c.dispatch(json!({"op":"get"})).unwrap());
    let mut bytes = 0;
    crate::perf_stages::enable(true);
    let times = (0..edits)
        .map(|i| {
            let t = Instant::now();
            let edit = json!({"op":"cells","cells":[{"sheetId":"s1","address":"A2","value":(i + 7).to_string()}]});
            let prepared = c
                .dispatch(json!({"op":"prepareLayoutEdit","edit":edit,"delta":base}))
                .unwrap();
            let token = prepared["layoutToken"].clone();
            let mut sizes = json!({});
            for (id, rows) in prepared["layoutRows"].as_object().unwrap() {
                sizes[id] = json!({});
                for row in rows.as_array().unwrap() {
                    sizes[id][row.to_string()] = json!(20);
                }
            }
            let prepared_text = json!({"ok":true,"value":prepared}).to_string();
            let v = c
                .dispatch(json!({"op":"commitLayoutEdit","token":token,"sizes":sizes,"delta":base}))
                .unwrap();
            base = epoch(&v);
            let text = json!({"ok":true,"value":v}).to_string();
            bytes = prepared_text.len() + text.len();
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            record_stages(stages);
            ms
        })
        .collect();
    crate::perf_stages::enable(false);
    (times, bytes)
}

fn stage_report(stages: &BTreeMap<&'static str, Vec<f64>>) -> Value {
    Value::Object(
        stages
            .iter()
            .map(|(label, v)| {
                (
                    label.to_string(),
                    json!({"p50": percentile(v, 0.5), "p95": percentile(v, 0.95)}),
                )
            })
            .collect(),
    )
}

fn percentile(values: &[f64], p: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

#[test]
fn perf_harness_smoke() {
    let (mut c, _) = loaded(2, 10);
    let full = measure_edits(&mut c, 2, false);
    assert_eq!(full.last["calculated"]["s1"]["B1"], 4.0);
    let s = measure_edits(&mut c, 5, true);
    assert_eq!(s.edit_ms.len(), 5);
    assert_eq!(s.last["delta"]["calculated"]["s1"]["B1"], 10.0);
    assert!(s.response_bytes > 0);
    assert_eq!(s.stages["sync.calc"].len(), 5);
    assert!(
        !s.stages.contains_key("snapshot.build"),
        "deltas skip the snapshot"
    );
    assert_eq!(measure_undo(&mut c, 2).len(), 2);
    let mut stages = BTreeMap::new();
    let (layout, bytes) = measure_layout_edits(&mut c, 2, &mut stages);
    assert_eq!(layout.len(), 2);
    assert!(bytes > 0 && stages.contains_key("layout.commit"));
    assert_eq!(c.workspace["sheets"][0]["cells"]["A2"], "8");
    assert_eq!(c.workspace["sheets"][0]["cells"]["A1"], "3");
    // The api-equivalent response text must actually be the wrapped
    // {"ok":true,"value":…} shape that api::dispatch returns, not just the
    // bare engine snapshot.
    let parsed: Value = serde_json::from_str(&s.last_response_text).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["value"]["delta"]["calculated"]["s1"]["B1"], 10.0);
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
    // The Phase 1 measure: full snapshot per edit.
    let full = measure_edits(&mut c, 20, false);
    let s = measure_edits(&mut c, edits, true);
    let undo_ms = measure_undo(&mut c, 50);
    let mut layout_stages = BTreeMap::new();
    let (layout_ms, layout_bytes) = measure_layout_edits(&mut c, 50, &mut layout_stages);
    let report = json!({
        "workload": {"sheets": sheets, "populatedCells": sheets * rows * 2, "edits": edits},
        "loadMs": load_ms,
        "editMs": {"p50": percentile(&s.edit_ms, 0.5), "p95": percentile(&s.edit_ms, 0.95),
                   "first10Avg": s.edit_ms[..10].iter().sum::<f64>() / 10.0,
                   "last10Avg": s.edit_ms[edits - 10..].iter().sum::<f64>() / 10.0},
        "serializeMs": {"p50": percentile(&s.serialize_ms, 0.5), "p95": percentile(&s.serialize_ms, 0.95)},
        "totalMs": {"p50": percentile(&s.total_ms, 0.5), "p95": percentile(&s.total_ms, 0.95)},
        "responseBytes": s.response_bytes,
        "undoMs": {"p50": percentile(&undo_ms, 0.5), "p95": percentile(&undo_ms, 0.95)},
        "fullSnapshotTotalMs": {"p50": percentile(&full.total_ms, 0.5), "p95": percentile(&full.total_ms, 0.95)},
        "fullSnapshotBytes": full.response_bytes,
        "layoutEditMs": {"p50": percentile(&layout_ms, 0.5), "p95": percentile(&layout_ms, 0.95)},
        "layoutResponseBytes": layout_bytes,
        "stages": stage_report(&s.stages),
        "layoutStages": stage_report(&layout_stages),
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

fn sizes_for(prepared: &Value) -> Value {
    let mut sizes = json!({});
    for (id, rows) in prepared["layoutRows"].as_object().unwrap() {
        sizes[id] = json!({});
        for row in rows.as_array().unwrap() {
            sizes[id][row.to_string()] = json!(20);
        }
    }
    sizes
}

/// CI guard, no timing: after warm-up, edits, undo and staged layout edits
/// on a mid-size workbook never take a whole-workbook path, and deltas stay
/// small. Wall-clock numbers come from `perf_baseline` on the baseline Mac.
#[test]
fn edit_path_stays_incremental() {
    let (mut c, _) = loaded(4, 1_500);
    let full_paths = |labels: &[&'static str]| {
        labels
            .iter()
            .copied()
            .filter(|l| l.starts_with("full.") || *l == "snapshot.build" || *l == "cells.size")
            .collect::<Vec<_>>()
    };
    let mut base = epoch(&c.dispatch(json!({"op":"get"})).unwrap());
    // Warm up: the first edits may compute sizes and set up the staged path.
    for i in 0..3 {
        let edit =
            json!({"op":"cells","cells":[{"sheetId":"s2","address":"A1","value":i.to_string()}]});
        let prepared = c
            .dispatch(json!({"op":"prepareLayoutEdit","edit":edit,"delta":base}))
            .unwrap();
        let sizes = sizes_for(&prepared);
        let token = prepared["layoutToken"].clone();
        base = epoch(
            &c.dispatch(json!({"op":"commitLayoutEdit","token":token,"sizes":sizes,"delta":base}))
                .unwrap(),
        );
    }
    crate::perf_stages::enable(true);
    for i in 0..20 {
        let value = (i + 10).to_string();
        let edit = json!({"op":"cells","cells":[{"sheetId":"s1","address":"A1","value":value}]});
        let (response, what) = match i % 4 {
            0 => (
                c.dispatch(json!({"op":"undo","delta":base})).unwrap(),
                "undo",
            ),
            1 => (
                c.dispatch({
                    let mut e = edit;
                    e["delta"] = base.clone();
                    e
                })
                .unwrap(),
                "edit",
            ),
            _ => {
                let prepared = c
                    .dispatch(json!({"op":"prepareLayoutEdit","edit":edit,"delta":base}))
                    .unwrap();
                assert!(
                    prepared.get("delta").is_some(),
                    "step {i}: prepare sent a full snapshot"
                );
                assert!(
                    prepared.to_string().len() < 16_000,
                    "step {i}: prepare response"
                );
                let sizes = sizes_for(&prepared);
                let token = prepared["layoutToken"].clone();
                (
                    c.dispatch(
                        json!({"op":"commitLayoutEdit","token":token,"sizes":sizes,"delta":base}),
                    )
                    .unwrap(),
                    "layout",
                )
            }
        };
        assert!(
            response.get("delta").is_some(),
            "step {i} ({what}): full snapshot"
        );
        assert!(
            response.to_string().len() < 16_000,
            "step {i} ({what}): response size"
        );
        base = epoch(&response);
        let labels: Vec<_> = crate::perf_stages::take()
            .into_iter()
            .map(|(l, _)| l)
            .collect();
        assert_eq!(full_paths(&labels), Vec::<&str>::new(), "step {i} ({what})");
    }
    crate::perf_stages::enable(false);
    assert!(
        c.history_bytes_for_test() < 100_000,
        "undo history grew with the workbook"
    );
}
