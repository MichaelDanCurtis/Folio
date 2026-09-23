# Page numbers and inline page fields

Version 0.67 adds an Insert page field menu (the number-sign toolbar button) to
rich document, table-cell and header/footer editors. Page number (PAGE), Total
pages (NUMPAGES), Pages in section (SECTIONPAGES) and Section number (SECTION)
insert at the selection. They are atomic inline objects: surrounding text stays
editable and the fields can be formatted, selected, deleted and undone.

The editor displays a labeled field token. Its tooltip distinguishes a saved
result from a value awaiting pagination. New fields carry an unresolved `?`
result and the Word dirty flag. The continuous editor does not calculate physical pages;
a cached Word value is not presented as a current page count. Version 0.68 adds
a separate derived page preview; see PAGINATION.md for its layout scope. The field code
survives export for a paginating application to evaluate.

In Headers & footers, Page number format opens a local draft for the selected
section. Choose numeric, Roman, letter and other supported number formats,
continue from the previous section or a nonnegative starting number, and an
optional chapter heading level/separator. Chapter headings need a numbered
heading style. Numbering belongs to the selected section independently of header
or footer linking. Done updates the dialog draft; the outer Apply commits story
and settings together, and Cancel discards them. Unknown imported format values
and unrelated section attributes remain intact when left unchanged.

Supported simple fields and direct-run complex fields (including split
instruction runs) retain their instruction, cache, run properties, dirty/lock
flags and source XML through surrounding edits. Explicit field formatting writes
a simple field with the selected formatting. Unsupported, nested and malformed
fields remain in preserved paragraphs; simple unsupported fields are no longer
flattened to ordinary text by surrounding editing. Body/table range mapping uses
cached UTF-16 lengths while the editor uses one atom per field. Bookmark/comment
markers at field boundaries are written outside the field structure.

The implementation follows Microsoft's [PAGE field](https://support.microsoft.com/en-us/word/field-codes-page-field),
[section numbering controls](https://support.microsoft.com/en-us/word/customize-page-numbers-and-their-formats-in-different-word-document-sections),
[field-code catalog](https://support.microsoft.com/en-us/word/list-of-field-codes-in-word)
and [page-number properties](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.pagenumbertype?view=openxml-3.0.1).
These are behavior/specification references, not proof of execution in Word.

## Boundaries and verification

Physical pagination, dynamic field recalculation, full field-code editing,
page-number galleries/position presets, margin page numbers, complex nested
fields and direct Word layout comparison remain open. Annotation boundaries
inside an imported field leave that paragraph preserved. Header/footer changes
remain unavailable while recording is enabled or protection is enforced; this
feature does not disable recording. Numeric bounds follow the Open XML integer
storage range; broader Word UI/locale limits still need comparison.

Six engine tests cover source-preserving simple/complex fields, unknown fields,
all four authored types, invalid requests, numbering attributes, linked footer
content with independent numbering, bookmark/comment/table interactions, stale
requests, workspace history and recovery. Four field/dialog tests cover inline
rendering and formatting, multi-digit offset mapping, deletion/Undo, numeric
validation and chapter controls. A real-engine application workflow covers field
menus, draft settings, Apply/Cancel, Undo/Redo, export and reopen. It also exposed
and now covers stale rich-editor focus listeners after read-only transitions.

Final suite, independent file and native results are recorded in VALIDATION.md.
Full Word/Excel parity remains incomplete; novel AI acceptance is still 0/20.

## Preview lifecycle

Keep at most one Folio preview running. Before replacing it, inspect the current
window for unsaved drafts, quit normally, and verify the old process exited.
Launch the replacement only after the process count reaches zero. Use
`scripts/verification/check_preview_processes.py` as the read-only count check.
Preserve saved documents and recovery data when retiring a preview.
