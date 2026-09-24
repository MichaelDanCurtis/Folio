# Phase 2: Edit-Path Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A single-cell edit on the 20-sheet, 100k-cell workload costs time proportional to what changed, not to workbook size: engine + response p95 ≤ 50 ms, undo accounting O(1) per commit, and edit cost that stays flat as undo history fills.

**Architecture:** Keep the existing "build the next workspace, then commit it" model that all 31 commit call sites use, but make everything after it incremental. Commit computes a structural diff (a *patch*) between the old and new workspace. That patch replaces the full-workspace undo snapshot, tells the calculator, display formatter and conditional-format evaluator which cells changed, and becomes the response to Flutter (a *delta* instead of the 7.76 MB snapshot). The two hottest ops (`cells`, `enterCell`) then get a fast path that edits in place and records the patch directly, skipping the full-workspace clone and diff. Every fast path keeps the existing full path as its fallback and is checked against it by differential tests.

**Tech Stack:** Rust 1.98.1, serde_json, vendored IronCalc 0.8.3 (`rust/vendor/ironcalc_base`), Flutter 3.44.7, flutter_rust_bridge 2.13.

## Global Constraints

- All commands run from `folio-native/` unless stated otherwise. Use the pinned toolchain (`cargo +1.98.1 …` if it isn't the default).
- **Observable behavior does not change.** Full `get` snapshots stay byte-for-byte the same shape; undo/redo produce the same workspaces, revisions and `documentHistoryChange` payloads as today. The existing Rust and Flutter suites (395 passed, 2 skipped) must pass after every task.
- **Every fast path has a fallback and a differential test.** When a change can't be expressed incrementally (sheet added or removed or renamed, tables, filters, hidden rows, styles, date system, conditional rules, document edits), take today's full path. A test must show that the fast path and the full path produce identical results.
- **No wall-clock timing assertions in CI** (runner speed varies). CI guards use structural measures: bytes, counts of cells touched, and whether a full rebuild ran. Timing is measured by the Phase 1 harnesses on the baseline machine.
- Rewriting the workspace from `serde_json::Value` to typed structs stays out of scope (see the program plan). Incremental formula recalculation inside IronCalc is also out of scope; see *Known floor* below.
- Branch from latest `main` after the Phase 1 PR merges; one PR for the phase.

## Where the time goes today

Measured 2026-09-24 by temporarily instrumenting `Core::commit_journal`, `Core::sync`, `Core::snapshot` and `WorkbookCalculator::update`. Run with the Phase 1 Rust harness in release mode, over the first ~50 of 200 edits. The machine was a 4-vCPU Linux container, roughly 5× slower than the M4 Max baseline, so compare the shares, not the absolute numbers. The instrumentation was not committed.

| Stage | Where | Median ms (container) | Share of engine time | Scales with |
|---|---|---:|---:|---|
| Display formatting of every cell | `number_format::Display::sync` | 330 | 22% | all cells |
| Calculator | `WorkbookCalculator::update` | 325 | 22% | all cells |
| ├ rebuild input map from JSON | `update` lines 178–201 | 73 | | all cells |
| ├ diff inputs and set changed ones | `update` lines 243–264 | 38 | | all cells (hash set of every address) |
| ├ IronCalc `evaluate()` (full recalc) | vendor `model.rs:3127` | 92 | | all formulas |
| └ read back every value into a new map | `update` lines 267–291 | 78 | | all cells |
| Undo size accounting | `commit_journal` lines 631–635 | 270 | 18% | cells × history length (grows until 100 entries) |
| Snapshot JSON build | `snapshot` line 420 | 227 | 15% | all cells (deep copies workspace + values + display) |
| Build proposal (clone + apply + validate) | `proposed` | 114 | 8% | all cells |
| Conditional-format state key | `sync` line 319 | 58 | 4% | all cells (clones every value) |
| Conditional-format compare/evaluate | `sync` lines 320–354 | 48 | 3% | all cells |
| File-state digests (SHA-256 over sheets + values) | `file_states` | 46 | 3% | all cells |
| Undo snapshot clone | `commit_journal` line 628 | 31 | 2% | all cells |
| Validate again | `commit_journal` line 624 | 20 | 1% | all cells |
| Drop previous workspace | `commit_journal` line 626 | 17 | 1% | all cells |

Not included in the table: `api::dispatch` then deep-copies and serializes the snapshot again (69.9 ms p50 on the M4 Max), and Dart decodes the 7.76 MB string (~80 ms of the 448.6 ms Flutter round trip).

**Conclusion:** No single stage dominates. Every stage above is O(workbook), and together they make up the whole cost. Reaching 50 ms requires removing all of them from the per-edit path except IronCalc's `evaluate()`.

**Known floor:** vendored IronCalc 0.8.3 recalculates every formula on `evaluate()`. On this workload that is ~92 ms in the container, estimated at ~18–25 ms on the M4 Max. That fits the 50 ms budget at 100k cells, but not at the M3 target of one million cells. Task 8 records the measured floor, and incremental recalculation becomes an M3 work item, not a Phase 2 one.

## Task order and dependencies

```
T1 stage timers ─┬─> T2 patch + undo ─> T3 change set ─┬─> T4 incremental calculator ─┐
                 │                                      ├─> T5 incremental display/CF ─┼─> T7 fast path ─> T8 delta responses ─> T9 re-measure
                 └─> T6 lazy file states ───────────────┴──────────────────────────────┘
```

T4, T5 and T6 are independent of each other and can run in parallel once T3 lands (T6 needs only T1).

| Task | Removes (container ms) | Expected share removed |
|---|---:|---:|
| T2 patch-based undo | 270 + 31 + 17 | ~21% |
| T4 incremental calculator | 73 + 38 + 78 (keeps `evaluate`) | ~13% |
| T5 incremental display + CF | 330 + 58 + 48 | ~29% |
| T6 lazy file states | 46 | ~3% |
| T7 in-place `cells` fast path | 114 + 20 | ~9% |
| T8 delta responses | 227 + all of api serialize + most of Dart decode | ~15% plus the response cost |

---

### Task 1: Permanent stage timers in the benchmark harness

**Files:**
- Create: `rust/src/perf_stages.rs`
- Modify: `rust/src/lib.rs` (add `mod perf_stages;`), `rust/src/workspace.rs`, `rust/src/calculation.rs`, `rust/src/perf_baseline_tests.rs`

**Interfaces:**
- Produces: `perf_stages::time(label: &'static str, f: impl FnOnce() -> T) -> T` and `perf_stages::take() -> Vec<(&'static str, f64)>`. Recording is off unless `perf_stages::enable(true)` was called on the current thread. When off, the cost is one thread-local bool read.

- [ ] **Step 1:** Implement `perf_stages` with a `thread_local!` `RefCell<Option<Vec<(&'static str, f64)>>>`. `time` records elapsed ms only when the option is `Some`. Unit-test that nothing is recorded while disabled and that labels come back in order while enabled.
- [ ] **Step 2:** Wrap the stages from the table above with `perf_stages::time`, using these labels: `proposed`, `validate`, `undo.push`, `undo.account`, `sync.calc`, `calc.inputs`, `calc.set`, `calc.evaluate`, `calc.collect`, `sync.cf_state`, `sync.cf_eval`, `sync.display`, `snapshot.file_states`, `snapshot.build`. Keep the wrapped expressions unchanged.
- [ ] **Step 3:** In `perf_baseline`, enable recording and add a `"stages"` object to the report: for each label, p50 and p95 over the measured edits. Also add `undo` to the report: time 50 `undo` dispatches after the edit loop (engine + api-equivalent response, same as `totalMs`), p50 and p95.
- [ ] **Step 4:** Run `cargo test` (default suite, harness smoke test included) and `cargo test --release perf_baseline -- --ignored --nocapture`. Expected: all pass, and the report contains `stages` and `undo`.
- [ ] **Step 5:** Commit: `Add per-stage timing to the edit-path benchmark`.

### Task 2: Patch-based undo history

**Files:**
- Create: `rust/src/json_patch.rs`, `rust/src/json_patch_tests.rs`
- Modify: `rust/src/document_history.rs`, `rust/src/workspace.rs` (`commit_journal`, the `undo`/`redo` arm, the history uses at the `commitLayoutEdit` arm near line 1169, and the `undo.clear()` sites), `rust/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  pub enum Op { Set { path: Vec<Key>, value: Value }, Remove { path: Vec<Key> } }
  pub enum Key { Field(String), Index(usize) }
  pub struct Patch { pub ops: Vec<Op> }
  /// Ops that turn `before` into `after`. Descends into objects key by key and
  /// into arrays of equal length index by index. Anything else is replaced whole.
  pub fn diff(before: &Value, after: &Value) -> Patch;
  pub fn apply(target: &mut Value, patch: &Patch) -> Result<(), String>;
  /// Applies `patch` and returns the patch that undoes it.
  pub fn apply_inverse(target: &mut Value, patch: &Patch) -> Result<Patch, String>;
  impl Patch { pub fn bytes(&self) -> usize } // computed once, from the ops' serialized sizes
  ```
- `document_history::Entry` becomes `{ patch: json_patch::Patch, bytes: usize, document: Option<Journal> }`, where `patch` turns the *current* workspace back into the one before the commit. `bytes` is computed once, at construction.

- [ ] **Step 1 (tests first):** In `json_patch_tests.rs`, write property tests with a small deterministic generator (seeded LCG; no new dependency). The generator builds random JSON of depth ≤ 4 and applies random mutations: set/remove object keys, change leaves, change array lengths, and replace containers of a different type. Assert, for 2,000 cases: `apply(before.clone(), diff(before, after)) == after`, and `apply_inverse` round-trips (`before` → `after` → `before`). Add a targeted test: two 100k-cell workbooks from the perf harness's `workbook()` that differ in one cell produce a patch with exactly one `Set` op, and `bytes() < 256`.
- [ ] **Step 2:** Implement `json_patch` until the tests pass. `diff` must not allocate for subtrees that are equal. Compare with `==` before descending, but descend into objects even when they differ in only one key.
- [ ] **Step 3:** Change `commit_journal` to push `Entry { patch: diff(&next, &previous) … }`. That is the patch from the new workspace back to the previous one, computed **before** `previous` is dropped. Keep a running `history_bytes: usize` field on `Core`: add on push, subtract on eviction or pop, and reset on the `undo.clear()`/`redo.clear()` sites. Delete the per-commit `iter().map(bytes).sum()`. Store the undo stack as a `VecDeque` so eviction from the front is O(1).
- [ ] **Step 4:** Rewrite the `undo`/`redo` arm. Pop the entry, then set `revision` first. Apply `apply_inverse(&mut self.workspace, &entry.patch)` to get the opposite patch, then push `Entry { patch: opposite, bytes: opposite.bytes(), document }` onto the other stack. Revision handling must match today exactly: the restored workspace gets `current revision + 1`. The revision field is excluded from the diff by setting it on both sides before diffing, or by diffing then dropping any op on `["revision"]`; either is fine, as long as the tests below pass. Check `sync`'s `_previous` argument: nothing reads it. Pass `&Value::Null` and remove the unused `old.clone()`.
- [ ] **Step 5:** Update the `commitLayoutEdit` arm (around line 1169), which reads `staged.undo.last()` for the document journal. Its behavior must stay the same.
- [ ] **Step 6:** Add `rust/src/document_history_tests.rs` cases (the file exists). For a scripted sequence of 30 mixed ops (cells, document edit, sheet add/rename/delete, style change, undo, redo, a new edit after undo), keep a second `Core` that uses a test-only `full_snapshot_history` flag. That flag holds today's full-workspace undo stack. Assert that the workspaces and `documentHistoryChange` values are identical after every step.
- [ ] **Step 7:** Run `cargo test` and the benchmark. Expected: all pass, `undo.account` p95 < 1 ms, and `first10Avg`/`last10Avg` differ by < 10%. Commit: `Store undo history as reversible patches with O(1) size accounting`.

### Task 3: Change set from each commit

**Files:**
- Create: `rust/src/change_set.rs`
- Modify: `rust/src/workspace.rs` (`commit_journal`, `sync`, undo/redo arm)

**Interfaces:**
- Produces:
  ```rust
  pub enum ChangeSet {
      /// Only `sheets[i].cells[address]` values changed (set or removed), and the
      /// sheet list's ids, names and order are unchanged.
      Cells(Vec<(usize /*sheet index*/, String /*address*/)>),
      /// Anything else: take the full path.
      Structural,
  }
  impl ChangeSet { pub fn from_patch(p: &Patch) -> Self; }
  ```
- `Core::sync` becomes `sync(&mut self, changes: &ChangeSet)`. Every caller passes `&ChangeSet::Structural` except `commit_journal` and undo/redo, which pass `ChangeSet::from_patch`.

- [ ] **Step 1 (tests first):** Unit tests for `from_patch`. A single-cell edit gives `Cells([(0,"A1")])`. Clearing a cell gives `Cells` with a remove. Editing a cell and a style, a sheet rename, a document edit, or a change to `tables`/`hiddenRows`/`autoFilter`/`mergedRanges`/`conditionalFormats` each give `Structural`. So does any `Set` whose path is shorter than `["sheets", i, "cells", addr]`.
- [ ] **Step 2:** Implement, then thread the change set through `sync`. No behavior change yet: `sync` ignores the argument in this task.
- [ ] **Step 3:** `cargo test`. Commit: `Derive a cell-level change set from each commit`.

### Task 4: Incremental calculator

**Files:**
- Modify: `rust/src/calculation.rs`, `rust/src/workspace.rs` (`sync`), `rust/vendor/ironcalc_base/src/model.rs` (one public accessor)
- Test: add cases to the existing `calculation.rs` test module or a new `rust/src/calculation_incremental_tests.rs`

**Interfaces:**
- Produces: `WorkbookCalculator::update_cells(&mut self, sheets: &Value, cells: &[(usize, String)], date1904: bool) -> Result<ChangedValues, String>`, where `ChangedValues` is `Vec<(usize, String)>`: the cells whose calculated value or error flag changed. The existing `sync_calendar` stays as the full path and returns `ChangedValues::All` (use an enum `Changed { All, Cells(Vec<…>) }`).
- The calculator keeps `formula_cells: Vec<HashSet<String>>` (addresses whose input starts with `=`) up to date for both paths.

- [ ] **Step 1 (tests first):** Differential test. Load the 20×2,500 perf workbook, extended with cross-sheet formulas (`='Sheet 1'!A1+1` on Sheet 2), a spill (`=SEQUENCE(3)`), an error (`=1/0`), a text cell, and a volatile function (`=NOW()` with a frozen clock). Then apply 200 seeded random single-cell edits. The edits include setting, clearing, formula-to-value and value-to-formula changes, and inputs that create or remove errors. After each edit, compare the incremental calculator's `values` and `cell_errors` to a fresh calculator's full `sync_calendar` on the same sheets: they must be equal. The `ChangedValues` set must contain every cell whose value differs from the previous step.
- [ ] **Step 2:** Implement `update_cells`. Fall back to the full path when the model is missing, when `force` is set, or when the calendar or clock day changed. For each changed cell: update `self.inputs[i]` in place, qualify the table reference as today, `set_user_input`, and update `formula_cells`. Then call `evaluate()`. Read back only the edited cells plus `formula_cells` and every spilled cell. IronCalc's `spill_cells` field is `pub(crate)`, so add a small public accessor such as `Model::spill_cells()` in `rust/vendor/ironcalc_base/src/model.rs` and note the vendor patch in the file's header comment, updating `self.values`/`self.cell_errors` in place and recording what differs. Bump `generation` as today.
- [ ] **Step 3:** In `Core::sync`, use `update_cells` when given `ChangeSet::Cells`, and the full path otherwise.
- [ ] **Step 4:** `cargo test` plus the benchmark. Expected: `calc.inputs` and `calc.set` p95 < 1 ms; `calc.collect` falls to the time needed to read the 50k formula cells. Record `calc.evaluate` unchanged. Commit: `Recalculate from changed inputs instead of rebuilding the calculator's input maps`.

### Task 5: Incremental display formatting and conditional-format state

**Files:**
- Modify: `rust/src/number_format.rs` (`Display::sync`), `rust/src/workspace.rs` (`sync`), `rust/src/conditional_format.rs` only if a helper for "sheet has rules" is missing
- Test: `rust/src/number_format_tests.rs`, `rust/src/conditional_format_tests.rs`

**Interfaces:**
- Produces: `Display::sync_cells(&mut self, sheets, calculated, errors, conditional, cells: &[(usize, String)])`, which recomputes display entries only for the listed cells. The existing `sync` stays the full path.
- `Core` replaces the `conditional_state: Value` key with two cheap values: a `rules_key: u64` hash of just the rule-bearing fields (`id`, `name`, `conditionalFormats`, `extendedConditionalFormats`, `conditionalRightToLeft` per sheet), and the calculator `generation`.

- [ ] **Step 1 (tests first):** Differential tests matching Task 4's pattern. After each random edit, compare `display.values` from the incremental path with a fresh full `Display::sync`. Do the same for the conditional overlays and warnings. Include sheets with and without conditional formats, a number-format style on the edited cell, and an error cell.
- [ ] **Step 2:** Conditional formats. If no sheet has rules, which the workload doesn't, skip evaluation entirely: overlays and warnings are empty per sheet. If rules exist and only values changed, re-evaluate sheets that have rules; that is today's behavior scoped to rule-bearing sheets. Record "CF re-evaluation is still per-sheet" as an M3 follow-up in `PERFORMANCE_BASELINE.md`. Never build the JSON clone of all calculated values again.
- [ ] **Step 3:** Display. With `ChangeSet::Cells`, and no change to rules, styles, date system or palette, format only (edited cells ∪ changed values ∪ cells whose conditional overlay changed), updating `self.values` in place. Replace the whole-map JSON `state`/`previous_values` equality checks with the generation and the change set. Any style, palette, date-system or conditional-overlay-wide change takes the full path.
- [ ] **Step 4:** `cargo test` plus the benchmark. Expected: `sync.cf_state`, `sync.cf_eval` and `sync.display` each have p95 < 2 ms on the workload. Commit: `Format and evaluate conditional rules only for changed cells`.

### Task 6: File-state digests on demand

**Files:**
- Modify: `rust/src/workspace.rs` (`snapshot`, `file_states`, add `fileStates` op, add it to the read-only op list in `dispatch`), `lib/src/core.dart` (`WorkspaceModel`), `lib/main.dart` (call sites at lines ~241, 1110, 1123, 1257, 1264, 1545, 1691)
- Test: `rust/src/file_state_tests.rs`, `test/working_files_test.dart`, `test/working_files_leave_test.dart`

**Interfaces:**
- Produces: a `{"op":"fileStates"}` request that returns `{"revision":n,"fileStates":{…}}`, cached on `Core` by revision and package-set identity. `snapshot()` keeps `fileStates` for full snapshots only; T8's deltas omit it.
- Dart: `Future<Map> WorkspaceModel.fileStates()`, which returns the cached value when `revision` matches and otherwise fetches through the same `CoreBridge` queue.

- [ ] **Step 1:** Rust: add the op and the cache, plus a test that the op's result equals `snapshot()["fileStates"]` before and after edits, undo, save and load.
- [ ] **Step 2:** Dart: convert each call site to `await model.fileStates()`. Every listed site is inside an async save, close or open flow; if one is synchronous (for example the collection-`for` near line 1105), make its enclosing function async, or read a value that the enclosing flow already awaited. Do not add a sync fallback that can report a stale "clean" state.
- [ ] **Step 3:** `flutter test` (the dirty-file and Don't-Save tests must pass unchanged) and `cargo test`. Commit: `Compute file-state digests on request instead of on every edit`.

### Task 7: In-place fast path for `cells` and `enterCell`

**Files:**
- Modify: `rust/src/workspace.rs` (`proposed`, the `cells` and `enterCell` arms, `commit_journal`), `rust/src/tables.rs` (`after_cells`), plus `data_validation::check` only if it needs a borrowed variant
- Test: create `rust/src/cells_fast_path_tests.rs`

**Interfaces:**
- Produces: `Core::commit_cells(&mut self, changes: &[Value], summary: &str) -> Result<Value, String>`. It applies cell edits to `self.workspace` in place, builds the inverse patch directly from the old cell values (no clone, no diff), and then runs the same post-commit steps as `commit_journal` (revision bump, history push, `sync(&ChangeSet::Cells(..))`, response).
- Eligibility: the fast path is taken only when every change is a plain cell value and `tables::after_cells` would not modify anything outside `sheets[i].cells`. Anything else calls today's `proposed` + `commit`.

- [ ] **Step 1 (tests first):** Differential test with two `Core`s loaded identically, one forced onto the slow path via a test-only flag. Run 300 seeded operations: single and multi-cell `cells`, `enterCell`, edits inside and outside a table, edits to validated cells, empty-string clears, oversized values that must error, and undo/redo between them. Compare the full snapshots, error strings, and undo/redo results after every step.
- [ ] **Step 2:** Extract the per-cell checks from `proposed` (address normalization, `parse_address`, the 10,000-change and 10,000-char limits) into a shared function, so both paths enforce identical rules. Add `tables::after_cells_is_local(&workspace, &changes) -> bool` for eligibility.
- [ ] **Step 3:** Replace the two full `validate()` calls on the fast path with validation of just the changed cells. Everything `validate` checks for a cell value must still be checked. Read `validate` (`workspace.rs:110`) and move the cell rules into a `validate_cell` helper that both paths call.
- [ ] **Step 4:** Route `cells` and eligible `enterCell` requests through `commit_cells`. `enterCell` must still run `data_validation::check` first, with identical results.
- [ ] **Step 5:** `cargo test` plus the benchmark. Expected: `proposed` and `validate` no longer appear for the benchmark edits. Commit: `Apply plain cell edits in place without cloning the workspace`.

### Task 8: Delta responses to Flutter

**Files:**
- Modify: `rust/src/workspace.rs` (`dispatch` result shaping, `snapshot`), `lib/src/core.dart` (`CoreBridge._dispatch`, `WorkspaceModel` snapshot updates)
- Test: create `rust/src/delta_tests.rs` and `test/delta_snapshot_test.dart`

**Interfaces:**
- Request: any mutating op may carry `"delta": {"baseRevision": n}`. If the engine can express the result as a delta from revision `n`, it returns:
  ```json
  {"delta": {
     "baseRevision": n, "revision": n2,
     "workspace": [<forward patch ops, serialized as {"op":"set"|"remove","path":[…],"value":…}>],
     "calculated": {"<sheetId>": {"A1": 2.0, "B1": null /* removed */}},
     "formatted":  {"<sheetId>": {"A1": {…}, "B1": null}},
     "set": {"canUndo": true, "canRedo": false, "evaluatedCells": 812, "calculationError": null,
             "conditionalWarnings": {…} /* only if changed */, "blankWorkspace": false}
  }}
  ```
  plus any extra top-level keys the op adds today (for example `documentHistoryChange`), unchanged. Otherwise (load, new, replace, `Structural` changes that touch many sheets, or a base-revision mismatch) it returns today's full snapshot. Callers that don't send `delta`, including every existing Rust and Flutter test, get full snapshots exactly as today.
- Dart: `WorkspaceModel` applies a delta to its `snapshot` map in place when `baseRevision` matches its current revision. On mismatch, it discards the delta and calls `{'op':'get'}`. The rest of the UI keeps reading `model.snapshot[...]` unchanged.

- [ ] **Step 1 (tests first, Rust):** In `delta_tests.rs`, run a scripted sequence covering every op the benchmark and the Flutter editing tests use, including undo and redo. For each op, request a delta; apply it to the previous full snapshot with a reference applier written in Rust in the test; and assert the result equals a fresh full `snapshot()`, ignoring `fileStates` (T6). Assert that a single-cell edit on the 100k-cell workload produces a response of < 16 KB.
- [ ] **Step 2:** Implement delta construction from the commit's forward patch (the inverse of the history entry), the calculator's `ChangedValues`, and the display change list. With `ChangedValues::All`, send full `calculated`/`formatted` for the affected sheets, or fall back to a full snapshot.
- [ ] **Step 3 (tests first, Dart):** In `delta_snapshot_test.dart`, use the real engine (as `perf_baseline_test.dart` does) and two `WorkspaceModel`s, one sending `delta` and one not. Run the same edit/undo/redo script on both and compare `snapshot` maps after every step, `fileStates` aside. Add a test that an injected revision mismatch triggers a full `get`.
- [ ] **Step 4:** Implement the Dart applier in `core.dart` and turn on `delta` for `WorkspaceModel.mutate` and undo/redo. The applier mutates nested maps in place; `workspace` getters that already copy (`Map.from`) keep working.
- [ ] **Step 5:** `cargo test`, `flutter analyze`, `flutter test`, `dart format --output=none --set-exit-if-changed lib test`. Commit: `Send edit results to Flutter as deltas`.

### Task 9: Re-measure, add CI guards, update docs

**Files:**
- Modify: `rust/src/perf_baseline_tests.rs`, `test/perf_baseline_test.dart`, `folio-native/docs/PERFORMANCE_BASELINE.md`, `docs/superpowers/plans/2026-09-23-folio-work-plan.md`

- [ ] **Step 1:** Make both harnesses exercise the path the app uses. The Rust harness sends `"delta"` and measures the api-equivalent response of the delta. The Flutter harness goes through `WorkspaceModel.mutate`, so Dart-side delta application is included. Keep a `full` variant of the Rust measurement for comparison with the Phase 1 numbers.
- [ ] **Step 2:** Add structural CI guards to the default `cargo test` suite (no timing). On the 20×2,500 workload built once in the test: a single-cell edit's delta response is < 16 KB; the pushed undo entry's `bytes` is < 4 KB; the calculator took `update_cells` (assert through a test-only counter of full rebuilds, which stays at 0 across 20 edits); the display formatter formatted < 100 cells per edit. This keeps later work from silently bringing back O(workbook) edits.
- [ ] **Step 3:** On the Phase 1 baseline machine (Apple M4 Max, same OS and toolchains, release build, median of three runs), run both harnesses and record in `PERFORMANCE_BASELINE.md` a *Phase 2* table next to the Phase 1 table: engine edit p50/p95, response p50/p95, engine + response p50/p95, undo p95, first10 → last10, response bytes, Flutter round trip p50/p95, and the per-stage p95s including the `calc.evaluate` floor. Record what was measured, not what was expected.
- [ ] **Step 4:** Check the exit gate:
  - engine + response p95 ≤ 50 ms for a single-cell edit;
  - undo p95 ≤ 50 ms;
  - `first10Avg` → `last10Avg` within 10%;
  - all existing tests pass.

  If the gate is missed, the stage table shows where; fix that before closing the phase. Record the Flutter round trip (JIT) against the ROADMAP target, but it is not the gate (see Phase 1's note on JIT vs AOT).
- [ ] **Step 5:** Update the program plan's Phase 2 row to link this plan and state the outcome. Add the M3 follow-ups found here: incremental IronCalc recalculation, per-sheet conditional-format re-evaluation, and any fallbacks that turned out to be common. Commit: `Record Phase 2 edit-path performance`.

## Risks

- **Silent behavior drift in fast paths.** Mitigated by the differential tests in T2, T4, T5, T7 and T8, which compare against the unchanged full path on seeded random sequences. The full paths stay in the code as fallbacks and test oracles.
- **Spills and volatile functions** can change cells that weren't edited. T4 reads back every formula cell and every spill range after `evaluate()`, not just the edited cells, so these are caught; its differential test includes both.
- **Undo semantics.** Revision numbering and `documentHistoryChange` payloads are part of the Flutter contract. T2 Step 6 compares them step by step against the old implementation.
- **The delta contract adds a second response shape.** It is opt-in per request, a revision mismatch always falls back to a full `get`, and the Dart test compares delta-driven and full-snapshot models directly.

## Progress and findings (2026-09-24)

| Task | State | Notes |
|---|---|---|
| 1. Stage timers | Done | Also added `undoMs` and `layoutEditMs` (see finding 1). |
| 2. Patch-based undo | Done | Out-of-band row-size changes re-base the adjacent undo/redo entries (`History::rebase`). |
| 3. Change set | Done | |
| 4. Incremental calculator | Done | Full and incremental paths share one routine driven by a changed-input list. No IronCalc accessor was needed: published values only cover input cells, so reading back edited and formula cells is complete. |
| 5. Incremental display + CF | Done | CF skips sheets without rules exactly (early return in `conditional_format::evaluate`). |
| 6. File states on demand | **Blocked on a decision** | See finding 2. |
| 7. In-place cell fast path | Done | Covers `cells`, `enterCell`, `validationEntryCheck`. Workbooks with any table take the general path, because `tables::after_cells` can rewrite table metadata. The document-normalization check is cached. |
| 8. Delta responses | Not started | Depends on the layout-path decision below. |
| 9. Re-measure and CI guards | Not started | |

### Container measurements

Measured with the Rust harness, release build, on a 4-vCPU Linux container. That machine is about 6× slower than the M4 Max baseline (Phase 1 engine + response p95: 381.6 ms on the Mac vs 2,251 ms here). These numbers show direction only; the exit gate is measured on the Mac.

| Measure (ms) | Before (after T1) | After T4 | After T7 |
|---|---:|---:|---:|
| Engine edit p50 / p95 | 1,847 / 2,022 | 851 / 996 | 275 / 310 |
| Engine + response p50 / p95 | 2,077 / 2,251 | 1,044 / 1,205 | 466 / 527 |
| First 10 → last 10 edits (avg) | 816 → 1,881 | 753 → 819 | 276 → 264 |
| Undo p95 | 1,798 | 1,152 | 438 |
| Layout edit path p50 / p95 (what the app does) | not measured | 3,776 / 4,182 | 2,445 / 2,764 |
| Response size | 7.76 MB | 7.76 MB | 7.76 MB |

Largest remaining direct-path stages after T7 (p95): snapshot build 106, IronCalc `evaluate` 73 (the floor), document check 70 (since cached), calculator read-back 47, file-state digests 23.

### Finding 1: the app's edit path is the layout path, and the plan didn't cover it

`WorkspaceModel.mutate` sends `cells`, `enterCell` and 14 other ops through `prepareLayoutEdit` → Dart row measurement → `commitLayoutEdit`. `prepareLayoutEdit` clones the workspace into a staged engine, syncs a second calculator that lags one edit behind, runs the edit there, computes row targets by scanning every cell (`row_reflow::targets`), and returns a full snapshot. `commitLayoutEdit` compares whole workspaces twice, validates twice, diffs, and returns another full snapshot. After T7 this path is ~5× the direct edit path. The Phase 1 benchmarks measured only the direct `cells` op.

Proposed new task (7b), keeping the rule that measuring never publishes an edit:
- Store the lagging calculator's changes since it was swapped out, and pass them to the staged sync as a `ChangeSet::Cells` hint. Keep its conditional-format state with it.
- Replace the `base` clone and compare with a workspace epoch counter bumped by `Core::sync`.
- Compute row targets from the change set (edited ∪ recalculated ∪ re-rendered cells), with the same comparisons `targets` makes today, plus a differential test against `targets`.
- Commit the staged patch directly instead of re-validating and re-diffing whole workspaces.

The alternative is to commit first and then apply measured row sizes as an out-of-band change, the way calculation-only layout edits already work. That is simpler and faster, but it drops the "measuring must not publish" rule that `prepareLayoutEdit`'s tests encode.

### Finding 2: file-state digests are read on every frame

`fileSaveStatus` ("Saved · name" / "Changes saved locally · name") is a synchronous getter that compares `snapshot['fileStates']` content signatures. So the digests are needed after every edit, not only on save or close. The options are:
- **(a)** Fetch them after edits with a short debounce. The status text can lag an edit by up to the debounce.
- **(b)** Replace the SHA-256 over serialized sheets with an incrementally updatable signature (for example a sum of per-cell hashes). This changes the persisted signature format, so every previously saved file shows "Changes saved locally" once, until it is next saved.

### Other notes

- Undo entries are now sized by their patches, so large workbooks keep the full 100-step history. Previously each entry was a full workspace copy under a 24 MB cap, which allowed only about 10 steps at the benchmark size.
- The Flutter suite also runs on Linux here (Flutter 3.44.7, `FOLIO_CORE_PATH` pointing at `libfolio_core.so`): 380 pass, 14 fail. The failures are in font-measurement and macOS-variant widget tests. I haven't checked whether they also fail on `main` under Linux; macOS CI is the authority.
