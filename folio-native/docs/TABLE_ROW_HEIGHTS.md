# Table row heights — 0.72 development

The editor and Page preview now share row-sizing rules. **Exactly** holds the
specified height even if the contents are taller. **At least** uses that height
as a minimum. **Automatic** sizes from the contents and ignores a stored height;
there is no additional 36-pixel row minimum. Existing cell padding still applies.

Ordinary cells are measured before vertically merged cells. Overflow in a
merged cell grows only rows whose height is not exact. If every spanned row is
exact, the merged cell keeps their combined height and clips its contents.

Preview carries clipping rectangles with text, custom inline painting, and
nested table fills/borders. They shift and intersect as content moves through
containing cells and paragraphs. Fully clipped lines do not appear in the
preview's text projection, and clipped field boxes do not acquire another
page's number. Top/center/bottom positioning uses the same signed remaining
space as the editor, including when content exceeds the available height.
Source text and field caches are retained; clipping is a display operation.

## Evidence

- Eight focused layout tests cover automatic/minimum/exact heights, heights
  below the old minimum, oversized text, following paragraphs, mixed and
  all-exact merged rows, alignment, nested offsets, repeated headings and page
  fields. A canvas test scans the full interior of the following row for nested
  fill/border/text leakage and saves a rendered image for visual inspection.
- A real-bridge widget workflow creates and imports a DOCX fixture with seven
  explicit line breaks, changes the first row through Table properties, tests
  nested draft Cancel/Apply, opens Page preview, and exercises one-step
  Undo/Redo, export, recovery and reimport. Its table changes from 216 to 72
  logical pixels; the first row changes to 24 points (32 logical pixels), while
  the second remains 30 points (40 logical pixels). Preview changes no workspace
  data and workbook sheets remain unchanged.
- The complete Flutter suite passes 337 tests with two optional provider tests
  skipped. The final explicit-line-break fixture and lint corrections are also
  checked in a focused rerun; analysis is clean. Rust implementation is
  unchanged, so the previous 360-test Rust result is historical, not a rerun.
- `scripts/verification/verify_table_row_heights.py` independently checks that
  only the first row's height changed in the DOCX XML, every clipped line remains
  saved, unrelated package parts are identical and recovery exports identical
  bytes. The source and two exported files pass Microsoft365 Open XML validation.

## Native qualification — 2026-09-20

The signed 0.72 package was checked against the tested implementation and
launched only after the saved 0.71 preview exited. Native checks pass for
automatic-height import, exact 24-point Apply, draft Cancel, immediate and
post-Save As keyboard Undo/Redo, minimum 24-point Apply and Undo, all three
modes in Page preview, working Save, export and restart recovery. The document
has two single-cell rows; the first contains eight lines separated by explicit
line breaks. Exact height clips the extra lines; automatic and minimum height
show them. The second row remains 30 points and the following paragraph stays
clear. Clipped text remains available in the source; the cell editor currently
presents this line-break paragraph as a preserved block rather than editable
rich text, which remains a separate parity gap.

Seventeen native checkpoints verify exact document history, unchanged source
package and workbook state, and byte-identical preview/cancellation/restart.
The final revision-8 recovery SHA-256 is
`cb4c828049d225f0e35827721a6e84d19251a34ecc0dca82c242b3338662b2b0`.
The working Save and recovered export match the first exact-height export.
Source and three native output files pass Microsoft365 schema validation.
The saved document is open in the sole 0.72 preview. The normal user workspace
and the closed 0.71 preview's saved workspace are unchanged. Evidence is in
`release/validation/table-row-heights-native`; the earlier source-only archive
remains immutable. This package uses an ad-hoc signature, not notarization.

The file picker initially reopened a path-entry sheet after confirming the
path. Dismissing that sheet exposed the correctly selected file. No document
text was entered accidentally in this run; save names were typed only after
the filename selection was visibly confirmed.

## Remaining work

The limits below describe the packaged 0.72 baseline. Subsequent source work on
ordinary row splitting and `w:cantSplit` is tracked separately in
[TABLE_ROW_SPLITTING.md](TABLE_ROW_SPLITTING.md); it has not replaced that preview.

Direct Microsoft Word comparison is pending, including overflowing vertical
alignment, row-height constraints and merged-cell distribution. Cell margins,
borders, table styles, text direction and intrinsic font metrics are not yet a
complete Word layout model. Splitting normal rows between pages, `w:cantSplit`,
oversized-row fragmentation, manual breaks inside cells, and independent cell
continuations still need implementation. Current pagination normally keeps row
groups intact and uses geometric clipping for oversized groups. No whole parity
family or additional AI feature is accepted by this change.

Nested clipping, merged rows, overflowing center/bottom alignment and exact
repeated headings have automated coverage but still need their own native
fixtures. The native check above does not stand in for those cases.

## Primary reference

[Microsoft Open XML: TableRowHeight](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.tablerowheight?view=openxml-3.0.1)
documents content-driven automatic sizing and absolute exact row heights.
