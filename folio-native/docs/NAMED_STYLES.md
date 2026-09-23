# Named styles — implemented scope and remaining work

The current development source contains a shared Word style catalog and
transactional style commands, a searchable style picker and editor, and inherited
text rendering. The reviewed application is 0.53.0 build 11, including in-use style replacement,
missing Heading 1–9 creation, built-in deletion protection and direct outline
level overrides. See `OUTLINE_LEVELS.md` for structure versus appearance.
This work does not complete W02 or any other Word/Excel parity family.

## Model and commands

`document.attrs.wordStyles` stores document defaults, the default paragraph
style identity and the source style declarations. Each style retains its stable
identity, display name, type, base style, next paragraph style, linked style and
modeled paragraph/tab/run properties. Paragraph layout and tab declarations remain
separate from their inherited values. Paragraph formatting now also carries
resolved run properties, alignment and outline metadata for the renderer.

`documentStyles` reads the catalog and resolved properties without changing the
workspace. `documentStyle` requires the current `workspaceId` and `baseRevision`.
Its actions are:

- `upsert`, with a full `style`: create or edit a declaration, including renaming
  its display name without breaking its identity or dependents.
- `apply`, with `styleId` and `paths`: assign an existing paragraph style to
  selected paragraphs, including nested table cells, preserving direct overrides.
  Each path indexes `content` arrays from the document root.
- `delete`, with `styleId`: remove an unused non-default style. With an explicit
  `replacementStyleId`, replace body/table assignments, related-style references
  and preserved Word XML references in one transaction. See `STYLE_REPLACEMENT.md`.

A successful change refreshes inherited formatting and commits as one workspace
history entry. Stale workspaces/revisions, invalid selections, duplicate paths,
new missing base/next/link references, cross-type inheritance and newly edited
cycles fail without changing the workspace. No-op commands do not add history.
Ordinary text updates and document replacement proposals retain the catalog;
style changes require the dedicated transaction.

`documentStylePreview` validates a revision-checked cell draft and returns the
styled document without changing workspace state. Only the outer table Apply
commits it. Table drafts can apply existing styles; global style creation and
modification are available in the main document's Styles dialog.

## Editor behavior

The Styles dialog searches paragraph styles by name and supports Apply, New,
Modify and Delete. Creating a style adds it to the catalog; Apply assigns it to
the captured paragraph selection. The editor exposes base and following styles,
font, point size, color, bold/italic/strike overrides and paragraph formatting.
Renaming retains the style identity. Cancel discards drafts, and changes made
elsewhere invalidate an open selection before Apply.

Inherited fonts, sizes, supported decorations, colors and alignment render in
the main editor and static table cells. Explicit off values for bold, italic,
underline and strike survive import, text editing, toolbar toggles and export.
Direct point sizes use the same 96-dpi display conversion as inherited sizes.
Native paragraphs use the same spacing-collapse rules before and after style
catalog materialization. Main text, table previews, cell editors and the cell
paragraph dialog share the native font/spacing defaults. Adding an unused style
therefore does not change existing text geometry; the 0.53 build 1 candidate
revealed and helped reproduce that initial-materialization bug.
Theme font/literal precedence is resolved through the hierarchy; basic theme
colors are resolved. Tint/shade transformations and script-specific selection
remain incomplete.

Enter at the end of a non-list paragraph uses its configured following style;
splitting in the middle and multiline insertion retain the original style.
Inserted paragraphs receive distinct source identity. Each Quill document owns
its custom rules, so a table draft cannot replace the main editor's catalog.
Conventional heading commands resolve Heading 1–9 and the following body style.
Missing declarations appear in Styles and are created only when used, including
inside table drafts. List and heading attributes coexist, and styled lists stay
editable. Built-in origins are retained and protected from deletion; copies are
custom styles. See `BUILTIN_STYLES.md` for behavior and qualification limits.

The resolver applies the supported run toggles through same-type style chains.
An enabled toggle reverses the inherited setting; a disabled toggle in a style
leaves it unchanged. This differs from a direct run override. Microsoft documents
[bold as a toggle property](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.bold?view=openxml-3.0.1)
and [the basedOn relationship](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.basedon?view=openxml-3.0.1).
Imported broken chains terminate during resolution rather than looping forever.

## File preservation

The style part is located through its relationship, including alternate paths
and namespace prefixes. Export patches changed declarations while keeping
unmodeled style elements, attributes, extension XML and unrelated package parts.
Missing style parts gain an internal relationship and content-type declaration.
Unchanged exports retain the original ZIP bytes.

Paragraph-only formatting changes now retain the original paragraph attributes
and full inline XML. This preserves run properties the editor does not yet model
when changing paragraph styles. The property writer also retains namespace
bindings declared locally on properties, including alternate prefixes and
whitespace around namespace declarations.

Legacy saved workspaces hydrate missing catalog and resolved metadata from their
source package, preserving explicit overrides. Save/reopen and workspace history
retain the catalog and source package together.

## Verification and remaining work

Engine regressions cover inheritance and toggle behavior, nested-cell application,
rename/base edits, custom outline headings, native documents, tab clears,
removing declarations, no-ops, history/recovery, missing style parts, legacy
hydration, invalid/stale operations and namespace/unknown-XML preservation.
`scripts/verification/verify_named_styles.py` independently opens the exported
artifacts with python-docx and checks style identities, relationships, declared
fonts/spacing, direct paragraph/run overrides and selected preserved XML.
See VALIDATION.md for completed runs.

The isolated 0.53 candidate has native evidence for Modify, inherited rendering,
keyboard Undo/Redo, following-style Enter, cell draft Apply and history, export
and byte-identical restart. The independent reader confirms style identity,
font size, direct-off formatting and body/cell relationships. The normal user
workspace is unchanged. This is scoped evidence, not Microsoft Word rendering
equivalence or complete style-family qualification.

Build 2 additionally passes native New/Modify Cancel, first custom style creation,
targeted body application, unused-style Delete, Apply/Delete keyboard history,
DOCX export and byte-identical restart. Creating the first style no longer moves
the existing native headings, bodies or bullets. The independent checker is
`scripts/verification/verify_native_style_creation.py`; its inputs and report are
in `../../work/native-style-creation`. In-use deletion is currently rejected;
replacement-style behavior remains unfinished.

Build 3 source preserves the current selection when committed formatting or
embedded-object properties refresh the document. Text changes map both endpoints
through the edit, keeping direction and valid UTF-16 positions. Explicit Open/New
starts a fresh selection context. This corrects the build 2 native observation
that Undo reset the caret to zero. It maintains the current editing position;
per-history-entry restoration of the original selection after later navigation
is not yet implemented. Native build 3 also exposed a separate macOS whole-line
selection bug: the shortcut included the preceding paragraph break. Build 4 uses
rendered line boundaries, retains existing selections when expanding their other
edge, and handles empty lines. Hard/soft-line keyboard and deletion checks pass.

Build 4 natively verifies whole-line selection, style modification with keyboard
Undo/Redo, and replacement of only the selected body. It exposed a separate paste
caret error. Build 5 corrects the insertion offset for plain and rich replacements;
six clipboard regressions cover forward/reversed selections, formatting, Unicode
and subsequent typing. Native build 5 subsequently passed continuation, keyboard
history, DOCX export and byte-identical restart at revision 29. Its full report is
in `../../work/native-style-paste/verification.json`.

Build 6 adds ordered input during asynchronous Undo/Redo. Deterministic delayed
engine tests and native rapid keyboard checks pass, including history after the
Save dialog, preserved neighbors, export and restart at revision 40. Build 7
retains exact history positions in repeated text and emoji; native rapid typing,
post-export history and byte-identical restart pass at revision 56. The current
suites pass 246 Rust tests and 235 Flutter checks with 2 optional AI-provider
skips. The normal user workspace remains byte-identical at revision 30. The
reviewable release is 0.53.0 build 9. See `DOCUMENT_HISTORY_INPUT.md` for the
separate history scope.

Build 8 also passes native in-use style replacement, Cancel, keyboard Undo/Redo,
post-Save history, DOCX export and byte-identical restart at revision 61.
See `STYLE_REPLACEMENT.md` and `../../work/native-style-replacement/verification.json`.

Build 9 passes native missing Heading 9 application/Cancel, Heading 7 in a table
draft, keyboard and post-Save Undo/Redo, DOCX export and byte-identical restart
at revision 70. See `BUILTIN_STYLES.md` and
`../../work/native-builtin-styles/verification.json`.

Next implementation work:

1. Complete original selection restoration after later navigation and history
   mapping for opaque embeds and untracked edits. Qualify list/heading transitions, table draft
   cancellation, next-style typing with IME, broader concurrent formatting and
   keyboard focus after nested dialogs.
2. Complete theme and script font resolution, character-style application,
   table/numbering-style cascades, default-style editing, templates and the style
   organizer. Comprehensive localized/legacy built-in identities and direct
   paragraph outline overrides need further modeling.
3. Qualify actual desktop interactions, rendering, recovery and Word round trips,
   including Microsoft Word comparison. Engine tests and an independent ZIP
   reader do not establish native or Word rendering equivalence.

No new package or native style feature is accepted by automated checks alone.
The full parity goal remains active; additional AI features accepted: 0/20.
