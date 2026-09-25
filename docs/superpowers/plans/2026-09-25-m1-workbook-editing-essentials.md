# M1 package: Workbook editing essentials

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the everyday Excel editing gaps that stop the M1 budget workflow from feeling like Excel. The package is rank 3 in [STATUS.md](../../../folio-native/docs/STATUS.md#ranked-gap-list).

**Exit gate:** each child below reaches "Automated verified". Each has:
- engine tests for any engine operation;
- widget tests through the real engine;
- Undo/Redo, save/reopen and XLSX export/reimport evidence where the feature changes the file.

Native qualification joins the rank 2 Mac sweep. The register rows and counts are updated with each task.

**Children:**
- X01.03 row/column/sheet selection and data-edge navigation
- X02.04 name box
- X02.05 Go To
- D01.06 status-bar aggregates
- X01.17 Clear menu
- X04.07 borders
- X04.08 fill colour
- X05.03 zoom
- X03.06 duplicate sheet
- X01.06 formula point mode
- X01.13 fill handle

**Constraints:**
- New UI goes in new files under `lib/src/`, not into `sheet_view.dart` (4,100 lines) or `main.dart`. Those only get the wiring (program plan: extract areas as they are touched).
- Excel behaviour is the reference. Where Excel's exact behaviour isn't documented and can't yet be compared, the chosen behaviour is written in the task notes. The child stays below "Office compared" until the comparison is made.
- Every workbook change goes through an engine op, with history, recovery and XLSX export.

---

### Task 1: Data-edge navigation and whole row/column/sheet selection (X01.03)

**Files:** `lib/src/sheet_navigation.dart` (new, pure functions), wiring in `sheet_view.dart`, `test/sheet_navigation_test.dart`

- [x] **Ctrl+arrow (Command+arrow on Mac) moves to the data edge**, as Excel does:
  - from a filled cell whose neighbour is filled, go to the last filled cell before a blank;
  - otherwise go to the next filled cell;
  - with no filled cell ahead, go to the sheet edge.

  Hidden rows and columns are skipped. Add Shift to extend the selection.
- [x] **Ctrl+Home goes to A1** (or the first unfrozen cell); **Ctrl+End goes to the last used row and column**; **Home goes to column A of the current row**.
- [x] **Selection shortcuts:** Shift+Space selects the row, Ctrl+Space the column, and Ctrl+A the current region, then the whole sheet on a second press.
- [x] **Tests:** unit tests for the edge rules (gaps, hidden rows, sheet edges, single cells), plus widget tests through the grid.

### Task 2: Status-bar aggregates (D01.06)

**Files:** `lib/src/sheet_status.dart` (new), status bar wiring, tests

- [x] For a multi-cell selection, show Average, Count and Sum, like Excel's defaults. Numerical Count, Min and Max can be turned on from the status-bar context menu, and the choice persists.
- [x] Use calculated values, and skip text, blanks and errors for Sum and Average.
- [x] Hidden rows: Microsoft's documentation doesn't say whether they count. Start with visible cells only, as SUBTOTAL 1xx does, and list this as an open point for the Excel comparison.
- [x] Update after edits and recalculation. Work on huge selections by walking only populated cells.

### Task 3: Name box and Go To (X02.04, X02.05)

**Files:** `lib/src/sheet_name_box.dart`, `lib/src/sheet_go_to_dialog.dart` (new), tests

- [ ] The name box shows the active cell or selection size while dragging.
- [x] Typing an address, a range, `Sheet!A1` or a defined name and pressing Enter selects it.
- [x] An invalid entry shows an error and restores the box.
- [x] Go To (F5 / Ctrl+G) lists defined names and recent Go To targets, and accepts a reference.
- [x] Defining a name from the name box waits for the Name Manager (X09.05). Split X02.04 so that part is its own child.

### Task 4: Clear menu (X01.17)

**Files:** engine op `clearRange` in the Rust core (`rust/src/sheet_clear.rs`), menu wiring, tests

- [x] **Clear All, Formats, Contents, Comments and Notes, and Hyperlinks**, on the selection.
- [x] **Formats** removes the cell style (to the row/column default), without touching values, validation or conditional formats.
- [x] One Undo step. Export writes the result to XLSX.
- [x] Also give Delete/Backspace (X01.18) its own tests.

### Task 5: Borders and fill colour (X04.07, X04.08)

**Files:** engine style op extension (`rust/src/xlsx_styles.rs` or a new `sheet_borders.rs`), a grid painter for borders and fills, `lib/src/sheet_borders.dart` (new), tests

- [x] **Border presets** (bottom, top, left, right, all, outside, thick box, none), plus line style and colour.
- [x] **Fill colour** from a palette, including No Fill.
- [x] **XLSX `border`/`fill` records added or reused**, without rewriting other XF entries (as the font edits do).
- [ ] **The grid paints imported and edited borders and fills**, including at frozen-pane edges and next to merges. Imported borders are painted even for files never edited in Folio.
- [ ] **Tests:** engine, rendering pixel tests, round trip and history.

### Task 6: Zoom (X05.03)

**Files:** `lib/src/sheet_zoom.dart` (new), grid scale plumbing, tests

- [x] **Zoom 10–400%:** a status-bar slider, Zoom to Selection, and Command/Ctrl with + and −.
- [x] Keep the zoom per sheet, read from and written to `sheetView/@zoomScale` in XLSX.
- [x] Hit testing, selection and editing stay correct at every zoom level.

### Task 7: Duplicate sheet (X03.06)

**Files:** engine op `duplicateSheet` (`rust/src/sheet_duplicate.rs`), tab context menu, tests

- [x] Copy the sheet's cells, styles, dimensions, merges, validation, conditional formats, notes and view.
- [x] Give copied tables new unique names and adjust their structured references.
- [ ] Copy sheet-scoped names, pointing at the copy.
- [x] Insert the copy after the source, with an Excel-style name ("Budget (2)").
- [x] One Undo step; round trip through XLSX.

### Task 8: Formula point mode (X01.06)

**Files:** `lib/src/sheet_formula_point.dart` (new), editor wiring, tests

- [ ] While a formula is being typed, and the caret is where a reference can go (after `=`, an operator, a comma or `(`), arrow keys and clicks insert or replace a reference; Shift extends it to a range.
- [ ] F2 toggles between Edit and Point mode, and the status bar shows the mode.
- [ ] Clicking another sheet's tab inserts a `Sheet!A1` reference.

### Task 9: Fill handle (X01.13)

**Files:** `lib/src/sheet_fill_handle.dart` (new), grid wiring, tests

- [ ] **Dragging the fill handle** fills down, right, up or left using the existing fill op (X01.12). Series (X01.14) stays separate.
- [ ] **Double-clicking the handle** fills down to the end of the adjacent data column.
- [ ] **Dragging back inside the selection** clears the cells released.

### Task 10: Close-out

- [ ] Update the register rows, STATUS.md counts and the change log.
- [ ] Run the local CI mirror. Compare with the Phase 2 baseline: 7 Flutter tests fail on Linux only.
