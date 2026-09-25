# Folio work plan

Planning date: 2026-09-23. Baseline: `main` at `c37552d` (0.82.0 preview).

## Goal

Get Folio as close as possible to full functional and file-compatibility parity with Microsoft Word and Excel, as defined by [ROADMAP.md](../../../folio-native/ROADMAP.md) and [PARITY.md](../../../folio-native/PARITY.md). The daily report-and-budget workflow is a checkpoint along the way, not the finish line. All 46 parity families stay in scope.

Before resuming roadmap feature work, fix the foundations that would otherwise cap how far parity can go: automated verification, measured performance, the per-edit copying cost, and crash safety on file input.

## Why the foundations come first

1. **No CI.** Five tests were broken on every machine except the original one, and nothing noticed. Parity work will touch hundreds of files, so each change needs an automatic check.
2. **The edit path can't scale.** Every committed edit (`folio-native/rust/src/workspace.rs:619-640`) clones the whole workspace onto the undo stack. It then re-serializes up to 100 undo entries to measure their size, and returns the entire workspace plus every calculated value as JSON (`workspace.rs:413-421`), which Flutter decodes in full (`folio-native/lib/src/core.dart:48-54`). Excel-scale workbooks (ROADMAP §7: 20 sheets / 100k cells now, one million cells at M3) are out of reach until this changes.
3. **Input robustness.** About 680 `unwrap()` calls in non-test Rust code. Parity means opening arbitrary real-world Office files, so a malformed file must fail cleanly instead of crashing the app.
4. **UI structure.** `folio-native/lib/main.dart` holds one ~4,400-line `State` class. Parity work adds hundreds of commands, so extract areas as they're touched (never as a standalone rewrite).

## Phases

Each phase gets its own detailed plan when it starts, because each depends on what the previous one finds.

| Phase | Outcome | Exit gate | Plan |
|---|---|---|---|
| **1. Safety net and measurement** | CI runs every Rust and Flutter check on each push/PR; a repeatable performance baseline exists for the edit path | CI green on `main`; baseline numbers recorded in `folio-native/docs/PERFORMANCE_BASELINE.md` | [Phase 1 plan](2026-09-23-phase-1-ci-and-baseline.md) |
| **2. Edit-path performance** | Edits cost time proportional to what changed, not to workbook size | Baseline benchmark shows p95 single-cell edit (engine + serialization) ≤ 50 ms and undo accounting O(1) per commit on the 100k-cell workload; all existing tests pass | [Phase 2 plan](2026-09-24-phase-2-edit-path-performance.md) |
| **3. File-input robustness** | Malformed or hostile DOCX/XLSX input produces an error, never a crash | No `unwrap()`/`expect()`/indexing panics on the import, export and recovery paths; a fuzz/corpus test of truncated and mutated packages runs in CI | [Phase 3 plan](2026-09-24-phase-3-file-input-robustness.md) — done: 5 crash/hang classes fixed, fuzz test in the default suite |
| **4. M0: parity inventory** (ROADMAP M0) | All 46 families decomposed into bounded child requirements with IDs and acceptance states; Office reference baseline recorded; comparison fixture corpus | `folio-native/docs/STATUS.md` register covers 46/46 families; every existing feature mapped to child IDs; ranked gap list with effort ranges | [Phase 4 plan](2026-09-25-phase-4-parity-inventory.md) — register done ([STATUS](../../../folio-native/docs/STATUS.md)); Office baseline waits on you |
| **5. M1 → M5** (ROADMAP milestones) | Parity delivered family by family: daily editing, then professional documents, analytical workbooks, advanced breadth, full parity | Per ROADMAP §3 exit gates; extract `main.dart` areas as each is touched | One plan per delivery package |
| **6. M6: AI differentiation** | 20 verified novel AI features | Per ROADMAP §8 | After parity |

### Phase ordering notes

- Phase 3 can run alongside Phase 2 because they touch different code (import/export versus commit/snapshot).
- Phase 4 (M0) needs no engine changes and can start any time after Phase 1. Doing it before M1 feature work keeps parity work focused on named requirements instead of isolated commands, which ROADMAP §1 identifies as the current failure mode.
- Postponed on purpose: rewriting the workspace from `serde_json::Value` to typed Rust structs. Revisit only if the Phase 1 baseline shows JSON manipulation, rather than copying or serialization, is the bottleneck.

## Dependencies that need you

These block specific acceptance states, not the phases above. Work continues around them.

1. **Office reference installs.** The "Office compared" acceptance state needs Microsoft 365 desktop Word and Excel: macOS (you likely have this) and, for Windows-only features, a Windows machine or VM. Needed from Phase 4 onward.
2. **Platform scope.** Folio is macOS-only today; ROADMAP names Windows and Linux as delivery targets. Parity with Windows-only features (VBA, Power Pivot, some connectors) needs a decision on Windows timing. Needed before M4.
3. **Commercial dependencies.** Some parity areas (font licensing, a VBA runtime, data connectors) may need paid or licensed components. ROADMAP §7 requires a cost decision before committing to them.

## Working rules for all phases

- Branch from latest `main` for each package; one PR per package; CI must be green before merge.
- Record measured results, not assumptions. Distinguish checks run now from historical evidence (ROADMAP §9).
- Don't add another isolated command unless it closes a named child requirement (after Phase 4).
