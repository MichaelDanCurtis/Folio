# Native Track Changes input regression

`tracked-recording.docx` is a synthetic Folio 0.60 native export, not a user
file. Alex's five pending changes replace the forecast, add a paragraph and
format the title. It is independently checked as a valid Word package.

`tracked-recording-enter.json` captures the committed document, proposed
five-paragraph document and exact journal from native Enter after reopening
that file and explicitly choosing Morgan as the next author. The old logical
text has 108 UTF-16 units; the journal inserts a newline at 108. The Rust
regression covers this endpoint, retained final paragraph marks, later typing
and rejecting only Morgan's changes.

The Flutter workspace regression exercises the paired platform-input case:
macOS can provide a new editing value with an insertion after Quill's hidden
final newline. The input client normalizes that position and caret before the
sentinel. The test checks a real new paragraph and continued typing, including
a preceding rejected-link/discard workflow.
