# Compatibility matrix and the pre-save impact report (STATUS rank 10)

**Goal:** ROADMAP §2's file compatibility contract for the formats Folio writes. The goal has three parts:
- a per-feature matrix for DOCX and XLSX (W16.09, X17.09);
- a report of what the open file contains and how Folio treats each part of it;
- before an export that would lose or invalidate something, a concrete impact preview with a safe alternative (D02.17).

**States**, as ROADMAP §2 names them:
- **Verified**: automated round-trip evidence.
- **Partial**: some of the feature is edited, the rest kept.
- **Preserved only**: kept unchanged, but not shown or edited.
- **Unsupported**: refused, dropped or invalidated.

The matrix and the report come from one table in the engine (`compatibility.rs`), so the app and the document can't disagree.

### Task 1: The feature table and the file inventory

- [x] One table: for each feature, how to detect it in a DOCX or XLSX package, and its state for open, display, edit, calculate, save, round-trip, print and automation, with the evidence.
- [x] Features:
  - Word: text and formatting, styles, lists, tables, inline pictures, floating pictures, fields, comments, tracked changes, headers and footers, footnotes and endnotes, equations, content controls, charts and SmartArt, embedded objects, bookmarks, hyperlinks, sections and columns.
  - Excel: cells and formulas, styles and number formats, merged cells, tables, conditional formatting, data validation, comments and notes, defined names, charts, pivot tables, sparklines, slicers, external links, data connections.
  - Both: macros, digital signatures, protection and encryption, custom XML, and unknown extension parts.
- [x] `compatibilityReport`: the features present in the open file, with counts, state and what saving will do.

### Task 2: The matrix document

- [x] `docs/status/COMPATIBILITY.md` is generated from the table. A test fails when the two differ.

### Task 3: The Compatibility Report in the app

- [x] The report lists what the open file contains, grouped by state, with counts and what saving will do. The format limits stay.

### Task 4: Impact preview before a lossy export (D02.17)

- [x] **Digital signatures:** a signed package that has been edited has its signatures removed on export. Keeping them would leave an invalid signature.
- [x] **Preview:** before an export whose effects include anything lost, changed or invalidated, a preview lists them. The choices:
  - Export anyway;
  - Save a Folio copy instead;
  - Cancel.
- [x] Exporting an unchanged file, or one with nothing at risk, shows no preview.

### Task 5: Close-out

- [x] Rerun the Office corpus: the report runs on every file without error.
- [x] Update the register rows, STATUS.md counts and the change log.
- [x] Run the local CI mirror. The Flutter baseline is 7 failures that happen only on Linux.
