# Phase 1: CI and Performance Baseline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every push and PR automatically runs the full Rust and Flutter checks, and a repeatable benchmark records how long a single-cell edit takes on an Excel-scale workbook.

**Architecture:** A GitHub Actions workflow on a macOS runner (the Rust engine is loaded as a `.dylib`, so Flutter tests need macOS). Two benchmark harnesses: a Rust test that drives `Core::dispatch` directly (engine + serialization time) and a Flutter test that drives the real `CoreBridge` (full round trip including Dart JSON decoding). Both are opt-in so normal test runs stay fast; a tiny Rust smoke test runs every time so the harness keeps compiling.

**Tech Stack:** GitHub Actions, Rust 1.98 / cargo, Flutter 3.44.7 stable (Dart ^3.12), flutter_rust_bridge 2.13.

## Global Constraints

- Flutter version in CI: `3.44.7`, channel `stable` (matches the verified local toolchain).
- CI runner: `macos-15` (Apple Silicon; the engine is loaded from `folio-native/rust/target/release/libfolio_core.dylib`).
- Only gate on checks that currently pass: `cargo fmt --check` (clean), `cargo test`, `flutter analyze` (clean), `flutter test`, and `dart format` (after Task 1 formats the four files that are currently unformatted). Do **not** gate on `cargo clippy` yet (27 existing warnings).
- Benchmarks must never run in the default `cargo test` / `flutter test` runs, except the tiny Rust smoke test.
- Live AI tests stay skipped in CI (they need a signed-in Codex runtime).
- All commands run from `folio-native/` unless stated otherwise.
- Branch from latest `main`; one PR for the whole phase.

## Prototype results (2026-09-23, Apple Silicon Mac, release build)

The harness code below was prototyped and run before this plan was written. Task 5 re-measures and records official numbers.

| Measure | Result | ROADMAP §7 target |
|---|---|---|
| Engine single-cell edit, p50 / p95 | 294 ms / 313 ms | ≤ 50 ms typing, p95 |
| Engine edit, first 10 vs last 10 of 200 edits | 238 ms → 299 ms (grows as undo fills) | — |
| JSON serialization per edit, p50 | 22 ms | — |
| Response size per single-cell edit | 7.76 MB | — |
| Full Flutter round trip, p50 / p95 | 446 ms / 460 ms | ≤ 50 ms |
| Load 100k-cell workspace | 373 ms | ≤ 5 s open |

---

### Task 1: CI workflow

**Files:**
- Create: `.github/workflows/ci.yml` (repository root, not `folio-native/`)
- Modify (formatting only): `folio-native/lib/src/document_list_rules.dart`, `folio-native/test/contextual_ai_test.dart`, `folio-native/test/document_revisions_workspace_test.dart`, `folio-native/test/watch_window_test.dart`

**Interfaces:**
- Produces: a workflow named `CI` with one job `test`. Later phases add steps to this job.

- [ ] **Step 1: Confirm the formatting gate currently fails**

Run: `cd folio-native && dart format --output=none --set-exit-if-changed lib test`
Expected: exit code 1, listing exactly the four files above as `Changed`.

- [ ] **Step 2: Format those four files**

Run: `cd folio-native && dart format lib/src/document_list_rules.dart test/contextual_ai_test.dart test/document_revisions_workspace_test.dart test/watch_window_test.dart`
Then: `dart format --output=none --set-exit-if-changed lib test`
Expected: exit code 0.

- [ ] **Step 3: Confirm formatting changed no behavior**

Run: `cd folio-native && flutter analyze && flutter test test/contextual_ai_test.dart test/watch_window_test.dart`
Expected: `No issues found!` and `All tests passed!` (skips are fine). This requires the engine built once: `cargo build --release --manifest-path rust/Cargo.toml`.

- [ ] **Step 4: Write the workflow**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  test:
    runs-on: macos-15
    timeout-minutes: 60
    defaults:
      run:
        working-directory: folio-native
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: folio-native/rust

      - uses: subosito/flutter-action@v2
        with:
          flutter-version: 3.44.7
          channel: stable
          cache: true

      - name: Rust format
        run: cargo fmt --check --manifest-path rust/Cargo.toml

      - name: Rust tests
        run: cargo test --manifest-path rust/Cargo.toml

      - name: Build engine
        run: cargo build --release --manifest-path rust/Cargo.toml

      - name: Flutter dependencies
        run: flutter pub get

      - name: Dart format
        run: dart format --output=none --set-exit-if-changed lib test

      - name: Flutter analyze
        run: flutter analyze

      - name: Flutter tests
        run: flutter test
```

- [ ] **Step 5: Validate the YAML locally**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('ok')"` (from the repository root)
Expected: `ok`

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/ci.yml folio-native/lib/src/document_list_rules.dart folio-native/test/contextual_ai_test.dart folio-native/test/document_revisions_workspace_test.dart folio-native/test/watch_window_test.dart
git commit -m "Add CI workflow and format the four unformatted Dart files"
```

- [ ] **Step 7: Push and confirm CI runs green**

```bash
git push -u origin HEAD
gh pr create --base main --title "Phase 1: CI and performance baseline" --body "Work in progress; see docs/superpowers/plans/2026-09-23-phase-1-ci-and-baseline.md"
gh pr checks --watch
```

Expected: the `test` job passes. If `flutter test` fails in CI but passes locally, look for a test that depends on local machine state (as the `../../work/` writes did) and fix it the same way: make it opt-in behind `FOLIO_CAPTURE_PREVIEWS=1`. Do not skip or delete assertions.

---

### Task 2: Rust edit-path benchmark

**Files:**
- Create: `folio-native/rust/src/perf_baseline_tests.rs`
- Modify: `folio-native/rust/src/lib.rs` (append the module declaration at the end)

**Interfaces:**
- Consumes: `crate::workspace::Core` — `Core::new()`, `Core::dispatch(&mut self, Value) -> Result<Value, String>`, public field `workspace: Value`. Ops used: `{"op":"new","blank":true}`, `{"op":"replace","workspace":…}`, `{"op":"cells","cells":[{"sheetId","address","value"}]}`. A successful `cells` edit returns the snapshot, including `calculated[sheetId][address]` as a JSON number.
- Produces: `perf_baseline` (ignored test) writes JSON to `$FOLIO_PERF_OUT` or `<temp>/folio-perf-baseline.json` with keys `workload`, `loadMs`, `editMs.{p50,p95,first10Avg,last10Avg}`, `serializeMs.{p50,p95}`, `responseBytes`. Phase 2 uses this exact command and output to prove its improvement.

- [ ] **Step 1: Write the smoke test and harness**

Create `folio-native/rust/src/perf_baseline_tests.rs`:

```rust
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
```

- [ ] **Step 2: Run the smoke test before registering the module**

Run: `cd folio-native/rust && cargo test perf_harness_smoke`
Expected: `0 passed … 405 filtered out`. The file is not compiled yet because `lib.rs` doesn't declare it; this confirms the test name is new.

- [ ] **Step 3: Register the module**

Append to the end of `folio-native/rust/src/lib.rs`:

```rust
#[cfg(test)]
mod perf_baseline_tests;
```

- [ ] **Step 4: Run the smoke test**

Run: `cd folio-native/rust && cargo test perf_harness_smoke`
Expected: `1 passed; 0 failed`.

- [ ] **Step 5: Confirm the full baseline stays out of normal runs**

Run: `cd folio-native/rust && cargo test 2>&1 | grep "test result" | head -1`
Expected: `406 passed; 0 failed; 1 ignored`.

- [ ] **Step 6: Run the baseline once**

Run: `cd folio-native/rust && cargo test --release perf_baseline -- --ignored --nocapture`
Expected: passes in roughly 1–2 minutes and prints JSON. `editMs.p50` should be in the hundreds of milliseconds (prototype: 294 ms). A number wildly different from the prototype means the workload or build profile differs; check that `--release` was used.

- [ ] **Step 7: Check formatting and commit**

```bash
cd folio-native/rust && cargo fmt --check
cd ../.. && git add folio-native/rust/src/perf_baseline_tests.rs folio-native/rust/src/lib.rs
git commit -m "Add Rust edit-path performance baseline harness"
```

---

### Task 3: Flutter bridge round-trip benchmark

**Files:**
- Create: `folio-native/test/perf_baseline_test.dart`

**Interfaces:**
- Consumes: `CoreBridge` from `package:folio_native/src/core.dart` — `CoreBridge.initialize()` (static, loads the dylib), `call(Map<String, dynamic>) -> Future<Map<String, dynamic>>`. Same ops as Task 2.
- Produces: with `FOLIO_PERF=1`, writes JSON to `$FOLIO_PERF_DART_OUT` or `<systemTemp>/folio-perf-dart.json` with keys `workload`, `roundTripMs.{p50,p95}`.

- [ ] **Step 1: Write the benchmark**

Create `folio-native/test/perf_baseline_test.dart`:

```dart
// Edit round trip through the real bridge, including Dart JSON decoding.
// Opt-in: FOLIO_PERF=1 flutter test test/perf_baseline_test.dart
import 'dart:convert';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/core.dart';

double percentile(List<double> values, double p) {
  final sorted = [...values]..sort();
  return sorted[((sorted.length - 1) * p).round()];
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final core = CoreBridge();
  setUpAll(CoreBridge.initialize);
  test(
    'single-cell edit round trip on a 20-sheet, 100k-cell workbook',
    () async {
      const sheets = 20, rows = 2500, edits = 50;
      await core.call({'op': 'new', 'blank': true});
      final workspace = Map<String, dynamic>.from(
        (await core.call({'op': 'get'}))['workspace'],
      );
      workspace['sheets'] = [
        for (var s = 1; s <= sheets; s++)
          {
            'id': 's$s',
            'name': 'Sheet $s',
            'cells': {
              for (var r = 1; r <= rows; r++) ...{
                'A$r': '$r',
                'B$r': '=A$r*2',
              },
            },
          },
      ];
      await core.call({'op': 'replace', 'workspace': workspace});
      final roundTrip = <double>[];
      Map<String, dynamic> last = {};
      for (var i = 0; i < edits; i++) {
        final watch = Stopwatch()..start();
        last = await core.call({
          'op': 'cells',
          'cells': [
            {'sheetId': 's1', 'address': 'A1', 'value': '${i + 1}'},
          ],
        });
        roundTrip.add(watch.elapsedMicroseconds / 1000);
      }
      expect(last['calculated']['s1']['B1'], edits * 2);
      final report = {
        'workload': {
          'sheets': sheets,
          'populatedCells': sheets * rows * 2,
          'edits': edits,
        },
        'roundTripMs': {
          'p50': percentile(roundTrip, 0.5),
          'p95': percentile(roundTrip, 0.95),
        },
      };
      final out =
          Platform.environment['FOLIO_PERF_DART_OUT'] ??
          '${Directory.systemTemp.path}/folio-perf-dart.json';
      File(out).writeAsStringSync(
        const JsonEncoder.withIndent('  ').convert(report),
      );
      // ignore: avoid_print
      print('${const JsonEncoder.withIndent('  ').convert(report)}\n$out');
    },
    skip: Platform.environment['FOLIO_PERF'] != '1',
    timeout: const Timeout(Duration(minutes: 10)),
  );
}
```

- [ ] **Step 2: Confirm it is skipped by default**

Run: `cd folio-native && flutter test test/perf_baseline_test.dart`
Expected: `All tests skipped.`

- [ ] **Step 3: Run it**

Run: `cd folio-native && FOLIO_PERF=1 flutter test test/perf_baseline_test.dart`
Expected: passes and prints JSON; `roundTripMs.p50` in the hundreds of milliseconds (prototype: 446 ms). Requires the release engine from `cargo build --release --manifest-path rust/Cargo.toml`.

- [ ] **Step 4: Check analysis and formatting, then commit**

```bash
cd folio-native && flutter analyze && dart format --output=none --set-exit-if-changed test/perf_baseline_test.dart
cd .. && git add folio-native/test/perf_baseline_test.dart
git commit -m "Add Flutter bridge round-trip performance benchmark"
```

---

### Task 4: Record the baseline

**Files:**
- Create: `folio-native/docs/PERFORMANCE_BASELINE.md`
- Modify: `folio-native/README.md` (Verify section, near line 536: add the two benchmark commands)

**Interfaces:**
- Consumes: the JSON outputs of Task 2 and Task 3.
- Produces: the numbers Phase 2's exit gate is measured against.

- [ ] **Step 1: Capture machine and build identity**

Run:
```bash
sysctl -n machdep.cpu.brand_string; sysctl -n hw.memsize | awk '{print $1/1073741824 " GiB"}'; sw_vers -productVersion; rustc --version; flutter --version | head -1; git rev-parse --short HEAD
```
Record each value.

- [ ] **Step 2: Run both benchmarks three times each**

```bash
cd folio-native/rust && for i in 1 2 3; do FOLIO_PERF_OUT=/tmp/folio-rust-$i.json cargo test --release perf_baseline -- --ignored >/dev/null; done
cd .. && for i in 1 2 3; do FOLIO_PERF=1 FOLIO_PERF_DART_OUT=/tmp/folio-dart-$i.json flutter test test/perf_baseline_test.dart >/dev/null; done
cat /tmp/folio-rust-*.json /tmp/folio-dart-*.json
```

Close other heavy applications first. Use the median of the three runs for each number.

- [ ] **Step 3: Write `folio-native/docs/PERFORMANCE_BASELINE.md`**

Use this structure, filling each cell with the measured median from Step 2 and identity from Step 1:

```markdown
# Performance baseline

Measured <date> on <CPU>, <RAM>, macOS <version>; rustc <version>, Flutter <version>; commit <sha>. Release build. Median of three runs.

Workload: 20 sheets × 2,500 rows (A = number, B = `=A{r}*2`), 100,000 populated cells. Each edit changes Sheet 1 A1.

| Measure | Result | ROADMAP §7 target |
|---|---|---|
| Load workspace (`replace`) | <loadMs> ms | open ≤ 5 s |
| Engine edit p50 / p95 (200 edits) | <p50> / <p95> ms | typing p95 ≤ 50 ms |
| Engine edit, first 10 avg → last 10 avg | <first10Avg> → <last10Avg> ms | flat |
| Serialize response p50 / p95 | <p50> / <p95> ms | — |
| Response size per edit | <responseBytes / 1e6> MB | — |
| Flutter round trip p50 / p95 (50 edits) | <p50> / <p95> ms | typing p95 ≤ 50 ms |

## Reproduce

    cd folio-native/rust && cargo test --release perf_baseline -- --ignored --nocapture
    cd folio-native && FOLIO_PERF=1 flutter test test/perf_baseline_test.dart

## Known causes (for Phase 2)

- `Core::commit_journal` (`rust/src/workspace.rs`) clones the full workspace into each undo entry and then re-serializes every undo entry to measure total history bytes, so commit cost grows with history length.
- `Core::snapshot` returns the full workspace plus all calculated and formatted values after every edit; `CoreBridge` decodes it in full.
```

- [ ] **Step 4: Add the commands to the README**

In `folio-native/README.md`, directly after the existing `flutter test` line in the verify commands (around line 538), add:

```sh
# Performance baseline (opt-in; see docs/PERFORMANCE_BASELINE.md)
cargo test --release --manifest-path rust/Cargo.toml perf_baseline -- --ignored --nocapture
FOLIO_PERF=1 flutter test test/perf_baseline_test.dart
```

Keep the surrounding fence intact.

- [ ] **Step 5: Commit and push**

```bash
git add folio-native/docs/PERFORMANCE_BASELINE.md folio-native/README.md
git commit -m "Record edit-path performance baseline"
git push
gh pr checks --watch
```

Expected: CI green. The PR is ready for review and merge.

---

## Phase 1 exit checklist

- [ ] CI `test` job green on the PR and, after merge, on `main`
- [ ] `cargo test` shows 406 passed, 1 ignored; `flutter test` shows 0 failures
- [ ] `folio-native/docs/PERFORMANCE_BASELINE.md` contains measured numbers and machine identity
- [ ] Phase 2 plan written from those numbers
