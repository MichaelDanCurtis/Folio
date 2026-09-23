# Watch Window

Version 0.79 adds a live, nonmodal Watch Window docked below the worksheet.
The Formulas tab provides the Watch Window toggle, Formula inspector and
Calculate now. Select a cell or range and choose Add watch in the panel, Cell
actions or the cell context menu. The Add watch dialog can target another
worksheet. Duplicate additions leave the workbook and history unchanged.

The panel shows workbook, worksheet, address, formatted value and formula.
Values update after normal entry, recalculation and history operations without
changing sheets. Double-click the detail portion of a row or use its Go to
button to select the watched cell; the adjacent Inspect button opens Formula
inspector there. Check multiple rows or Select all to delete watches. Removing
watches does not remove cell contents. Column boundaries and the panel's top
edge can be dragged to resize the view. Closing the panel retains the watches.

Watches are worksheet address records in the local recovery and standard XLSX
`cellWatches/cellWatch` elements. Add/remove operations are transactional,
undoable and saved as workbook changes. Formula calculation generations do not
change just because a watch was added. Cell entry still passes normal validation
before watch commands run. Add watch rejects a workbook changed while its
dialog was open; delayed removal failures do not attach to a different workbook.

Row/column insertion and deletion transform watch addresses in both the editable
projection and preserved Office XML. Deleting a watched cell removes its watch;
Undo restores both. Watches on otherwise blank edge cells reject insertions
that would push them outside Excel's coordinate bounds. Sheet rename/move/delete
follow the owning worksheet. Sorting keeps watches at their addresses as values
change; no separate row-identity promise is made.

Older recovery files hydrate watch metadata from their embedded workbook only
when the field is absent. Intentional empty watch maps are preserved, preventing
removed watches from reappearing. Files without watches gain no empty metadata.
Unedited imported workbook bytes remain unchanged. Supported watch changes
preserve unrelated worksheet nodes and unknown watch metadata; unsupported
references remain in the source and produce a panel notice. Multiple watch
collections are rejected on edit rather than merged silently.

## Evidence and remaining work

Six Rust cases cover uniqueness, non-mutating duplicate additions, cell and
calculation preservation, undo/redo, atomic invalid/stale requests, OOXML
roundtrips, row/column changes, old recovery, removal without resurrection,
blank edge cells and source-metadata preservation. Widget cases cover explicit
navigation, resizing, multiple selection, retry, stale asynchronous completion
and the Add watch dialog. The integration test uses the real app and Rust bridge
to add watches across two worksheets, edit a source value, observe its total,
inspect its formula, remove/undo/redo watches, insert/delete rows and undo the
deletion, then save/reload recovery and export/reimport XLSX.

`scripts/verification/verify_watch_window.py` independently checks two saved
watches (Forecast C1 and Sources B4), the shifted source range B3:B5, inputs
10/25/30 and total 65. Microsoft Open XML SDK validates five focused XLSX files
against Microsoft 365 schemas. A widget-rendered screenshot demonstrates the
live panel; it does not prove packaged native behavior or Excel execution.

The single packaged 0.79 preview passes native two-watch import, navigation,
F8/Finish selection, range addition without duplicates, watch deletion and
keyboard Undo/Redo. Typed source entry updates the watched total (25/65 to
26/66); typed Escape cancellation leaves recovery bytes unchanged, and Undo
restores the original values. Formula inspector follows a source cell, shows
its dependent, returns with Back and selects the exact source range. Export,
in-place Save, normal quit/restart and XLSX reimport retain three watches.
`scripts/verification/verify_native_watch_window.py` independently verifies
14 captured states, byte-identical restart recovery and matching saved/exported
files. The sole semantic workbook change is the additional Sources B5 watch;
the formula cache only gains an equivalent namespace declaration. Both native
XLSX files pass schema checks. Evidence is in
`release/validation/watch-window-native`; screenshots are retained in the
conversation. This does not establish execution in Microsoft Excel.

Floating/multiple-edge docking, saved panel
geometry, named-range labels, simultaneous external workbooks, full keyboard
row selection and direct Excel workflow comparison remain open. Current limits
are 100,000 watches per worksheet and per add request, within the existing
workspace size limit; large-list performance has not been qualified. X10 and
the overall parity goal remain incomplete. This is an Excel parity feature,
not one of the 20 novel AI features.

Baseline references: Microsoft's [Watch Window workflow](https://support.microsoft.com/en-us/excel/watch-a-formula-and-its-result-by-using-the-watch-window),
[cell watch record](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.cellwatch?view=openxml-3.0.1)
and [watch collection](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.cellwatches?view=openxml-3.0.1).
