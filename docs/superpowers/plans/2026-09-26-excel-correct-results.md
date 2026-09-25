# Excel-correct results on real files (STATUS rank 2)

**Goal:** real Excel workbooks open and calculate as Excel does. Evidence comes from the Office-created corpus (`docs/status/CORPUS.md`): Excel's own cached values, and the files Folio rejects.

**Baseline (2026-09-25):**
- Formulas: 744 of 23,116 differ from Excel, in 21 files.
- XLSX: 272 of 291 open, and 271 round-trip.

**Out of scope:**
- the 32 MB workspace limit (X19.03, rank 21);
- missing functions (X08.03);
- files that are correctly refused: 2 with an XML DTD, 2 with corrupt checksums.

### Task 1: 3-D references (X08.15)

- [x] `SUM(Sheet1:Sheet4!C1)` and the other aggregates over a sheet span calculate as Excel does. This includes spans whose end sheets are later moved, inserted between or deleted.
- [x] The formula text stays as written, and it round-trips.

### Task 2: Implicit intersection gaps (X08.14)

- [x] Unary plus or minus on a range (`=+F9:H9`) is intersected, as Excel does.
- [x] Names Folio can't use give `#NAME?`, not `#ERROR!`.

### Task 3: Coercion and error cases (X08.08)

- [x] Each corpus case gets Excel's result, or a documented reason it doesn't:
  - text numbers in number arguments (`RIGHT("a string","6")`, `SUMSQ("2","3",4)`, `MIN(…,"-20")`);
  - `AND("1",1)`;
  - `MODE` with text;
  - `AVEDEV` of nothing;
  - `SUMX2MY2` and its relatives when the array sizes don't match;
  - `LOOKUP` on text;
  - `MINA` with logical values;
  - `MINVERSE` of a singular matrix;
  - `T` of a range.

### Task 4: TEXT format codes (X04.04)

- [ ] Classify the corpus's TEXT differences (POI's format test workbooks), and fix the ones that affect real formats:
  - codes the formatter rejects;
  - literal text in conditional sections;
  - `;;;` text sections;
  - elapsed time in brackets.
- [ ] Record what is left as known differences.

### Task 5: Tolerant XLSX reading (X17.12)

- [ ] Open the workbooks Excel opens that Folio rejects (done except 3 shared-string files):
  - a cell without an `r` address;
  - shared strings Folio calls invalid (3 files);
  - unusual worksheet dimensions (2);
  - a sheet relationship target that is missing (1);
  - beta or unusual structures (2).
- [ ] Fix the export whose content type doesn't reimport (bug66675).

### Task 6: Strict Open XML (X17.11)

- [x] Read Strict workbooks (3 corpus files) by mapping the Strict namespaces to Transitional ones.
- [x] Save them as Transitional, and say so in the save report.

### Task 7: Close-out

- [x] Rerun the corpus and record the results in CORPUS.md and STATUS.md, with the register rows and change log updated.
- [ ] Run the local CI mirror.
