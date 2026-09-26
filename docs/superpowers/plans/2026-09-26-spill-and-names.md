# Dynamic-array spill and the Name Manager (STATUS rank 18)

**Goal:** show dynamic-array results where they spill (X09.02), and let people manage defined names, including named LAMBDAs (X09.04, X09.05).

### Task 1: Spilled values in the grid
- [x] After each calculation, each dynamic formula's spill range comes from the engine (`ArrayFormula` with `ArrayKind::Dynamic`). Its cells' values are published like a CSE array's followers.
- [x] A spill that shrinks clears the cells it no longer fills. A value in the way shows #SPILL! and removes the spill.
- [x] Snapshots and deltas carry `spills` (`{sheetId: {anchor: range}}`).
- [x] In the grid, a blue outline marks the active cell's spill. A spilled cell shows the anchor's formula greyed in the formula bar.

### Task 2: The Name Manager
- [x] A `nameManager` op: `list` gives each name with its scope, its formula as typed, and its value. `add`, `edit` and `delete` apply one change.
- [x] Each change is checked by a trial model: valid name, unique in its scope, formula parses. It is then written into the workbook package's `definedNames`, which structural edits already keep current.
- [x] Unqualified references get the active sheet, as in Excel.
- [x] A workbook made in Folio gets a package on its first name.
- [x] Formulas → Name Manager: a list with New…, Edit… and Delete, and a dialog for name, scope and formula that shows the engine's reason when a change is refused.

### Task 3: Close-out
- [x] Rust tests: spill, shrink, collision and undo; add, edit, delete, scope, LAMBDA, export and row insertion.
- [x] A widget test for the outline, the formula hint and the Name Manager.
- [x] Register: X09.02, X09.04 and X09.05.
