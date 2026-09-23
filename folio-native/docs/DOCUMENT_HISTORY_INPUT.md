# Document input during workspace history

Version 0.53 build 6 records Undo/Redo intent before the first asynchronous save.
Typing can remain visible while history is pending. Its actual rich-text
operations wait in order and are moved through the committed document change;
an older whole-document snapshot cannot overwrite the result of Undo.

Consecutive history commands and edits retain their request order. Insertions
from the earlier history operation precede later typing at the same position;
later explicit formatting wins over the earlier formatting change. Selection
mapping uses the change applied to the visible document, including queued input.
Logical edit journals are retained for bookmarks and edits inside table embeds.

A rejected history command leaves following typing available to commit. A rejected
queued document write keeps the draft and its exact operations, shows **Document
changes not saved**, and prevents Save/Close or further history from pretending
that the draft was saved. The next document edit retries the rejected operations
before submitting the new edit; each successful edit keeps its own history step.

## Evidence

The real-engine workspace tests inject delays both before history (saving) and
inside history dispatch, and also type immediately after the request. They cover
multiple alternating edits/history commands, Unicode, caret placement, untouched
neighboring text, bookmark positions, rejected history, rejected writes, retry,
local recovery and DOCX export. Pure editor checks cover formatting precedence,
rich repeated-text journals and internal table journals through surrounding
text changes. The full Flutter suite passes 228 checks with 2 optional AI skips.

`scripts/verification/verify_history_input.py` independently opens three saved
workspaces and their DOCX exports. Native build 6 separately passes keyboard
Undo then typing, Redo then typing, consecutive Undo commands then typing,
keyboard history after the Save dialog, export and byte-identical restart.
`scripts/verification/verify_native_history_input.py` verifies the retained
native states and export in `../../work/native-history-input`.

## Exact history positions in build 7

Committed document edits now carry their supplied logical edit journal in the
Rust history entry. Undo returns its validated inverse; Redo returns the original
journal. Both use UTF-16 positions in their own source document. Journal memory
counts toward the existing 100-entry/24 MB history limits. Journals remain
session history and do not enter saved document metadata. New/Open clear them.
Measured AI proposal application retains the same journal through its commit.

The editor captures operations even without bookmarks. It accepts a returned
journal only for the matching workspace and before/after revisions, projects
logical positions to real editor boundaries, takes inserted rich content from
the committed document and reconciles formatting after applying those positions.
Inverse edits sharing an insertion point retain their original order.

The pre-fix real-engine regression produced `aXaaa` instead of `Xaaaa` after
undoing an insertion at the start of `aaaaa`. It now passes that case, repeated
emoji and identical paragraphs, including immediate following input, Redo, export
and reopening. Four engine regressions cover inverses, malformed journals with no
bookmarks, mixed workbook commands, measured proposals, history limits and reset.
Four projection regressions cover rich insertion order, embedded-table offsets,
invalid journals and later typing. The complete suites pass 239 Rust checks and
233 Flutter checks, with 2 optional AI-provider skips; analysis is clean.

The packaged build 7 native check also passes rapid Undo/Redo followed by typing
within repeated letters and emoji, keyboard history after DOCX export, preserved
neighboring content and style, and byte-identical restart at revision 56.
Independent DOCX inspection confirms the final text, named style, headings and
list. Evidence is in `../../work/native-exact-history/verification.json`.
The normal build 7 app opens the user's unchanged revision-30 workspace.

## Remaining work

Changes without a supplied journal still use a snapshot comparison. Main-editor
positions inside opaque table content cannot be represented exactly and retain
the object-replacement fallback; mixed object/content structural cases need more
work. Per-command restoration of the original selection after later navigation
is also unfinished. Native IME, broader formatting/history interleaving, other
asynchronous document-refresh commands, embedded-object conflicts, and Microsoft
Word comparison remain unqualified.
These checks do not accept the entire clipboard/history or Word parity families.
