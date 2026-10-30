# Shared desktop requirements (D01–D10)

States and ID rules: [README](README.md). Family scope: [PARITY](../../PARITY.md#shared-desktop-requirements) and ROADMAP §4 S01–S07.

## D01 Desktop application, windows and Office-like interaction

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| D01.01 | Launch to an editable blank workspace with the Word and Excel editors in one window, switching views without losing state | M1 | Native verified | 0.43 and later native restarts ([VALIDATION](../../VALIDATION.md)) | Cold-launch budget (ROADMAP §7) unmeasured |
| D01.02 | Ribbon tabs grouping the existing commands (Home, Insert, Layout, Review, Formulas, Data) with keyboard access | M1 | Native verified | 0.79 Formulas tab ([WATCH_WINDOW](../WATCH_WINDOW.md)) | Full Office group layout, KeyTips and ribbon collapse |
| D01.03 | Contextual tabs that appear for the selected object (table, picture, chart, header/footer) | M2 | Specified | — | Not built |
| D01.04 | Command search ("Tell me") that runs any ribbon command by name | M2 | Specified | — | Not built |
| D01.05 | Document status bar: page, word count including tables, language, view and zoom controls | M1 | Automated verified | 0.15 word count includes table text | Page/language/zoom items; native check |
| D01.06 | Workbook status bar: Sum, Average, Count and other selection aggregates | M1 | Automated verified | Workbook essentials Task 2 ([plan](../../../docs/superpowers/plans/2026-09-25-m1-workbook-editing-essentials.md)) | Persist the chosen aggregates; which hidden cells Excel counts; native check |
| D01.07 | Several documents and workbooks open at once, as tabs or windows, with independent save state | M2 | Automated verified | Open files [plan](../../../docs/superpowers/plans/2026-09-26-open-files.md): each tab has its own engine session, recovery file and save prompts | Separate windows (D01.08); native check of the tab strip and menus |
| D01.08 | Separate windows for one file (New Window, Arrange All) and task switching | M4 | Specified | — | Not built |
| D01.09 | Window size, position and open files restored after restart | M2 | Native verified | Open files [plan](../../../docs/superpowers/plans/2026-09-26-open-files.md): the open tabs and the tab in front come back. Window frame native check 2026-10-30 (macOS 27, two displays): moved, quit and relaunched, the window reopens at the same position and size on the built-in and the external display; `scripts/test_macos_lifecycle.py` checks the restore on each connected screen | A window on a display that has been removed reopens where AppKit places it |
| D01.10 | Native macOS menu bar (File, Edit, View, Window) sharing commands with the ribbon | M1 | Native verified | Build 11, 0.64 ([FILE_COMMANDS](../FILE_COMMANDS.md), [HISTORY_MENUS](../HISTORY_MENUS.md)) | Windows/Linux menus |
| D01.11 | Context menus on text, cells, rows/columns and objects with Office-grouped commands | M1 | Native verified | 0.29/0.30 row/column context menus ([VALIDATION](../../VALIDATION.md)) | Object and text context menus beyond spelling; grouping audit |
| D01.12 | Quick Access Toolbar with user customization | M3 | Specified | — | Not built |
| D01.13 | Customize ribbon and keyboard shortcuts | M4 | Specified | — | Not built |

## D02 Files: new, open, save, recent, templates, autosave and recovery

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| D02.01 | New blank document/workbook, keeping the outgoing workspace in recovery | M1 | Native verified | Build 11 ([FILE_COMMANDS](../FILE_COMMANDS.md)) | Office comparison of defaults |
| D02.02 | Open DOCX, XLSX and `.folio` from the native panel; cancel and failure keep the current workspace | M1 | Native verified | Build 11 ([FILE_COMMANDS](../FILE_COMMANDS.md)) | Open-time budget (ROADMAP §7) |
| D02.03 | Save to the working DOCX/XLSX file in place, with byte checks and atomic replacement | M1 | Automated verified | 0.54 ([WORKING_FILES](../WORKING_FILES.md)) | Native sandbox Save and post-restart access |
| D02.04 | Save As to a new location or format, keeping the original and retargeting after the write | M1 | Automated verified | 0.54 ([WORKING_FILES](../WORKING_FILES.md)) | Native qualification; 4 Save/Save As Flutter tests fail on Linux (Phase 2 baseline) |
| D02.05 | Unsaved-change prompts on New, Open and Close (Save / Don't Save / Cancel), including first save of unnamed content | M1 | Automated verified | 0.54 builds 2–3 ([WORKING_FILES](../WORKING_FILES.md)) | Native qualification |
| D02.06 | External change detection: a file changed on disk is not overwritten silently | M1 | Automated verified | 0.54 ([WORKING_FILES](../WORKING_FILES.md)) | Native qualification; "external writes" Flutter test fails on Linux |
| D02.07 | Export Word/Excel and Save Workspace Copy as separate commands that never retarget Save | M1 | Native verified | Build 11 ([FILE_COMMANDS](../FILE_COMMANDS.md)) | — (Office compare via D02.03) |
| D02.08 | Recent files with search, type filters, pinning, removal and Locate for moved files | M1 | Automated verified | 0.55 ([RECENT_FILES](../RECENT_FILES.md)) | Native qualification |
| D02.09 | Recovery browser: preview and restore earlier workspace copies without overwriting them | M1 | Automated verified | 0.55 ([RECENT_FILES](../RECENT_FILES.md)) | Native qualification |
| D02.10 | Continuous local autosave with crash recovery of committed edits | M1 | Native verified | Restart checks across 0.4x–0.82 ([VALIDATION](../../VALIDATION.md)) | Controlled-crash budget (ROADMAP §7) |
| D02.11 | New from Template gallery with bundled starters and a personal DOTX/XLTX library | M2 | Automated verified | 0.56 ([TEMPLATES](../TEMPLATES.md)) | Native qualification |
| D02.12 | Save as Template (DOTX/XLTX); opening a template never overwrites it | M2 | Automated verified | 0.56 ([TEMPLATES](../TEMPLATES.md)) | Native qualification; Office compare |
| D02.13 | Default/global templates (Normal.dotm, Book.xltx) and macro-enabled templates | M4 | Specified | — | Not built |
| D02.14 | Version history: browse and restore earlier saved versions of a file | M4 | Specified | — | Timed/cloud history not built |
| D02.15 | Finder file associations and Open With; double-clicking a file opens it | M2 | Specified | — | Not built |
| D02.16 | File properties (title, author, tags) viewable and editable | M2 | Specified | — | Not built |
| D02.17 | Compatibility report before saving content the target format can't hold | M1 | Automated verified | Build 11 menu item ([FILE_COMMANDS](../FILE_COMMANDS.md)); rank 10 impact preview: signatures removed from a changed signed file, formatting lost in edited paragraphs, with Export Anyway, Save Folio Copy and Cancel (`compatibility_report_workspace_test.dart`) | Native check of the impact preview |

## D03 Clipboard, drag and drop, undo/redo and shortcuts

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| D03.01 | One workspace Undo/Redo across document and workbook edits, from toolbar, keyboard and native menu | M1 | Native verified | 0.53–0.64 ([HISTORY_MENUS](../HISTORY_MENUS.md)) | Office comparison of step granularity |
| D03.02 | Field-level draft history in dialogs, formula bar, search and assistant fields that doesn't leak into the document | M1 | Native verified | 0.17.2–0.17.3, 0.64 ([HISTORY_MENUS](../HISTORY_MENUS.md)) | Document-conversion post-Save case |
| D03.03 | Typing stays ordered during asynchronous saves and history (no lost or reordered keystrokes) | M1 | Native verified | 0.42–0.43, 0.53 build 6 ([DOCUMENT_HISTORY_INPUT](../DOCUMENT_HISTORY_INPUT.md)) | Clipboard/command shortcuts during queued entry |
| D03.04 | Cut, copy and paste inside Folio keeping rich document content | M1 | Automated verified | 0.47 rich/internal paste ([BOOKMARKS](../BOOKMARKS.md)) | Native check; cut semantics |
| D03.05 | Paste from and copy to Office and other apps: HTML/RTF for documents, HTML/TSV for cells | M1 | Specified | — | Only plain text and TSV interchange today |
| D03.06 | Drag and drop of text, cells and files | M2 | Specified | — | Not built |
| D03.07 | Office keyboard shortcuts for formatting, navigation and editing in both editors | M1 | Native verified | 0.11.1, 0.31, 0.42 subsets ([VALIDATION](../../VALIDATION.md)) | Full shortcut inventory against Office; Windows Ctrl mapping |
| D03.08 | Input methods: IME composition, dead keys and international keyboards in both editors | M1 | Automated verified | Composition guards 0.17.3, 0.31 | Native IME and dead-key checks |
| D03.09 | Repeat last action (F4 / Command-Y) | M2 | Specified | — | Not built |
| D03.10 | Format Painter in documents and workbooks | M1 | Automated verified | Documents: character and paragraph formatting painted onto the next selection or the word clicked, one Undo step (`format_painter.dart`); workbooks: see X04.12; a double-click keeps it on until Esc | Drag-selecting a target range in the grid; native check |

## D04 Accessibility, zoom and localization

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| D04.01 | Every command reachable by keyboard with visible focus | M1 | Specified | — | No audit; partial evidence in D03.07 |
| D04.02 | Cell and range selection announced to screen readers; custom cell actions | M1 | Automated verified | 0.76–0.77 ([CELL_SELECTION_ACCESSIBILITY](../CELL_SELECTION_ACCESSIBILITY.md), [CELL_ACTIONS](../CELL_ACTIONS.md)) | Native custom-action delivery; VoiceOver task test |
| D04.03 | VoiceOver task tests for the daily workflow in both editors | M1 | Specified | — | No formal audit |
| D04.04 | NVDA task tests on Windows | M5 | Specified | — | Needs Windows build |
| D04.05 | High contrast and increased-contrast modes | M2 | Specified | — | Not built |
| D04.06 | UI scale to 200% without clipping; document and sheet zoom | M2 | Specified | — | Page preview zoom only (0.68) |
| D04.07 | Right-to-left text entry and layout | M2 | Automated verified | 0.69 RTL columns ([COLUMNS](../COLUMNS.md)) | RTL editing, bidi runs, RTL sheets |
| D04.08 | CJK input and rendering | M2 | Specified | — | No evidence |
| D04.09 | Locale-aware dates, numbers and list separators in input and display | M2 | Specified | — | en-US assumptions; see X04, X08 |
| D04.10 | Translated UI | M5 | Specified | — | English only |
| D04.11 | Linux assistive technology audit | M5 | Specified | — | Needs Linux build |

## D05 Printing, page setup and PDF

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| D05.01 | Document print preview with page flow, section geometry, headers/footers and page fields | M2 | Native verified | 0.68 ([PAGINATION](../PAGINATION.md)) | Word layout comparison (W05) |
| D05.02 | Print a document through the system print dialog: printer, copies, page ranges | M2 | Specified | — | No printing |
| D05.03 | Workbook print preview and printing with print areas, titles and scaling | M3 | Specified | — | See X18 |
| D05.04 | Export to PDF matching print preview | M2 | Automated verified | Rank 15 ([plan](../../../docs/superpowers/plans/2026-09-26-pdf-export.md)): File → Export PDF… writes the preview's pages with text as text, embedded font subsets and pictures (`pdf_export_tests.rs`, `document_pdf_workspace_test.dart`); tab leaders are written as text, and a right tab's field ends at a line break, so table-of-contents page numbers at every level end at the stop | Native check; CFF-outline fonts are substituted; complex scripts (Arabic, Indic) are not shaped |
| D05.05 | Tagged, accessible PDF: reading order, headings, table headers, links, alt text | M2 | Automated verified | Rank 15 ([plan](../../../docs/superpowers/plans/2026-09-26-pdf-export.md)): tags headings, paragraphs and figures with alt text; links, outline, language and title; headers and footers are artifacts | PDF/UA validation; tables and lists (D05.07) |
| D05.06 | Printed output matches the page model: nothing clipped or omitted | M2 | Specified | — | Needs D05.02 |
| D05.07 | Tagged tables (TR/TD with headers) and lists (L/LI) in exported PDF | M2 | Specified | — | Tables and lists are tagged as paragraphs today |

## D06 Security, protection, encryption and trusted automation

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| D06.01 | Encrypted (password-to-open) DOCX/XLSX: open with a password prompt, save encrypted | M4 | Specified | — | Rejected today ("not an unencrypted package") |
| D06.02 | Digital signatures: show status; saving warns that it invalidates signatures | M4 | Specified | — | Not built |
| D06.03 | Macro-bearing files open with macros disabled and a visible notice; saving never drops macros silently | M4 | Specified | — | Macro templates rejected (0.56) |
| D06.04 | Trust Center: trusted locations, external content and link prompts | M4 | Specified | — | Not built |
| D06.05 | macOS sandbox with user-granted file access only; file contents can't grant access | M1 | Automated verified | 0.54–0.55 ([WORKING_FILES](../WORKING_FILES.md)) | Native sandbox qualification |
| D06.06 | Robust handling of hostile files: errors instead of crashes, hangs or runaway memory | M1 | Automated verified | Phase 3 ([plan](../../../docs/superpowers/plans/2026-09-24-phase-3-file-input-robustness.md)) | Fuzz in CI; Office-produced corpus |
| D06.07 | Protection semantics consistent across UI, paste, automation and AI (see W15, X15) | M4 | Specified | — | Depends on W15/X15 |

## D07 Collaboration, sharing and conflict recovery

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| D07.01 | Author identity for comments and revisions, set once and shown consistently | M2 | Automated verified | 0.57–0.60 ([COMMENTS](../COMMENTS.md), [TRACK_CHANGES_AUTHORING](../TRACK_CHANGES_AUTHORING.md)) | Native qualification |
| D07.02 | Share a file (share sheet, email a copy) | M4 | Specified | — | Not built |
| D07.03 | Co-authoring on cloud files (OneDrive/SharePoint) with presence | M4 | Specified | — | Needs architecture decision (see STATUS risks) |
| D07.04 | Offline edits merged on reconnect with visible conflict resolution | M4 | Specified | — | Not built |
| D07.05 | Concurrent or failed saves never lose work silently | M1 | Automated verified | 0.54 external-change guards ([WORKING_FILES](../WORKING_FILES.md)) | Native qualification; multi-process case |

## D08 Packaging, updates and distribution

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| D08.01 | Signed Apple Silicon macOS build that runs as a single preview instance | M1 | Native verified | Preview hygiene ([PREVIEW_WORKFLOW](../PREVIEW_WORKFLOW.md)) | Developer-ID signing and notarization |
| D08.02 | Notarized macOS distribution with install, update, rollback and uninstall | M4 | Specified | — | Not built |
| D08.03 | Intel macOS build | M4 | Specified | — | Not built |
| D08.04 | Windows build and installer | M4 | Specified | — | Platform decision (program plan dependency 2) |
| D08.05 | Linux build and packaging | M5 | Specified | — | Not built |
| D08.06 | CI on every push: Rust and Flutter checks | M1 | Automated verified | Phase 1 (`.github/workflows/ci.yml`) | GitHub runners not starting on this account since Phase 2; runs locally |
| D08.07 | Migration backups when the workspace format changes | M2 | Implemented | Legacy recovery hydration (0.55, 0.79) | Explicit migration tests and backup copy |

## D09 Performance, resources, lifecycle and cancellation

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| D09.01 | Single-cell edit, engine plus response, p95 ≤ 50 ms on the 100k-cell workload | M1 | Automated verified | Phase 2 ([PERFORMANCE_BASELINE](../PERFORMANCE_BASELINE.md)); container p95 113 ms | Measure on the M4 Max reference machine |
| D09.02 | Cold launch to editable workspace p95 ≤ 2 s | M1 | Specified | — | Unmeasured |
| D09.03 | Key-to-visible typing p95 ≤ 50 ms; selection and menus ≤ 100 ms | M1 | Specified | — | Unmeasured end to end |
| D09.04 | Scroll p95 frame ≤ 16.7 ms while editing | M2 | Specified | — | Unmeasured |
| D09.05 | Open budgets: 50-page DOCX ≤ 3 s, 20-sheet 100k-cell XLSX ≤ 5 s | M2 | Specified | — | Engine load measured (Phase 1); app-level open unmeasured |
| D09.06 | Save p95 ≤ 2 s with a responsive UI | M2 | Specified | — | Unmeasured |
| D09.07 | Memory: idle RSS ≤ 350 MiB; corpus peak ≤ 1 GiB; no growth across open/close cycles | M2 | Native verified | 0.13 sparse layout, 331 MiB single sample ([VALIDATION](../../VALIDATION.md#013-sparse-worksheet-viewport-2026-09-18)) | Repeatable measurement; peak and cycle growth |
| D09.08 | Cancellation acknowledged ≤ 250 ms for long local jobs | M3 | Automated verified | Goal Seek Stop/Cancel 0.80 ([GOAL_SEEK](../GOAL_SEEK.md)) | General long-operation cancellation (recalc, open, save) |
| D09.09 | Foreground and display changes recover rendering | M1 | Native verified | 0.16.1 ([VALIDATION](../../VALIDATION.md)) | Physical sleep/wake, lock, display disconnect, minimize/restore |
| D09.10 | Resource bounds on hostile input: time, memory, recursion | M1 | Automated verified | Phase 3 fuzzing and formula read budget | Office-created large corpus |

## D10 Extensibility, automation and assistant integration

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| D10.01 | Codex App Server sign-in, streaming, cancel and reconnect for the assistant | M1 | Automated verified | [VALIDATION](../../VALIDATION.md#live-codex-app-server) | Qualify auth/stream/cancel/timeout separately (ROADMAP §6.8) |
| D10.02 | Assistant proposals applied through typed commands and normal history, with one-step Undo | M1 | Automated verified | Assistant tests in disposable workspaces (PARITY AI section) | Real-model runs; native Apply/Undo |
| D10.03 | Native A2UI forms with allowlisted controls, keyboard access and deterministic actions | M1 | Automated verified | PARITY AI section | Accessibility and real-model qualification |
| D10.04 | Selected-context chips: passage, cell/range, attached workbook, with revision and scope | M1 | Automated verified | PARITY AI section | Chart/table/multi-source context |
| D10.05 | Stale-context conflict preview when the source changes during a request | M1 | Specified | — | ROADMAP §6.6 |
| D10.06 | Office add-in (Office.js) compatibility | M4 | Specified | — | Architecture decision (STATUS risks) |
| D10.07 | Folio's own extension/automation interface, documented and tested | M4 | Specified | — | Not designed |
| D10.08 | macOS Services, AppleScript/Shortcuts automation | M5 | Specified | — | Not built |
