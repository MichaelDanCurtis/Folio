# Track Changes development — 0.59

Review > Track Changes opens the revision list for the current document. Import
retains revision XML. The document and review preview show insertion underlines
and deletion strikethroughs. Authors, types and locations can filter the list;
Previous/Next and Accept/Reject act on the displayed changes. Accept/Reject all
shown applies the complete filtered selection as one undoable transaction.
Formatting revisions display previous and current direct properties. Imported
authorship is retained; no author identity is inferred.

This document describes the 0.59 review mechanics. The 0.60 recording controls
and editor integration are described in
[TRACK_CHANGES_AUTHORING.md](TRACK_CHANGES_AUTHORING.md), with scoped native
qualification. Version 0.61 adds the in-document views and navigation described
in [REVIEW_DISPLAY.md](REVIEW_DISPLAY.md). Version 0.62 adds paired named-move
review as described in [TRACKED_MOVES.md](TRACKED_MOVES.md). Full revision authoring, editable
markup, balloons, precise page layout, Compare/Merge and Word review parity
remain open; these development previews do not establish full compatibility.

## Supported mechanics

- Run/content insertions and deletions, including selected nested revisions.
  Rejecting deletion restores deleted text/instruction elements with their
  original attributes, escaping and namespace bindings. An unresolved nested
  deletion remains a deletion when only its outer deletion is rejected.
- Prior snapshots for run, paragraph, table/row/cell/grid/exception and section
  properties. Accept removes the snapshot; Reject restores the previous
  properties. Paragraph mark and section metadata survive pPr restoration.
- Inserted/deleted paragraph marks: removing a break joins the following
  paragraph without removing its text; the surviving paragraph mark supplies
  the properties. Multiple adjacent break changes resolve from the end.
- Inserted/deleted table rows without vertical merges; keeping a row resolves
  its row marker separately from the row's tracked text. Removing the final row
  removes its table; a final nested table leaves a valid cell paragraph.
- Relevant Word story parts identified by content types, including headers,
  footers, notes, comments, styles and numbering. These are listed by location;
  the editor still lacks complete rendering/authoring for those story types.
- Source package lineage, pending ordinary edits, live workbook block identity,
  Undo/Redo and `.folio` recovery. Linked figures still refresh after review.

## Boundaries and atomic checks

Unmatched/unsupported move groups, custom XML revision ranges, table-cell revisions, math
control changes and conflict records are retained and may be unsupported. A
selected unsupported record disables the corresponding bulk action. A final
paragraph mark or a break across an object/section boundary cannot currently be
joined. Rows with vertical merges cannot currently be removed through review.
Enforced document editing protection disables revision acceptance/rejection.

A join that changes or removes a live workbook block fails before commitment;
its binding is preserved. Cross-boundary bookmark/comment/permission/move ranges
that would become unbalanced also fail atomically. Complete annotation deletion,
interleaved fields/content controls and complex revision combinations still need
Word differential qualification. The UI preview is a text/revision preview,
not exact Word pagination or complete rich typography.

Ordinary document changes cannot remove or replace an imported revision block.
Export checks pending source revisions before writing. Older recovered documents
upgrade unchanged revision blocks; actual old user edits are not overwritten.
Temporary identities allow nested bulk operations without stale byte offsets and
are removed before any package is committed or exported.

## Evidence

`rust/src/docx_revisions_tests.rs`, `test/document_revisions_workspace_test.dart`
and `scripts/verification/verify_docx_revisions.py` cover the engine, real Flutter
bridge/editor flow and independent XML inspection. The persistent Microsoft
Open XML SDK validator checks the output packages against Microsoft 365 schemas.
Dated results and native observations are in `VALIDATION.md`.

The fixture/rendered widget preview uses synthetic content. It is not a native
macOS screenshot or evidence of Microsoft Word behavior. No parity family is
accepted complete and no novel AI feature is counted (0/20).

## Primary references

- [Microsoft: accept revisions and revision element semantics](https://learn.microsoft.com/en-us/office/open-xml/word/how-to-accept-all-revisions-in-a-word-processing-document)
- [Microsoft: inserted paragraph mark clarification](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/bd1804da-fd6b-4782-81a0-42c16ce279eb)

The implementation follows the element semantics rather than copying the
simplified sample traversal. Microsoft Word desktop comparison remains open.
