# Named move review — 0.62 development

Imported Word moves expose a source and destination in both review surfaces.
Go to linked location selects the opposite side, clearing filters as needed.
Accept or Reject on either side resolves all members of the named move together,
including a side hidden by a type or author filter. The interface states this
scope beside the controls. Unrelated revisions remain pending. One workspace
Undo restores the whole operation, and the existing save/recovery flow persists
the result.

Word range names link the two sides; revision IDs do not identify pairs. The
engine matches start/end IDs within a story and links exactly one source and
destination range with the same name. Multiple moved runs and paragraph marks
can belong to the same pair. Current and Original project the corresponding
side on a temporary copy. Moved paragraph marks are located at their logical
paragraph boundary rather than their earlier XML property position. A wholly
moved paragraph can disappear at the end of a story; an emptied table cell keeps
its required final paragraph. Partial paragraph moves use the existing guarded
paragraph join behavior.

Review retains namespace-scoped payloads, restores source deleted-text elements
when rejecting a move, and preserves nested deletion markup on the retained side.
Nested changes on content removed by review disappear with their containing
content. Top-level range markers are retained as hidden source blocks during
ordinary document edits and export. Removing only one side through an unrelated
containing revision is rejected atomically.

Malformed, unmatched, duplicate, overlapping, column-scoped, displaced,
text-box or cross-story pairs are preserved with unavailable review controls. Existing
protection and stale-workspace guards still apply. This is scoped review of
imported named moves; it does not author new move revisions or provide Compare,
Merge, cross-story move resolution, full custom XML handling, or complete Word
revision parity. All markup still uses textual previews in changed blocks,
and edit-mode hidden markers may consume an editor line. Exact Word layout,
large-document performance and Microsoft Word differential comparison remain
unqualified. No parity family is accepted; novel AI remains 0/20.

Verification includes paired actions from either side, multiple members, nested
deletions, whole paragraphs at story end/in table cells, note-story isolation,
malformed-range rejection, ordinary edit/export retention, read-only projections,
filtered review, linked navigation, Undo/Redo and saved recovery. An independent
ZIP/XML reader checks exported contents and payload preservation; Microsoft's
Open XML SDK separately checks schema validity. Native evidence is recorded in
VALIDATION.md after its own checks.

References consulted 2026-09-20:

- [Move source ranges and name matching](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.movefromrangestart?view=openxml-3.0.1)
- [Moved run destinations](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.movetorun?view=openxml-3.0.1)
- [Paragraph-mark move semantics](https://learn.microsoft.com/en-us/office/open-xml/word/how-to-accept-all-revisions-in-a-word-processing-document)
