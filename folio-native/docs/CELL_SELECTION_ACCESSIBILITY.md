# Cell selection and accessibility

Document-table and worksheet cells distinguish ordinary accessibility activation
from extending a selection. Activating a cell selects that cell; the table
editor's explicit **Select cells** mode still extends its range. Cell addresses,
content and selection remain available to accessibility clients.

Both grids register **Extend selection to this cell**, and worksheet cells
register **Edit cell** as custom semantics actions. Their handlers pass widget
tests, including committing the previous draft before editing another cell and
retaining a rejected draft for retry. They are **not yet native-qualified**:
the current macOS accessibility tree does not expose them. The installed
Flutter desktop delegate handles default activation, focus and show-on-screen,
but returns false for other actions, including custom actions. Native delivery
needs further implementation. Keyboard editing and Shift-arrow selection, and
the table editor's Select cells mode, remain available and pass native checks.

Mouse selection continues to honor Shift. Worksheet selection captures modifier
state when the selection starts, before awaiting pending cell, formula or queued
entry commits. Releasing Shift while a commit finishes no longer changes the
original range-selection intent.

## Reproduced cause

In the 0.75 native preview, Command-Shift-E followed by cancelling the native
Export panel left Flutter reporting Shift during accessibility cell activation.
Activating table row 3 selected rows 1–3; Sort consequently proposed that partial
range. A real mouse click on the same row selected a single cell and Sort again
proposed the whole five-row table.

The installed Flutter 3.44.7 macOS controller reconciles native modifiers in
`FlutterViewController.mm` before sending a pointer event. An accessibility tap
does not travel through that pointer route. Folio now gives accessible selection
an explicit intent instead of inferring it from the cached keyboard state.
Existing native keyboard normalization and actual mouse input are unchanged.

## Verification scope

`test/cell_selection_accessibility_test.dart` covers accessible single-cell
activation with Shift held, actual Shift-click range selection, explicit range
actions, table Select cells mode, worksheet editing, rejected-save retry and
Shift release while a save is pending. Table selection alone preserves the
complete original draft; worksheet editing cancellation creates no edit.

The change is part of D03/D04 and cell navigation, not full accessibility or
Office parity. VoiceOver reading/navigation, input methods, all modifier/focus
transitions, row/column headings, large virtualized ranges and actual Word/Excel
comparison still require qualification. The separate native Export focus issue
is not claimed fixed by this selection change. Package-specific native evidence
belongs in VALIDATION.md.
