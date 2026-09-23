# Scenario Manager

Version 0.81 adds **Data > Scenario Manager** for each worksheet. Add a named
set of 1–32 changing cells, capture their current values, edit the saved values
and comment, rename/delete scenarios, and Show a selected set. Saving a scenario
does not change worksheet inputs. Showing one recalculates the workbook and is
one Undo step. Changing cells containing formulas require explicit replacement
acknowledgement. A formula captured into a scenario uses its evaluated value.

Merge copies selected scenarios from another worksheet in the same workspace,
or (in version 0.82) **Choose workbook…** reads an XLSX/XLTX source without opening
it into the active workspace. Select a source worksheet and the scenarios to copy.
The source is a frozen snapshot for that dialog: later source-file edits require
choosing it again. Cancel leaves the workspace unchanged. Copy retains input
addresses, so source and destination layouts should match. Loading and copying
do not execute source formulas or modify the source file. Copying is one history
step, changes only target scenario definitions and does not apply their values.
Duplicate names receive case-insensitive numbered suffixes, truncating the base
name at a UTF-16-safe character boundary to fit the file's limit.

Custom input number-format definitions travel with copied scenarios in recovery.
Export maps them to matching or new IDs in the destination's format catalog,
preserving destination cell formats even when the original IDs collide. Built-in
format IDs retain their identity. Missing custom records fail the whole copy.
Protected or unsupported source collections are unavailable. Source selection is
bounded to one in-memory workbook, replaced on a new read and closed with the
dialog; workspace replacement invalidates it. This uses the existing Office
package limits and supports file selection, not an open-workbook window list.
Summary creates a separate worksheet with current inputs, each scenario's inputs
and the requested result cells. Each case starts from the same current workbook.
Summary values are static, including a distinction between calculation errors
and literal error-shaped text. Creating the summary does not switch the source
worksheet's inputs. The report can be undone and exported as an ordinary sheet.

Add/Edit drafts have Cancel, field-local history and visible save errors that
retain entered values for retry. Replacing the workbook invalidates open dialogs;
late value capture cannot populate a stale editor. Show and Summary use the
shared staged cell-edit and row-measurement transaction, so a validation or
measurement failure leaves worksheet content and history unchanged. Cell changes
retain existing table handling. Identified merged, protected, array/data-table,
spill and table-header inputs are checked before showing a scenario.

## Files and structure

Scenario definitions are projected from SpreadsheetML `scenarios`, `scenario`
and `inputCells`. Names, comments, users, protection flags, input values,
number-format IDs, deletion/undo flags, current/shown indices and summary
references are retained. ST_Xstring decoding and XML attribute encoding preserve
literal escape-like text, Unicode, tabs and line breaks. Unrecognized attributes
and child metadata remain in scoped XML templates when supported fields change.
An unchanged imported workbook retains its original export bytes. Recovery
hydrates older files only when scenario metadata is absent; an explicit empty
collection does not resurrect deleted scenarios.

Row/column operations move scenario input and summary references alongside cells.
Deleted input entries retain their file metadata and are skipped when shown;
entries marked `undone` retain their coordinates. Direct Microsoft Excel
qualification of these deletion/undo flags remains open: Microsoft's Office
implementation note differs from the standard's deleted-input behavior.

The shared font exporter now inserts newly added font properties in the schema's
order. Scenario-summary headings exposed the previous invalid append of bold
after the font name; the fix also covers other new font and rich-run properties.
It preserves the original font record and changes only the new/edited record.

## Qualification and remaining work

Nine new scenario engine cases cover creation/editing/switching, copying/deletion,
limits/duplicates/stale requests, cancellation of staged layout, validation,
formula replacement, static independent summaries, typed errors, metadata/text
roundtrips, structural insertion/deletion, protection and recovery. A font-export
regression covers new styled headings without altering the Normal font.
Four dialog tests cover saved drafts and retry, late capture, cancellation,
formula-replacement acknowledgement, stale worksheet removal, copy selection and
summary retry. The real Flutter/Rust application test creates Base and Upside,
cancels an edit, retries a failed measurement, Shows 15×30=450, undoes/redoes,
creates a report comparing 450/200/450, saves recovery and exports/reimports XLSX.

The single 0.81 desktop preview passes a disposable native workbook workflow:
cancel an Add draft without any saved change, create Downside (5,8), Show 40,
Undo to 450 and Redo to 40, rename/edit it to Steady (5,12) without applying,
then Show 60. A new Scenario Summary 2 compares current/Base/Upside/Steady as
60/200/450/60 without changing the older report. Merge copies Base and Steady
to Result without replacing its formula; deleting Base supports Undo and Redo.
Native save/export produce identical files. A normal quit, zero-process check
and restart preserve recovery bytes. Reimport retains definitions, the linked
formula, watched cell and reports; showing imported Base returns 200 and Undo
restores 60. Eighteen captured states pass independent whole-workspace checks;
both saved XLSX files pass Microsoft 365 schema validation. Evidence is frozen
in release/validation/scenarios-native. The application/test sources remain
identical to the passing automated runs; only documentation and the independent
native verifier were added afterward. The normal user's recovery is unchanged.

Version 0.82 adds four engine regression cases for frozen sources, matching sheet
IDs in separate workbooks, source/target immutability, copy history/recovery,
stale/closed/invalid requests, protected sources, custom-format collisions and
maximum-length/Unicode duplicate names. Three dialog cases cover selection,
failed-copy retry, picker cancellation, returning to the current workbook and
late completion after a workspace change. A real application test drives the
file-picker callback, previews/cancels, copies Sales (12,25), Shows 300, undoes
to 2, redoes, saves/reopens recovery and exports/reimports XLSX. The source bytes,
document, formula and watch remain unchanged. Independent history/OOXML checks
and seven-file Microsoft 365 schema validation pass.

The single 0.82 desktop preview also passes native source-file selection,
preview/Cancel with unchanged recovery, scenario-only copy, Show 7×9=63 from a
starting result 200, keyboard Undo/Redo, duplicate copy as Formatted (2), and
copy Undo/Redo. Summary compares current/both scenarios as 63/63/63. In-place Save
and export produce identical files. Normal quit, a zero-process guard and restart
preserve recovery bytes; reimport retains both editable definitions, values,
comments, protection flags and summary. Fifteen checkpoints pass independent
whole-workspace comparisons, including expected automatic row fitting. Native
file inspection confirms source custom format 164 (0.0000) transfers to destination
165 while destination 164 (0.00000) remains unchanged, and the two copied scenarios
reuse 165. The donor stays byte-identical. Donor and both outputs pass Microsoft
365 schema validation. Evidence is frozen in release/validation/scenarios-merge-native;
the earlier source archive remains immutable. Wider native source/error/protection
cases and direct Microsoft Excel execution remain outside this workflow.

Direct Microsoft Excel comparisons remain pending. Scenario PivotTable reports,
full protection/hidden behavior, formatted scenario-value editing, named inputs,
large-workbook responsiveness/cancellation and broader table/array/external and
volatile formula combinations are unfinished.
The report uses the application's existing formula, formatting and file limits.

No parity family is accepted by this addition. Scenario Manager contributes to
Excel parity and is not one of the later 20 novel AI features.

References: [Microsoft's scenario workflow](https://support.microsoft.com/en-us/excel/switch-between-various-sets-of-values-by-using-scenarios),
[Office scenario limits](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/1b8f96a0-6603-40d0-93e9-5ae91253e978),
[input-cell semantics](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/61222717-6b5c-4896-abce-4e8e61648416),
and [Open XML SDK schema definitions](https://github.com/dotnet/Open-XML-SDK/blob/main/generated/DocumentFormat.OpenXml/DocumentFormat.OpenXml.Generator/DocumentFormat.OpenXml.Generator.OpenXmlGenerator/schemas_openxmlformats_org_spreadsheetml_2006_main.g.cs).
