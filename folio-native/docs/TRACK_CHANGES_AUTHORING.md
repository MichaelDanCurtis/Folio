# Track Changes authoring — 0.60 development

Review > Record changes opens recording settings for an explicit review author,
recording on/off and formatting tracking. The adjacent settings button can
change these later. A band above the editor shows the author and recording
state. Control+Shift+E toggles recording in the document editor; Command+Shift+E
remains Export. Turning recording off leaves earlier changes available to review.

An imported Word file with trackRevisions enabled opens read-only until the user
supplies a review author and applies the recording settings. Recording can be
turned off in that dialog; the current UI still requires a nonblank author.
Folio does not adopt another reviewer's identity from the source file. The
source file's formatting-tracking choice initializes the dialog.

When a recording operation cannot preserve the requested edit, the draft stays
visible with an unsaved error. Save and close cannot silently persist an older
document. Copy draft text lets the user retain its text; Discard unsaved changes
explicitly restores the last committed document before editing continues.
Copy draft text is plain text and does not preserve formatting.

The native engine can record exact UTF-16 text edits, replacements, tabs, soft
breaks, paragraph splits/joins and direct run/paragraph formatting. It retains
earlier pending changes, distinguishes edits to the named author's own pending
insertions, and supports ordinary text edits inside existing table cells.
Replacement previews put deleted text before inserted text. Turning recording
off keeps earlier revisions; turning formatting tracking off still applies
formatting directly. Author names are explicitly supplied, not authenticated
or inferred ownership.

`documentTracking` is an internal, revision-guarded command with workspace ID,
base revision, author, enabled and formatting settings. Ordinary `document`
operations then record against the immutable source package using their exact
edit journal. Settings, author, source package and the current editable view
travel through Undo/Redo and `.folio` recovery. DOCX export preserves source
revision markup and Word's trackRevisions/doNotTrackFormatting settings.
Review accepts/rejects the recorded source and restores an editable current view.

The bridge aligns source stories and editor snapshots by logical positions and
container boundaries. It does not trust editor office IDs: Enter can duplicate
them, and deleted paragraph marks change the number of physical source blocks.
Comparing the editor's before/after formatting projections identifies actual
user edits; omitted source settings are retained. Coverage includes small caps,
language, East Asian font settings, compound paragraph spacing, paragraph-mark
typing formats, explicit fonts overriding theme fonts and local namespaces.
Run metadata is shared between character records instead of copied per letter.
The final editable region also accepts native paragraph appends after the
editor's terminal newline, translating that sentinel to Word's retained final
paragraph mark. The captured native operation is a regression fixture.
At the editor boundary, native insertions after Quill's hidden final newline
and their selection offsets are normalized before that sentinel so that the
visible tree, Delta, caret and file journal remain aligned.

## Current boundaries

- Editing through fields, hyperlinks, annotations, moves, section boundaries,
  content controls and other preserved objects remains unsupported by this
  recorder. Unchanged objects and unrelated package parts remain preserved.
- Row/cell/table structural and property changes, cross-cell edits, other Word
  story authoring and live workbook bindings require further integration.
- Style-catalog changes and document-level setting/annotation mutations while
  using the authoring projection are guarded pending faithful recording support.
- Paragraph/current-view fidelity, mixed author formatting, broad IME/clipboard
  cases, accessibility, large-document latency/history memory,
  broader native macOS interaction and actual Microsoft Word comparison remain open.
- The editor shows current editable content. 0.61 adds a separate in-document
  review surface with current/original/markup projections and navigation; see
  REVIEW_DISPLAY.md. Editing within markup, full inline formatting/structural
  fidelity, balloons, simple markup and Compare/Merge remain unfinished.

These are implementation boundaries, not a reduced definition of parity.
Full Word/Excel parity remains open and the additional novel AI feature count
remains 0/20.

## Verification

`rust/src/docx_track_edit_tests.rs` tests exact journals and Word revision
semantics. `rust/src/docx_tracking_tests.rs` covers package/container mapping,
source property preservation, engine settings, history, review, recovery,
export and atomic failures. `test/document_tracking_workspace_test.dart` uses
the real native bridge and existing Flutter editor to choose an author, queue
repeated-text and emoji edits, Enter/Backspace, formatting, Undo/Redo, export,
review and reopening. It also covers a rejected hyperlink edit, blocked Save,
explicit draft discard, resumed typing, the recording shortcut and author setup
for an imported file. The Review ribbon scrolls at narrow window widths.

`scripts/verification/verify_docx_tracking.py` reads exported ZIP/XML directly
with lxml. The separate Microsoft Open XML SDK validator checks the saved
packages against Microsoft 365 schemas. These checks do not substitute for
Microsoft Word desktop comparison or packaged native UI verification.

The isolated 0.60 build 3 package passed native end-of-document Enter and
continued typing, author-filtered rejection and Undo, DOCX save/reimport,
explicit author setup, clean quit/restart and Save after restart. The reopened
review shows all seven revisions with five Alex and two Morgan changes.
`scripts/verification/verify_native_recording.py` independently checks the three
native-saved DOCX artifacts; all three also pass Microsoft365 schema validation.
Earlier native build 1 checks covered recording/export shortcuts, formatting,
replacement, rejected-link draft retention and explicit discard. The final
package includes both native-discovered terminal-paragraph fixes. Evidence is
retained in `release/validation/tracking-recording`; the normal release was not
promoted and no full parity family is accepted.

## References

- [Word tracking controls and review behavior](https://support.microsoft.com/en-us/word/training/track-changes-in-word) (checked 2026-09-20)
- [Word run-property revision snapshots](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.runpropertieschange?view=openxml-3.0.1)
- [Word settings and revision options](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.settings?view=openxml-3.0.1)
- [Paragraph-mark properties](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.paragraphmarkrunproperties?view=openxml-3.0.1)
