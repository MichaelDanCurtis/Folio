# Physical page preview — 0.68 development

Layout > Page preview derives pages from the current document after pending
edits are flushed. The preview has previous/next navigation, Fit page and
50–200% zoom. Editing remains in the continuous document surface. Previewing
never commits field results, changes source text, or adds an Undo entry.

The Rust snapshot resolves section geometry, page numbering and all linked
header/footer variants against the current Office package. Shared story parts
are projected once per part. The Flutter layout uses the existing native font,
inline-format, paragraph-spacing, line-height and tab-stop helpers. Text is
measured at 96 logical pixels per inch, independently of preview zoom. It flows
at shaped line boundaries and carries the same text painter into each page
fragment. List labels and table cell formatting use the document model.

Implemented page-flow behavior includes:

- Section widths, heights, margins, mirrored margins and gutter geometry;
  next-page and odd/even-page starts, including inserted blank pages.
- Continuous sections sharing compatible geometry on the same page.
- Manual page breaks inside paragraphs, paragraph page-break-before,
  keep-lines, one-paragraph keep-with-next lookahead, and widow/orphan handling.
- Table row groups that keep vertical merges together, cell widths/fills,
  vertical alignment and nested cell text.
- Default, first-page and odd/even header/footer variants, at their stored
  distances. Header/footer height reserves body space when it exceeds margins.
- PAGE, NUMPAGES, SECTIONPAGES and SECTION values in the preview. Fields in
  paragraph fragments use their page location. Decimal, Roman, letter, leading
  zero, dash and hexadecimal formats and supported instruction overrides are
  evaluated; locked fields retain their saved result.
- Repeated layout passes until both page placements and field values settle.
  An eight-pass bound reports unsettled fields rather than claiming convergence.

Imported manual page/column break types and line-break clear flags are now
retained in the document model and export. Paragraph flow flags participate in
existing direct/inherited formatting and source-preserving edits. Paragraphs
containing explicit hard breaks remain preserved blocks in the current editor;
this change does not claim full inline break authoring.

## Remaining layout work

This is Folio's first physical page preview, not completed Word pagination or
printing parity. The native fixture checks below pass; no page counts have yet
been compared against Microsoft Word on a layout corpus. Printing/PDF, paginated
editing, rulers, page-number galleries, large-file cancellation and performance
comparison remain open.

Version 0.69 adds equal and custom column flow, distinct column/page breaks,
reading order and separators; see COLUMNS.md. Version 0.70 adds balancing before
compatible continuous section boundaries; see COLUMN_BALANCING.md.
Chapter prefixes, floating objects, images, footnotes/endnotes,
advanced revisions and complex field codes still require layout work. Unsupported
objects use their retained text representation; layout details identify this
approximation. Continuous sections with different geometry start a new preview
page. Restarted numbering in continuous
sections sharing a page is not complete.

Table header repetition, exact row clipping, oversized row splitting, complete
borders/themes and advanced table layout remain incomplete. An oversized line
or row crosses a page boundary with a layout notice. Full keep-with-next chains,
widow/orphan interactions, contextual and character/line-based paragraph spacing,
first-line/hanging indents, document grids, complex bidi and exact Office font
metrics remain open. The inherited text renderer's limits are surfaced by the
preview. Unsupported numeric formats retain saved field values with a notice.

## Evidence

The current engine suite passes 351 tests. Three added engine checks cover the
immutable preview snapshot, linked stories, pending body edits, Undo, source
fixtures, manual break types and paragraph flow flags. All 295 Flutter tests
pass, with two optional AI-provider checks skipped; static analysis is clean.
The real-engine application check covers pending text, preview navigation,
history, export and recovery. Independent XML checks and the Microsoft365
schema validator pass for the source and edited artifacts.

The isolated 0.68.0 build 1 displays the native fixture as seven pages:
paragraphs 00–06, 07–13, 14–20, 21–27 and 28–34 on pages 1–5, paragraph 35 and
the section ending on page 6, and the landscape section on physical page 7.
Each page repeats the shared header and calculated footer; the final footer
reads `Page v of 7` after the section's numbering restart. This fixture needs
no inserted blank page; odd-page blank insertion is covered by the layout test.
Every native page was visually inspected, followed by 200% zoom, scrolling and
return to Fit page. Previewing leaves recovery bytes identical at revision 1.
Save As retains the entire source DOCX byte-for-byte, including cached field
values of 99. Normal quit, zero-process confirmation, reopen and restored
seven-page preview pass; recovery again remains identical. Exactly one preview
is left running, and the normal user workspace and release are unchanged.

Evidence and source/artifact hashes are in `release/validation/pagination`.
These are scoped Folio checks; Microsoft Word was not executed.
No complete parity family or additional AI feature is accepted by this work.

Primary references:

- [Word line and page breaks](https://support.microsoft.com/en-us/word/line-and-page-breaks)

- [Word section placement](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.sectiontype?view=openxml-3.0.1)
- [Flutter shaped line metrics](https://api.flutter.dev/flutter/painting/TextPainter/computeLineMetrics.html)
