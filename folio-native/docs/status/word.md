# Word requirements (W01–W17)

States and ID rules: [README](README.md). Family scope: [PARITY](../../PARITY.md#word-requirements) and ROADMAP §4 W-A…W-H.

## W01 Rich text, fonts, symbols and paragraph formatting

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W01.01 | Character formatting: bold, italic, underline, strike, super/subscript, font family, size, colour, highlight | M1 | Native verified | Quill marks since 0.1; DOCX projection ([VALIDATION](../../VALIDATION.md)) | Underline styles, character spacing, small caps, all caps; Word comparison |
| W01.02 | Font dialog with every Word character option (effects, spacing, kerning, OpenType features) | M2 | Specified | — | No document Font dialog |
| W01.03 | Font fallback and substitution for fonts missing on this machine, with a visible notice | M2 | Specified | — | Not built; see font-licensing risk |
| W01.04 | Paragraph alignment, indents (left, right, first-line, hanging) and before/after spacing | M1 | Native verified | 0.52 ([PARAGRAPH_FORMATTING](../PARAGRAPH_FORMATTING.md)) | Implicit hanging indents; Word comparison |
| W01.05 | Line spacing: single, multiple, exactly, at least | M1 | Native verified | 0.52 ([PARAGRAPH_FORMATTING](../PARAGRAPH_FORMATTING.md)) | Word comparison |
| W01.06 | Default and custom tab stops (left, centre, right, decimal, bar) with leaders | M1 | Automated verified | 0.19–0.21 ([VALIDATION](../../VALIDATION.md#0210-paragraph-tab-leaders-2026-09-19)) | Bar tabs; native package check; exact leader typography |
| W01.07 | Ruler showing and editing indents and tab stops | M2 | Specified | — | Not built |
| W01.08 | Line and page breaking controls: widow/orphan, keep with next, keep lines together, page break before | M2 | Specified | — | Not built; see W05.08 |
| W01.09 | Manual line breaks (Shift+Enter) in body, lists, cells and headers | M1 | Native verified | 0.74 ([MANUAL_LINE_BREAKS](../MANUAL_LINE_BREAKS.md)) | IME and list/mixed-format native cases |
| W01.10 | Insert Symbol and special characters (em dash, non-breaking space, optional hyphen) | M2 | Specified | — | Not built |
| W01.11 | Paragraph borders and shading | M2 | Specified | — | Not built |
| W01.12 | Drop caps | M4 | Specified | — | Not built |
| W01.13 | Change Case, Clear Formatting, and the other Home-tab text commands | M1 | Automated verified | Change Case (five cases; Shift+F3 cycles UPPERCASE, lowercase, Capitalize Each Word) and Clear All Formatting (keeps links, lists and Office identity; paragraphs back to Normal), one Undo step each, through DOCX (`document_case.dart`) | The rest of the Home-tab text commands (grow/shrink font, highlighter pen); native check |
| W01.14 | Hyperlinks: insert, edit, remove and follow; external and internal targets | M1 | Automated verified | Ctrl/Cmd+K inserts and edits links to web and email addresses or to headings and bookmarks (a heading gets a hidden `_` bookmark); remove from the dialog or menu; Ctrl/Cmd+click follows only http, https, mailto and in-document links (`document_links.dart`); DOCX round trip | ScreenTips; links in tables and headers; native check |
| W01.15 | Complex scripts and bidirectional paragraphs in the editor | M2 | Specified | — | See D04.07 |

## W02 Styles, themes and templates

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W02.01 | Apply paragraph and character styles from a searchable style picker | M1 | Native verified | 0.53 builds 1–10 ([NAMED_STYLES](../NAMED_STYLES.md)) | Word comparison |
| W02.02 | Create and modify styles: base, next, linked, formatting; dependents refresh | M2 | Native verified | 0.53 ([NAMED_STYLES](../NAMED_STYLES.md)) | Full cascade cases; Word comparison |
| W02.03 | Delete a style and replace an in-use style everywhere | M2 | Native verified | 0.53 build 8 ([STYLE_REPLACEMENT](../STYLE_REPLACEMENT.md)) | Word comparison |
| W02.04 | Built-in Heading 1–9 and Normal created on use; built-in origins protected | M1 | Native verified | 0.53 build 9 ([BUILTIN_STYLES](../BUILTIN_STYLES.md)) | Localized built-in names |
| W02.05 | Direct formatting over styles, including explicit false overrides | M1 | Automated verified | 0.53 ([NAMED_STYLES](../NAMED_STYLES.md)) | Native check |
| W02.06 | Table styles: apply and edit Word table styles | M2 | Specified | — | Table style application is a draft only (0.53) |
| W02.07 | Document themes: colours, fonts and effects, with theme-aware formatting | M2 | Specified | — | Not built |
| W02.08 | Style organizer and copying styles between documents and templates | M3 | Specified | — | Not built |
| W02.09 | Style Inspector / Reveal Formatting | M3 | Specified | — | Not built |
| W02.10 | Attach a template and update styles from it | M3 | Specified | — | Not built |

## W03 Lists and outline numbering

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W03.01 | Bulleted and numbered lists with nesting, Enter continuation and Tab demotion | M1 | Native verified | 0.14 ([VALIDATION](../../VALIDATION.md#014-word-list-numbering-2026-09-18)) | Word comparison |
| W03.02 | Restart, continue and set start value in one Undo step | M1 | Native verified | 0.14 | Word comparison |
| W03.03 | Decimal, letter, Roman and bullet label formats from DOCX numbering | M1 | Automated verified | 0.14 | Picture bullets and other numbering systems |
| W03.04 | Define a new multilevel list: per-level format, indent, alignment, font, suffix | M2 | Specified | — | Not built |
| W03.05 | Lists linked to heading styles (outline numbering) | M2 | Specified | — | Style-linked abstract definitions missing |
| W03.06 | Exact list layout: label indents, tabs, suffixes and fonts | M2 | Specified | — | Not qualified |
| W03.07 | Outline levels 1–9 and body text on paragraphs | M2 | Native verified | 0.53 build 10 ([OUTLINE_LEVELS](../OUTLINE_LEVELS.md)) | Outline view (W17.03) |

## W04 Find, replace, navigation and bookmarks

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W04.01 | Literal find and replace with match case and whole word, Replace All with one Undo | M1 | Native verified | 0.5 ([VALIDATION](../../VALIDATION.md#05-navigation-and-replacement)) | Word comparison |
| W04.02 | Wildcard search | M2 | Specified | — | Not built |
| W04.03 | Find and replace formatting and styles | M2 | Specified | — | Not built |
| W04.04 | Special characters in search (^p, ^t, ^m and so on) | M2 | Specified | — | Not built |
| W04.05 | Search scope: selection only, headers/footers, notes and comments | M2 | Specified | — | Body only |
| W04.06 | Navigation pane: headings, jump, and drag to reorder sections | M2 | Automated verified | 0.5 heading navigation | Drag reorder; pages and results tabs |
| W04.07 | Bookmarks: add, move, delete, sort, hidden, Go To | M2 | Native verified | 0.46–0.47 ([BOOKMARKS](../BOOKMARKS.md)) | Other stories; cut/move copying |
| W04.08 | Go To page, section, line, table, comment and so on | M2 | Specified | — | Only bookmark/table-cell Go To |

## W05 Page layout: sections, margins, columns and breaking

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W05.01 | Page setup per section: paper size, orientation, margins, gutter, header/footer distance | M2 | Native verified | 0.63–0.64 ([PAGE_SETUP](../PAGE_SETUP.md)) | Word comparison |
| W05.02 | Insert and remove section breaks: next page, continuous, odd, even | M2 | Native verified | 0.65 ([SECTION_BREAKS](../SECTION_BREAKS.md)) | Tracked section-break edits |
| W05.03 | Page and column breaks inserted from the ribbon | M2 | Automated verified | Break rendering 0.69 ([COLUMNS](../COLUMNS.md)) | Direct column-break editing; native check |
| W05.04 | Columns: equal/custom widths, spacing, separator, RTL order | M2 | Native verified | 0.69–0.70 ([COLUMNS](../COLUMNS.md)) | Automatic section creation around a selection |
| W05.05 | Column balancing before continuous section breaks | M2 | Native verified | 0.70 ([COLUMN_BALANCING](../COLUMN_BALANCING.md)) | Word comparison |
| W05.06 | Pagination matches Word: page count and break positions on the reference corpus | M2 | Automated verified | 0.68 ([PAGINATION](../PAGINATION.md)) | Word comparison corpus; the rich-text layout risk |
| W05.07 | Print layout editing: type directly on the paginated page canvas | M2 | Specified | — | Editing is continuous; pages only in preview |
| W05.08 | Widow/orphan control and keep with next / keep together applied in pagination | M2 | Specified | — | Not built |
| W05.09 | Hyphenation, automatic and manual | M3 | Specified | — | Not built |
| W05.10 | Line numbering | M4 | Specified | — | Not built |
| W05.11 | Page borders | M3 | Specified | — | Not built |

## W06 Headers, footers, page numbers and backgrounds

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W06.01 | Edit rich headers and footers per section and type (first, odd, even) | M2 | Native verified | 0.66 ([HEADERS_FOOTERS](../HEADERS_FOOTERS.md)) | In-page editing; Word comparison |
| W06.02 | Link to Previous; different first page; different odd/even | M2 | Native verified | 0.66 | Word comparison |
| W06.03 | PAGE, NUMPAGES, SECTIONPAGES and SECTION fields with number format and restart | M2 | Native verified | 0.67 ([PAGE_NUMBERS](../PAGE_NUMBERS.md)) | Chapter numbering cases |
| W06.04 | Headers and footers rendered on each preview page | M2 | Native verified | 0.68 ([PAGINATION](../PAGINATION.md)) | Word comparison |
| W06.05 | Watermarks (text and picture) | M3 | Specified | — | Not built |
| W06.06 | Page colour and background | M3 | Specified | — | Not built |
| W06.07 | Tracked changes inside headers and footers | M3 | Specified | — | Recording header edits missing |

## W07 Tables

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W07.01 | Insert a table; rich multi-paragraph cell editing | M1 | Native verified | 0.15 ([VALIDATION](../../VALIDATION.md#015-word-table-editing-2026-09-18)) | Inline authoring without the draft editor |
| W07.02 | Insert and delete rows and columns | M1 | Native verified | 0.15 | Word comparison |
| W07.03 | Merge and split cells | M1 | Native verified | 0.15 | Arbitrary split counts |
| W07.04 | Nested tables | M2 | Automated verified | 0.15 | Native check |
| W07.05 | Column widths and row heights (auto, at least, exactly) | M1 | Native verified | 0.16, 0.72 ([TABLE_ROW_HEIGHTS](../TABLE_ROW_HEIGHTS.md)) | Equal-distribution native check |
| W07.06 | Drag to resize columns and rows | M1 | Automated verified | Drag an interior column border (the columns on either side share the change) or a row's bottom border (at-least height; exact stays exact); no borders inside merged cells; one Undo step; DOCX round trip (`document_table_resize.dart`) | Resizing the outer table edge; live reflow while dragging; native check |
| W07.07 | AutoFit to contents and window | M2 | Specified | — | Not built |
| W07.08 | Cell shading, vertical alignment and table alignment | M1 | Native verified | 0.16 | Borders editing |
| W07.09 | Cell and table borders | M2 | Specified | — | Not built |
| W07.10 | Convert text to table and table to text; selected rows to text | M2 | Native verified | 0.17–0.18 ([VALIDATION](../../VALIDATION.md#0170-text-and-table-conversion-2026-09-19)) | Selected-row native check; disjoint rows |
| W07.11 | Sort table rows by up to three keys | M2 | Native verified | 0.75 ([DOCUMENT_TABLE_SORTING](../DOCUMENT_TABLE_SORTING.md)) | Remaining options; Word comparison |
| W07.12 | Repeat header rows across pages | M2 | Native verified | 0.71 ([TABLE_HEADERS](../TABLE_HEADERS.md)) | Word comparison |
| W07.13 | Allow rows to break across pages | M2 | Native verified | 0.73 ([TABLE_ROW_SPLITTING](../TABLE_ROW_SPLITTING.md)) | Complex tables |
| W07.14 | Table formulas (=SUM(ABOVE)) and cell references | M4 | Specified | — | Not built |
| W07.15 | Table properties dialog: text wrapping around tables, positioning, cell margins | M2 | Specified | — | Not built |

## W08 Pictures, shapes, text boxes, SmartArt and charts

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W08.01 | Inline pictures render in the editor and preview | M1 | Automated verified | Inline pictures import as `image` nodes and export from their source drawing (`docx_pictures.rs`); shown in the editor (scaled to the line) and the page preview; 44 of the corpus's 56 inline pictures shown, all readable ([CORPUS](CORPUS.md#office-created-corpus-real-files)) | Floating pictures (W08.04); pictures in tables, headers and paragraphs with fields or tracked changes; native check |
| W08.02 | Insert a picture from a file | M1 | Automated verified | Insert Picture: PNG, JPEG or GIF up to 10 MB, stored in the document's package and scaled to the text width in one Undo step; `.folio` save, reopen and DOCX export keep it ([tests](../../rust/src/docx_pictures_tests.rs), `document_pictures_workspace_test.dart`) | Paste and drag pictures; resize, crop and alt text (W08.03); native check |
| W08.03 | Resize, crop and rotate pictures; alt text | M2 | Specified | — | Not built |
| W08.04 | Floating objects: text wrapping, anchors and positioning | M2 | Specified | — | Not built; rich-text layout risk |
| W08.05 | Shapes and text boxes | M4 | Specified | — | Not built |
| W08.06 | SmartArt, rendered and editable | M4 | Specified | — | Not built |
| W08.07 | Charts in documents, rendered and editable, optionally linked to a workbook | M4 | Specified | — | Not built |
| W08.08 | Unsupported objects kept intact on save, with a notice before any destructive change | M1 | Automated verified | Source-part preservation (DOCX projection) | Impact preview (ROADMAP §2) |

## W09 Equations, embedded objects, media and content controls

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W09.01 | Equations (OMML) rendered | M3 | Specified | — | Preserved only |
| W09.02 | Equation editor | M4 | Specified | — | Not built |
| W09.03 | Embedded and linked OLE objects shown as images with an open action | M4 | Specified | — | Preserved only |
| W09.04 | Audio and video | M5 | Specified | — | Not built |
| W09.05 | Content controls: rich text, plain text, checkbox, date, drop-down | M4 | Specified | — | Preserved only |

## W10 Fields, captions, cross-references, TOC and indexes

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W10.01 | Field display and update (F9) for common fields: DATE, PAGE, REF, SEQ | M2 | Automated verified | Page fields 0.67 ([PAGE_NUMBERS](../PAGE_NUMBERS.md)) | General field update (DATE and other fields) |
| W10.02 | Toggle field codes (Alt+F9) and edit a field code | M3 | Specified | — | Not built |
| W10.03 | Captions for tables and figures with automatic numbering | M2 | Native verified | 0.82 native check, 2026-09-26 ([NATIVE_CHECKS_2026-09](../NATIVE_CHECKS_2026-09.md)): Insert Caption gives "Figure 1:" as a SEQ field; rank 14 ([plan](../../../docs/superpowers/plans/2026-09-26-references.md)) | Word comparison; table captions checked by automated tests only; chapter numbers in captions; captions for pictures by position |
| W10.04 | Cross-references to headings, bookmarks, captions and notes that update | M2 | Native verified | 0.82 native check, 2026-09-26 ([NATIVE_CHECKS_2026-09](../NATIVE_CHECKS_2026-09.md)): a cross-reference to a caption inserts REF with a hidden `_Ref` bookmark; rank 14 ([plan](../../../docs/superpowers/plans/2026-09-26-references.md)) | Word comparison; references to headings and bookmarks checked by automated tests only; references to notes and numbered paragraphs; the editor does not get focus back after Insert |
| W10.05 | Table of contents: insert, update, styles and levels | M2 | Native verified | 0.82 native check, 2026-09-26 ([NATIVE_CHECKS_2026-09](../NATIVE_CHECKS_2026-09.md)): TOC field inserted, Update Fields picks up a renamed heading; rank 14 ([plan](../../../docs/superpowers/plans/2026-09-26-references.md)) | Word comparison; hyperlinked entries; custom styles; in the page preview and PDF the level-1 page number is not at the right tab stop |
| W10.06 | Index: mark entries, insert and update | M4 | Specified | — | Not built |
| W10.07 | Table of authorities and table of figures | M4 | Specified | — | Not built |
| W10.08 | Missing targets show visible errors | M2 | Native verified | 0.82 native check, 2026-09-26 ([NATIVE_CHECKS_2026-09](../NATIVE_CHECKS_2026-09.md)): deleting the caption and running Update Fields shows “Error! Reference source not found.”; rank 14 ([plan](../../../docs/superpowers/plans/2026-09-26-references.md)) | Word comparison |

## W11 Footnotes, endnotes, citations and bibliography

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W11.01 | Footnotes and endnotes: display, insert, edit, number, and place on pages | M2 | Automated verified | Rank 14: references import as inline notes (they were dropped before); insert, edit and number; new notes parts created (`docx_notes_tests.rs`) | Notes at the bottom of their page in the preview (W11.05); formatting inside an edited note |
| W11.02 | Note options: numbering format, restart, position, convert | M3 | Specified | — | Not built |
| W11.03 | Citations and bibliography with styles (APA, MLA…) | M4 | Specified | — | Not built |
| W11.04 | Reference-manager integration (Zotero, EndNote) | M5 | Specified | — | Not built |
| W11.05 | Footnotes placed at the bottom of their page in preview and print, continuing on the next page when long | M2 | Specified | — | Page-aware layout (rank 13) |

## W12 Proofing

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W12.01 | Continuous local English spelling with underlines | M1 | Native verified | 0.4 onward ([VALIDATION](../../VALIDATION.md#spelling-and-grammar-verification)) | Accuracy corpus (ROADMAP §5); timing budget |
| W12.02 | Right-click suggestion replaces exactly the word, keeps formatting, one Undo, divider before other commands | M1 | Native verified | 0.4.1 ([VALIDATION](../../VALIDATION.md#041-spelling-menu-correction)) | Tables, headers and footers; keyboard and screen-reader paths |
| W12.03 | Personal dictionary: add, ignore and learn, persisting across restarts | M1 | Automated verified | Personal dictionary (PARITY W12) | Native check; Undo of learn |
| W12.04 | Local grammar checking with declared rule categories | M1 | Automated verified | Harper checks | Precision/recall on the held-out corpus |
| W12.05 | AI grammar review of a selection or document with reviewable proposals | M1 | Automated verified | Reviewed AI grammar edits (PARITY W12) | Real-model evaluation; protected-fact corpus |
| W12.06 | Proofing language per paragraph; spelling in other languages | M4 | Specified | — | English only |
| W12.07 | Thesaurus | M3 | Specified | — | Not built |
| W12.08 | Translation of a selection or document | M4 | Specified | — | Not built |
| W12.09 | Word count dialog (pages, words, characters, paragraphs, including notes) | M1 | Automated verified | Word Count from the status bar or Ctrl+Shift+G: pages and lines from the page layout, words, characters with and without spaces, paragraphs, for the document or the selection; text boxes, footnotes and endnotes from the source package (`document_word_count.dart`, `docx_note_text.rs`) | Lines for a selection; notes added in Folio (notes are read from the source package); native check |
| W12.10 | AutoCorrect and AutoFormat as you type (quotes, lists, capitalisation) | M2 | Specified | — | Not built |

## W13 Comments, track changes, compare and merge

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W13.01 | Comment threads on body text: add, reply, edit, resolve, reopen, delete, navigate | M1 | Automated verified | 0.57 ([COMMENTS](../COMMENTS.md)) | Native qualification; margin balloons |
| W13.02 | Comments in tables, headers, footers and notes | M2 | Specified | — | Body only |
| W13.03 | Review imported revisions: insertions, deletions, formatting; accept/reject one or filtered all | M2 | Native verified | 0.59–0.61 ([TRACK_CHANGES](../TRACK_CHANGES.md), [REVIEW_DISPLAY](../REVIEW_DISPLAY.md)) | Other revision types; balloons |
| W13.04 | Record tracked changes while typing and formatting, with author identity | M2 | Native verified | 0.60 ([TRACK_CHANGES_AUTHORING](../TRACK_CHANGES_AUTHORING.md)) | Structures, annotations, performance |
| W13.05 | Markup views: All, Simple, No markup, Original | M2 | Automated verified | 0.61 ([REVIEW_DISPLAY](../REVIEW_DISPLAY.md)) | Simple markup; balloons |
| W13.06 | Tracked moves: review, accept and reject as a linked group | M2 | Automated verified | 0.62 ([TRACKED_MOVES](../TRACKED_MOVES.md)) | Authoring moves; native check |
| W13.07 | Compare two documents | M3 | Specified | — | Not built |
| W13.08 | Combine revisions from several authors | M4 | Specified | — | Not built |
| W13.09 | Tracked table-structure changes (rows and cells inserted or deleted) | M3 | Automated verified | Simple-row cases 0.59 | General structural tracking |

## W14 Mail merge, labels and envelopes

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W14.01 | Mail merge from a workbook: recipients, merge fields, preview, finish to a document | M4 | Specified | — | Not built |
| W14.02 | Merge to email and PDF | M5 | Specified | — | Not built |
| W14.03 | Labels and envelopes | M4 | Specified | — | Not built |
| W14.04 | Merge rules (IF, Skip Record If) | M5 | Specified | — | Not built |

## W15 Forms, restricted editing, protection and macros

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W15.01 | Restrict editing: read-only, comments only, tracked changes only, form filling | M4 | Specified | — | Protected files are view-only in review (0.61) |
| W15.02 | Legacy form fields and ActiveX controls | M5 | Specified | — | Not built |
| W15.03 | Mark as final and document inspector | M4 | Specified | — | Not built |
| W15.04 | VBA macros in DOCM: preserved, and run where supported, with trust controls | M5 | Specified | — | Architecture decision (STATUS risks) |

## W16 Document format fidelity

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W16.01 | DOCX import and export of supported content, keeping unknown parts | M1 | Automated verified | Many versions; independent readers and schema checks ([VALIDATION](../../VALIDATION.md#independent-office-readers)) | Word round trip: open in Word, edit, reopen in Folio |
| W16.02 | DOTX templates with template semantics | M2 | Automated verified | 0.56 ([TEMPLATES](../TEMPLATES.md)) | Word comparison |
| W16.03 | DOCM and DOTM: macros preserved | M4 | Specified | — | Rejected today |
| W16.04 | DOC (Word 97–2003) open, with conversion | M4 | Specified | — | Not built |
| W16.05 | RTF open and save | M4 | Specified | — | Not built |
| W16.06 | ODT open and save | M4 | Specified | — | Not built |
| W16.07 | HTML and plain text open and save | M3 | Specified | — | Not built |
| W16.08 | PDF open for editing (reflow) | M5 | Specified | — | Not built |
| W16.09 | Per-feature compatibility matrix: open, display, edit, save, round-trip, print | M1 | Automated verified | Rank 10: [COMPATIBILITY](COMPATIBILITY.md), generated from the engine table and checked by `compatibility_tests.rs`; the Compatibility Report lists the open file by state | Word comparison of the Verified rows; Print waits for printing (rank 15) |
| W16.10 | Word-created documents from the corpus open and round-trip with their text | M1 | Automated verified | 92/96 open; all 92 round-trip; 78 at ≥99% word overlap ([CORPUS](CORPUS.md#office-created-corpus-real-files)) | Two rejections; text in shapes and fields |

## W17 Views, zoom and accessibility checking

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| W17.01 | Continuous authoring view with page preview | M1 | Native verified | 0.68 ([PAGINATION](../PAGINATION.md)) | Print layout editing (W05.07) |
| W17.02 | Zoom in the editor: percentage, page width, whole page | M1 | Automated verified | 10–500% from the status bar, Command/Ctrl with + and −, Page Width and Whole Page; the page is laid out unscaled and drawn scaled, so clicks, the caret and scrolling follow; kept per document for the session (`scaled_box.dart`) | A Draft view to zoom; pinch zoom; native check |
| W17.03 | Outline view: promote, demote, collapse and move headings | M3 | Specified | — | Not built |
| W17.04 | Draft, web layout, read mode and focus mode | M3 | Specified | — | Not built |
| W17.05 | Split window and side-by-side documents | M4 | Specified | — | Not built |
| W17.06 | Accessibility checker with fixes | M3 | Specified | — | Not built |
| W17.07 | Show/hide formatting marks (¶) | M1 | Automated verified | Show/Hide ¶ (Ctrl/Cmd+Shift+8) paints paragraph marks, spaces, tabs and line breaks from the editor's caret geometry; never copied, saved or counted (`document_formatting_marks.dart`) | Marks inside tables (edited in a dialog) and hidden text; native check |
