# In-document revision review — 0.61 development

Review > Review in document opens a read-only document page and revision pane.
All markup shows retained insertion/deletion text. Current projects the pending
changes as incorporated; Original shows their earlier text and formatting.
Changing the view does not accept/reject changes or enter document history.
Back to editing restores the existing editor and its selection.

Previous/Next, a revision row or a changed paragraph selects a change and reveals
its source block. Source anchors are retained across paragraph joins, so either
side of a removed paragraph break still locates the merged paragraph. Author
filtering controls navigation, change bars and the revision list; it does not
remove another author's markup from the document. Individual Accept and Reject
use the same workspace/revision/source-key guards as the full Track Changes
window. Workspace Undo/Redo refreshes the page and change list.

The native engine constructs each view from a temporary package copy. Current
and Original resolve supported revision structures only in that copy, including
text, paragraph breaks, property snapshots and row lifetimes. Local edit state,
file contents, recording settings and history remain untouched. Protected files
can be viewed; Accept/Reject remains unavailable. Unknown revision structures
produce an explicit projection error and retain the last displayed mode.
Private source-block attributes never enter the persisted document or export.

The static paragraph/table renderer is shared with table cells. Current/Original
reuse imported paragraph/run properties, tabs, list labels and tables. All markup
still uses the existing textual revision preview for changed blocks; it does not
yet reproduce every source format or changed table layout. Other-story revisions
remain navigable in the pane with an explicit location, but those stories are
not laid out on the main page. A block removed in the chosen projection remains
reviewable in the side preview.

Remaining Word parity includes editable markup, simple markup, inline revision
selection at character precision, full changed-table formatting and geometry,
balloons, pagination, all revision types and stories, persistent view preferences,
complete keyboard/assistive-technology behavior, large-document performance,
and differential checks in Microsoft Word. This surface does not replace those
requirements. No parity family is accepted and the novel AI count remains 0/20.

Verification covers non-mutating projections, exact current/original text and
formatting, merged source anchors, protected/unsupported cases, table rows,
real-engine view switching and review history, narrow layout, distant paragraph
navigation, and byte-identical export after view switches and undone review.
An independent Python ZIP/XML reader inspects the source and serialized views;
Open XML validation checks the exported source. Native package evidence is
recorded separately in VALIDATION.md.

Reference: [Microsoft's review and display behavior](https://support.microsoft.com/en-us/accessibility/word/use-a-screen-reader-to-track-and-review-changes-in-a-document-in-word)
(checked 2026-09-20).
