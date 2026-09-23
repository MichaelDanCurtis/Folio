# Document bookmarks — implementation and remaining acceptance

The 0.46 implementation includes the Bookmarks dialog and Go To controls.
Version 0.47 tracks formatted paste and cell drafts.
Native/package qualification is recorded separately in `VALIDATION.md`.
W04 is not accepted complete.

## Current representation

`document.attrs.bookmarks` is an array of `{name, start, end, sourceId?}`.
`sourceId` identifies a source DOCX marker pair, not a user-visible bookmark.
Names are unique ignoring case; authoring permits a leading letter followed
by letters, numbers or underscores, at most 40 characters. Imported names
beginning with an underscore are retained. Unicode naming behavior and limits
still require direct Word comparison.

Offsets count UTF-16 code units in a logical body-story stream. A paragraph or
heading contributes its inline text plus one trailing newline. Table cells,
lists and blockquotes recursively contribute their paragraphs. Other blocks,
including preserved Office objects and live workbook blocks, contribute an
object character and newline. Endpoints must be valid character boundaries
inside modeled paragraphs; they cannot split surrogate pairs. These are
**not Quill offsets**: a whole table is one editor embed but contains multiple
logical paragraphs. `lib/src/document_bookmarks.dart` maps these coordinate
systems, including Unicode boundaries, list paragraphs and table-cell targets.

## Engine commands and edit tracking

`bookmark` accepts `action: set | delete`, `name`, and a mandatory
`baseRevision`. Set additionally needs `start` and `end`; an existing name moves
to the new range. These commands validate before committing, create a workspace
Undo step, and persist in Folio recovery files. Set/delete do not change text.

The `document` operation accepts optional `textEdits`, an ordered array of
`{start, end, text}` in **old logical document coordinates**. The engine checks
that the edits are disjoint, preserve character boundaries and reconstruct the
new document text exactly. A failed check leaves the entire workspace unchanged.
This protocol is needed to distinguish edits in repeated text. Insertion at a
range's opening is included; insertion at its ending is excluded. Insertion
inside expands it. A point stays immediately before insertion at that point.
Partial deletion contracts a range; deleting/replacing its entire contents
removes it. Undo/Redo restore the stored ranges together with document content.

Content replacements that do not include exact edits use a character diff
from pinned `similar` 3.2.0, with a 100 ms bound. This fallback is an inference,
including possible deadline approximation. It cannot prove the intended
location of edits in repeated text or distinguish a cut/move from deletion
and insertion. Main-editor string replacements, rich Delta/internal/HTML paste
and `compose` changes now supply validated exact operations. A separate
read-only synchronous stream in the pinned Quill fork exposes the normalized
operations before controller listeners run; it does not change the existing
asynchronous stream. Capture composes all operations within a notification and
validates both document snapshots and exact reconstruction. It is disabled in
the main editor when there are no bookmarks. Built-in writing corrections,
Find/Replace, AI proofreading and selected-passage proposals retain their
explicit operations.

Table drafts capture each cell's operations synchronously, map them into the
table's logical text, and compose them across cell switches. Apply passes the
verified internal operations to the parent editor instead of treating the
table embed as a wholesale replacement. Final typing is flushed before cell
switches, structure commands, conversion and Apply. Cancel keeps the draft out
of the workspace; Apply is one workspace Undo step. A replaced parent document
invalidates the dialog even when an identical table appears at the same offset.
JSON key ordering is ignored when matching the engine's and editor's snapshots.

Row/column/merge/split transformations and table-to-text conversion still use
the diff fallback. Cut/move and cross-document copying of the bookmarks
themselves, draft-local Undo semantics and all AI fidelity remain separate
qualification gaps; ordinary formatted paste does not prove those behaviors.

## Desktop controls

In Word, choose **Bookmarks…** in the ribbon, or press Ctrl+G / F5. Command+G
retains Find Next on macOS. Name and Location sorting, hidden bookmarks, exact
text previews, Add, Move and Delete are available. Move uses the document
selection captured when the dialog opens. Mutations guard both workspace and
revision; changed context cannot relocate an old selection silently.

Go To closes the dialog, selects the saved range and brings it into view with
editor focus. Bookmarks inside a top-level table open its target cell editor
with the passage selected. Ranges spanning cells open their first cell; nested
table and preserved-paragraph drill-down still need work. Navigation changes
no document content and creates no Undo entry. Changing bookmark metadata no
longer triggers a redundant content save when editor selection changes.

AI `replaceDocument` explicitly clears bookmarks; append retains/rebases them.
Ordinary document replacement cannot inject or overwrite bookmark metadata;
it is derived from the previous authoritative workspace. Imports, workspace
replacement, and Undo/Redo retain their own full document state. Legacy Folio
files hydrate missing bookmark metadata from their preserved source package
and rebase it to current text. An explicit empty array means deletion and is
never hydrated back from the source.

## Word import and export

Ordinary paired body bookmarks import at their logical text locations,
including cross-paragraph and table-cell ranges, points, and hidden names.
Internal `w:hyperlink w:anchor` targets round-trip as `#name` link marks.
An unedited imported document still exports its original package bytes.

Edited export removes only managed source marker pairs, assigns collision-free
IDs, and projects fresh endpoints into their paragraphs. A source paragraph
that has not otherwise changed keeps its run properties and hyperlink wrappers;
the marker injector can split a formatted run without losing those properties.
Generated markers are checked before the package is written. A projection
mismatch returns an ordinary export error instead of panicking across FFI.
Untouched package parts, including headers, remain unchanged.

Column bookmarks, endpoints in preserved objects, unmatched/duplicate IDs,
unsupported names/attributes and other unmodeled source markers are retained
in the source and reported. If an edit would drop or change one of those
markers, export returns an error before writing the destination. This is a
temporary compatibility limitation, not support for authoring those ranges.
Editing a paragraph's text still uses the existing supported rich-text
projection; complete preservation of unmodeled inline formatting is a
separate outstanding Word fidelity requirement.

## Evidence and next gates

`rust/src/document_bookmark_tests.rs` covers authoring validation, Unicode
boundaries, repeated-text exact edits, insertion/deletion boundaries, points,
multiple replacements, paragraph splits, formatting-only edits, stale-command
rejection, history, AI append/full replacement, source imports, crossing ranges,
table cells, hidden bookmarks, internal hyperlinks, untouched run properties,
deletion, legacy hydration, recovery, and unsupported export failures.

The recovery case writes `folio-word-bookmarks-tests` under the system temporary
directory. `scripts/verification/verify_bookmarks.py` independently parses its
source/edited/reopened DOCX packages with ElementTree and python-docx. It checks
UTF-16 ranges, paired markers, text, internal targets, untouched parts and exact
package-entry equality after recovery. Run it with the fixture directory as
its sole argument. This is package verification, not Microsoft Word testing.

Four Dart mapping/edit tests plus a real-engine full-workspace test cover
authoring, invalid names, rapid repeated-text edits, endpoint affinities,
navigation without extra history, stale dialogs, hidden names, sorting,
deletion/Undo, Ctrl+G, export/recovery, table-cell selection, real spelling
corrections and exact Find/Replace. A core proposal regression covers exact
insertion into repeated text and rejection of a different workspace.

Five additional capture/composition cases plus a real-engine workspace case
cover engine/editor key ordering, rich operations, multiple operations before
notification, Unicode, table embeds, internal and external HTML paste, edits
in multiple cells, immediate Apply, Cancel, replacement/deletion, history,
replacement-workspace rejection and export/recovery. The independent
`scripts/verification/verify_bookmark_paste.py` checks the generated DOCX files
and saved workspace, including all ranges, table text, bold runs, unchanged
workbook/settings and package-entry equality after recovery.

Remaining gates include native mouse/keyboard and IME qualification, structural
table tracking, nested table destinations, other stories, advanced
column/object bookmarks, copy/cut/move semantics and reference fields. Full
W04 also needs the remaining search/navigation behaviors in `PARITY.md`.

Behavioral references:
[Microsoft bookmark authoring](https://support.microsoft.com/en-us/word/add-or-delete-bookmarks-in-a-word-document-or-outlook-message),
[Microsoft bookmark editing](https://support.microsoft.com/en-US/Word/troubleshoot-bookmarks),
[Open XML BookmarkStart](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.bookmarkstart?view=openxml-3.0.1).
