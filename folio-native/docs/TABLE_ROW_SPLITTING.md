# Table rows across pages — 0.73 development

Table properties now includes **Allow row to break across pages**, with a mixed
state for a selection containing different settings. Only toggling the checkbox
changes it. The properties dialog updates the table draft; the outer Apply makes
one workspace edit, while Cancel discards the draft.

Ordinary automatic/minimum-height rows can use the remaining page space. Each
cell continues at its own paragraph/text position. Different fonts and line
spacing no longer share a geometric cut, and paragraphs are reshaped when the
destination column has a different width. Finished cells stay empty while their
neighbors continue. Nested ordinary tables participate in cell continuation.
Rows marked not to split move to the next page when necessary; an oversized row
starts fresh and still continues. Repeated outer-table headings, per-fragment
page fields and continuous-section column balancing use the same source cursors.

DOCX import retains direct `w:cantSplit` separately from inherited defaults.
Export writes `on`/`off`, preserving unrelated row properties and extension
attributes/children. Table-style base chains, the default table style, horizontal
bands, first/last row conditions and row `cnfStyle` flags are projected on import.
A direct false value overrides an inherited true value. Cycles terminate.

## Verification

Final source checks: **363 Rust tests passed; 348 Flutter tests passed with two
optional AI-provider skips; Flutter analysis is clean.** The optimized Rust
library was rebuilt before the real-bridge application and full Flutter runs.
No desktop preview was launched or closed during this change.

- Three new Rust tests cover boolean spellings, source extension preservation,
  property order, invalid-value atomicity, style inheritance, default styles,
  cycles, export/reimport, save/load and byte-identical Undo/Redo/recovery.
- Ten focused Flutter checks cover independent cell line heights, actual mixed
  font sizes, page-space use, keep-together/oversized behavior, inherited/direct
  precedence, nested tables, unequal RTL columns, continuation during balancing,
  wrapped paragraphs, page fields, repeated headings and mixed dialog state.
- One real-Rust application check drives both dialogs, Cancel, Apply, preview,
  Undo/Redo, export, workspace recovery and DOCX reimport. The workbook is unchanged.
- `scripts/verification/verify_table_row_flow.py` independently verifies all 67
  text runs and 63 genuine Word line breaks in the three application artifacts,
  the single requested row-property change, unrelated ZIP parts and identical
  recovered bytes. These three files pass Microsoft365 Open XML validation.
- The older `word-tables` preservation fixtures contain three pre-existing
  missing `w:shd/@w:val` attributes. The engine's original/edited/restored outputs
  retain those same errors. They are preservation checks, not clean schema cases.
- The mixed-font canvas image was inspected: 11-point and 19-point Arial content
  continues independently, with every A00–A18 and B00–B18 label present. The
  unrelated trailing paragraph uses the test runner's Ahem font. This is a
  renderer test image, not a screenshot of the packaged desktop application.

Evidence for this source change is in `../../work/table-row-flow-*` relative to
the project: `rust-all.log`, `app.log`, `flutter-all.log`, `analyze.log`,
`independent.json`, `schema-app.json` and `schema-engine.json`.
The 0.73 package and native ordinary-row workflow are now qualified as described
below. Comparison in Microsoft Word remains pending.

## Native 0.73 qualification

The signed package matches 301 app source files, 107 Rust source files and six
template assets from the tested implementation. Only version metadata changed
after the automated run. Its ZIP SHA-256 is
`0c0c6e042e91748fbd29169e1dc5b808a40f82786b93b7c308d55039a29d5005`.
This is an ad-hoc-signed preview, not a notarized release or a promotion of the
normal installed application.

Nineteen native checkpoints verify import, read-only preview, Cancel, Apply,
ordinary keyboard Undo/Redo, export, working Save As, immediate post-Save
keyboard Undo/Redo, explicit Allow Apply/export, Undo back to keep-together,
working Save and restart recovery. With splitting allowed, page one contains
A00–A13/B00–B13 and page two continues A14–A17/B14–B17. With keep-together, the
whole row starts on page two. Source text is retained in each case.

All unrelated workspace state stays unchanged. The saved and recovered Word
exports are byte-identical, and source, kept, allowed, working-save and recovery
files all pass Microsoft365 schema validation. The recovered revision 8 workspace
SHA-256 is `60d8400425c379612caa1eb2d1ffee5a391040a3bce25b87968282892364abc7`.
The previous 0.72 preview and normal user workspace remain unchanged. Native
screenshots and accessibility observations are retained in the task conversation.

The previous preview was quit normally and zero processes confirmed before
launching 0.73. Restart likewise went through verified zero and one process
states. The final saved `row-flow.docx` was left in the sole 0.73 preview; that
preview was subsequently closed normally for 0.74 and its recovery retained. Native
evidence is archived in `release/validation/table-row-flow-native`; the source
and automated evidence is in `release/validation/table-row-flow-development`.

This native scenario uses one ordinary row with two cells and fixed paragraph
line spacing. Mixed fonts, nested continuation, unequal/RTL columns, column
balancing, repeated headings, fields and inherited style rules have automated
coverage but still need native fixtures. The 0.73 preview showed this fixture's
line-break paragraphs as preserved blocks. Version 0.74 makes ordinary breaks
editable; see `MANUAL_LINE_BREAKS.md` for automated and packaged native evidence.

## Still required

This is progress toward table pagination, not full table-layout parity:

- Exact-height rows retain existing fixed geometry and are still treated as
  whole rows unless oversized. Vertical merge groups remain grouped; oversized
  merged groups use geometric clipping with a visible notice. Both need a joint
  continuation model that honors row-span and fixed-height constraints.
- Split center/bottom-aligned cells currently use top alignment and report that
  limitation. Complete vertical alignment, borders and cell-margin behavior at
  page boundaries require direct Word comparison.
- Repeating headings inside nested tables, manual breaks in table cells and full
  paragraph keep-with-next/orphan behavior inside cells need further work.
- Cell/corner conditional table-style row properties are retained and diagnosed.
  The entire table-style cascade, especially recomputing conditional inheritance
  after structural row/style edits, is not accepted as complete.
- Text taller than a printable cell fragment has an explicit clipping fallback.
  Full typography, field-width convergence and large-document performance still
  need broader evidence. No speed improvement is claimed.

The full Word/Excel goal remains incomplete. Novel AI features accepted: 0/20.

## References

[Microsoft: CantSplit](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.cantsplit?view=openxml-3.0.1)
specifies inherited defaults, keep-together movement and the oversized-row exception.
[Microsoft: TableLook](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.tablelook?view=openxml-3.0.1)
and [ConditionalFormatStyle.Val](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.conditionalformatstyle.val?view=openxml-3.0.1)
describe conditional style controls. Specifications guide implementation; they
do not establish matching behavior in Microsoft Word.
