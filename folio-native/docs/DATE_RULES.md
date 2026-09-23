# Date rules and recalculation

The Conditional formatting manager supports the ten date-occurring periods:
yesterday, today, tomorrow, last seven days, last/this/next week, and
last/this/next month. Week intervals are Sunday through Saturday; the last seven
days include today. Fractional times fall within their calendar day. New rules
use half-open date intervals and month boundaries constructed with DATE so
December/January and leap-year transitions do not rely on month-number equality.

Rules apply to selected, disjoint or full-column ranges, use ordinary formatting
and priority/Stop if true, and participate in Apply/Undo, structural reference
updates, saved-workspace recovery and standard XLSX import/export. Imported
formulas remain intact during style-only edits. Changing the period or range
constructs a new predicate. When an imported rule omits a formula, its period
metadata supplies the predicate. Unmodeled rule attributes remain protected by
the existing source-preservation handling.

The worksheet toolbar's **Recalculate workbook (F9)** command first commits an
active edit, then refreshes formulas and conditional styles. A single local
clock is captured for each calculation, including TODAY/NOW and conditional
expressions. The app checks local date/UTC-offset changes every thirty seconds
and when it resumes. A normal check within the same local day and offset leaves
NOW unchanged. Opening a workspace recalculates it.

Calculation results and necessary automatic row measurements publish together.
Recalculation itself creates no Undo entry, clears no Redo history and advances
no document revision. A current edit committed by F9 is still an ordinary edit.
Changed automatic row heights are saved as derived layout. Fixed/hidden/merged
rows retain the existing automatic-layout exclusions.

## Evidence and limits

Engine tests cover ten periods, fractional days, year transitions, leap days,
Sunday/Saturday boundaries, local-midnight/offset/DST changes, clock rollback,
1900/1904 calendars, priority, formula-only repainting, measured edits,
recalculation history, imported formulas and structural/file recovery. Independent
openpyxl fixtures and readers check both calendars and OOXML fields. Native UI
and packaged-build checks are recorded separately in VALIDATION.md.

This is progress within conditional formatting and calculation, not acceptance
of either complete parity family. Direct Excel comparison of date coercion,
locale/week conventions, noncanonical imported formulas and mixed-rule priority
remains outstanding. Calculation modes, iterative/circular calculation,
precision-as-displayed, all volatile functions, full dependency scheduling and
cross-document recalculation remain incomplete. Full physical sleep/wake and
system timezone-change qualification, browser clock execution and automatic
row sizing immediately on file open remain separate checks.

## Primary references

- [Microsoft: ten time periods](https://learn.microsoft.com/en-us/office/vba/api/excel.xltimeperiods)
- [Microsoft: NOW calculation timing](https://support.microsoft.com/en-us/excel/functions/now-function)
- [Microsoft: conditional-format rule fields](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.conditionalformattingrule?view=openxml-3.0.1)
- [XlsxWriter: date conditional formats](https://xlsxwriter.readthedocs.io/working_with_conditional_formats.html)

The last source's public formulas inform Sunday-based week bounds; its
last/next-month formula construction is not used for year transitions. None of
these references is a substitute for running the differential corpus in Excel.
