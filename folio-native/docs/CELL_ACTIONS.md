# Desktop cell actions

The worksheet's **Cell actions** button beside the formula bar opens ordinary
desktop menu items:

- **Edit cell** selects the active cell alone and focuses its editor. A pending
  or rejected cell/formula save must finish successfully before another edit
  starts. A rejected draft remains available for correction or retry.
- **Extend selection** toggles range selection without holding Shift. Click or
  activate a destination cell, or use the arrow keys. The existing range anchor
  stays in place. A visible banner identifies the mode.
- **Select active cell** finishes range mode and selects only the active cell.

**F8** toggles worksheet extend mode when the grid has focus. **Escape** and
**Finish selection** leave the selected range in place and return to ordinary
selection. F8 inside a text editor does not change the grid's mode. Moving
between worksheets/workspaces resets the mode. Hidden rows/columns continue to
use the existing navigation rules. Noncontiguous selection and Shift-F8 are
still unfinished. The F8 baseline comes from Microsoft's
[Excel keyboard shortcuts](https://support.microsoft.com/en-us/accessibility/excel/keyboard-shortcuts-in-excel).

The document table editor has a direct **Edit cell** button beside the active
row/column label. It ends Select cells mode, collapses the selection to that
cell and focuses the existing rich-text editor without replacing its draft.
The existing **Select cells** control remains the explicit table range route.

These are standard visible controls and standard accessibility tap actions.
They do not depend on the Flutter Mac bridge's unsupported custom-action
dispatcher described in `CELL_SELECTION_ACCESSIBILITY.md`. Custom actions remain
registered for supporting clients; native custom-action dispatch itself is not
claimed fixed. The visible controls provide the corresponding desktop workflow.

## Transaction boundaries

An open Cell actions menu records its worksheet, workspace, active cell and
range. Changing that context invalidates its result. Selection also verifies
workspace/worksheet identity after waiting for queued input and between cell
and formula commits. A delayed action from an old sheet must not commit the new
sheet's formula draft or move its selection.

## Qualification

Five new widget scenarios in `test/cell_actions_test.dart` cover standard
accessibility activation of the menu, mouse/accessible/keyboard extension,
F8/Finish/Escape, active-cell selection, edit cancellation, failed-save retry,
stale menus, delayed commits across worksheet changes and table editor focus.
The preceding four custom/single-cell selection checks remain in
`test/cell_selection_accessibility_test.dart`. Package-specific native and full
suite evidence is recorded in VALIDATION.md.

Source qualification: 373 Flutter tests pass (two optional AI skips), including
all nine focused cases, and analysis is clean. The verified 0.77 package is
prepared without replacing the existing preview while the Mac is locked.

Follow-up native qualification in the single 0.79 preview verifies worksheet
Cell actions/Edit cell, ordinary typed entry/commit, typed Escape cancellation,
F8 plus an arrow, Finish selection and range retention in Add watch. Cancelled
typing leaves the saved recovery byte-identical. Keyboard workspace Undo/Redo,
XLSX export/reimport and restart pass in the Watch Window fixture. See
`release/validation/watch-window-native`. This qualifies inherited worksheet
controls in 0.79; it does not launch or qualify the separate 0.77 candidate.

The remaining native sequence includes the disposable annotated table's
Select cells/Edit cell/focus/type/Cancel and worksheet pointer extension,
Select active cell and additional focus/accessibility cases.
Keep one preview and confirm zero processes before replacing it. Do not regard
the custom-action framework tests as native control evidence.

Full VoiceOver navigation, all screen-reader/platform combinations, native
delayed/failing-save timing, all IME/focus transitions, noncontiguous ranges and
actual Word/Excel comparison remain open. These controls do not complete a
parity family or count toward the subsequent 20 novel AI features.
