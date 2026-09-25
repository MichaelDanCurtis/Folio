# Excel requirements (X01–X19)

States and ID rules: [README](README.md). Family scope: [PARITY](../../PARITY.md#excel-requirements) and ROADMAP §4 X-A…X-J.

## X01 Selection, editing, clipboard and fill

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X01.01 | Select a cell or rectangular range by mouse, keyboard and F8 extend mode | M1 | Native verified | 0.11.1, 0.76–0.77 ([CELL_ACTIONS](../CELL_ACTIONS.md)) | Excel comparison |
| X01.02 | Multi-area selection (Command/Ctrl-click) used by formatting, copy and delete | M2 | Specified | — | Not built |
| X01.03 | Select whole rows, columns and the sheet; Ctrl+arrow and Ctrl+Shift+arrow to data edges | M1 | Automated verified | Workbook essentials Task 1 ([plan](../../../docs/superpowers/plans/2026-09-25-m1-workbook-editing-essentials.md)) | Native check; Excel comparison. Whole-row/column formatting is still limited to 2,000 cells |
| X01.04 | Type into a cell, commit with Enter/Tab/Shift variants, cancel with Escape, and keep fast first keys | M1 | Native verified | 0.31, 0.42–0.43 ([VALIDATION](../../VALIDATION.md#0310-multiline-cell-and-formula-editing-2026-09-19)) | IME and dead keys |
| X01.05 | Formula bar editing, expandable, with multiline text (Option/Alt+Enter) | M1 | Native verified | 0.31 | Excel comparison |
| X01.06 | In-cell edit mode (F2) with caret movement and reference pointing by arrows | M1 | Specified | — | F2 exists; point mode not built |
| X01.07 | Copy and paste inside Folio: values, formulas with reference adjustment, formats, transpose, skip blanks | M1 | Native verified | 0.33 ([VALIDATION](../../VALIDATION.md#033-clipboard-qualification--2026-09-19)) | Native skip-blanks check |
| X01.08 | Paste Special property modes: number formats, validation, column widths | M2 | Automated verified | 0.51 ([PASTE_PROPERTIES](../PASTE_PROPERTIES.md)) | Native check |
| X01.09 | Paste Special arithmetic (add, subtract, multiply, divide) and paste link | M2 | Specified | — | Not built |
| X01.10 | Cut and paste (move) with reference updates | M1 | Specified | — | Cut/move not built |
| X01.11 | Clipboard exchange with Excel and other apps (HTML, TSV, RTF) | M1 | Specified | — | TSV only (see D03.05) |
| X01.12 | Fill Down/Right/Up/Left (Ctrl+D, Ctrl+R) with relative formula adjustment | M1 | Automated verified | 0.22 ([VALIDATION](../../VALIDATION.md#0220-worksheet-directional-fill-2026-09-19)) | Native package check |
| X01.13 | Fill handle drag and double-click to extend | M1 | Specified | — | Not built |
| X01.14 | Series fill (linear, growth, dates) and custom lists | M2 | Specified | — | Not built |
| X01.15 | Flash Fill equivalent | M3 | Specified | — | Not built |
| X01.16 | Conditional formatting copied by paste and fill | M2 | Automated verified | 0.50 ([CONDITIONAL_COPY](../CONDITIONAL_COPY.md)) | Native check |
| X01.17 | Clear menu: formats, comments, hyperlinks or all (Delete already clears contents) | M1 | Specified | — | Not built |
| X01.18 | Delete/Backspace clears the selected cells' contents in one Undo step; on whole rows or columns, only the populated cells | M1 | Automated verified | Workbook essentials Task 1 | Native check |

## X02 Find, replace, Go To and navigation

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X02.01 | Find and replace in sheet or workbook, by rows or columns, in formulas or values, match case and entire cell | M1 | Native verified | 0.5 ([VALIDATION](../../VALIDATION.md#05-navigation-and-replacement)) | Excel comparison |
| X02.02 | Wildcards (* ? ~) in find | M2 | Specified | — | Not built |
| X02.03 | Find by format; search comments and notes | M3 | Specified | — | Not built |
| X02.04 | Name box: shows the selection; jumps to a typed cell, range, whole rows/columns, a reference on another sheet, or a defined name | M1 | Automated verified | Workbook essentials Task 3 ([plan](../../../docs/superpowers/plans/2026-09-25-m1-workbook-editing-essentials.md)) | Show the active cell (Excel) rather than the range; native check |
| X02.05 | Go To (F5, Ctrl+G) with defined names, recent targets and a reference | M1 | Automated verified | Workbook essentials Task 3 | Special… button (X02.06); native check |
| X02.06 | Go To Special: blanks, constants, formulas, visible cells, differences and so on | M2 | Specified | — | Not built |
| X02.07 | Sheet navigation: tabs, Ctrl+PgUp/PgDn, sheet list | M1 | Automated verified | 0.6 sheet tabs | Keyboard sheet switching; native check |
| X02.08 | Define a name by typing it into the name box | M2 | Specified | — | Needs the Name Manager (X09.05) |

## X03 Rows, columns and sheets

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X03.01 | Insert and delete rows and columns, transforming formulas, names, styles, charts, comments and ranges | M1 | Automated verified | 0.6 ([VALIDATION](../../VALIDATION.md#06-worksheet-structure-and-sheet-management)) | Native check; Excel comparison |
| X03.02 | Row heights and column widths: numeric entry, dragging and hiding/unhiding | M1 | Native verified | 0.11 ([VALIDATION](../../VALIDATION.md#011-rowcolumn-dimensions-and-manual-visibility-2026-09-18)) | Excel comparison |
| X03.03 | AutoFit row height and column width (menu and boundary double-click) | M1 | Native verified | 0.29–0.30 ([VALIDATION](../../VALIDATION.md#0300-native-autofit-qualification-2026-09-19)) | Excel comparison |
| X03.04 | Automatic row height after edits, keeping manual heights | M1 | Native verified | 0.32 ([VALIDATION](../../VALIDATION.md#0320-automatic-row-sizing-2026-09-19)) | Excel comparison |
| X03.05 | Rename, reorder and delete sheets | M1 | Automated verified | 0.6 | Native check |
| X03.06 | Duplicate a sheet; move or copy a sheet to another workbook | M1 | Specified | — | Not built |
| X03.07 | Hide and unhide sheets; tab colours | M2 | Specified | — | Not built |
| X03.08 | Move rows and columns by drag or cut/insert | M2 | Specified | — | Not built |
| X03.09 | Group and outline rows and columns (collapse/expand) | M3 | Specified | — | Not built |
| X03.10 | Structural edits update pivots, external links and drawings | M3 | Specified | — | Depends on X11, X12 |
| X03.11 | Add a sheet from the sheet tabs | M1 | Implemented | `addSheet` op and tab button | Dedicated test; insert position and naming vs Excel |

## X04 Cell formatting and styles

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X04.01 | Font dialog: family, size, colour, effects, underline styles, super/subscript, with preview | M1 | Automated verified | 0.24 ([VALIDATION](../../VALIDATION.md#0240-worksheet-font-controls-and-rendering-2026-09-19)) | Native package check |
| X04.02 | Rich text runs within a cell: display and per-character formatting | M2 | Automated verified | 0.25–0.26 | In-cell rich editing; native check |
| X04.03 | Alignment dialog: horizontal, vertical, wrap, shrink, indent, rotation, reading order | M1 | Automated verified | 0.28 ([VALIDATION](../../VALIDATION.md#0280-worksheet-alignment-and-text-layout-2026-09-19)) | Native check |
| X04.04 | Number formats: presets and custom codes with preview, applied through shared display | M1 | Automated verified | 0.34 ([VALIDATION](../../VALIDATION.md#034-number-format-qualification--2026-09-19)) | Native check; locale formats |
| X04.05 | Width-dependent numeric display (#### and precision reduction) | M1 | Native verified | 0.45 ([VALIDATION](../../VALIDATION.md#045-numeric-column-width-display--2026-09-19)) | Exact Excel thresholds |
| X04.06 | Text overflow into empty neighbouring cells | M1 | Native verified | 0.44 ([VALIDATION](../../VALIDATION.md#044-text-overflow--2026-09-19)) | Colliding labels; table areas |
| X04.07 | Borders: styles, colours, presets and draw-border | M1 | Specified | — | Not built |
| X04.08 | Fill colours and patterns | M1 | Specified | — | Not qualified as an editing command |
| X04.09 | Merge and unmerge cells (Merge & Center, Across) with rendering and editing | M2 | Specified | — | Imported merges shown read-only (0.44) |
| X04.10 | Cell styles gallery and custom named styles | M2 | Specified | — | Not built |
| X04.11 | Theme colours and fonts; indexed colours | M2 | Specified | — | Not built |
| X04.12 | Format Painter (see D03.10) | M1 | Specified | — | Not built |
| X04.13 | Automatic column widening for numbers entered in General columns | M2 | Specified | — | Not built |

## X05 Views, panes and zoom

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X05.01 | Freeze panes, top row and first column; unfreeze | M1 | Native verified | 0.12 ([VALIDATION](../../VALIDATION.md#012-frozen-worksheet-panes-2026-09-18)) | Horizontal wheel input |
| X05.02 | Split panes, draggable | M3 | Specified | — | Not built |
| X05.03 | Zoom: percentage, zoom to selection | M1 | Specified | — | Not built |
| X05.04 | Page Layout view and Page Break Preview | M3 | Specified | — | See X18 |
| X05.05 | Show/hide gridlines, headings and formula bar | M2 | Specified | — | Not built |
| X05.06 | Custom views and New Window for the same workbook | M4 | Specified | — | Not built |

## X06 Sort, filter and tables

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X06.01 | Sort a range by several keys, with headers, excluding hidden rows | M1 | Native verified | 0.9–0.10 ([VALIDATION](../../VALIDATION.md#09-worksheet-range-sorting-and-custom-totals)) | Sort by colour/icon; left to right; custom lists |
| X06.02 | AutoFilter: value lists and custom conditions, reapply, clear, remove | M1 | Native verified | 0.10 ([VALIDATION](../../VALIDATION.md#010-worksheet-autofilter-and-explicit-filter-visibility-2026-09-18)) | Structural filter workflows (native) |
| X06.03 | Tables: create, name, style, header and totals rows | M1 | Native verified | 0.7–0.10 ([VALIDATION](../../VALIDATION.md#07-tables-sorting-and-filtering)) | Full design options |
| X06.04 | Structured references and calculated columns | M1 | Automated verified | 0.7 | Horizontal fill; Excel comparison |
| X06.05 | Resize a table, with totals relocation | M2 | Native verified | 0.8–0.10 ([VALIDATION](../../VALIDATION.md#08-table-resizing)) | Occupied totals destination |
| X06.06 | Convert a table to a range | M2 | Specified | — | Not built |
| X06.07 | Advanced Filter with a criteria range and copy-to | M3 | Specified | — | Not built |
| X06.08 | Slicers for tables | M3 | Specified | — | Not built |
| X06.09 | Filter by colour, and date grouping in filter lists | M2 | Specified | — | Not built |
| X06.10 | Remove Duplicates, Text to Columns | M2 | Specified | — | Not built |

## X07 Data validation and conditional formatting

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X07.01 | Data validation: all standard types and operators, input messages, stop/warning/information alerts, in-cell lists | M1 | Native verified | 0.39 ([VALIDATION](../../VALIDATION.md#039-data-validation-qualification--2026-09-19)) | Named ranges; Excel comparison |
| X07.02 | Circle invalid data; select cells with the same validation | M2 | Specified | — | Not built |
| X07.03 | Conditional formatting: value, formula, text, blank/error, duplicate/unique, top/bottom, above/below average | M1 | Native verified | 0.40 ([VALIDATION](../../VALIDATION.md#040-conditional-formatting-qualification--2026-09-19)) | Excel visual comparison |
| X07.04 | Colour scales | M1 | Native verified | 0.40 | Interpolation compared with Excel |
| X07.05 | Data bars, including negative axis and bar only | M2 | Native verified | 0.41 ([VALIDATION](../../VALIDATION.md#041-data-bar-qualification--2026-09-19)) | Excel geometry comparison |
| X07.06 | Icon sets: all 20 styles, custom mixtures, thresholds | M2 | Automated verified | 0.48 ([ICON_SETS](../ICON_SETS.md)) | Native check |
| X07.07 | Date-occurring rules | M2 | Automated verified | 0.49 ([DATE_RULES](../DATE_RULES.md)) | Native check |
| X07.08 | Rules manager: priority, Stop if true, edit and delete | M1 | Native verified | 0.40 | Excel comparison |
| X07.09 | Differential formats: borders, theme/indexed colours, number formats in rules | M2 | Specified | — | Advanced dxf missing |
| X07.10 | Extended (x14) validation and rule editing | M3 | Specified | — | Preserved only |

## X08 Functions and calculation semantics

Coverage of Excel's catalog of 514 worksheet functions, measured by `formula_catalog_tests.rs` ([excel-functions.tsv](excel-functions.tsv)): **483 evaluate, 4 are recognized but not implemented (FORECAST.ETS family), and 27 are unknown to the engine.** "Evaluate" means the engine computes a result. It does not mean the results match Excel; that needs X08.02.

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X08.01 | The 483 catalog functions the engine evaluates | M3 | Automated verified | IronCalc suite (2,228 tests); catalog test | Differential corpus against Excel (X08.02) |
| X08.02 | Differential function corpus: recorded Excel results for every function, including edge cases | M3 | Specified | — | Needs the Office baseline |
| X08.03 | Missing functions: AGGREGATE, BAHTTEXT, DBCS, JIS, PHONETIC, HYPERLINK, IMAGE, GROUPBY, PIVOTBY, GETPIVOTDATA, RTD, FORECAST.ETS family | M3 | Specified | [excel-functions.tsv](excel-functions.tsv) | Implement; GETPIVOTDATA needs X12 |
| X08.04 | Service functions: WEBSERVICE, FILTERXML, ENCODEURL, STOCKHISTORY, TRANSLATE, DETECTLANGUAGE, CUBE* (7), CALL, REGISTER.ID, EUROCONVERT | M4 | Specified | [excel-functions.tsv](excel-functions.tsv) | Need network/data-model decisions (X13) |
| X08.05 | Error values: the seven classic errors typed through import, calculation, display, clipboard and export | M1 | Native verified | 0.37 ([VALIDATION](../../VALIDATION.md#037-typed-spreadsheet-error-qualification--2026-09-19)) | Newer errors (#SPILL!, #CALC!) in files |
| X08.06 | 1900 and 1904 date systems through input, calculation and export | M2 | Automated verified | 0.36 ([VALIDATION](../../VALIDATION.md#036-workbook-calendar-and-typed-export-qualification--2026-09-19)) | Native check |
| X08.07 | Early-1900 dates, serial zero and the 1900 leap-year bug | M2 | Native verified | 0.38 ([VALIDATION](../../VALIDATION.md#038-early-date-qualification--2026-09-19)) | Financial and week-number edge cases |
| X08.08 | Coercion and precision semantics (text to number, 15-digit precision, booleans) | M3 | Specified | — | Needs the differential corpus |
| X08.09 | Volatile functions (TODAY, NOW, RAND, OFFSET, INDIRECT) recalculated when Excel would | M2 | Automated verified | 0.49 calculation clock ([DATE_RULES](../DATE_RULES.md)) | RAND/OFFSET/INDIRECT scheduling |
| X08.10 | Calculation modes: automatic, automatic except tables, manual, F9 | M2 | Automated verified | 0.49 explicit F9 | Mode settings UI and persistence |
| X08.11 | Circular references: warning, and iterative calculation settings | M3 | Specified | — | #CIRC shown; no iteration settings |
| X08.12 | Locale-specific formula syntax (argument separators, decimal comma) | M4 | Specified | — | en-US only |
| X08.13 | Formulas over huge ranges stay bounded (read budget, clipping) | M1 | Automated verified | Phase 3 ([formula limits](../../rust/src/formula_limits_tests.rs)) | Iterate only populated cells |
| X08.14 | Legacy formulas passing a range to a scalar function use implicit intersection, as Excel does for pre-dynamic-array files | M1 | Automated verified | Imported formulas get Excel 365's automatic `@` ([legacy formula tests](../../rust/src/legacy_formula_tests.rs)); corpus rerun | Excel comparison of the corpus cells still differing |
| X08.15 | 3-D references (`Sheet1:Sheet3!A1`) in SUM, MIN, MAX and the other aggregates | M2 | Specified | [CORPUS](CORPUS.md#office-created-corpus-real-files) | 13 corpus cells give #VALUE! |
| X08.16 | Whole-column lookups (VLOOKUP, HLOOKUP, MATCH, LOOKUP) read only populated rows, and sorted searches end at the last value not greater than the target | M1 | Automated verified | Corpus finding; [formula limits](../../rust/src/formula_limits_tests.rs) | Excel comparison of blank-cell edge cases |

## X09 Arrays, spill, names and external references

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X09.01 | Shared formulas expanded on import and materialized on edit | M1 | Automated verified | 0.6 | Excel round trip |
| X09.02 | Dynamic arrays spill into neighbouring cells with #SPILL! on collision, and render in the grid | M3 | Specified | — | Engine spills; grid spill display not integrated |
| X09.03 | Legacy CSE array formulas: enter, edit and keep | M3 | Specified | — | Not qualified |
| X09.04 | LET and LAMBDA, including named LAMBDAs | M3 | Specified | — | Engine support; no name manager |
| X09.05 | Defined names: Name Manager to create, edit and delete; names used in formulas | M2 | Specified | — | Package names kept; no Name Manager |
| X09.06 | Names follow structural edits and sheet renames | M2 | Automated verified | 0.6 | Named-formula evaluation parity |
| X09.07 | External workbook references: display cached values; update links | M4 | Specified | — | Not built |
| X09.08 | Defined names evaluate in formulas (workbook and sheet scope, references, constants and formulas) | M1 | Automated verified | [names tests](../../rust/src/xlsx_names_tests.rs); corpus rerun | Name Manager (X09.05); names in exported new workbooks |

## X10 Auditing

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X10.01 | Formula inspector: direct precedents and dependents across sheets, with navigation history | M3 | Native verified | 0.78–0.79 ([FORMULA_AUDITING](../FORMULA_AUDITING.md)) | Named/dynamic/external references |
| X10.02 | Trace precedents/dependents arrows on the grid | M3 | Specified | — | Not built |
| X10.03 | Evaluate Formula step by step | M3 | Specified | — | Not built |
| X10.04 | Watch Window: add, remove, live values, navigation, persistence | M3 | Native verified | 0.79 ([WATCH_WINDOW](../WATCH_WINDOW.md)) | Floating/other-edge docking; names |
| X10.05 | Error checking with rules and trace error | M3 | Specified | — | Not built |
| X10.06 | Show formulas (Ctrl+`) | M2 | Specified | — | Not built |

## X11 Charts, sparklines and drawings

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X11.01 | Imported Office charts rendered faithfully | M3 | Specified | — | Simple local preview only |
| X11.02 | Insert column, bar, line, pie, area, scatter and combo charts from a selection | M3 | Implemented | Simple local chart preview; formatted labels 0.35 | Office chart output; tests |
| X11.03 | Chart editing: type, data range, titles, axes, legend, labels, styles | M3 | Specified | — | Not built |
| X11.04 | Charts saved as editable Office charts | M3 | Specified | — | Not built |
| X11.05 | Specialist charts: histogram, box and whisker, waterfall, treemap, sunburst, funnel, maps, stock | M4 | Specified | — | Not built |
| X11.06 | Sparklines | M3 | Specified | — | Not built |
| X11.07 | Pictures, shapes and text boxes on sheets | M4 | Specified | — | Preserved only |
| X11.08 | Drawings keep their anchoring through structural edits | M3 | Automated verified | 0.6 tested OOXML charts in structural edits | Rendering (X11.01) |

## X12 PivotTables

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X12.01 | Imported PivotTables display their cached values and stay intact | M3 | Specified | — | Not qualified |
| X12.02 | Create a PivotTable: rows, columns, values, filters | M3 | Specified | — | Not built |
| X12.03 | Refresh; value summaries and Show Values As | M3 | Specified | — | Not built |
| X12.04 | Grouping (dates, numbers), calculated fields and items | M3 | Specified | — | Not built |
| X12.05 | PivotCharts and slicers | M4 | Specified | — | Not built |
| X12.06 | Data model pivots | M4 | Specified | — | See X13 |

## X13 Data import, Power Query and Power Pivot

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X13.01 | Import CSV/text with encoding, delimiter and locale choices | M2 | Specified | — | CSV export only (X17.07) |
| X13.02 | Get data from web, databases and files, with refresh and credentials | M4 | Specified | — | Architecture decision (STATUS risks) |
| X13.03 | Power Query transformations, preserved and refreshable | M4 | Specified | — | Not built |
| X13.04 | Data model relationships and DAX measures | M5 | Specified | — | Not built |
| X13.05 | Stock and geography data types | M5 | Specified | — | Not built |

## X14 What-if analysis

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X14.01 | Goal Seek on one input across sheets, with Stop/Cancel and reviewed Apply | M3 | Native verified | 0.80 ([GOAL_SEEK](../GOAL_SEEK.md)) | Excel numerical comparison |
| X14.02 | Scenario Manager: add, edit, delete, show, summary | M3 | Native verified | 0.81 ([SCENARIOS](../SCENARIOS.md)) | PivotTable report; protection |
| X14.03 | Scenario Merge from this and other workbooks | M3 | Native verified | 0.82 ([SCENARIOS](../SCENARIOS.md)) | Excel comparison |
| X14.04 | One- and two-variable data tables (TABLE()) | M3 | Specified | — | Not built |
| X14.05 | Solver: linear, nonlinear and integer models | M3 | Specified | — | Not built |
| X14.06 | Analysis ToolPak tools | M4 | Specified | — | Not built |
| X14.07 | Forecast Sheet | M4 | Specified | — | Needs FORECAST.ETS (X08.03) |

## X15 Notes, comments, protection and sharing

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X15.01 | Notes and threaded comments: add, reply, resolve, navigate, markers | M1 | Automated verified | 0.58 ([SHEET_REVIEWS](../SHEET_REVIEWS.md)) | Native check; persistent panes |
| X15.02 | Show/hide notes and note geometry | M2 | Specified | — | Not built |
| X15.03 | Protect a sheet: locked cells and allowed actions, enforced in UI, paste, automation and AI | M2 | Specified | — | Protection flags kept; not enforced everywhere |
| X15.04 | Protect workbook structure; allow edit ranges | M3 | Specified | — | Not built |
| X15.05 | Shared workbook, co-authoring and change history | M4 | Specified | — | See D07 |

## X16 Macros, add-ins and scripts

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X16.01 | XLSM opened with macros preserved and disabled, with a notice | M4 | Specified | — | Rejected today |
| X16.02 | Run VBA macros | M5 | Specified | — | Architecture decision (STATUS risks) |
| X16.03 | Office Scripts | M5 | Specified | — | Not built |
| X16.04 | Excel add-ins (XLAM and Office.js) | M5 | Specified | — | See D10.06 |

## X17 Workbook format fidelity

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X17.01 | XLSX import and export of supported content, keeping unknown parts | M1 | Automated verified | Many versions; independent readers and schema checks | Excel round trip |
| X17.02 | Typed values and caches on export: numbers, booleans, errors, dates | M1 | Native verified | 0.36–0.37 | Excel comparison |
| X17.03 | XLTX templates | M2 | Automated verified | 0.56 ([TEMPLATES](../TEMPLATES.md)) | Excel comparison |
| X17.04 | XLSM and XLTM with macros preserved | M4 | Specified | — | Rejected today |
| X17.05 | XLS (97–2003) open | M4 | Specified | — | Not built |
| X17.06 | XLSB open and save | M4 | Specified | — | Not built |
| X17.07 | CSV export with correct encoding and quoting | M1 | Implemented | Core CSV export (`workspace.rs`) | Tests; delimiter/locale options |
| X17.08 | ODS open and save | M4 | Specified | — | Not built |
| X17.09 | Per-feature compatibility matrix for workbooks | M1 | Specified | — | ROADMAP §2 matrix |
| X17.10 | Excel-created workbooks from the corpus open, round-trip and recalculate like Excel | M1 | Automated verified | 272/291 open; 271 round-trip; 95.9% of 23,116 formulas match ([CORPUS](CORPUS.md#office-created-corpus-real-files)) | X08.14, X09.08, X08.15, X17.11–X17.12 |
| X17.11 | Strict Open XML workbooks | M2 | Specified | [CORPUS](CORPUS.md#office-created-corpus-real-files) | 3 corpus files rejected |
| X17.12 | Tolerant XLSX reading: cells without `r`, unusual shared strings, dimensions and relationship targets | M1 | Specified | [CORPUS](CORPUS.md#office-created-corpus-real-files) | 7 corpus files rejected; one export content type fails to reimport |
| X17.13 | Formulas entered or changed in Folio are written in worksheet form: `_xlfn.` prefixes for newer functions, and `@` resolved or written as `_xlfn.SINGLE` | M1 | Automated verified | [legacy formula tests](../../rust/src/legacy_formula_tests.rs) | Excel opening a Folio-edited file (corpus round trip through Excel) |

## X18 Page setup, printing and PDF for workbooks

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X18.01 | Page setup: orientation, margins, paper, scaling (fit to pages) | M3 | Specified | — | Not built |
| X18.02 | Print areas and print titles | M3 | Specified | — | Imported names kept; no UI |
| X18.03 | Manual page breaks and Page Break Preview | M3 | Specified | — | Not built |
| X18.04 | Sheet headers and footers with fields | M3 | Specified | — | Not built |
| X18.05 | Print and export PDF for a selection, sheet or workbook | M3 | Specified | — | See D05 |

## X19 Scale and resource limits

| ID | Requirement | M | State | Evidence | Gap / next step |
|---|---|---|---|---|---|
| X19.01 | Full Excel address space (1,048,576 × 16,384) navigable, with sparse rendering | M1 | Native verified | 0.13 ([VALIDATION](../../VALIDATION.md#013-sparse-worksheet-viewport-2026-09-18)) | Excel comparison of limits |
| X19.02 | Incremental recalculation: edits cost time in proportion to what changed | M1 | Automated verified | Phase 2 | M4 Max measurement |
| X19.03 | Remove the 30-sheet and 100,000-populated-cell limits | M3 | Specified | — | Current hard limits |
| X19.04 | One-million-populated-cell workbooks, dense and sparse, within budgets | M3 | Specified | — | Stress tier not defined (ROADMAP §7) |
| X19.05 | Long recalculations show progress and can be cancelled | M3 | Specified | — | Not built |
| X19.06 | Cell content limits: 32,767 characters and 8,192-character formulas | M3 | Specified | — | Current limit is 10,000 bytes per cell |
