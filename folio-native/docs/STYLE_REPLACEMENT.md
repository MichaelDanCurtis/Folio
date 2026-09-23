# Replace a style that is in use

Build 8 adds a replacement choice to **Styles → Delete**. The dialog explains
that text and related styles will use the replacement, while direct formatting
remains. Cancel returns to Styles without changing the document. The document's
default paragraph style cannot be deleted, and descendants of the selected style
are excluded from the replacement choices to avoid inheritance cycles.

The engine still accepts deletion without a replacement for an unused style.
With `replacementStyleId`, deletion updates matching body and nested-cell style
assignments and related `basedOn`, `next` and `link` declarations. It refreshes
inherited run, paragraph and tab formatting. A heading replaced by a body style
becomes a body paragraph; existing direct overrides remain separate.

Only schema-defined style-reference attributes are rewritten in preserved Word
XML: `pStyle`, `rStyle`, `tblStyle`, `basedOn`, `next`, `link`, `numStyleLink` and
`styleLink`. Namespace prefixes, foreign attributes, text and field instructions
are retained. This covers references in headers, footnotes, numbering and opaque
Word objects without flattening those objects into editable text. A new immutable
source package participates in the same workspace transaction as the catalog
change. Undo, Redo and local save/reopen retain the matching package and document.
Pending body edits remain in the editable projection and are exported normally.

The existing revision/workspace guard applies to the entire operation. Invalid,
self, missing, non-paragraph and cyclic replacements fail atomically. A cycle
already present in unrelated imported styles does not prevent this operation.

Microsoft's documentation describes the [base-style relationship](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.basedon?view=openxml-3.0.1)
and [creating and modifying named styles](https://support.microsoft.com/en-us/word/customize-or-create-new-styles).
These sources inform the model; they do not establish behavioral equivalence of
this replacement dialog with Microsoft Word.

## Evidence and limits

Engine regressions exercise body and table replacement, related-style updates,
source XML retention, pending body edits, invalid replacements, Undo/Redo and
recovery. A real-engine Flutter workflow exercises the dialog, Cancel, replacement,
Undo/Redo and reopening. `scripts/verification/verify_style_replacement.py` reads
engine and UI-produced DOCX files independently with python-docx and lxml.
Dated test and native results are recorded in `VALIDATION.md`.

Build 9 adds missing Heading 1–9 creation and metadata-based built-in deletion
protection; see `BUILTIN_STYLES.md`. Comprehensive localized/legacy built-in
identity, character/table/numbering style authoring, full theme and script
cascades, templates, style organizer and Microsoft Word comparison remain open.
This change does not accept the whole named-styles parity family.
