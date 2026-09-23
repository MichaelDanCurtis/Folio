# Formula inspector

Version 0.78 adds a read-only Formula inspector to the worksheet **Cell actions**
menu and cell context menu. Select a formula or input cell and choose
**Formula inspector…**. It shows its formula/input and calculated value, direct
precedents, direct dependents across worksheets, and unresolved formulas.
**Inspect** follows another cell inside the dialog; Back retraces that path.
**Go to cell** or **Go to range** selects the exact source in its worksheet.
Range inspection explicitly starts with its first cell. Lists load in pages of
100 without expanding whole-row or whole-column ranges into individual cells.

The calculation model's existing parsed AST is authoritative. Relative and
absolute references resolve against the formula cell; duplicates are removed,
and range containment finds dependents even when the selected cell is empty.
Qualified sheet names, structured table columns and this-row references share
the calculation engine's parsing. Literal strings and escaped formula-like text
are not treated as references. The index is cached for the calculation generation
and discarded after edits, table changes, undo/redo, loading or recalculation
that changes the model. Reading does not change history or cancel staged row
layout edits.

This is a syntactic trace: every conditional branch is included. INDIRECT,
OFFSET, calculated range endpoints, spilled ranges, unresolved names, bare
table names and parse/reference errors report explicit limitations. The
Unresolved tab identifies affected formulas across the workbook, because even
an ordinary input cell may have untraced dependents. Dynamic targets are not
silently presented as a complete dependency graph. Native arrows, exact runtime
dependency tracing, full named-formula handling, Evaluate Formula, full Watch
Window parity, protected/hidden-formula semantics and external workbook traversal
remain open. This is partial progress in X10, not family acceptance or one of
the 20 novel AI features.

Version 0.79 adds the initial persistent [Watch Window](WATCH_WINDOW.md), with
an Inspect action that opens this dialog for the watched cell.

Desktop/UI safety: opening from Cell actions first commits the pending cell
through its normal validation path. Rejected edits stay editable. The dialog
pins workspace identity, revision, document epoch and calculation generation.
A changed workbook disables navigation and discards outstanding reads; the
caller rechecks before and after switching worksheets. Selection explicitly
sets range intent instead of depending on a physical Shift key.

## Verification

- Seven Rust cases cover mixed references, quoted sheet names, sparse bounds,
  literal text, conditional branches, circular/self references, deduplication,
  paging, dynamic and broken-reference reporting, structured-table extents,
  edit/history invalidation, stale requests and staged-layout preservation.
- Widget cases cover paging, inspecting/back, exact destinations, unresolved
  notices, failed-read retry and rejection of a late response after a change.
- A real Flutter/Rust workspace scenario opens the visible menu, selects a
  cross-sheet range, commits a draft, traces a changed total, rejects stale
  navigation, undoes the inputs, saves/reloads recovery and exports/reimports
  XLSX. It can capture a rendered widget image with `FOLIO_CAPTURE_PREVIEWS=1`.
- `scripts/verification/verify_formula_audit.py` reads the recovery and XLSX
  independently. Microsoft Open XML SDK checks the generated workbook against
  Microsoft 365 schemas. Neither check executes Microsoft Excel.

The packaged 0.79 preview now verifies opening from a watched formula, following
its first source cell, displaying the source cell's direct dependent, returning
with Back and selecting the exact cross-sheet source range. The watched formula
also updates after typed source entry and Undo. Native evidence is archived in
`release/validation/watch-window-native`; the broader dynamic, structured,
stale-read and accessibility cases remain separate from these native checks.

Packaged desktop verification remains separate from automated checks.
Keep one preview process: inspect its saved state, quit normally, verify zero,
launch the exact prepared replacement, then verify one. A locked Mac or missing
window is not a stopped process and must never trigger an extra launch.

Baseline: Microsoft's [formula relationships](https://support.microsoft.com/en-us/excel/display-the-relationships-between-formulas-and-cells)
and [precedents/dependents API](https://learn.microsoft.com/en-us/office/dev/add-ins/excel/excel-add-ins-ranges-precedents-dependents)
describe direct tracing and following relationships across sheets. Full Excel
workflow and native visual comparison remain unverified.
