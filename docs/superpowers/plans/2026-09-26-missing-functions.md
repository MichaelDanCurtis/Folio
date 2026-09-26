# Missing Excel functions (STATUS rank 17)

**Goal:** evaluate the catalog functions IronCalc lacks that don't need a server, a data model or PivotTables (X08.03, part of X08.04). Before this: 483 of Excel's 514 functions evaluated.

**Approach:** the functions live in `vendor/ironcalc_base/src/functions/folio/`. `Function::folio_lookup` finds them by name, so IronCalc's binary language tables stay unchanged.

### Task 1: AGGREGATE
- [x] Functions 1–19 and options 0–7: ignore hidden rows, error values and nested SUBTOTAL/AGGREGATE.
- [x] Reference form (several ranges) and array form with k (LARGE, SMALL, PERCENTILE and QUARTILE, .INC and .EXC).

### Task 2: Text and conversion functions
- [x] BAHTTEXT: Thai words, with ยี่/เอ็ด and ล้าน groups.
- [x] DBCS and JIS: full-width ASCII and katakana, with voiced marks joined.
- [x] PHONETIC: the text of the top-left cell, since Folio keeps no phonetic guides.
- [x] ENCODEURL.
- [x] HYPERLINK: the value shown in the cell.
- [x] EUROCONVERT: fixed rates, precision and triangulation.
- [x] FILTERXML: an XPath subset.

### Task 3: GROUPBY and PIVOTBY
- [x] Function names as values (`SUM`, stored as `_xleta.SUM`) and LAMBDAs.
- [x] Headers: detected, hidden or shown.
- [x] Grand totals and subtotals, at the top or bottom.
- [x] Sort by keys or values, ascending or descending.
- [x] Filter arrays.
- [x] PIVOTBY: row and column fields with totals.

### Task 4: FORECAST.ETS family
- [x] Additive triple exponential smoothing. The timeline is sorted, duplicates are aggregated and gaps are filled.
- [x] Seasonality is detected from the autocorrelation.
- [x] Parameters are fitted by least squares.
- [x] Confidence intervals and statistics.
- [x] Excel doesn't publish its fitting method, so results are close to Excel's, not equal.

### Task 5: Close-out
- [x] IronCalc suite and catalog. COUNT now counts the numbers in an array argument.
- [x] Register and corpus: 498 of 514 functions evaluate. Corpus differences fell from 31 to 30.
