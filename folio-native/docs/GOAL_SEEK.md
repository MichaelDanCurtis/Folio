# Goal Seek

Version 0.80 adds **Data > Goal Seek**. Choose a formula cell, the desired
numerical result and one changing input cell. Either cell can be on another
worksheet. A blank input starts at zero; existing numeric inputs use the
calculation engine's parsed value. The target field also accepts percentages.
Search settings expose the absolute maximum result difference (default
0.000001) and a limit of 4–10,000 trials (default 200).

The dialog shows the original input, proposed input, target, calculated result,
difference and trial count. Displayed numbers use twelve significant digits;
hover over a result to see its full stored precision. **Apply** keeps a matching
result. **Stop** ends the search and shows its best trial. An unmatched result
requires the explicit **Apply best value** action. **Cancel**, closing the dialog
and changing its inputs discard the old search. A changed workbook disables
the result and requires a new search.

## Calculation and editing

Trials use an isolated copy of the existing calculation model, including its
date system, clock, filtered rows and table definitions. The changing cell is
recalculated one trial per native request; the UI can stop between workbook
calculations. The search combines a bracketed secant/bisection method with
secant extrapolation and expanding probes. Formula errors are skipped as
invalid trials. A small interval or exhausted trial budget is not reported as
convergence: the formula result must meet the requested absolute tolerance.
Convergence is numerical; it does not prove an exact root exists. A matching
input is evaluated again before the result is offered as converged.

Search trials do not change workspace values, formula caches, calculation
generation, documents, Undo/Redo, file state or saved recovery. The native
session is ephemeral and tied to workspace identity, revision, calculation
generation and a serial token. Cancellation of an old token cannot affect a
new search. Search reads also preserve an unrelated staged row-layout edit.

Apply uses the existing measured cell-entry transaction: normal cell validation,
table handling and automatic row sizing run before committing. The formula is
recalculated and checked against the reviewed trial result. A validation or
measurement failure leaves the workbook unchanged and the result available for
retry. An accepted change is one Undo step; an unchanged best input creates no
history record. XLSX export uses the normal cell and formula-cache path.

Formula cells cannot be used as the changing input. Protected sheets, merged
inputs, imported array/data-table members, spill results and table headers are
rejected where identified by the current projection/source model. Their broader
editing semantics remain part of the unfinished protection/array/table work.
The search has no global-root or optimal-root guarantee; discontinuities,
unreachable values and flat regions may stop without reaching the target.
Volatile formulas can change after a calculation; Apply rejects a result that
no longer reproduces within the selected tolerance. Full volatile, dynamic,
named/external and financial-function behavior inherits the calculation
engine's existing gaps. Table expansion caused by an accepted ordinary entry
must reproduce the trial's result or Apply rejects it. Searches do not offer
multi-variable constraints, scenarios, data tables or Solver functionality.

## Qualification

Rust cases cover linear/nonlinear equations, negative inputs, a flat derivative,
domain errors, discontinuities, impossible targets, tolerance-based convergence,
cross-sheet PMT with an independently computed payment, immutable trials,
Stop/Cancel, stale and invalid requests, token isolation, validation rejection,
no-op input, measured-edit retry, source protection/merge/array guards, hidden
and filtered rows, the 1904 calendar and Excel export/reimport.

Dialog tests cover validation, result review, Apply failure/retry, Stop and
explicit best-value acceptance, closing during an outstanding Start, stale
trial completion and invalidation after editing settings. The real Flutter/Rust
application test opens the Data tab, searches across worksheets, cancels, retries
a simulated measurement failure, applies, undoes/redoes, saves recovery and
exports/reimports XLSX while retaining the watched cells and formula.

The single 0.80 desktop preview passes a cross-sheet search from 50 to 75 for a
formula target of 750. Preview/Cancel preserve recovery bytes; Apply changes only
the selected input, Undo restores it and Redo reapplies it. Native Excel export
and in-place Save are byte-identical; recovery survives a normal quit/restart,
and reimport retains the result, formula and watched cells. Eleven captured
states pass independent comparison and both native exports pass Microsoft 365
schema validation. Native Stop/best-value, broader numerical/volatile/performance
cases and direct Microsoft Excel workflow comparison remain pending. Evidence:
`release/validation/goal-seek-native`.

Full X14, Office parity and the later twenty novel AI features
remain incomplete. Goal Seek is part of the Excel parity work and does not
count as a novel AI feature.

Baseline: Microsoft's [Goal Seek workflow](https://support.microsoft.com/en-us/excel/use-goal-seek-to-find-the-result-you-want-by-adjusting-an-input-value)
describes Set cell, To value, By changing cell and the single-input scope.
