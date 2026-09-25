# Folio patch to IronCalc 0.8.3

Upstream: https://github.com/ironcalc/IronCalc (crate ironcalc_base 0.8.3).
Upstream source and MIT license are retained in this directory (MIT option of MIT OR Apache-2.0).

Adds Model::set_filtered_rows and ephemeral per-sheet filter results. SUBTOTAL
uses these exact results when supplied, including for worksheet AutoFilter and
cells outside a table, while retaining upstream behavior for callers that do
not supply them. Manual hidden-row state remains independent. Folio rebuilds
its calculation model and reinstalls filter results after metadata or sheet
structure changes. This is a calculation adapter, not a serialized file format
change. Folio tests exercise the real patched engine.

The vendored build script reports the pinned crate version with a folio.1
suffix instead of querying an absent Git checkout. It watches build.rs rather
than nonexistent .git paths, preventing unnecessary recompilation on every
Cargo invocation. Modified upstream files: src/model.rs, src/new_empty.rs,
src/functions/subtotal.rs and build.rs.

## 0.35 TEXT formatting

`src/functions/text/common.rs` routes numeric `TEXT` results through the same
vendored `ssf-rs` formatter as Folio's cell display. The new path validates format
shape and output length, returns formula errors for unsupported patterns, and
broadcasts both values and format-code arrays. It returns text for logical inputs
and converts numeric-looking strings with the engine's existing coercion helper.
Accounting fill/skip codes produce plain characters, not measured cell layout.
`Cargo.toml` adds the local `ssf-rs` dependency.

Direct Excel comparison for coercion, error precedence, arrays and negative dates
is outstanding. The engine still uses the 1900 calendar for TEXT, matching its
other date functions; 1904 calendar formula evaluation remains incomplete. Folio's
visible dynamic-spill projection is a separate outstanding integration task.

## 0.36 workbook calendar

The host sets `Model::set_date1904` before loading inputs and rebuilds when the
setting changes. The setting is ephemeral and does not change IronCalc's binary
workbook schema. Date input/coercion, VALUE/TEXT, date criteria, date/time
functions and financial date paths use the selected calendar. Bond helpers
continue using their original 1900 serial domain internally, with conversion
at their argument/result boundaries. Helpers that call YEARFRAC retain workbook
serials, avoiding a second conversion. XIRR/XNPV accept serial zero in 1904 mode.

Date arithmetic uses checked month/day offsets. The new `input_parses_as_number`
host query lets OOXML importers preserve explicitly typed date/currency/percent
strings instead of coercing them. Tests cover paired Gregorian calendars,
calendar invalidation/isolation, date-part arrays and supported financial paths.
Direct Excel differential coverage, early-1900 special cases, all coercion and
locale rules, complete negative-date behavior, and conditional-format/rendering
integration remain separate acceptance work. IronCalc's standalone legacy
formatted-cell API still uses its original renderer; Folio uses shared SSF.


## 0.38 early worksheet dates

Calendar components retain serial zero (1900-01-00), serial 60 (1900-02-29)
and the Excel weekday sequence. Gregorian conversion now maps real dates before
March 1900 to the correct serials. DAY/MONTH/YEAR, DATE, EDATE, WEEKDAY,
WORKDAY/NETWORKDAYS (including INTL variants) and holiday identity use checked
serial/component helpers. DATE supports years 0–1899 by adding 1900 and normalizes
month/day overflow in serial space. EDATE clamps and EOMONTH month ends use
Gregorian month lengths: February 1900 ends at serial 59. The standalone date formatter uses exact
worksheet fields; Folio display and TEXT continue using SSF.

The fixture expectations are independently modeled, not outputs from Microsoft
Excel. EOMONTH's February exception follows a firsthand Microsoft Community Hub
report (4335766); ClosedXML regression tests independently confirm the EDATE
January 31 -> February 28 clamp and the EOMONTH exception. These are supporting
references, not live Excel execution. Direct Excel comparison, remaining EDATE
boundary/coercion behavior,
week-number/financial/DATEDIF conventions around serial 60, negative dates and
full locale/timezone/coercion rules remain unqualified. Gregorian bridge users
still see serials 59 and 60 as the same real date. Upstream tests that deliberately
expected a different early calendar were changed to the Excel serial convention.


## Folio 0.39: validation expression context and clock input

`Model::evaluate_expression_cells` evaluates a bounded scalar/range/array in a
specified worksheet/cell context without inserting a helper cell. Typed values,
errors, context bounds and unchanged workbook bytes have dedicated tests. Folio
uses this for validation criteria and range-backed list choices. It is not
support for arbitrary dynamic spilling or named ranges in the Folio projection.
The existing number-input parser now recognizes colon-separated clocks,
optional seconds/fractions and AM/PM as fractional days. The parser is shared by
cell entry and existing numeric coercion paths. Locale-specific clocks,
combined date/time input and full Excel coercion comparison remain unqualified.

## 0.49 calculation clock

`Model::set_calculation_clock(millis, offset_seconds)` captures one instant and
host-local UTC offset for a calculation. Default TODAY/NOW use that local civil
clock; the existing optional named-timezone extension uses the same instant.
The clock is ephemeral and does not alter workbook settings or its binary
schema. Invalid timestamps/offsets leave the previous clock intact. Native and
browser timezone helpers now share the Gregorian-to-worksheet serial bridge,
including real dates before March 1900. Whole-second precision is retained.

Folio refreshes the captured context on calculation, open, local date change
or UTC-offset change. NOW is not a running wall-clock display. The adapter's
separate conditional-format engine uses this same expression context; the
standalone IronCalc conditional-format implementation remains outside Folio's
rendering path. Browser execution and complete timezone/locale behavior have
not been qualified in native Excel.

## Phase 3 formula read budget and range clipping

Each formula may read at most `MAX_FORMULA_READS` (2^24) cells directly;
reads made by the formulas it depends on count against their own budgets.
`evaluate_cell` counts reads on a per-formula stack, and a formula that goes
over unwinds (with `resume_unwind`, so no panic hook runs) to where its own
evaluation started, which stores `#NUM!` ("This formula reads too many
cells"). Before this, one formula over a whole-sheet range could visit 17
billion cells: `COUNTBLANK(A1:XFD1048576)` took over three minutes, and array
arithmetic such as `SUM(A1:XFD1048576*1)` would have tried to allocate
hundreds of gigabytes, which aborts the process. A real panic inside a
formula still propagates.

`Model::clamp_to_data` clips a range of more than 2^20 cells to the sheet's
populated extent (upstream's `SUM` already did this for whole rows and
columns). COUNT, COUNTA, COUNTBLANK, AVERAGE, AVERAGEA and the other
`for_each_value` users, MIN, MAX, MINA, MAXA, SUMSQ, PRODUCT, AVEDEV, LARGE,
SMALL, SUBTOTAL, CONCAT, PERCENTILE/QUARTILE/MEDIAN (`collect_sorted_values`)
and MODE use it, since they skip empty cells. COUNTBLANK adds the clipped-off
area arithmetically. Functions that keep empty cells or their positions
(TEXTJOIN with empties, paired-array statistics, lookups, array builders) are
unchanged and rely on the read budget. Data spread to opposite corners of a
sheet makes the extent the whole sheet, so whole-sheet aggregates over it hit
the budget; iterating only populated cells would lift that and is not done.

Modified upstream files: src/model.rs, src/new_empty.rs,
src/functions/statistical/count_and_average.rs,
src/functions/statistical/mode_functions.rs,
src/functions/statistical/percentile.rs,
src/functions/math_and_trigonometry/mathematical.rs,
src/functions/subtotal.rs and src/functions/text/common.rs. Folio's
`formula_limits_tests.rs` covers exact whole-sheet results, the budget, and
whole-column array math. The vendored suite passes except
`test_to_from_bytes::errors`, which fails identically without these changes.
