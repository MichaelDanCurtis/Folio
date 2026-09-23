# Paste Special: number formats, validation and column widths

Version 0.51 adds five choices to the worksheet's Paste Special dialog:

| Choice | Result |
| --- | --- |
| Values and number formats | Uses the copied values/formula results and number formats, retaining the destination's other cell formatting. |
| Formulas and number formats | Copies formulas with relative-reference adjustment, literal contents, and number formats. |
| Validation rules only | Replaces validation in the written cells without replacing their contents or ordinary/conditional formatting. |
| Column widths only | Copies the selected source columns' widths to destination columns without changing cell contents. |
| Cells with source column widths | Performs ordinary Paste All and also copies column widths. |

Use the existing Paste menu or the Paste Special shortcut (Command-Shift-V on
Mac). The expanded options scroll in shorter windows. The dialog disables
Transpose for width modes, and disables Skip blanks for width-only paste. Skip
blanks still applies to the cell content of Cells with source column widths;
widths apply to the destination columns regardless of their cell contents.

A copied snapshot retains values, formulas, formats, validation and dimensions
at Copy time. Later source edits do not replace that snapshot. For a source
column without an explicit width, the copied width comes from its stored sheet
default. When Folio uses a visual default, the desktop supplies its measured
width at Copy time. A direct engine caller must supply that measurement when
there is no stored default and a width paste is requested. Explicit stored
widths take precedence over the measurement; no font width is guessed in Rust.
Column-width selections may have a different row count from their source.

Number-format transfer resolves cell/row/column inheritance and the source XF's
number-format inheritance. Built-in format IDs stay intact, including formats
not rendered by Folio's current locale catalog. Custom format records receive
collision-free target IDs. Destination font, fill, border, alignment, protection,
base-style reference and unrelated XF XML remain attached to the target cell.
Only values/formulas and the number-format component change. Real formula
errors remain distinct from formula results that merely look like error text.

Validation-only paste supports the existing source map: range tiling,
transpose, and Skip blanks. Blank cells without a rule clear destination rules
unless skipped. A copied rule can exist on an empty cell. Destination fragments
retain their rules, and relative formulas follow the new anchors. Reanchoring
retained rules uses the destination sheet catalog; a copied formula that names
a missing destination worksheet rejects the operation. Input/error prompts use
the existing preserved validation XML. Full name/external-reference rebinding
and extended validation remain unfinished.

Widths preserve destination hidden/outline/style metadata. A zero stored width
continues to hide that column under the existing geometry rules. Widths are
SpreadsheetML units, not a guarantee of equal pixels across different Normal
fonts. Direct Excel comparison, cross-font/theme behavior and all hidden/zero
width combinations remain unqualified.

All choices use the existing staged edit and automatic-row measurement path.
The complete paste is one Undo step; failed measurement publishes none of it.
Save/reopen, XLSX export and independent file readers are separate acceptance
checks. Metadata-only paste can cross merged cells without modifying the merge.
Content-changing pastes retain existing restrictions on merges, array formulas,
protection, structured references, cross-workbook formulas and size limits.
Only in-process copies carry these attributes; external text has no such data.

## Evidence and remaining work

`VALIDATION.md` records the executed checks. The independent input workbook is
produced by openpyxl; the verifier reads saved outputs without resaving them.
Its extension warnings are not interpreted as proof that extension data is lost
from the on-disk files. Direct Microsoft Excel execution remains pending.
Arithmetic operations, notes/comments-only paste, links, all-except-borders,
source-theme conversion, Cut/move, rich external clipboard formats and complete
Paste Special parity remain incomplete. No whole parity family is accepted.

Primary behavior references:

- [Microsoft Support: Paste options](https://support.microsoft.com/en-us/excel/paste-options)
- [Microsoft Learn: XlPasteType enumeration](https://learn.microsoft.com/en-us/office/vba/api/excel.xlpastetype)
