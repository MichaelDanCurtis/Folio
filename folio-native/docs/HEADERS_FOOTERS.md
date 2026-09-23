# Headers and footers

Version 0.66 adds Layout > Headers & footers. Choose a section, Header or
Footer, and Default/Odd, First or Even pages. Rich text and supported existing
tables/objects use the document codec. Apply commits the selected story and its
page settings in one workspace operation. Cancel leaves the document unchanged.
Switching a dirty draft offers Apply, Discard draft or Keep editing. Focused
keyboard Undo/Redo edits the draft; committed changes use workspace history.

Link to Previous is independent for each header/footer and page type. Editing
linked content updates its owner and following linked sections; the dialog lists
the affected sections. Unlink creates an independent copy. Relink previews the
previous content and explains that Apply replaces the local story. Toggling back
retains the uncommitted local draft. Different first page belongs to the selected
section; Different odd and even pages is a document setting. Hidden variants can
be edited and stored before enabling their display.

Shared styles and numbering remain available to story edits. Original package
relationships and opaque content are retained through the supported codec.
Unlinking copies relationships, gives colliding bookmark names distinct names,
and retargets local hyperlinks and supported REF/PAGEREF/NOTEREF/HYPERLINK/TOC
bookmark references, including split instruction runs. Body drafts, live workbook
bindings and body comments/bookmarks survive the package update.

The behavior targets Microsoft's [Link to Previous](https://support.microsoft.com/en-us/word/link-to-previous),
[first-page controls](https://support.microsoft.com/en-us/word/change-or-delete-a-header-or-footer-on-a-single-page),
[header-reference inheritance](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.headerreference?view=openxml-3.0.1)
and [bookmark restrictions](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/da8b8f59-2af1-49d9-8553-f0c1ff2fb18d).
These sources are specifications, not execution evidence from Word.

## Boundaries

The editor remains a continuous body view; it does not show repeated page headers
or calculate physical pagination. Page numbers, field calculation, watermarks,
background authoring and full header/footer layout are still open. Preserved
objects are not equivalent to editable support. Copying complex drawing/comment/
revision relationships needs a broader compatibility corpus and direct Word
qualification. A live workbook block cannot be inserted in a header/footer yet.

Protected documents are read only. Recording header/footer edits is unsupported:
the user must turn recording off explicitly in Review. Imported recording requests
still require an explicit author. Stale workspace revisions and story keys are
rejected without mutation; failed Apply retains the draft.

## Verification

Six focused engine tests cover inherited and independent content, relationships,
first/even variants, numbering, cloned bookmark references, stale/protected/
recording guards, body drafts, live data, annotations, exact Undo/Redo and recovery.
Four standalone dialog tests cover draft history, relink preview, scope changes,
failed reads/Apply, disabled states and a smaller desktop window. A real-engine
Flutter workflow covers Cancel, unlinking/editing, first/even settings, a first-page
footer, body preservation, workspace history, DOCX export and reopen.

342 Rust and 280 Flutter tests pass, with two optional AI checks skipped;
analysis is clean. Seven engine/editor DOCX artifacts and four native artifacts
pass independent XML and Microsoft365 schema checks. The isolated 0.66.0 build 1
preview passes native linked/unlinked editing, first/even content, draft Cancel
and history, separate saves, post-Save Undo/Redo and byte-identical restart.
Native Command-Up caret positioning needs follow-up; Command-Left works. See
VALIDATION.md and `release/validation/headers-footers` for scoped evidence.
Microsoft Word was not executed.
Full Word/Excel parity is incomplete; no whole family is accepted and novel AI
remains 0/20.
