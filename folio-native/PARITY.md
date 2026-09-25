# Word and Excel parity ledger

> **Current status lives in [docs/STATUS.md](docs/STATUS.md)** and the child-requirement register under [docs/status/](docs/status/README.md). This ledger remains the family map and the release-by-release history.

The goal remains full Word and Excel parity, followed by at least 20 implemented, verified AI features unavailable in Word and Excel. A working prototype, preserved ZIP part, passing unit test, or limited feature slice does not satisfy that goal. Overall status: **incomplete**. Additional AI features accepted toward the 20-feature requirement: **0**.

## Acceptance rules

Current 0.82 development adds cross-workbook scenario Merge from selected
XLSX/XLTX files, a frozen read-only source preview, guarded copy/cancel/retry,
custom number-format remapping and Unicode-safe duplicate-name suffixes.
Four new engine cases, three dialog cases and a real application file-picker,
copy/Show/history/recovery/XLSX roundtrip pass, with independent content checks
and seven schema-valid files. The single 0.82 desktop preview passes native
preview/Cancel/copy, duplicate names, application/copy Undo/Redo, custom-format
transfer, summary, save/export, restart and reimport. Fifteen native checkpoints
and three XLSX files pass independent/schema checks. Direct Excel comparison,
broader source/protection/large-workbook cases remain pending. No entire parity
family or novel AI feature is accepted.

Current 0.81 development adds worksheet Scenario Manager with Add/Edit/Delete,
current-value capture, Show with measured history and formula-replacement
acknowledgement, same-workbook Merge and static Summary sheets. Nine scenario
engine cases, four dialog cases, a real Flutter/Rust application test and an
additional font-export regression pass. Independent recovery/XLSX checks and
nine-file Microsoft 365 schema validation pass. The single 0.81 native preview
passes Add/Cancel/Edit, Show/Undo/Redo, same-workbook Merge, Delete/Undo/Redo,
Summary, save/export, restart and reimport/application checks. Eighteen native
checkpoints and two saved XLSX files pass independent and schema checks. Direct Excel
comparison, cross-workbook Merge, PivotTable reports, full protection/names,
large-workbook cancellation and remaining X14 tools are still open. See
docs/SCENARIOS.md. No entire parity family or novel AI feature is accepted.

Current 0.80 development adds Data > Goal Seek for one changing input across
worksheets. Incremental isolated trials, Stop/Cancel, review, explicit best-value
acceptance, stale guards, normal validation and measured Apply/history are
implemented. All 391 Rust and 386 Flutter tests pass with two optional AI skips;
analysis and independent recovery/XLSX checks pass. The single 0.80 preview
passes native cross-sheet preview/Cancel, Apply, Undo/Redo, export/save, restart
and reimport, with eleven captured states and two schema-valid XLSX files.
Direct Excel comparison, broader numerical/volatile/table cases and the other
what-if tools remain unfinished. See docs/GOAL_SEEK.md. No entire parity family
or novel AI feature is accepted.

Current 0.79 development adds the persisted, nonmodal Watch Window and Formulas
tab. Watches update from the workbook calculation, navigate/inspect across
sheets, resize, support multiple removal and history, survive recovery/XLSX,
and follow row/column edits. Old recovery hydration preserves intentional
removals and does not add empty metadata to files without watches. All 381 Rust
and 381 Flutter tests pass with two optional AI skips; analysis, independent
recovery/XLSX checks and five-file Microsoft 365 schema validation pass. The
single 0.79 preview passes native watch addition/removal, live recalculation,
keyboard history, inspection/range navigation, export/reimport and restart.
Fourteen captured states and two native XLSX files pass independent checks and
schema validation. Floating/other-edge docking, names, external workbooks and direct
Excel comparison remain open. See docs/WATCH_WINDOW.md. No entire parity family
or novel AI feature is accepted.

Current 0.78 development adds Formula inspector with direct AST-based
precedents/dependents, cross-sheet and sparse-range navigation, inspection
history, paged results and explicit unresolved-reference reporting. It pins
workbook/calculation state and preserves pending edits and history. All 375 Rust
and 377 Flutter tests pass with two optional AI skips; analysis, independent
XLSX/recovery checks and Microsoft 365 schema validation pass. Native 0.79 also
verifies inspection/Back, a direct dependent and exact source-range navigation.
Arrows, dynamic targets, Evaluate Formula, complete Watch Window behavior and
direct Excel comparison remain open. See docs/FORMULA_AUDITING.md.
This contributes to X10; no entire family or novel AI feature is accepted.

Current 0.77 development adds standard desktop controls for cell editing and
explicit range selection, worksheet F8/Finish/Escape mode handling, and stale
menu/selection guards across worksheet changes. All 373 Flutter tests and nine
focused checks pass, with two optional AI skips; analysis is clean. Native 0.79
verifies worksheet Edit cell, typed cancellation, F8, Finish selection and
range retention in Add watch. Document-table checks remain pending. See docs/CELL_ACTIONS.md. Native
custom-action dispatch, full accessibility and complete parity remain open.

Current 0.76 development fixes cell accessibility activation after a native
Export shortcut and retains worksheet Shift-selection intent across pending
commits. Explicit range and Edit cell actions are tested, including rejected
save retry. All 368 Flutter tests pass with two optional AI skips; analysis is
clean. The single 0.76 preview passes post-Export single-cell selection in both
editors, table Select cells, worksheet Shift-arrow/release, edit cancellation,
byte-identical Word export and restart. Custom semantics handlers pass widget
tests but native Mac delivery remains unfinished. Five captured states retain
identical workspace bytes. See docs/CELL_SELECTION_ACCESSIBILITY.md.
This contributes to D03/D04; no family or novel AI feature is accepted.

Current development adds document-table sorting with three text/number/date
keys, heading/range choices, native locale collation, exact decimal comparison,
draft preview/Cancel/Apply, selected-cell following, and row-aware bookmark/comment
rebasing. All **368 Rust and 364 Flutter tests pass**, with two optional AI skips;
Foundation collation, independent DOCX checks and seven-file schema validation
pass. The single 0.75 preview passes native sorting, typing in the moved cell,
annotation navigation, Cancel, ordinary/post-Save history, export, restart and
DOCX reimport. Sixteen states, four byte-identical outputs and five native schema
files pass independent checks. Swedish/German previews pass; modifier/focus and
input-method anomalies, remaining options/complex structures and direct Word
comparison remain open.
See `docs/DOCUMENT_TABLE_SORTING.md`. No family or novel AI feature is accepted.

The preceding 0.74 qualification closes the preserved-paragraph gap for ordinary
manual line breaks in body/list paragraphs, cells and header/footer stories.
Shift+Enter and the toolbar insert editable inline breaks; DOCX keeps paragraph
identity, break formatting and explicit ordinary-break metadata. Tests exercise
proofreading after breaks, text/search/bookmark coordinates, two-line/caret
geometry, internal rich-copy content, draft and workspace history, Cancel/Apply,
export/reimport and recovery. **365 Rust and 356 Flutter tests pass; two optional
AI checks skip.** Independent readers and Microsoft 365 schema validation pass
for the focused Office artifacts. The single packaged 0.74 preview passes native
body/cell/header input, spelling replacement after a break, Cancel/Apply,
ordinary and post-Save keyboard history, page preview, export/reimport and
restart. Twenty-nine captured states pass independent comparisons; four native
outputs are byte-identical and all five source/output files pass schema checks.
See `docs/MANUAL_LINE_BREAKS.md`. Native list/mixed-format and input-method
coverage, formal accessibility, external clipboard and Word comparison remain.
No entire family or novel AI feature is accepted by this change.

This is a growing requirements inventory, not a definition that excludes unlisted Office features. Expand each family into concrete commands, options, formats, behaviors, accessibility paths, and error cases as it is implemented. The comparison baseline is current Microsoft 365 desktop Word and Excel, including Windows-only capabilities; desktop-first/macOS-first delivery does not remove those capabilities from the objective. Recheck the moving Microsoft baseline before final acceptance.

Every completed requirement needs a reproducible scenario, native UI evidence, saved/reopened behavior, appropriate undo/recovery coverage, and Office import/edit/export comparison where applicable. Separate implementation, automated evidence, packaged-app evidence, and actual Word/Excel comparison. An opaque object retained on export is not editable parity. Functional equivalence needs to cover workflows and results, not just matching ribbon labels. Performance and accuracy claims need measurements. No entire family below is accepted as complete yet.

Source baseline reviewed 2026-09-17:

- [Word desktop feature families](https://support.microsoft.com/en-us/word/word-features-comparison-word-for-the-web-vs-desktop).
- [Word find and replace](https://support.microsoft.com/en-us/word/training/find-and-replace-text-in-word).
- [Excel find and replace](https://support.microsoft.com/en-us/excel/get-started/find-or-replace-text-and-numbers-on-a-worksheet).
- [Excel Copilot baseline](https://support.microsoft.com/en-us/excel/copilot/get-started-with-copilot-in-excel).

These sources guide implementation; they are not an exhaustive product specification. Detailed Office format specifications and behavior comparisons must be added to each format-related workstream.

## Shared desktop requirements

| ID | Requirement | Current evidence and outstanding work |
| --- | --- | --- |
| D01 | Native desktop application, document/workbook windows and Office-like interaction | Flutter/Rust macOS app; single workspace window. Multi-window, document tabs, task switching, window recovery incomplete. |
| D02 | Open/new/save/save-as/recent files, templates, autosave, recovery and version history | `.folio` autosave, save copy, recovery snapshots and Office import/export. 0.55 development adds a searchable recent-file/pin list and recovery browser with immutable copies, new-identity restore, and unsaved whole-workspace drafts; native qualification remains pending. 0.56 development adds DOTX/XLTX create/edit/save workflows, starter and personal galleries, package-type conversion and template-source protection. Global/default/macro/legacy templates, timed/cloud version history and full recovery management remain incomplete. Build 11 adds native macOS File commands and shared keyboard/ribbon handling; Save Copy waits for queued history, New/Open retain snapshots, and duplicate/modal file operations are blocked. Native Open, Save, New, exports, post-Save history and restart pass. In-place Save/Save As, Finder associations and full file lifecycle remain open. See docs/FILE_COMMANDS.md. |
| D03 | Clipboard, drag/drop, undo/redo and command shortcuts | Shared workspace undo and range TSV. 0.11.1 fixes immediate grid selection, inline key bubbling and macOS grid responder restoration; native repeated arrows/Tab and full-text commit passed in the isolated preview. Modifier fallback passes in-memory AppKit checks; the isolated 0.11.1 package subsequently passed Shift+Tab, Shift-arrow/release, Command-F/Z/Q, caret editing, Return and Escape cancellation. During 0.17 conversion qualification, a Command-Z attempt with only the window focused after native Save did not change the revision; toolbar Undo/Redo passed. 0.17.1 traced this to an active/key macOS window retaining itself as first responder after an accessible dialog text field closes; guarded native routing now recovers that state. Packaged typed-field Cancel followed by keyboard Undo/Redo and Apply followed by ordinary typing passed. Remaining post-Apply Undo, Save return and restart qualification was interrupted by a locked Mac. 0.17.2 routes workspace keyboard Undo/Redo to the focused field when it owns draft history and resets formula history across cell/committed-value changes. A real-engine widget regression covers prompt, Find/Replace, dialog, formula and inline cell drafts, exhausted history, preserved saved workspace, and committed document/grid history. Packaged acceptance is still pending. 0.17.3 fixes the app-side rapid first-edit history loss with an immediate initial draft snapshot and retained pending text for Redo. All app-authored plain fields use the adapter; focused tests cover immediate typing, focus changes, form callbacks/formatters and composition commit/cancel. The real workspace regression no longer waits to seed field history. Packaged 0.30 worksheet fitting now passes Command-Z/Command-Shift-Z immediately after the native Excel Save panel, plus ordinary fit history; saved-workspace comparisons confirm exact restoration. Document conversion post-Save and native input-method/menu verification, plus a causal diagnosis of the earlier native dialog symptom, remain pending. Build 11 verifies native Command-O/N/S/Shift-E and File-menu state across both editors and dialogs. Do not claim complete global shortcut coverage. 0.53 build 6 fixes typing during asynchronous document Undo/Redo with ordered rich edits, bookmark journals and visible failure/retry handling. Build 7 retains exact history edit positions within repeated text and emoji. 234 Flutter checks and 241 Rust tests pass; native rapid Undo/Redo typing, post-Save history, export and restart pass. Original selection restoration after later navigation, opaque embed internals and untracked edits, broader rich clipboard, drag/drop and IME coverage require expansion. |
| D04 | Full screen-reader/keyboard accessibility, contrast, zoom and localization | Native widgets only; no formal VoiceOver/NVDA audit. Locale/RTL/IME coverage incomplete. |
| D05 | Print, print preview, page setup and accessible PDF export | Word section Page setup is implemented in 0.63; see W05. 0.68 adds document page preview with shaped line flow, section geometry, repeated stories and derived page fields. All 351 Rust and 295 Flutter checks pass, with two optional AI-provider skips. The isolated build passes seven-page native navigation, landscape/restarted numbering, zoom/scroll, unchanged DOCX export and byte-identical recovery after restart. See docs/PAGINATION.md and release/validation/pagination. Microsoft Word layout comparison, printing, workbook page setup and accessible PDF export remain incomplete. |
| D06 | Security/protection, encryption, signatures and trusted automation | macOS sandbox retained; Office file protection/signatures and macro security not implemented. |
| D07 | Collaboration, comments, sharing, coauthoring and conflict recovery | 0.57 implements local document review threads with replies and resolve/reopen; see docs/COMMENTS.md. Native review qualification, cloud sharing and coauthoring remain incomplete. |
| D08 | Windows/Linux/macOS packaging, updates and distribution | Apple Silicon local ad-hoc build only. Other platforms, signing/notarization, update/rollback missing. |
| D09 | Large-file responsiveness, memory/startup benchmarks, cancellation | 0.13 replaces eager finite row/column span metrics with sparse indexed layout. Regression checks bound cell construction and dimension lookups at the first and last Excel cells, including custom dimensions, hidden positions, frozen clipping and content shrink at the scroll limit. A single fresh-launch sample fell from 933 MiB to 331 MiB RSS (64.5%); native last-cell edit/Undo, return navigation, frozen vertical scrolling, resizing/Undo and restart passed. 0.16.1 traces established a stale-hidden Flutter lifecycle state despite an active, key, visible window. Event-driven foreground recovery passes native menu activation and built-in/external display transfers, plus fresh external-display spelling Apply/Undo/Redo/reopen. Synthetic native checks cover inactive/minimized/closed gates, screen/session suspension and teardown. Physical sleep/wake, locking, display disconnection and final native minimize/restore qualification remain; background automation can leave the app inactive and is not evidence of an active-window failure. Representative corpus, load tests, peak memory, startup/cancellation and measured comparison with Office remain outstanding. |
| D10 | Office extensibility, automation, connections and assistive integrations | Codex dynamic tools/A2UI supported; Office automation/add-in API compatibility missing. |

## Word requirements

| ID | Requirement | Current evidence and outstanding work |
| --- | --- | --- |
| W01 | Rich text, fonts, languages, symbols, paragraph formatting | Common inline marks/headings/alignment through Quill. 0.19 adds measured default tab advances in the Quill editors, preserving literal characters and offsets; renderer, input/composition/clipboard, proofing, wrapping and source-file checks pass. 0.20 adds imported default spacing, direct/document-default/paragraph-style/numbering-level tab layers, explicit inherited clears, left/center/right/decimal controls, rich paragraph previews, multi-paragraph Apply/Cancel and atomic workspace Undo. Settings survive content/proofreading/AI changes and legacy recovery; alternate settings/styles part paths and prefixes are tested. Static table previews now share tab shaping. Engine, native-bridge widget and independent DOCX-reader checks pass; native package and Word comparison remain pending. 0.21 adds all six Word leader choices, character-preserving leader painting in editors/table previews, cached repainting, and pixel/semantics/history/clipboard/proofing/round-trip tests. 0.52 adds selected-paragraph and table-cell indents, before/after spacing and automatic/exact/minimum line-height controls, inherited/direct DOCX metadata, character-preserving rendering, spacing collapse, tab origins and Enter propagation. Native paragraph Apply, keyboard Undo/Redo, Cancel, DOCX export, restart, table draft Apply and table keyboard history passed in the isolated check. See docs/PARAGRAPH_FORMATTING.md and VALIDATION.md for scoped evidence. Exact leader typography, ruler/bar controls and rendering, implicit hanging-indent and table-style stops, inheritance after changing styles/list levels, paragraph indent/justification interactions, overlong aligned fields, complete bidi/inline-object cases, native qualification, advanced typography, font substitution and remaining paragraph options are incomplete. |
| W02 | Named styles, themes, templates, style inheritance and organizer | 0.53 build 10 adds a shared source style catalog, base/next/link declarations, create/edit/apply/delete transactions, a searchable style picker/editor, dependent paragraph/tab refresh, inherited run rendering, explicit false overrides, next-style Enter behavior, conventional heading synchronization and editable styled lists. Table style application is a draft until outer Apply. Per-document typing rules prevent table/main-editor interference. 250 Rust tests and 240 Flutter tests pass, with 2 optional AI-provider skips; three UI-produced DOCX files pass independent style/override/numbering/recovery checks. Isolated build 2 natively verifies creation, New/Modify cancellation, targeted application, unused-style deletion, formatting/history, DOCX export and byte-identical restart. First-style creation retains native page geometry. Build 3 maps selections through committed refreshes. Build 4 natively verifies whole-line selection and targeted replacement. Build 5 passes native paste continuation, keyboard history, export and restart. Build 6 adds ordered asynchronous history/input, verified by delayed-engine tests and native rapid keyboard/post-save/history/export/restart checks. Build 7 retains exact repeated-text and emoji history positions, with native typing, post-export history and restart verified. Build 8 adds in-use style replacement across body/table text, related declarations and preserved Word XML, with Cancel, keyboard/post-save history, DOCX export and restart verified natively. Build 9 creates missing Heading 1–9 definitions only when applied, supports all nine editor levels, keeps rapid next-style typing ordered and protects built-in origins from deletion. Native body/table Apply, Cancel, keyboard/post-save history, DOCX export and byte-identical restart pass. See docs/NAMED_STYLES.md, docs/STYLE_REPLACEMENT.md, docs/BUILTIN_STYLES.md and VALIDATION.md. Build 10 adds direct outline levels 1–9, explicit Body text and Use style with native Cancel, Apply, history, independent DOCX and restart evidence. See docs/OUTLINE_LEVELS.md. Remaining native selection/input-method qualification, comprehensive localized/legacy style identities, full cascades/themes/templates/organizer and Microsoft Word comparison remain incomplete. |
| W03 | Lists, multilevel numbering, restart/continuation and outline | 0.14 implements real DOCX numbering for native lists, nested-level retention, imported common instance/level/start overrides and style inheritance, decimal/letter/Roman/bullet labels and continuation across prose. Numbering controls restart, continue and set start/style as one Undo transaction; Enter/multiline insertion retains the instance. Engine, codec, desktop widget and independent-reader checks cover the implemented paths. Packaged list controls, Enter, Undo, restart persistence and sandboxed DOCX export/replacement passed. Full list-definition editing, style-linked abstract definitions, picture/other numbering systems, exact indents/tabs/fonts/suffix layout, numbered tables, all multi-paragraph/nested-list cases and differential Word qualification remain. |
| W04 | Find/replace, navigation, Go To and bookmarks | Literal search/replace and heading navigation qualified for the 0.5 workflows. Version 0.46 adds Bookmarks/Add/Move/Delete, name/location sorting, hidden names, range/point Go To, top-level table-cell navigation, exact ordinary typing/correction/search/selection-proposal edits, history/recovery and body DOCX markers/internal link preservation. Version 0.47 captures actual rich/internal/HTML paste and compound operations, composes cell edits into one table Apply, flushes final typing, and rejects drafts after document replacement. Mapping, core, workspace and independent package checks pass; see `docs/BOOKMARKS.md` and `VALIDATION.md` for native evidence. Other stories, advanced/column/object bookmarks, structural table tracking, draft-local Undo, cut/move/cross-document bookmark copying, nested destinations, wildcards, formatting search, special codes, selection-only scope and full Go To remain. |
| W05 | Page geometry, sections, columns, margins, line/page breaking | 0.63 adds section-aware Page setup for paper size/orientation, margins/gutter, header/footer distances and section start, selected/all-section scope, recorded formatting review and source preservation. The continuous editor renders Section 1 geometry at physical width with horizontal scrolling. 326 Rust and 270 Flutter tests pass (two optional AI checks skipped); 15 DOCX artifacts pass independent XML/Open XML SDK checks. Native Cancel/Apply, edit-mode keyboard history, Enter, recording review, save and byte-identical restart pass in an isolated preview. 0.64 fixes and natively verifies post-Save review Undo, native Edit history and readable section comparisons; native filename keyboard routing and input-timing edges remain open. 0.65 adds explicit insertion/removal with four start types, rich text and linked header retention, following-section formatting, annotations, exact history and recovery. Recorded section-break edits, genuine pagination, per-section on-screen layout, columns and differential Word comparison remain incomplete. See docs/PAGE_SETUP.md and docs/SECTION_BREAKS.md. |
| W06 | Headers/footers, page numbers, watermarks and backgrounds | 0.66 adds section/type-aware rich header/footer drafts, Link to Previous, different first/odd/even settings, Apply/Cancel, history, source-preserving export and recovery. Focused engine/UI checks cover inheritance, copied bookmark and field references, numbered content, protection and stale requests. Native linked/unlinked edits, first/even content, draft and post-Save history, separate saves and byte-identical restart pass in an isolated preview. 0.67 adds inline PAGE/NUMPAGES/SECTIONPAGES/SECTION authoring, supported simple/complex field preservation and section format/restart/chapter controls. Native field insertion, section settings, Cancel, post-Save keyboard history and byte-identical restart pass; eight engine/editor/native Office artifacts pass independent content and schema checks. Cached values remain distinct from calculated page values. Recording header edits, paginated rendering/calculation, advanced field editing, watermarks/backgrounds and direct Word comparison remain incomplete. See docs/HEADERS_FOOTERS.md and docs/PAGE_NUMBERS.md. |
| W07 | Tables: insertion, resizing, merging/splitting, styles, sorting, nested tables and text conversion | 0.15 adds rich multi-paragraph cell editing, logical horizontal/vertical merges, span-aware rendering with source column proportions, nested tables, row/column insertion/deletion and merge/split commands. Draft Apply is one workspace revision/Undo; Cancel leaves the document unchanged. DOCX source table/row/cell properties and nested paragraph identities survive edits; legacy saved tables hydrate missing identities when geometry matches. Engine, native-bridge widget and independent-reader checks pass. Packaged checks passed rich typing, row insertion, selection/merge, one-step Apply/Undo/Redo, split/Cancel, column insertion/deletion through merges, separate Undo, external DOCX export and restart persistence. Status word count includes table text. 0.16 adds numeric/equal column sizing, automatic/minimum/exact row heights, direct fills/vertical alignment and table alignment, with fixed-width horizontal scrolling and DOCX projection/patching. Format/Undo/round-trip and rendering checks pass; packaged exact-height/width/fill/alignment Apply/Undo/Redo, native DOCX export and restart passed on the built-in display. Equal-distribution and automatic/minimum-height native checks remain. 0.17 adds selected text-to-table conversion with separator/column/width preview and whole-table-to-text conversion, retaining rich runs, nested/opaque content and one-step Undo. Merged covered positions become empty fields; paragraph joins follow the first paragraph format. Native keyboard selection, typed preview settings, both conversions, separate toolbar Undo/Redo, external DOCX exports and byte-identical revision-7 restart passed in 0.17.0. 0.17.1 corrects the post-conversion caret; regression checks cover both directions, and native typing retained the new table at revision 6. Independent readers verify the exported content and formatting. 0.18 adds contiguous selected-row conversion, keeping table fragments above/below, expanding vertical-merge boundaries with a preview explanation, and applying current table drafts plus conversion in one Undo step. Model, real-engine widget and independent DOCX-reader checks cover Apply/Cancel, source changes, export/reopen/reimport and formatting/merge preservation. Packaged native selected-row conversion and differential Word behavior remain unqualified; disjoint rows and partial-cell conversion are unfinished. Full inline authoring, table style/design editing, drag sizing, AutoFit, arbitrary split counts, sorting, page layout/repeating headers, omitted-grid/legacy/revision structures, exhaustive accessibility/performance and differential Word qualification remain. |
| W08 | Images, wrapping, cropping, drawings, shapes, text boxes, SmartArt and charts | Some imported data retained; native object layout/editing missing. |
| W09 | Equations, embedded/linked objects, media and content controls | Preserved or opaque where possible; full authoring/rendering missing. |
| W10 | Fields, cross-references, captions, TOC, index and table of authorities | Missing. |
| W11 | Footnotes/endnotes, citations, bibliography and reference manager | Missing editor behavior. |
| W12 | Spelling, grammar, thesaurus, translation and multilingual proofing | Local English Harper checks, personal dictionary, reviewed AI grammar edits. Other languages, thesaurus, translation and accuracy qualification incomplete. |
| W13 | Comments, threaded review, track changes, compare and merge | 0.57 adds body-story anchored comment threads, explicit attribution, plain-text editing, replies, resolution, navigation, filters, Undo/recovery and DOCX comment/reply metadata. Engine and real-bridge widget checks plus independent Microsoft Open XML schema validation cover supported cases. Native/Word desktop comparison, margin balloons, inline markers, direct table-cell creation, rich comment authoring, other story types, advanced cloud/thread variants, Track Changes, Compare and Merge remain incomplete. See docs/COMMENTS.md. |
| W14 | Mail merge, labels, envelopes and data sources | Missing. |
| W15 | Forms, restricted editing, document protection, macros and automation | Missing. |
| W16 | DOCX/DOCM/DOTX/DOTM/DOC/RTF/ODT/HTML/text format fidelity | Basic DOCX package projection; other formats and exhaustive OOXML fidelity missing. |
| W17 | Read mode, focus/outline/draft/web views, zoom and accessibility checking | Single authoring view; heading navigation does not establish full outline-view parity. |

## Excel requirements

| ID | Requirement | Current evidence and outstanding work |
| --- | --- | --- |
| X01 | Cell/range selection, multi-area selection, editing, clipboard, fill/series and autofill | Rectangular selection/basic TSV/editing. 0.22 adds directional Fill Down/Right/Up/Left, Ctrl+D/Ctrl+R and macOS ⌘D/⌘R, adjacent-source single-row/column behavior, relative/mixed/absolute A1 formula copying, blank clearing, cell-format/rich-source copying, inherited source row/column styles, shared-formula materialization, cache refresh, one-action Undo and save/reopen/reimport. Pending input commits separately and failed drafts prevent filling. Engine, real-Rust widget and independent XLSX-reader checks cover these paths, including prefixed/row-scoped XML and invalid/protected/merged/array cases. Packaged interaction and differential Excel checks remain pending. 0.31 adds multiline inline/formula input, Option/Control-Option-Return on Mac and Alt-Enter elsewhere, an expandable formula bar, Enter/Shift-Enter and Tab/Shift-Tab commit navigation, Escape cancellation and save-failure draft protection. Widget/native-bridge and independent XLSX-reader checks cover imported lines, selection replacement, Unicode, field paste/history, composition guards, formula calculation and exact Undo/Redo/reopen/reimport. Packaged 0.31 checks passed native import, Escape cancellation, both Mac line-break shortcuts, Unicode paste/rendering, Enter navigation, expansion without committing, separate text/formula/sizing keyboard history, native Excel exports and byte-identical restart. Native IME, failure injection, remaining shortcuts, accessibility and direct Excel comparison remain unqualified. Multi-area, drag fill handles, double-click extension, series/custom lists, Flash Fill, across-sheet fill, notes/hyperlinks/validation/conditional-rule copying, filtered/visible-only semantics, horizontal structured-reference fill, array/spill/merge/protection behaviors, 0.33 adds quoted multiline/tab/quote grid clipboard, immutable source snapshots, full XF dependency/rich-string copying, relative/mixed/absolute formula translation, cross-sheet destinations, exact-multiple range tiling, values/formulas/formats modes, skip blanks, transpose, atomic row reflow and Undo, and an opaque native clipboard token. Engine, real-bridge widget and independent XLSX-reader checks cover the implemented paths. Packaged 0.33 checks passed copy/paste, reference adjustment, multiline/rich cell formats, values-only/formatting-only menus, transpose, row fitting, post-Save Undo/Redo, and byte-identical restart/export. Native blank skipping, additional combinations, accessibility and direct Excel qualification remain. Cut/move, HTML/RTF and external-app format interchange, complete Paste Special arithmetic/link/width/validation/comment choices, clipboard persistence, cross-workbook theme/reference conversion, full cell-length limits and direct Excel comparison remain unfinished. |
| X02 | Find/replace, Go To, special selection and navigation | Literal search, row/column order, sheet/workbook scope, formula/text vs displayed values implemented and tested in 0.5; native mouse replacement/recalculation/Undo passed. Wildcards, comments/notes/format search and Go To Special remain. |
| X03 | Insert/delete/move/resize/hide rows, columns and sheets; reference transformations | 0.6 adds row/column insertion/deletion and sheet rename/reorder/deletion, with atomic Undo, formula/style/link transformations and tested OOXML names, charts, comments and ranges. 0.11 adds manual row/column visibility, exact heights/widths, heading-boundary dragging, hidden-column keyboard navigation, sparse scrolling and dimension import/export/transformation. Packaged mouse checks cover numeric and dragged sizing, hide/adjacent-unhide, Undo/Redo, restart persistence and matching assistant selection; subsequent isolated 0.11.1 checks passed repeated navigation, Shift/Command shortcuts, caret editing, commit and cancel. Formal keyboard/assistive-technology coverage remains incomplete. 0.29 adds explicit row AutoFit with shared text measurement, hidden/filter preservation, one-step Undo and saved/reopened XLSX evidence. 0.30 adds explicit column AutoFit, font-dependent width conversion, manual character-width entry, localized Normal-font projection and one-step saved/reopened sizing. Packaged 0.30 checks passed native import, both AutoFit menu commands, hidden/offscreen content handling, keyboard Undo/Redo (including after native Save), boundary fitting after drag, byte-identical restart/export and native XLSX reimport. An independent reader verifies content, formats, caches and dimensions. Native context-menu/multi-selection boundaries, cancellation, accessibility and direct Excel comparison remain pending. 0.32 adds persistent automatic/manual row-height modes, post-edit row fitting across formula dependents and offscreen content, shared-renderer measurement, and an atomic staged edit/layout commit. Engine tests, real-bridge app checks and independent XLSX readers cover manual/hidden/merged preservation, shrinking, width/font changes, one-step history, failure/cancellation/stale guards, queued saves, sparse last-row sizing, source-mode hydration and save/reopen/reimport. Packaged 0.32 checks passed multiline growth/shrink, fixed-height preservation, row-context AutoFit, column dragging with atomic history, cross-sheet formula refitting, post-Save Undo/Redo, byte-identical restart/export and XLSX reimport followed by automatic shrink. Independent retained-artifact validation passes. Broader native combinations, accessibility/performance and direct Excel comparison remain pending. Move rows/columns and automatic numeric column growth, default/inherited dimension choices, outline controls, sheet duplication, pivot/external reference structures and remaining table options, all drawing modes and differential Excel qualification remain. |
| X04 | Cell/row/column formatting, styles/themes, merges, alignment, borders and number formats | 0.24 adds a mixed-selection Font dialog with installed-family search, half-point size controls, RGB/automatic color, bold/italic/strike, five underline choices, normal/super/subscript, preview, Cancel/Apply/Reset and grid shortcuts. Actual cell fonts/effects render with cached text layout and inherited blank-cell defaults. Rust guards invalid/stale requests; pending input commits separately, one Undo reverses font changes. XLSX property edits retain other XF/font metadata and update rich runs without changing other users of a shared string. Native bridge, drawing/pixel/semantics/cache checks and independent file readers are recorded in VALIDATION.md. Packaged interaction and differential Excel qualification remain pending. 0.25 projects and renders rich/shared-string runs, summarizes mixed properties within one cell, previews run formatting and retains per-property whole-cell overrides. Source phonetics remain in XML but do not enter displayed or calculated cell text; legacy upgrades preserve actual edits. Run metadata follows fill and structure edits. 0.26 adds selected-character formatting in the Font dialog with persistent selection highlighting, per-span previews, stale/Unicode validation, compact property maps and preserved source run metadata. Keyboard-selection Apply/Undo and independent XLSX checks pass. 0.27 decodes escaped cell strings once, protects literal escape tokens on export, keeps cached text types and uses decoded UTF-16 offsets for character formatting. Independent fixture/reader and history/migration tests cover these paths; ambiguous legacy selections inside escape tokens remain unsupported. 0.28 adds an Alignment dialog and ribbon commands, general value-type alignment, top/middle/bottom, left/center/right/wrapped justification, wrap/manual breaks, single-line shrink, indent, reading order, signed rotation and grapheme stacking. Engine, rendering/pixel and real-Rust dialog tests plus independent XLSX readers cover the implemented paths. 0.29 adds explicit AutoFit Row Height through the ribbon, row context menu and row-boundary double-click, with shared-renderer measurements across offscreen cells, point-height persistence, one-step Undo, stale/invalid guards, hidden/filter preservation and merged-row rejection. Engine, native-bridge UI, layout and independent XLSX-reader tests cover the implemented paths. 0.30 adds column AutoFit across offscreen rows, shared Normal-font width conversion, rich/numeric measurement, hidden/filter preservation, exact size recovery and selection-fill contrast. Packaged AutoFit menu/boundary, keyboard history, native Save, restart and XLSX reimport checks passed with independent-reader evidence. 0.32 automatic row sizing covers committed content, formatting, width and cross-sheet formula-result changes while preserving fixed sizes. Advanced alignment modes and automatic numeric column expansion, neighboring-cell overflow, remaining native formatting/accessibility workflows and direct Excel comparison remain incomplete. Rich inline editing and in-cell/formula-bar character-format shortcuts, exact font metrics/script positioning/accounting-line placement, font substitution, theme/indexed colors, named/default style authoring, complete inherited defaults and automatic sizing, locale/date systems and full alignment/border/merge editing remain unfinished. 0.34 adds shared Rust number-format display, custom/preset preview and Apply, conditional/text/color sections, fractions/scientific/date/duration formatting, glyph-sized accounting skip/fill, inherited formats, formatted clipboard/search/linked document output and atomic row/history integration. Engine and real-bridge widget checks pass, with an independent SSF reference comparison and XLSX-reader checks; 124 Rust/130 Flutter tests and 14 independent readers passed; package/native qualification is tracked in VALIDATION. Width-dependent display, locale/calendar/built-in coverage, complete date/TEXT coercion and array semantics, early-1900 edge cases and formatted-value filter grouping, advanced accounting layout and direct Excel comparison remain. 0.35 shares numeric TEXT rendering with cell display and formatted filter/sort/chart labels; full-app and independent-reader evidence is in VALIDATION. |
| X05 | Worksheet views, frozen/split panes, zoom and page layout | 0.12 adds selection-based freeze, top-row/first-column commands, unfreeze, pinned scrolling with hidden/sized rows and columns, separate imported scroll origins, selection reveal, workspace persistence, Undo and XLSX import/export. Engine, native-bridge widget and independent-reader checks cover the implemented paths, including structural boundary changes and collapsed-pane selection metadata across saved views. Packaged checks passed imported boundaries, selection-based freeze, vertical scrolling, horizontal address navigation, pinned-cell selection, unfreeze, top-row/first-column commands, separate Undo and restart persistence. Native horizontal-wheel input, additional structural/view cases and differential Excel checks remain pending. Oversized frozen areas temporarily scroll with a visible notice; stored boundaries remain unchanged. Interactive split panes, custom upper-pane scroll origins, split restoration on unfreeze, multiple-view selection, zoom and page layout remain. |
| X06 | Sort, filter, advanced filter, tables, structured references, totals and slicers | 0.7 implements named tables, structured formulas/calculated columns, stable multi-key table sorting, value/custom filters, visible-row navigation, totals and basic styles. Engine/native widget checks, packaged mouse interaction and independent XLSX readers cover record metadata, source names, structural edits, persistence and Undo. 0.8 adds table boundary resizing, formula preservation for excluded cells, filter/sort remapping and totals relocation with automated/native widget and independent-reader evidence. Follow-up packaged checks in 0.10 passed table expansion with totals relocation, narrowing with excluded cells retained, separate Undo steps, two-key ordinary range sorting and custom totals hide/show. 0.9 adds ordinary range sorting with optional headers, hidden-row exclusion, persisted settings and worksheet sort metadata, plus custom totals editing/metadata/hide-show. Engine, native-widget and independent-reader checks cover those paths. 0.10 adds ordinary worksheet AutoFilter controls, cross-column filter choices, explicit filtered/manual row semantics for SUBTOTAL, reapply/clear/remove, structural metadata changes and saved/reopened exports, with engine, desktop-widget and independent-reader checks. Packaged mouse checks passed for value selection/apply, subtotal recalculation, clear preserving manually hidden rows, removal, separate Undo steps and persistence after restart. Follow-up packaged checks also passed initial enabling with absolute addresses, explicit reapply after cell edits, separate Undo, cross-column choices and two numeric conditions. Structural filter workflows still need packaged interaction checks. Occupied totals destinations, all resize style/object cases, conversion, left-to-right/outline/color sorting, advanced filters, slicers, locale/custom lists, complete design options and direct Excel qualification remain. |
| X07 | Data validation, input messages, error rules and conditional formatting | 0.39 implements standard validation types/operators, relative/custom/range formulas, input hints, blocking/override alerts, typed cell/formula entry, searchable list choices, rule clearing, Undo/recovery and OOXML retention. Rules follow supported row/column/sheet transformations and copy with internal Paste All/fill; other paste modes retain destination validation. Engine and real-bridge UI coverage is recorded in VALIDATION. Named-range evaluation, x14 validation editing/evaluation, circle-invalid/find-validation commands, apply-to-same-settings, full input-message placement, IME/locale/coercion/error semantics, differential Excel and conditional formatting remain incomplete. |
| X08 | Full function catalog, errors, coercion, precision, locale and date-system semantics | IronCalc subset and cross-sheet calculations; 0.10 distinguishes manual and filter-hidden rows for SUBTOTAL and refreshes changed-workbook formula caches on export. 0.35 shares numeric TEXT rendering with the cell formatter, supports format-code array broadcasting in the engine, and tests output types, errors, reference edits and save/reopen caches. 0.36 carries the 1900/1904 calendar through supported date/input/criteria/financial paths, cached-model invalidation, structural calculations and persistence, with paired fixtures, widget and independent-reader evidence. 0.38 adds exact early-date components, serial-zero/fictitious-leap-day construction, DATE year normalization, weekday/workday serial stepping and distinct EDATE/EOMONTH month-end clamps, with 774 modeled expectations, independent file readers and native entry/history/export/restart checks recorded in VALIDATION.md. Complete early-date financial/week-number/DATEDIF semantics, coercion/locale/timezone behavior and visible dynamic-spill integration remain incomplete. Differential Excel formula corpus missing. |
| X09 | Shared/array/dynamic formulas, spill, LET/LAMBDA, names and external references | 0.6 expands shared formula followers on import and materializes groups when their sheet is edited. Array/dynamic formulas, LET/LAMBDA, names and external-reference calculation coverage remain partial; package name updates do not establish named-formula evaluation parity. |
| X10 | Formula auditing, dependency tracing, evaluate formula and watch window | Partial: Formula inspector follows direct parsed references across worksheets with sparse ranges, paging and navigation history; unresolved formulas are explicit. 0.79 adds persisted Watch Window values/formulas, cross-sheet navigation, resizing, history, structure and XLSX/recovery. Automated bridge and file checks pass. Native 0.79 verifies watch editing/history, live values, inspection/Back, dependents, exact-range navigation, export/reimport and restart. Native arrows, complete dynamic/named/external reference tracing, protected formulas, Evaluate Formula, full Watch Window behavior and Excel comparison remain open. |
| X11 | Charts, chart formatting/types, sparklines and drawing objects | Simple local chart preview, with formatted category/value labels since 0.35; Office chart editing/rendering missing. |
| X12 | Pivot tables/charts, calculated fields/items, grouping and data models | Missing. |
| X13 | Power Query, data import/connectors, refresh, relationships and Power Pivot/DAX | Missing. |
| X14 | What-if analysis, scenarios, data tables, Goal Seek, Solver and Analysis ToolPak | Partial: 0.80 adds single-input Goal Seek with cross-sheet targets, incremental isolated numerical trials, Stop/Cancel, reviewed Apply/best-value acceptance, validation, measured cell history and XLSX/recovery. Automated application/engine, native cross-sheet preview/Cancel/Apply/history/export/restart/reimport and independent file checks pass. 0.81 adds editable named scenarios, Show/history, same-workbook Merge, static Summary and scenario OOXML/recovery/structure support; native controls, history, file and recovery checks pass. 0.82 adds cross-workbook Merge with a frozen source and custom number-format transfer; automated application/engine, native copy/application/history/custom-format/export/restart/reimport and independent file checks pass. PivotTable reports/full protection, direct Excel comparison, broader source and solver accuracy/volatile/table cases, data tables, multivariable Solver and Analysis ToolPak remain open. |
| X15 | Protection, sharing/coauthoring, comments/notes and change history | 0.58 development adds distinct cell notes/threaded comments, author attribution, review/filter/navigation, markers/hover, annotation-only paste, structural mapping and XLSX/recovery support. Rich source preservation is limited; persistent panes, note geometry/display/printing, mentions, identity, coauthoring, protection and tracked history remain incomplete. Native and Excel-desktop qualification are separate; see docs/SHEET_REVIEWS.md. |
| X16 | VBA/macros, add-ins, Office Scripts and automation compatibility | Missing. |
| X17 | XLSX/XLSM/XLTX/XLTM/XLS/XLSB/CSV/ODS fidelity | Basic XLSX projection and source part preservation, core CSV export; 0.36 exports parsed date/currency/percent inputs as numbers, booleans as booleans and preserves explicitly typed formatted-number strings. Broad format fidelity, inferred input-format persistence remain incomplete. The 0.37 typed-error cache path is implemented and independently verified; broader error/coercion semantics remain unfinished. |
| X18 | Print areas/titles, page breaks, scaling, headers/footers and PDF | Missing. |
| X19 | Excel limits, sparse/dense workbooks, calculation scheduling and resource handling | Full address grid, bounded populated model. Current 30-sheet/100k-populated-cell bounds and engine coverage fall short of parity. |

Column progress in 0.69 contributes to W05/D05: section-scoped equal/custom
widths, spacing, separators and reading order now round-trip through Word
files and feed physical page flow. The renderer reflows rich paragraphs and
table row groups into unequal columns, distinguishes column/page breaks and
keeps page fields tied to physical pages. Recorded formatting, Apply/Cancel,
Undo and recovery have automated coverage. All 355 Rust and 303 Flutter tests
pass (two optional AI-provider skips); six DOCX artifacts pass Microsoft365
schema validation. Native 0.70 qualification subsequently passes Cancel,
custom-left/separator/RTL Apply, ordinary and post-Save keyboard Undo/Redo,
all seven fixture pages, independently validated DOCX export and exact restart
recovery. See docs/COLUMNS.md, release/validation/columns-development and
release/validation/columns-native. Automatic section
creation around a selection, direct column-break editing and Word layout
comparison remain open; no whole family is accepted.

Version 0.70 adds final-page balancing before compatible continuous section
boundaries and reads Word's noColumnBalance compatibility setting. Earlier
pages and source data stay unchanged. All 356 Rust and 316 Flutter tests pass
(two optional AI-provider skips), including read-only preview/export/recovery
for enabled and disabled fixtures. Four DOCX artifacts pass independent content
and Microsoft365 schema checks. Native 0.70 previewing of both modes, two
byte-identical DOCX exports and unchanged restart recovery pass; both native
exports also pass schema checks. The Columns dialog's native workflow also
passes in this same 0.70 build; direct Word comparison remains pending. See
docs/COLUMN_BALANCING.md. These checks do not accept an entire parity family.

Version 0.71 development adds repeated table-header controls, direct DOCX flags,
and derived heading rows in Page preview. Consecutive prefixes, merged headings,
per-page fields, body-row preservation and column balancing have automated
coverage. Native Cancel/Apply, mixed/multi-row selection, ordinary/post-Save
keyboard history, enabled/disabled pagination, DOCX export and exact restart
recovery pass. Direct Word comparison remains pending; manual breaks inside
table cells and tracked table-format edits remain open.
See docs/TABLE_HEADERS.md. This does not accept a whole parity family.

Version 0.72 shares table row sizing between the editor and Page
preview: exact rows stay fixed, minimum/automatic rows follow their rules, and
vertically merged overflow grows only flexible rows. Nested text, fills and
borders are clipped to their containing cells. Nine added layout/workspace
tests cover pagination, repeated headings, Apply/Cancel, history, export and
recovery; the full Flutter suite passes 337 tests with two optional provider
skips. Source and two exports pass independent XML and Microsoft365 schema
checks. The signed 0.72 preview passes native automatic/exact/minimum display,
Cancel/Apply, ordinary/post-Save As keyboard history, working Save, export and
byte-identical restart across 17 checkpoints. Four native files pass schema
validation. Native nested/merged/overflow-alignment cases and Word comparison
remain pending; row splitting, editable line-break paragraphs in table cells,
and keep-together semantics remain open. See docs/TABLE_ROW_HEIGHTS.md.

Source development after 0.72 advances row pagination with a mixed-state
allow-break checkbox, direct/inherited DOCX settings, independent cell text
cursors, nested ordinary-table continuation, page fields, repeating outer
headings and replay through unequal columns/continuous-section balancing.
Engine and real-bridge application checks cover Cancel/Apply/history/recovery.
The signed 0.73 preview now passes 19 native checkpoints for an ordinary two-cell row: Cancel/Apply, both settings, two-page flow, ordinary/post-Save keyboard history, exports and exact restart recovery. Five native DOCX files pass schema validation; one preview remains running. Native complex-table cases remain separate. Exact/merged row continuation, split-cell
vertical alignment, nested headings, full paragraph/style rules and Word
comparison remain open. See docs/TABLE_ROW_SPLITTING.md. No parity family or
novel AI feature is accepted by this source change.

## AI-human interface and the additional 20-feature requirement

Existing assistant integration remains part of the baseline product: supported Codex App Server sign-in, selected passage context, attached workbook ranges, source-linked blocks, native A2UI forms, exact proposed edits, explicit Apply and one-action Undo. These are implemented in part and tested in disposable workspaces; they do not automatically count as unavailable in Word/Excel.

Phase ordering from the goal is preserved: complete and verify parity first, then implement at least 20 additional distinct AI features. During parity work, maintain candidate research without counting it as delivery. Ordinary rewriting, summarization, grammar review, formula creation, worksheet edits, data analysis and charts cannot be counted merely because they use AI; Microsoft already provides substantial Copilot functionality.

Each of AI01–AI20 needs its own: (1) distinct user problem and workflow; (2) dated Word/Excel/Copilot comparison with actual product tests where available; (3) an explanation of what those products cannot do; (4) implementation and reachable native UI; (5) successful real-model execution, persistence/recovery and Apply/Undo tests; (6) usability evidence. Similar prompts for the same mechanism count as one feature. If novelty cannot be verified, replace that candidate. No slot is accepted merely from an absence in a web search.

| Slots | Design | Novelty verified | Implemented and verified | Accepted count |
| --- | --- | --- | --- | --- |
| AI01–AI20 | Pending after baseline parity; ideas may be researched earlier | No | No | 0/20 |

## 0.49 date-rule and calculation progress

X07 gains the ten date-occurring rule periods, authored/imported predicates and
priority/history/file recovery. X08 gains one local calculation clock across
TODAY/NOW and date predicates; X19 gains explicit F9 and local day/offset refresh.
Recalculation measures changed automatic rows without adding an Undo entry.
Engine, desktop-worker and independent-file evidence is in VALIDATION.md.
These do not accept any full parity family; calculation modes, full volatile
function scheduling, Excel differential tests and the remaining ledger gaps
still need work. Novel AI remains 0/20.

## 0.50 conditional-rule copy progress

X07 and clipboard/fill workflows gain conditional-rule copying in Paste All,
Paste Formats, explicit merge-paste and directional Fill. Source snapshots,
relative/mixed formulas, destination subtraction, priority, aggregate populations
and extension identifiers are covered by engine and independent-file checks.
All eighteen currently modeled rule types participate. Native clipboard/input,
differential Excel rule consolidation, Cut/move, Format Painter and remaining
paste/format/validation requirements are still incomplete. This accepts no full
parity family, and additional novel AI remains 0/20.

## 0.51 Paste Special property progress

Five added modes cover values/formulas with number formats, validation-only,
column-width-only and cells with source widths. Effective number formats copy
without replacing destination XF components. Validation reanchoring uses the
correct destination sheet catalog, and missing explicit source sheets reject
the paste. Widths retain source snapshots, stored or measured defaults, target
visibility/outline metadata and measured-row transaction history. Metadata-only
pastes can cross merge records without editing them.
Engine, application and independent-reader evidence is in `VALIDATION.md`;
`docs/PASTE_PROPERTIES.md` records limits. Native and direct Excel comparisons,
arithmetic/link/comments/border/theme paste modes and the rest of X01 remain
incomplete. No entire family is accepted; novel AI remains **0/20**.

## Next engineering work

0. Extend lifecycle qualification to physical sleep/wake, repeated locking, display disconnection, Mission Control and native minimize/restore. The reproduced active-window activation/display-transfer failure is fixed in 0.16.1; do not force rendering for an inactive background app to satisfy an automation check.
1. Finish 0.17.1 native post-Apply Undo/Redo, Save return and restart checks after the Mac is unlocked; then qualify the 0.18 selected-row conversion and 0.17.3 focused-field/formula-history and immediate Undo/Redo paths natively, including typing immediately into a newly opened dialog and IME composition commit/cancel. Expand keyboard/IME coverage. Complete remaining table resize cases, disjoint-row/partial-cell conversion, tab-stop settings/ruler/leader/alignment and all rendering surfaces, advanced worksheet AutoFilter cases and remaining range-sort modes; extend structural operations to pivots, external links, row/column move, AutoFit and sheet duplication. Qualify all drawing anchoring modes and formula/reference behavior against Excel. The implemented transformer uses IronCalc syntax trees plus interval deletion semantics; the upstream displacement helper invalidates deleted range endpoints instead of shrinking ranges.
2. Complete advanced filters, validation, conditional formatting and named styles.
3. Establish document pagination/sections and printing; expand the OOXML model before replacing more opaque content.
4. Extend search to the remaining Word/Excel options and stories, improve large-document responsiveness, and qualify native keyboard behavior.
5. Work through every family above, expanding the requirements inventory and differential Microsoft Office corpus. No small release marks the overall goal complete.

## Text and table conversion baseline

Microsoft's [text/table conversion instructions](https://support.microsoft.com/en-us/word/convert-text-to-a-table-or-a-table-to-text), [Rows.ConvertToText API](https://learn.microsoft.com/en-us/office/vba/api/word.rows.converttotext) and [Table.ConvertToText API](https://learn.microsoft.com/en-us/office/vba/api/word.table.converttotext) were checked for separator, row, width and nested-table behavior. Implemented behavior is described in README and scoped evidence in VALIDATION. These sources do not establish merged-cell, rich paragraph, partial-row, AutoFit or nested-table equivalence; native Microsoft Word comparison remains required.

## 0.37 error-value compatibility progress

Classic Excel error values and error-shaped text now remain distinct through explicit OOXML string import, calculation, custom text display, native clipboard snapshots, Paste Values and XLSX export. Cached formula errors use `t="e"`; formula text uses `t="str"`, and formula caches follow result-type changes even when the visible string is identical. Cell metadata and unchanged source bytes are preserved. Exact legacy source string projections are repaired while retaining actual user edits. Engine-only formula diagnostics have no fabricated Excel cached value. Automated coverage includes seven classic errors, ISERROR/IFERROR/ERROR.TYPE, clipboard/history, structural row edits, fill, save/reopen and independent file-reader checks. Packaged 0.37 native #N/A versus text entry/display, Paste Values, keyboard history, native export and byte-identical quit/restart/export passed in an isolated container. The user app is upgraded and its original workspace unchanged. The broader seven-error corpus and remaining qualification limits are recorded in VALIDATION.md. This is not acceptance of X03 or full error/coercion/dynamic-array compatibility. Early-1900 date semantics and the previous calendar/formatting native workflows remain outstanding. Novel AI remains 0/20.


## 0.38 qualification

Early-date fixes target X08 and date rendering within X04; they do not accept either family. The independent corpus is modeled from calendar rules and supporting references, not captured Excel outputs. 137 core Rust and 134 Flutter tests, 2,226 vendor unit tests plus 23 documentation tests, 19 independent readers and the isolated native editor/history/export/restart checks pass. The main 0.38 app is running with the user workspace unchanged. Remaining early-date financial/week-number/coercion cases, the prior formatting/calendar native workflows and direct Excel comparison are outstanding. Full parity remains incomplete and novel AI remains 0/20.


## 0.39 qualification

Data validation advances X07 without accepting the family. Standard rules,
copy/fill, input hints, dialogs, typed entry and Office persistence have engine,
widget, independent-reader and scoped native evidence in VALIDATION.md. The
main 0.39 app is running with the user's latest revision-26 workspace unchanged.
145 core Rust/135 Flutter tests, 2,228 vendor unit tests plus 23 documentation
tests, 20 file readers and the separate native-artifact reader pass. Named
ranges, extended validation, invalid-data auditing, error/coercion details,
conditional formatting and differential Microsoft Excel qualification remain.
Full Word/Excel parity and the subsequent 20 novel AI features remain incomplete.


## 0.40 conditional-formatting progress

X07 remains incomplete. Value/formula, text, blank/error, duplicate/unique,
rank/average and color-scale rules now have a derived display projection,
worksheet rule manager, priority/property composition and Stop if true. Rule
edits participate in atomic history and automatic row layout. Full-column
blank predicates evaluate the visible cells without expanding the worksheet;
aggregate rules use the full populated rule population. Standard OOXML rule
and differential-style records round-trip, and the existing structural
transformer now rebinds their metadata for new and imported workbooks.

Still required: data bars, icon sets, date-period authoring, theme/indexed and
border/advanced differential formats, pivot/table-specific rule semantics,
copy/fill/cut rule behavior, duplicate/blank/coercion/wildcard/locale parity,
full formula/name support, and direct Microsoft Excel comparison. Color
interpolation/threshold edge behavior also needs differential qualification.
Views exceeding two million cell/rule comparisons report a display limit.
No Word/Excel requirement family is accepted complete; novel AI remains 0/20.

The 0.40 final release passes 157 core tests, 137 Flutter tests with the release
engine (two optional provider skips), 21 independent file readers and a separate
native-workflow reader. Packaged value/rule changes, native Undo, last-row blank
highlighting, native export and byte-identical quit/reopen/export passed. The
user workspace remains unchanged. Seven-digit row-header wrapping observed in
the native last-row check remains a separate UI issue.


## 0.41 conditional-formatting progress

X07 now includes editable data bars, negative-value axes/colors, bar-only
view, standard and Office 2010 import/export, and supported structural range
movement. Native seven-digit row headers no longer wrap. Automated scope and
unqualified Excel geometry/threshold cases are recorded in VALIDATION.md;
the final package passed midpoint/visibility/history, last-row, native export
and byte-identical restart checks. The user app is upgraded with its workspace
unchanged. Icon sets and the other X07 gaps remain. No family is accepted complete and novel AI remains 0/20.

The 0.41 native walkthrough found a rapid first-entry loss (`60` followed
immediately by Enter saved `6`). F2 with an observed active editor succeeded.
The 0.42 keyboard handoff fix below addresses this sequence without accepting full
typing parity. The midpoint bar-scale discrepancy found in the same walkthrough was
corrected to use one absolute-value scale, with a regression test.

## 0.42 keyboard handoff progress

D03 now retains fast first-cell text and completion keys before the editor has
mounted, leaves the caret after newly typed text, guards stale focus callbacks,
and prevents repeated completion keys from reopening stale values during a save.
Nine new widget cases/variants and the expanded real-engine workflow pass within
the 148-test suite. The native package passes the previously failing `60`+Enter
sequence, formula recalculation, keyboard Undo/Redo, fast Tab/Shift-Tab entry,
Escape cancellation and byte-identical saved recovery across restart.

Next keyboard gates: continuous multi-cell entry while saves are pending,
pre-attachment editing shortcuts and local history, native composition/dead keys,
and international keyboards. These remain part of D03; the broader Word/Excel
families and the 20 additional novel AI features remain unaccepted.


## 0.43 continuous-entry progress

D03 now queues subsequent cell typing during asynchronous saves, preserves key
modifiers, advances against the accepted revision and gives each cell its own
Undo step. Failed or detached entry stays recoverable through explicit controls.
Flush rechecks late text, Command-Q survives editor replacement, and native quit
coalesces repeated requests while saving. The 162-test Flutter suite, 47 AppKit
keyboard checks, native lifecycle checks and final packaged entry/history/quit/
restart evidence are scoped in VALIDATION.md. The user app is upgraded to 0.43
with its saved workspace unchanged.

Next keyboard work includes clipboard and command shortcuts during queued entry,
pre-attachment local history, native IME/dead keys and international keyboards.
Waiting-key crash recovery and broader validation/close combinations remain.
Word/Excel families and the 20 additional novel AI features remain unaccepted.


## 0.44 text-overflow implementation

X04 now paints ordinary text into adjacent empty cells without creating values
or changing selection/semantics ownership. Left/right/center, numeric text versus
numbers, error types, wrap/shrink/rotation, offscreen owners and frozen boundaries
have rendering coverage. Imported merge ranges are projected as read-only
barriers, follow supported structural rebasing and recover in older workspaces.
Five new index/pixel cases, a full-workspace edit/history/export/reopen test and a
core merge-history/recovery test pass within the 169-core/168-Flutter suites.
The signed native package passes blank-cell selection, obstruction, Undo/Redo,
native XLSX export and exact restart. The main app is upgraded to 0.44 with the
user workspace unchanged. Evidence is scoped in VALIDATION.md; X04 is not
accepted complete.

Direct Excel comparison remains required for colliding labels (the current
renderer partitions their overlapping blank region), table text, rich/bidi
edge cases, fills/borders and exact glyph/gridline geometry. Table areas and
AutoFilter headers currently remain clipped. Merged-cell editing/rendering,
width-dependent numeric display and the other X04 gaps remain. Full Word/Excel
parity and novel AI 0/20 are unchanged.

## 0.45 width-dependent numeric display

X04 now distinguishes numeric width behavior using the engine's selected format
section. Ordinary General decimals reduce displayed precision or use scientific
notation; fixed numeric/date formats use hashes when too wide. Invalid numeric
dates fill the available width. Stored values, full accessibility text, formula
bar, copy and exports remain independent of this paint decision. AutoFit measures
the natural text and widening restores the original representation.

Four focused renderer/candidate tests and a real-engine full-workspace workflow
cover sizing, scaling, text/error types, accounting color/spacing, Shrink to Fit,
AutoFit, copy, history, exports and reopening. A core test covers section/value
classification, including Text format and invalid-date handling. The complete
170-core/173-Flutter suites pass. The signed native package passes width changes,
full formula-bar display, Undo/Redo, context-menu AutoFit, native export and exact
restart. The main app runs 0.45 with the user's saved workspace unchanged.
Qualification scope is recorded in VALIDATION.md.

Exact Excel rounding/notation thresholds, very narrow columns, selected-cell
border differences, international/locale formats, rotated and wrapped numeric
clipping, custom General formats with surrounding literals, merged-cell display
and automatic numeric width growth remain unqualified or incomplete. The current
adaptive-number path preserves the existing wrapped/rotated rendering. No parity
family is accepted complete and the additional novel AI count remains 0/20.

## 0.48 conditional-formatting icon sets

X07 now includes all 20 icon styles, custom category mixtures/No cell icon,
reverse order, icon-only display and four threshold types. Standard and Office
2010 rule records round-trip; custom category counts may differ from the unused
base style. Derived icon display respects priority and ordinary Stop if true,
recalculation, full populated ranges, Undo, saved recovery and supported
structural reference movement. The cell layout and AutoFit account for icons
without changing cell values. Automated and independent-reader evidence is in
VALIDATION.md and `docs/ICON_SETS.md`.

Native macOS interaction, direct Microsoft Excel visual/numerical comparison,
printing, icon-based sort/filter, copy/fill/cut behavior and the remaining X07
requirements are still open. No Word/Excel family is accepted complete. These
20 Excel icon styles are not novel AI features; the novel AI count remains 0/20.


## 0.54 working-file Save / Save As development

D02 gains active DOCX/XLSX and full-workspace save locations, Save As retargeting,
Copy/Export separation, local-only bookmark metadata, interrupted-association
recovery, source identity checks across Undo, and external-file fingerprints.
The core checks expected bytes before atomic replacement and reports the saved
revision/fingerprint. Full engine/editor and independent-reader evidence is in
VALIDATION.md. Native sandbox Save and post-restart access are still unqualified
because the Mac locked during preview startup. The normal reviewable release
remains 0.53.0 build 11. D02, Office compatibility, and the full goal remain
incomplete; the novel AI phase remains 0/20.


0.54 build 2 development adds content signatures for independent document/workbook
changes, linked-source updates, Undo back to saved content and stable file lineage
across package revisions. New/Open/Close decisions protect changed named files;
Cancel, cancelled Save As, failures and repeated native close requests are covered
by editor/method-channel/AppKit tests. Don’t Save leaves external files unchanged
and retains local recovery, as stated in the dialog. Unnamed-file first-save prompts,
conventional discard/revert and separate windows remain open, along with native
sandbox/restart qualification. This advances D02 but does not complete it. No new AI
feature or parity family is accepted (novel AI 0/20).


0.54 build 3 development adds independent first-save prompts for changed unnamed
Word/workbook content, persistence of their initial states, and focus restoration
after cancelled leave operations. Close with Don’t Save writes a recoverable
snapshot and a blank restart workspace instead of reopening discarded edits;
a queued autosave cannot overwrite that decision. Failure retains live content
and Undo. D02 still requires native qualification, recent files/recovery browsing,
templates/version history, file associations, multiple windows and provider
coordination. No full parity family is accepted; novel AI remains 0/20.


0.55 development adds File/sidebar Recent Files and Recovery, persistent pinned
locations, search/type filters, non-destructive list removal, moved-file/Locate
handling, and legacy recovery discovery. Copies now use unique OS-created
directories. Preview runs in an isolated core; Restore and ordinary Open from
the recovery directory produce a fresh unsaved workspace and cannot bind Save
to the snapshot. First Save chooses a whole-workspace file. Startup typing
retains the original unsaved baseline while local history loads. See
docs/RECENT_FILES.md for scope, evidence and remaining gates. D02 remains
incomplete; no parity family or novel AI feature is accepted (0/20).


0.56 development adds native/shared New from Template and Save as Template,
six authored starters, a searchable personal library, isolated content previews,
new-identity DOCX/XLSX creation, explicit DOTX/XLTX editing and save-format
preservation. Package main types are converted without replacing other parts;
macro-bearing or ambiguous templates are rejected. First-save Cancel and stale
source checks retain current work. 263 Rust and 257 Flutter tests pass, with two
optional provider skips; independent readers verify the generated template and
regular document/workbook artifacts. Native sandbox/restart, Office application
comparison, global/default/macro/legacy template behavior and remaining D02/W02
requirements are unfinished. See docs/TEMPLATES.md. No full parity family or
novel AI feature is accepted (0/20).


## 0.59 Track Changes review development

W13 gains imported revision review, insertion/deletion markup, author/type/story
filters, previous/next, individual and filtered bulk Accept/Reject, direct
formatting comparisons, exact source revision guards, nested operations and
one-step history/recovery. Ordinary pending edits and live workbook bindings
survive review. Relevant text, paragraph-break, property and simple-row cases
have focused automated/independent validation. See docs/TRACK_CHANGES.md.

Remaining authored revision types, moves and other revision types, inline page
navigation/balloons/display modes, Compare/Merge, Word desktop comparison and
all inherited W13 requirements remain unfinished. No family is accepted;
Word/Excel parity remains incomplete and novel AI remains 0/20.

Authoring development after 0.59 adds exact tracked text/format edits and
paragraph splits/joins through the native engine and normal Flutter edit queue.
Supported edits inside table cells preserve their container and unrelated Word
objects. Source-only formatting, explicit authorship, recording/formatting
settings, Undo/Redo, review, `.folio` recovery and DOCX revision/settings export
have focused checks. 0.60 adds desktop recording/author/formatting controls,
status, a keyboard toggle, explicit imported-file author setup and recoverable
unsaved draft errors. The isolated 0.60 build 3 native preview passes end-of-document
Enter and continued typing, author-filtered rejection and Undo, DOCX save/reimport,
explicit imported author setup, clean quit/restart and post-restart Save. Seven
revisions retain five Alex and two Morgan changes. Earlier native checks cover
replacement/formatting, recording/export shortcuts and rejected-draft recovery.
Advanced structures, annotations, live workbook
bindings, performance and the full W13 scope remain open; see
docs/TRACK_CHANGES_AUTHORING.md. No parity family or novel AI feature is accepted.

0.61 adds a transient in-document review surface with All markup, Current and
Original display, source-aware block navigation, reviewer filtering and individual
Accept/Reject through the existing guarded history. Read-only projections retain
source keys and combine anchors across paragraph joins. Current/original text,
formatting and supported row lifetimes are covered; protected files can be viewed
while review actions remain disabled. Unknown revision types fail explicitly
when they cannot be projected. Full editing within markup, simple markup,
balloons, per-character revision selection, faithful changed-table/formatting
markup, additional stories on page, performance/accessibility and Microsoft Word
comparison remain unfinished. See docs/REVIEW_DISPLAY.md; no family is accepted.

0.62 adds imported named-move source/destination navigation and paired review,
including multiple members, moved paragraph marks and supported cell/story
containers. Filtered Accept/Reject resolves the whole linked group with one Undo;
Current/Original projections and ordinary export retain its semantics. Ambiguous
or unsupported pairs fail without committing partial changes. Authoring moves,
custom XML/column/cross-story cases, exact layout, performance and actual Word
differential qualification remain open. See docs/TRACKED_MOVES.md. W13 and every
other family remain incomplete; novel AI acceptance remains 0/20.


0.64 fixes review focus after removed controls and Save return, and routes native
Edit-menu history to the active document/workbook or focused draft. 328 Rust and
272 Flutter tests pass, with two optional AI checks skipped and clean analysis.
An isolated signed preview verifies imported author setup, readable quarter-inch
gutter review, post-Save keyboard and menu Undo/Redo, assistant/dialog/formula
draft ownership, committed workbook history, DOCX/XLSX saves and byte-identical
revision-11 recovery. Native filename menu clicks pass; automated filename
Undo/Redo keys and a batched typing/Undo tail need further diagnosis. See
docs/HISTORY_MENUS.md. D03 remains incomplete; no parity family or novel AI
feature is accepted as complete.


0.65 adds section-break insertion/removal with four start types, exact text
history, linked headers, following-section formatting, source annotations and
live workbook retention. 336 Rust and 275 Flutter tests pass (two optional AI
checks skipped), static analysis is clean, and scoped engine/UI/native DOCX
artifacts pass independent XML and Microsoft365 schema checks. Native Continuous
insertion, section-specific margin editing, removal, Save As, keyboard history,
post-Save Undo and byte-identical restart pass. This does not accept W05 or D05:
recorded break edits, pagination, columns, full headers/footers, printing and Word
differential layout remain unfinished. The novel-AI milestone remains 0/20.

0.66 adds section/type-aware header/footer drafts with Link to Previous and first/
even settings. 342 Rust and 280 Flutter checks pass (two optional AI skips), plus
independent XML/schema checks on seven engine/editor and four native artifacts.
Native save/history/restart passes in an isolated preview. Header page rendering,
recorded story edits, page numbers and differential Word comparison remain open;
no complete parity family or novel AI feature is accepted.
