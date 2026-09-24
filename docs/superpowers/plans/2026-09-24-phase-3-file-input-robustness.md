# Phase 3: File-Input Robustness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A malformed, truncated or hostile DOCX, XLSX, template or `.folio` file produces an error message, never a crash, a hang or runaway memory, and never leaves the engine unusable.

**Exit gate (from the program plan):** the import, export and recovery paths have no reachable `unwrap()`/`expect()`/indexing panics, and a fuzz/corpus test of truncated and mutated packages runs in CI.

**Architecture:** The work is driven by a fuzzer, not a count of unwraps. There are 688 `unwrap()`/`expect()` calls in non-test Rust, and most operate on data the engine itself validated. Grepping can't tell which ones input can reach; a fuzzer can. The fuzzer builds a corpus from the repository's own fixtures (53 Office files and JSON workspaces). It mutates them at three levels: raw bytes, package parts, and the XML inside parts. It then drives each result through every entry point that accepts a file, plus a short follow-up session: export, edit, undo, save and reload. Every panic is recorded with its source location. Each distinct panic site gets a fix and a regression case. Behind that, the FFI boundary gets a safety net, so a panic the fuzzer never found still returns an error and leaves the engine usable.

**Tech Stack:** Rust 1.98.1 (`std::panic::catch_unwind`, a panic hook for locations), `zip` and `roxmltree` (already dependencies), no new crates.

## Global Constraints

- All commands run from `folio-native/rust` unless stated otherwise.
- No behavior change for valid files. The existing Rust suite (429 tests) and Flutter suite must pass unchanged after every task.
- Error messages for bad files must be user-readable, in the style of existing ones: "This file is not an unencrypted DOCX/XLSX Office package." Don't expose Rust panic text to users.
- The fuzz test in the default suite must be deterministic (fixed seeds) and take under 60 s in a debug build, so CI stays usable. A longer opt-in run is `#[ignore]`d.
- Resource limits: no single file operation may take longer than 10 s or allocate more than ~1 GB on the fuzz corpus. The fuzzer reports slow cases as failures, the same as panics.

## Entry points that accept external bytes

| Op | Input | Code |
|---|---|---|
| `importOffice` | DOCX/XLSX/DOTX/XLTX path | `office::Package::read` → `docx::import` / `xlsx::import` and the `docx_*`/`xlsx_*` readers |
| `inspectTemplate`, `newFromTemplate` | template path | `office_templates` |
| `load`, `recoverWorkspace`, `inspectWorkspace` | `.folio` JSON, including embedded base64 packages | `workspace.rs` load arm, `validate`, `Package::from_bytes` |
| `pasteCells` | clipboard package (Folio's own export) | `sheet_clipboard` |
| `inspectScenarioSource`, scenario merge | external XLSX | `scenario_source` |

Export (`exportOffice`, `save`) must also survive state reached from any of those imports.

---

### Task 1: Panic safety net at the FFI boundary

**Files:** `rust/src/api.rs`, `rust/src/workspace.rs`, test in `rust/src/api_tests.rs` (new)

- [ ] Wrap `Core::dispatch` in `api::dispatch` in `catch_unwind(AssertUnwindSafe(..))`. On a panic, return `{"ok":false,"error":"Folio hit an internal error and did not finish this action. Your work is unchanged; please report this file."}`.
- [ ] Recover a poisoned mutex with `PoisonError::into_inner` instead of failing every later call.
- [ ] After a caught panic, restore a consistent engine: `Core::recover_after_panic()` drops any pending layout edit and staged state, then re-syncs (`sync(&ChangeSet::Structural)`) so the calculator, display and caches match the workspace. The workspace itself is only replaced at the end of an op, so a panic mid-op leaves the previous one. Where an op edits in place (the Phase 2 cell fast path), check whether a partial edit can survive a panic, and if so, snapshot the edited cells so they can be restored.
- [ ] Install the panic hook once so panics are still logged to stderr with their location.
- [ ] Test: a test-only op that panics mid-dispatch. The next call succeeds, returns the unchanged workspace, and edits and undo still work.

### Task 2: Deterministic fuzz harness

**Files:** `rust/src/fuzz_tests.rs` (new)

- [ ] **Corpus:** every `test/fixtures/*.{docx,xlsx,dotx,xltx}`, plus a `.folio` produced by `save` after importing one DOCX and one XLSX (so embedded packages are covered).
- [ ] **Mutators** (seeded LCG, each records a one-line description):
  - byte level: truncate at a random offset; flip 1–8 random bytes;
  - package level: delete a random part; duplicate a part under another name; replace a part with empty bytes;
  - XML level (parse the part with `roxmltree`, re-serialize a mutated version, re-zip): truncate the text; delete a random element; duplicate a random element; set a random attribute to one of `""`, `"-1"`, `"0"`, `"4294967296"`, `"99999999999999999999"`, `"NaN"`, `"XFD1048576"`, `"A1:XFD1048576"`, `"🙂"`, or 5 000 × `"A"`; remove a random attribute;
  - `.folio` level: delete a random JSON field; change a field's type; replace a base64 package with garbage.
- [ ] **Driver:** for each case, call the matching entry point: `importOffice` for documents, both `inspectTemplate` and `newFromTemplate` for templates, `load` and `recoverWorkspace` for `.folio`. If the call succeeds, run a short session: `get`, `exportOffice` to a temp path (both formats where applicable), one `cells` edit, `undo`, `save` then `load`. Each call runs under `catch_unwind` with a panic hook that captures `file:line`. Time each case.
- [ ] **Report:** failures grouped by panic location, each with the fixture, mutation and op. The default-suite test runs a fixed seed set (target ~400 cases) and fails on any panic or case over 10 s. `#[ignore]`d `fuzz_long` takes `FOLIO_FUZZ_CASES` and `FOLIO_FUZZ_SEED`.

### Task 3: Fix every panic site the fuzzer finds

- [ ] For each distinct panic location, change the code to return a readable error instead. Don't mask panics with `catch_unwind` inside parsers; fix the reachable `unwrap`/`expect`/index.
- [ ] Add a named regression case for each fixed site (the minimized fixture mutation) to `fuzz_tests.rs`.
- [ ] Repeat the long run (`FOLIO_FUZZ_CASES=20000`) with several seeds until it finds nothing.

### Task 4: Resource limits

- [ ] **Decompression:** `Package::from_bytes` checks each part's declared size, but a zip entry can lie. Cap bytes actually read per part and in total, with a `Read::take`-style limit, and test with a crafted entry whose declared size is smaller than its content.
- [ ] **Range explosion:** find every loop over a cell range taken from file content (dimensions, merged cells, `sqref`s for conditional formatting and data validation, table refs, print areas, defined names, autofilter) and bound it by the populated cells or an explicit cap. The fuzzer's `XFD1048576` mutations should catch these as timeouts. Add a targeted test per loop.
- [ ] **XML depth and size:** confirm `roxmltree`'s nesting limit applies, and cap the text size of any part parsed into memory.

### Task 5: Close out

- [ ] Run the local CI steps (Rust format and tests, release build, Dart format, `flutter analyze`, Flutter tests) and compare the result with the Phase 2 baseline: 389 passed, 7 Linux-only failures.
- [ ] Update this plan's progress section, and the program plan's Phase 3 row, with what was found and fixed. Include the count of distinct panic sites and the long-run case counts.
