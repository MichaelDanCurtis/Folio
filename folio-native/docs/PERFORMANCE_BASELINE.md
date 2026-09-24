# Performance baseline

Measured 2026-09-23 on Apple M4 Max, 128 GiB, macOS 27.0; rustc 1.98.1, Flutter 3.44.7; commit 21f63e7. Release build. Median of three runs.

Workload: 20 sheets × 2,500 rows (A = number, B = `=A{r}*2`), 100,000 populated cells. Each edit changes Sheet 1 A1.

| Measure | Result | ROADMAP §7 target |
|---|---|---|
| Load workspace (`replace`) | 382.4 ms | open ≤ 5 s |
| Engine edit p50 / p95 (200 edits) | 292.8 / 304.2 ms | typing p95 ≤ 50 ms |
| Engine edit, first 10 avg → last 10 avg | 238.1 → 297.2 ms | flat |
| Serialize response p50 / p95 | 20.3 / 21.1 ms | — |
| Response size per edit | 7.76 MB | — |
| Flutter round trip p50 / p95 (50 edits) | 448.6 / 461.2 ms | typing p95 ≤ 50 ms |

## Reproduce

    cd folio-native/rust && cargo test --release perf_baseline -- --ignored --nocapture
    cd folio-native && FOLIO_PERF=1 flutter test test/perf_baseline_test.dart

## Known causes (for Phase 2)

- `Core::commit_journal` (`rust/src/workspace.rs`) clones the full workspace into each undo entry and then re-serializes every undo entry to measure total history bytes, so commit cost grows with history length.
- `Core::snapshot` returns the full workspace plus all calculated and formatted values after every edit; `CoreBridge` decodes it in full.
