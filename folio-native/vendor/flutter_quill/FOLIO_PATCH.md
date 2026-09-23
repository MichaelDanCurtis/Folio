# Folio editor extensions

Based on Flutter Quill 11.6.0 from the installed pub.dev package. Only `lib`,
`pubspec.yaml` and the original MIT `LICENSE` are vendored. The omitted example
screenshots are removed from the package metadata. `upstream-sha256.json` records
the copied upstream library bytes. Folio also bundles the upstream license.

The new `QuillParagraphLayout` scope supplies an optional, character-preserving
paragraph transform at the actual available width, after Quill has composed rich
styles, IME decoration and diagnostics. `TextLine` passes it to `RichTextProxy`,
which applies it using Flutter's supported layout callback. Existing callers
without a scope retain their original text.

`QuillLayoutText` uses a RenderParagraph subclass to distinguish layout-only
spacer spans from real child widgets and to choose the closest cursor edge when
clicking a spacer. Ordinary text painting, selection boxes, semantics and inline
widgets continue through Flutter's RenderParagraph. A `QuillInlinePaintSpan`
may paint inside its final character box without adding text or semantic nodes.
Those decorations are recorded once per text/layout change, cached for repaints,
and disposed on invalidation and render-object disposal. Folio uses this for
literal-tab leaders, including native RTL glyph spans. Application tab geometry
lives in `lib/src/document_tabs.dart`, outside this fork. No Flutter SDK files
were modified.

Changes to upstream library files: `lib/flutter_quill.dart`,
`lib/src/editor/widgets/proxy.dart`, `lib/src/editor/widgets/text/text_line.dart`;
new file: `lib/src/editor/widgets/text/paragraph_layout.dart`.

## Paragraph geometry

The same optional scope supplies line padding, text style, strut, and block
spacing callbacks. `text_line.dart` uses them for both rendering and editable
line geometry; `text_block.dart` allows managed paragraphs to own their spacing
instead of adding the block's outer gap a second time. No text characters are
inserted for indentation or spacing. Defaults remain unchanged without callbacks.
Folio resolves direct/inherited Word paragraph properties outside the fork in
`document_paragraph_format.dart`. Tests measure wrapping, caret positions,
hit-testing, line height, tabs, draft history and source-file retention. First-line
and hanging geometry, full bidi, pagination and Word differential results remain
unqualified.

Qualification must cover character/selection invariants, composing, rich text,
clipboard, proofing, mixed directions, line wrapping, inline widgets, semantics,
native input methods, performance and upstream upgrades. A passing unit test or
screenshot does not qualify all those paths.

## Synchronous edit observation

`lib/src/document/document.dart` adds a separate read-only
`Document.synchronousChanges` stream. It publishes each actual, normalized
`DocChange` after composition and history handling, before controller listeners
run. The original asynchronous `changes` stream and its consumers retain their
existing behavior. Closing a document closes both streams. Observers must not
mutate the document during this callback.

Folio's `DocumentEditCapture` composes these operations for bookmark tracking,
covering controller string/Delta replacements, formatting and `compose` calls.
The main editor enables capture only when the current document has bookmarks.
Table drafts capture cell operations and apply their verified logical edits
as one workspace change. The full suite must continue to cover proofing,
clipboard, delayed notifications, history, document replacement and disposal;
native IME/performance qualification remains separate.

## Named-style rendering and document-local rules

The paragraph scope also provides inline-style and alignment callbacks. Folio
uses these to apply inherited Word formatting and explicit false run overrides
before proofreading spans are added. `text_line.dart` no longer treats a false
bold/italic/underline/strike attribute as enabled.

`Document` now creates a local `Rules` instance through `Rules.forDocument()`.
The default rule definitions are shared, but mutable custom rules are local to
the owning document. This prevents cell drafts and other editors from replacing
the main editor's following-style catalog. The regression creates two live
documents with different following styles and edits both before closing one.

## macOS visual-line selection

`ExpandSelectionToLineBreakAction` now uses the rendered line boundaries and
`TextSelection.expandTo`, following Flutter's expand-selection semantics. The
upstream implementation searched for newline characters, accidentally selected
the preceding/following paragraph break, ignored soft wrapping and could shrink
an existing selection. The line geometry code also handles empty visual lines
without indexing an empty selection-box list. The real-workspace macOS regression
covers backward whole-line selection, wrapped-line expansion in both directions,
Backspace preserving surrounding text, and empty paragraphs.

## Paste caret positions

Plain and rich selection replacement place the caret after the inserted content,
using the selection start rather than its former end. The controller's intended
Delta now concatenates rich insertions as their actual character/embed operations
before comparing with document normalization; it no longer treats a whole pasted
Delta as one synthetic embed. Clipboard regressions exercise plain, internal rich
and HTML replacement in both selection directions, retained rich properties,
Unicode text, following input and unchanged neighboring paragraphs.

## Inline line breaks

`Embeddable.lineBreakType` is a one-position inline atom. Its payload remains
opaque to Quill. `Embed.toPlainText` and `TextLine` project it as one U+2028;
Flutter text shaping creates a line break without splitting a Quill paragraph.
It needs no widget embed builder. Ordinary literal U+2028 text remains distinct
in Delta. Inline formatting survives the atom, and the insert rule retains that
formatting when typing after it.

Mixed text/embed rendering now groups original nodes in an ordinary list instead
of cloning them into detached linked lists. Their document offsets are therefore
available to proofreading, link handling and IME composition on both sides of
an inline atom. A regression checks the actual misspelling underline after a
break and applies its correction without removing the break.

`folio-line-breaks.patch` records this change against the qualified 0.73 source
in `release/validation/table-row-flow-native/source/vendor/flutter_quill`.
Focused and real-workspace tests cover Shift+Enter, toolbar insertion, ordinary
Enter input, paragraph/list identity, line/caret geometry, formatting, rich
internal copy, Backspace, search/bookmarks, Apply/Cancel, history and recovery.
Native IME, accessibility and external clipboard qualification remain pending.
