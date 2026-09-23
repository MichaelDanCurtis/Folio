# Document review comments — 0.57 development

Review > New comment, the document context menu, and Command–Option–A
(Control–Alt–M) add a comment at the current selection or insertion point.
Review > Comments shows local threads, author names, dates, replies and the
anchored excerpt. The dialog supports editing plain comment text, deleting a
reply or thread, resolve/reopen, Active/Resolved/All filters, Go to text and
workspace Undo/Redo. Drafts are not posted implicitly; switching threads or
closing with an unposted draft offers Keep writing or Discard.

The reviewer enters a name explicitly. This is an attribution field saved in the
file, not authenticated identity, ownership or a cloud-sharing action. Imported
names and dates are retained. There are no notifications or @mention delivery.

## Data and editing

`document.attrs.comments` stores threads and messages. Ranges share the logical
UTF-16 body-story index used by bookmarks: paragraph text plus one newline,
including list and table paragraphs. Exact checked editor edits disambiguate
repeated characters. Insertion at a range's opening joins it; insertion at its
end stays outside. Removing all anchored text removes its thread. Undo restores
both. Explicit whole-document AI replacement clears comments and is undoable.

Commands require the current workspace identity and revision. Ordinary editor
updates cannot inject or erase comment metadata. A dialog invalidated by a
concurrent workspace change rejects mutation. Comments persist in `.folio`
recovery and participate in the normal document dirty signature and history.
After Undo changes document text, close the dialog and select text again before
creating a new thread; navigation and existing threads still work.

## Word files

The reader follows document relationships, including nonstandard relative paths
and namespace prefixes. Plain body-story comments, point references, replies and
resolved state are projected from `comments.xml` and `commentsExtended.xml`.
Exports write range start/end/reference markers and the `w15:commentEx` parent
and done attributes. The last comment paragraph receives a unique valid
`w14:paraId`. Comment IDs are reserved against source records and body markers.

Unchanged source comment content, unknown attributes, run properties, hyperlinks,
media relationships and unrelated parts are retained. Rich comment content is
shown as text and kept read-only for text editing; reply, status and thread
removal remain available. Deleting a managed reply/thread removes its matching
commentsIds/commentsExtensible metadata, retaining unrelated records.
Unmodeled or malformed conversations remain in the source; body export rejects
an edit that would silently erase their unmanaged anchors. No-edit export returns
the original bytes. Older `.folio` projections hydrate comment metadata from
retained source packages unless they explicitly contain an empty comment list.

## Evidence and remaining work

`rust/src/document_comments_tests.rs` covers commands, Unicode, repeated-text
edits, history, recovery, stale commands, AI replacement, overlapping bookmarks,
paragraph/table ranges, reply/status round trips, alternate part paths/prefixes,
rich-source preservation and deletion of related modern metadata.
`test/document_comments_workspace_test.dart` drives the actual Flutter widgets
and Rust bridge through posting, reply editing, resolve/reopen, Undo/Redo,
navigation, draft retention, stale-command rejection, DOCX reopen and recovery.
The optional render uses real fonts and synthetic test content; it is not a
native-app screenshot.

The independent Microsoft Open XML SDK validator is reproducible with:

```
dotnet run --project scripts/verification/openxml -- /absolute/path/review.docx
```

Its package is pinned to 3.5.1. The generated cross-paragraph/table fixture passes
Microsoft 365 schema validation, including the reply/status metadata. This is
file-structure evidence, not a Microsoft Word application round trip.

Still pending: native mouse/keyboard/accessibility qualification; Word desktop
comparison; contextual margin balloons and continuous inline highlighting;
creating comments directly inside the table-cell editor; rich comment authoring;
cloud identity/coauthoring/@mentions/reactions/tasks; all Word story types and
advanced modern-comment variants. Excel notes and threaded comments remain a
separate missing track. Track Changes, Compare and Merge are not implemented by
this work. W13/D07 and overall parity remain incomplete; novel AI stays 0/20.

## Primary references

- [Microsoft: insert a Word comment](https://learn.microsoft.com/en-us/office/open-xml/word/how-to-insert-a-comment-into-a-word-processing-document).
- [Microsoft: comment reply and resolved attributes](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-docx/9660dacc-2ceb-4352-87d2-42ba1184f522).
- [Microsoft: paragraph identifier constraints](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-docx/a0e7d2e2-2246-44c6-96e8-1cf009823615).
- [Microsoft: Open XML validation](https://learn.microsoft.com/en-us/office/open-xml/word/how-to-validate-a-word-processing-document).
