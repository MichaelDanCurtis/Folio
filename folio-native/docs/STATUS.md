# Folio status register

Current state of Word and Excel parity, as of 2026-09-25 (after Phase 3, `main` at `3717237`). The detailed register is in [status/](status/README.md): 46 families, each split into bounded child requirements with a state and evidence. [PARITY.md](../PARITY.md) keeps the family map and release history; [VALIDATION.md](../VALIDATION.md) keeps the evidence.

**Outcome so far: 0 of 46 families accepted, and 0 of 20 additional AI features accepted.** No child requirement is "Office compared" yet, because the Office reference baseline has not been recorded ([OFFICE_BASELINE](status/OFFICE_BASELINE.md)).

## Counts by family and state

States are defined in [status/README](status/README.md). `test/status_register_test.dart` fails if this table disagrees with the register.

| Family | Name | Specified | Implemented | Automated verified | Native verified | Office compared | Accepted | Total |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| D01 | [Desktop application, windows and Office-like interaction](status/desktop.md#d01-desktop-application-windows-and-office-like-interaction) | 7 | 0 | 2 | 4 | 0 | 0 | 13 |
| D02 | [Files: new, open, save, recent, templates, autosave and recovery](status/desktop.md#d02-files-new-open-save-recent-templates-autosave-and-recovery) | 4 | 1 | 8 | 4 | 0 | 0 | 17 |
| D03 | [Clipboard, drag and drop, undo/redo and shortcuts](status/desktop.md#d03-clipboard-drag-and-drop-undoredo-and-shortcuts) | 4 | 0 | 2 | 4 | 0 | 0 | 10 |
| D04 | [Accessibility, zoom and localization](status/desktop.md#d04-accessibility-zoom-and-localization) | 9 | 0 | 2 | 0 | 0 | 0 | 11 |
| D05 | [Printing, page setup and PDF](status/desktop.md#d05-printing-page-setup-and-pdf) | 5 | 0 | 0 | 1 | 0 | 0 | 6 |
| D06 | [Security, protection, encryption and trusted automation](status/desktop.md#d06-security-protection-encryption-and-trusted-automation) | 5 | 0 | 2 | 0 | 0 | 0 | 7 |
| D07 | [Collaboration, sharing and conflict recovery](status/desktop.md#d07-collaboration-sharing-and-conflict-recovery) | 3 | 0 | 2 | 0 | 0 | 0 | 5 |
| D08 | [Packaging, updates and distribution](status/desktop.md#d08-packaging-updates-and-distribution) | 4 | 1 | 1 | 1 | 0 | 0 | 7 |
| D09 | [Performance, resources, lifecycle and cancellation](status/desktop.md#d09-performance-resources-lifecycle-and-cancellation) | 5 | 0 | 3 | 2 | 0 | 0 | 10 |
| D10 | [Extensibility, automation and assistant integration](status/desktop.md#d10-extensibility-automation-and-assistant-integration) | 4 | 0 | 4 | 0 | 0 | 0 | 8 |
| W01 | [Rich text, fonts, symbols and paragraph formatting](status/word.md#w01-rich-text-fonts-symbols-and-paragraph-formatting) | 9 | 1 | 1 | 4 | 0 | 0 | 15 |
| W02 | [Styles, themes and templates](status/word.md#w02-styles-themes-and-templates) | 5 | 0 | 1 | 4 | 0 | 0 | 10 |
| W03 | [Lists and outline numbering](status/word.md#w03-lists-and-outline-numbering) | 3 | 0 | 1 | 3 | 0 | 0 | 7 |
| W04 | [Find, replace, navigation and bookmarks](status/word.md#w04-find-replace-navigation-and-bookmarks) | 5 | 0 | 1 | 2 | 0 | 0 | 8 |
| W05 | [Page layout: sections, margins, columns and breaking](status/word.md#w05-page-layout-sections-margins-columns-and-breaking) | 5 | 0 | 2 | 4 | 0 | 0 | 11 |
| W06 | [Headers, footers, page numbers and backgrounds](status/word.md#w06-headers-footers-page-numbers-and-backgrounds) | 3 | 0 | 0 | 4 | 0 | 0 | 7 |
| W07 | [Tables](status/word.md#w07-tables) | 5 | 0 | 1 | 9 | 0 | 0 | 15 |
| W08 | [Pictures, shapes, text boxes, SmartArt and charts](status/word.md#w08-pictures-shapes-text-boxes-smartart-and-charts) | 7 | 0 | 1 | 0 | 0 | 0 | 8 |
| W09 | [Equations, embedded objects, media and content controls](status/word.md#w09-equations-embedded-objects-media-and-content-controls) | 5 | 0 | 0 | 0 | 0 | 0 | 5 |
| W10 | [Fields, captions, cross-references, TOC and indexes](status/word.md#w10-fields-captions-cross-references-toc-and-indexes) | 7 | 0 | 1 | 0 | 0 | 0 | 8 |
| W11 | [Footnotes, endnotes, citations and bibliography](status/word.md#w11-footnotes-endnotes-citations-and-bibliography) | 4 | 0 | 0 | 0 | 0 | 0 | 4 |
| W12 | [Proofing](status/word.md#w12-proofing) | 5 | 0 | 3 | 2 | 0 | 0 | 10 |
| W13 | [Comments, track changes, compare and merge](status/word.md#w13-comments-track-changes-compare-and-merge) | 3 | 0 | 4 | 2 | 0 | 0 | 9 |
| W14 | [Mail merge, labels and envelopes](status/word.md#w14-mail-merge-labels-and-envelopes) | 4 | 0 | 0 | 0 | 0 | 0 | 4 |
| W15 | [Forms, restricted editing, protection and macros](status/word.md#w15-forms-restricted-editing-protection-and-macros) | 4 | 0 | 0 | 0 | 0 | 0 | 4 |
| W16 | [Document format fidelity](status/word.md#w16-document-format-fidelity) | 7 | 0 | 3 | 0 | 0 | 0 | 10 |
| W17 | [Views, zoom and accessibility checking](status/word.md#w17-views-zoom-and-accessibility-checking) | 6 | 0 | 0 | 1 | 0 | 0 | 7 |
| X01 | [Selection, editing, clipboard and fill](status/excel.md#x01-selection-editing-clipboard-and-fill) | 8 | 0 | 6 | 4 | 0 | 0 | 18 |
| X02 | [Find, replace, Go To and navigation](status/excel.md#x02-find-replace-go-to-and-navigation) | 4 | 0 | 3 | 1 | 0 | 0 | 8 |
| X03 | [Rows, columns and sheets](status/excel.md#x03-rows-columns-and-sheets) | 5 | 1 | 2 | 3 | 0 | 0 | 11 |
| X04 | [Cell formatting and styles](status/excel.md#x04-cell-formatting-and-styles) | 5 | 0 | 6 | 2 | 0 | 0 | 13 |
| X05 | [Views, panes and zoom](status/excel.md#x05-views-panes-and-zoom) | 5 | 0 | 0 | 1 | 0 | 0 | 6 |
| X06 | [Sort, filter and tables](status/excel.md#x06-sort-filter-and-tables) | 5 | 0 | 1 | 4 | 0 | 0 | 10 |
| X07 | [Data validation and conditional formatting](status/excel.md#x07-data-validation-and-conditional-formatting) | 3 | 0 | 2 | 5 | 0 | 0 | 10 |
| X08 | [Functions and calculation semantics](status/excel.md#x08-functions-and-calculation-semantics) | 7 | 0 | 7 | 2 | 0 | 0 | 16 |
| X09 | [Arrays, spill, names and external references](status/excel.md#x09-arrays-spill-names-and-external-references) | 5 | 0 | 3 | 0 | 0 | 0 | 8 |
| X10 | [Auditing](status/excel.md#x10-auditing) | 4 | 0 | 0 | 2 | 0 | 0 | 6 |
| X11 | [Charts, sparklines and drawings](status/excel.md#x11-charts-sparklines-and-drawings) | 6 | 1 | 1 | 0 | 0 | 0 | 8 |
| X12 | [PivotTables](status/excel.md#x12-pivottables) | 6 | 0 | 0 | 0 | 0 | 0 | 6 |
| X13 | [Data import, Power Query and Power Pivot](status/excel.md#x13-data-import-power-query-and-power-pivot) | 5 | 0 | 0 | 0 | 0 | 0 | 5 |
| X14 | [What-if analysis](status/excel.md#x14-what-if-analysis) | 4 | 0 | 0 | 3 | 0 | 0 | 7 |
| X15 | [Notes, comments, protection and sharing](status/excel.md#x15-notes-comments-protection-and-sharing) | 4 | 0 | 1 | 0 | 0 | 0 | 5 |
| X16 | [Macros, add-ins and scripts](status/excel.md#x16-macros-add-ins-and-scripts) | 4 | 0 | 0 | 0 | 0 | 0 | 4 |
| X17 | [Workbook format fidelity](status/excel.md#x17-workbook-format-fidelity) | 7 | 1 | 4 | 1 | 0 | 0 | 13 |
| X18 | [Page setup, printing and PDF for workbooks](status/excel.md#x18-page-setup-printing-and-pdf-for-workbooks) | 5 | 0 | 0 | 0 | 0 | 0 | 5 |
| X19 | [Scale and resource limits](status/excel.md#x19-scale-and-resource-limits) | 4 | 0 | 1 | 1 | 0 | 0 | 6 |
| **All** | | **233** | **6** | **82** | **80** | **0** | **0** | **401** |

By milestone, the M1 (daily report and budget) children are:
- 136 in total;
- 52 native verified;
- 56 automated verified only;
- 4 implemented without dedicated tests;
- 24 not built.

## Ranked gap list

Ordered by what blocks the M1 daily-workflow scenario (ROADMAP §6) first, then by milestone.

- **Effort:** S ≤ 1 week, M 1–3 weeks, L 3–8 weeks, XL > 8 weeks, for one engineer working with this codebase.
- **Confidence:** how well the scope is known today.

| Rank | Gap | Children | Effort | Confidence | Needs |
|---:|---|---|---|---|---|
| 1 | Record the Office baseline and create the daily-workflow comparison files in Word/Excel. The real-world corpus (387 Office-saved files) now covers much of the file-compatibility evidence; the baseline is still needed for interaction and layout comparisons. | [OFFICE_BASELINE](status/OFFICE_BASELINE.md), [CORPUS](status/CORPUS.md) | S | High | Owner, with Office installs |
| 2 | **Wrong results on real Excel files** (from the corpus): defined names in formulas, implicit intersection in legacy formulas, 3-D references, coercion cases; tolerant reading of the 7 rejected workbooks and Strict Open XML | X09.08, X08.14, X08.15, X08.08, X17.11, X17.12 | M | High | — |
| 3 | Native qualification sweep: raise the 42 M1 children that are only automated-verified; resolve the 7 Flutter tests that fail on Linux (Save/Save As, external writes, leave-file prompts, native menus) | D02.03–D02.09, D03.04, X01.12, X04.01–X04.04, W13.01, X15.01 and others | M | High | The Mac (packaged build) |
| 4 | Workbook editing essentials: name box and Go To, whole row/column/sheet selection and Ctrl+arrow, fill handle, borders and fill colour, Clear menu, status-bar aggregates, zoom, duplicate sheet, F2 point mode | X02.04–X02.05, X01.03, X01.06, X01.13, X01.17, X04.07–X04.08, D01.06, X05.03, X03.06 | L | High | — |
| 5 | Clipboard exchange with Office and other apps, and cut/move | D03.05, X01.10, X01.11 | L | Medium | Mac clipboard testing |
| 6 | Document editing essentials: Home-tab text commands, hyperlink UI, word-count dialog, editor zoom, formatting marks, table drag-resize, Format Painter | W01.13, W01.14, W12.09, W17.02, W17.07, W07.06, D03.10, X04.12 | L | High | — |
| 7 | Pictures in documents: render inline images, insert from file | W08.01–W08.02 | L | Medium | Depends on risk R1 |
| 8 | Measure the ROADMAP §7 budgets on the reference Mac: launch, typing latency, app-level open/save, memory | D09.01–D09.03, D09.05–D09.07 | S | High | The Mac |
| 9 | Keyboard and VoiceOver audit of the daily workflow | D04.01–D04.03 | M | Medium | The Mac |
| 10 | Compatibility matrix and the pre-save impact report | W16.09, X17.09, D02.17 | M | Medium | Corpus (rank 1) for the Office columns |
| 11 | Proofing accuracy corpus and measurement (ROADMAP §5) | W12.01, W12.04 | L | Medium | Human-reviewed corpus |
| 12 | Assistant: stale-context conflicts and real-model qualification | D10.01–D10.05 | M | Medium | Live provider runs |
| 13 | Print-layout editing and page-aware layout: widow/orphan, keep rules, floating objects | W05.07–W05.08, W08.04, W01.08 | XL | Low | Risk R1 decision |
| 14 | References: TOC, captions, cross-references, footnotes | W10.03–W10.05, W10.08, W11.01 | L | Medium | Page model from rank 12 for page numbers |
| 15 | Printing and PDF (documents, then workbooks) | D05.02, D05.04–D05.06, X18 | L | Medium | — |
| 16 | Several open files, as tabs or windows | D01.07, D01.09 | L | Medium | — |
| 17 | Differential function corpus, then the missing functions (AGGREGATE, GROUPBY, PIVOTBY and others) | X08.02–X08.03 | L | Medium | Office baseline |
| 18 | Dynamic-array spill in the grid; Name Manager; LET/LAMBDA names | X09.02, X09.04–X09.05 | M | Medium | — |
| 19 | Charts: render imported charts, insert and edit, save as Office charts | X11.01–X11.04 | XL | Low | Risk R3 |
| 20 | PivotTables | X12.01–X12.04 | XL | Low | — |
| 21 | Scale: remove the 30-sheet/100k-cell limits; one-million-cell tier | X19.03–X19.05 | L | Medium | Stress tier definition |

The M4/M5 breadth (formats beyond DOCX/XLSX, macros, data connectors, co-authoring, mail merge, add-ins) stays in the register at "Specified". It is ranked after these, and its risks below need decisions before it can be scheduled.

### First delivery package

**Rank 4, workbook editing essentials, is the first M1 package that can be built and verified with automated evidence in any environment.** It closes 10 named M1 children. Its [plan](../../docs/superpowers/plans/2026-09-25-m1-workbook-editing-essentials.md) is in progress: Tasks 1–2 are done (X01.03, D01.06). Its native qualification joins the rank 3 sweep on the Mac.

**Rank 2 comes before the rest of it.** The corpus showed wrong calculated values on real Excel files, which is worse than a missing command. Ranks 1, 3, 8 and 9 need the owner's Office installs or the Mac, and can run alongside.

## Risk register

ROADMAP §7 asks for the highest-risk dependencies to be evaluated early. "Evaluated" here means the current position is recorded, with the proof or decision that would settle it. It doesn't mean the risk is resolved.

| ID | Risk | What is known | Measured here | Decision or proof still needed |
|---|---|---|---|---|
| R1 | Rich-text layout ceiling | The Word editor is a continuous flutter_quill editor, and pages exist only in preview (0.68). Floating objects, footnotes on pages, print-layout editing and complex scripts all need a page-aware layout engine. | — | A prototype: page-canvas editing with one floating picture and one footnote, compared with Word's page breaks. Its outcome decides whether to extend Quill or build a layout engine (XL). |
| R2 | Formula-engine coverage | IronCalc is vendored and patched. It has incremental recalculation (Phase 2) and bounded ranges (Phase 3), and it evaluates 483 of Excel's 514 catalog functions. | Catalog coverage ([excel-functions.tsv](status/excel-functions.tsv)); IronCalc suite 2,228 pass | Correctness against Excel is unmeasured. The differential corpus (X08.02) needs the baseline. Decision made: keep IronCalc (Phase 2). |
| R3 | Editable OOXML objects | Charts, pictures, shapes, SmartArt and equations are preserved as XML and parts, but not rendered or editable. | — | A DrawingML rendering proof: one imported chart and one picture rendered in both editors. Chart editing is XL. |
| R4 | VBA, add-ins and Office Scripts | No runtime. Macro-bearing files are rejected (0.56). | — | Decide the M4 stance: at minimum, open with macros preserved and disabled (D06.03, X16.01). Running VBA needs an interpreter; no mature embeddable one is known. Office.js needs a web host. Both may carry licence costs (program plan dependency 3). |
| R5 | Data connectors and Power Query | None built. The M language, credentials and refresh are each large. | — | A scope decision for M4: which sources, and whether queries are refreshable or preserved only. |
| R6 | Collaboration | Local only. Co-authoring on OneDrive/SharePoint needs Microsoft's co-authoring protocols or a Folio service. | — | An architecture decision before M4. Offline conflict handling (D07.04) can start from the existing external-change guards. |
| R7 | Font licensing | Office documents use Calibri, Cambria and Aptos, which can't be bundled. Metric-compatible open fonts exist for some (Carlito for Calibri, Caladea for Cambria), and layout fidelity depends on them. | — | A decision on bundled substitutes and substitution notices (W01.03). Record the reference fonts in the baseline. |
| R8 | Platforms | macOS only. Windows is the ROADMAP breadth reference, and some features are Windows-only. | — | Windows timing (program plan dependency 2), needed before M4. |
| R9 | Evidence reproducibility | Native evidence archives (`release/validation`) live on the owner's Mac and are not in the repository. GitHub Actions runners haven't started since Phase 2, so CI runs locally. | Local CI mirror | Store the archives or their hashes with the repo. Restore the GitHub runners (billing or limits). |

## Change log

| Date | Change |
|---|---|
| 2026-09-25 | Real-world corpus: 387 Word/Excel-saved files (Apache POI test data) with `office_corpus_tests.rs`. Added X08.14–X08.16, X09.08, X17.10–X17.12 and W16.10 from its findings. Fixed whole-column lookups (X08.16). New rank 2 for the calculation gaps it found. Workbook essentials Tasks 1–2 moved X01.03, X01.18 and D01.06 to Automated verified. |
| 2026-09-25 | Register created: 46 families, 391 child requirements, states set from PARITY/VALIDATION evidence as recorded up to 0.82 and Phases 1–3. The function catalog was measured (514 functions). |
