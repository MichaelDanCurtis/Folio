# Excel-correct results, part 2 (STATUS rank 2)

**Goal:** finish rank 2. The first plan ([part 1](2026-09-26-excel-correct-results.md)) left 656 of 23,120 corpus formulas differing from Excel, 615 of them in `TEXT`, plus 3 rejected shared-string files and one export that doesn't reimport.

**Evidence:**
- `TEXT` is measured against every `TEXT(value, code)` row of Apache POI's format test workbooks: 805 cases with Excel's own results, in `rust/vendor/ssf-rs/tests/data/text_format_cases.tsv`, run by `cargo test --test text_format_cases` in `rust/vendor/ssf-rs`. The count of matches is a ratchet.
- Baseline: 213 of 805 match. By workbook: dates 55/55, General 70/70, numbers 29/325, scientific 0/248, elapsed time 24/50, choices 14/24, conditions 21/33.
- Everything else is measured with the Office corpus (`office_corpus_tests.rs`).

### Task 1: Number codes (NumberFormatTests)

- [ ] Non-ASCII literal characters such as `£` and `¥`.
- [ ] Thousands separators anywhere in the integer part (`#,##`, `0,0`, `?,???`), and trailing commas that scale by 1,000.
- [ ] `?` placeholders in integers and decimals, padded with spaces.
- [ ] Percent signs, including more than one.
- [ ] Fractions with literal text between the parts (`#\:#=/=#`, `#_#/#`).

### Task 2: Scientific notation (NumberFormatApproxTests)

- [ ] `E+`, `E-`, `e+` and `e-` with literal text and placeholders around the mantissa and exponent.

### Task 3: Elapsed time, conditions and choices

- [ ] `[h]`, `[m]` and `[s]` alongside literal text.
- [ ] Conditional sections (`[>100]`) and the section Excel chooses for each value, including `;;;` and text sections.
- [ ] `TextFormatTests`: text sections and `@`.

### Task 4: Function edge cases

- [ ] Each remaining corpus difference outside `TEXT` gets Excel's result or a recorded reason: RANK over a union, EDATE through names, SUMIF with a range criterion, LOOKUP on text, MINA/MAXA, MODE with text, SUMPRODUCT with errors, MDETERM/MINVERSE on text, UPPER of `@range`, HYPERLINK, GETPIVOTDATA, a name resolving to another sheet.

### Task 5: Tolerant reading and export

- [ ] The 3 files with shared strings Folio calls invalid or missing.
- [ ] The bug66675 export that doesn't reimport.

### Task 6: Close-out

- [ ] Rerun the corpus; update CORPUS.md, STATUS.md, the register rows and the change log.
- [ ] Run the local CI mirror.
