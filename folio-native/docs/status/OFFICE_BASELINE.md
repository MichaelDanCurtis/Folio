# Office reference baseline

ROADMAP §2 requires a frozen, versioned comparison baseline before any requirement can be "Office compared". **Status: not recorded.** The macOS machine, OS and locale were recorded on 2026-09-26, but Word and Excel are not installed on that Mac yet. It needs the Microsoft 365 installs, which only the project owner has (program plan, "Dependencies that need you" 1). Until it is filled in, the register's highest reachable state is "Native verified".

File-level evidence from real Office output comes from the [Office-created corpus](CORPUS.md#office-created-corpus-real-files) in the meantime. Each file records the Word/Excel build that saved it, and workbooks carry Excel's own calculated results. That covers opening, round trips and calculation. Interaction, layout and printing still need the installs below.

## What to record (once per milestone)

Fill in one column per reference machine. The Windows column is the breadth reference; the macOS column is the native interaction reference.

| Field | macOS reference | Windows reference |
|---|---|---|
| Machine and CPU | Mac16,5, Apple M4 Max, 128 GiB | |
| OS version and build | macOS 27.0 (26A428) | |
| Word version and build | Not installed (checked 2026-09-26) | |
| Excel version and build | Not installed (checked 2026-09-26) | |
| Update channel | | |
| Licence / plan | | |
| Copilot capabilities enabled | | |
| UI language and locale (region, date and number formats) | English (en-US), region US; dates M/d/yy and h:mm a; numbers 1,234,567.89; currency USD (−$1,234.50); Gregorian, week starts Sunday; US measurement; America/Los_Angeles | |
| Installed fonts relevant to the corpus | | |
| Add-ins and extensions installed | | |
| Date recorded, and who recorded it | 2026-09-26, native checks ([NATIVE_CHECKS_2026-09](../NATIVE_CHECKS_2026-09.md)); Office rows still empty | |

To collect these:
- **Word/Excel:** Word or Excel → About (macOS), or File → Account → About (Windows).
- **OS:** `sw_vers` (macOS), `winver` (Windows).
- **Locale:** System Settings → Language & Region, or the Windows Region settings.

Freeze the baseline for each milestone. A later Office update starts a new column set and a line in the STATUS.md change log. The earlier results stay tied to the build that produced them.

## Folio reference machine

Performance budgets (ROADMAP §7) are measured on the same machine class as the Office runs.

| Field | Value |
|---|---|
| Machine | Apple M4 Max, 128 GiB ([PERFORMANCE_BASELINE](../PERFORMANCE_BASELINE.md)) |
| OS | macOS 27.0 |
| Toolchain | rustc 1.98.1, Flutter 3.44.7 |
| Recorded | 2026-09-23 (Phase 1) |

ROADMAP §7 names "an Apple Silicon Mac with 16 GB" as the first reference class. The M4 Max is faster and has far more memory than that. Memory budgets and slow-machine latency need a 16 GB run as well, or an explicit decision that the M4 Max is the reference.

Which §7 budgets are measured:

| Budget | Status |
|---|---|
| Engine edit on the 100k-cell workload | Measured (Phase 1 on the M4 Max; Phase 2 on the Linux container only) |
| Workspace load (engine) | Measured (Phase 1) |
| Cold launch, key-to-visible, scroll, open/save in the app, memory, cancellation, recovery | Not measured |

## Comparison procedure

For each child requirement being compared:
1. Run the scenario from its fixture in Word/Excel, then in Folio.
2. Record the Office build (from this file), the fixture's hash, and the outputs: saved files, screenshots or PDF pages, calculated values.
3. List the differences. Mark each as an intended difference, a Folio defect, or an Office behaviour Folio doesn't need to match (with a reason).
4. Set the child to "Office compared" only when there is no Folio defect left.
