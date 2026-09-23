# Editable manual line breaks

The source implementation makes ordinary Word line breaks editable in document
paragraphs, list items, table cells, and header/footer stories. Shift+Enter or
the **Line break** toolbar button inserts a break without creating a paragraph.
Plain Enter continues to create a paragraph/list item through normal text input.

## Representation and preservation

- A break is a one-position inline atom in the editor, with its original JSON
  retained separately from formatting. It shapes as U+2028, so typing, selection,
  caret movement and rich internal copying have matching character offsets.
- A literal U+2028 in an imported text run remains text. Formatting cannot erase
  a break marker, and typing after a break inherits its inline formatting.
- The document model retains `hardBreak` nodes. Body text, search and bookmark
  projections use a single newline at that position; paragraph boundaries stay
  separate. Word export writes `w:br`, not a new `w:p` or a literal newline run.
- Import/export now retains a break's supported run formatting and hyperlink.
  No-edit export still retains the original package bytes, including `w:cr`.
- Page/column breaks, wrap-clearing breaks and unmodeled break attributes keep
  their preserved paragraph representation. Their richer layout/authoring
  behavior is still pending. Ordinary breaks with explicit `clear="none"` or
  `type="textWrapping"` are editable and retain those declarations.

## Qualification

Source qualification passes: **365 Rust tests, 356 Flutter tests**, and two
optional AI-provider skips. Seven focused editor tests cover literal separators,
leading/trailing/consecutive breaks, supported marks, search/bookmark offsets,
exact edit journals, internal rich-copy content, Shift+Enter, plain Enter input,
list structure, two-line shaping/caret geometry, Backspace and draft Undo/Redo.
Separate engine tests cover break types, formatting and independent XML node
counts after a surrounding-text edit and a fresh export.

One full application test using the real Rust bridge edits imported body and
cell breaks, cancels a table draft, applies it, exercises separate workspace
Undo/Redo, creates a multiline header, checks unrelated workbook data, exports
DOCX, reimports it and reopens local recovery. The independent XML reader in
`scripts/verification/verify_line_breaks.py` confirms paragraph/break counts,
header relationships and byte-identical recovered export. Three application
files and two engine files pass Microsoft 365 Open XML schema validation.

The mixed text/embed renderer also retains original node offsets instead of
cloning detached text nodes. This fixes misplaced proofreading underlines after
inline objects; a regression checks the actual underline and correction after
a break. Analysis is clean.

## Native 0.74 qualification

The isolated 0.74.0 preview contains the tested implementation. All 302 app
source files, 108 Rust files and six template assets match the build evidence;
strict/deep ad-hoc signature and normalized engine comparisons pass. Only
version metadata changed after the full automated suite. The normal release
has not been promoted and this package is not notarized.

Native keyboard/mouse checks pass for imported body and table-cell breaks,
Shift+Enter, the cell toolbar button, and ordinary Enter creating a separate
paragraph. A misspelled word on the third visual line shows suggestions with a
divider above editing commands; choosing **received** actually replaces it.
Immediate keyboard Undo/Redo restores both versions. Table and header drafts
remain outside the workspace until Apply; Cancel leaves recovery bytes intact.
Body, cell and header lines appear in Page preview. Immediate and post-Save As
keyboard history, working Save, export, restart and native DOCX reimport pass.

The independent native verifier checks 29 captured states: exact paragraph
properties/source identities, all unrelated workspace data, whole-workspace
history including Office package state, byte-identical Cancel/preview/export
and restart groups, and unchanged document content after reimport. Four native
outputs (initial export, working save, restart export and reimport export) are
byte-identical. Those four files and the source pass Microsoft365 schema checks.
Reproduce with `scripts/verification/verify_native_line_breaks.py`; evidence and
source are archived in `release/validation/manual-line-breaks-native`.

The prior preview was quit normally before replacement; replacement and restart
each passed zero/one process guards. Only the current preview remains running,
showing Saved recovered.docx at revision 31. Its final recovery SHA-256 is
`634faa7f3a58464332daa5a6603718032dda262ffb1d072082c43d555146c3cb`.
The normal workspace and the closed 0.73 workspace remain unchanged. A later
desktop observation temporarily lost the window; the same process recovered
without relaunching it. Native screenshots and input observations are retained
in the task conversation; checkpoint assertions alone do not prove input paths.

This native fixture covers ordinary English left-to-right body/cell/header
content. Native list numbering, mixed formats, leading/trailing/repeated breaks,
input methods, formal accessibility, external Word clipboard and direct Word
comparison remain pending. The full parity goal and subsequent 20 novel AI
features remain incomplete.

## References

[Microsoft Open XML Break](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.break?view=openxml-3.0.1)
defines a break within run content, with behavior controlled by its type and
clear attributes. [Flutter TextSpan](https://api.flutter.dev/flutter/painting/TextSpan-class.html)
documents the text/style projection used by the editor; actual line shaping is
checked by the widget tests rather than inferred from that API description.
