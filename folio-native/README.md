# Folio Native — 0.82.0 preview

Product direction and acceptance gates: [Roadmap](ROADMAP.md).
Reusable implementation instructions: [Goal prompt](docs/GOAL_PROMPT.md).

Version 0.82 extends **Scenario Manager > Merge** with **Choose workbook…**.
Read scenarios from an XLSX/XLTX file, select its worksheet and scenarios, then
copy them into the active worksheet. Source loading is read-only; copying is one
Undo step and does not apply the saved values. Custom scenario number formats
are remapped across workbook catalogs, and long duplicate names receive valid
suffixes. All 405 Rust and 395 Flutter tests pass, with two optional AI skips,
clean analysis, independent content checks and seven schema-valid automated XLSX files.
The single 0.82 preview passes native source preview/Cancel, copying, duplicate
names, application/copy Undo/Redo, custom-format transfer, summary, save/export,
restart and reimport. Fifteen checkpoints and three native XLSX files pass
independent and schema checks. Direct Excel comparison remains pending. See
[Scenario Manager](docs/SCENARIOS.md).

Version 0.81 adds **Data > Scenario Manager**: save/edit named input sets,
Show a scenario with normal validation and Undo, copy scenarios from another
worksheet and create a static comparison report. Excel scenario metadata and
structural references survive export/recovery. All 401 Rust and 391 Flutter checks pass, with two optional AI skips,
clean analysis and nine schema-valid automated XLSX outputs. The single 0.81
desktop preview passes Add/Cancel/Edit, Show/Undo/Redo, same-workbook Merge,
Delete/Undo/Redo, static Summary, save/export, restart and reimport checks.
Eighteen native checkpoints and two schema-valid saved workbooks pass independent
verification. Direct Excel comparisons and full parity remain open; see VALIDATION.md and
[Scenario Manager](docs/SCENARIOS.md).

Version 0.80 adds **Data > Goal Seek**: choose a target formula result and one
input to adjust, review the numerical solution, then Apply or Cancel. Stop,
explicit best-value acceptance, validation, automatic row sizing, Undo/Redo
and XLSX/recovery use the existing workbook pipeline. All 391 Rust and 386
Flutter tests pass, with two optional AI skips. The single 0.80 desktop preview
passes cross-sheet preview/Cancel, Apply, Undo/Redo, Excel export/save, restart
and reimport checks. See [Goal Seek](docs/GOAL_SEEK.md).

Version 0.79 adds a persistent **Watch Window** and a **Formulas** tab. Watch
values and formulas across sheets, resize the panel and columns, inspect or
navigate to a watched cell, and add/remove watches with history and XLSX
roundtrips. Row and column edits move watches with their cells. All 381 Rust
and 381 Flutter tests pass, with two optional AI skips. The single 0.79 desktop
preview passes watch addition/removal, keyboard history, live values, formula
navigation, Excel export/reimport and restart checks. See
[Watch Window](docs/WATCH_WINDOW.md).

Version 0.78 adds **Formula inspector…** to worksheet Cell actions and the cell
context menu. Follow direct precedents and dependents across sheets, inspect
linked cells, jump to exact ranges, and see unresolved-reference notices.
All 375 Rust and 377 Flutter tests pass, with two optional AI skips. XLSX recovery,
reimport, independent formula/value checks and Microsoft 365 schema validation
pass for the focused fixture. The 0.79 preview also verifies native inspection,
Back, direct dependents and exact source-range navigation. See
[formula auditing](docs/FORMULA_AUDITING.md).

Version 0.77 adds a visible **Cell actions** menu for editing, extending a range
and selecting the active cell. F8, Escape and a Finish selection control manage
worksheet range mode. Document tables have a direct **Edit cell** button. Menu
actions and delayed selections are guarded against worksheet changes. All
373 Flutter tests pass, with two optional AI skips. The 0.79 preview verifies
worksheet Edit cell, typed Escape cancellation, F8 and Finish selection;
document-table and broader accessibility checks remain pending. See
[desktop cell actions](docs/CELL_ACTIONS.md).

Version 0.76 makes accessible cell activation select one cell even when the
native keyboard state retains Shift after Export. Mouse selection also preserves
Shift across pending saves. All 368 Flutter tests pass, with two optional AI
checks skipped. The single desktop preview passes post-Export table/worksheet
selection, explicit table ranges, keyboard ranges, edit cancellation and restart;
its Word export is byte-identical to the source. Custom screen-reader range/edit
actions have automated coverage but still need native Mac support. See
[cell selection](docs/CELL_SELECTION_ACCESSIBILITY.md).

Version 0.75 adds **Sort…** in the table editor: three column/field levels,
text/number/date ordering, headings, selected row ranges and locale/case options.
Rows retain their content and formatting; Apply, Undo, annotated row movement,
Word export and recovery pass automated checks. The single 0.75 desktop preview
passes a native three-level sort, moved-cell typing, bookmark/comment navigation,
Cancel, Undo/Redo, save, restart and DOCX reopening. Swedish/German previews and
five-file schema checks pass. Native modifier/focus input anomalies still need
investigation. See `docs/DOCUMENT_TABLE_SORTING.md` for evidence and remaining
Word behavior; this remains an isolated preview, not a promoted release.

Version 0.74 supports **editable manual line breaks** in document
text, table cells and header/footer stories, using Shift+Enter or **Line break**.
Formatting, paragraph identity, proofreading positions, history and Office
round trips have automated coverage. The single 0.74 preview passes native body,
cell and header editing, spelling replacement, Cancel/Apply, keyboard history,
Word export/reimport and restart checks. Direct Word comparison, input-method
and formal accessibility qualification remain. See `docs/MANUAL_LINE_BREAKS.md`.

Version 0.73 adds **Allow row to break across pages** and independent
continuation for ordinary table cells. Native Cancel/Apply, both settings,
page continuation, keyboard history, Word export, working Save and restart
checks pass. The verified preview is running as a single instance. See
`docs/TABLE_ROW_SPLITTING.md` for the remaining merged-row, exact-height,
style and Office-comparison work.

Version 0.72 makes **table row heights consistent in the editor and Page
preview**. Exact rows stay fixed and clip overflowing content; automatic and
minimum-height rows grow for their contents. Nested table fills, borders and
text stay inside their containing cells. Native row-height controls, history,
save and restart checks pass; full row splitting and Word layout comparison
remain open. See `docs/TABLE_ROW_HEIGHTS.md` for the verified scope.

Version 0.71 adds **repeating table headings** from Table properties. Consecutive
marked rows repeat in Page preview with per-page fields while source rows stay
unchanged. See `docs/TABLE_HEADERS.md` for native and compatibility evidence.

Version 0.70 adds **column balancing before continuous section boundaries**.
Page preview redistributes the ending section's final-page content and places
the following section beneath it. Imported Word compatibility settings can
disable balancing. Previewing preserves document content and saved field
values. See `docs/COLUMN_BALANCING.md` for scope and verification status.

Version 0.69 adds **section columns** from Layout: One, Two, Three, Left and
Right presets, equal or custom widths, spacing, separator lines and reading
order. Page preview flows paragraphs and table row groups through each column,
remeasuring text when the next column has a different width. Column and page
breaks have distinct behavior. Apply/Cancel, recorded formatting, Undo and file
recovery share the existing section workflow. Direct column-break editing and
full Word layout comparison remain open. See
`docs/COLUMNS.md` for the current qualification and remaining scope.

Version 0.68 adds **physical page preview** from Layout, with section paper
geometry, shaped line flow, repeated headers/footers, table row groups, manual
page breaks and derived page-field values. Fit-page and zoom controls do not
change the document. Editing remains continuous; full Word pagination,
printing/PDF and advanced layout still need work. See `docs/PAGINATION.md`.
The isolated 0.68.0 build 1 passes the seven-page native fixture, zoom,
save/reopen and unchanged-document checks. Evidence is recorded in
`release/validation/pagination`; matching Microsoft Word layout remains open.

Version 0.67 adds **page-number fields and section numbering settings**. Insert
Page number, Total pages, Pages in section or Section number as inline fields.
Headers & footers includes number format, continue/restart and chapter settings.
Fields remain intact when editing surrounding text, bookmarks and comments.
The editor identifies saved Word results as cached values; 0.68 calculates
supported field values separately in Page preview.
See `docs/PAGE_NUMBERS.md` for scope and evidence.

Version 0.66 adds **header and footer editing** from Layout. Edit each section's
header/footer, link or unlink previous content, and choose different first or
odd/even pages. Rich drafts support Apply, Cancel and Undo; copied bookmarks
receive distinct names. Recording these edits and paginated display remain
incomplete. See `docs/HEADERS_FOOTERS.md`.

Version 0.65 adds **section-break insertion and removal** from Layout. Split at
the caret with Next page, Continuous, Even page or Odd page, then edit the new
section in Page setup. Removal explains and retains the following section's
formatting. Undo, linked workbook data, annotations and recovery are covered by
focused checks. Recording section breaks and full pagination remain incomplete.
See `docs/SECTION_BREAKS.md`.

Version 0.64 fixes **Undo after saving in document review** and connects the Mac
Edit menu to the focused document, workbook or text draft. Page-formatting
comparisons show separate changed settings with readable measurements. See
`docs/HISTORY_MENUS.md` for scope and validation.

Version 0.63 adds **section page setup**: paper size, orientation, margins, gutter,
header/footer distances and section start, with section/document scope and
recorded formatting review. The continuous editor uses Section 1’s paper width
and margins. True pagination and full Word layout remain incomplete. See
`docs/PAGE_SETUP.md`.

Version 0.62 adds **linked move review** for imported Word revisions. Navigate
between source and destination, then accept or reject the pair together—even
when a filter shows only one side. Supported text/paragraph moves participate in
Current/Original views, Undo and file recovery. See `docs/TRACKED_MOVES.md` for
scope and remaining compatibility work.

Version 0.61 adds **in-document revision review** with All markup, Current and
Original views, source-aware paragraph navigation, author filtering and individual
Accept/Reject actions. Display choices retain pending changes and stay outside
the document history. See `docs/REVIEW_DISPLAY.md` for supported cases and evidence.

Version 0.60 adds **Track Changes recording controls**: an explicit review
author, recording and formatting switches, a visible recording status and
Control+Shift+E in the document editor. Imported Word files that request
recording ask for an author before editing. Supported text, paragraph and
formatting edits enter the existing Accept/Reject, Undo and recovery workflow.
Rejected edits remain visible as unsaved drafts with Copy and Discard recovery.
See `docs/TRACK_CHANGES_AUTHORING.md` for support and verification boundaries.

Version 0.59 adds **review of existing tracked Word changes**: inline revision
markup, author/type/location filters, individual and bulk Accept/Reject,
formatting comparisons, Undo/Redo and recovery. Supported text, paragraph-break,
formatting and row revisions retain source content and live workbook links.
Full Track Changes/Compare/Merge remain open.
See `docs/TRACK_CHANGES.md` for the supported cases and evidence boundaries.


Version 0.58 adds **worksheet comments and notes**: cell markers, hover text,
right-click and Review controls, replies, editing, resolve/reopen, workbook-wide
navigation, and annotation-only Paste Special. Notes remain separate from
conversations. XLSX threads, authors, legacy placeholders, VML notes, structural
moves, copying, Undo and recovery are covered by focused checks. See
`docs/SHEET_REVIEWS.md` for boundaries; native/Excel qualification remains open.

Version 0.57 adds **document review comments**: anchored threads, explicit author
names, replies, plain-text edits, resolve/reopen, filters, navigation and Undo.
Comments persist in recovery and round-trip through Word comment/reply metadata.
See `docs/COMMENTS.md` for supported cases and remaining review work. Native and
Word desktop qualification remain pending; the normal release is still 0.53.0
build 11.

Version 0.56 adds **New from Template** and **Save as Template**, a searchable
Word/Excel gallery, six starter templates and a persistent personal library.
Create makes an independent DOCX/XLSX; Edit template explicitly edits DOTX/XLTX.
First saves, external-change checks and original-template preservation share the
working-file lifecycle. See `docs/TEMPLATES.md`. Native qualification is pending;
the normally running release remains **0.53.0 build 11**.

Version 0.55 adds **Recent Files and Recovery** with search, file-type filters,
pinned locations and safe recovery previews. Restoring a recovery copy creates an
unsaved workspace with a new identity; the first Save chooses a new `.folio` file.
Snapshots now have unique locations so preserving outgoing work cannot overwrite
an earlier copy. See `docs/RECENT_FILES.md`. Native qualification remains pending;
the normal running release is still **0.53.0 build 11**.

Version 0.54 development adds **Save** and **Save As** with separate document/workbook
locations, persistent user-selected file access and outside-change checks. Build 2
adds separate document/workbook change indicators, stable Save locations through
structural/style edits, and Save/Don’t Save/Cancel for leaving changed named files.
Build 3 extends this to changed unnamed documents/workbooks. Explicit Don’t Save
on Close leaves a blank workspace on the next launch and keeps a recovery snapshot. The
0.54 candidate has passed engine/editor/independent-reader checks; native sandbox
Save and restart qualification is pending while the Mac is locked. The reviewable
and normally running release remains **0.53.0 build 11**. See
`docs/WORKING_FILES.md` for behavior and limits.

The 0.53 source adds a searchable **Styles…** picker, creation/modification,
inherited text formatting and following-paragraph styles. Styled lists remain
editable, and table-cell style changes stay in the table draft until Apply.
See `docs/NAMED_STYLES.md` for scope and remaining work. The reviewable release
and normal running app are **0.53.0 build 11**. This build also corrects visual-line
selection, paste caret placement and typing during asynchronous Undo/Redo,
including exact edit positions within repeated text and emoji.
See `docs/DOCUMENT_HISTORY_INPUT.md` for the verified behavior and remaining
history/input limits. **Styles → Delete** now lets you replace a style that is
already in use, with Cancel and one-step Undo. Body/table text, related styles
and preserved Word references update together; direct formatting stays.
See `docs/STYLE_REPLACEMENT.md`. Build 9 adds missing Heading 1–9 styles on
use, including table cells, and protects built-in styles from deletion. Rapid
heading/Enter/typing retains the following body style while saving. See
`docs/BUILTIN_STYLES.md` for verified behavior and remaining style limits.
Build 10 adds **Paragraph → Outline level** (levels 1–9, Body text and Use style)
without changing paragraph appearance. See `docs/OUTLINE_LEVELS.md`.

Build 11 adds a native macOS **File** menu with **Command-O** Open,
**Command-N** New Workspace, **Command-S** Save Workspace Copy and
**Command-Shift-E** Export Word/Excel. File operations wait for pending edits,
avoid overlapping dialogs and preserve the outgoing workspace before New/Open.
See `docs/FILE_COMMANDS.md` for verified behavior and remaining desktop work.

Version 0.52 adds **Paragraph…** controls to the document and table-cell editors: left/right indents, before/after spacing and automatic/exact/minimum line spacing, with a rich preview and one-step Apply/Undo. Imported style inheritance and direct formatting remain separate; typing, local recovery and DOCX export retain the settings. See `docs/PARAGRAPH_FORMATTING.md` for supported behavior and remaining layout limits.

Version 0.51 expands **Paste Special** with **Values and number formats**, **Formulas and number formats**, **Validation rules only**, **Column widths only**, and **Cells with source column widths**. Number-format pastes keep destination fonts, borders and alignment; metadata-only choices preserve cell contents. The copied snapshot, measured row layout, Undo/Redo and recovery stay in one edit flow. See `docs/PASTE_PROPERTIES.md` and `VALIDATION.md` for verified scope and remaining comparison work.

Version 0.50 keeps conditional rules with copied, pasted and filled cells. Paste All and Formatting only replace rules in the destination cells; Paste Special adds **Cells and merged conditional formats** to keep existing rules too. Relative/mixed references, graphical rule populations, extension identifiers and one-step history follow the copied cells. See `docs/CONDITIONAL_COPY.md` and `VALIDATION.md` for scope and remaining Excel comparison work.

Version 0.49 adds **A date occurring** rules for ten periods, local-calendar TODAY/NOW calculation, and **Recalculate workbook (F9)**. Date/UTC-offset changes refresh formulas and conditional styles; recalculation measures automatic rows without creating an Undo entry. Imported date formulas, date systems, priority and structural references survive the supported file paths. See `docs/DATE_RULES.md` and `VALIDATION.md` for scope and evidence.

Version 0.48 adds **Icon sets** to Conditional formatting: 20 styles, custom mixes including No cell icon, icon-only display, reverse order, and number/percent/percentile/formula thresholds. Icons update with calculations, preserve cell values, and participate in Undo, AutoFit, recovery and XLSX import/export. Standard and Office 2010 rules have separate file representations. See `docs/ICON_SETS.md` for scope, source references and remaining qualification; native Excel comparison is still pending.

Version 0.47 keeps bookmarks attached during formatted internal and HTML paste, compound editor operations, and text edits across table cells. Table drafts apply as one Undo step; Cancel discards the draft. Applying immediately after typing includes the last edit. Replacing the open document invalidates an older table dialog. Automated editor and independent Office-file checks are recorded in `VALIDATION.md`; native input qualification, structural table edits and copy/cut/move bookmark semantics remain incomplete.

Version 0.46 adds **Bookmarks…** to the Word ribbon (Ctrl+G / F5). Name a selection or insertion point, sort bookmarks by name or location, show hidden bookmarks, move or delete them, and use **Go To** to select the passage. Top-level table bookmarks open the target cell editor. Ordinary text edits, spelling/grammar corrections, Find/Replace and selected-passage AI proposals supply exact edit positions; bookmarks participate in Undo and saved-workspace recovery. Word import/export preserves supported body bookmarks and internal link targets. Advanced ranges, other stories, nested table destinations, exact rich-paste/table-draft tracking and cut/move semantics remain incomplete. See `docs/BOOKMARKS.md` and `VALIDATION.md` for scope and evidence.

Version 0.45 adapts ordinary numeric cell displays to the column width. General decimals shorten or use scientific notation; fixed numeric and date formats show hashes when they cannot fit. Invalid dates fill the available width with hashes. Widening or AutoFit restores the full display, and Shrink to Fit retains its existing behavior. The formula bar, copied formatted value, calculations and Office export retain the full value. Hover a cell to read its full formatted text. See VALIDATION.md for qualification and remaining display limits.

Version 0.44 displays long text across adjacent empty worksheet cells while preserving cell ownership, selection and saved values. Ordinary left, right and centered text respects occupied cells, wrapping, shrinking, rotation, frozen panes and imported merge boundaries; text whose owning cell is offscreen remains visible. Numbers and real errors do not overflow. See VALIDATION.md for evidence and remaining Excel comparison work.

Version 0.43 retains continuous cell entry while a prior save is finishing. Values stay ordered across Tab/Enter, preserve their original modifiers and receive separate Undo steps. Later typing waits when a cell needs correction; it can resume after correcting or skipping that cell, or be discarded explicitly. Leaving the worksheet and closing the app wait for queued typing. See VALIDATION.md for evidence and remaining keyboard qualification.

Version 0.42 fixes rapid first-cell entry during the grid-to-editor focus handoff. Typed characters leave the caret at the end; characters, deletion, line breaks and completion keys arriving before the editor mounts are retained. Repeated Enter/Tab cannot reopen the old value while a save is pending. Native and automated qualification is recorded in VALIDATION.md.

A desktop-first document and workbook workspace built with Flutter and Rust, with Codex App Server as its assistant runtime. This is an evolving office editor, not yet a fully compatible Microsoft Office replacement.

Version 0.41 adds **Data bars** to Conditional formatting: automatic or explicit thresholds, solid/gradient fills, positive/negative colors and borders, axis placement, direction and bar-only display. Values and formulas remain available for editing, accessibility and copying. Standard and modeled Office 2010 data bars import/export with linked fallback records; supported row/column edits move both representations. Unknown extended settings remain preserved and visibly unavailable. Seven-digit row headers now stay on one line. See VALIDATION.md for measured scope and remaining Excel comparison work.

Version 0.40 adds **Conditional formatting** beside Format as table. Create, edit, reorder and delete worksheet rules, change Applies to ranges, and combine compatible formats with priority and Stop if true. Value/formula, text, blank/error, duplicate/unique, top/bottom, average and two-/three-color-scale rules derive their display from current values. Conditional fonts, fills and number formats feed the grid, copied display text and automatic row sizing without overwriting ordinary cell styles. The manager validates pending edits before applying them as one undoable change. In 0.40, imported data bars, icon sets, extended/pivot rules and unsupported differential styles are retained with a visible warning; they are not rendered or editable yet. Qualification and remaining limits are in VALIDATION.md.

Version 0.39 adds **Data validation** in the worksheet toolbar: whole-number, decimal, list, date, time, text-length and custom-formula rules; selection input hints; Stop, Warning and Information alerts; and searchable in-cell list choices (also Alt+Down). Typed entries are checked before committing, and corrections, cancellation, overrides and row reflow use the existing history transaction. Rules persist in `.folio` and XLSX, follow supported structural edits, and copy with ordinary internal paste and directional fill. Paste Values/Formulas/Formats retain destination validation. Clock input such as `12:30` and `1:30 PM` now parses as a fractional day. Native and independent-reader qualification is tracked in VALIDATION.md; this is not acceptance of the full validation/conditional-formatting family.

Version 0.38 fixes early worksheet dates: date parts and construction retain serial zero and the fictional February 29, 1900; working-day stepping keeps distinct holiday serials; EDATE and EOMONTH use Gregorian month-end clamps. A modeled boundary corpus, native-bridge editor checks and independent file readers are tracked in VALIDATION.md. Direct Microsoft Excel qualification and other date/coercion edge cases remain incomplete.

Version 0.37 preserves the difference between spreadsheet errors and text with the same spelling. Imported strings such as `#N/A` remain text; formula and literal errors export with their error type. Paste Values preserves source result types, and formatted text stays distinct from errors through edits, Undo/Redo and save/reopen. Engine-only formula diagnostics are exported without a cached result so Excel can recalculate the retained formula.

Version 0.36 carries an imported workbook's 1900/1904 calendar through supported date formulas, date entry, TEXT/VALUE, date criteria, working-day and financial calculations. Calendar changes invalidate cached calculation models. Structural filtering, table resizing, history and save/reopen retain the calendar. Typed dates/percentages/currencies export as numbers, booleans as booleans, and explicitly stored date-like text remains text.

Version 0.35 routes numeric `TEXT()` results through the cell number formatter and uses formatted labels in worksheet/table filters, range-sort headings and the local chart preview. Formula results remain text; source numbers and stored filter keys retain their original values. Format-code cell references recalculate through the existing edit, row-layout and Undo path.

Version 0.34 adds a shared spreadsheet number-format renderer and **More formats…** with a live preview. Decimal, percentage, currency/accounting, fraction, scientific, date/time, duration, conditional/color, and text sections now feed the grid, AutoFit, copied text, displayed-value search, and linked document tables. Formatting preserves the underlying cell value and participates in the existing atomic row-layout/Undo transaction.

Version 0.33 adds snapshot-based cell copying, quoted multiline/tab-separated paste, formula-reference adjustment, source cell formatting and rich text, and Paste Special choices for values, formulas, formatting, transpose and skip blanks. A paste and its automatic row sizing share one Undo step.

Version 0.32 adds automatic row sizing when edits, formatting, column widths or formula results change. Manually sized rows retain their height; AutoFit Row Height restores automatic sizing. Each edit and its resulting row heights share one Undo step. Version 0.31 added multiline cell and formula editing and an expandable formula bar. On Mac, Option+Return or Control+Option+Return inserts a line break; on Windows/Linux, use Alt+Enter. Enter commits and moves down, Shift+Enter moves up, and Tab/Shift+Tab move across visible columns. Escape cancels the current draft. Full Word/Excel parity remains in progress.

## Use it

On an Apple Silicon Mac, extract `release/Folio Native.zip` and move the app into Applications. The ZIP is the verified distribution build; a loose `.app` in a synced folder can acquire Finder metadata that disrupts signature checks. The app saves its current workspace locally and keeps the existing `.folio` format.

- Select text, or place the cursor in a paragraph, then choose **Ask about selection**. The selection context remains visible while you work with the assistant. Right-click also offers **Ask assistant** and **Use workbook data**.
- Choose **Use workbook**, pick a sheet and range, inspect the source preview, and attach it. The assistant can rewrite the selected passage using those figures, explain them, or propose a linked table/chart.
- Review the before/after text or cell edits, then choose **Apply changes**. One workspace Undo reverses the whole proposal. A proposal becomes unavailable if the workspace revision changes.
- Linked blocks remain connected to their worksheet. The source badge identifies the sheet/range and revision; the open-source button jumps to the sheet. Charts currently plot the first two columns as labels and values. Up to 30 data rows are shown in document previews. DOCX exports capture linked blocks as ordinary Word tables with their source label; the live link remains in `.folio`.
- **File → Open / attach DOCX or XLSX** imports a Word document or replaces the workbook attached to the current workspace. The other half of the workspace is retained, and the previous workspace is snapshotted. **Export Word / Export Excel** writes the active Office format.
- The worksheet supports Shift-click/Shift-arrow rectangular selection, copy/paste of ranges, an address box, bold/italic/fill, number/currency/percentage formats, and formulas across sheets.
- **Review → Compatibility report** explains current limits and features preserved from imported files.

The blue document and green workbook chrome, ribbon tabs, formatting controls, formula bar and worksheet tabs follow familiar Office conventions. This is a custom native Flutter interface, without an Electron or webview shell.

The worksheet lays out visible cells and a small surrounding buffer. Direct navigation reaches the full Excel row and column limits without building layout records for all intervening positions. Frozen panes, hidden positions and custom dimensions share the same sparse layout.

On macOS, Office exports and workspace copies use native atomic file replacement, including destinations chosen outside the app sandbox.

Use the chevron beside the formula bar to expand or collapse it without committing the current edit. Editing or pasting within a cell/formula field preserves line breaks in that one cell; pasting on the grid reads quoted tab-separated cells, preserving embedded line breaks, tabs, quotes and blank edges. Imported multiline text stays multiline when edited. Committing an edit automatically fits eligible rows. If a row has a manually chosen height, use **Rows and columns → AutoFit Row Height** to show all lines and restore automatic sizing for future edits. Live input-method composition retains ownership of Enter and Escape. A rejected save preserves the draft and blocks navigation/export until it is corrected or cancelled. Cell input still has the existing 10,000-byte engine limit, below full Excel limits.

Use **Copy cells** / **Paste cells** or Command-C / Command-V on the grid. The Paste arrow offers **Values only**, **Formulas and values**, **Formatting only**, **Paste text**, and **Paste special…** (also Shift-Command-V). Paste Special combines a mode with **Skip blank cells** and **Transpose rows and columns**. Select one destination cell or a rectangle whose dimensions are exact multiples of the copied range. Draft edits finish before copying or pasting; a failed draft prevents the clipboard command.

Folio keeps an immutable source snapshot in memory and places only selected text plus an opaque token on the Mac clipboard. Copying does not add an Undo step. Later source edits and overlapping destinations cannot alter that snapshot. Relative references move, absolute components stay fixed, and pasted values retain full stored precision. Full XF dependencies, inherited row/column cell formats and rich-string XML are copied within the supported paths. Source worksheet dimensions stay at their destination; automatic row sizing responds to the new content.

Clipboard parity remains incomplete: notes, hyperlinks and conditional-format rules are not copied; cut/move, HTML/RTF interoperability, external-app formatting and clipboard persistence across app restarts remain. Structured-reference formulas, merged/array/data-table areas and protected destinations are rejected atomically. Formula copies after changing worksheet names or across workspaces require copying again or Paste values. Different workbook themes/metadata need source-format conversion; Paste values is available. The in-memory token expires after another range is copied or the process exits; **Paste text** explicitly uses its public text representation. Copy/paste is currently bounded to 100,000 cells, 32 MB and the existing 10,000-byte cell limit. Actual Microsoft Excel differential qualification remains pending.

Command-Z and Shift-Command-Z undo and redo the focused text field's draft, including assistant prompts, Find/Replace, dialog values, formula input and inline cell input. The first edit is available to Undo immediately, and Redo retains text removed before a typing pause. Once a field's history is exhausted, the shortcut stays in that field. With document or grid focus, the same shortcuts use workspace history. Formula draft history starts fresh when you select another cell or its committed value changes, so Undo cannot copy a previous cell's value into the current one. Moving between fields preserves their separate drafts. Live IME composition stays intact; completing it creates one draft edit and cancelling it retains existing Redo. These paths have widget regression coverage, including the real Rust workspace; packaged desktop and native IME qualification remain pending.

The macOS runner recovers drawing for an active, visible, focused window when stale occlusion information would otherwise pause Flutter 3.44. Native menu activation and moving between the built-in and external display are verified. Recovery follows activation, window and display events, with guards for hiding, minimization, screen sleep and session suspension; it does not run a continuous redraw timer. It can be removed after upgrading to an engine with the [upstream lifecycle fix](https://github.com/flutter/flutter/pull/188772) and repeating the native checks, including display transfers. Physical sleep/wake, repeated locking and final native minimize/restore qualification remain outstanding.

## Spreadsheet number formats

Select cells and choose **More formats…** beneath the number-format dropdown. Pick a preset or enter an Excel format code (up to 255 characters), inspect the first selected cell's preview, then Apply. Mixed selections are identified. Invalid codes cannot be applied; Cancel leaves the workbook unchanged. Concurrent edits invalidate an open dialog's Apply. Font color rules such as `[Red]` and `[Color3]` use the chosen numeric section and workbook indexed palette. `_x` reserves the actual glyph width; `*x` fills available cell width. AutoFit measures the unfilled content. Formula errors stay visible even under a format such as `;;;`.

The Rust renderer uses a pinned, locally patched [ssf-rs 0.1.0](https://docs.rs/ssf-rs/0.1.0/ssf_rs/) port of SheetJS SSF. Folio adds shared display caching, color/layout projection, explicit invalid-format fallback, correct time carry through noon/midnight, bounded format input, and retained 1900/1904 date-system display. Failed imported formats retain their values and expose a tooltip. Values-only internal paste still uses numeric results; copying public text uses the full formatted value, independent of narrowing or hashes in the grid.

Number-format parity remains incomplete. Width-sensitive General/scientific fallback and overflow hashes, complete locale/calendar and localized built-in handling, complete date/coercion semantics (including remaining early-date financial/week-number and negative-date cases), all TEXT coercion/error/array semantics, formatted-value filter identity and date grouping, rotated/stacked/bidirectional accounting layout, all special format grammars, and direct Microsoft Excel comparison remain. A 1904 workbook's compatibility report identifies its date system; supported calculations now use it. The independent SSF comparison is a renderer comparison, not Excel qualification. The packaged 0.34–0.36 formatting/calendar paths still need native testing.

`TEXT(value, format_text)` accepts a format code or a cell containing one, including supported fractions, scientific notation, dates and durations. Store numeric-looking format codes as text (for example, enter an apostrophe before `0.0000`). It produces plain text: color and width-dependent accounting fill do not become text styling. Invalid numeric patterns return `#VALUE!`. Values and format-code arrays broadcast in the calculation engine, but visible dynamic-spill integration remains incomplete. Direct Excel comparison for all input coercions and error precedence is outstanding.

Filter lists show the cell's formatted text while retaining the existing raw-value identity. Different numbers that round to the same label remain distinct, and the first occurrence supplies the label for identical raw values with different formats. Full Excel formatted-value grouping/filter criteria semantics are not yet implemented.

## Worksheet alignment

Choose **Alignment…** in the worksheet ribbon (the alignment icon beside the font name in compact windows). Set General/Left/Center/Right/Justify, Top/Middle/Bottom, indent, Context/Left-to-right/Right-to-left reading order, Wrap text, Shrink to fit, rotation from −90° to 90°, or stacked characters. Positive rotation tilts text upward. The preview uses the first selected cell; mixed settings remain unchanged unless you edit them. Reset clears the draft, Cancel discards it, and Apply creates one Undo step. The wider ribbon also provides Left/Center/Right and Wrap shortcuts. Pending input commits separately; an open dialog rejects changes after the workbook revision changes.

General aligns numbers to the right, booleans to the center and text according to its reading direction. Wrapping follows the column width; manual line breaks remain visible. Shrink-to-fit scales single-line text without changing its saved font size. Indent uses three normal-font spaces per unit; entering an indent chooses Left if the current horizontal choice does not support it. Rotation and stacked text retain character font properties; stacked text keeps Unicode grapheme clusters together.

Automatic rows refit after changes to content, fonts, alignment or column widths. Formula results on other worksheets also participate. Rows imported with fixed heights stay fixed until you use AutoFit Row Height. Hidden/filtered rows keep their stored heights; newly visible automatic rows refit. Rows containing merged cells retain their previous height. Imported Fill, Distributed, Center across selection, vertical Justify/Distributed and last-line justification settings are preserved in files but do not yet have complete on-screen behavior or authoring controls. Combined rotation/wrapping/accounting-underlines, exact Excel positioning, overflow into empty neighbors, comprehensive bidirectional and accessibility qualification, and large selections still need work. These controls do not establish complete formatting parity.

## Worksheet fonts

Click the font name/size above **Bold** to open **Font**, or use ⌘1 on macOS (Ctrl+1 also works). Choose an installed font or enter a family name, set a size from 1 to 409 points in half-point steps, and edit color, bold, italic, strikethrough, single/double or accounting underline, and superscript/subscript. The preview uses the first selected cell. Mixed options remain unchanged unless you edit them; **Reset changes** restores the draft, **Cancel** discards it, and **Apply** creates one workspace Undo step. Pending cell input commits separately, and a changed workbook invalidates an open font dialog.

For one text cell, turn on **Format selected characters** in the Font dialog. Highlight the desired characters with the mouse or keyboard, then choose the font settings. The highlight remains visible while you use the controls, and the preview shows the entire cell. Selecting a different span resets the draft settings. **Apply** changes only the selected characters and remains one workspace Undo step. This mode is for literal text, including numeric-looking text stored with a leading apostrophe; it is not offered for numeric values or formulas.

With grid focus, ⌘B/⌘I/⌘U and Ctrl+B/Ctrl+I/Ctrl+U toggle bold, italic and underline. Focused text fields keep their own input behavior. The grid uses point-sized fonts, text effects and a cached text layout. Imported default, row and column fonts apply to blank cells as well; direct cell styles take precedence. The font list comes from macOS, and missing families retain their names for export while using a local substitute.

Font edits preserve unrelated XLSX cell formatting. Whole-cell font commands also update the selected properties of each rich-text run; shared strings are copied into the affected cell so other users of that string remain unchanged. Other rich-run properties and text boundaries are retained in the file. The grid and Font preview display individual rich runs; mixed options are also detected within one cell. Whole-cell changes override only the chosen properties. Changed text does not display stale character runs. Existing workspaces gain the new run metadata while preserving user edits; old phonetic-text import mistakes are repaired only when values still exactly match that old import. Rich inline text editing and character-format shortcuts in the cell/formula editor, exact Office typography, theme/indexed color resolution and named-style/default-font controls remain incomplete. Native packaged and Microsoft Excel comparison checks are still pending.

Excel cell strings are decoded once, including encoded supplementary characters. Literal text resembling an Excel escape is protected on export. Formula expressions retain their existing XML handling; string results use the cell-text codec. Older unchanged imported values and unambiguous character selections are upgraded without replacing user-edited values. If a legacy character selection ends inside an escape token, opening reports an error and leaves the file and current workspace unchanged; that file still needs the previous Folio version. String metadata such as sheet names, number-format codes and font names is outside this fix.

## Word tables

Click a table cell or **Edit table** to open a rich cell editor. Formatting and multiple paragraphs remain editable. Use **Rows and columns** to insert or delete rows/columns. Shift-click another cell, or turn on **Select cells**, to choose a rectangle, then use **Merge cells**. **Split cell** separates an existing merged area into its grid cells while retaining the content in the first cell. **Apply changes** commits the table as one workspace Undo step; Cancel discards the draft.

Select document text and choose **Text to table**. Pick tabs, commas, paragraph breaks or one custom character, set the column count and an optional width in points, inspect the preview, then choose **Convert**. Paragraphs normally start rows; when paragraph breaks are the separator, paragraphs fill cells across the chosen columns. Short rows are padded and excess cells wrap. The preview shows at most ten rows; conversion supports 1,000 rows/columns and 100,000 total cells. A partial paragraph selection becomes a table between its remaining text.

Choose **Convert to text** below a table to replace all or selected rows with separated paragraphs. Set the first and last row in the preview. Alternatively, select cells in the table editor and choose **Convert rows to text**; **Convert and apply** saves the current table edits and conversion together. Cancel returns to the table draft. Rows above and below the conversion remain as tables with their widths, formatting and merges. A range touching a vertical merge expands to include it, with the expanded range shown before Apply.

You can convert nested tables too or retain them. Text in merged cells appears once, with covered grid positions represented by empty fields. Rich character formatting is retained; joined paragraphs take the first paragraph’s formatting. Existing lists and opaque objects stay separate to preserve their structure. Each conversion is one workspace Undo step, and changing the source table invalidates an open table-edit preview. Disjoint row selections, converting only part of a row, Word AutoFit modes, and exact behavior comparison with Microsoft Word remain unfinished. The new row conversion has widget and independent DOCX-reader evidence; packaged native qualification is pending.

Select one or more document paragraphs and choose **Tab stops…**. Add, update or remove left, center, right or decimal stops in points, or clear all custom stops. Choose no leader, dots, dashes, a solid line, a heavy line or middle dots for the gap before a stop. The preview uses the first selected paragraph’s rich text; a notice identifies mixed paragraph settings. Default spacing applies throughout the document, following the last custom stop. **Apply** is one workspace Undo step; Cancel discards the draft, and an intervening document revision invalidates the preview.

DOCX import reads the default spacing, direct tab declarations, document defaults, paragraph-style chains and numbering-level tab definitions. Direct clears keep removed inherited stops from returning in Word. Settings survive ordinary typing, proofreading/AI edits, saving and reopening. Document editors, table-cell editors and static table previews use measured tab layout. Real tab characters remain intact for selection, clipboard, spelling and Undo. Leaders draw inside the actual tab gap, using the paragraph’s text color and size; they do not add characters to the document or clipboard. Unknown imported leaders and other alignment types remain in the file, with an incomplete-preview notice. Exact leader typography, bar rendering and authoring, ruler controls, implicit hanging-indent stops, table-style tab inheritance, style-change recalculation, complex list indentation, mixed-direction and overlong aligned fields, and differential Microsoft Word qualification remain unfinished.

After conversion, the caret moves past the converted content so typing does not replace the table. On macOS, keyboard input recovers if closing a dialog leaves the window frame as the first responder. Active text fields and native Save panels keep their input route.

The status word count includes text inside tables and nested tables, and updates after Apply and Undo.

Open **Table properties** from the table editor to set each selected column’s width in points, distribute selected columns evenly, choose automatic/minimum/exact row heights, set cell fill and vertical alignment, or align the whole table. Only changed fields apply to the draft. Width changes use fixed widths; wide tables scroll horizontally. Exact row heights clip the preview while preserving the complete cell text in the editor.

The document displays horizontal/vertical merges, unequal column widths, rich text and nested tables. Word import/export retains original table, row and cell properties while patching changed content and grid geometry. Unsupported omitted-grid, legacy horizontal-merge and revision-property structures stay preserved as read-only blocks. Table styles and exact Office typography/layout, dragging column widths, arbitrary split counts, sorting, page breaks/repeated headers, and full inline table authoring still need work.

## Word lists

The list buttons create editable bulleted and numbered paragraphs; indentation retains nested levels. Select a numbered paragraph and open **Numbering** to restart at 1, continue a previous list, or choose a starting value and decimal, letter, or Roman number style. The command applies from that paragraph through its following contiguous list items and is one workspace Undo step. Typing or pasting multiple lines into an imported list retains its numbering instance; Enter on the final empty item exits the list.

Common imported Word numbering now displays starting values, overrides, nested labels, continuation across prose and paragraph-style inheritance. DOCX export writes real numbering definitions and preserves inline marks/links, source numbering parts on text-only edits, and unmodeled source paragraph properties. Complex and picture numbering, full list-definition editing, exact list typography and table-list rendering still require further work.

## Find, replace and navigate

Use the search icon beside the ribbon tabs, **⌘F / Ctrl+F** for Find, or **⌘Shift+H / Ctrl+H** for Replace. **⌘G / F3** advances to the next result; add Shift for the previous result. Enter in the Find field also advances. Escape closes the panel.

- In documents, search body paragraphs, headings, lists and table cells. Match case or whole words, click a result to reveal its location, or use **Headings** to navigate long documents. A table result reveals the table; its row and cell appear in the result label.
- In worksheets, choose the current sheet or the entire workbook, row or column order, case sensitivity and entire-cell matching. **Formulas & text** searches editable inputs. **Displayed values** searches calculated/formatted cell values and is read-only. Selecting a workbook result opens its worksheet and scrolls to the cell.
- **Replace** changes the selected document occurrence or all matching occurrences inside the selected cell. **Replace all** applies the current scope as one workspace transaction. One Undo restores it. Changed content invalidates the results before replacement; original text formatting outside a match and worksheet styles remain intact. Replacement text inherits the first matched text run's style.
- Search is literal: wildcard/format searches, selection-only scope, headers/footers, comments, and paragraph-break replacement are not implemented yet. Up to 10,000 results are listed; if there are more, the panel says so and disables Replace all until the search is narrowed. Unsupported embedded objects remain intact and are not searched.

See [PARITY.md](PARITY.md) for the broader Word/Excel requirements and the separate 20-feature AI acceptance gate.

## Fill cells

Select a worksheet range and choose **Fill → Down, Right, Up or Left**. **Ctrl+D** fills down and **Ctrl+R** fills right while the grid has focus. On macOS, **⌘D / ⌘R** work as well. The source is the top, left, bottom or right edge of the selection. For a single row or column, the command uses the adjacent source row/column; it leaves the selection in place. Formula-bar and inline-cell editing keep their own keyboard behavior.

Fill copies values, formulas and cell formatting. Relative A1 references move with each destination; absolute and mixed references retain their locked axes. Blank source cells clear destination content and direct formatting. Every selected physical cell participates, including hidden rows and columns. Pending edits commit before the fill and remain a separate Undo step; a failed pending edit prevents filling. One workspace Undo restores the fill, including imported file records.

For imported XLSX files, filling copies complete cell styles and rich text from the source records, resolves inherited row/column styles, expands shared formulas, refreshes calculated caches and retains worksheet geometry. Save/reopen/reimport and exact original-file restoration after Undo are independently checked. Merged ranges, array/data-table formulas, protected worksheets and horizontal filling of table-column references currently reject the operation without applying it. Other unsupported formula syntax can also reject copying. Fill currently supports up to 100,000 cells and the existing workspace/file size limits.

Fill handles, series/custom lists, Flash Fill, across-sheet fill, visible-only/filter-specific behavior, notes/hyperlink/conditional-rule copying, full formula syntax and differential Excel qualification remain unfinished. The directional controls have native-bridge widget coverage; packaged desktop interaction remains pending.

## Rows, columns and worksheets

Right-click a row number or column letter to insert, delete, hide, unhide adjacent rows/columns, or set its exact height/width. The **Rows and columns** menu in the workbook toolbar operates on the rows or columns covered by the current selection. Right-click a worksheet tab, or use **Sheet options**, to rename, move left/right or delete a sheet. Sheet deletion has a confirmation; every operation supports one-action Undo.

Structural edits update cell addresses, formatting, supported formula references across sheets and document-linked ranges together. Deleted references become `#REF!`; deleted document sources show a readable message. Sheet names are validated, including duplicate names. Pending edits are committed before the structure changes.

For imported XLSX files, the tested transformation includes ordinary/shared cell formulas, names and print ranges, row heights/column widths, merges, filters, frozen panes, validation/conditional-formatting ranges and formulas, comments, internal hyperlinks, and basic chart/drawing anchors. Shared formula followers are expanded into independent formulas when editing their worksheet. Stale calculation/chart caches are cleared and Excel is asked to recalculate.

This is partial structural compatibility. Pivots, external workbook links, macros, some extension ranges and formula syntax cannot yet be safely transformed and produce an error without committing the operation. Partial array edits are rejected. Row/column move, automatic numeric column growth, sheet duplication, all drawing anchoring modes and exhaustive Excel comparison remain unfinished.

Set **Row height…** in points and **Column width…** in character units, or drag the boundary beneath a row number or to the right of a column letter. A size of zero hides the selection while keeping its previous size for Unhide. The toolbar can hide/unhide every row or column covered by a selected range and offers **Unhide all rows / columns**. Enter a range such as `A1:D8` in the address box to select it, including hidden positions. Arrow keys and Tab skip hidden rows/columns. Shift+Tab moves to the previous visible cell without extending the selection. Unhide changes manual visibility; active filters still apply.

Dimensions and manual visibility survive `.folio` save/reopen and XLSX export, including empty rows with custom heights and grouped-column formatting. Insert/delete operations move explicit dimensions with their original rows/columns. Resizing supports heights up to 409 points and widths up to 255 characters; each operation covers up to 100,000 rows or columns. Automatic numeric column growth, changing sheet defaults, all inherited-size behaviors, outline controls and full default-width inheritance, font substitution and direct Excel width comparison remain unfinished.

**AutoFit Column Width** fits natural text lines across all populated cells in the selected columns, including offscreen rows. Rich font runs, indents, signed/stacked text and displayed number formats contribute to the measurement. Manual line breaks stay intact. Wrap and Shrink settings remain saved; measurement does not constrain itself to the previous width. Hidden and filtered rows are ignored, hidden columns stay unchanged, and empty columns use the worksheet default. Merged columns reject the operation without applying partial widths. Automatic rows refit to the new widths in the same Undo step. Fixed row heights stay unchanged; use row AutoFit to restore automatic sizing.

Imported widths, numeric width entry, dragging and fitting now share the documented Excel width conversion. The widest digit in the workbook's Normal font determines the pixel scale; localized Normal styles are identified by their built-in id and font record rather than by an English name or cell XF 0. Manual entry uses character widths; exported widths retain their 1/256-character precision. Font substitution, platform rasterization, extreme sizes and direct Excel comparison still need qualification. Unspecified column widths retain the existing Folio default until complete sheet-default inheritance is implemented. Fitting stores an explicit size, not Excel's automatic numeric best-fit growth policy. Refit after later content, style or font changes.

**AutoFit Row Height** measures all populated and explicitly styled cells in the selected rows, including offscreen columns and calculated values. It uses the grid's current fonts, rich runs, wrapping, indentation, rotation, reading direction, text scaling and header-button space. The whole-row command keeps manually hidden and filtered rows unchanged and ignores hidden columns. Empty rows return to the default-height minimum; explicit row/cell fonts can increase that minimum. Height is limited to 409 points. If a row in the selection contains an imported merged cell, the operation reports that it cannot fit those rows and leaves the entire selection unchanged; use manual row height for now.

Pending input commits before measurement and has its own Undo step. A second invocation shares the current operation; identical automatic heights do not add an undo step. Switching a fixed row back to automatic is an undoable policy change even if its measured height is unchanged. Large operations yield to the interface and offer Cancel. Changed workbooks invalidate pending measurements. The resulting point heights survive save/reopen, XLSX export and row insert/delete. Row AutoFit stores both the measured height and an automatic-sizing policy; later edits refit it. Manual numeric sizing or dragging makes the height fixed again. The policy survives save/reopen and XLSX export/reimport through the row customHeight flag. Older workspaces recover unchanged source modes while retaining changed explicit heights as manual. Direct Excel visual comparisons, exact Office font/rotation/accounting metrics, merged-cell behavior and representative large-file performance remain unqualified.

## Frozen rows and columns

Open **Freeze panes** in the worksheet toolbar. Choose **Freeze panes at selection** to keep the rows above and columns left of the active cell visible, or choose **Freeze top row**, **Freeze first column**, or **Unfreeze panes**. Hidden rows and columns retain their original addresses and do not occupy space in the frozen area. A divider marks each frozen boundary.

Frozen boundaries survive workspace save/reopen and XLSX export and participate in Undo/Redo. Pending cell and formula-bar edits commit before the view change. Imported frozen panes use their saved scrolling origin separately from the frozen row/column counts. Selecting a frozen cell keeps the scrolled area in place; navigating elsewhere reveals the target beside the headings.

If the saved frozen area is too large for the window, a notice appears and the oversized axis scrolls temporarily. The stored boundary is retained and resumes pinning when it fits. Ordinary split panes and other source workbook views are retained in exports; interactive split panes, view switching, zoom, custom upper-pane scroll origins and exact split restoration after unfreezing a source frozen/split view remain unfinished. This is progress toward worksheet-view parity, not completion of that track.

## Excel tables, sort and filter

Use **Format as table** in the worksheet toolbar, confirm a range containing column headings and data, and optionally name the table. Selecting one of its cells reveals **Table options**: multi-column sorting, filters, calculated columns, table names, four color styles and totals. The arrow on each table heading opens its filter. Value checklists and one or two text/number conditions are supported; **Reapply filters** refreshes criteria after editing values.

Filtered rows disappear from the grid while retaining their Excel addresses. Arrow-key navigation skips hidden rows. A calculated column accepts formulas such as `=[@Qty]*[@Price]`; entering data immediately below the table extends it and fills its calculated columns. Renaming a heading updates matching structured references. Table sorting is stable and excludes headings and totals. The totals row supports Sum, Average and Count in the menu; the engine also supports numeric count, minimum, maximum, sample standard deviation and sample variance. **Custom total formula** accepts your own expression, such as `=SUM(Sales[Amount])*0.08`. Editing a total directly updates its stored definition. Custom formulas and text labels survive hide/show, renaming, structural edits and save/reopen; choosing a standard total replaces the custom definition.

Table metadata, filter state, styles, calculated-column definitions and totals round-trip through XLSX. Sorting moves source rich-text cells, notes and hyperlinks with their records. Row/column insertion and deletion update table ranges, columns and references. Each operation is undoable, and source packages survive `.folio` save/reopen. Existing saved workspaces recover table metadata from retained Office packages when loaded.

Use **Table options → Resize table** to expand or shrink its range. Keep the header on the same row and overlap the existing table. Removed rows and columns remain on the worksheet; their local table formulas become ordinary cell references. New columns acquire unique headings, blank cells in new calculated rows inherit the column formula, and existing values remain. Filters are reapplied and retained sort columns keep their priority. If totals move, their new cells must be empty; the tested cell formatting, notes and hyperlinks move with them. Undo restores the complete previous table and source package.

Use the **Sort** icon for ordinary worksheet ranges. Confirm the range, choose whether it has headers, add up to 64 sort levels, choose ascending/descending and optionally case sensitivity. Only cells inside the chosen columns move; hidden rows remain in place. A single active cell suggests the surrounding data, while a selected rectangle keeps its boundaries. The last sort settings are retained and reused when the active cell is inside that range. Sorting does not create an Excel table. Tested source formatting, rich text, notes and hyperlinks move with their visible records, and one Undo restores the previous order. Formulas recalculate after sorting, so their displayed values can change.

Use **Worksheet filters → Filter a range** for ordinary cells. Include a heading row, then use the arrows on those headings to choose values or text/number conditions. One ordinary filter range is supported per worksheet, alongside non-overlapping tables. Choices reflect the other column filters. **Reapply worksheet filters** refreshes results after cell edits; **Clear worksheet filters** keeps the arrows, and **Remove worksheet filter** removes them. Each action supports Undo and saved-workspace recovery. Manual hidden rows remain hidden when filters are cleared.

`SUBTOTAL(1–11, …)` includes manually hidden rows; `SUBTOTAL(101–111, …)` excludes them. Both exclude filtered rows, including references outside the filter's columns and from other sheets. Folio supplies the exact filter results to its patched IronCalc engine. XLSX exports include ordinary AutoFilter criteria, row visibility, filter mode and the worksheet's filter database name. Successful formula results refresh their exported caches without replacing source formula metadata; unsupported array/data-table formula-cell caches are cleared for recalculation. A private worksheet extension retains manual-versus-filtered provenance in Folio round trips; it is ignored if the standard range, criteria or visible rows disagree. Other editors may discard that extension.

This is partial table parity. Convert-to-range, advanced/date/color/top-ten filters, slicers, left-to-right, custom-list, color and locale sorting, and the complete table design gallery remain unfinished. Unsupported source filters are retained; clear or replace them before resizing. Tables cannot overlap merged cells. Adding or moving totals requires empty destination cells; shrinking over occupied totals destinations requires hiding totals first. Range sorting and filtering currently support up to 100,000 rows; filter menus show at most 10,000 unique values. outlined groups, intersecting merges/array formulas and partial-table selections need further support and are rejected without changing the workbook. Import cannot always distinguish a manually hidden row from a filtered row in files produced by other editors, because standard row visibility does not record the reason. Header-only source filter ranges and unsupported filter kinds remain retained rather than fully editable. Direct comparison in Microsoft Excel, every style/object behavior during resize, and all structured-reference forms remain outstanding.

## Spelling and grammar

Choose the **Spelling and grammar** icon beside the ribbon tabs, or **Review → Spelling & grammar**, to open the Writing Editor. Built-in checks run locally as you type. Red waves identify spelling; blue waves identify grammar; optional purple waves identify style suggestions. Click a suggestion in the panel or select a marked word and use its context menu. The right-click menu separates spelling suggestions from editing and assistant actions with a divider. Each accepted correction is undoable.

- Built-in engine: [Harper 2.11.0](https://github.com/Automattic/harper/tree/v2.11.0), embedded in the existing Rust library. No network service, Java runtime or separate account is required for automatic checking.
- Languages: English US, UK, Canada, Australia and India. Choose the proofing language explicitly; other languages are not yet supported by the built-in engine. Grammar is on by default; style is optional.
- Add names and specialist terms to a personal dictionary, manage that dictionary, or ignore an individual suggestion until the next edit. Settings and words are saved locally beside the workspace in `proofreading.json`.
- **Check selection / paragraph** and **Check entire document** ask Codex for a contextual spelling and grammar review. The assistant submits small corrections with before/after text. Nothing changes until **Apply changes**; one Undo reverses the proposal. Facts, numbers and voice are included in the preservation instructions.
- Diagnostics are display-only and never become DOCX formatting. Corrections preserve formatting outside the corrected span; replaced text inherits its original starting style. Review corrections across mixed styles before applying.
- Current scope: document prose, excluding embedded tables, linked blocks, code and URLs. Spreadsheet cells are not automatically proofread yet. Built-in passes skip paragraphs longer than 12,000 UTF-16 units and text after the first 100,000 units. AI review supplies up to 60,000 units of eligible passages. The UI/assistant reports a partial check when these limits apply.

Why this engine: Flutter's [built-in spell service](https://api.flutter.dev/flutter/material/TextField/spellCheckConfiguration.html) and [`flutter_spell_checker`](https://pub.dev/packages/flutter_spell_checker) do not supply desktop grammar checking. Harper provides both spelling and grammar in Rust and fits this native, offline architecture. This is an engineering fit, not a demonstrated claim that it catches every error or outperforms commercial grammar products. Context-sensitive suggestions can be wrong; the AI second pass also remains reviewable.

## Assistant and A2UI

The app uses the official Flutter [GenUI renderer](https://pub.dev/packages/genui) and [A2UI v0.9 protocol](https://a2ui.org/specification/v0.9/). Its `com.folio.office` catalog includes Text, Column, Button, TextField and WorkbookRange. The assistant can create forms, choices and source previews through `present_ui`; user actions return to the Codex conversation.

Generated forms are limited to the local catalog. They cannot inject scripts, load remote media or directly apply workspace edits. The host validates message sizes, surface/component counts, referenced children, recursion depth and cycles before rendering. Normal buttons submit intent; edits still require a proposal and explicit Apply.

The Dart client starts the bundled **Codex 0.154.0 native App Server** over stdio and exposes `read_workspace`, `read_range`, `present_ui`, and `propose_changes`. No separate OpenAI API key is embedded. Use Connect to sign in through the supported ChatGPT flow. AI requests send requested workspace context to OpenAI. Workspace previews are bounded; exact numerical work uses explicit range reads.

Selected-text proposals carry the workspace identity, revision and exact original text. The client materializes the replacement using a copy of the editor document, preserving surrounding formatting. Rust validates and commits the final document and cell edits atomically. A stale proposal is rejected.

The app requests a read-only Codex thread and declines runtime approval requests. The macOS App Sandbox remains enabled. These measures and the local tool catalog are not a comprehensive security audit of the upstream Codex runtime.

## Office compatibility

| Area | Implemented | Remaining limitations |
| --- | --- | --- |
| DOCX | Paragraphs, headings, common run formatting, links, simple tables; package-preserving import/export | Word pagination, section layout, tracked-change editing, embedded object rendering, equations and advanced table layout |
| XLSX | Multiple worksheets, sparse cells, formulas, cross-sheet references, basic styles, row/column insertion/deletion, sheet management, shared-formula expansion, tables, structured references, table sorting/filtering and totals | Not every Excel formula; array formulas, dynamic arrays, complete date/locale formatting, named ranges, charts/pivots, advanced conditional formats and workbook UI fidelity |
| Original packages | A no-change export returns the original bytes. Edited exports retain untouched package parts, including tested headers, images, charts, comments, freezes and merges | Preservation does not mean Folio displays or edits those objects. Editing a paragraph/table rebuilds its supported representation; unsupported formatting within that edited structure may change |
| Linked data | Live tables/charts in `.folio`, source range badges, preview and jump to source | Office export captures ordinary table data, not a cross-application live link or editable Excel chart |
| Other formats | `.folio` persistence; HTML/CSV export operations remain in the core | `.doc`, `.xls`, encrypted/password-protected Office files and VBA execution are unsupported |

The calculation engine is [IronCalc](https://github.com/ironcalc/ironcalc), replacing the initial small arithmetic parser. It supports a substantially broader Excel-style function set and references between sheets; it does not establish complete Excel parity. Imported formula caches and advanced formula structures may remain preserved in the source package; edited XLSX exports request a full recalculation in Excel. The native grid displays its own engine results.

Current bounds: 32 MB per compressed Office file; 128 MB total expanded package; 32 MB workspace model excluding embedded original packages; 128 MB saved `.folio` file; 30 sheets; 100,000 populated cells per sheet; cell input up to 10,000 bytes. Excel addresses A1:XFD1048576 are accepted and the viewport is virtualized; this is not a promise to load a densely populated Excel-size sheet. Linked ranges and formatting selections are limited to 2,000 cells. Undo retains up to 100 in-memory snapshots within a 24 MB model budget. Undo and assistant conversations are not retained across app restarts.

Full Office compatibility remains a product target. The next acceptance work needs a representative customer file corpus and comparison against Microsoft Word/Excel themselves, including editing and saving there. Independent XML/library validation is useful but does not replace that qualification.

## Stack

| Layer | Implementation |
| --- | --- |
| Desktop UI | Flutter 3.44.7 / Dart 3.12.2 |
| Document editor | Flutter Quill 11.6.0 with loss-preserving block embeds |
| Spreadsheet viewport | Sparse indexed layout on Flutter’s two-dimensional viewport |
| Assistant UI | GenUI 0.10.3, a2ui_core 0.1.1 |
| Data, validation, persistence, Office packages, history | Rust 1.98.1, serde_json, zip, roxmltree |
| Formula engine | ironcalc_base 0.8.3 |
| Number display | ssf-rs 0.1.0 with local time-carry and layout patches |
| Native bridge | flutter_rust_bridge 2.13.0, asynchronous worker pool |
| Agent integration | Codex 0.154.0 App Server, JSON over process stdio |
| macOS integration | Swift runner, native file selection and browser sign-in |

## Build and verify

Keep at most one Folio preview running. Reuse the current instance for checks.
Before replacing it, preserve unsaved drafts, quit through the app, and run
`python3 scripts/verification/check_preview_processes.py --expected 0`.
Launch only after that check passes, then check `--expected 1`. Retain documents
and recovery data when retiring a preview.

Requirements: Apple Silicon Mac, Xcode, Flutter stable and Rust. The current build uses Flutter 3.44.7 and Dart 3.12.2.

```sh
flutter pub get
bash scripts/build_macos.sh
```

The build script stages Flutter sources outside synced folders, builds Rust, bundles the native engine and Codex runtime, signs with the existing sandbox entitlements, verifies the staged bundle and creates `release/Folio Native.zip` and `.app`. File-provider metadata may affect signature checks on the copied app folder; the archive is made from the verified staged bundle before that metadata is attached. This local build is ad-hoc signed, not Developer ID notarized for public distribution.

On a clean checkout, `scripts/fetch_codex.py` downloads the pinned official native npm archive and verifies SHA-512. No Node.js installation is needed for the app. `FOLIO_CODEX_RUNTIME` and `FOLIO_BUILD_STAGE` override the runtime and build-stage paths.

```sh
cargo test --manifest-path rust/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
flutter analyze
flutter test
# Performance baseline (opt-in; see docs/PERFORMANCE_BASELINE.md)
cargo test --release --manifest-path rust/Cargo.toml perf_baseline -- --ignored --nocapture
FOLIO_PERF=1 flutter test test/perf_baseline_test.dart
python3 scripts/test_macos_keyboard.py # macOS; in-memory events, no app launch
python3 scripts/test_macos_lifecycle.py # macOS; synthetic notifications, no app launch
flutter test test/live_agent_test.dart test/live_grammar_test.dart --dart-define=FOLIO_LIVE_TEST=true
```

The live test uses disposable sample data and an existing development Codex sign-in. It checks range reads, a real A2UI surface, proposal staging, Apply and Undo. See `VALIDATION.md` for the verified scope.

Development can select binaries with `FOLIO_CORE_PATH` and `FOLIO_CODEX_PATH`. Generated bridge code is checked in. A change to the Rust API signature requires regenerating it with flutter_rust_bridge_codegen 2.13.0.

## Recovery and source layout

The sandbox application-support directory contains `FolioNative/workspace.folio` and `Snapshots/`. Damaged recovery files are copied to timestamped `.unreadable-*` backups before replacement. Save a copy provides a user-controlled portable workspace, including original imported Office packages.

- `lib/main.dart`: desktop workspace, source selection, proposal review and file commands.
- `lib/src/proofreading.dart`, `writing_panel.dart`, `rust/src/proofreading.rs`: local diagnostics, dictionary, decorations and safe grammar corrections.
- `lib/src/document_*`: rich-text editing, conversion and exact selection anchors.
- `lib/src/assistant_ui.dart`: validated native A2UI catalog and surfaces.
- `lib/src/workbook_block.dart`: linked source previews and document blocks.
- `lib/src/sheet_view.dart`: virtualized sheet and formatting controls.
- `rust/src/workspace.rs`: canonical model, transactions, packages and history.
- `rust/src/docx.rs`, `xlsx.rs`, `xlsx_styles.rs`, `office.rs`: Office package projections and exports.
- `rust/src/calculation.rs`: persistent IronCalc model; document-only edits do not recalculate unchanged inputs.

Upstream components retain their own licenses. IronCalc is MIT OR Apache-2.0; Codex is Apache-2.0 with its bundled components carrying their own terms. Complete a third-party notice audit, signing/notarization and platform acceptance before public distribution.
