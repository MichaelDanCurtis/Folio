# Folio Native validation

## Cross-workbook scenario Merge — 0.82 source and native qualification, 2026-09-20

All **405 Rust and 395 Flutter tests pass**, with two optional AI checks skipped
and clean analysis. The full Flutter suite uses the rebuilt 0.82 release engine.

Four new engine cases cover frozen read-only source loading, identical source and
target sheet IDs, source files changing after preview, scenario-only copying,
Undo/Redo, recovery and XLSX reimport. Invalid/closed/expired requests and protected
sources leave the target unchanged. Number-format tests use different custom codes
at source and destination ID 164, confirm the copied code receives ID 165, reuse
that ID for another copy and preserve the destination's format. A new workbook
also receives the portable custom format. Long UTF-16 and Unicode-case duplicate
names now copy with valid unique suffixes rather than failing at the name limit.

Three dialog cases check external selection even with a matching worksheet ID,
failed-copy retry, canceled source pickers, returning to This workbook, and stale
asynchronous completion without writes. The real Flutter/Rust application test
uses the file-picker callback to read Sales.xlsx. Cancel preserves the workspace;
copy changes only the target scenario collection and one revision, retaining the
current result 2. Show applies 12×25=300; Undo returns 2 and Redo returns 300.
Recovery/export/reimport retain the scenario, document, formula and watch; source
bytes are unchanged. The independent verifier checks complete workspace changes
and the exported source/target cells and scenario values. Seven XLSX files pass
Microsoft 365 schema validation, including custom-format collisions/new workbooks.

The first application-test run used an ambiguous dropdown finder; scoping it to
the merge dialog fixes the test harness. All eight focused dialog/application
tests pass.

The saved 0.81 preview quit normally, with zero processes confirmed before one
0.82 candidate launched. Native file selection loads a donor with a custom
0.0000 format at ID 164 into a destination whose ID 164 means 0.00000. Preview
and Cancel leave recovery byte-identical. Copy adds only the selected scenario;
Show changes A1/B1 from 10/20 to 7/9 and both formula results from 200 to 63.
Keyboard Undo restores 200 and Redo returns 63 while B1 keeps its five-decimal
display. A second copy gets Formatted (2); copy Undo/Redo changes only the scenario
collection. The summary reports 63/63/63 with destination formatting retained.

In-place Save and independent export produce identical XLSX files. Normal quit,
a zero-process check and reopening the same candidate preserve recovery bytes.
Reimport retains both scenario definitions, comments, protection flags, input
values, formulas, summary formatting and frozen panes. Inspecting the second
definition and canceling leaves recovery unchanged. Fifteen checkpoints pass the
independent whole-workspace/history comparison. It includes the existing automatic
fitting of both recalculated rows to 24 pixels; no unrelated content changes.
ZIP/XML checks confirm copied format 164 is remapped to 165, destination 164 is
retained, both copies reuse 165, and the donor bytes remain unchanged. All three
native XLSX files have zero Microsoft 365 schema errors. Screenshots of source
selection, reopened scenario values and the final report were inspected.

Evidence is frozen in release/validation/scenarios-merge-native. The 405/395
passing runs were not repeated during native checks; production and test source
hashes still match those runs. Only documentation and the native verifier were
added afterward. The earlier development archive, normal user recovery and old
preview recovery remain unchanged. Exactly one saved 0.82 preview is running.
Direct Microsoft Excel execution, broader native error/protection/large-workbook
cases, full parity and the later 20 novel AI features remain unfinished.
See docs/SCENARIOS.md, scripts/verification/verify_scenario_merge.py and
scripts/verification/verify_native_scenario_merge.py.

## Scenario Manager — 0.81 source and native qualification, 2026-09-20

All **401 Rust and 391 Flutter tests pass**, with two optional AI skips and
clean analysis. The final full runs include the shared font-export correction.
Four dialog cases and a real Flutter/Rust application case also pass in a
focused run against the rebuilt release engine.

The scenario cases cover named input CRUD/copy, unchanged calculations on Save,
Show/Undo/Redo, invalid/stale requests, 32-cell limits, duplicates, cancelled
row-measurement staging, data validation, explicit formula replacement, static
summaries, typed errors, imported unknown metadata and escaped text, structural
insert/delete, protected sheets and empty recovery without resurrection.

The real application test uses Inputs A1/B1 and a cross-sheet dependent formula.
It creates Base (10×20) and Growth (15×30), renames Growth to Upside, cancels a
999 draft, fails row measurement without mutation, retries Show successfully,
undoes/redoes and creates a static report with current/Base/Upside = 450/200/450.
Recovery and XLSX reimport retain scenario definitions, formula and watched cell.
An independent ZIP/XML/recovery verifier confirms these values and four history
steps across Show, Undo, Redo and summary creation.

Nine regenerated files pass Microsoft 365 schema validation. The first check
found that the existing shared font exporter appended a new bold property after
the font name. New font properties now follow the schema's order; a separate
regression keeps the Normal font unchanged and tests combined font properties.
The summary files and the new font regression now validate. Metadata-extension
fixtures are checked for preservation separately from the valid-schema corpus.

Dialog tests found and fixed overflowing protection labels and a short command
column; labels wrap and the commands scroll. A rendered Scenario Manager image
is inspected with readable commands.

The saved 0.80 preview quit normally before the prepared 0.81 build launched;
zero and one processes were verified at the transition. Native Scenario Manager
imports Base/Upside and cancels an Add draft with byte-identical recovery. Adding
Downside (5,8) changes only scenario metadata. Show changes the result from 450
to 40; keyboard Undo returns 450 and Redo returns 40. Renaming it Steady and
editing its second input to 12 still changes only metadata; Show then returns 60.
Summary creates Scenario Summary 2 with current/Base/Upside/Steady results
60/200/450/60, preserving the earlier report. Merge copies Base and Steady to
Result without overwriting its cross-sheet formula. Deleting copied Base,
keyboard Undo and Redo change only that scenario collection.

In-place Save and the independent XLSX export are byte-identical. After normal
quit, a zero-process check and restart, recovery remains byte-identical. Reimport
retains all scenario definitions, the formula and watched cell, both reports,
summary formatting and frozen panes. Applying imported Base returns 200; Undo
restores 60, and the preview is left saved. Eighteen native snapshots pass the
independent verifier's whole-workspace/history/content checks; both native XLSX
files have zero Microsoft 365 schema errors. Screenshots of the native dialogs
and report were inspected and remain in the conversation. Native formula
replacement acknowledgement was visible; accepting that replacement was covered
by automated tests, not this native workflow. No actual Microsoft Excel executed.

Evidence is frozen in release/validation/scenarios-native; the earlier development
archive remains immutable. The 401/391 passing runs were not rerun during native
qualification: all production/test sources still match those runs. Only docs and
the native verifier changed. The normal user and old preview recovery files are
unchanged. Exactly one 0.81 preview remains open. Direct Office comparison,
broader performance/protection/what-if cases, full parity and the subsequent
20 novel AI features remain incomplete.

See docs/SCENARIOS.md, scripts/verification/verify_scenarios.py and
scripts/verification/verify_native_scenarios.py.

## Goal Seek — 0.80 source and native qualification, 2026-09-20

All **391 Rust and 386 Flutter tests pass**, with two optional AI skips; analysis
is clean. Ten new Rust cases cover isolated linear/nonlinear searches, negative
inputs, domain errors, flat derivatives, discontinuous/unreachable targets,
tolerance-based convergence, a cross-sheet PMT equation checked independently,
token cancellation, Stop/best-value acceptance, stale/invalid requests, validation,
no-op history, pending-layout preservation, retry after cancelled measurement,
protected/merged/array inputs, hidden/filter rows and the 1904 calendar.

Four dialog cases cover validation, preview without mutation, Apply failure/retry,
Stop, explicit best-value acceptance, closing during Start, stale completion and
changed inputs. The real Flutter/Rust application test searches from Summary C1
into Inputs B3, previews/cancels without changing state, retries a simulated row
measurement failure, applies the result, undoes/redoes, saves recovery and
exports/reimports XLSX. Only the intended input changes from 20 to approximately
50; the original formula returns 500 within an absolute tolerance of 0.000001.
Documents, other inputs and watched cells remain unchanged. The independent
ZIP/XML/recovery verifier confirms these values and the three history revisions.

The dialog screenshot is visually checked, including readable numerical values
and full precision in tooltips. The 0.80 packaged source matches the passing
runs: 309 application/native/project files, 114 Rust files and six template
assets; the normalized packaged calculation engine matches the release build.

The 0.79 saved preview quit normally, the process guard confirmed zero, and one
0.80 preview launched. A disposable workbook uses Summary C1 = Inputs!B2*Inputs!B3
with B2 = 10. Native Goal Seek targets 750 by changing Inputs B3 from approximately
50 to 75. The first preview and Cancel leave recovery byte-identical. A second
search applies one history step; keyboard Undo returns 500 and Redo returns 750.
Excel export and in-place Save produce identical bytes. After a normal quit and
zero-process check, reopening the same candidate preserves recovery exactly.
Reimporting the export shows 750 and retains the formula and both watches.

Eleven captured native states pass an independent whole-workspace comparison;
only the chosen input changes apart from history and source metadata on reimport.
Both native XLSX files have zero Microsoft 365 schema errors. Three automated
XLSX files also pass schema validation. The native screenshots are retained in
the conversation. Native Stop/best-value, broader numerical/volatile/performance
cases and direct Microsoft Excel execution remain outside this native check.
Evidence is frozen in release/validation/goal-seek-native. Normal user recovery
and the old preview recovery remain unchanged. Exactly one 0.80 preview is open.
X14, full Office parity and the subsequent 20 novel AI features remain incomplete.
See docs/GOAL_SEEK.md, scripts/verification/verify_goal_seek.py and
scripts/verification/verify_native_goal_seek.py.

## Watch Window — 0.79 source and native qualification, 2026-09-20

All **381 Rust and 381 Flutter tests pass**, with two optional AI skips; analysis
is clean. Six new Rust cases exercise watch persistence, history, structural
coordinates, stale/invalid request atomicity, OOXML metadata preservation,
unchanged import export bytes, legacy recovery hydration, intentional removal
without resurrection and blank edge cells. Three widget tests and a real
Flutter/Rust application scenario check navigation, live values, column/panel
resizing, multiselection, retry, delayed failures, watch editing/history,
cross-sheet inspection, row edits, recovery and XLSX export/reimport.

Testing found two issues that are corrected in the final source: a row-wide
double-click detector delayed checkbox/button clicks, and recovery hydration
added empty watch metadata to files that never contained watches. Double-click
navigation now covers only the row details, and absent source collections leave
old files unchanged. The full passing runs include both corrections.

Independent ZIP/XML and recovery checks confirm watched Forecast C1 and Sources
B4, the shifted formula range B3:B5, inputs 10/25/30 and total 65. Five XLSX files
pass Microsoft Open XML SDK validation against Microsoft 365 schemas, including
shifted watches in new/imported files and removal of the final watch. The
widget-rendered panel is visually checked. Actual Microsoft Excel execution
remains pending. See docs/WATCH_WINDOW.md.

The prepared 0.79 build now passes native import of two watches, Go to Sources
B4, worksheet Edit cell, F8 range extension and Finish selection. Add watch
retains B4:B5 and adds only the missing B5. Deletion, keyboard Undo/Redo and a
final Undo restore three watches. Normal typed entry changes B4 from 25 to 26
and the watched total from 65 to 66. Typing 27 then Escape preserves the committed
26 and leaves recovery byte-identical; Undo returns 25/65. Formula inspector
opens Forecast C1, follows its first source cell, shows its direct dependent,
returns with Back and selects the exact Sources B3:B5 range.

Fourteen captured recovery states pass an independent verifier. Export and
in-place Save produce identical XLSX bytes. The only semantic XML change from
the original workbook is the added Sources B5 watch; the formula cache gains
an equivalent namespace declaration. Normal quit with Save, restart and XLSX
reimport retain all three watches and original cells/formulas. Restart recovery
is byte-identical. Both native XLSX outputs pass Microsoft 365 schema validation.
CUA accessibility value-setting did not exercise the normal Flutter draft
callback; the accepted entry/cancellation checks use ordinary typed input.

App hygiene is verified: the saved 0.76 preview was quit normally, zero processes
were confirmed, then 0.79 was launched as one process. The restart guard detected
the Save prompt and did not launch another copy; Save completed the quit, zero
was confirmed again and only then was 0.79 reopened. Exactly one preview remains.
The prior preview recovery and normal user workspace hashes are unchanged.
The 0.79 package matches 308 application/native/project files, 112 Rust files
and six template assets, and passes strict/deep signature and normalized-engine
checks. ZIP SHA-256: `1032b385656f365424ecb059fbda76869136a49fee8a7ab3c609914ea566042a`.
Evidence is archived under `release/validation/watch-window-native`.
The normal release is not promoted or notarized. Native document-table 0.77
controls, broader Watch Window behavior, direct Excel comparison, X10, overall
Office parity and the subsequent 20 novel AI features remain incomplete.

## Formula inspector — 0.78 source qualification, 2026-09-20

All **375 Rust and 377 Flutter tests pass**, with two optional AI skips; analysis
is clean. Seven new Rust cases cover direct references, sparse ranges, table
columns, scope guards, history/cache invalidation and staged-layout preservation.
Three dialog tests and a real Flutter/Rust workspace scenario verify paging,
inspection/back, cross-sheet range navigation, pending-draft commit, stale
responses, failed-read retry, undo, recovery and XLSX export/reimport.

The real workspace test exposed an exact-range navigation defect: the final
selection had only extended when physical Shift was held. The new caller now
passes explicit range intent. The passing full suite includes that correction.
Test screenshot capture uses the asynchronous renderer correctly; an initial
capture-only wait and a fake-clock settle issue were corrected in the harness.

The disposable two-sheet workbook's three formulas and cached total of 60 pass
an independent ZIP/XML reader, with restored input values 10/20/30 in both XLSX
and recovery. Microsoft Open XML SDK reports no Microsoft 365 schema errors.
The rendered widget image shows the formula, calculated value, precedents,
dependent count and unresolved-reference notice. It is not native desktop or
Microsoft Excel execution evidence. See docs/FORMULA_AUDITING.md.

The existing 0.76 preview remains the sole running Folio instance. Native checks
for 0.77 and 0.78 remain pending while the Mac's desktop cannot be inspected.
No replacement preview is launched until the existing app's saved state can be
checked and it is quit normally. No parity family or novel AI feature is accepted.

The isolated 0.78 package passes strict/deep signature checks and matches 307
application/native/project files, 110 Rust source files and six template assets.
The packaged engine matches the release engine after signature normalization.
ZIP SHA-256: `96cc89c821c9386ed9dcc226ef45596ddb6c3fd107bc4ce425419850ff6ff218`.
The prepared candidate has not been launched. The old preview and normal user
recovery hashes are unchanged. Evidence is archived under
`release/validation/formula-audit-development`; the normal release is not
promoted or notarized. Desktop inspection of the existing process returned
`cgWindowNotFound`; the process guard still confirms exactly one live preview.

## Desktop cell actions — 0.77 source qualification, 2026-09-20

The worksheet Cell actions menu exposes ordinary desktop actions for editing,
extending a range and selecting the active cell. F8 toggles range mode; Escape
and Finish selection retain the range and end the mode. Pointer, accessibility
activation and arrows share its anchor, including hidden-row navigation. The
table Edit cell button collapses a range to the active cell and focuses the
existing rich-text draft. Open menus and delayed selections reject a changed
worksheet/workspace, including a scope check between cell and formula commits.

All **373 Flutter tests** pass with two optional AI skips, and analysis is clean.
The final run includes the table active-cell collapse adjustment. Five new tests
cover the visible controls, keyboard/mouse/accessible ranges, failed-save retry,
stale menus, preservation of a new-sheet formula draft while an old commit
finishes, and rich-editor focus. They pass together with the four preceding
selection cases. Rust implementation is unchanged; its prior 368-test result
is not a new Rust run. See docs/CELL_ACTIONS.md.

The Mac locked during preview inspection. Native verification remains pending;
the existing 0.76 preview is kept untouched until its saved state can be checked.
The 0.77 package is prepared separately. Native custom-action dispatch, full
VoiceOver/platform coverage, noncontiguous selection and actual Office
comparison remain unfinished. No parity family or novel AI feature is accepted.

The completed package matches 306 application/native/project files, 108 Rust
source files and six assets. Strict/deep signatures and the normalized engine
comparison pass. ZIP SHA-256:
`1635e55002cc8f8875d582e3ada15fa8302bcb5d9e779f24020bcfaa20b4752d`.
The prepared candidate is not launched; the single running 0.76 preview and its
recovery remain unchanged. Evidence is archived under
`release/validation/cell-actions-development`. The normal release is not promoted
or notarized. A successful build does not establish native UI qualification.

## Cell selection — 0.76 native qualification, 2026-09-20

The isolated 0.76 package matches 306 Dart/Swift/entitlement/project files,
108 Rust source files and six assets. ZIP and strict/deep signature checks pass;
the normalized packaged engine matches the release build. ZIP SHA-256:
`13b6591fd711e588aaf7d60fab6a4e167e265c009466dff332547fb5edd3c7ab`.

The old 0.75 preview again reproduced the unintended rows 1–3 selection after
Command-Shift-E, Cancel and accessible activation of row 3. In 0.76 the same
steps select one cell and Sort proposes the whole table (rows 1–5). Enabling
Select cells then activating row 5 correctly proposes rows 3–5. Both drafts
are cancelled. Worksheet post-Export activation changes B4 to C5 alone;
Shift-Left selects B5:C5 and Right restores C5. F2 opens the selected cell,
typing changes its draft from 4200 to 4201, and Escape restores 4200 at the same
revision. Native accessible labels retain addresses and cell contents.

All five fixture snapshots (imported, table cancelled, worksheet cancelled,
exported and restarted) are byte-identical, revision 1. Source and native Word
export are byte-identical, SHA-256
`1a1a8b8303531020ca6a91750781dff5d209dd72a4fad846f7fadb4d3b7bd822`.
`scripts/verification/verify_native_cell_selection.py` verifies those comparisons.
The underlying source was schema-qualified in the preceding 0.75 fixture run;
no new Microsoft Word execution or broader compatibility claim is made here.

Replacement and restart each use normal Quit, verified zero processes before
launch and verified one process afterward. The normal user's workspace and the
closed 0.75 fixture recovery remain unchanged. The source/evidence archive is
`release/validation/cell-selection`; the normal release is not promoted.

Custom Extend selection/Edit cell semantics actions pass widget tests, but the
native AX tree does not expose them. The installed Flutter desktop delegate's
action dispatcher handles default activation, focus and show-on-screen; its
other-action fallback returns false. Native custom-action delivery remains
unfinished. Native delayed-save timing, all modifier/focus/IME paths, VoiceOver
navigation, complete accessibility and actual Office comparison remain open.
No full parity family or novel AI feature is accepted.

## Cell selection — 0.76 source verification, 2026-09-20

All 368 Flutter tests pass, with two optional AI checks skipped, and analysis is
clean. Four new focused checks cover accessible single-cell and explicit range
selection, real Shift-click, table Select cells mode, editing/cancelling a cell,
rejected-save retry, and preserving Shift across a delayed commit. The final
focused run strengthens the delayed-commit case to use a real pointer tap and
checks address/content semantics. Rust and native keyboard implementation are
unchanged; their previous 368-test Rust result is prior evidence, not a new run.

The 0.75 native reproduction distinguishes stale keyboard state on AX activation
from the correctly reconciled real-pointer route. See
`docs/CELL_SELECTION_ACCESSIBILITY.md`. The subsequent native checks are above.
Full accessibility, IME, all modifier/focus transitions, actual Office comparison
and the full parity/20 novel AI goal remain incomplete.

## Document table sorting — 0.75 native qualification, 2026-09-20

The isolated 0.75 package matches 306 application/native/project files, 108 Rust
source files and six assets against the tested development implementation.
Only package version metadata changed. Native multi-key Amount/Name/Date sorting,
heading retention, draft Cancel, moved-cell typing, one-step Apply, ordinary and
post-Save keyboard Undo/Redo, and bookmark/comment Go To pass. Swedish/German
accented-name previews pass; the disposable locale draft is cancelled.

Sixteen fixture checkpoints pass `scripts/verification/verify_native_table_sort.py`.
Complete rows and annotation messages survive, unrelated workspace state remains
unchanged, and restart restores exact recovery bytes. Four native DOCX outputs
(Save As, recovered Save As, direct-mouse toolbar Export and post-reimport native
shortcut Export) are byte-identical. The independent reader verifies row properties,
annotation endpoints, comment paragraph links and other package parts. Five native
source/output files pass Microsoft365 Open XML validation. JSON reimport comparisons
normalize only regenerated XML positions and equal-format text-run boundaries.

The old preview quit normally, zero processes were verified before launch and
restart, and exactly one new preview remains. Its saved fixture is revision 7;
the normal user workspace hash is unchanged. The archive is
`release/validation/document-table-sorting-native`; the normal release is not
promoted or notarized. Full suites remain 368 Rust/364 Flutter with two optional
AI skips; this metadata-only packaging turn does not claim a new full-suite run.

Input limits are retained in `docs/DOCUMENT_TABLE_SORTING.md`: accessibility-opened
unfocused Export panels dismissed during automated input, while direct mouse and
native shortcut Export completed. Some post-shortcut table clicks unexpectedly
extended selection until a Shift-arrow pair reset it. The cause is unverified.
Automated typing omitted an initial accented character; clipboard input supplied
the exact sample. Broader modifier/focus, IME, accessibility, large-table, complex
annotation and actual Microsoft Word qualification remain open. No entire parity
family or novel AI feature is accepted.

## Document table sorting — source development after 0.74, 2026-09-20

The table editor now offers three sort keys with Text/Number/Date, ascending or
descending order, heading/range choices, field delimiters, case and language.
Foundation supplies collation ranks, preserving ties for secondary keys. Decimal
comparison avoids large-integer rounding. Complete rows move in a draft; Apply
is one workspace step, and the focused cell follows its row. Newly created,
inserted and converted rows have durable identities. Identified row permutations
carry bookmarks/comments, rebasing local text edits before adding the new row
offset. Reversed cross-row endpoints fail without publishing a corrupted range.
Unchanged rows retain full source XML when their geometry permits reuse.

All **368 Rust tests** and **364 Flutter tests** pass (two optional AI checks
skipped). Seven new Dart tests and one real-core UI workflow cover ordering,
types/locales, exact numbers, ranges/headings, full-row state, field boundaries,
merges, late preview responses, Cancel/Apply, typing after movement, annotated
row changes, exact Undo/Redo, unrelated data, DOCX export/reimport and recovery.
Three new Rust tests exercise range relocation and source row XML preservation.
Native Foundation checks cover accent equivalence, case, Swedish/German/Turkish
and empty/multiple columns. Analysis is clean. A final assertion confirms a
manual break remains inside its paragraph sort field; the focused suite passes.

scripts/verification/verify_document_table_sort.py independently verifies actual
Word row order/properties, bookmark/comment endpoints, author/text, linked
comment paragraphs and semantically unchanged other package parts. Sorted and
recovered DOCX are byte-identical. Three app artifacts and four engine artifacts
pass Microsoft365 schema checks. A missing required shading attribute in the
new engine fixture was corrected, then its row-retention test and schema check
were rerun; no production code changed after the full suites.

The source/evidence archive is release/validation/document-table-sorting-development.
At the end of that source-development run, the existing 0.74 preview remained
unchanged. The subsequent 0.75 native qualification is recorded above; broader
IME/accessibility/performance and actual Word comparison remain pending. See docs/DOCUMENT_TABLE_SORTING.md
for unimplemented sorting options and complex annotations/structures. The full
parity goal and the subsequent 20 novel AI features remain incomplete.

## Editable manual line breaks — 0.74 native qualification, 2026-09-20

The isolated 0.74.0 package matches the tested implementation: 302 app source
files, 108 Rust source files and six template assets. Strict/deep ad-hoc signature
and normalized engine comparisons pass. The package ZIP SHA-256 is
`a48952e336ac903b940caf26218ed6205e8d5853d38a98b47756af9717f9ea33`.
Only version metadata changed after the 365-Rust/356-Flutter run (two optional
AI checks skipped). This is not a notarized or promoted normal release.

Native input checks pass for imported body/table paragraphs containing breaks,
Shift+Enter, cell toolbar insertion, ordinary Enter, and spelling correction on
the third visual line. The spelling menu has a divider above editing commands.
Clicking received replaces the misspelling; keyboard Undo/Redo restores both
states. Cell and header Cancel leave recovery bytes unchanged; outer Apply,
immediate keyboard history and post-Save As history preserve complete workspace
state. Page preview shows all body/cell/header lines without changing recovery.

The independent verifier in scripts/verification/verify_native_line_breaks.py
passes all 29 checkpoints, retaining paragraph identities/properties and every
unrelated workspace key. Working Save, initial export, restart export and
native reimport export are byte-identical (SHA-256
`f6ae995eeba9555af2a2b107cf645971951b43a9fbd4d1caa58303e8f3b2d1b4`).
All four outputs and the source pass Microsoft365 Open XML schema validation.
Reimport retains the exact document model and its editable body/cell/header.
Final saved recovered.docx is revision 31, recovery SHA-256
`634faa7f3a58464332daa5a6603718032dda262ffb1d072082c43d555146c3cb`.

Previous-preview replacement and restart each used normal Quit followed by
verified zero/one process guards. The final guard confirms one preview. Normal
user recovery and the closed 0.73 recovery are unchanged. A temporary missing
window observation did not trigger another launch; the same process subsequently
completed the final native export. Evidence is archived under
release/validation/manual-line-breaks-native; the earlier development archive
remains byte-for-byte unchanged. Input/visual observations are in the task.

Native coverage is ordinary English left-to-right body, one table cell and a
default header. Native list numbering, mixed formats, repeated/edge breaks,
input methods, formal accessibility, external clipboard and direct Word
comparison remain pending. No full parity family or novel AI feature is accepted.

## Editable manual line breaks — source qualification after 0.73

The implementation in `docs/MANUAL_LINE_BREAKS.md` passes **365 Rust tests** and
**356 Flutter tests**, with two optional AI-provider checks skipped. Seven new
focused editor checks cover break metadata/marks, literal Unicode separators,
exact journals, rich-copy content, leading-break search/bookmarks, keyboard and
toolbar insertion, list paragraph identity, rendered lines/caret positions,
Backspace/Undo/Redo and proofreading underline/correction coordinates. One new
real-Rust app workflow exercises body, table-cell and header edits, Cancel,
separate workspace history, preserved workbook data, DOCX reimport and recovery.
Two new engine checks retain run formatting, page/column/wrapping metadata,
hyperlinks, genuine break elements and literal U+2028 text.

`scripts/verification/verify_line_breaks.py` independently reads the UI-produced
source, edited and recovered DOCX files: one body paragraph, one cell paragraph,
two original breaks, four edited body/cell breaks and one header break. Edited
and recovered exports are byte-identical. All three application files and both
engine files pass Microsoft 365 Open XML schema validation. This does not run
Microsoft Word. Logs and reports are in `../../work/line-breaks-*`.

At the source-qualification checkpoint, 0.73 was the sole running preview,
PID 40025. Its saved
row-flow fixture was not edited or relaunched during source qualification.
Package and native ordinary-input follow-up is recorded above; input methods,
formal accessibility, external clipboard and actual Word comparison remain
pending. Full parity and 20 novel AI features remain incomplete.

## 0.52 paragraph formatting — 2026-09-19

`docs/PARAGRAPH_FORMATTING.md` describes the controls, inheritance and remaining
layout limits. Document and table-cell dialogs edit positive left/right indents,
paragraph spacing and automatic/exact/minimum line spacing. Formatting retains
characters, rich marks, source identities and tabs; new paragraphs carry their
settings forward. Mixed start/left indent representations resolve per paragraph.
New Folio paragraphs retain their current spacing on an indent-only edit.

- **221 Rust tests passed**, including six new paragraph tests for inherited
  styles/defaults/numbering, cycles, native twip units, nested cells, alternate
  namespace prefixes, retained unknown attributes and unrelated ZIP parts,
  property order, invalid/overflowing requests, clear-direct edits, inserted
  styles/headings, legacy hydration, save/load, reimport and byte-identical Undo.
- **202 Flutter tests passed; two optional provider checks skipped**. New tests
  measure indent/wrapping/caret/hit-test geometry, spacing collapse across plain,
  aligned, heading and list blocks, exact/minimum line metrics, table previews,
  tab origins, rich codec metadata, Enter and multiline insertion. The real Rust
  app flow covers input validation, Cancel, Apply/Undo/Redo, selection restoration,
  stale previews, table drafts, recovery and export. Analysis is clean.
- Independent python-docx/lxml checks validate **10 DOCX artifacts** from the
  engine and application flows. This includes rich text, source styles, tabs,
  unknown XML, table paragraphs, new headings/native paragraphs, unchanged parts
  and exact Undo/reimport bytes. These checks do not run Microsoft Word.

Native testing of the first 0.52 package verified the Paragraph dialog at the
1152×768 desktop size, typed indents/spacing, Double selection, single-step
keyboard Undo/Redo, Cancel, sandboxed DOCX export and byte-identical restart.
The independent reader confirmed 72-point left indent, 18-point after spacing,
double lines, retained bold/italic text and existing table formatting.
Table paragraph edits stayed drafts until the outer Apply committed one revision.

That native check exposed lost document focus after closing a table dialog:
Command-Z did nothing until focus returned to the editor. The final source
restores the containing editor's focus (including nested table editors), and the
app regression now sends real keyboard shortcut events after table Apply and
waits for the resulting workspace revision. The rebuilt **0.52.0 build 2** also passed native keyboard Undo/Redo immediately
after table Apply. The table gap changed from 24 to 30 points at revision 5;
Command-Z restored the prior document at revision 6, and Redo restored the
change at revision 7. Native export retained that 30-point gap and the document's
earlier formatting. Quit/relaunch preserved all saved bytes at revision 7.
The independent native-artifact verifier passes on both exported DOCX files.

The final ZIP is **141,542,182 bytes**, SHA-256
`9857ecd0709b686329a3c8b4aeeacfafcf10ca1e146c2806ac6c8848f5641cd8`.
Strict/deep signatures passed after extraction; all **270 Dart/Swift source
files** matched staging, and the packaged Rust engine matched the release
binary after normalizing temporary-copy signatures. Three focused app workflows
passed again against the final packaged engine. The final full Flutter suite
passed 202 tests with two optional provider skips; analysis is clean. This is
a locally signed build, not a notarized distribution.

Source, fixtures, passing logs, screenshots, independently read exports and
native saved snapshots are archived in `release/validation/paragraph-formatting`.
The isolated **Folio Paragraph Check** app remains open at revision 7. The
user workspace remains unchanged at revision 30. A normal 0.48 app process was
observed separately; this validation did not relaunch or replace that workspace.

First-line/hanging/negative/character-unit indents, line/automatic/contextual
paragraph gaps, complete style-change resolution, bidi/document grids,
pagination, broad native IME/accessibility/performance qualification and direct
Word comparisons remain incomplete. **No whole parity family is accepted;
novel AI remains 0/20.**

## 0.51 Paste Special properties — 2026-09-19

`docs/PASTE_PROPERTIES.md` describes the five new options, source references,
file behavior and limits. Number-format transfer retains other destination XF
components; validation-only and width-only pastes retain cell contents. Source
snapshots, inherited formats, custom format IDs, validation anchors, dimensions,
measured automatic rows and single-step history use the existing transactions.

- Full Rust suite: **215 passed**. Eight added tests cover both number-format
  modes, formula/error/text types, immutable copies, transpose/tiling/blanks,
  effective XF inheritance and built-in IDs, validation replacement/reanchoring,
  missing sheet references, width defaults/tiling/history/recovery, rejected
  requests, staged row measurement and all five modes on an openpyxl input.
- Full Flutter suite: **193 passed; two optional provider checks skipped**.
  The new real-worker application test drives all five dialog choices, verifies
  destination formatting, visible column extents, Undo/Redo, Cancel, atomic
  row-measurement failure, XLSX export and saved-workspace recovery. Analysis
  reports no issues. The native clipboard channel is mocked at its text/token
  boundary; these results do not establish native OS clipboard interaction.
- Independent openpyxl/ElementTree readers passed on **13 XLSX artifacts**.
  These include imported borders, alignment, validation prompts, conditional
  rules, hidden/outline column metadata, formulas/caches and recovery. Direct
  Excel execution is not established by these checks. The reader passed again
  after the packaged engine produced the application-flow export.

The macOS build and freshly extracted ZIP passed deep/strict signature checks.
All 268 Dart/Swift source files matched the build stage. The packaged engine
matched the current release binary after normalizing signatures on temporary
copies. Four focused application tests passed with the freshly extracted
package's engine: Paste Special properties, conditional-copy, clipboard and fill.
ZIP SHA-256:
`9f0c80727dd61ecdadefa15393a81d1ea814859b33f144e1fbdbd35d59fcdcd1`.
Source, fixtures, passing logs and independently read files are archived under
`release/validation/paste-properties`. The existing 0.48 app/server processes
remain running and the user's recovery file retains its recorded hash.

At package time, native qualification awaited desktop unlock. Later on
2026-09-19, 0.51 launched with the user workspace at revision 30 unchanged and
Connected to Codex visible. This establishes startup/recovery/connection only;
native Paste Special interaction remains unqualified. Direct Excel differential behavior,
cross-font width interpretation and the remaining Paste Special choices still
need work, along with the rest of the Word/Excel parity ledger.
No complete parity family is accepted; novel AI remains **0/20**.

## 0.50 conditional-format copy, paste and fill — 2026-09-19

`docs/CONDITIONAL_COPY.md` describes behavior, source references and limits.
Conditional rules now follow Paste All, Paste Formats and directional Fill.
Paste Special adds Cells and merged conditional formats. Replacement subtracts
only written destinations; merge preserves their rules. Copied rules precede
retained rules, keep source ordering and Stop if true, and recalculate aggregate
populations over the copied ranges. Relative/mixed formulas are reanchored;
graphical-rule identifiers are reserved across the entire workbook.

Executed evidence:

- Full Rust suite: **207 passed**. Nine new tests cover replacement and merge,
  all paste modes, overlap/snapshot identity, blanks, tile/transpose mapping,
  cross-sheet and missing-sheet references, four fill directions, sparse range
  subtraction/reanchoring, aggregate populations, graphical formula thresholds,
  global extension identifiers, Stop if true, opaque-rule boundaries, history,
  save/recovery and all eighteen modeled rule types from an independently
  produced openpyxl workbook.
- Full Flutter suite: **192 passed; two optional provider checks skipped**.
  The added complete application flow uses the actual Rust worker and native
  clipboard adapter contract. It drives copy/paste keys, visible copied icons,
  Paste Special merge, fill, Undo/Redo, rejected row measurements, recovery and
  Excel export. Existing proofing, document, bookmark, date, formatting and
  clipboard regression tests passed. Analysis reports no issues.
- Independent openpyxl/ElementTree readers validate **18 exported workbooks**:
  ordinary/range/formula/style semantics, all copied types, modes, fill, preserved
  opaque metadata, graphical thresholds, unique IDs/fallback links across sheets,
  recovery and the complete editor-flow export. This is not Excel execution.

The macOS 0.50 build and freshly extracted ZIP passed deep/strict signature
verification. All 268 Dart/Swift source files matched the build stage; the
packaged engine matched the release binary after signature normalization on
throwaway copies. ZIP SHA-256:
`b5a2f12d811f4466e8addc1442b562391e5bae59c00d7129820df8fe01c0325c`.
Three focused application tests passed using that extracted package's engine,
covering conditional copy, clipboard and fill. Independent readers passed
again on all 18 artifacts, including the resulting editor-flow export.

The desktop controller still reported a locked Mac when asked to show Folio.
Native startup and interaction for 0.50 remain unverified; no new native
clipboard, gesture or visual result is claimed. The 0.48 app and its Codex
app-server remain running, and the user's recovery file is unchanged from
its recorded pre-launch hash. Source/test inputs, logs and independent
artifacts are archived under `release/validation/conditional-copy`.
Excel differential behavior,
rule consolidation, all aggregate/transpose/merge cases, Cut/move, Format Painter,
remaining paste options and the rest of the parity ledger remain incomplete.
No complete parity family is accepted; novel AI remains **0/20**.

## 0.49 date-occurring rules and local recalculation — 2026-09-19

Scope and primary references are in `docs/DATE_RULES.md`. The manager supports
all ten date periods, ordinary conditional styles, priority/Stop if true,
disjoint/full-column ranges and imported metadata-only rules. Style-only edits
preserve imported formulas; period/range changes construct a new predicate.
The supported structure, recovery and XLSX paths retain date rules.

TODAY/NOW and date predicates use one captured local clock per calculation.
Calendar/UTC-offset changes refresh derived values; NOW stays fixed within an
unchanged day until calculation. The Recalculate toolbar button and F9 commit
an active edit, then publish results and measured automatic rows together.
Calculation adds no Undo entry, clears no Redo history and advances no document
revision. Derived row-height changes are saved without a separate user edit.

Executed evidence:

- Full application engine suite: **198 passed**. Eight date/clock tests cover
  the ten periods, fractional times, leap and year transitions, week edges,
  both calendars, local midnight, DST/offset changes, rollback, independent
  model clocks, explicit NOW refresh, rule priority, imported formulas,
  missing formulas, staged row sizing, history and file/structural recovery.
- Full Flutter suite: **191 passed; two optional provider checks skipped**.
  Added coverage exercises calendar callback coalescing/retry/disposal and the
  actual native-worker rule dialog, Apply, toolbar/F9, unchanged history,
  Undo/Redo, saved recovery and XLSX export. Source analysis reports no issues.
- Vendored IronCalc suite: **2,229 passed; eight existing ignored tests**.
  A new controlled-clock test checks host-local versus named-zone time,
  early-1900 conversion, one-instant consistency and invalid input atomicity.
- Independent openpyxl readers: **eight XLSX files passed**, including new-rule
  output, moved references, recovery, two independently produced calendar
  fixtures and the complete editor-flow export. These validate file fields
  and cached values, not Microsoft's execution of the files.

The macOS 0.49 build and freshly extracted ZIP passed deep/strict signature
verification. All 268 Dart/Swift source files matched the build stage; the
packaged engine matched the release binary after signature normalization on
throwaway copies. ZIP SHA-256:
`8513355735a1dc81a4231c0e3b9b8079f9e79ed58d7a20a9d7fe2b0d8420fb99`.
Four focused tests, including the complete editor flow, passed again using the
engine from that freshly extracted package. Independent readers passed on the
resulting files, including frozen NOW caches after imported row insertion.

The attempt to open the isolated native preview reported that the Mac was
locked. Native startup/input/visual qualification of 0.49 remains pending an
unlock. The user-facing 0.48 app remains the last observed running build; its
recovery file still matches the pre-launch hash. The ZIP is at
`release/Folio Native.zip`; source/test inputs, logs and independent artifacts
are archived under `release/validation/date-rules`.
Physical sleep/wake/timezone changes and direct Microsoft Excel differential
checks remain unqualified. Calculation modes, iteration, complete volatile
function scheduling, locale/coercion and all remaining X07/X08/X19 requirements
remain incomplete. No complete parity family is accepted; novel AI is **0/20**.

## 0.48 conditional-formatting icon sets — 2026-09-19

Scope and primary references are in `docs/ICON_SETS.md`. The rule manager now
edits 20 icon styles, custom category mixtures including No cell icon, reverse
order, icon-only display, and Number/Percent/Percentile/Formula thresholds with
strict/inclusive comparisons. The engine derives icons from current numeric
values across the applied ranges, excluding text/Boolean/blank/error cells.
Icons respect their own priority channel and ordinary Stop if true, remain
separate from saved values/styles, and participate in Undo/recovery. Display
and AutoFit account for the icon while preserving accessible cell values.

Standard OOXML covers 17 styles; newer styles/custom mixtures use independent
Office 2010 rules with IDs/priorities. Custom threshold counts can differ from
the unused base style, including a NoIcons base. Supported structural edits
move range and formula references. Older opaque rules upgrade by original
source identity, without resurrecting deleted rules. Unmodeled extension
content is preserved; unsafe standard-to-extension metadata promotion fails
atomically. Full vendor-extension support is not claimed.

Executed evidence:

- `cargo test --manifest-path rust/Cargo.toml --lib`: **190 passed**. Seven
  new tests cover the catalog, whole-column/offscreen populations, numeric and
  percentile boundaries, strict equality, formula recalculation/errors,
  extreme finite values, custom/reverse/NoIcons, priority/Stop if true,
  undo/redo, structural references, recovery, opaque metadata and an independent
  openpyxl-produced 17-style import fixture.
- `FOLIO_CORE_PATH="$PWD/rust/target/release/libfolio_core.dylib" flutter test
  --no-pub`: **187 passed; two optional provider skips**. The new tests paint
  every glyph/category distinctly, check narrow cells/accessibility, edit an
  imported custom rule, and exercise the actual new-rule/custom picker/apply
  flow against the worker. They check icon-only values, live recalculation,
  Undo, column AutoFit, save/recovery and Excel export. Existing data-bar and
  document/bookmark regressions passed in the complete run.
- `flutter analyze --no-pub`: **no issues**.
- `scripts/verification/verify_icon_sets.py`: independent openpyxl and
  ElementTree readers passed on **ten exported workbooks**. Checks include
  standard versus x14 namespaces, all style names, thresholds/comparators,
  IDs/priorities, custom glyphs, NoIcons, icon-only settings, original values,
  edited openpyxl input and formula/range movement. Extension files are read
  without a potentially lossy openpyxl resave.
- The rendered `icon-catalog.png` was inspected against Microsoft's category
  shapes/order. This is an automated raster result, not a native-window check.
- The macOS 0.48 build and freshly extracted ZIP passed deep/strict code-signature
  verification. **267 Dart/Swift source files** matched the build stage. The
  packaged engine matched the release engine after normalizing signatures on
  temporary copies only. The ZIP SHA-256 is
  `87b0a964bad09986012ea96a2069092486c7127eec6b5c776cd24c89fc10585e`.

Five selected checks also passed with the **freshly extracted package engine**,
including two complete icon/data-bar workspace flows. The independent reader
passed again on the resulting exported icon workbook. Logs, files, the package
report and an integrity manifest are archived under `release/validation/icon-sets`.

The isolated icon-rule preview is prepared but **not launched**. Subsequently,
the user-facing 0.48 app was launched and raised on the unlocked desktop. Its
Word view and Connected to Codex status were observed; the process path/version
and unchanged recovery file were independently checked. This establishes startup,
not icon-rule native input or direct Excel comparison. The saved workspace hash is `3e899a047d3fd28e0926872a3cdbcde936ee11ae4c1e35e1e40b5bfc95369f22`.
Printing, icon sorting/filtering, copying/filling/cutting rules, pivot/table/locale
semantics and other X07 gaps remain. No parity family is accepted complete and
novel AI remains **0/20**.


## 0.47 exact bookmark tracking for rich paste and table drafts — 2026-09-19

The main editor now captures actual normalized Quill operations through a
separate synchronous read-only stream, retaining the original asynchronous
stream. Multiple operations within one notification are composed without a
text diff, then validated against both snapshots and final logical text. The
main editor skips capture when no bookmarks exist. Internal rich copy/paste,
external HTML paste, string replacements, formatting and `compose` are covered.

Table cells use the same capture and compose edits across cell switches.
Apply translates verified table-local operations to the parent document and
creates one workspace Undo step. It flushes final typing synchronously; Cancel
does not commit the draft. An old dialog cannot write into a replaced document,
including an identical-looking table. Structural table operations explicitly
invalidate the exact journal and retain the previous diff fallback. The first
new workspace regression exposed JSON key-order differences between Rust and
the editor; content comparison now ignores object-key order while preserving
array order and values. That failing run remains in the work log.

The full Rust suite passes **183 tests** at version 0.47; Rust behavior is
unchanged in this update. Analysis is clean. The full Flutter suite passes
**184 tests**, with two optional provider tests
skipped, against the release engine. Five new capture/composition cases and
one real-engine workspace workflow cover internal/HTML paste, multiple pending
operations, formatting-only changes, Unicode, table embeds, repeated letters,
multi-cell drafts, immediate Apply, Cancel, whole-marked-text replacement,
Undo/Redo, another-workspace rejection, saved recovery and DOCX export. The
existing spelling/grammar correction and Find/Replace regression still passes.

Independent ElementTree and python-docx reading verifies exact saved document
changes, all four bookmark ranges, paired/unique Word markers, table/cell text,
bold formatting, unchanged workbook/settings, and identical package entries
after recovery. Evidence is in `../../work/bookmarks-exact-files`,
`../../work/bookmarks-exact-independent-reader.json` and
`../../work/bookmarks-exact-flutter-tests.log`.

The native release compiles, with strict deep signature verification and a
fresh ZIP extraction. All 265 staged Dart/Swift files match current source;
the packaged engine matches the release engine after normalizing only the
signatures on temporary copies. Package SHA-256:
`91f00c3a588dcbedf81adfdd822a4088f4a711531b2b342a06d1457dbc9b68a4`.
The three full-workspace bookmark/paste/table flows also pass against the
dylib from that extracted signed package. Independent readers pass for those
new packaged-engine fixtures. Build/package/flow evidence is archived under
`release/validation/bookmark-editing`.

The desktop-control tool still reports the Mac locked. The 0.47 isolated
preview has not launched, and the user's running 0.45 app has not been
replaced. Its saved revision-26 workspace remains byte-for-byte unchanged.
Native keyboard/mouse/IME and performance
qualification, structural table tracking, nested navigation, other stories,
advanced ranges, draft-local Undo semantics, reference fields, cut/move and
cross-document bookmark copying still remain. No provider request or new novel
AI feature was run; no Word/Excel parity family is accepted complete.

## 0.46 Bookmarks and Go To — 2026-09-19

The Word ribbon now opens Bookmarks; Ctrl+G/F5 also open it. Add, Move and
Delete validate names and revision/workspace identity. Lists support name or
location ordering and hidden names. Go To selects the saved passage with focus
and scrolling; a bookmark in a top-level table opens the target cell editor.
Metadata changes and navigation do not create phantom content Undo entries.

Main-editor string replacements carry exact logical text operations into the
engine, including rapid edits before earlier saves return. Built-in writing
corrections, Find/Replace, AI proofreading and selection proposals also supply
exact operations. Proposal validation checks reconstruction before applying.
Unchanged document content avoids rebuilding bookmark indices during unrelated
workbook edits. The earlier body DOCX preservation/recovery work is included.

The complete suites pass **183 Rust tests** and **178 Flutter tests** against
the release engine, with two optional provider tests skipped. Analysis is clean.
Four new mapping tests cover logical/Quill offsets, Unicode boundaries, lists,
tables/opaque blocks, separate exact replacements, paragraph breaks, stale and
overlapping edits. One real-engine workspace test covers invalid names, Add,
rapid repeated-letter edits, opening/ending affinities, Go To without extra
history, stale dialogs, hidden names, sorting, Delete/Undo, Ctrl+G, DOCX export,
Folio recovery, exact table-cell selection, a real spelling correction and
replacement of a whole bookmarked occurrence through Find/Replace. The new
core proposal test verifies a repeated-text insertion and rejects a different
workspace. No provider request was run by these checks.

The final macOS build and strict deep signature checks pass. A fresh ZIP
extraction matches all 264 staged Dart/Swift files. Its engine matches the
tested release engine after normalizing signatures on temporary copies (the
packaging signature replaces the linker's signature). The package SHA-256 is
`bc868d65d5a1664c979091c479d5889936b721befcaa98559687a28eb0909984`.
Independent bookmark package reading passes again. Native launch and mouse/
keyboard qualification remain pending: the desktop-control tool reported the
Mac locked and could not launch the isolated preview. The existing 0.45 user
app remains running; its saved revision-26 workspace is byte-for-byte unchanged.
The 0.46 build is not yet qualified by native interaction or promoted as the
running user app. See `../../work/folio-0.46-package-verification.json`.

Other stories, advanced ranges, exact
rich-paste/table-draft/cut/move behavior, nested-table destinations, reference
fields and direct Microsoft Word comparison remain incomplete. Full W04 is
not accepted complete; no parity family or additional novel AI feature is
accepted by these scoped checks.

## Unreleased bookmark engine foundation — 2026-09-19

After the published 0.45 build, the engine gained validated bookmark ranges,
revision-checked add/move/delete operations, history/recovery, exact-edit
rebasing and a bounded diff fallback. Body DOCX bookmarks include table-cell,
cross-paragraph, point and hidden ranges. Export preserves untouched source
run properties and internal hyperlinks, checks generated markers, and rejects
loss of unsupported source bookmark records before writing a destination.

Twelve core tests cover the paths and limitations recorded in
`docs/BOOKMARKS.md`. The complete suite passes **182 Rust tests** and **173
Flutter tests** against the rebuilt release engine, with two optional provider
tests skipped. Rust formatting checks pass; the final release engine build
finished in 63 seconds. Independent ElementTree/python-docx checks verify three
generated DOCX packages, bookmark text and UTF-16 ranges, untouched package
parts, internal links and exact package-entry equality after recovery. Source,
edited and reopened fixtures plus the saved workspace are in
`../../work/bookmarks-core`; reports are in `../../work/bookmarks-rust-tests.log`
and `../../work/bookmarks-independent-reader.json`. The source, engine and
evidence hashes are recorded in `../../work/bookmarks-foundation-verification.json`.

This is not a packaged release or native bookmark acceptance. Bookmark/Go To
controls and exact edit wiring in the Flutter editor remain to be implemented.
Diff fallback alone is insufficient for repeated-text/cut/move fidelity; other
stories, advanced ranges and direct Microsoft Word comparison remain. The
user's running 0.45 app is separate from this development engine. No parity
family is accepted complete and additional novel AI remains **0/20**.

## 0.45 numeric column-width display — 2026-09-19

The engine now identifies the width behavior of the selected numeric format
section. Ordinary General numbers choose a shorter decimal or scientific
representation that fits the measured font width. Fixed numeric/date formats
show hashes when the full result cannot fit. Invalid dates fill the available
width with hashes. Full formatted text remains in accessibility and hover text;
formula editing, copying, calculations, format codes and exported values do not
use the shortened paint result. AutoFit measures the natural content.

The documented baseline is Microsoft's [General-format rounding](https://support.microsoft.com/en-us/excel/reset-a-number-to-the-general-format)
and [hash display for narrow columns and invalid dates](https://support.microsoft.com/en-US/Excel/how-to-correct-a-error).
Microsoft also distinguishes [stored numbers from their visible display](https://support.microsoft.com/en-us/excel/stop-rounding-numbers).
These sources establish the broad behavior, not exact thresholds or differential
Office qualification for this renderer.

Four new candidate/render tests cover decimal/scientific shortening, sign and
exponent carry, nonzero values not becoming zero, width changes, text scaling,
full semantics, numeric text/errors, hidden formats, accounting spacing/color,
invalid dates, Shrink to Fit and natural AutoFit. A real-engine full-workspace
case covers fixed/general/date/text/error/formula cells, the full formula bar,
copy, width changes, AutoFit, Undo/Redo, unchanged calculations/cells, XLSX export
and reopening. One new core case covers format-section and value-type
classification, including Text format. The complete suites pass **170 Rust
tests** and **173 Flutter tests**, with two optional provider tests skipped.
Analysis is clean. The release engine compiled in 55.75 seconds; the native
build's subsequent engine check was cached in 0.36 seconds.

In the signed native preview, the narrow fixed number `12345.6789` shows hashes,
the General result `1763.668414` displays `1764`, and the date shows hashes.
Selecting the fixed number exposes its full raw value in the formula bar.
Setting Column width to 30 stores OOXML width 30.625 and restores the full
numeric and date displays; the invalid date stays hashed. Native Undo/Redo
restores narrow/wide displays at revisions 8/9. After another Undo, the column's
context-menu AutoFit restores all valid values at stored width 18.5625. Undo
returns the complete original workspace at revision 12. Native Excel Save
exports `number-width-native.xlsx`. Quit exits; reopening retains identical
saved bytes and restores narrow displays. The second quit also exits.

Independent readers verify the automated and native XLSX files' full numeric
values, formulas/caches, date serials, text/error types, Shrink to Fit and widths.
The native reader additionally compares every field in six saved workspace
snapshots, including expected automatic row-sizing metadata, and exact restart
bytes. openpyxl warns about the existing missing default style and an unknown
extension; the readers never save these packages. Those warnings are not proof
of exhaustive style or Office-file fidelity.

The ZIP is 141,208,240 bytes, SHA-256
`80e18ce45df2f842e7d887f46cb8ddd753a5ad29cf863722a23eccef0082858b`.
Fresh extraction passes strict/deep signature verification, 262 Dart/Swift
source files match the build stage and the extracted engine matches the signed
stage. The launched copy also passes strict/deep verification. This remains an
ad-hoc local package, not notarized distribution. The main app runs 0.45 on
Project budget, shows Connected to Codex, and preserves the user's revision 26
workspace byte-for-byte. No new provider request was run. Evidence is archived
in `release/validation/number-width`.

Exact Excel decimal/notation thresholds, extreme narrowness, selected-border
width differences, locale/calendar cases and custom General sections with
surrounding literals remain unqualified. Wrapped/rotated numbers retain the
prior renderer; their clipping still needs work. Merged-cell rendering, numeric
automatic column growth and the other X04 gaps remain. No full Word/Excel
parity family is accepted complete; additional novel AI remains **0/20**.

## 0.44 text overflow — 2026-09-19

Ordinary worksheet text now paints across adjacent empty cells while each cell
retains its own value, selection, hit target and accessibility content. A sparse
row index finds nearby text owners, including owners outside the viewport. The
existing font renderer supplies shaping and styling; foreground paint stays
inside the regular/frozen viewport clips. Wrapped, shrinking and rotated text
keep their existing rendering. Typed errors are explicitly distinguished from
strings in the display projection. Imported merge ranges act as read-only
barriers and recover from older workspaces and supported structural changes.

Five new index/pixel tests cover occupied/formula-empty cells, sparse edge
addresses, hidden and frozen boundaries, left/right/center text, numeric text,
typed errors, merged barriers, wrap and offscreen owners. A full-workspace test
with the real engine covers editing a blank neighbor, history, XLSX export and
recovery. The new Rust regression covers merge projection, structure/history,
legacy recovery and export/reimport. The complete suites pass **169 Rust tests**
and **168 Flutter tests**, with two optional provider tests skipped. Analysis is
clean. The final locked release build was cached in 0.22 seconds. The native
keyboard and termination sources are unchanged from 0.43; their previous
dedicated results are not new runs for this version.

In the isolated signed macOS package, A1's long label, A3's formula-returned text
and A4's numeric text visibly cross empty cells; A2's division error remains in
its own cell. Clicking visible A1 lettering inside B1 selects blank B1 and
shows an empty formula bar. Typing `x` saves revision 6, clips A1 at B1 and
leaves C1 empty. Native Undo/Redo/Undo produces revisions 7/8/9 with the expected
visible overflow and saved values. Native Save exports `overflow-native.xlsx`.
Quit exits; reopen retains identical recovery bytes and restores visible
overflow. The second quit also exits. An independent reader checks every field
in five saved workspace snapshots against the seed, including the automatic
row sizing introduced by the edit, and checks the native XLSX values and types.
Separate automated exports retain text-formula and error caches, numeric text,
blank neighbors and a rebased merged range. These readers load but never save
the exported packages; the openpyxl unknown-extension warning does not modify
the files.

The ZIP is 141,192,567 bytes, SHA-256
`ff5a69c88b5a6e7271456856e2b9c70100a4e9356361680f6a8d21299952c13e`.
Fresh extraction passes strict/deep signature verification; 261 Dart/Swift
files match the build stage and the extracted engine matches the signed stage.
The launched copy was rechecked with strict/deep signature verification. This
is an ad-hoc local package, not notarized distribution. The main app now runs
0.44, shows Connected to Codex and displays the Project budget worksheet.
The user's revision 26 workspace is byte-identical to its pre-upgrade backup.
No new provider request was run. Scoped evidence is collected in
`release/validation/text-overflow`.

X04 remains incomplete. Collision handling partitions overlapping labels and
still needs direct Excel comparison. Table areas and AutoFilter headers remain
clipped; rich/bidi edge cases, fills/borders and exact glyph/gridline geometry
need differential qualification. Merged-cell editing/rendering and
width-dependent numeric display are still missing. No Word/Excel parity family
is accepted complete; additional novel AI remains **0/20**.


## 0.43 continuous entry and quit — 2026-09-19

Cell entry now retains subsequent typing while a previous cell is saving.
Queued keys carry their original modifiers, each accepted cell has a separate
Undo step, and navigation waits for the accepted model revision. An outgoing
editor's blur cannot save the next unfinished draft. Failed entries pause later
typing with retry, skip and discard actions; a replacement worksheet cannot
receive queued input from the previous one. Flush retains the text client and
rechecks for text arriving while the last draft is being saved.

Fourteen focused widget cases cover those paths, delayed writes, cell/formula
focus transitions, pointer navigation, captured modifiers, grapheme caret edits,
worksheet replacement and late text during flush. The real-engine workflow
covers three continuous values, recalculation and three individual keyboard
Undo steps. The complete release-engine Flutter suite passed **162 tests**, with
two optional provider tests skipped; analysis is clean. The final native-only
changes additionally pass **47 in-memory AppKit keyboard checks** and lifecycle
checks for repeated quit, save failure/retry and synchronous completion.
Rust is unchanged from 0.41 except version; its prior 168-test result is not a
new run. The final locked release build was cached in 0.19 seconds.

Native testing exposed two additional issues: an initial package saved `4`
after typing `43` immediately before quitting; a later package retained `43`
but missed Command-Q during editor replacement. Native menu Quit saved and
exited, distinguishing that second failure from a proven save deadlock. The
final window routes Command-Q through the native menu before asynchronous text
dispatch. The quit coordinator cancels the first termination request, allows
saving on the normal event loop, coalesces repeated requests and quits again
only after success. This follows the protocol in the installed Flutter
`FlutterAppDelegate.mm`, while retaining Folio's save-error handling.

The final isolated package passed `42 Tab 43 Command-Q` and exited at revision
15 with both values. Reopening preserved the exact recovery bytes. Continuous
`60 Tab 70 Tab 80 Return` saved A3/B3/C3, recalculated B1 to 120 and moved to C4.
Three keyboard Undo steps restored A3=40 and B1=80; three Redo steps restored
all entries. A second immediate quit with `87 Tab 88 Command-Q` exited at
revision 26 with both new values. Minimized-window Quit also exited. An
independent reader checks ten complete workspace snapshots, including the
expected automatic sizing of newly populated rows and unchanged metadata.

The ZIP is 141,171,468 bytes, SHA-256
`a5e551443f6f22c91727c75c99af4a103baaee7cde30d0b9449549b20bef1f45`.
Fresh extraction passed strict/deep signature verification, 260 Dart/Swift files
match the build stage, and the extracted engine matches the signed stage.
This is an ad-hoc local package, not notarized distribution. The working app
now runs 0.43, shows Connected to Codex and retains the user's revision 26
workspace byte-for-byte. No new provider request was run. Evidence and earlier
failed artifacts are separately identified in `release/validation/continuous-entry`.

These results do not accept all keyboard behavior or any full parity family.
Queued clipboard/other command shortcuts, native composition/dead keys and
international layouts, crash recovery of waiting keystrokes, and broader
focus/validation/close combinations still require work. Additional novel AI
remains **0/20** under the original acceptance requirement.

## 0.42 rapid first-cell entry — 2026-09-19

The grid previously ignored subsequent characters and completion keys while
waiting for the first editor frame, and selected the entire initial character.
Four initial regression variants failed before the fix. The handoff now retains
printable key-down/repeat input, grapheme deletion and the shared Enter/Tab/Escape
and Alt-Enter behavior while the grid owns primary focus. The initial typed
character leaves a collapsed caret; F2/double-click retain whole-value selection.
After focus transfers, ordinary text and composition still belong to Flutter's
text input client. Deferred focus work checks mounted/edit/session/focus state.
A second failing regression exposed repeated Enter reopening the original value
during an asynchronous save; completion keys are now consumed during that save.

Nine added widget cases/variants cover fast typing without an intervening frame,
Enter/Tab/Shift-Tab, cancellation and disposal, replacement, repeats, Unicode
deletion, newlines, failed-save retry, in-flight repeated completion, caret
placement and no double insertion when platform text input takes over. Existing
composition, clipboard and local draft-history tests pass. The real-engine
data-bar workspace test now types `60` and Enter without a render between keys,
checks A3=60, B1=120, the recalculated bar, and keyboard Undo back to A3=40/B1=80.
The complete release-engine Flutter suite passed **148 tests**, with two optional
provider tests skipped. Analysis is clean. The Rust sources are unchanged from
0.41 except package version; the prior 168-test Rust evidence is not a new run.

In the isolated signed 0.42 macOS app, one uninterrupted computer-use sequence
clicked A3, typed `60` and pressed Return. It saved both digits, recalculated B1
to 120 and moved to A4. Keyboard Undo/Redo restored 40/60 and 80/120. Typing
`12345` followed immediately by Tab saved B3 and moved to C3; typing `24` and
Shift-Tab saved C3 and moved left. Rapid Escape discarded a replacement draft
without changing the saved bytes. Quit/reopen retained revision 18 and all
values; the recovery files are byte-identical across restart. Native saved
fixtures and regression logs are collected in `release/validation/rapid-entry`.

The ZIP is 141,139,782 bytes, SHA-256
`c78574e31c3a5e4747c795ef48688f549eedda7a41462dd8a05a65677c6157bc`.
Fresh extraction passed strict/deep signature verification; 258 Dart/Swift source
files match the build stage and the extracted engine matches the signed stage.
A subsequent locked release build was cached in 0.20 seconds. This is an ad-hoc
local package, not notarized distribution.

The working app is running 0.42 on the Project budget worksheet and shows
Connected to Codex after reusing saved sign-in. The original revision 26
workspace is byte-identical to its pre-upgrade backup. No new provider request
was run in this qualification.

This closes the observed 0.41 first-entry loss for the tested paths. It does not
qualify all keyboard behavior. Continuous entry into subsequent cells while a
backend write is outstanding, shortcuts before editor attachment, native input
methods/dead keys and broad international layout testing remain separate gates.
No requirement family is accepted complete and additional novel AI remains 0/20.
The implementation follows Flutter's documented distinction between
[focus key handling and platform text input](https://docs.flutter.dev/ui/interactivity/focus).

## 0.41 data-bar qualification — 2026-09-19

Data bars derive normalized geometry from the populated rule range, including
formula results, without changing stored values or ordinary styles. Standard
bars retain their min/max length percentages; Office 2010 bars add automatic
zero bounds, negative colors/borders, automatic/midpoint/no axis, direction,
solid/gradient fills and show/hide value. The painter clips to the cell and
bar-only display retains value semantics, formula-bar text and copied data.

The rule editor exposes the settings with color swatches and editable RGB
values. Supported linked and standalone x14 rules import; export emits linked
standard fallback and x14 records. Row/column edits transform both ranges and
absolute threshold formulas. Legacy recovery upgrades only exact preserved
bar rules, leaving prior deletions and edited rules intact. Unknown extension
metadata, colors or rules remain source-preserved and block unsafe rule and
structural changes. Seven-digit row labels use one line and scale to fit.

Automated evidence: **168 core Rust tests passed** (eleven added data-bar tests),
**139 Flutter tests passed**, with two optional provider tests skipped, and
analysis is clean. Tests cover type distinctions, negative/positive/zero and
constant populations, priority/Stop, recalculation, undo, formula thresholds
and errors, invalid atomic edits, legacy recovery, source preservation,
structural movement and file round trips. Flutter tests use the real engine
for editor Apply, swatch changes, bar-only value retention, recalculation,
undo, sparse last-row rendering, save/export; a pixel test checks the painter.
The six-sheet-range import fixture is generated by XlsxWriter, not Excel.
`scripts/verification/verify_data_bars.py` independently checks six exported
files with openpyxl and namespace-aware XML, including link/range consistency,
required children, fallback bounds, retained formulas and raw cell values.

The final package passed native midpoint scaling, bar-only Apply, keyboard
Undo/Redo, the seven-digit last-row labels, export and quit/reopen checks.
Revision 13 and the two native XLSX exports are byte-identical across restart;
`scripts/verification/verify_native_data_bars.py` checks the saved artifacts.
The initial native value-edit/undo check occurred before the midpoint engine
correction; the final engine's value-change path is covered by the full
139-test Flutter suite. The earlier 21 independent readers, the six-file
new data-bar reader and the native artifact reader pass. There are 30 evidence
files with hashes in `release/validation/data-bars/manifest.json`.

The final ZIP is 141,133,108 bytes, SHA-256
`594a9756d74fa0ae428627786bffb0951d826b0a9ab03d493aa162a058eeef3c`.
A fresh extraction passed strict/deep ad-hoc signature verification; 258
Dart/Swift source files match the stage and its signed engine matches the
extraction. A final locked release build was cached (0.91 seconds). This is a
local ad-hoc package, not notarized distribution. The user app is running
0.41 with the original revision 26 workspace byte-identical and the assistant
showing Connected to Codex; no new provider execution was tested.

No live Microsoft Excel comparison has been performed. Exact geometry at equal bounds, nondefault min/max lengths,
midpoint/automatic-axis edge cases, gradient appearance and percentile semantics
still require a Microsoft Excel reference corpus. Theme/indexed/tinted bar
colors, newer extensions, icon sets, pivot behavior, copy/fill/cut rule semantics
and the rest of X07 remain incomplete. The bar-only setting currently applies
to the worksheet grid; other document/chart/print renderers need qualification.
A native rapid-entry sequence (`60` immediately followed by Enter from grid
focus) committed `6`; F2, entering `60`, observing the editor and then Enter
committed `60` correctly. The artifact `rapid-entry-observed.folio` records the
failed sequence. The 0.42 section above records the handoff fix and its qualification;
do not treat the data-bar checks as full keyboard-input qualification.
No requirement family is accepted complete; additional novel AI remains 0/20.

Primary format references: [Microsoft x14 data-bar schema](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.office2010.excel.databar?view=openxml-3.0.1),
[standard data bars](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.databar?view=openxml-3.0.1),
[PercentMin behavior](https://learn.microsoft.com/en-us/office/vba/api/excel.databar.percentmin),
[PercentMax behavior](https://learn.microsoft.com/en-us/office/vba/api/excel.databar.percentmax),
[axis visibility](https://learn.microsoft.com/en-us/office/vba/api/excel.databar.axisposition),
and [threshold types](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-xlsx/df3812c6-077a-4ecd-853f-95e9c593ed38).



## 0.40 conditional-formatting qualification — 2026-09-19

Standard value and formula predicates, text/blank/error predicates,
duplicate/unique values, top/bottom ranks or percentages, average/deviation
rules and two-/three-color scales now derive sparse display overlays. Priority
combines nonconflicting properties, with higher-priority properties winning and
Stop if true ending evaluation. The underlying cell value and ordinary format
remain unchanged. Conditional font, fill and number-format output shares the
grid, sizing, copied display text and linked-document display paths.

The worksheet rule manager creates, edits, reorders and deletes rules, edits
Applies to and previews each rule's font/fill. Apply validates the final
reviewed rule list before closing and commits one history/layout transaction.
An invalid formula retains the draft for correction; Cancel discards pending
rule edits. Visible blank cells are evaluated in bounded asynchronous batches
with revision guards; aggregate thresholds use the whole populated rule range.
Standard rules and differential styles persist in `.folio` and XLSX. The
existing OOXML structural transformer now rebinds conditional-rule metadata
for new and imported workbooks. Changes to supported imported differential
properties retain tested vendor attributes in the source style records.

**157 core Rust tests passed**, including 12 new conditional-format tests:
comparison operators, relative/mixed references, whole-column blanks, typed
errors/text, cross-rule property precedence, Stop if true, recalculation,
rank/average/duplicate populations, two-/three-color interpolation, formula
thresholds, read-only preflight, stale/invalid/protected/extended guards,
atomic final-list Apply and history, XLSX differential records, structure,
recovery and vendor style metadata. **137 Flutter tests passed with the final release engine** (two optional
provider tests skipped). Analysis is
clean. The native-bridge workflow covers rendered fills, recalculation, invalid
formula correction, manager Cancel, reordering, whole-column blank display,
Undo, save and XLSX export. Rich-run tests verify conditional font/color
precedence without altering the stored runs.

**21 independent file-reader scripts passed**, including the new openpyxl/XML
conditional-rule verifier. A separate native-artifact reader passes on nine
saved/exported files. Readers verify rule formulas/ranges, priority, differential
font/fill/number records, schema order, ordinary cell styles, preserved vendor
attributes and structural changes; they do not evaluate Excel's rendering.

The final **0.40.0** ZIP is **141,077,348 bytes**, SHA-256
`42f44b93054b246aff2c15a5d4fc6af1210dd34af9342c7f1074c79f0580a1bc`. A fresh extraction
passes strict/deep ad-hoc signature verification; all **257** Dart/Swift source
files match staging, and the engine matches the signed stage. Release freshness
was checked with a cached locked build. The package is not notarized.

An isolated signed desktop app visibly rendered A2=20 in red and blank D cells
in gray. Typing 6 removed the red fill and recalculated B2 from 40 to 12;
native Cmd+Z restored the original value/fill. The native manager changed the
threshold from 10 to 25, applied it without changing A2, and one Cmd+Z restored
the earlier rule. Native Save exported XLSX, the process quit normally, and a
new process restored revision 11 and both rules. The blank-cell rule also
rendered at D1048576. The post-restart workbook and XLSX export are byte-identical
to the pre-restart files. The second Save dialog appended a duplicate extension;
only the evidence filename was normalized afterward. Native review also exposed
wrapping in seven-digit row headers; adaptive header width remains a UI gap.

The isolated app is closed. The user app now runs 0.40, displays both the original
document and Project budget, and shows Connected to Codex. Its saved revision-26
workspace is byte-identical to the pre-launch backup (SHA-256
`3e899a047d3fd28e0926872a3cdbcde936ee11ae4c1e35e1e40b5bfc95369f22`). This is connection
status, not a new AI provider request/Apply qualification. Evidence and a SHA-256
inventory are retained under `release/validation/conditional-formatting/`.

Known gaps remain: data bars and icon sets, date-period authoring, theme/indexed
colors and border/advanced differential formatting, pivot/table-specific rules,
copy/fill/cut rule behavior, complete formula/name support and direct Microsoft
Excel comparison. Unsupported standard rules and extensions are retained and
flagged; extended-rule sheets are not edited by the rule manager. Blank,
duplicate/type coercion, wildcards, locale, zero-width/equal/invalid scale
thresholds and interpolation edge cases still need Excel differential checks.
The evaluation guard reports views exceeding two million cell/rule comparisons;
this is not full-size Excel performance qualification. No requirement family
is accepted complete; novel AI remains **0/20**.

Primary references consulted:
[Microsoft's conditional-formatting guide](https://support.microsoft.com/en-us/excel/use-conditional-formatting-to-highlight-information-in-excel)
and the [OpenXML rule schema](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.conditionalformattingrule?view=openxml-3.0.1).
These describe the intended behavior; they are not execution evidence.

## 0.39 data validation qualification — 2026-09-19

Standard SpreadsheetML rules now live in the workbook projection and survive
`.folio` save/reopen and XLSX import/edit/export. The native Flutter editor has a
three-tab Settings/Input message/Error alert dialog. Typed entries use a
revision-checked preflight and authoritative commit; Stop cannot be overridden,
Warning/Information require an explicit override, retry retains the draft and
cancel discards it. Existing invalid cells are not rewritten when a rule is
applied. Bulk copy/fill and deletion use their separate existing entry paths.
List choices are searchable and use underlying typed source values; the OOXML
`showDropDown` setting is inverted for the visible checkbox, as Excel's true
value hides the arrow. Relative criteria are anchored at the selection's first
cell. Input hints currently appear below the formula bar and dismiss with Esc.

Supported structural transformations reuse the OOXML range/formula transformer
for imported and newly authored workbooks. Partial clearing splits rectangles
and reanchors formulas without enumerating full-sheet ranges. New package
baselines rebind source rule indices so later edits preserve the correct unknown
attributes and children. Internal Paste All and directional fill copy rules;
Paste Values/Formulas/Formats retain destination rules. Portable source fragments
retain namespace scope for cross-sheet copies. A new-workbook clipboard test
also exposed a missing optional styles-part failure; copying now materializes
the source default style instead of adopting the destination's default.

Current evidence: **145 core Rust tests**, **135 Flutter tests** and static analysis pass. Eight validation
scenarios cover types/operators, relative/custom/range formulas, whole-number
rejection, 1900/1904 dates and typed clocks, Stop/Warning/Information, disabled
alerts, current-criterion recalculation, stale revisions, read-only preflight,
atomic rejection, existing invalid data, large/disjoint ranges, clearing,
source attributes, fill/paste/cross-sheet copy, structural changes, history,
save/reopen, protected sheets and explicit x14 limitations. Vendored expression
and clock-parser regressions pass with the full vendor suite: **2,228 unit tests**
plus **23 documentation tests**, with eight pre-existing ignored tests. The full
Flutter suite passes with the debug engine; the validation and AutoFit workflows
also pass with the final release engine. Two optional provider tests remain
skipped. **20 independent file-reader scripts** pass, including openpyxl and XML
checks for six validation workbook exports. A separate native-artifact reader
passes for the three native exports. These are not live Excel comparisons.

The real-bridge widget scenario exercises typing, Stop retry/cancel, warning
override, list search/selection, rule authoring with an input hint, clear,
Undo/Redo and saved XLSX. The settings and list dialogs do not send data to AI.

Native qualification used a disposable `com.folio.validation.check` workspace.
Typing 9 into the 1–5 whole-number cell raised Stop without committing; Edit
value retained the draft and correcting it to 4 committed one change. Native
keyboard Undo restored every seed worksheet exactly, and Redo restored the
edited sheets. A list search selected Blue and persisted its text type. Typing
Purple raised Warning; explicit Use anyway committed it, and Undo restored Blue.
The native settings dialog changed a list source to add XL; Undo restored the
previous rule. The app exported XLSX through the macOS Save panel, exited with
its process verified terminal, reopened the same values/rules, rejected another
invalid entry and cancelled it without changing revision 18. Recovery bytes and
the two native exports match exactly. A final rebuild included the quoted-XML
attribute-boundary fix; the final signed package reopened this workspace,
repeated Stop/cancel and exported identical bytes. Its real-engine widget
validation and AutoFit checks also pass. Earlier full native history/rule-edit
observations came from the preceding package with identical Dart/Swift sources.

The final ZIP is **140,941,992 bytes**, SHA-256
`4fae519748a693a0785baf08368f6be4265d464289c25bb983bd96d9436f3c41`.
Fresh extraction passes strict/deep ad-hoc signature verification; **256**
Dart/Swift application source files and the signed engine match the build stage.
It remains locally signed and not notarized. The main app now runs 0.39 and shows
Connected to Codex; no new provider request was tested. The user's latest
revision-26 workspace was backed up before restart and remains byte-identical:
`3e899a047d3fd28e0926872a3cdbcde936ee11ae4c1e35e1e40b5bfc95369f22`.

Evidence and its SHA-256 inventory are retained in
`release/validation/data-validation/`. Reproduce engine checks with
`cargo test --manifest-path rust/Cargo.toml`, widget checks with the selected
`FOLIO_CORE_PATH`, and file checks with `verify_data_validation.py` and
`verify_native_data_validation.py`. The latter accepts the native artifact folder.


Limitations remain: no direct Microsoft Excel differential run; named ranges
are not loaded into the Folio calculator; x14 rules remain opaque and prevent
entry only in their known target ranges (unknown ranges are guarded), while
editing/copying those rules is blocked explicitly. Circle-invalid, selecting
all validation cells, apply-to-same-settings, per-cell input-message positioning,
complete locale/IME/blank/coercion/error behavior and conditional formatting are
not implemented or qualified. This does not accept X07 or full Word/Excel parity.
Novel AI features remain 0/20, following the goal's parity-first ordering.

Primary behavior references checked 2026-09-19:
[Microsoft data validation](https://support.microsoft.com/en-us/excel/more-on-data-validation),
[SpreadsheetML DataValidation](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheetml.datavalidation),
and [openpyxl validation and dropdown polarity](https://openpyxl.readthedocs.io/en/stable/validation.html).
The independent reader validates file structure and stored settings; it does not
evaluate Excel's data-validation behavior.

## 0.38 early-date qualification — 2026-09-19

The previous engine used ordinary Gregorian dates for some operations while the worksheet renderer retained Excel's fictional February 29, 1900. DAY/MONTH/YEAR could therefore disagree with the visible date, and early real dates were off by one. The engine now retains exact worksheet fields for serial zero (1900-01-00) and serial 60 (1900-02-29), maps real January/February dates to the correct serials, normalizes DATE year/month/day arguments, and steps WORKDAY/NETWORKDAYS and their INTL variants through serials with distinct holiday identities. DATE adds 1900 to years 0–1899. EDATE clamping and EOMONTH use Gregorian target-month lengths; February 1900 ends at serial 59 for those month-end calculations. The standalone IronCalc date formatter also retains worksheet fields; Folio display/TEXT continue using SSF.

Evidence:

- **137 core Rust tests**, **134 Flutter tests**, and static analysis pass. Two optional provider-dependent Flutter tests remain skipped. The complete vendored IronCalc suite passes **2,226 unit tests** plus **23 documentation tests**, with eight pre-existing ignored tests. Four upstream early-date tests explicitly expected a different calendar; their assertions now follow the intended Excel serial convention. All other vendor expectations pass unchanged.
- `early_dates_fixture.py` creates **774 modeled expectations** across serials 0–71, date components, weekdays, DATE/DATEVALUE/VALUE/TEXT, constructor overflows/year rules/bounds, arrays, EDATE/EOMONTH and working-day holidays. This is independently modeled expected data, **not a Microsoft Excel calculation capture**. Engine checks also cover typed date entry, date criteria, Undo and exact save/reopen exports. The existing paired 1900/1904 corpus still passes.
- A real-Rust Flutter test exercises serial-zero and fictional-leap-day display, F2 date entry, formula recalculation, exact worksheet Undo/Redo, formatted clipboard text and saved/reopened XLSX. It passes using the packaged release engine. The initial targeted debug-engine run also passed.
- **19 independent file-reader checks** pass after the full suites regenerate their artifacts. The new reader checks raw OOXML serials, formulas, typed caches/errors, number formats and explicit date-looking strings. Raw XML is essential because openpyxl maps both serials 59 and 60 to Gregorian February 28. Engine and widget exports are byte-identical after save/reopen.
- An isolated signed **0.38 macOS app**, `com.folio.dates.check`, loaded the engine-generated workspace. Native Sheets displayed serial zero and February 29/day 29. F2 entry of `1900-02-28` in A62 changed the dependent day to 28, weekday to 3 and constructor result to 59. Keyboard Undo restored the complete seed worksheets; Redo restored the edited worksheets exactly. Native Save exported the workbook, the process exited normally, and a new process restored revision 5 and the edited date. Recovery bytes and the second native export are identical to the pre-restart files. `verify_native_early_dates.py` independently checks all retained data and the 774-result corpus in both exports. The isolated app then exited normally.

The ZIP is **140,775,405 bytes**, SHA-256 `a09d782e9b8234bf24f283356d91ad7dfd617c393b638e84afe769f732adeb00`. Fresh extraction passes strict/deep ad-hoc signature verification; **255 Dart/Swift application files** and the signed Rust engine match the build stage. The app remains locally signed, not notarized. The existing objective_c framework-name warning remains. The main app was upgraded after normal exit and a workspace backup; it restored the original revision-25/76-word document and workbook, reconnected to Codex, and is open on Sheets at B4=2400. Its workspace remains byte-identical, SHA-256 `74c2a1b50655eb826c9bd987e1e53aae4c438e806eea9a59704bde9fc4e6abb7`. Connection status does not establish a new provider-request/Apply test.

Supporting references: Microsoft's [1900 leap-year compatibility explanation](https://learn.microsoft.com/en-us/office/troubleshoot/excel/wrongly-assumes-1900-is-leap-year) and [DATE documentation](https://support.microsoft.com/en-us/excel/functions/date-function); a [firsthand EOMONTH report](https://techcommunity.microsoft.com/discussions/excelgeneral/issue-with-the-sn-for-2291900-when-using-eom-function/4335766); and [ClosedXML date regression tests](https://github.com/ClosedXML/ClosedXML/blob/09f2f21a60e821ce7fb18ca3576b272e5dd6d921/ClosedXML.Tests/Excel/CalcEngine/DateAndTimeTests.cs), including EDATE January 31 → February 28 and EOMONTH serial 60 → 59. These sources prompted corrections to the initial all-functions-use-February-29 assumption. ClosedXML is an independent engine, not an Excel oracle. No local Excel app or connected Codex Excel session was available for direct comparison.

Retained evidence and its SHA-256 inventory are under `release/validation/early-dates/`. Full date parity remains unproven: financial day-count, DATEDIF, WEEKNUM/ISOWEEKNUM conventions around serial 60, full coercion/error/array behavior, locale/timezone and negative dates still need qualification. Gregorian bridge users currently collapse serial 60 to February 28. The previous 0.34–0.36 native formatting/calendar workflows remain separately outstanding. No Word/Excel requirement family is accepted complete; novel AI remains **0/20**.

## 0.37 typed spreadsheet error qualification — 2026-09-19

Spreadsheet errors and text with identical characters now keep distinct types. Import protects explicitly stored shared, inline and scalar strings recognized by the calculation engine as error tokens; the existing legacy-source migration repairs exact prior projections without replacing actual user edits. Export receives the calculator's error flags for every ordinary and temporary calculation path. Formula caches use error, string, Boolean or number types, preserve source formula/cell metadata, and omit unavailable or engine-only diagnostic results so Excel can recalculate the retained formula. Paste Values snapshots also retain type flags, preventing an error result from becoming quoted text.

Four new core regressions reproduced the import, paste and export failures before the changes; a fifth covers result-type transitions and diagnostic caches. **136 Rust tests pass.** Coverage includes all seven classic errors, lower-case error-shaped text, shared rich/inline/scalar strings, ISERROR/IFERROR/ERROR.TYPE, an identical-string error-to-text transition with different custom text formatting, numeric transitions, changed formulas, source attributes/extensions, omitted circular-diagnostic caches, original-byte preservation, exact history, Paste Values, structural movement, fill, legacy migration, .folio recovery and XLSX reimport. An independent openpyxl/lxml reader checks the saved data types, values, formulas, metadata and recovery files. Fixture extension ranges are retained in the cache-refresh checks; a separate fixture without extensions exercises structural movement, which still intentionally rejects opaque extension ranges.

[Microsoft's CellValues specification](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.cellvalues?view=openxml-3.0.1) establishes the distinct error and string representations. [IS function documentation](https://support.microsoft.com/en-us/excel/functions/is-functions) supports checking actual types rather than recognizing error-shaped strings. These sources and independent file readers are not a substitute for running the same workbook in Microsoft Excel.

**133 Flutter tests pass**, with two optional real-provider tests skipped; analysis is clean. The new real-engine widget test uses the actual Paste Values menu, tests the public formatted clipboard text separately from its private type-preserving snapshot, verifies calculated ISERROR results and custom display after an error-to-text edit, reverses edits, and saves/reopens/exports. The first widget run exposed only an incorrect test expectation for the existing CRLF clipboard row separator; that expectation was corrected. **18 independent file readers pass**, including the calendar reader now asserting actual error cache types, plus the separate native artifact reader.

The isolated signed `com.folio.errors.check` app passed native B2:B3 copy, Values only into C2:C3, ISERROR TRUE/FALSE discrimination, keyboard Undo/Redo, F2 formula edit from error to text with an immediate custom-format change, another history cycle, native XLSX export, normal quit/process-exit/relaunch and second export. Recovery remained byte-identical at revision 16; both native XLSX files are byte-identical (SHA-256 `3f9e2e22550fd2c7586f0883de659b0f8bcfa2fdbc064a3ef78233c55ee9e0b7`). Source-load normalization adds empty tables/hidden rows and null pane/filter fields; the history comparison accounts for these pre-paste import defaults. The native test exercised #N/A; all seven classic errors are covered by the engine and independent-reader corpus. An accessibility-index click did not focus the address field; its unintended test-grid edit was undone and visible-coordinate entry succeeded. The native Save path also required coordinate focus after a clipboard timeout. These observations do not qualify full accessibility or every focus path.

The **0.37.0** ZIP is **140,776,573 bytes**, SHA-256 `2d76077cc3aed7bf292ed6fe7e206402fbaffd8ac3ce914d795ba46ec5e93239`. Fresh extraction passes strict/deep ad-hoc signature verification; 255 Dart/Swift application files and the signed Rust engine match the build stage. It is not notarized. The existing objective_c framework-name warning remains. The isolated test app exited normally. The user app was then upgraded from 0.36 to 0.37 after normal exit and backup, restored the original revision-25/76-word workspace, reconnected to Codex and was left visibly open on Sheets at B4=2400. The user workspace SHA-256 remains `74c2a1b50655eb826c9bd987e1e53aae4c438e806eea9a59704bde9fc4e6abb7`. Connection status is not a new provider-request/Apply qualification.

Retained engine/widget/native files, verification logs and package identity are under `release/validation/error-types/`, with a SHA-256 inventory. This closes the classic error-cache issue identified in 0.36, not full error/coercion/dynamic-array compatibility. All Word/Excel families remain incomplete; the novel-AI acceptance count remains **0/20**.


## 0.36 workbook calendar and typed export qualification — 2026-09-19

The imported OOXML calendar now configures each calculation model before loading inputs. Changing calendars rebuilds even an otherwise identical model. Temporary models used by table resizing, structural filtering and package refreshes receive the same setting. Supported date entry/coercion, TEXT/VALUE, criteria, working-day/date functions and financial paths use workbook serials. Bond helpers convert only at their 1900-domain boundaries; functions calling YEARFRAC retain workbook serials. Month/day arithmetic checks bounds instead of panicking on extreme offsets.

Export qualification exposed typed dates being written as text. Newly entered dates, percentages/currencies and booleans now export with their calculated scalar types. Explicit OOXML strings that resemble formatted numbers remain text, with migration for exact old source projections while preserving user edits. The obsolete saved 1904-calculation warning is updated during hydration.

**131 Rust tests passed.** Paired independently generated 1900/1904 workbooks provide **47 explicit expectations each (94 total)** using Python Gregorian dates, plus **28 paired financial comparisons** requiring finite equivalent results for the same calendar dates. Coverage includes serial zero, leap day, date-part arrays, date constructors/parsers, TEXT, working days/holidays, week numbering, date criteria/DSUM, coupon boundaries, terminal dates and oversized month arguments. Additional tests cover model isolation/invalidation, table resize, table and worksheet filtering after column deletion, exact Undo/Redo, preserved source text, migration and saved/reopened exports. Paired financial equivalence is not direct Excel qualification. The full vendored calculation suite passed **2,226 tests with 8 ignored**, plus **23 documentation examples**.

**132 Flutter tests passed**, with two optional real-provider tests skipped; analysis is clean. The new real-FFI widget test enters a date in the cell editor, verifies its numeric serial and formatted display, copies it, chooses a displayed date in the filter menu, checks visible rows and one-step history, and saves/reopens/exports. **17 independent file-reader checks passed.** The calendar reader verifies workbook metadata, raw serials (including zero), numeric formula caches, typed date/number/boolean exports, retained literal strings, structural filter metadata and reopened packages. Only the intentionally volatile NOW cache is excluded from the reopened worksheet comparison. This does not qualify formula-error cache typing or every coercion rule.

The packaged 0.36.0 ZIP is **140,775,978 bytes**, SHA-256 `f32b3cdc15916b643559a45508b5cb03798d3e12c8c6e63623b471ca4a027072`. A fresh extraction passes strict/deep ad-hoc signature verification. All **255 Dart/Swift application source files** match the build stage; the signed Rust engine and license notices match. The app is not notarized, and the existing objective_c framework-name dependency warning remains.

Desktop access returned after the build. The saved 0.33 process exited normally before 0.36 launched. Native startup visibly restored the original document (76 words, revision 25), then the Project budget worksheet at B4 = 2400. Connect restored **Connected to Codex**; this is connection status, not a real-model proposal/apply test. The user workspace remains byte-identical, SHA-256 `74c2a1b50655eb826c9bd987e1e53aae4c438e806eea9a59704bde9fc4e6abb7`. The latest app is left open on Sheets for review. These observations qualify startup/navigation/reconnection only; native calendar fixture editing/filtering/export/restart remains pending.

Evidence and SHA-256 manifest are under `release/validation/date-systems/`. Microsoft's [date-system guidance](https://support.microsoft.com/en-US/Excel/date-systems-in-excel) and [Date1904 specification](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-xls/4a5e900a-0eb0-4355-8fc1-81aab8f46e8b) establish the epoch relationship and serial-zero boundary, not complete formula behavior. Remaining work includes early-1900 special cases, complete DATE year/coercion/negative-date semantics, locale/timezone behavior, date-system authoring, conditional-format integration, inferred entry formats, error-cache typing and direct Excel comparison. No Word/Excel family is accepted complete; novel AI remains **0/20**.

## 0.35 TEXT and formatted-label qualification — 2026-09-19

Numeric `TEXT()` results now use the same vendored formatter as cell display. Format-code references recalculate through the existing staged edit/layout transaction. The function bounds format shape and output length, supports value/format-code array broadcasting in the engine, and returns plain characters for accounting spacing. Formula output remains text; source numbers and cell formatting are unchanged. Worksheet/table filter choices, range-sort headings and the local chart overview now use formatted labels while retaining numeric values and raw filter keys for calculation/filtering.

**127 Rust tests passed**, including twelve numeric TEXT/display consistency examples covering precision, currency, percentage, fractions, scientific notation, scaling, leap-day serial 60, noon/elapsed-time carry, conditional sections, hidden output and accounting spacing. Regressions cover numeric-string/boolean/blank inputs, errors, invalid patterns, mixed array dimensions and per-element format failures. Filter tests verify read-only choices, raw identity, cross-column cascading, exact history and exported criteria. Targeted vendored IronCalc suites passed **131 text-related tests and 8 array tests**; these are not a claim that every upstream test ran.

**131 Flutter tests passed**, with two optional real-provider tests skipped; analysis is clean. A full-app test against the new release Rust engine edits a referenced format-code cell, verifies formula/grid output, copied text, one-step Undo/Redo, chart labels, range-sort headings, percentage-filter search/selection, retained checked state, filtered row visibility and byte-identical save/reopen export. It uses actual widgets and FFI, not a packaged native application session.

**16 independent openpyxl/lxml file checks passed**, including filters/range sorting and the existing formatting, layout, clipboard and source-fidelity suites. The new reader resolves worksheet relationships and verifies numeric source types, string formula caches, retained formula/format codes, raw filter criteria, hidden rows and byte-identical saved/reopened XLSX. A separate SheetJS SSF 0.11.2 comparison matched ten TEXT cases; two time-carry cases use explicit regression expectations because the reference has an upstream rounding defect. The existing formatter corpus still matches 18,020 applicable cases; 2,000 invalid negative-date cases remain separately counted. Neither reader nor SSF is Microsoft Excel.

Retained evidence is under `release/validation/text-formats/`. The 0.35.0 ZIP built successfully (140,772,821 bytes, SHA-256 `b23eeb9064d24bef47c31054390b731d4162bb3d45e2c9a45bfcda31742b553f`); a fresh extraction passes strict/deep ad-hoc signature verification. All 255 Dart/Swift application source files match the build stage; the extracted signed Rust engine and bundled notices match the stage/source. The app is not notarized. The existing objective_c framework-name build warning remains. Package/runtime evidence is recorded in that directory's `package-verification.json`. The desktop was checked after building and still reported locked. The user's running app remains 0.33.0, and its revision-25 workspace hash is unchanged. A freshly extracted 0.35.0 application and backup are ready for the requested launch when desktop access returns. Native 0.35 qualification remains pending desktop access; no user workspace was used for these test scenarios.

Remaining: direct Excel comparison of coercion/error precedence/array behavior, 1904 calendar formula evaluation, full visible dynamic-spill integration, display-based filter identity/grouping and date filters, complete custom/locale formats, width-sensitive General/overflow display, advanced accounting layout and full Office chart handling. Filter labels currently deduplicate by the existing raw identity; equal raw values with different formats use the first occurrence's label, while distinct values with identical rounded labels stay separate. No requirement family is accepted complete; novel AI remains **0/20**.

## 0.34 number-format qualification — 2026-09-19

A shared Rust display projection now supplies the grid, automatic/explicit sizing, clipboard public text, displayed-value search, and linked document tables/export. Source numeric values, formulas and OOXML format codes remain separate. A native Flutter dialog provides presets/custom codes, mixed-selection indication, async preview, validation, Cancel/Apply and revision guards. Accounting skip/fill directives use shaped glyph widths in the grid and preview; AutoFit uses natural unfilled width. Colors respect numeric section selection and indexed palettes. Calculation error types bypass text/hiding sections.

Engine evidence: **124 tests passed**, including custom precision, hidden zero/text sections, conditions/colors, inherited row/column formatting, raw 1900/1904 serial display, the fictional 1900 leap day, last supported calendar day, noon/midnight/elapsed-time carry, invalid patterns, private-use Unicode text, formatted copy, calculation-generation stability, staged row layout, read-only preview during staging, exact history and saved/reopened packages. A generated 20,020-case corpus matched **18,020** nonnegative-date/applicable cases against separately installed SheetJS SSF 0.11.2. The other 2,000 negative date cases deliberately show invalid-date hashes instead of the reference's empty string; they are counted separately. Noon/midnight carry uses explicit regression expectations because the original port's seconds-only rendering was incorrect. This comparison does not qualify actual Microsoft Excel behavior.

The real-Rust widget test exercises the reachable dialog, preview, color, accounting width, Apply, mixed selection, cancelled invalid code, concurrent edit rejection, clipboard text, displayed-value search, one-step history, unchanged calculated values, and byte-identical save/reopen exports. An independent openpyxl/lxml reader verifies raw numbers, formula/cached result, complete unaffected cell styles, custom format codes, inherited styles, automatic heights, date-system metadata and identical reopen output. The dialog screenshot is a Flutter test render, not native packaged evidence.

Full regression suite: **124 Rust tests and 130 Flutter tests passed**, with two optional AI-provider tests skipped; analysis is clean. **14 independent file-reader checks passed**. Retained outputs, test logs, reference counts, a test-rendered dialog preview and SHA-256 manifest are under `release/validation/number-formats/`. Packaged 0.34.0 built successfully; fresh ZIP extraction passed strict/deep ad-hoc signature verification. 255 Dart/Swift application files match the build stage; the packaged signed Rust library matches the stage, and the new formatter license notices are bundled. The original workspace remains revision 25 with its pre-test SHA-256 unchanged. The running user app remains 0.33.0. The Mac was rechecked after the build and still reported locked. Native 0.34 qualification is pending a usable desktop; the last attempt to show the existing 0.33 application reported the Mac locked. No user workspace has been modified for this feature's test scenarios.

Remaining: width-dependent General/overflow display, locale/localized built-ins and non-Gregorian calendars, 1904 calendar formula evaluation, TEXT-function consistency, filter/chart label formatting, rotated/stacked/bidirectional accounting layout and native/direct-Excel comparison. No requirement family is accepted complete; novel AI remains **0/20**.


## 0.33 clipboard qualification — 2026-09-19

Implemented quoted TSV with embedded CR/LF/tabs, doubled quotes and blank edges; an immutable process-local copied snapshot; native public text plus opaque token; mixed/absolute/relative formula adjustment; complete source XF/font/fill/border/number-format dependency remapping; inherited styles and rich/shared-string materialization; tiled destinations, cross-sheet copies, values/formulas/formats, transpose and skip blanks. Paste stages content, recalculation and automatic row fitting in one history transaction. Invalid bounds, incompatible selections, unavailable tokens, malformed text, unsupported structured references and merged/array/protected areas reject without editing.

Engine tests: **117 passing**, including six clipboard cases. Real-bridge widget coverage exercises actual copy/paste shortcuts and ribbon/dialog controls, copy-time immutability, source mutation, one-step history, async destination stability, measurement rollback, external multiline text and save/reopen. Independent openpyxl/lxml reads cover complete styles, formulas/caches, rich XML, literal numeric text, inherited column styles and saved/reopened packages. The clipboard test exposed unnecessary empty defined-name/relationship records on reopen; exporters now avoid adding them. The final suite passed **117 Rust tests** and **129 Flutter tests**, with two optional AI-provider tests skipped; Flutter analysis is clean. Thirteen independent file readers passed, including the existing row sizing, multiline, geometry and formatting readers plus clipboard and native clipboard checks.

The packaged Mac test used a separate `com.folio.clipboard.check` container and an independently generated workbook. Command-C/Command-V copied A2:D2 to A5:D5: B5 became `=A5*3`, source font/fill/border/number-format and multiline/tab/quote text survived, numeric text `001` stayed text, and row 5 grew to 31.5 points. Keyboard Undo/Redo restored the exact earlier sheets. Native **Values only** retained destination italics; **Formatting only** retained the pasted number while applying Georgia 16/bold and fitting row 8 to 24.75 points. Shift-Command-V opened Paste Special; values plus transpose produced F2:F5 correctly. Skip blanks was enabled in that native scenario, but actual blank skipping is covered by engine tests rather than this nonblank native range.

Native Excel export succeeded, with Undo/Redo still functioning after Save. Normal quit/process-exit/relaunch retained the saved recovery file byte-for-byte at revision 9; a second native export was byte-identical. `scripts/verification/verify_native_clipboard.py` verifies **12 retained snapshots/exports** under `release/validation/native-clipboard/`, including formulas/caches, full source formats, preserved unselected content, exact history, row heights and restart/export equality. The dropdown's offscreen accessibility tree exposed only the selected choice; visible menu coordinates were used for Values only. This is not complete accessibility qualification. The isolated test app exited normally.

The final ZIP is **140,657,310 bytes**, SHA-256 `f22de8ab832d96090d155878adcf308843d374ba6ead1d47ec27381bc070d352`. Its fresh extraction passes strict/deep signature verification. **253 Dart/Swift application and vendor source files** match the staging source, and the extracted engine matches the signed staged library. The app is ad-hoc signed, not notarized. The previously observed objective_c framework-name warning remains a packaging dependency warning. Version 0.33.0 is open on the user's unchanged revision-25 workspace and shows **Connected to Codex**. The old 0.32 process and isolated test process both exited before their respective replacements; user data stayed byte-identical with SHA-256 `74c2a1b50655eb826c9bd987e1e53aae4c438e806eea9a59704bde9fc4e6abb7`.

This is not full clipboard parity. Notes/hyperlinks/validation/conditional rules, cut/move, HTML/RTF, external-app formatting, all cross-workbook theme/named-reference cases, remaining Paste Special modes and direct Excel comparison are outstanding. No Word/Excel requirement family or novel-AI slot is accepted complete.

## 0.32.0 automatic row sizing (2026-09-19)

Automatic rows now grow and shrink after committed content, font/alignment, column-width and calculated-result changes. Formula dependents on other worksheets and cells outside the viewport participate. Imported/custom manual heights remain fixed. Explicit row AutoFit restores automatic sizing; numeric sizing/dragging restores manual sizing. Hidden/filtered and zero-height rows retain their geometry, and rows containing source merges are excluded. Newly visible automatic rows are measured again. This does not add merged-cell fitting, automatic numeric column growth, or full default-dimension authoring.

Rust stages the existing workbook operation and recalculation in a disposable core, determines sparse eligible rows, and validates the complete set of returned measurements. Flutter uses the same text renderer and column-width conversion as explicit fitting. The bridge queues reads, saves and subsequent edits behind preparation, measurement and commit. Failed measurements discard the entire edit; successful text/format/width changes and their resulting heights produce one revision/Undo record. Source packages are shared immutably between cores rather than copied for every keystroke. Intervening writes invalidate pending work even when a loaded file reuses the same workspace id/revision.

`rowHeightModes` distinguishes cached automatic sizes from authored fixed sizes. XLSX import/export uses `customHeight`; a sheet-wide custom default is retained. Row modes follow row insertion/deletion, save/reopen and package rebasing. Legacy saved files infer modes only for unchanged source heights; changed explicit heights stay manual. As with other legacy hydration, a past manual reset to the exact original automatic height cannot be distinguished without historical mode metadata.

- The **111-test Rust suite passes**. New coverage includes complete/partial/invalid/stale/cancelled staged operations, cross-sheet formula results, hidden/filter/manual/merge exclusions, full-row-limit sparse targets, default/manual mode parsing, structural movement, legacy hydration, saved recovery and byte-exact original-file Undo.
- The real-engine application workflow passes inline three-line input and actual workspace keyboard Undo/Redo, manual-height preservation, explicit-fit restoration, automatic shrink, width/font-driven growth, offscreen/last-row sizing, cross-sheet formula updates, existing AI Apply with document plus cell changes, injected measurement failure, queued save/edit ordering, and XLSX/recovery reopen/reimport. The fixture is generated independently with openpyxl/lxml. Test font fallback aliases use the macOS system font instead of Flutter's test-only Ahem; these checks do not establish Office typography equivalence.
- `scripts/verification/verify_row_reflow.py` independently reads produced XLSX with openpyxl/lxml. It verifies literal content and formula caches, manual/automatic OOXML flags, fixed/hidden/merged preservation, rich runs/styles/comments/views, source package parts, sparse row 1048576, and exact Undo/Redo/reopen/reimport. The exporter adds one empty relationship part to the second worksheet; the reader verifies its exact role and that no source part is removed. openpyxl derives its `customHeight` property from `ht`, so actual flags are checked in OOXML instead.

The final **128-test Flutter suite passes**, with two optional AI-provider checks skipped; static analysis is clean. All eleven independent readers pass (row reflow, multiline, columns, rows, geometry, alignment, character formatting, rich text, fonts, styles and strings). Two older widget workflows now wait for native operation/dialog completion instead of assuming animation settling also completes FFI work. The existing row-fit workflow now verifies that fitting an already-automatic pending edit adds no duplicate history step.

The macOS package built successfully and passed strict deep signature verification after fresh ZIP extraction. Distribution: **140,619,811 bytes**, SHA-256 **fd0e0f0c9c391f8df2a22af3537c1fdcafd2ad2f9613e5a501b4b5e192d8465c**, generated 2026-09-19T16:25:28.280877+00:00. All **300** application/test/native/vendor Dart/Swift files matched staging, and extracted Rust library bytes matched the signed staged app. The build retains the existing upstream objective_c architecture-name warning. It is locally ad-hoc signed, not notarized.

An isolated `com.folio.reflow.check` copy passed actual macOS import, Option-Return multiline typing, Enter commit, growth from the source default to 44.25 points, and keyboard Undo/Redo restoring text and geometry together. A fixed 18-point row stayed fixed after multiline typing. Its right-click AutoFit command switched it to automatic at 44.25 points; replacing the text with Short then reduced it to 18.75 points in the same edit. Native column-boundary dragging reduced B to 11.3125 stored units and increased the wrapped row to 82.5 points, refitted rich/offscreen/last-row content, and retained hidden row 5 and merged row 8. Keyboard Undo/Redo restored the entire column/row transaction.

Editing Inputs!A1 with three lines resized Notes!B7 to 82.5 points and the input row in one revision. A native Excel export succeeded; Command-Z after the Save panel restored source text and both heights, and Redo restored them. Quit/process-exit/relaunch preserved the entire recovery file byte-for-byte at revision 13. The second native export was byte-identical. Native XLSX reimport retained all sheet properties except the expected refreshed B7 cached formula result. Editing B2 to Short after reimport shrank its automatic height to 18.75 points at revision 15.

`scripts/verification/verify_native_reflow.py release/validation/native-reflow` passes independently using openpyxl/lxml. It verifies exact history snapshots, allowed text/width/height/mode changes, formula source and cache, source styles/rich runs/comments/views/package parts, unchanged hidden/merged geometry, restart/export byte equality, and reimport plus subsequent editing. The **18** retained snapshots/exports and hashes are in `release/validation/native-reflow/`. The isolated app quit normally and process exit was verified. An initial file-picker typing/clipboard attempt did not confirm the path; the native path field's supported value action and the file icon's Open action succeeded. This is an automation-path observation, not an accessibility qualification.

The verified 0.32.0 app then opened the user's revision-25 workspace unchanged, showed both document and workbook, and reconnected to Codex. Current live app: `/private/tmp/folio-native-0.32.0-jbovgjin/Folio Native.app`, PID 80626. No new live AI-provider run or novel-AI acceptance is claimed. Word/Excel parity remains incomplete and novel AI acceptance remains **0/20**.

The automatic-versus-fixed behavior follows [Microsoft's wrapping and AutoFit guidance](https://support.microsoft.com/en-us/excel/wrap-text-in-a-cell-in-excel-for-mac) and the [SpreadsheetML row attributes](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.row?view=openxml-3.0.1). These references do not replace differential tests in Excel. Broader native merged/hidden/filter combinations, accessibility, cancellation of very large automatic layouts, platform font substitution, exact Excel sizing and representative large-workbook performance remain open.

## 0.31.0 multiline cell and formula editing (2026-09-19)

Inline cells and the formula bar now use multiline input, preserving authored/imported/pasted line breaks instead of stripping them. `SheetEditingKeys` inserts a newline at the current selection through Flutter's public editing path, including selection replacement, callbacks and local Undo/Redo. Option+Return and Control+Option+Return work on Mac; Alt+Enter works on Windows/Linux. Regular/keypad Enter commits and moves down, Shift+Enter moves up, Tab/Shift+Tab move across visible columns, and Escape discards the draft. Formula Enter now follows the same navigation. Other text editing keys retain Flutter's normal handling. Live IME composition keeps Enter/Tab/Escape rather than saving incomplete candidate text.

The formula-bar chevron expands/collapses the editor without losing input focus, selection or an uncommitted draft. Save navigation is single-flight; failed commits retain the source cell and text, while selection changes, export/mode changes and other flush callers stop with an explanation. Successful text edits remain one workspace Undo step. This pass changes Flutter editing and version metadata, not Rust engine behavior.

- **127 Flutter tests pass**, with two optional AI-provider checks skipped. Static analysis is clean. New checks cover both Mac shortcut forms, Alt+Enter under Windows/Linux target variants, reversed Unicode selection replacement, clipboard paste inside both fields, field-local Undo/Redo, Enter/keypad/Shift/Tab navigation, hidden-row skipping, Cancel, expanding with focus intact, composition guards, failed/asynchronous saves and prevented navigation. The Flutter Windows raw-key simulator lacks a keypad-Enter mapping, so that variant tests regular Enter; keypad Enter is covered on Mac/Linux variants. These are widget tests, not real Windows/Linux desktop qualification.
- The real-Rust application test edits imported B3 through the inline field and actual line-break key events, appends a fourth Unicode line, commits, uses actual workspace keyboard Undo/Redo, enters a multiline B10 formula through the formula bar, verifies its 24691.34 result and separate history, explicitly fits the row, saves/reopens and reimports XLSX. The existing focused-history test now deliberately reselects the edited cell after Enter moves to the next row.
- `scripts/verification/verify_multiline.py` independently reads the resulting XLSX with openpyxl/lxml. It checks literal newlines/Unicode and formula source, cached result, exact original-file Undo, exact Redo/reopen/reimport files, row fitting, source formatting/rich runs/comments/frozen views and retained package members. The ten multiline/column-fit/row-fit/geometry/alignment/character/rich-text/font/style/string readers pass against the freshly generated test outputs.
- `release/previews/multiline-editing.png` is an offscreen Flutter render, visually inspected. The expanded formula bar and four-line fitted cell are visible. Its test font set lacks the native emoji fallback, so the owl appears as a missing glyph in this preview; Unicode file/value checks pass; the subsequent native check below renders the owl correctly. The first optional preview capture awaited image rendering inside the test clock and stalled; it was stopped and corrected to use the existing real-async capture pattern. The final focused and complete suites pass.

Shortcut reference: Microsoft's [in-cell line-break instructions](https://support.microsoft.com/en-us/excel/start-a-new-line-of-text-inside-a-cell-in-excel) and [Excel keyboard shortcuts](https://support.microsoft.com/en-US/Accessibility/excel/keyboard-shortcuts-in-excel). These references and tests do not establish exact Excel editing equivalence.

The earlier **106 Rust tests** remain the unchanged engine baseline. Native 0.31 checks are recorded below. Automatic row growth/refitting, rich character formatting during typing, complete formula-edit/reference-pointing behavior, Ctrl+Enter selection fill, quoted multiline grid clipboard, the existing 10,000-byte engine limit, full IME/accessibility behavior and direct Microsoft Excel comparison remain unfinished. No Word/Excel requirement family is marked complete. Accepted novel AI features remain **0/20**.

### Packaged 0.31.0 verification

The release build and fresh-extraction strict/deep signing checks pass. Archive: **140,589,630 bytes**, SHA-256 `0b2bc43ae162b0b01b65a1d59f0f4dc5f41d96d35f7691bc28de5e579fec194e`, generated 2026-09-19T15:47:52.197207+00:00. All **299** application, test, native and vendored Dart/Swift files match staging; extracted Rust and Quill-license bytes match the signed staged app. This remains locally ad-hoc signed, not notarized.

An isolated `com.folio.multiline.check` copy passed actual macOS Open/Save and keyboard workflows. Escape cancelled a two-line cell draft without a revision. Control-Option-Return inserted an in-cell line break; native Unicode paste, Enter commit to the next row, and one-step Command-Z/Command-Shift-Z passed. The stored B3 value is exactly `Native 🦉 first\nNative second`, with the owl rendered in both grid and formula bar. The initial automation `typeText` attempt produced punctuation for the emoji and was cancelled; the clipboard path preserved it. This establishes the observed paste/render path, not all native Unicode input methods.

Option-Return inserted a newline into `=SUM(A10,\nA10)` in the formula bar. Expanding the bar retained focus and left the entire saved workspace unchanged; Enter committed once and moved from B10 to B11. Its displayed/exported result remained 24691.34. Row AutoFit displayed both authored lines. The native Excel Save panel succeeded, and subsequent keyboard Undo/Redo independently restored formula and sizing states while retaining the cell-text edit. Quit/relaunch preserved the entire workspace byte-for-byte at revision 10; the second native XLSX export matched the first byte-for-byte.

`scripts/verification/verify_native_multiline.py release/validation/native-multiline` independently verifies all retained snapshots and exports, including exact history, Unicode/newlines, formula source/cache, row size, original rich text/styles/comments/views and retained package parts. The 11 evidence files and hashes are in `release/validation/native-multiline/`. The isolated app was quit and its process exit verified. The original user app then quit normally; the verified 0.31.0 copy opened the saved revision-25 workspace unchanged and reconnected to Codex. Live copy: `/private/tmp/folio-native-0.31.0-r_b2m1sn/Folio Native.app`, PID 69455. Native IME, failure-injection, all editing shortcuts, accessibility and direct Excel differential qualification remain open.

## 0.30.0 native AutoFit qualification (2026-09-19)

The signed 0.30.0 build was copied to an isolated `com.folio.autofit.check` app/container. The existing user app and old navigation-check session were preserved. All following actions used the actual macOS app and its Open/Save panels, not widget-test drivers or direct engine calls:

- Imported `test/fixtures/column-autofit.xlsx`, selected A1:D11 through the visible address field, and applied **AutoFit Column Width** from the ribbon menu. A/B/C became 11.50390625/27.8671875/13.05078125 stored units. Offscreen C200 participated. Hidden D stayed 17; all cell content and row heights stayed exact. Keyboard Command-Z and Command-Shift-Z restored the complete pre/post workspaces except the advancing revision.
- Applied **AutoFit Row Height** to the same range. Offscreen Z2's four lines produced a 57-point row; three-line row 3 became 44.25 points, mixed-font row 4 became 30 points, and quarter-turn row 11 became 43.5 points. Hidden rows 6/7 stayed hidden, row 6 stayed 75 points, and column widths stayed exact. Native visual inspection showed the multiline/rich/rotated content visible.
- Exported through the sandboxed native Save panel. With only the window focused after Save, Command-Z undid row AutoFit and Command-Shift-Z redid it, preserving the column fit and all other workspace data. This closes that specific worksheet post-Save shortcut check, not every native editor/dialog case.
- Quit, verified process exit, and relaunched. The entire saved workspace remained byte-identical at revision 9. Switching back to Sheets showed the fitted geometry. A second native Excel export was byte-identical to the first.
- Dragged B's boundary wider, then sent two actual boundary clicks; column AutoFit restored the exact prior width. Dragged row 4 taller, then used the lower-boundary click pair; row AutoFit restored the exact prior height. Each fit was one revision. The automation tool's `clickCount: 2` call did not trigger Flutter's double-tap recognizer; two actual clicks did. This distinguishes the tool-input limitation from native gesture behavior.
- Reopened the exported XLSX through the native Open panel. All projected sheet properties matched the fitted workspace except the expected refreshed B10 formula cache (24691.34). The original source file supplied no cached B10 result.

`scripts/verification/verify_native_autofit.py release/validation/native-autofit` passes using openpyxl/lxml, without calling Folio. It checks exact history/restart snapshots, boundary fitting, reimport, both native exports, rich runs, cell styles/number formats, comments, hidden rows/columns, frozen panes, localized Normal font, geometry attributes and package fidelity. Export changes only the worksheet and workbook calculation-request XML; all other package members are byte-identical. Workbook semantics are unchanged except the explicit request for Excel to recalculate. Evidence and hashes are retained in `release/validation/native-autofit/`.

Accessible text-field activation remains unqualified: an element-index click on the address field did not establish editing focus, so subsequent text entered the active test cell. Keyboard Undo restored it exactly; a direct mouse click in the address field worked. A native file-picker icon also required its explicit Open accessibility action. No cause is assigned to the app versus the automation bridge, and no accessibility fix is claimed. Actual Microsoft Excel comparison, context-menu/multi-selection boundary cases, native cancellation/large-file performance and complete accessibility qualification remain open.

The isolated check app was quit and process exit verified; the original user app is again foreground with **Connected to Codex**, revision 25. Its saved workspace still matches the pre-update backup byte-for-byte. No application source or release binary changed in this pass; the earlier 106 Rust / 119 Flutter results remain the automated baseline, rather than a freshly rerun suite. Full Word/Excel parity remains incomplete; accepted novel AI features remain **0/20**.

## 0.30.0 column AutoFit and font-dependent widths (2026-09-19)

Column AutoFit is available in the Rows and columns menu, column context menu and right-boundary double-click. The operation scans populated/styled and calculated cells across all selected columns, including offscreen rows; hidden and filtered rows do not contribute, and hidden columns retain their size. Rich text, manual newlines, script sizes, signed rotation, stacked characters, indentation, formatted values and filter-header space contribute to the natural-line measurement. Wrap/Shrink properties remain unchanged. Row heights are not modified. Blank columns use the existing default; columns containing source merges reject the transaction atomically. Large scans share the yielding/cancellation and revision guards from row fitting.

- **106 Rust tests pass.** Two added integration cases cover per-column measurements, unchanged hidden and row geometry, one revision, identical-size no-ops, bounds/incomplete/stale/hidden rejection, last-column export, retained original style parts, formula recalculation, structural movement and byte-exact Undo/Redo/reopen. The existing merged-row guard test now also checks each covered merged column.
- Imported Normal-font projection follows the built-in style id, its cellStyleXF and font record. It does not assume the English name Normal or cell XF 0. The independent fixture has a localized Standard style pointing to a different font and a namespace-qualified lookalike xfId. Legacy hydration fills only missing Normal-font metadata in current/baseline sheets; the source package remains unchanged.
- `SheetColumnMetrics` measures the widest digit at nominal 96dpi. Stored widths, displayed pixels, character-width entry, boundary dragging and upward-rounded 1/256-character fitting share the documented formulas. Stored width already includes margin padding; the former fixed-seven-pixel conversion added padding again. The sparse axis no longer imposes a separate 4096px ceiling on otherwise valid large-font columns. Unspecified-column defaults and complete inherited sheet-default behavior still need work.
- **119 Flutter tests pass**, with two optional AI-provider checks skipped. Static analysis is clean. New checks cover the published width example, upward quantization across several digit widths, large-font metrics, wrapping/shrink independence, manual newlines, rich runs, indentation, quarter-turn/stacked text, hidden/offscreen cells, numeric formatting and blank columns. Tiny differences between paragraph layouts use a 0.001px tolerance, not exact float equality.
- The real-Rust application test invokes the actual menu, context menu and double-click controls, verifies offscreen C200 contributes, preserves hidden column D and all row sizes, enters a manual character width, exports, saves/reopens, undoes/redoes, and fits a pending draft with separate text/size Undo steps. Existing sizing tests now assert the new character/pixel conversion and real drag distance instead of a fixed seven-pixel font assumption.
- `scripts/fixtures/column_autofit.py` creates the input independently with openpyxl/lxml. `scripts/verification/verify_column_fit.py --with-ui` independently checks widths, unchanged row heights/visibility, localized Normal, comments, rich runs, formatting, frozen views, formula caches, original style bytes and exact history/reopen files. `release/previews/column-autofit.png` is an offscreen Flutter render with real macOS fonts, not native or Excel evidence. Visual review caught selected white text being painted on a light replacement fill; selection now tints the existing cell fill and retains table-header text contrast.
- Baseline sources: [Microsoft AutoFit controls](https://support.microsoft.com/en-us/excel/change-the-column-width-and-row-height), [SpreadsheetML column width and best-fit definitions](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.column?view=openxml-3.0.1), and [built-in named-style identity](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.cellstyle?view=openxml-3.0.1). Implementation/rendering checks establish the paths described here, not full Excel visual equivalence.

All nine independent column-fit, row-fit, geometry, alignment, character-format, rich-text, font, style and escaped-string readers pass against the current outputs. The macOS release built successfully. ZIP integrity and fresh-extraction strict/deep signing checks pass. Archive: **140,580,293 bytes**, SHA-256 `08d80b84d72450ec0266a0222dca1d23f1fe0c56cfb3e00e9af8c62bf9e91cf3`; **253** application/native/vendor Dart/Swift files match staging, and extracted Rust/Quill-license bytes match the signed staged app. It is locally ad-hoc signed, not notarized. Prepared and launched copy: `/private/tmp/folio-native-0.30.0-6gilz6vm/Folio Native.app`.

Desktop control became available during this pass. The old 0.16.1 user app quit normally and its process exit was verified; the saved workspace was backed up before launching the verified 0.30.0 copy (PID 87872). Native accessibility and screenshots show the saved document and workbook at revision 25, the new row/column AutoFit menu entries, and **Connected to Codex** after reconnecting. The menu was dismissed and the workbook left open. Saved document, sheets and revision remain unchanged; no user content or dimensions were edited. The isolated 0.17.1 test session remains untouched. This establishes native launch/recovery, menu availability and assistant connection status only: actual native AutoFit Apply/Undo/export and AI-provider behavior still require qualification. Full parity remains incomplete. Automatic row expansion/refitting and numeric column best-fit growth, complete sheet-default inheritance, named-style authoring, font substitution and exact platform metrics, merged sizing, advanced alignment, neighboring overflow, accessibility and native/Excel differential qualification remain open. Accepted novel AI features remain **0/20**.

## 0.29.0 explicit row AutoFit (2026-09-19)

The worksheet now exposes AutoFit Row Height in the Rows and columns menu, row context menu and lower-row-boundary double-click. A multi-row selection produces one measured point-height transaction. The command scans populated and explicitly styled cells across the full selected rows, including offscreen cells and calculated display values. Measurement uses the same rich-text shaping, wrapping, indentation, rotation, stacking, direction and scaling as the grid; quarter-turn text measures its natural length to avoid depending on the previous row height. Measurement skips paint-layer allocation. Grid padding, selected borders and filter-button width are included.

- **104 Rust tests pass.** Two new integration cases cover independent per-row sizes, one revision, hidden/filtered preservation, empty/repeated no-op requests, invalid/stale/incomplete requests, new-workbook and last-row bounds, structural movement, unchanged style parts, exact Undo/Redo and saved/reopened exports. Imported merged rows reject the entire operation. The merge guard uses the source package; its initial one-based/zero-based mismatch was caught by the regression and corrected.
- Three focused Flutter tests cover width-dependent wrapping, manual breaks, signed/quarter-turn/stacked text, script fonts, shrink, mixed font sizes, actual rendered vertical bounds, offscreen cells, hidden rows/columns, filtered rows, empty styled cells/rows, the 409-point ceiling and cancellation at a cooperative checkpoint. Large measurements yield every 128 entries and expose Cancel; representative large-workbook performance and native cancel interaction remain unqualified.
- A real-Rust app test imports an independently generated XLSX, invokes the actual menu, checks rendered bounds, preserves the hidden row, exports, undoes/redoes, saves/reopens, invokes the real row-boundary double-click, and commits a pending long cell draft before fitting it. The draft and fitted height undo separately. The existing single-line editor strips typed line breaks, so pending-input verification uses long wrapped text; imported manual newlines are tested separately. The early UI run loaded the prior engine while the release library was still rebuilding; that result is discarded. The verified UI run uses the completed 0.29 engine.
- `scripts/fixtures/autofit.py` uses openpyxl to create the independent input. `scripts/verification/verify_autofit.py --with-ui` reads engine/UI exports with openpyxl/lxml, checking heights, offscreen multiline content, retained rich runs/font/fill/borders/alignment, comments, frozen views, source style bytes and exact Undo/Redo/reopen files. `release/previews/row-autofit.png` is an offscreen Flutter render using real macOS fonts, not a native screenshot.
- Baseline references: Microsoft's [row/column size and AutoFit instructions](https://support.microsoft.com/en-us/excel/change-the-column-width-and-row-height) and [wrapped-text row-height guidance](https://support.microsoft.com/en-us/excel/wrap-text-in-a-cell-in-excel). The point-height limit, menu/row-boundary interaction and wrapping scenarios follow those definitions. These references and automated checks do not establish exact Excel metrics or native parity.

**116 Flutter tests pass**, with two optional AI-provider checks skipped; static analysis is clean. Independent geometry, alignment, character-format, rich-text, font, style and escaped-string readers also pass against the current outputs. The macOS release built successfully. The archive is **140,566,749 bytes**, SHA-256 `eb25b62a19ee1c4b55fe39f239f42da492ca6916f93534322f09ec96a8dd6fc3`. ZIP integrity and a fresh extraction's strict/deep signature passed; **252** application/native/vendor Dart/Swift sources match staging, and the extracted Rust library and Quill license match the signed staged app. Prepared copy: `/private/tmp/folio-native-0.29.0-u05dgti5/Folio Native.app`. This is locally ad-hoc signed, not notarized. Desktop control still reports a locked Mac, so **0.29.0 has not been launched**; the existing user and isolated sessions remain unchanged. No live AI-provider or native acceptance is claimed. The explicit command stores measured custom heights. Automatic row expansion/refitting after text, formula, font, width or alignment changes is still incomplete, as are column AutoFit, merged-cell sizing, exact Office metrics, advanced alignment, neighboring overflow, comprehensive accessibility, large-corpus performance and direct Word/Excel comparison. Full parity remains incomplete; accepted novel AI features remain **0/20**.

## 0.28.0 worksheet alignment and text layout (2026-09-19)

The worksheet now offers an Alignment dialog and horizontal/Wrap ribbon shortcuts. The dialog previews the first selected cell, keeps mixed properties unchanged, validates indent/angle drafts, and applies the selected properties as one workspace operation. Other controls cannot clear an invalid numeric draft. Wrap/Shrink choices are mutually exclusive when authored. Font and Alignment dialogs send the captured revision through to the Rust command instead of substituting a newer revision at callback time.

- **102 Rust tests pass**. Two added integration cases cover direct and inherited alignment, explicit `applyAlignment="0"`, signed angle conversion, 255 stacked text, blank/new cells and new workbooks, invalid/stale atomic rejection, property-only edits, unchanged other font/fill/border/protection/number-format metadata, exact Undo/Redo and saved/reopened exports. Legacy hydration fills missing cell and default/row/column alignment properties without overriding authored choices. Ignored direct alignment records retain their qualified metadata when the effective inherited settings are materialized.
- `scripts/fixtures/alignment.py` creates the input independently, including qualified attribute lookalikes, a retained relative-indent value, parent style alignment and row/column defaults. `scripts/verification/verify_alignment.py` uses openpyxl/lxml to check edited properties, XML rotation values, retained source formatting and metadata, inherited settings after a partial edit and byte-exact history/reopen exports. Openpyxl style proxies are copied before equality comparisons; proxy-to-proxy equality is not a semantic comparison.
- The grid resolves General from the calculated value type and reading direction; numeric-looking stored text stays text. The renderer supports top/middle/bottom placement, left/center/right and wrapped justification, width-dependent wrapping, manual newlines, single-line shrink without font-size mutation, normal-font-space indentation, signed rotation and grapheme-aware stacked text. Rich font runs and script effects remain shaped together. Script struts prevent top-aligned raised glyphs from being clipped. Visible transformed bounds control extra paint layers; underline baselines use a binary line lookup and offscreen underline segments are skipped. Adjacent stacked characters with the same font share a text run. These are implementation safeguards, not a large-file performance benchmark.
- Six focused Flutter checks cover value types/direction, placement/indent, width changes, manual breaks, shrink exclusions, cached repainting, signed geometry, Unicode grapheme stacking, mixed/invalid dialog behavior and actual pixel coverage of top-aligned raised glyphs. The pixel capture uses the test framework's real asynchronous context; an initial missing wrapper stalled capture and was corrected before the passing rerun.
- The existing real-Rust font workflow also imports the alignment fixture, opens the dialog on two cells with mixed settings, changes dropdowns/numeric fields/Wrap, verifies rotation and wrapping previews, applies, exports, undoes/redoes, saves/reopens and rejects a stale dialog. `verify_alignment.py --with-ui` independently checks those exported files. `release/previews/cell-alignment.png` is an offscreen Flutter render using macOS fonts, not a native app screenshot; the first visual review identified an unavailable test font and raised-text clipping, both addressed before recapture.
- Baseline sources: [Microsoft alignment/orientation controls](https://support.microsoft.com/en-us/excel/get-started/align-or-rotate-text-in-a-cell), [wrapping and row height](https://support.microsoft.com/en-us/excel/wrap-text-in-a-cell-in-excel), and [SpreadsheetML alignment fields](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.alignment?view=openxml-3.0.1). Indent units, reading order, rotation encoding and multiline shrink restrictions follow those definitions; direct Microsoft Excel visual comparison remains pending.

- **112 Flutter tests pass**, with two optional AI-provider checks skipped; static analysis is clean. The full suite caught a new ribbon overflow and commands becoming unreachable in narrower windows. The ribbon now uses a compact alignment icon beside the font name at narrower widths and a bounded full control group when there is room. A dedicated wide/compact test covers accessibility of both paths; the existing geometry, structure, table, filter, sort and worksheet regressions pass again. The final Alignment/Font UI exports pass their independent readers, including omitted boolean attributes interpreted with their defined false defaults.
- The final macOS build completed and its ZIP/fresh extraction passed integrity and strict/deep signature checks. Archive size: **140,542,530 bytes**; SHA-256 `4a1b150562b72d3f0dff1fae185330dd2d89e6d411685098c6499537fcf5dabe`. All **251** app/native/vendor Dart/Swift sources match staging, and extracted Rust/Quill-license bytes match the signed staged app. It is locally ad-hoc signed, not notarized. Prepared copy: `/private/tmp/folio-native-0.28.0-bn0gry7m/Folio Native.app`. Desktop control again reported the Mac locked; the existing user/isolated sessions are unchanged and **0.28.0 has not been launched**. No native or live-AI acceptance is claimed.

Still incomplete: AutoFit/automatic row expansion; overflow into empty neighbors; Fill/Distributed/Center across selection, vertical Justify/Distributed and last-line justification rendering/authoring; exact combined rotation/wrapping/accounting metrics; full bidirectional/complex-script/assistive-technology qualification; large-selection and large-workbook behavior; and native packaged interaction. These limitations remain in the parity ledger. No parity family is accepted complete; novel AI acceptance remains **0/20**.

## 0.27.0 escaped Excel cell strings (2026-09-19)

Cell string handling now follows a single-pass SpreadsheetML escape codec, distinct from ordinary XML escaping. Inline/shared text and stored string values decode protected underscores, CR/LF/TAB, control codes and adjacent UTF-16 surrogate-pair escapes. Literal escape-looking text is protected on export; XML-prohibited controls and carriage returns use encoded tokens. Malformed or unpaired surrogate tokens are retained literally. Cached formula strings keep their string type, including `001`; formula source continues to use its existing XML handling.

- **100 Rust tests pass**. Five new codec/workspace cases cover independent source fixtures, literal and numeric/formula/Boolean-looking strings, Unicode/UTF-16 positions, unknown rich-run metadata, partial formatting across escaped characters, materialized stored strings, new edited cell values, refreshed formula string caches, unchanged original bytes, exact Undo/Redo, saved/reopened workspaces and legacy migration.
- Legacy source-matching imported values are corrected while actual user edits remain literal user text. Complete-token character-selection boundaries are mapped to decoded UTF-16 positions and their font overrides retained. A boundary inside an escape has no unambiguous equivalent; opening that legacy workspace fails with a specific explanation without modifying its file or replacing the active workspace. This rare legacy migration case remains unsupported and requires the previous Folio version. Repeated successful upgrades are idempotent.
- The initial broad cache-upgrade implementation changed unrelated saved formula caches after fill/sort/filter. The full-suite regressions caught this; migration now changes only source-matching stored-string cache interpretations. The final complete Rust suite passes, including those saved-workspace equality checks.
- `scripts/fixtures/escaped_strings.py` creates the XLSX input independently. `scripts/verification/verify_strings.py` uses lxml plus an independent regex/UTF-16 decoder for value checks and openpyxl for file/font checks. Installed openpyxl does not decode all SpreadsheetML tokens, so its successful open alone is not counted as text-correctness evidence. The reader verifies values, caches, original formulas, rich-run effects and qualified attributes, phonetics, other shared-string users, unchanged shared-string/stylesheet parts and exact history/reopen exports. Existing independent character, rich-text, font, style, fill and general Office readers pass.
- Codec rules were checked against [Microsoft's ST_Xstring implementation notes](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/bd0aa042-434a-4ca7-b25f-4e1fd25a954d). This is cell-value and rich-character-format coverage, not full ST_Xstring coverage of all package metadata. Sheet names, font names, number-format codes, formula-source control-character behavior, tokens split across rich runs, arbitrary malformed source files and direct Microsoft Excel differential qualification remain open.

- **106 Flutter tests pass**, with two optional provider checks skipped. Static analysis is clean. The existing native-bridge Font dialog and document/workbook/history/proofreading regressions pass against the newly built Rust library. Independent character/rich/font/style readers also pass for those freshly generated UI exports.
- The macOS release build completed. The ZIP is **140,510,751 bytes**, SHA-256 `90bea3f7839d6e3ff8d95185ec00eaa80612465472431d5f60d372c8b7dc41ac`. Its integrity and a fresh extraction's strict/deep signature were verified. All **250** app/native/vendor Dart/Swift source files match staging; extracted Rust and Quill-license bytes match the signed staged app. This is locally ad-hoc signed, not notarized. The prepared launch copy is `/private/tmp/folio-native-0.27.0-kb7f04c1/Folio Native.app`. Desktop control again reported a locked Mac, so the running user and isolated apps remain unchanged and this build has **not been launched**.

Full Word/Excel parity remains incomplete. Accepted novel AI features remain **0/20**. Native desktop launch is still unavailable because desktop control reports the Mac locked; no new native or AI-provider acceptance is claimed.

## 0.26.0 selected-character font formatting (2026-09-19)

The Font dialog now offers **Format selected characters** for one literal text cell. Mouse/keyboard selection identifies the span, a persistent highlight keeps that span visible while editing controls, and the whole-cell preview shows the partial change. Choosing a new span resets the draft. Apply is a single workspace transaction; pending cell input still commits separately. This is a dialog workflow, not yet direct rich editing or character formatting within the inline/formula editor.

- **95 Rust tests pass**. Three new cases cover edits spanning original runs, inherited/unselected properties, partial plus whole-cell font commands in either order, repeated-command compaction, Unicode boundaries, numeric/formula-looking/Boolean literal strings, stale/invalid atomic rejection, new cells/new workbooks, replaced text, exact Undo/Redo/reopen, structural moves and fill after package rebasing. Font-property validation is shared with whole-cell formatting. Source text and workspace revision must match before a character command applies.
- A disjoint, coalesced property map records the final per-character changes. Repeated identical operations do not grow a command log. Export splits only affected source runs, clones their original properties and namespace metadata, and changes requested properties. Source phonetics remain valid because text does not change. Only the selected cell's shared string is materialized. Character metadata is excluded from cell-XF generation, so character-only formatting leaves the original styles part unchanged. Stale character ranges are ignored after the cell value is replaced; newly formatting that replacement establishes a fresh text binding.
- **106 Flutter tests pass**, with two optional AI-provider checks skipped. No provider was run. The existing real-Rust Font workflow now selects a cross-run span using actual keyboard events in the read-only selector, verifies its range and preview, applies font-family/Bold changes, exports, and checks workspace Undo/Redo. Formula cells do not offer character selection. A separate Unicode/reversed-selection preview case verifies inherited whole-cell overrides and untouched neighboring font properties. The full document, proofreading, workbook and history regressions still pass. Static analysis is clean.
- `scripts/verification/verify_characters.py --with-ui` checks the output using openpyxl/lxml independently of the application. It compares every character's font, retained outline/shadow effects and qualified run/text attributes, unchanged shared-string and stylesheet bytes, phonetic data, whitespace, literal/new/replaced text cells, real-dialog exports and byte-exact recovery. The rich-text fixture now includes extra source run/text attributes and effects. Existing rich-text/font/style/fill readers also pass.
- The final compatibility audit added a stored-string (`t="str"`, without a formula) fixture. Import and character Apply succeeded, but export initially rejected that source representation. The regression reproduces the failure; the writer now materializes it as inline rich text while retaining its string value. The full Rust suite and independent character reader pass after the fix.
- `release/previews/character-fonts.png` is an offscreen test render with macOS fonts, visually inspected. The first visual review exposed a clipped scope heading and disappearing selection highlight when font controls gained focus. A compact selector/list and persistent highlight resolve those presentation problems. This is not native desktop or Microsoft Excel evidence.

Still unfinished: direct rich inline/formula-bar editing and shortcuts, preserving character styles through arbitrary typing/paste/IME changes, exact Office typography, full bidi/long-text and large-workbook performance qualification, complete font/theme/AutoFit controls, native packaged interaction and differential Excel acceptance. No Word/Excel parity family is marked complete. Accepted novel AI features remain **0/20**.

Reference behavior: Microsoft's [Characters object](https://learn.microsoft.com/en-us/office/vba/api/Excel.Characters) describes formatting a portion of cell text without changing the rest, and [text color instructions](https://support.microsoft.com/en-us/excel/change-the-color-of-text) include selecting a portion of a cell. These references do not establish complete UI or file-format equivalence.

The final **0.26.0** archive passed integrity and fresh-extraction strict deep signature checks: **140,491,640 bytes**, SHA-256 `fd8e2fbc5f4128700b2771e1bed6a577bea93ac4ed10069d310a04214cd09110`, generated 2026-09-19T13:37:49.543341+00:00. All **250** application/native/vendored Dart or Swift source files match staging. The extracted signed Rust library and Quill license match staging and the source license. A verified launch copy is recorded in `work/folio-latest-launch.json`. The release is locally ad-hoc signed, not notarized.

The final desktop-control attempt still reported a locked Mac. Existing user and isolated sessions remain preserved; native launch and interaction with 0.26.0 remain pending.

## 0.25.0 mixed character rendering and Font inspection (2026-09-19)

Imported inline and shared strings now project their individual text runs, including partial font-family/size, bold/italic/strike, underline, baseline/super/subscript and RGB/automatic color overrides. The worksheet and Font preview render those runs; mixed fields are detected within one cell as well as across a selection. Whole-cell formatting changes only selected properties and overrides those properties on each run. Toolbar font summaries and shortcuts use the active cell's run properties. Changed cell text rejects stale run positions.

- **92 Rust tests pass**. Three additional cases cover partial/inherited/explicit-false run properties, Unicode and literal numeric/formula-looking/Boolean strings, inline/shared phonetic exclusion, shared-string isolation, exact Undo/Redo/reopen, legacy metadata hydration, preservation of actual edits, fill and structural movement. Phonetic annotations remain in source XML. Old workspaces whose values exactly match the old annotation-concatenating importer are repaired; edited values are preserved. Repeated hydration is idempotent.
- **105 Flutter tests pass**, with two optional AI-provider checks skipped. No AI provider was run. The real-Rust Font workflow now opens a single mixed cell, verifies mixed family/size/bold controls and its run preview, applies Bold off, verifies another cell using the same shared string remains mixed, exports, and restores the exact original file through Undo. The prior selection/stale-dialog/pending-input tests still pass.
- New renderer tests check inheritance, explicit overrides, stale-run rejection, actual pixels for mixed sizes/colors/script positions and per-run accounting rules, clipping, semantics and cached repainting. Text shaping is shared across runs; baseline-shift groups use matching full paragraphs with other groups transparent so italic overhang is not cut at run boundaries. Only visible groups allocate extra paint layouts. An additional long-text check verifies invisible script runs do not allocate such layers. This is not large-corpus or full bidi/ligature performance qualification.
- `scripts/fixtures/rich_text.py` creates an independent XLSX fixture with rich inline/shared strings, pronunciation annotations, namespace-qualified lookalike attributes, explicit false flags, inherited styles and Unicode. `scripts/verification/verify_rich_text.py --with-ui` uses openpyxl/lxml to check the original and edited text/runs, isolated shared strings, preserved phonetic XML and byte-exact recovery. Existing independent font, style and fill readers also pass.
- Static analysis is clean. `release/previews/rich-cell-fonts.png` is an offscreen Flutter test render with macOS fonts, visually inspected: the preview retains the smaller red Times New Roman text and larger italic text, and the controls indicate one mixed cell. It is not native desktop evidence. Pixel-test development exposed assumptions about adjoining and half-covered antialiased pixels; the final checks account for that coverage while still verifying positions and effects.

Remaining work includes character selection/formatting and rich inline editing, exact Office font metrics/baseline/accounting-rule matching, theme/indexed colors, font substitution, named/default styles, AutoFit, complete long/mixed-bidi and large-workbook qualification, native packaged interaction and differential Microsoft Excel checks. Full Word/Excel parity is not achieved. Accepted novel AI features remain **0/20**.

Source model references: Microsoft's [rich text run definition](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.run?view=openxml-3.0.1), [run properties](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.runproperties?view=openxml-3.0.1) and [phonetic runs](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.phoneticrun?view=openxml-3.0.1). These references and file-reader checks do not establish full Excel rendering equivalence.

The final **0.25.0** archive passed integrity and fresh-extraction strict deep signature checks: **140,453,514 bytes**, SHA-256 `60d1230f8853a85286d7309c859026b04003a68a40d2bcd796e604ecbc7a1adf`, generated 2026-09-19T13:09:29.277629+00:00. All **250** application/native/vendored Dart or Swift source files match staging, and the extracted signed Rust library and Quill license match their staged counterparts. The verified launch copy is recorded in `work/folio-latest-launch.json`. The release is locally ad-hoc signed, not notarized. The existing objective_c framework-name warning was emitted; compilation, signing and archive verification succeeded. Independent range-sort and general Office-package readers also pass.

The final native-launch attempt still reported a locked Mac. The existing user and isolated sessions remain preserved; opening and interacting with 0.25.0 on the desktop remains pending.

## 0.24.0 worksheet font controls and rendering (2026-09-19)

The worksheet font label opens a Font dialog for the selected cells. It includes installed-family search or a portable family name, half-point sizes from 1–409, RGB/automatic color, bold, italic, strikethrough, none/single/double/accounting underlines, normal/superscript/subscript, mixed-property handling, preview, Reset, Cancel and one-transaction Apply. Grid-only Command/Control B, I, U and 1 toggle styles or open the dialog. A changed workspace rejects stale choices; a failed pending cell/formula draft prevents the operation. Simultaneous attempts to open Font share the pending result, including a blur-started commit failure.

- **89 Rust tests pass**. Three new cases cover all font properties and underline modes, half-point bounds, invalid/stale atomic rejection, original and edited recovery bytes, blank-cell default/row/column style metadata, structural transformation of column defaults, and rich-run/shared-string changes. Source font-family changes remove the old family, charset and theme-scheme hints. Other font effects and XF properties remain intact.
- Whole-cell font commands retain explicit property intent even when it matches the base XF, so turning Bold off also clears a rich run's own Bold override. The exporter changes only those properties in existing run records. It materializes only the affected shared-string cell, leaving other cells and the shared-string part unchanged. Formatting-only edits preserve cell values, rich runs and namespace-qualified metadata; value edits now also retain attribute namespace identities. This closes a separate content-flattening problem in the old style-edit branch.
- `test/font_workspace_test.dart` uses the real Rust bridge and actual Font controls. It covers Cancel, mixed selections, size validation, sample typography, Apply, retained unrelated properties, shared Undo/Redo, saved/reopened/exported data, stale dialogs, grid shortcuts and failed pending input. The font catalog is mocked in this widget test. A separate read-only AppKit check returned **182 installed families**, including Georgia and Arial; this is not a packaged method-channel interaction check.
- `test/sheet_font_test.dart` checks inherited/direct style precedence, point scaling, text-position direction, constrained layout and accessibility semantics. A pixel check distinguishes the two full-width accounting rules from an ordinary text-width underline. Repainting reuses the cached text layout; font/scale changes invalidate it. Grid drawing clips oversized glyphs within their cells and uses the system's font fallback. Inline editing uses the selected family, size and basic effects; exact script and underline editing presentation remains unfinished.
- The full regression initially caught a two-pixel toolbar overflow with a platform's larger icon-button target. A bounded icon row resolves it. The focused font workflow also caught a second open attempt returning before a failed pending commit had finished; sharing the pending dialog future fixes that race. Visual inspection caught a narrow preview sample, which now fills the dialog width. `release/previews/font-dialog.png` is an offscreen test render, not native desktop evidence.
- `scripts/verification/verify_fonts.py` uses openpyxl/lxml to check all font settings, preserved border/fill/alignment/protection/number formatting, rich run text and unrequested properties, explicit false overrides, shared-string isolation, namespace metadata and exact recovery. It also checks real toolbar-generated exports. The existing style and directional-fill readers pass.

Full regression: **102 Flutter tests passed**, with two optional AI-provider checks skipped; static analysis is clean. No AI provider was run. The independent range-sort and general Office-package readers also pass after the cell-export changes. The grid/dialog still summarize a cell with one font; mixed character rendering, character-range editing, exact Office font metrics/script positions/accounting-line placement, theme/indexed colors, named/default font styles, font substitution and AutoFit remain unqualified or unfinished. Full Office parity is not achieved; accepted novel AI features remain **0/20**.

The final **0.24.0** ZIP passed archive integrity and fresh-extraction strict deep signature checks: **140,436,898 bytes**, SHA-256 `c2f8256965ed4593f71512ffa15969fc2d2971d7e24ab0606cbdc0542baadc43`, generated 2026-09-19T12:43:47.958808+00:00. All **250** application/native/vendored Dart or Swift files match staging. The extracted app contains the exact staged signed Rust library and Quill license. A verified copy is recorded in `work/folio-latest-launch.json`; it has not been launched. The final desktop-control attempt still reported a locked Mac, so existing user and isolated sessions remain preserved and native acceptance remains pending. The release is locally ad-hoc signed, not notarized. The existing objective_c framework-name warning was emitted; compilation, signing and archive verification succeeded.

Reference behavior: Microsoft's [Format Cells controls](https://learn.microsoft.com/en-us/troubleshoot/microsoft-365-apps/excel/format-cells-settings), [font-size range and increments](https://support.microsoft.com/en-us/office/fonts/change-the-font-size), [superscript/subscript workflow](https://support.microsoft.com/en-us/excel/format-text-or-numbers-as-superscript-or-subscript), and Apple's [available font families API](https://developer.apple.com/documentation/appkit/nsfontmanager/availablefontfamilies). These sources and automated checks do not substitute for direct Word/Excel comparison.

## 0.23.0 preserve imported XLSX formats when editing (2026-09-19)

The original reproduction toggled Bold off on an imported Cambria 18 cell. The old exporter replaced its font with Aptos and discarded unrelated format details. The exporter now clones the source XF and effective font, changes only requested properties and retains the remaining attributes and children. This follows the separate font/fill/number IDs, inheritance, apply flags and alignment/protection/extension children in Microsoft's [CellFormat specification](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.cellformat?view=openxml-3.0.1).

- The five new Rust cases cover each existing formatting property, Cambria size/underline/strike retention, inherited row/column and named-style fonts, explicit false font flags, built-in date formats and unknown localized number-format IDs, old `.folio` metadata upgrades without losing user overrides, 200-cell record reuse, exact original bytes after Undo and exact edited bytes after Redo/reopen/reimport.
- `scripts/fixtures/styles.py` extends the independent openpyxl fixture with additional font metadata, built-in formats, explicit inheritance, namespace prefixes and parent-scoped extension metadata. The extension fixture places a qualified `fontId` before the normal `fontId`; reads and writes must distinguish their namespaces. The regression exposed the XML library's namespace-agnostic string attribute lookup, which is now avoided for XLSX style properties.
- `scripts/verification/verify_styles.py` independently compares fonts, fills, borders, alignment, protection, number-format IDs, formulas, untouched theme/core parts, extension children and exact recovery bytes using openpyxl/lxml. Openpyxl warns that it does not model the synthetic extension; the XML reader separately verifies its retained data. No Excel application execution is implied.
- `test/style_workspace_test.dart` drives the actual worksheet Bold and Italic buttons through the Rust bridge, commits a pending formula separately, checks Undo/Redo and workspace reload, and exports the resulting workbooks for the independent reader. Its first run completed those assertions but left the test's autosave timer pending; the harness now flushes that save before completing.

Final regression: **86 Rust tests passed**, **99 Flutter tests passed**, and static analysis is clean. Two optional AI-provider tests were skipped; no provider execution occurred. The independent style and fill readers pass, including the newly exported toolbar workflow. The worksheet's font/size controls and full visual rendering, named styles/themes, localized number/date rendering, alignment/border/merge editing and comprehensive Office comparison remain unfinished. Full Word/Excel parity is incomplete; accepted novel AI features remain **0/20**.

The final **0.23.0** ZIP passed archive integrity and fresh-extraction strict deep signature checks: **140,384,600 bytes**, SHA-256 `198b5d2c3ad406465ef07cdb8318076ba8b3ee4cdd9693c4d6777b80720f87ee`, generated 2026-09-19T12:11:38.781139+00:00. All **248** application/native/vendored Dart or Swift source files match staging, and the extracted app contains the exact staged signed Rust library and Quill license. A verified copy is recorded in `work/folio-latest-launch.json`; it has not been launched. The final desktop-control attempt still reported a locked Mac. Existing user and isolated sessions remain preserved, and native interaction/Excel comparison remain pending. The release is locally ad-hoc signed, not notarized. The build reported the existing objective_c package framework-name warning but completed signing and archive verification.

## 0.22.0 worksheet directional fill (2026-09-19)

**Fill** in the worksheet toolbar provides Down, Right, Up and Left. Ctrl+D/Ctrl+R and macOS ⌘D/⌘R act only with grid focus. The operation captures the selected range, commits pending input, uses immutable source edge cells, retains selection and returns focus to the grid. A single row or column uses its adjacent source edge and checks worksheet boundaries. All physical cells in the rectangle participate; hidden positions stay hidden. This is not yet differential qualification of Excel's filtered-range behavior.

- **81 Rust tests passed**. Five new cases cover four directions, multiple source rows/columns, mixed/absolute/cross-sheet references and quoted formula strings, out-of-bounds references, the last Excel cell, vertical structured formulas, blank/style clearing, hidden geometry, shared-formula materialization, native/imported recovery, exact original bytes after Undo and exact edited bytes after Redo/reopen/reimport. Stale, invalid, oversized, unsupported-formula, merged, array and protected operations leave the workspace unchanged.
- Imported fills copy the complete source cell record into a disposable XLSX package, including full XF formatting and rich text. Source row/column styles become explicit at destinations. Row geometry remains attached to worksheet positions. Copied cell attributes retain their namespace identities, including namespace declarations inherited from a source row. All formula caches are recalculated after the package edit, including dependent formulas on other worksheets. Source packages remain available for Undo.
- `scripts/fixtures/fill.py` independently creates normal and prefixed XLSX fixtures with shared formulas, Cambria font/size/underline/strike, borders, rotated/wrapped alignment, protection flags, numeric formats, rich text, numeric-looking literal strings, inherited styles, hidden/sized geometry, merged/array objects and row-scoped metadata. `scripts/verification/verify_fill.py` uses openpyxl/lxml to check complete source style equality, rich-run equality, text types, copied metadata, translated formulas/caches, default-format clearing, retained geometry/objects and recovery. Both fixture variants pass.
- The focused real-engine widget workflow and existing field-history regression passed. The new workflow uses the actual toolbar menu, range address field and Ctrl+D/Ctrl+R; it verifies pending formula commits, separate Undo/Redo, retained selection/focus, style copying, recalculation, field shortcut isolation, saved data and XLSX export. A rejected 10,001-character pending value does not fill the older value. Cell/formula commits now share an in-flight result and retain failed drafts so a blur-started commit cannot race the fill.

Final Flutter verification: **98 regular tests passed**, with two opt-in AI-provider checks skipped, and analysis is clean. The initial full run exposed toolbar controls pushed outside the viewport; compact vertical grouping restored their accessibility. A subsequent run caught a 10-pixel overflow with a platform's larger button touch target; an explicit button height fixed it. The final complete rerun passed, including existing row/column, geometry, table, keyboard/history, document and spelling workflows. The final focused preview workflow also passes with real macOS fonts. `release/previews/fill-cells.png` is an offscreen test render, visually inspected; it is not native desktop evidence. No AI-provider execution occurred.

Sources: Microsoft's [directional formula fill](https://support.microsoft.com/en-us/excel/fill-a-formula-down-into-adjacent-cells), [FillDown contents/format contract](https://learn.microsoft.com/en-us/office/vba/api/excel.range.filldown), and [keyboard shortcuts](https://support.microsoft.com/en-us/accessibility/excel/keyboard-shortcuts-in-excel). Remaining work includes native packaged and differential Excel qualification; fill handles/series/Flash Fill/across-sheet options; notes, hyperlinks, validation and conditional-rule copying; filtered and visible-only semantics; horizontal structured references; complete formula/array/spill/merged/protection behavior and higher workbook limits. Full Office parity remains incomplete; accepted novel AI features remain **0/20**.

The final **0.22.0** ZIP passed archive integrity and fresh-extraction strict deep signature checks: **140,358,061 bytes**, SHA-256 `c9cca36064d8868ac7302c7c904d8f0fa5fede4cec03e374234639ca6c907584`, generated 2026-09-19T11:42:55.765654+00:00. All **248** application/native/vendored Dart or Swift sources match staging. The extracted app contains the exact staged signed Rust library and Quill license. A verified copy is recorded in `work/folio-latest-launch.json`; live launch remains pending desktop access. The final desktop-control attempt still reported a locked Mac; the existing user and isolated sessions remain preserved. The release is locally ad-hoc signed, not notarized.

## 0.21.0 paragraph tab leaders (2026-09-19)

**Tab stops…** now offers None, Dots, Dashes, Solid line, Heavy line and Middle dots. Selecting a stop restores its leader choice; editing only the leader and applying is one workspace Undo step. Unknown imported leader values remain preserved and identified as incomplete previews. The three controls fit the dialog without overflow.

- **97 Flutter tests passed**, with two optional AI-provider checks skipped; static analysis is clean. The real Rust-backed dialog workflow covers rich preview, Cancel, Apply, a leader-only edit, Undo/Redo, immediate typing, stale-state rejection, save/load and DOCX reimport. The existing clipboard/composition/Tab-key and spelling-menu workflows now also run with dotted leaders before the affected text. Actual suggestion mouse down/up replaces the word, retains formatting and supports Undo; the menu separator remains verified.
- Pixel tests on real macOS Arial distinguish all six choices, constrain leader ink to the final tab gap, retain its caret/nearest-edge geometry and unchanged literal-tab text/semantics, and verify removal by switching to None. Tests include an RTL leading tab and blue heavy leaders in a static table cell. They do not establish exhaustive Word typography or native accessibility behavior.
- Leader drawing uses a Folio-owned inline paint interface in the existing Quill paragraph extension. Decorations are recorded once per text/layout change and reused for repaints, then disposed on invalidation/disposal. Tests verify cache reuse and invalidation on text, width, scale and color changes. Glyph leaders use the text's font/style and baseline; line leaders use its color and scale. Rendering does not insert repeated characters into model text or clipboard data.
- **76 Rust tests passed**. The added case verifies all six leader values, a single literal tab per field, bold formatting, workspace save/reopen, DOCX reimport and byte-identical reimport/export. The independent python-docx/lxml reader confirms all six enums and 216-point right stops in original, reopened and reimported exports. Existing tab inheritance, settings, atomic edits and exact Undo checks pass.
- `release/previews/tab-leaders.png`, `tab-stops.png` and `tab-aligned-document.png` are offscreen Flutter test renders, not native screenshots. The gallery and dialog were visually inspected; the initial cramped dropdown layout was corrected before the final full-suite run. No AI-provider execution is claimed.

Checked source: Microsoft's [TabStopLeaderCharValues reference](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.tabstopleadercharvalues?view=openxml-3.0.1). Exact Microsoft Word leader font/phase/thickness matching, complex wrapping and mixed bidi, bar/ruler controls, implicit hanging-indent and table-style tabs, inheritance refresh after style/list changes, indentation/justification, inline objects and native input/assistive-technology qualification remain open. Full Word/Excel parity is incomplete; accepted additional novel AI features remain **0/20**.

Desktop control still reports a locked Mac. The existing user and isolated test sessions are preserved; live launch and interaction with this release remain pending.

The **0.21.0** ZIP passed archive integrity and fresh-extraction strict deep signature checks: **140,337,512 bytes**, SHA-256 `68c1df3400f9b0f52fa1c89a3d7cceaf221e20b3d1da4cdc9106362f08383ee4`, generated 2026-09-19T11:16:10.274473+00:00. All **248** application/native/vendored Dart or Swift source files match staging. The extracted app contains the exact staged, signed Rust library and Quill license. The recorded Quill patch reconstructs all four modified or added upstream files exactly. A verified launch copy is recorded in `work/folio-latest-launch.json` and has not been launched. This release is locally ad-hoc signed, not notarized. The initial verification script looked for the license at the wrong path; the final check uses the actual bundled `third_party/flutter_quill-LICENSE.txt` and confirms its bytes against upstream.

## 0.20.0 custom and imported paragraph tabs (2026-09-19)

The new **Tab stops…** control edits the effective stops of one or more selected document paragraphs, previews the first paragraph with its existing rich text and shared Quill styles, and applies paragraph stops plus the document default interval in one workspace transaction. The dialog supports left, center, right and decimal stops; mixed selections are identified, selecting through the next paragraph’s start does not format that paragraph, and source/revision changes reject stale previews. Imported leaders and other stop types are retained with an explicit incomplete-preview notice.

- **92 Flutter tests passed**, two optional AI-provider checks skipped; analysis is clean. The new workspace workflow uses the real Rust bridge, exercises Cancel, Apply, immediate typing, separate Undo/Redo, stale-preview rejection, save/load and DOCX reimport. It measures the actual document paragraph’s right-aligned field, checks rich formatting, and verifies settings remain after subsequent typing. Geometry checks cover all four alignments, automatic intervals after custom stops, mixed-size decimal prefixes and a comma-decimal locale. Multi-paragraph formatting preserves identities/marks, clears each paragraph’s distinct inherited stops and inverts exactly.
- **75 Rust tests passed**. New cases cover direct clears, document-default and paragraph-style inheritance, numbering-level overrides, style cycles, nested cell paragraphs, alternate XML prefixes and relocated settings/styles parts; text/format retention, new settings parts with valid OPC relationships/content types, atomic invalid/stale edits, legacy workspace hydration, save/reopen/reimport and byte-identical original DOCX export after Undo.
- `scripts/verification/verify_word_tabs.py` uses python-docx and lxml independently of the application. It confirms right/clear stops and a preserved dash leader, default spacing, paragraph styling, source styles/header/relationship/content-type bytes, nested table text, reopened/reimported exports and byte-identical Undo. The fixture generator is `scripts/fixtures/word_tabs.py`.
- Root document settings now survive content-only document changes and AI/proofreading replacements. The normal editor codec accepts explicit root attributes; the Rust document/proposal paths preserve existing settings when a content-only caller omits them. The default interval is projected from `w:defaultTabStop` and patched into its existing package part without overwriting unrelated settings.
- Static table previews now use the same tab shaper as the Quill cell and document editors. No Flutter SDK or additional vendored Quill files changed. The full existing table, proofing, conversion, keyboard/input-client, worksheet and history regressions passed.
- `release/previews/tab-stops.png` and `tab-aligned-document.png` are **offscreen test-rendered previews**, inspected after loading actual macOS fonts. They are not native screenshots. Initial verification defects were corrected: the alternate-prefix fixture initially collided with Word’s existing VML prefix; a reader assertion guessed the library’s CLEAR enum value; and the new widget finder initially excluded the RichText subclass. The final artifacts and tests use the corrected fixtures/readers/finders. The initial preview also normalized text formatting; the final preview uses a read-only Quill paragraph with the original rich Delta and shared document styles.

Scope still open: bar and leader rendering/authoring, ruler UI, implicit hanging-indent stops and table-style tabs, inherited-stop refresh after changing paragraph styles or list levels, exact long/aligned/wrapped-field and full bidi behavior, indentation/justification interaction, inline objects, native input/assistive-technology qualification and actual Microsoft Word comparison. Checked sources: [paragraph tabs](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.tabs?view=openxml-3.0.1), [individual stops](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.tabstop?view=openxml-3.0.1), and [default interval](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.defaulttabstop?view=openxml-3.0.1). These specifications and passing local checks do not establish full Word parity.

Native launch is still pending the locked desktop. Existing user and isolated navigation sessions are preserved. Full Word/Excel parity remains incomplete, and accepted additional novel AI features remain **0/20**. The **0.20.0** ZIP passed archive integrity and fresh-extraction strict deep signature checks: **140,319,056 bytes**, SHA-256 `e5902b10ad0d02788224f784869b3cf3875b1918777abb0a755861368965092f`, generated 2026-09-19T10:51:48.217159+00:00. All 245 application/native/vendored Dart or Swift source files match the build staging copy. The freshly extracted app contains the exact staged, signed Rust library and upstream Quill license. Its verified launch path is recorded in `work/folio-latest-launch.json`; it is not yet launched. Local ad-hoc signing is retained; this is not a notarized release.

## 0.19.0 measured default tabs (2026-09-19)

The editor now lays out literal tabs on a default 48-logical-pixel grid (half an inch at the page surface's 96-pixel/inch scale). Left-to-right paragraphs use character-preserving spacer spans; the complete styled source still contains one U+0009 per tab. Cursor positions, text traversal and clipboard data retain their original offsets. RTL paragraphs use measured native tab-glyph spacing because the current placeholder probe returned swapped range rectangles for two RTL placeholders. Configured RTL cases with leading/repeated tabs pass; this is not exhaustive mixed-direction or imported Word direction qualification.

The application has a small, pinned Flutter Quill 11.6.0 fork in `vendor/flutter_quill`. It adds an optional paragraph-layout scope and a RenderParagraph subclass for non-widget spacers and nearest-edge hit testing. The layout callback receives the actual line width after Quill creates rich, composing and diagnostic spans. Only three existing upstream library files changed; one layout extension was added. The original 265 library files have recorded SHA-256 identities, the patch and provenance are documented, and the upstream MIT license is included in the distribution. Flutter SDK source was not modified.

- **87 Flutter tests passed**, two opt-in AI-provider checks skipped; analysis is clean. Model/renderer tests cover leading, repeated and final tabs, Unicode and combining text, mixed sizes, recognizers and diagnostic decoration, selection geometry, nearest-edge clicking, resizing/text scaling, many visual wraps and configured RTL stop positions. The no-tab path retains the original span tree.
- A Quill editor workflow verifies actual Tab key insertion and cursor placement, its Undo, live composition and commit, exact tabbed clipboard text, and Undo back to the original document. These are macOS widget/input-client checks, not native IME qualification.
- The existing real Rust-backed spelling-menu test now runs both with ordinary spaces and with tabs before the bold misspelling. Both use mouse down/up on the suggestion and verify correction, formatting, menu separator, subsequent review, selection and Undo. Two new-test setup defects were corrected: the bridge must initialize once per test process, and initial Quill document content must be loaded rather than inserted into its Undo history.
- The selected-row conversion workspace test passes with the new renderer, covering the unchanged conversion/Cancel/stale-source/typing/Undo/export/reopen/reimport paths. The independent DOCX reader still verifies real tab elements, both fragments, formatting and byte-identical Undo/reimport exports. No Rust implementation or AI-provider execution changed.
- The first measured implementation repeatedly shaped the full paragraph. The debug probe measured 500 tabs / 2,000 characters at **1,392 ms**. The final algorithm measures each text segment once, carrying its starting horizontal position, and the corresponding probe measured **16 ms**. These are single local test-environment samples, not release/native responsiveness benchmarks. The complete repeated-wrapping geometry check passes across 120 mixed-size fields and three viewport widths. Evidence: `work/tab-layout-benchmark-before.log`, `work/tab-layout-optimized.log`, `work/folio-0.19-tests.log` and `work/folio-0.19-analyze.log`.

The scope is default-tab rendering in the Quill document and cell editors. Static table-cell previews, imported custom/default tab settings and style inheritance, user-facing tab/ruler controls, left/center/right/decimal/bar choices, leaders, paragraph indentation/justification interaction, inline-object mixtures, complete bidi behavior, native IME/accessibility and actual Microsoft Word comparison remain open. Microsoft's [defaultTabStop specification](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.defaulttabstop?view=openxml-3.0.1) and [paragraph tab-stop specification](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.tabs?view=openxml-3.0.1) were checked; they do not substitute for those missing workflows and comparison checks.

The latest desktop-control observation still reports a locked Mac. Native launch and the outstanding conversion/field-history/Save/restart checks remain pending. Existing user and isolated navigation sessions are untouched. Full Word/Excel parity remains incomplete; accepted additional novel AI features remain **0/20**.

The **0.19.0** ZIP passed archive integrity and fresh-extraction strict deep signature checks: **140,259,741 bytes**; SHA-256 `58a68a0fcbfef2c17c5a75eaf20143e9f5be39b0c10212812f622e1de7d1f819`; generated 2026-09-19T10:22:32.384985+00:00. All 242 application/native/vendored Dart or Swift files match staging, and the extracted package contains the exact upstream Quill license. A separately verified launch copy is recorded in `work/folio-latest-launch.json`; it has not been launched. The package is locally ad-hoc signed and not notarized.

## 0.18.0 selected table rows to text (2026-09-19)

The conversion preview now accepts a contiguous first/last row range. The table editor also offers **Convert rows to text**, initialized from its selected cells. **Convert and apply** commits current table drafts and conversion in one workspace revision; Cancel returns to the draft, and cancelling the parent editor leaves the saved document unchanged. Rows above and below remain separate tables with their source formatting, widths and merge geometry. A row boundary intersecting a vertical merge expands until all affected merged cells are included, and the preview explains that expansion before Apply. This is explicit Folio behavior, not verified equivalence to Word's merge-selection behavior.

The document editor applies the replacement as one transaction and explicitly collapses the caret after it. A changed source table invalidates a pending conversion with a visible explanation. Table source references are retained for original formatting; each fragment keeps only its own row/cell source references.

- **78 Flutter tests passed**, with two opt-in provider checks skipped; static analysis is clean. New model cases cover middle/first/last/all row ranges, invalid ranges, chained vertical merges, source immutability and typing at the document end without removing either remaining table.
- The real Rust-backed workspace widget workflow covers selecting rows, range expansion, invalid input, editing a cell before conversion, both Cancel paths, one-revision Apply, collapsed caret, subsequent typing, separate Undo of typing and conversion, Redo, saved-workspace reload, DOCX reimport/export, and refusal of a stale preview. An initial full-suite failure was a test-harness guard conflict caused by an assertion inside the asynchronous pump helper; moving that assertion after the helper fixed it. The full rerun passed. Evidence: `work/folio-0.18-tests.log` and `work/folio-0.18-analyze.log`.
- An independently authored python-docx fixture includes multi-paragraph rich text, a nested table, horizontal and vertical merges, unequal widths, shading, a repeating header row, exact row height, paragraph settings and a document header. The independent python-docx/lxml reader verified text order, both retained fragments, formatting, actual Word tab elements, merge XML, untouched header bytes, byte-identical original-file export after Undo, canonical-equivalent export after workspace reload, and byte-identical DOCX reimport/export. Reproduce using `test/document_table_rows_workspace_test.dart` followed by `scripts/verification/verify_word_row_conversion.py`.
- Optional offscreen widget previews are in `release/previews/selected-row-conversion.png` and `release/previews/converted-row-document.png`. The capture loads local system/interface/icon fonts and substitutes Arial for Aptos. These show the preview layout and converted result; they are not native desktop screenshots. Inspection confirmed readable controls without overflow, and also the existing incomplete tab-stop layout: text retains tabs in the document/DOCX but the editor does not position them at Word tab stops.
- No Rust implementation or native runner source changed. Their prior scoped verification remains applicable; the version-only Rust change does not constitute new engine behavior. No real AI-provider run occurred.

Packaged native row-conversion, immediate Undo/Redo, IME, Save return and restart qualification remain pending. Desktop control still reports a locked Mac. The user and isolated navigation sessions remain untouched. Actual Word comparison, disjoint row selections, partial-cell conversion, AutoFit, tab-stop layout and the remaining parity families are unfinished. Full Word/Excel parity remains incomplete; the separate novel-AI acceptance count is **0/20**.

The **0.18.0** distribution ZIP passed integrity and fresh-extraction strict deep signature checks: **140,235,337 bytes**; SHA-256 `2ed02fad137c0dcca8648f6cea82e2a4f3cee08e562b9a0926d33c6dd528f5aa`; generated 2026-09-19T09:51:21.390077+00:00. All 30 production Dart/Swift files match staging. A separately extracted and signature-verified launch copy is prepared in `work/folio-latest-launch.json`; it has not been launched. The package is locally ad-hoc signed and not notarized.

## 0.17.3 immediate draft Undo and Redo (2026-09-19)

The 0.17.2 first-edit timing issue is now reproduced and fixed in the application layer. Removing the history-seeding delay from the real workspace regression failed at the assistant prompt: Command-Z left the first draft unchanged. `draft_text_field.dart` records a field's initial value synchronously. Before Undo it commits pending typed text into the local history, so Redo retains that text even if Undo occurs before the typing-coalescing timer fires. Subsequent typing coalesces in 500 ms groups; moving the selection or leaving a field closes the current group. A new edit discards the old redo branch, while cancelling an uncommitted IME composition does not.

Every app-authored plain TextField/TextFormField now uses the shared adapter, including conversion/formatting/filter dialogs, assistant A2UI fields, Find/Replace, naming/address controls and worksheet drafts. The adapter renders the standard Flutter field with its existing key and options and restores values through the public `EditableTextState.userUpdateTextEditingValue` path, preserving input formatters, selection, callbacks and the input connection. It owns only local draft history; Quill and the Rust workspace retain their existing committed-edit histories. No Flutter SDK files or native runner implementation were changed.

- **73 Flutter tests passed**, with two opt-in provider checks skipped; static analysis is clean. The new focused checks cover empty/nonempty Unicode initial values, immediate/delayed Undo, immediate Redo, selection restoration, typing groups, caret movement, redo branching, switching fields, selection-only blur updates, externally supplied values, controller replacement, read-only state, form callbacks and input formatters. First-edit tests run under macOS, Windows and Linux widget variants; these are not native cross-platform tests.
- The IME widget check retains live composition on Undo, records completed composition as one edit and preserves Redo after composition cancellation. A cancellation regression first failed before its branch-preservation fix. Native input-method, composition-window and platform Undo/menu qualification remain open.
- The real-engine workspace test now types immediately after focus, with no baseline wait, across prompt, Find/Replace, conversion fields, formula input and inline cell input. It still verifies unchanged document/revision/saved bytes for local drafts, one-revision formula commit, cell/history isolation, and document/grid workspace Undo/Redo. The existing conversion, table, proofing, search, geometry and other workflow checks passed in the full suite.
- Before/after evidence is in `work/rapid-workspace-before.log`, `work/rapid-workspace-after.log`, `work/draft-composition-before.log` and the final `work/folio-0.17.3-tests.log`. The stock Flutter probe remains a framework observation; it does not use the new adapter. No new AI-provider execution or Rust implementation test run occurred. The unchanged Rust/native components retain their prior scoped evidence.

Packaged native acceptance remains pending, including the unfinished 0.17.1 conversion/Undo/export/reopen sequence and the new immediate field-history behavior. The last desktop-control check reported a locked Mac; the existing user and isolated test sessions have not been replaced. Full Word/Excel parity and the separate novel-AI acceptance gate remain incomplete, with **0/20** accepted novel AI features.

The **0.17.3** ZIP passed fresh-extraction strict deep signature verification: **140,221,018 bytes**; SHA-256 `71bedc568c767f33530ca90723a49afeee4ee651aa41773da5e297329bd9a382`; generated 2026-09-19T09:21:51.808107+00:00. All 30 production Dart/Swift files match the package staging checkout. The latest post-build desktop-control attempt still reported a locked Mac. A clean launch copy is prepared and recorded in `work/folio-latest-launch.json`, but has not been launched. The package remains locally ad-hoc signed and not notarized.

## 0.17.2 focused-field and formula draft history (2026-09-19)

The workspace-level Command-Z binding intercepted text-field history in the assistant prompt, navigation fields and worksheet drafts. A real-engine widget regression first failed because Undo left `Explain the budget` in the prompt rather than restoring its initial value. The keyboard action now delegates to the focused field's existing Flutter Undo/Redo action when present, including when its history is exhausted. It does not flush a formula or cell draft into the workspace before performing local Undo. Quill's document history action and the grid's workspace fallback remain separate.

Enabling local history exposed a second defect: after editing B4's formula draft and moving to A1, Undo could put B4's `2400` into A1's formula field instead of retaining `PROJECT BUDGET`. The formula field now gets a fresh history when the sheet/cell identity or committed cell value changes. Its controller, focus node and existing commit behavior are retained.

- **66 Flutter tests passed**, two opt-in provider checks skipped; static analysis is clean. The new native-bridge widget workflow checks local Undo/Redo and exhausted history in the assistant prompt, Find, Replace, conversion width/column inputs, formula bar and inline cell editor. It verifies unchanged document/revision and byte-identical saved workspace through draft edits and Cancel, cell-to-cell history isolation, document/grid workspace shortcuts, one-revision formula commit and its workspace Undo/Redo, and a clean local history on refocusing the committed formula.
- The original prompt failure is recorded in `work/field-history-before.log`; the cross-cell failure is in `work/formula-history-before.log`. Focused workflow checks and the full release run are in `work/field-history-focused.log`, `work/field-history-commit.log` and `work/folio-0.17.2-tests.log`. No Rust implementation or native runner source changed in this release. Their prior scoped checks remain the evidence for those components; they were not rerun solely for a version bump.
- **A separate first-edit Undo issue remains.** A minimal unmodified Flutter TextField probe on the installed SDK (3.44.7) typed immediately after focus, waited 600 ms and then pressed Command-Z: `hello` remained. Waiting 600 ms before typing allowed Undo to restore the initial empty string. The SDK's UndoHistory defers its first snapshot using the same 500 ms coalescing timer as later changes. This gives a source-level lead for the earlier native width-field symptom, not proof of that native cause. The release regression establishes each field's initial history before editing and does not qualify this rapid first-edit path. Diagnostic source/results are in `work/flutter-probes/rapid_field_probe_test.dart` and `work/rapid-field-probe.log`; its reported test pass only means the observation ran, not that both behaviors are correct.
- **Packaged desktop checks remain pending.** Desktop control again reported a locked Mac after packaging. The original 0.16.1 user session and isolated 0.17.1 navigation session are still running; neither was replaced. The isolated session's unfinished conversion/Undo/export/reopen checks and backed-up revision-30 workspace remain as described below. Native dialog Undo, IME, accessibility, keyboard/menu routing and cross-platform qualification are not complete.

The **0.17.2** ZIP passed fresh-extraction strict deep signature verification. It contains **140,219,459 bytes**; SHA-256 `ee544bb79379affe2690dd787b3ce1f3a758c687563d5253cef69da7bb73967d`; generated 2026-09-19T09:05:29.535155+00:00. All 29 production Dart/Swift files match the package staging checkout. This is a local ad-hoc-signed build, not notarized. Word/Excel parity remains incomplete and the separate novel-AI acceptance gate remains **0/20**.

## 0.17.1 keyboard recovery and conversion caret (2026-09-19)

The 0.17 keyboard symptom is reproduced specifically after typing into a Flutter dialog field and closing it. Normal document typing, keyboard Undo/Redo, native Save return and conversion without a typed field all worked in the comparison run. Temporary native tracing then captured an active, key window whose `firstResponder` was `MainFlutterWindow`; Command-Z reached the window but never its Flutter controller or Dart key handler. Dart focus had already returned to a `FocusNode`. This is an input-routing failure, separate from the earlier rendering lifecycle issue.

`restoreFlutterResponder` now recovers window/missing responders through the public Flutter view controller. The existing grid recovery remains; active text-field responders are retained unless Dart owns grid focus, and an attached native sheet prevents either recovery path. The adapter still normalizes aggregate modifier bits. Temporary tracing was confined to a staging checkout and is absent from production and the final package.

Conversion also now explicitly collapses the caret after the inserted content. Quill 11.6's `compose` transforms the old selection instead of using its requested selection argument: the new regression first failed with selection 18–20 instead of caret 20. Both conversion directions now set the intended caret explicitly, avoiding replacement of a newly created table by the next typed character.

- Flutter: **65 passed**, two opt-in provider checks skipped. The conversion workflow now exercises actual Command-Z/Shift-Command-Z events on macOS and checks the caret in both directions, alongside prior Apply/Cancel/stale-state/Undo/export checks.
- Rust: **70 passed**. Native keyboard adapter: **39 in-memory checks**, including named window/missing-responder, active-field, attached-sheet and grid cases. Lifecycle harness passed. Static analysis is clean. Independent Word-conversion export checks passed.
- Clean packaged native evidence so far: inserted X at revision 2; typed a conversion width and Cancel retained revision 2; Command-Z removed X at revision 3; Shift-Command-Z restored X at revision 4. Typed width 120 and Convert produced revision 5. Immediately typing Y produced revision 6 with the table intact and `YAfter conversion` in the following paragraph. The saved revision-6 artifact confirms the content. This passed the previously failing empty native responder path without clicking the document to recover focus.
- **Still pending for this release:** remaining Undo/Redo after Apply and typing, native Save/Cancel return, native exported-file verification and restart. The Mac locked before the next Command-Z could be sent; the control tool reported that automatic unlock failed. The isolated test app remains open at revision 6, with its original revision-30 workspace backed up at `work/navigation-before-shortcuts.folio`; restore that backup only after quitting the test app. The user's original 0.16.1 session remains separate.
- An Undo attempt inside the active width field changed neither that field nor the workspace in the native check. Local text-field Undo is not accepted as complete and needs its own follow-up. Full keyboard/IME/accessibility coverage remains incomplete.

The **0.17.1** ZIP passes fresh-extraction strict deep signature verification. Size: **140,214,346 bytes**; SHA-256: `34856b1ce5f233b893aa3d1d3b361a2a723167ecf863ab33d204649573c9ce69`; generated 2026-09-19T08:47:27.889817+00:00. All 29 production Dart/Swift sources matched the package staging checkout. It remains locally ad-hoc signed, not notarized. The broader Word/Excel parity goal and the separate 20-feature AI gate remain incomplete.

## 0.17.0 text and table conversion (2026-09-19)

Selected text can become a table using tabs, commas, paragraph breaks or a custom Unicode character, with configurable column count and fixed width. Partial selections preserve surrounding prose; rich runs and imported paragraph property references are retained. Whole tables can become text, with optional nested-table conversion and empty fields for covered merged positions. Incompatible list/opaque block boundaries remain separate. Preview, invalid input, Cancel, one-transaction Apply, stale-document rejection, Undo/Redo and persistence are exercised.

- Flutter suite: **65 passed**, two opt-in provider checks skipped. After making preview validation follow controller changes, all six affected conversion tests passed again, including direct controller-update coverage. Static analysis is clean.
- Rust suite: **70 passed**. New checks cover formatted Word `w:tab` elements and unchanged DOCX export through the workspace resolver. The resolver no longer inserts null content fields into text leaves and therefore does not spuriously rewrite untouched paragraphs.
- The independent `python-docx`/`lxml` conversion reader verifies 2×2 content, bold/italic runs, cloned keep-next/spacing/indent properties, column widths, retained headers, real tab elements, reopened exports and byte-exact original DOCX after Undo. Existing merged-table, table-layout and numbering readers also passed on the regenerated artifacts.
- Final packaged app: keyboard text selection opened the expected 2×2 preview; typed zero columns showed validation; typed 120-point widths changed the preview; Convert committed revision 2. Toolbar Undo/Redo produced revisions 3/4. Table-to-text preview and Convert produced revision 5, with separate toolbar Undo/Redo at 6/7. Native Save panels exported both states to DOCX. The independent native reader verified cell text, widths, bold/italic runs, imported paragraph properties, retained headers and true Word tab elements. Quit/relaunch showed the same text at revision 7; the saved file was byte-identical. The isolated test workspace was restored to its prior revision 30 after quitting.
- Native automation's AX `setValue` did not reliably deliver Flutter text-field edits, even though direct controller updates now invalidate the preview. Keyboard typing was used for native validation. This is not a passed native accessibility audit. A Command-Z attempt after the Save panel, with accessibility focus on the window rather than the editor, did not change the revision; toolbar Undo/Redo passed. The keyboard/focus path remains unqualified and needs a targeted follow-up.

The final **0.17.0** ZIP passed strict deep signing verification after fresh extraction. Size: **140,214,906 bytes**; SHA-256: `2a408554757a1ddeb48d04df5eb888a57f37aea0ef78fafc70ca1a83fb5e7f90`; manifest generated 2026-09-19T08:26:41.030958+00:00. All **29** production Dart/Swift source files match the package staging checkout. The package is local, ad-hoc signed, and not notarized. The user's running 0.16.1 session was not replaced.

Reproduce with `flutter test`, `cargo test --manifest-path rust/Cargo.toml`, and `scripts/verification/verify_word_conversion.py` using the bundled Python runtime with python-docx/lxml. Set `FOLIO_CORE_PATH` to the freshly built Rust release library for Flutter tests. `word_conversion_fixture.py` generates the independent source fixture; `verify_native_word_conversion.py` checks the separate native interaction artifacts. Logs are in the thread's `work/folio-0.17-*` files.

**Not accepted as complete Word parity:** converting selected rows out of a table, Word AutoFit modes, direct tab-stop layout, full paragraph/list/opaque-object equivalence, all merged/nested/RTL cases, huge-table performance/accessibility and actual Microsoft Word comparison. Conversion supports up to 1,000 rows/columns and 100,000 cells; the dialog renders the first ten preview rows. Full Word/Excel parity and the separate 20-feature AI gate remain incomplete, with **0/20** novel AI features accepted.

## 0.16.1 foreground-window rendering recovery (2026-09-18)

Temporary traces reproduced a framework lifecycle failure, rather than a blocked editor handler: a ribbon click arrived while Flutter reported `AppLifecycleState.hidden`, `framesEnabled=false`. On the external Odyssey Ark display, AppKit could report the app active, its window key/visible/not minimized, and both occlusion flags as 8192 (visible bit absent). Moving back from the built-in Retina display produced this contradictory state while the app remained active, so activation-only recovery was insufficient. The installed Flutter 3.44.7 source retains the older visibility-dependent activation handler. The activation aspect matches [Flutter's upstream diagnosis and fix](https://github.com/flutter/flutter/pull/188772); the additional display-transfer evidence comes from this isolated local reproduction.

`FolioActivationRecovery` coalesces application activation, key-window, restoration, screen-change and occlusion notifications, then rechecks native state after Flutter's handlers run. It sends the public Flutter lifecycle message only when this window is active, key, visible and not minimized, and the app is not hidden. Screen sleep and session suspension independently inhibit recovery. Other windows' notifications cannot target this window's recovery; queued work uses weak references and observers are removed on teardown. There is no continuous timer, private engine-state mutation, system preference change or document change. `MainFlutterWindow` supplies native state and the engine messenger.

The production Swift notification harness verifies notification ordering, coalescing, display-transfer recovery, inactive/non-key/hidden/minimized/closed-window gates, both orders of waking screens and restoring a session, foreign-window filtering, queued cancellation and teardown. Run `python3 scripts/test_macos_lifecycle.py`; it compiles the production helper and uses synthetic notifications without launching or controlling a UI. All **29 existing in-memory AppKit keyboard checks** also pass. The full regular Flutter suite passed **59 tests**, including the mouse spelling-menu/divider/Undo regression; two opt-in live-provider checks were skipped. All **68 Rust tests** pass and static analysis is clean. Later changes were restricted to the native helper and its harness; those final sources were recompiled, tested and packaged. No new AI-provider run occurred.

The final clean package, with temporary tracing removed, passed the previously failing native sequence: activate through the Window menu on Odyssey Ark, move to Built-in Retina Display, switch to Review, move back to Odyssey Ark, switch to Home and back to Review. Each ribbon drew without a resize or another display move. A separate fresh launch on the external display passed a coordinate right-click on bold `recieved`, a visible divider below the three suggestions, and a coordinate click on `received`. The document changed at revision 31; toolbar Undo restored the typo at 32 and Redo restored the correction at 33. The saved model retained bold formatting. A complete quit/reopen visibly retained the corrected text, and the saved/reopened workspace bytes matched exactly. The preview was closed and its process exit verified before restoring the prior revision-30 worksheet sample byte-for-byte. The original user workspace was not used.

Qualification limits: the automation can restore a window while leaving its app inactive and non-key; traces confirmed that state, and recovery correctly refuses to force foreground rendering in it. One traced restoration followed by foreground activation resumed drawing, but the final native minimize/restore sequence is not accepted as fully qualified. Physical sleep/wake, repeated lock/unlock, monitor disconnection, Mission Control and background-visible rendering still need native checks. The synthetic suspension tests do not substitute for those physical workflows. This fixes the reproduced active-window activation/display-transfer case, not every possible rendering or platform issue. No parity family is complete; novel AI acceptance remains **0/20**.

The **0.16.1** ZIP passes strict deep signature verification after fresh extraction. Size: **140,185,782 bytes**; SHA-256: `83dfb90b39465451461efb44cb0cd31991763b55afbce1e8e35c4448d707f3b8`. Manifest generated at 2026-09-18T20:10:35.805468+00:00. All 27 staged Dart/Swift source files match production, with temporary tracing absent. The package remains ad-hoc signed and not notarized.

## 0.16 Word table dimensions and appearance (2026-09-18)

Historical 0.16 results follow. The 0.16.1 section above records the subsequent display diagnosis, scoped recovery and remaining lifecycle qualification.

Table properties now edits selected column widths in points, equal distribution, automatic/at-least/exact row heights, solid cell fills, cell vertical alignment and whole-table alignment. Only changed fields enter the table draft; the enclosing Apply remains one workspace revision/Undo. Fixed-width tables use twips converted at 96 logical pixels per inch and scroll horizontally when wider than their viewport. Exact rows clip visual content without changing the underlying rich paragraphs; minimum and automatic rows expand for content. Merged-row height distribution grows only flexible rows. Imported direct fills and vertical alignment are projected and displayed. Style/theme/pattern/border resolution remains incomplete.

DOCX import/export projects and patches direct tblLayout/jc, trHeight, shd and vAlign properties in namespace-aware schema order, retaining unrelated source properties and package parts. Width edits set fixed layout and update tblGrid, preferred table width and all affected cell widths. Changed merged-cell formatting propagates to vertical continuations and survives later splitting. Legacy saved documents hydrate missing format keys only where source grid geometry matches; explicit automatic heights/no-fill overrides remain. Read/import and valid authoring limits remain distinct from full layout fidelity. Unsupported style inheritance, preferred percentage widths/AutoFit algorithms, drag sizing, borders/pattern/custom theme design, pagination and direct Microsoft Word comparison remain open.

Three additional Rust workflows cover local property namespace/attribute retention, formatted standard/prefixed exports, row rules, merged continuations, reset values, invalid-request atomicity, save/reopen, original-byte Undo, and legacy migration with explicit overrides. A model regression verifies selection scope, rich-content retention, span continuations, equal widths and row-structure transforms. The real-engine desktop workflow now uses Table properties, rejects an invalid width, applies multiple properties in one draft, checks physical merged-cell geometry, exports and exercises Undo/Redo. A separate rendering check measures fixed widths and row heights, verifies vertical placement and fill pixels, checks clipping and horizontal reachability. Independent python-docx/lxml verification is in `scripts/verification/verify_word_table_layout.py`.

References: [Word row height rules](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.tablerowheight), [table layout](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.tablelayout), and [cell vertical alignment](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.tablecellverticalalignment). The full regular Flutter suite passed **59 tests** (two opt-in live-provider checks skipped), and all **68 Rust tests** passed. Static analysis is clean. The final property-dialog adjustment also passed the real-engine workflow. The scoped packaged checks below passed on the built-in display; external-display incremental refresh remains unresolved. No parity family is accepted complete; novel AI acceptance remains 0/20.

### Packaged 0.16 checks and display limitation

The disposable `com.folio.navigation.check` app opened the source sample at revision 2 with 35 words and now displayed its direct blue fills and vertical alignment. In Table properties, selecting the top-left 2×2 merged area, entering 90 pt per column, choosing Exactly 24 pt for both selected rows, light-yellow fill, bottom cell alignment and right table alignment changed only the draft. The enclosing Apply saved all properties at revision 3. The visible table narrowed and aligned right; the merged region clipped content to the exact row heights, while the editor, saved document and word count retained all four paragraphs. Undo returned the document exactly to the original source projection at revision 4; Redo restored formatting at revision 5. The native Save dialog exported `Native-table-layout.docx` to the disposable external work folder. Running `scripts/verification/verify_word_table_layout.py` with that file independently confirmed fixed layout, right alignment, 1800/1800/2100/1700-twip columns, both exact 480-twip rows, fill and vertical alignment on both merge rows, 3600-twip merged-cell width, rich paragraphs, nested table and retained header. A full quit/reopen retained the exact revision-5 workspace and visible layout. The preview was closed, process exit verified and only its sandbox restored to the prior revision-30 worksheet sample. The original user workspace was untouched.

Native observations exposed an unresolved display-dependent refresh problem. On the external Odyssey Ark display, initial rendering worked but subsequent Flutter accessibility/raster frames could stay stale while click handlers still queued dialogs. Native resize/Zoom refreshed them; moving the preview to the built-in Retina display restored normal incremental interaction. Returning the normal-sized window to the external display reproduced stale updates, and moving it back restored them again. Queued drafts were cancelled before the accepted workflow; no test text or formatting was applied through stale UI state. The accepted editing workflow ran on the built-in display. These observations do not establish the cause or prove behavior with a physical user on a fully active external display. External-display lifecycle/vsync/occlusion and monitor-state investigation is now a priority; full desktop/platform qualification is not claimed.

The final **0.16.0** ZIP passed strict deep signature verification after fresh extraction. It contains 140,178,798 bytes; SHA-256 `d6e7ad504710e58351bc3ac075ca09238b9bab9b4cb45c35cc2a582678da4990`. The manifest was generated at 2026-09-18T19:41:25.624475+00:00. All 26 staged Dart/Swift source files match the tested source. The package remains ad-hoc signed and not notarized. No new live AI-provider run occurred. Native equal-distribution and automatic/minimum-height choices still have automated coverage only; the native check above covers the exact-height path and listed formatting choices.

## 0.15 Word table editing (2026-09-18)

Tables use logical anchor cells with rowspan/colspan, source row/cell identities and retained vertical-continuation properties. A shared occupancy model rejects overlapping, uncovered and out-of-bounds grids. Imported grid widths are retained as twips. The Flutter layout measures single-row cells, distributes spanning-cell height over the covered rows, and positions one interactive surface per logical cell; column widths retain their relative proportions. Rich text and nested tables render inside cells. The table dialog edits a selected cell with Quill instead of replacing its entire rich content with a plain-text paragraph. Row/column commands transform spans across insertion/deletion; merging retains each selected cell's paragraphs, and splitting retains the merged content in the first unit cell. Draft changes commit once on Apply; Cancel does not mutate the workspace. Shift-click and Select cells both extend the selected rectangle. Retired cell editors are disposed after detachment.

DOCX import resolves gridSpan/vMerge into the logical model. Continuation source cells are retained so their properties survive edits elsewhere. Export emits actual gridSpan/vMerge cells, updates preferred widths for changed geometry, preserves source table/row/cell properties and table grids when unchanged, and retains nested block identity so untouched paragraph XML is reused. Cells end with a paragraph after a nested table. HTML export emits span attributes. Legacy workspaces gain only missing source identities/widths when their table geometry matches; authored content stays intact. Unsupported row omissions, legacy horizontal merges, complex revision properties or nonempty/malformed merge continuations remain preserved read-only content. New/edited tables currently have a 1,000-row/1,000-column geometry bound in addition to the existing document size/node limits; large-table responsiveness is not qualified.

Four Rust workflows cover merged import, rich editing, properties/parts retained, nested content, exact original Undo bytes, save/reopen, native vertical merges, invalid-request atomicity, alternate namespaces, malformed-source preservation and legacy hydration. Four pure editor checks cover rectangle expansion, content/mark retention, split behavior, insertion through spans, deletion of span anchors, dimension bookkeeping and invalid operations. The real-engine widget workflow exercises the merged hit/layout surfaces, cell edits, menus, insertion, Shift-click merge, one-revision Apply/save/export, visible Undo/Redo and split/Cancel. Independent python-docx/lxml checks verify source properties, actual merge identities, nested tables, rich paragraphs, widths, headers, legacy/prefixed exports and exact Undo bytes. References: [Word table structure](https://learn.microsoft.com/en-us/office/open-xml/word/working-with-wordprocessingml-tables), [gridSpan](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.gridspan) and [vertical merges](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.verticalmerge).

The status word count now includes projected table and nested-table text, separates paragraph/cell boundaries and keeps adjacent formatted text runs together. Retained source metadata is excluded. It uses the existing whitespace-based definition of a word; locale-aware segmentation, headers/footers/other stories and exact Word-count equivalence remain unqualified. The count is cached when document content changes, instead of decoding the entire document on unrelated UI rebuilds. A pure regression and the real-engine widget workflow verify the fixture's 35 words, an edit to 36, and Undo/Redo back to 35/36.

The full regular Flutter suite passed **57 tests** (two opt-in live-provider tests skipped), and all **65 Rust tests** passed. Static analysis is clean. The packaged checks below passed. Full W07 parity is not established: inline table editing, border/shading/style choices, exact font/paragraph/layout reproduction, row sizing/AutoFit, drag sizing, arbitrary split counts, sorting, text conversion, repeating headers/page breaks, cross-cell numbering, table-aware proofing, all unsupported structures, formal keyboard/accessibility and large-file behavior, and direct Microsoft Word comparison remain. No parity family is complete and novel AI acceptance remains 0/20. The original user workspace is excluded from all tests.

### Packaged 0.15 checks

The isolated `com.folio.navigation.check` application opened the merged-table fixture with 35 words. In the active native window, typing ` checked` into R3C2 preserved its italic first paragraph and bold second paragraph. Row insertion and Select cells/Merge produced a fourth row whose first cell spans two columns. The draft left the main document at revision 36; Apply saved all edits in revision 37 with 36 words. One Undo returned the original three-row document and 35 words at revision 38; Redo restored the complete edit at revision 39. Splitting the new merged cell and choosing Cancel left revision 39 and its merge unchanged. Earlier background-window automation briefly showed stale accessibility/raster state and typed into the main paragraph; those test-only keystrokes were undone before the accepted workflow. Activating the preview through its Window menu restored fresh native observations.

The sandboxed native Save dialog wrote `Native-tables.docx` into the disposable external work folder. `scripts/verification/verify_native_word_tables.py` independently verified the edited text and rich marks, three actual merge relationships, fourth row, nested table, source grid widths, repeating-header property, shading/vertical alignment, retained header and unchanged surrounding paragraphs. Column insertion inside the existing merged spans expanded the table to five columns at revision 40; deletion contracted it to four at revision 41. Separate Undo steps restored five and then four columns at revisions 42 and 43. The final document exactly matched the revision-39 export state. Full quit/reopen retained that document, revision 43 and the 36-word count. The process was then closed and its exit verified; only the disposable sandbox was restored to the previous revision-30 worksheet sample. The actual user workspace was not opened or changed. These are scoped native checks, not comprehensive keyboard/accessibility or Microsoft Word qualification.

The final **0.15.0** ZIP passed strict deep signature verification after fresh extraction. It contains 140,136,561 bytes; SHA-256 `cbdd3ea6a05d770873fa78bbc6fba2e69511d083b6bcb7f2213ef98f29a6d86a`. The manifest was generated at 2026-09-18T19:24:54.439909+00:00. All 25 staged Dart/Swift source files match the tested source. The package remains ad-hoc signed and not notarized. No new live AI-provider run occurred.

## 0.14 Word list numbering (2026-09-18)

Native list export previously wrote literal marker text and flattened rich content. Export now allocates real abstract numbering definitions and concrete instances, supplies package relationships/content types, and preserves the list paragraphs’ inline text, marks and links. Native nested levels survive codec round trips. Import resolves direct and inherited paragraph-style numbering, instance start/level overrides, separate instances and explicit numbering suppression. The editor renders decimal, letter, Roman, bullet and composite level labels, retains continuation across prose and applies supported restart rules. Unknown/picture definitions remain preserved with limited authoring support; this does not qualify the full Word list model.

The Numbering menu adds Restart at 1, Continue previous list and Set numbering (start plus number style). Each command composes one editor transaction, preserving text and inline marks, then uses the shared save/Undo contract. The controls apply from the selected paragraph through following contiguous items in that list, stopping before a separate instance at the same level; continuation can join sequences separated by prose. A focused extension of the pinned Quill 11.6.0 insertion rule preserves the numbering instance through Enter and multiline insertion, and leaves its existing empty-item exit behavior in place. The marker builder is also a pinned experimental Quill API; both integration points have regression coverage. Source paragraph properties are patched as whole namespace-aware elements in schema order, including compound numPr; explicit suppression overrides inherited numbering. Old saved workspaces hydrate missing numbering metadata from their source package while respecting explicit removal.

Four new Rust workflows cover imported definitions, formatting-preserving edits, exact Undo bytes, new/nested lists and ID collisions, restarts/suppression, saved/reopened exports, legacy hydration, invalid requests and alternate XML prefixes. Six pure editor checks cover counters, styles, nested round trips, removing/restoring numbering, Enter/multiline insertion, restart and continuation, preserved punctuation and adjacent-instance boundaries. The desktop widget workflow exercises the actual menus, invalid input, rendered markers, one-revision Apply, save/export, Undo/Redo, continuation and Roman restart with the real native bridge. The test also exposed an existing proofreading timer race: an older in-flight check could overwrite a newer debounce timer without cancelling it. The retry now cancels that timer first; a disposal regression verifies cleanup. The added toolbar controls wrap at narrow widths.

Independent python-docx/lxml checks passed eight exported files: genuine numPr references, unique definitions/instances in schema order, relationships/content types, source-part preservation, rich marks/links, nested levels, restart values, inherited suppression, prefixed XML and exact original bytes after Undo. References: [Word numbering properties](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.numberingproperties), [starting values](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.startnumberingvalue), [level restart rules](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.levelrestart), and [level label placeholders](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.leveltext).

The full regular Flutter suite passed **51 tests** (two opt-in live-provider tests skipped), and all **61 Rust tests** passed. Static analysis is clean. Packaged list interaction and sandboxed export/save checks passed. Full list-definition editing, picture/other number systems, style-linked abstract numbering, all linked-style inheritance cases, exact font/tab/indent/suffix layout, numbered table rendering, all nested/multi-paragraph list cases and comparison in Word remain outstanding. Word/LibreOffice were not found in /Applications for a local differential check. No parity family is complete; novel AI acceptance remains 0/20. The original user workspace is not used for verification.

### Packaged 0.14 checks and native save correction

The separate `com.folio.navigation.check` application loaded the imported fixture at revision 2. Native Set numbering changed the first list to uppercase letters starting at 7 (`G.`, `G.a)`, `G.b)`) at revision 3, retaining the bold first paragraph. A full quit and reopen retained those values. Restarting `IV)` produced `I)` without dropping punctuation or renumbering the adjacent independent list, and Undo restored `IV)`. Continue previous list joined the selected separate paragraph to its preceding sequence and Undo restored it. Enter followed by typing added `H.` and adjusted the nested labels to `H.a)`/`H.b)`; subsequent Undo restored the prior document exactly at revision 39. Undo history itself is session-local and is not persisted across restart.

The first native Save dialog export exposed a macOS sandbox failure (`Operation not permitted`): the old POSIX writer created a sibling temporary file outside the specific file grant returned by the Save panel. Office exports and workspace saves now share a writer that uses Foundation's atomic file replacement on macOS, retaining the current sandbox entitlements and avoiding a destructive direct-write fallback. A regression verifies creation, binary/empty replacement, Unicode paths, failed directory replacement, parent-path failures and preservation of existing data. The Rust suite passes all 61 tests. In the rebuilt sandboxed app, the same external-folder Save dialog successfully created a DOCX, replaced that file after changing the start from 7 to 1, wrote a complete .folio workspace copy and exported XLSX. Independent python-docx/lxml checks verified both DOCX versions contain real list definitions with the requested start and retained bold formatting. The saved workspace matched the current revision-40 state exactly. openpyxl verified the exported sample workbook's formula and cached total (20,000). The final build's Save Copy dialog suggested a single .folio suffix and successfully saved using that default name. Its bytes decoded to the exact current workspace. The preview was closed and its process exit verified; the earlier revision-30 worksheet sample was restored only in the disposable sandbox. The original user workspace remained untouched. The native export reader checks are reproducible with scripts/verification/verify_native_saves.py and the disposable output folder. References: [Apple Save panel access](https://developer.apple.com/documentation/appkit/nssavepanel) and [Foundation atomic writing](https://developer.apple.com/documentation/foundation/nsdata/write(to:atomically:)).

The final **0.14.0** ZIP passed strict deep signature verification after fresh extraction. It contains 140,057,126 bytes; SHA-256 `8273ecb674ecd30798c58f4a7377cdd56592fe5d4a4267e1eba0d01c1d6ebaf5`. The manifest was generated at 2026-09-18T18:59:05.910887+00:00. All 23 staged Dart/Swift source files match the tested source; the final filename adjustment was verified through the native Save dialog. New Foundation binding license notices are bundled. The package remains ad-hoc signed and not notarized. No new live AI-provider run occurred.

## 0.13 sparse worksheet viewport (2026-09-18)

Replaced the finite TableView span layout with a direct indexed viewport on Flutter's TwoDimensionalScrollView/RenderTwoDimensionalViewport. The old builder virtualized cell widgets but allocated row metrics for the entire finite Excel grid. The new layout retains only visible/pinned cell positions and a 120-pixel surrounding cache, backed by sparse dimension prefix sums and hidden-position lookup. Binary search locates distant rows/columns without scanning from the beginning; full Excel limits remain addressable. Paint, pointer hit testing and semantics are clipped separately to the frozen corner, top, left and scrolling body. Implicit text-field reveal respects frozen boundaries. The now-unused two_dimensional_scrollables dependency is removed. No Rust workbook or Office serialization behavior is changed.

Three focused checks compare sparse offsets with an independent linear oracle across randomized hidden/sized axes, bound actual cell builds and layout lookups at the last Excel cell, verify exact finite scroll extents and end-of-sheet dimension removal, and check clipped raster pixels, pointer selection, accessibility traversal and implicit reveal around frozen boundaries. The existing seven worksheet view/geometry/input checks passed against the replacement. Framework contracts were checked against the installed Flutter 3.44.7 source and [TwoDimensionalViewport](https://api.flutter.dev/flutter/widgets/TwoDimensionalViewport-class.html) / [RenderTwoDimensionalViewport](https://api.flutter.dev/flutter/widgets/RenderTwoDimensionalViewport-class.html) documentation.

The final regular Flutter suite passed **43 tests**, with two opt-in live-provider tests skipped. All **56 Rust tests** passed; static analysis is clean. The signed packaged checks below also passed. No parity family is complete; novel AI acceptance remains 0/20. No new live AI-provider run occurred. The original user workspace is not used for verification.

### Packaged 0.13 checks and memory observation

The isolated signed 0.13 app reopened the unchanged revision-26 worksheet with hidden rows/columns, custom dimensions and two-axis frozen panes. Fresh launch plus Sheets used **339,408 KiB RSS (331 MiB)** versus **955,248 KiB (933 MiB)** for 0.12 under the same interaction, sample bytes and 1152×768 screenshot size: **64.5% lower** in this paired observation. The observations were made at 2026-09-18T18:08:53.487863+00:00 and 2026-09-18T18:18:46.297421+00:00; process IDs differ because these were fresh launches. This is a single sample observation, not a controlled benchmark distribution, peak-memory result, large-file qualification or Excel/Electron comparison. Raw records and limitations are in `release/viewport-memory-check.json`.

Native address navigation reached **XFD1048576** with frozen rows 1–2 and column A still visible (B is hidden). A mouse selection and F2 edit saved `Boundary check` in that exact last cell at revision 27; a separate Undo removed it at revision 28. Navigating back to C3 restored the near viewport. Native vertical scrolling reached approximately row 50 while the headings stayed fixed; selecting pinned A1 retained the scrolled region. Dragging the C boundary changed its stored width from 45 to 49.29 characters, and Undo restored 45. A click on G3 followed by Right selected H3. Reopening retained the restored dimensions, pane state and absent boundary edit at revision 30. The preview was then closed. These checks used only the disposable `com.folio.navigation.check` workspace.

Native horizontal-wheel automation again produced no visible movement. Horizontal address navigation passed, and the existing Flutter frozen-pane regression exercises actual horizontal pointer-scroll events. Physical horizontal-wheel behavior remains unqualified; no claim is made that this build resolves it. Formal screen-reader/IME and broader platform/performance qualification remain open.

The **0.13.0** distribution ZIP passed strict deep signature verification after fresh extraction. It is 139,987,953 bytes with SHA-256 `a7a6aca7aafdcc3286d971bbad8e0b628fe633f11ed350e5664ef35bf1770642`; its manifest was generated at 2026-09-18T18:18:09.235014+00:00. All 21 staged Dart/Swift sources match the tested source. The package remains ad-hoc signed and not notarized.

## 0.12 frozen worksheet panes (2026-09-18)

Added Freeze at selection, Freeze top row, Freeze first column and Unfreeze, using physical Excel row/column counts with hidden positions removed only from viewport layout. Sparse dimension offsets account for custom heights/widths. Frozen boundaries have visible lines; horizontal and vertical scrolling retain the pinned area. Navigation reveals unpinned cells within the remaining viewport and leaves scrolling unchanged when selecting a pinned cell. Imported lower/right scroll origins are independent of the frozen boundary. Oversized panes show a notice and temporarily scroll on the affected axis; their stored boundary is not reduced.

The canonical pane state uses the shared revision and Undo/Redo contract. Existing workspaces hydrate it from their retained source package only when the field is absent; an explicitly unfrozen state stays unfrozen. Imported frozen/frozenSplit views retain their source state and counts. Changing panes modifies the primary workbook view and preserves other view settings and views, ordinary split panes on other sheets, source formatting, notes and formulas. New elements use the SpreadsheetML namespace and schema order. Structural edits transform a nonempty frozen boundary using its first unfrozen coordinate, separately from the scrolled origin. An unfrozen axis remains unfrozen after insertion. Collapsing an axis repairs active-pane/selection roles in every source workbook view, retaining the active selection when roles combine.

Four engine workflow tests cover distant source scroll origins, freeze/unfreeze, native workbooks, exact Undo bytes, Redo, workspace save/reopen, legacy hydration, stale/invalid request atomicity, prefixed XML and structural insertion/deletion. The two new desktop checks use the real Rust bridge and pointer menus/wheel events: custom-size/hidden frozen headings remain fixed during two-axis scrolling; distant navigation targets remain visible; a pending edit and the freeze command create separate revisions/Undo steps; saved state, unfreeze and top-row/first-column commands work; hidden-column navigation remains correct. An oversized-boundary fixture verifies the notice and accessible editable grid. The asynchronous test waits for the requested model state before checking its save, rather than treating a dismissed menu as operation completion.

Independent openpyxl/lxml checks pass for counts and scroll origins, view attributes, preserved secondary views/ordinary splits, source notes/styles/formulas, child order/namespaces, collapsed-pane selections, native and prefixed exports, exact Undo, legacy load and reopened exports. The existing structure, table, AutoFilter, geometry and range-sort reader checks also pass. Sources: [Microsoft Freeze Panes](https://support.microsoft.com/en-us/excel/get-started/freeze-panes-to-lock-rows-and-columns), [worksheet freeze API](https://learn.microsoft.com/en-us/javascript/api/excel/excel.worksheetfreezepanes) and [Office pane attributes](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/bb178944-7838-4a58-9c16-e0c63d428a43).

The final **56-test Rust suite** and **40-test regular Flutter suite** passed; two opt-in live-provider tests were skipped. Static analysis is clean.

Comparison in Microsoft Excel, physical horizontal-wheel behavior and additional native structural/view cases remain unverified. Interactive ordinary splits, custom upper-pane scroll origins, exact restoration of splits after unfreezing frozenSplit source panes, multiple-view selection, zoom and page-layout views remain on X05. No parity family is complete. No new live AI-provider run occurred, and accepted novel AI features remain **0/20**. The original user app/workspace has not been used or replaced.

### Packaged 0.12 checks

The signed isolated 0.12 preview displayed the sample's imported frozen row/column. Selecting C3 and choosing Freeze panes at selection saved rows 1–2 and columns A–B; hidden B remained omitted and row 2 retained its custom height. Native vertical scrolling reached row 50 with rows 1–2 fixed. Address-box navigation to M50 moved the horizontal viewport while column A remained fixed, and selecting A1 kept the scrolled row/column region in place. Unfreeze removed both boundary lines; an ordinary Right skipped hidden B, and Command-Z restored the panes. Closing and reopening retained the two-axis freeze at revision 22, confirmed both visually and in the separate saved workspace.

The dedicated top-row command left row 1 visible while lower rows scrolled to row 45. The first-column command left column A fixed during navigation to M50, with no frozen rows. Two separate Command-Z actions restored the two-axis sample. The preview was closed with its saved state verified at revision 26. A native horizontal-wheel automation attempt produced no visible movement; horizontal pinning was therefore verified with address-box navigation, while the Flutter regression separately exercised horizontal wheel input. No working user data was used.

The **0.12.0** distribution ZIP passed strict deep signature verification after fresh extraction. It is 140,007,263 bytes with SHA-256 `6e0aaa1f547bb4cedd533542800834513b07373eb5be75b0ccc374186b58d5b2`; its manifest was generated at 2026-09-18T18:01:09.507404+00:00. All staged Dart/Swift source files match the tested source. The new package remains ad-hoc signed and not notarized.

### Deferred 0.11.1 native keyboard checks completed

The Mac became available during the 0.12 work. The isolated preview was first updated to the verified 0.11.1 package, with the clean native modifier adapter and no temporary tracing. Actual native input verified C3 → Right → E3 across hidden D; Shift+Tab returned to C3; Shift+Right selected C3:E3; a subsequent ordinary click selected G3 alone. Command-F opened Navigation and Escape closed it. F2 selected G3's contents; typing `Typing ABC`, moving the caret left, Backspace and typing `Z` produced `Typing AZC`. Return committed exactly one revision and moved to G6 past hidden rows 4/5. Command-Z restored `Keyboard check`. A separate `Cancel me` edit was discarded with Escape without a new revision; an ordinary Right then selected H6. Command-Q closed the app and the process exited. These observations qualify those specific native paths; physical modifier-side distinctions, IME and a formal VoiceOver audit remain outstanding.

## 0.11.1 desktop input corrections (2026-09-18)

The unlocked, isolated 0.11 preview exposed two separate worksheet defects. The grid's key handler received printable events bubbling from its inline text field and restarted the edit on each character, leaving only the final character. It now leaves inline text input to the text field. Separately, AppKit accessibility could restore Flutter's old text-input responder after its editing client had been cleared, swallowing subsequent grid keys despite primary Dart focus. A narrow macOS method-channel bridge restores the public Flutter view controller as first responder only while the worksheet grid owns primary focus. Formula fields, inline editing and dialogs retain their own text responders.

Cell selection now occurs on pointer-down, before the double-click recognizer's delay, while double-click editing and clicks inside an existing editor are preserved. Filter buttons are excluded from pointer-down selection: selecting a table can insert its toolbar and move the button before release. The new regression tests first reproduced delayed selection, erased inline content and the moving filter-button failure; all now pass. The focus channel is also checked when entering/leaving inline and formula editing and disposing the worksheet.

The isolated native preview verified immediate cell selection, repeated Right and Tab navigation across accessibility observations, skipping hidden column D, and complete inline entry of `Keyboard check` in G3. Return committed the full text once, saved revision 15 and moved to G6 past hidden rows 4/5. These checks used only `com.folio.navigation.check` and its disposable worksheet. The user's working app and saved document were not used.

The same preview exposed an additional accessibility-keyboard issue: generated Shift/Command events carried aggregate AppKit modifier flags without the device-side flags used by Flutter. The adapter now supplies a left-side fallback only when side information is absent, preserving ordinary physical left/right flags, combined modifiers and unrelated flags. `python3 scripts/test_macos_keyboard.py` passed 29 checks against the production adapter with in-memory AppKit key-down, key-up and flags-changed events. It does not post events or launch an app. The Mac locked again before the updated native shortcut check, so packaged Shift+Tab, Shift-arrow selection, Command shortcuts, modifier release, subsequent plain clicks, text-caret editing and native Undo/restart qualification remain pending. These checks are not substitutes for physical keyboard, IME or VoiceOver acceptance.

The final regular Flutter suite passed **38 tests**, including the existing spelling-menu replacement/divider/Undo regression; two opt-in live-provider tests were skipped. All **52 Rust tests** passed and static analysis is clean. No new live AI-provider behavior is claimed. The previously verified spelling fix remains in this build. This release adds no frozen-pane controls and completes no parity family; accepted novel AI features remain **0/20**.

The final **0.11.1** distribution ZIP built successfully and passed strict deep signature verification after fresh extraction. It is 139,977,485 bytes with SHA-256 `879386e017e500bd01a2e4d2e87513d84ea87c4b9342ad1486c565763d7b56f2`; its manifest was generated at 2026-09-18T17:43:09.587699+00:00. All 19 staged Dart/Swift source files match the checked source, with temporary diagnostic tracing removed. The isolated preview is still running the earlier diagnostic candidate because desktop control reports a locked Mac; it must be closed before updating it for the remaining live checks. The working user app has not been replaced.

## 0.11 row/column dimensions and manual visibility (2026-09-18)

Added row height in points, column width in character units, manual Hide/Unhide, adjacent-row/column recovery, unhide-all commands and direct heading-boundary dragging. Size zero hides without discarding the prior size; positive resize unhides manual visibility while retaining active filter results. Pending edits commit before geometry operations. Arrow keys and Tab skip hidden columns; the address box can select rectangular ranges and intentionally target hidden positions. Each edit uses the revision check and shared Undo/Redo contract.

The viewport uses sparse dimension prefix sums for scrolling to distant cells. Source dimensions are projected on import and legacy workspace load. XLSX export splits grouped column records only as necessary while preserving style/outline/collapsed metadata, creates dimension-only rows, preserves XML namespaces and places column definitions before sheet data. Explicit dimension maps and hidden columns follow insert/delete operations. Manual hiding continues to use the filter-aware SUBTOTAL calculation contract.

Four new Rust checks cover grouped source columns, exact dimensions, empty rows, source styling, saved/reopened state, exact original bytes after Undo, Redo, filter/manual visibility distinctions, zero size, invalid/stale request atomicity, structural movement, native workbook export, prefixed worksheet namespaces and recovery from explicit source zero-height rows. Two new Flutter checks cover sparse offsets and boundary navigation plus a native-bridge workflow for menus, pending edits, invalid size input, rendered extents, hiding, adjacent unhide, hidden-column navigation, pointer dragging, narrow cells, saving and separate Undo. The drag test caught and fixed discarded movement before the gesture threshold. Independent openpyxl/lxml checks validate dimensions, split-column metadata, styles, notes, hyperlinks, XML order, filtering, structural movement and recovery.

The final 52-check Rust suite and 36-check regular Flutter suite passed; two opt-in live-provider tests were skipped. Static analysis is clean. The independent geometry, filter, table, range-sort and structural readers all passed. The 0.11.0 macOS package built successfully and its distribution ZIP passed strict deep signature verification after fresh extraction. This remains partial row/column parity: AutoFit, defaults editing, inherited dimensions on insertion, outline controls, all drawing-anchor behaviors, source default-zero-height sheets, exact font-dependent column-width rendering and direct Excel comparison are outstanding. Heights are limited to 409 points and widths to 255 characters; an operation covers up to 100,000 rows/columns. Sources: [Microsoft row/column dimensions](https://support.microsoft.com/en-us/excel/change-the-column-width-and-row-height) and [hidden first rows/columns](https://support.microsoft.com/en-US/Excel/unhide-the-first-column-or-row-in-a-worksheet).

### Packaged 0.11 geometry checks

The isolated signed preview, using a disposable workbook, displayed imported heights, widths and hidden rows/columns. The column-width dialog changed C from 28.5 to 35 characters; dragging its boundary changed it to 45, confirmed in the dialog and saved workspace. Row 2 changed from 36 to 48 points, hiding it moved the selection to row 3, and adjacent-row recovery restored its 48-point height. A boundary drag then set 70.5 points; separate toolbar Undo and Redo restored 48 and 70.5. Reopening retained those dimensions and hidden rows 4/5 and columns B/D. Adjacent-column recovery displayed B/D at their original widths, and Undo restored hiding. The first package exposed stale initial assistant context when hidden rows/columns relocated the selection. The correction passed a desktop regression and a rebuilt-package restart: both editor and assistant showed C6. The working user app and workspace were not used for these checks.

A packaged keyboard check exposed an unresolved focus discrepancy: a formula-bar submission allowed an arrow to move across a hidden column, but subsequent native automation did not consistently move the grid. The grid now exposes its own named accessibility focus target; its focused state and repeated navigation pass the expanded Flutter workflow with accessibility enabled. That workflow uses actual pointer selection, arrows, Tab, reverse Tab and leaving cell editing. It also caught and fixed reverse Tab incorrectly extending the range (A2:E2 instead of A2). The Mac locked again before the rebuilt native focus check could run. Native keyboard/accessibility qualification therefore remains pending; the packaged mouse results above do not establish that the focus discrepancy is resolved. The verified geometry sample remains isolated from the user workspace. Shortcut semantics follow [Microsoft keyboard shortcuts](https://support.microsoft.com/en-US/Accessibility/excel/keyboard-shortcuts-in-excel).

The final release ZIP is 139,969,958 bytes, with SHA-256 `9c6f79c8c1edf723bb9bf87d552f8396038cdbfbbb3abd0ccbc6f209a36a8bd4`. Its manifest was generated at 2026-09-18T15:29:58.310064+00:00 after fresh-extraction signature verification. All 18 staged Dart source files match the final checked source. The final regular Flutter rerun passed 36 checks with two opt-in live-provider checks skipped; analysis is clean. The isolated preview has been updated to this final package and remains closed, ready for the pending native keyboard check.

### Deferred packaged checks completed in 0.10 after the Mac became available

Using only the separate signed preview and disposable samples, the ordinary-filter workflow passed initial enabling on `$A$1:$D$7`, changing a visible Pear label to Apple, keeping the existing filter results until Reapply, hiding that record on Reapply and separately undoing the reapplication and cell edit. The Qty choices reflected the existing Label filter (1, 3, 99); combining `>1` and `<10` displayed only row 2. Two-key ordinary range sorting by Label then Qty retained headings, kept the manually hidden row in place and left columns outside the chosen range fixed; Undo restored the records.

The table workflow expanded Sales from A1:E8 to A1:F10, created the new heading and moved its 41 total to row 10. Narrowing to A1:D10 left the excluded E/F heading cells on the worksheet. A custom `=SUBTOTAL(109,Sales[Amount])*0.1` total displayed 4.1 and survived hiding/showing the totals row. Five separate toolbar Undo actions restored the original A1:E8 table and 41 total. These observations qualify the tested workflows from 0.8/0.9 in the 0.10 package; they do not retroactively qualify every earlier case or establish direct Excel equivalence.

## 0.10 worksheet AutoFilter and explicit filter visibility (2026-09-18)

Implemented ordinary worksheet filter ranges with heading arrows, value checklists, blanks, one/two text or numeric conditions, cross-column choices, explicit reapply, clear criteria and remove arrows. The app preserves one ordinary range per sheet alongside non-overlapping tables. Filters retain their results when cells change until reapplied. Sorting skips hidden rows; structural edits transform filter ranges/column indices/results, remove criteria for deleted columns and remove the filter when its header is deleted. Pending edits flush before dialogs and operations use the existing revision/Undo contract.

IronCalc 0.8.3 is now vendored with a documented, narrowly scoped patch that supplies explicit per-sheet filtered rows. No synthetic tables are inserted. SUBTOTAL includes or excludes manually hidden rows according to its function number and always excludes filtered rows, including references outside the filter columns or from another sheet. The host rebuilds and reinstalls this ephemeral metadata after structure or filter changes. The upstream MIT license and change notice ship in third_party/ironcalc. The vendored build script records its pinned identity instead of repeatedly watching absent Git paths.

The Rust suite passes **48 checks**. Six new workflow checks cover ordinary and table filter visibility, SUBTOTAL 9/109 across sheets and outside filter columns, non-dynamic edits/reapply, blanks and compound criteria, cross-column choices, sorting, structural insert/delete/rename, invalid/stale request atomicity, overlap rejection, workspace save/reopen, exact Undo source bytes, native workbook export without tables, retained unsupported source filter XML and rejection of unsupported reapplication. The source fixture includes independent openpyxl rich text, styles, a note, hyperlink and manually hidden row. A private worksheet extension records the reason rows are hidden in Folio exports. Import uses it only when descriptors and standard hidden-row flags agree; a test changes standard visibility externally and verifies stale metadata is ignored.

Independent openpyxl/lxml checks pass for standard AutoFilter criteria, filterMode, hidden row flags, schema child order, scoped _xlnm._FilterDatabase names, correctly quoted renamed-sheet references, source rich text/font/number format/note/link/frozen pane, absence of generated tables, conditions, clear/remove, structure, source restoration and save/reopen. These checks caught and fixed stale exported formula caches: unchanged ordinary formulas now receive current calculated results while retaining formula attributes and source metadata; unsupported array/data-table formula-cell caches are cleared for recalculation. A separate prefixed-namespace worksheet fixture verifies that filter changes, new cells and refreshed formula caches retain the SpreadsheetML namespace, original formula metadata, styles and notes. Existing table, range-sort and structural independent-reader checks also pass.

The new automated desktop workflow passes at 1152×768 using the real Rust bridge: pending edits commit before enabling a range, actual header/button interactions select criteria, physical row numbers disappear, arrow navigation skips hidden rows, another column's choices honor the first filter, custom conditions recalculate both SUBTOTAL modes, edits leave existing results until reapply, saved JSON retains state, clear preserves manual hiding, remove hides the arrows and Undo restores the prior criteria. Existing table desktop tests also passed. The final **34-check regular Flutter suite passed** (two opt-in live-provider checks skipped), including spelling-menu regression coverage; static analysis is clean. A filter-dialog test also checks numeric normalization, case-insensitive text and preservation of numeric-looking text distinctions. The final 0.10.0 macOS build completed. Its distribution ZIP passed archive integrity, identity/hash checks and strict deep signature verification after fresh extraction outside the synced folder. Staged Flutter source matches the final source, and the ZIP includes the upstream IronCalc MIT license. Source and packaged Rust binaries have identical UUIDs and sections; an additional comparison verifies every byte outside the code-signature blob and its size fields (normalized SHA-256 b7b8d6f6631457c2dbbe16ef731d58e8f7d2382f2badba225264e02e20701256). The signature storage size changes when packaging; removing the signatures alone leaves size metadata and is not a reliable byte comparison. The next unchanged engine build completed in 0.15 seconds, confirming the vendored build-script correction. After the Mac became accessible, the separate signed 0.10 preview passed a packaged mouse check using a disposable workbook. Opening the Label heading showed Pear and Hidden checked; selecting Apple and applying displayed only physical row 3 and recalculated Summary subtotals to 8/8 (and outside-range-column subtotals to 20/20). Clear filters restored five visible records while manually hidden row 4 stayed hidden; the two subtotal modes then displayed 239/41 and 210/180. Remove worksheet filter removed the arrows and range strip. Three toolbar Undo actions separately restored the range controls, Apple criterion and original Pear/Hidden criterion, with totals 224/26 and 140/110. Quitting and reopening the isolated app preserved the restored filter, visible rows and totals at revision 11. The preview was closed after testing. This first mouse check covers selection, apply, clear, remove, Undo, calculation and restart persistence. The follow-up checks recorded above add initial enabling, reapply and compound conditions; structural filter changes still retain automated desktop/engine evidence only. The working user app and workspace have not been used for verification. No new live AI-provider behavior is claimed.

Sources: [Microsoft AutoFilter behavior](https://support.microsoft.com/en-us/excel/get-started/filter-data-in-a-range-or-table-in-excel) and [SUBTOTAL hidden/filtered row rules](https://support.microsoft.com/en-us/excel/functions/subtotal-function). Still outstanding: remaining packaged workflows, direct Excel comparison, advanced/date/color/top-ten filtering, slicers, all filter button/extension semantics and full sort/table parity. Editing supports up to 100,000 data rows, and menus list up to 10,000 unique values with a partial-list message. Source filters can be retained without being editable. Standard OOXML does not always distinguish manual and filter-hidden rows from other editors; other editors may remove Folio's private provenance extension. No entire parity family is complete and the subsequent novel-AI acceptance count remains **0/20**.

Validated on this Apple Silicon Mac through 2026-09-18. This record separates automated checks, independent file validation, live provider behavior and desktop observations. It does not assert complete Word/Excel compatibility or a measured runtime performance advantage over Electron.

## 0.9 worksheet range sorting and custom totals

The Rust suite passed **42 tests**. Ordinary-range sorting uses the shared stable multi-key sorter with a separate command and no synthetic table in the workbook or calculation model. Headers can be included or excluded; case-sensitive descending and multiple keys are tested. Hidden rows stay at their physical positions. Relative formulas move with records, while cells outside the selected columns and references from other sheets keep their positions. Worksheet sort metadata is imported/exported and transformed during row/column edits. Invalid ranges, stale requests, partial tables, intersecting merges and outlined groups are rejected atomically.

A separate openpyxl source fixture contains visible duplicates, hidden data, relative formulas, rich text, font/number formats, a note, an internal hyperlink, a frozen pane and adjacent cells excluded from the sort. Rust and independent openpyxl/lxml checks cover the record mapping, hidden-row exclusion, formulas/results, source objects, schema order for worksheet sortState, absence of table parts, headerless sorting, metadata after column insertion, exact source bytes after Undo and saved/reopened exports. Existing table and structural export checks pass.

Custom totals now have an editable totalsFormula definition paired with totalsFunction=custom. Direct edits, the new menu command, standard-function replacement and text labels update both cells and table metadata. Stored formulas participate in table/column renaming, structural reference changes and table removal. Tests cover hide/show, header and table renaming, column insertion, file reimport, workspace save/reload, Undo, invalid requests and standard/label replacement. Export writes calculatedColumnFormula before totalsRowFormula and retains source formula attributes. The independent reader normalizes the standard's "none" enum to null; raw XML separately verifies the correct enum value.

The focused Flutter desktop checks passed (three checks): ordinary-range sorting flushes a pending edit, preserves headers and hidden rows, recalculates formulas, retains settings, supports headerless descending order, saves and undoes sorting separately from the pending edit. The existing table workflow now exercises the custom total dialog, hide/show restoration and Undo. A narrow-window toolbar overflow was caught and fixed with a compact Chart button and horizontal overflow support. The final library passed the complete **32-check regular Flutter suite** (two opt-in live-provider checks skipped), and Flutter analysis passed. The final 0.9.0 macOS build completed. Its ZIP manifest verifies identity, archive integrity, SHA-256 and strict deep signature after fresh extraction outside the synced folder. A separate preview identity and disposable sample are prepared for the pending manual check. Request routing also keeps range-sort guards and formula-copy semantics independent of extraneous action fields; a regression verifies those fields cannot bypass merge/outline rejection. Actual packaged mouse acceptance remains pending because the Mac was locked; no new computer-use acceptance is claimed.

Sources: [Microsoft's range-sort behavior](https://support.microsoft.com/en-gb/excel/sort-data-in-a-range-or-table-in-excel) and [custom totals metadata](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/99e8f4cc-2f6c-4663-8cf5-b044dddc7e1c). Direct comparison in Excel, left-to-right/locale/custom-list/color sorting, outline sorting, worksheet AutoFilter editing and complete table conversion remain outstanding. The full Word/Excel objective and subsequent 20 additional AI workflows are not complete; AI accepted count remains 0.

## 0.8 table resizing

The Rust suite passed **37 tests**. The new workflows resize imported tables vertically and horizontally, preserve column IDs, reindex filters and sort priorities, fill newly included calculated cells without overwriting exceptions, retain excluded worksheet data, resolve detached local references and invalidate references to removed table columns. Tests cover moving totals, filtered totals recalculation, duplicate headings, occupied destinations, invalid ranges, stale revisions, save/reload and exact exported bytes after Undo.

Independent openpyxl/lxml checks passed for expanded, narrowed, reopened and undone exports, including source rich text, total-cell font, notes and hyperlinks, hidden rows, calculated-column metadata, sort/filter ranges and defined names. Existing structural export verification passed. The source fixture now includes a styled total cell with a note and hyperlink to exercise relocation, in addition to the previous record-bound objects. A review caught imported custom-sort settings being dropped on resize: the fix retains unsupported sort choices, updates their ranges and removes only criteria for excluded columns. Rust and independent-reader checks cover that path; execution of those custom sorts remains unimplemented. Totals relocation preserves ordinary A1 operands rather than applying record-copy offsets; a regression and independent export check verify an overridden `SUM($B$2:B4)` still targets the same cells after relocation. Standard structured totals continue to follow the resized table.

Flutter analysis and all **31 regular Flutter checks** passed against the final Rust library (two opt-in live-provider tests skipped). The final macOS 0.8.0 build and distribution ZIP completed; its manifest verified ZIP integrity, version, SHA-256 and strict deep code signature after fresh extraction outside the synced output folder. The expanded desktop workflow opens Resize table from its menu, validates an out-of-bounds address without changing the workbook, accepts absolute-address input, expands a calculated table, shrinks it while retaining excluded values/formulas, and restores both changes with Undo. A packaged-app mouse check is pending: computer-use access reported that the Mac was locked and could not be unlocked automatically. No current packaged-app interaction is claimed for 0.8; the automated desktop workflow is separate evidence. The implementation follows the documented boundary model in [Microsoft's ListObject.Resize reference](https://learn.microsoft.com/en-us/office/vba/api/excel.listobject.resize) and [header/overlap constraints](https://learn.microsoft.com/en-us/dotnet/api/microsoft.office.tools.excel.listobject.resize?view=vsto-2022); these documents do not establish direct behavioral equivalence. Remaining work includes comparison in Excel, occupied totals destinations, all structured-reference combinations and table-style/object cases, custom totals formula metadata/toggling, convert-to-range and ordinary range sort/filter. No AI workflow is newly accepted toward the separate 20-feature requirement.

## 0.7 tables, sorting and filtering

The complete Rust suite passed (**32 tests**), Flutter analysis passed, and the complete regular Flutter suite passed (**31 tests**, two opt-in live-provider tests skipped). The macOS 0.7.0 release build, deep signature verification and ZIP/manifest generation completed. The release-copy check caught Finder metadata repeatedly introduced by the synced output folder. The packaging script cleans only FinderInfo/ResourceFork metadata from the convenience app copy; quarantine and other attributes are retained. The ZIP is the authoritative distribution artifact: the manifest script now extracts it into a temporary directory outside the synced folder and requires deep signature verification of that fresh copy. This check passed for the final ZIP. Extract the app into Applications for normal use. Existing AI protocol behavior was not changed or newly qualified.

- New table operations create named tables, set calculated columns, rename tables, select four visual styles, apply or clear value/custom filters, sort by up to 64 columns, and add/remove a totals row or choose its aggregation. Names/column IDs, overlap, invalid requests and stale revisions are validated before commit. Occupied cells below a table prevent totals insertion. Structured references retain their syntax through sorting and A1 structural transforms; the calculation adapter supplies table definitions and expands local `[@Column]`/bare-table references for IronCalc.
- Direct cell edits update table headings/references and extend tables downward when data is entered immediately below them, filling calculated columns. Worksheet insertion/deletion updates table ranges, columns and filter positions, fills inserted calculated rows and invalidates references to deleted columns/tables. Deleting a table-bearing sheet also invalidates retained defined names. Older `.folio` projections recover table and hidden-row metadata from their retained Office packages.
- The independent openpyxl source fixture includes a calculated table column, totals, structured-reference formulas on another sheet, a defined name, rich-text runs, styled numeric input, a note, internal hyperlink, a manually hidden row and frozen panes. Tests exercise multi-key stable sorting, filter-aware `SUBTOTAL`, exact original bytes after Undo, saved/reopened workspaces, table/column renaming, retained totals choices across hide/show, reapplying remaining filters after column deletion, retained sort priority, added tables on new imported-workbook sheets, structural operations, prefixed table XML, numeric filter values, wildcards, totals and failure atomicity. Source rich-text cells, comments, hyperlink anchors and formatting follow sorted records; headings and totals remain in place.
- `scripts/verification/verify_tables.py` independently opens generated exports with openpyxl and parses their XML with lxml, checking table/column descriptors, calculated formulas, totals, filter state, row visibility, sort metadata, rich text, styles, notes, hyperlinks, names, frozen panes, newly added tables, deleted tables/sheets and source-byte restoration. Existing structural-export verification also passes. No direct Microsoft Excel comparison was performed.
- A real-engine Flutter desktop test at 1152×768 uses the reachable table dialogs and ribbon menus: it filters records, checks physical row labels, edits visible B4 without writing hidden B3, verifies recalculation, navigates over hidden rows, sorts at two levels, creates a second table, fills a structured calculated column, adds its total, undoes changes and saves. A separate viewport test covers hidden first/last rows, duplicates and Excel's final physical row. It caught a sort-direction control overflow, which was corrected.
- The packaged app was exercised in the isolated `com.folio.navigation.check` sandbox with a disposable Orders/Summary workbook. Actual mouse clicks filtered Product to Pear (physical rows 3/4, total 23), sorted Product ascending then Qty descending (amounts 15/8 remained with their records), and used Undo to restore order and then all records/total 31. The Amount total was changed to Average (10.333333) and undone. A custom numeric Amount > 8 filter displayed physical row 4 with total 15; Undo restored all records. The preview was closed after restoration. The working user workspace was not opened or changed. A subsequent display refinement corrects singular/plural row counts and accounts for all hidden rows in the table's visible count; the focused table workflow and full regular Flutter suite were rerun successfully after the final changes.
- Still incomplete: table resize and conversion to ranges, general range sorting/worksheet AutoFilter, advanced/date/color/top-ten filters, slicers, locale/custom-list sort order, all structured-reference specifier combinations, complete table style/design choices, pivots/external-link/macro relationships, every drawing/validation behavior under sorting and direct Microsoft Excel qualification. Existing unsupported filter XML is preserved until explicitly cleared/replaced. Imported filtered tables cannot always distinguish manually hidden rows from filter-hidden rows. These limitations remain in the parity ledger.

## 0.6 worksheet structure and sheet management

The complete Rust suite passed (**20 tests**), Flutter analysis passed, and the complete regular Flutter suite passed (**29 tests**, two opt-in live-provider tests skipped). The macOS release build, deep signature verification and ZIP integrity/manifest generation completed. AI protocol and model behavior were not changed or newly qualified in this release.

- A syntax-tree transformer handles single cell references, mixed absolute/relative references, quoted sheet names, strings that look like references, full-row/full-column ranges, reversed ranges and shrinking ranges after endpoint deletion. IronCalc 0.8.3's own displacement helper documents a different range-deletion behavior, so interval transformation is implemented explicitly. Unparseable formulas, edge overflow, invalid sheet names, duplicate names, stale revisions and deletion of the last sheet are rejected before committing.
- Structural operations commit the workbook, styles and linked document ranges as one revision. Tests verify calculated cross-sheet results, shared formula followers, moved formatting, invalidated links, sheet rename/reorder/delete, repeated Undo/Redo, exact original XLSX bytes after Undo, and `.folio` save/reload followed by export. Missing/deleted sources remain readable in document/HTML exports. Existing pending edits are flushed before an operation; they remain a separate Undo step.
- The richer fixture is generated by openpyxl and includes shared formulas, named ranges, scoped names, print area/titles, row height/column width, comments, a chart, merges, a frozen pane, filters, data validation, absolute/relative conditional-formatting rules and an internal hyperlink. Structural exports update these tested objects and invalidate formula/chart caches. The reader expands shared followers; changing a shared master now materializes the remaining group so it does not leave dangling followers. Partial array deletion is rejected even when it would remove the master cell.
- `scripts/verification/verify_structure.py` independently opens the exports with openpyxl and lxml. It verifies changed cells/formulas, styles, names and local scope, comment movement/deletion, chart references/anchors, panes, filters, validations, both conditional-rule reference modes, merged ranges, hyperlinks, unchanged style/theme parts, recovery, exact Undo bytes, editing a shared master and adding a sheet after deletion/reordering. This also caught and fixed the missing relationship namespace when appending worksheets to an imported workbook; new part paths and relationship/sheet IDs avoid collisions.
- A real-engine desktop widget test at 1152×768 uses mouse context menus and dialogs. It saves a pending formula-bar edit, inserts a row, checks the recalculated total, undoes only the structural operation, inserts a column, inserts multiple selected rows, renames a sheet with an apostrophe, moves it, deletes it, restores it with Undo and saves. Menus wait for pending edits before showing; the test waits for that asynchronous work.
- Packaged 0.6.0 was exercised in the separate sandbox identity `com.folio.navigation.check`. Actual mouse actions inserted row 4, moving Research to row 5 and the total to row 10 while retaining 20,000/14,750/5,250; Undo restored the original positions. Column B insertion moved Planned/Actual/Remaining to C/D/E; Undo restored them. Renaming Project budget to Project forecast updated the tab and assistant context; Undo restored it. A disposable sheet was added, moved left, deleted through the confirmation dialog, and restored with Undo. Further Undo restored the sample to its original single-sheet state. The preview was closed and its process exit verified. The working user workspace was not opened or changed.
- Still incomplete: Excel table structural changes, pivots, external workbook links, macro-containing packages, some extension ranges, unsupported formula syntax, row/column move/resize/hide, sheet duplication, every drawing anchor behavior, named-formula calculation, all array/dynamic formula semantics and direct Microsoft Excel qualification. Unsupported structures encountered by these operations report an error rather than committing a partial edit. The fixture evidence is not a full compatibility claim.

## Automated checks

- Rust tests cover atomic document/workbook proposals, stale-revision rejection, rollback for invalid cell edits, shared undo, `.folio` save/reload, Excel address bounds and cross-sheet formula evaluation.
- DOCX fixture tests verify exact bytes for an unchanged export; edited paragraph content/alignment; unchanged header/media/style parts; a new document with a heading, hyperlink and table.
- XLSX fixture tests verify exact bytes for an unchanged export; edited values/new cells; retained formulas and untouched charts/comments/styles; new workbooks; literal numeric/formula-looking text kept as text.
- Imported packages survive `.folio` save/reload. Linked document data changes with its source and returns on Undo. Cell formatting survives XLSX export/import.
- Flutter tests cover selection anchors, Unicode text, empty paragraphs, stale-selection rejection, Quill formatting/Office block identity, actual A2UI rendering/action events, forbidden components and cyclic UI definitions.
- Widget tests exercise a real Rust-backed desktop workspace: cell editing/recalculation, undo, a selected-passage proposal that does not change the document before Apply, Apply/Undo, linked block Apply/Undo, and saving. The million-row virtualized viewport creates fewer than 500 visible cell widgets in the test viewport.

## 0.5 navigation and replacement

The full regular Flutter suite passed: **28 tests**, with the two opt-in live-provider tests skipped. Flutter analysis passed. Rust implementation and AI tool behavior are unchanged from 0.4.1; no new live-AI acceptance is claimed for this release.

- Seven search-engine tests cover Unicode/UTF-16 offsets and word boundaries, literal punctuation, case sensitivity, multiple occurrences, replacements spanning differently formatted runs, retained outside marks and paragraph identity/alignment, list/table paths, hard breaks, opaque linked blocks, stale/overlapping edits, worksheet/workbook scope, row/column ordering, displayed number formats, formula inputs, forced-text cells and explicit result truncation.
- The new desktop workflow runs at 1152×768 against the real Rust engine. It opens Find using ⌘F while Quill owns focus; navigates to a heading after 35 paragraphs and checks that its caret is visible; replaces two document occurrences in one revision while retaining bold text; undoes the action; verifies pending edits invalidate old results; finds across two sheets; jumps to C5000 and verifies scrolling; replaces both sheets in one transaction and undoes; replaces part of a formula, checks 50→75 recalculation and restores 50 with Undo. This caught and fixed narrow dropdown overflow before packaging.
- An Office workflow imports independently generated DOCX/XLSX fixtures, replaces a body occurrence and table header plus a workbook input, verifies recalculation and stale-transaction rejection, undoes atomically, reloads a saved `.folio`, exports and reimports both formats. `scripts/verification/verify_search.py` then reopens those exports with python-docx/openpyxl and parses XML with lxml. It verifies changed content, retained bold/number formatting, formulas, header, image, chart, comment, frozen pane, filter and merge, plus unchanged header/media/chart/comment parts.
- Packaged 0.5.0 was verified using the separate sandbox identity `com.folio.navigation.check`. Actual mouse clicks opened Navigation, found two occurrences of `workbook`, replaced both with `spreadsheet`, restored both with one Undo, and selected/highlighted the exact passage from a result card. In Sheets, replacing input 2400→2500 changed the planned total 20,000→20,100 and remaining totals; Undo restored the input and totals. Controls were inspected at 1152×768. The preview was closed afterward; the working user's app/workspace was not used.
- A final panel-switching fix was checked by rerunning the complete desktop search workflow: asking the assistant while Navigation is open now reveals the assistant's A2UI choices. The full 28-test pass preceded that final small fix; the focused rerun passed afterward. The release manifest is generated from the finished ZIP and verifies its members, version, size and SHA-256 rather than carrying metadata from the previous release.
- Native launch automation stalled before returning its initial window and subsequently lost its handle; reattaching to the already-running preview succeeded. That delay is not a measured application-startup benchmark. Automation's modifier-key/select-text actions did not reliably change the native text fields; the packaged checks used visible mouse selection and typing. Shortcut behavior is verified in the desktop widget test, not claimed as a packaged keyboard acceptance test.
- Search is literal and scoped to supported model content. Formatting/wildcard searches, other Word stories, comments/notes and all advanced Office search options remain on the parity ledger. The 10,000-result limit is visible and disables partial Replace All. These checks establish the described workflows, not full Word/Excel search parity or Microsoft Office visual equivalence.

Full product acceptance remains incomplete; see `PARITY.md`. No additional feature has yet been accepted toward the 20 novel AI features.

## 0.4.1 spelling-menu correction

The reported mouse-click failure was reproduced against 0.4.0: a right-click menu opened and showed `received`, but pressing the suggestion with a real mouse event dismissed the overlay before release. Quill considered the menu an outside tap and dropped editor focus. This was not exercised by the earlier sidebar/direct-callback checks.

Menu controls now share the editor's text-field tap region, so click release reaches the action. A divider appears after the spelling suggestions and is absent when there are no suggestions. All ordinary menu actions use the same click handling. Correction validation compares a current diagnostic's range/text/message and allowed replacements, rather than relying on object identity across background rechecks; changed document content is still rejected.

The new macOS mouse regression test passed: actual right-click on an emoji-containing formatted paragraph, pointer-down/pointer-up on a suggested word, native workspace mutation, bold preservation and Undo. It also verifies divider placement, accepts an unchanged correction after diagnostics are recreated, omits the divider for normal text, and activates Select All through the menu. Six existing proofreading tests passed alongside it; Flutter analysis found no issues. The Rust engine and AI protocol are unchanged from 0.4.0. In the isolated packaged 0.4.1 app, a coordinate-based right-click on `recieved` displayed spelling alternatives above a visible divider. A coordinate-based primary mouse click on `received` corrected the text and reduced the diagnostic count; toolbar Undo restored the misspelling. The user's working document was not used.

## Spelling and grammar verification

- Harper 2.11.0 is pinned in Cargo.lock and embedded in the Rust dynamic library. The engine runs on the bridge worker pool under a separate lock from workspace editing/calculation. The Flutter adapter debounces typing, caches paragraphs, discards results from older document generations, and renders diagnostics without writing them into document styles.
- Two real-engine tests detect spelling and grammar errors, verify UTF-16 ranges after an emoji, distinguish US/UK spelling, and reject unsupported languages/oversized requests. The full Rust suite passed: 13 tests.
- Six focused Flutter tests passed: mixed-style preservation, exact replacements/emoji boundaries, scoped and stale AI corrections, code/URL masking, paragraph caching/personal dictionary writes/forget/ignore, and delayed-result invalidation while typing.
- The expanded real-engine desktop widget test passed after fixing Review-ribbon overflow, the Writing Editor material surface and editor focus lifetime. It exercises platform text input and verifies that keyboard focus survives background refreshes. It opens the Writing Editor, applies a local spelling correction, undoes it, stages multiple AI-format grammar edits without changing the document, applies them, checks bold formatting, and undoes the batch. Existing document, worksheet, storage, A2UI and selection checks also passed (18 regular Flutter checks across the suite and focused reruns).
- A live Codex grammar test passed using a disposable three-paragraph fixture. It called the real App Server, proposed small `proofreadEdits`, corrected spelling and an article, preserved a bold word and `$25,000`, left unrelated paragraphs alone, and left the model unchanged before Apply. Apply/Undo succeeded. No working user document was sent for this test.
- Built-in proofreading coverage is English prose. Tables, code, linked blocks and spreadsheet cells are excluded. Long/oversized documents report partial coverage. No benchmark establishes superiority over Microsoft Editor, Grammarly or LanguageTool; accuracy and false-positive rates still need representative user-document evaluation.

## Independent Office readers

Source fixtures were produced with python-docx and openpyxl. Exported files were reopened using those independent libraries and XML parts parsed with lxml. Assertions checked:

- Edited Word text, bold formatting, centered paragraph, retained header and inline image.
- New Word document and hyperlink relationship; readable table grid.
- Linked workbook data exported into a readable Word table.
- Excel formulas, changed numeric values, number formatting, italic/bold style edits, a retained chart, comment, frozen pane and merged range.

These are structural and semantic fixture checks. No visual round-trip comparison in Microsoft Word or Microsoft Excel has been completed. Advanced layout, charts, pivots, tracked revisions, complex formulas and real-world file diversity still need acceptance testing.

## Live Codex App Server

The opt-in live test passed against the pinned native Codex runtime. It read a sample workspace and range, called `present_ui` to create a real A2UI workbook preview, and submitted a proposal. The model stayed unchanged before Apply. Apply updated the budget total from 20,000 to 20,600 plus a document paragraph; Undo restored both. This test used disposable sample data, not the user's working document.

## Desktop observations

The packaged app launched with the existing saved workspace intact. Visual inspection confirmed the Word-style blue header/ribbon/page, selection context, native worksheet/range dialog, calculated source preview and A2UI action buttons. A live linked chart with a worksheet/range badge rendered in the document. The existing workspace was backed up before restarting the earlier app build.

The macOS App Sandbox remains enabled. The signed staged build is verified by the packaging script; its ZIP is created before file-provider metadata can alter the copied bundle. The local app has not been Developer ID notarized. Upstream GenUI media dependencies emit macOS deprecation/framework naming warnings during compilation; they did not prevent the build. The Folio A2UI catalog exposes no media components.

The 0.4 package was also inspected in an isolated app identity using only disposable sample text. Native typing continued across automatic checks. Red and blue diagnostic underlines appeared, the Writing Editor displayed spelling and article suggestions, accepting `received` changed the sample, and the toolbar Undo restored `recieved`. The language picker, local dictionary controls and both AI review actions were visible. The original working app and workspace were not used for this test. A computer-use clipboard paste attempt timed out; direct typing was verified, and packaged rich-text clipboard behavior needs a separate acceptance check.

## Remaining product qualification

Full Office fidelity; pagination/printing/PDF; advanced tables and styles; comments/track changes editing; charts/pivots and named ranges; shared/array/dynamic formulas; date systems; accessibility qualification; large-file benchmarks; Windows/Linux builds; secure public distribution; and resumable assistant history remain unfinished.


## 2026-09-19 — named styles engine foundation (unreleased)

The development source adds a shared style catalog and revision/workspace-checked
create/edit/apply/delete commands. Dependent paragraph layout, tabs, supported
run properties and outline metadata are recomputed without flattening direct
overrides. Source styles, stable identities, unknown properties, locally declared
namespaces and full unchanged inline XML survive paragraph/style edits. No-op
commands do not add history; legacy recovery, missing style parts and invalid
operations have regression coverage. Ordinary document and AI replacement edits
cannot bypass style-catalog transactions.

Completed checks: **232 Rust tests passed**, release engine build succeeded,
**202 Flutter tests passed with 2 optional AI-provider skips** against the final
engine, and **13 DOCX outputs** opened and checked independently with python-docx
and lxml. The style tests include mixed base/child run toggles, alternate style
part paths/prefixes, nested cells, renamed styles, custom heading identity after
outline changes, tab clears/removal, direct overrides, unchanged package parts,
Undo/Redo, saved-file recovery, no-ops and atomic rejection. The full Flutter
run establishes compatibility with existing app workflows; it does not exercise
a named-style interface, which is not connected yet.

Evidence is retained at `../../work/named-styles-engine/verification.json` and
its adjacent logs, source snapshots, independent reader and DOCX/Folio artifacts.
`integrity.json` records hashes of those evidence files. Implementation contract
and remaining work: `docs/NAMED_STYLES.md`.

No desktop package was produced for this foundation. The existing reviewable
release remains 0.52.0 build 2. Named-style UI/rendering, direct false run
overrides, Enter/toolbar synchronization, full cascades/themes/templates, native
interaction and Microsoft Word comparison remain unverified or unimplemented.
W02, all other full parity families, and the 20 novel AI feature requirement
remain incomplete (novel AI accepted: 0/20).

## 2026-09-19 — named-style editor and inherited rendering (0.53 development)

The development interface connects the catalog to searchable Apply/New/Modify/
Delete controls, existing-style application in cell drafts, inherited text
rendering and explicit direct-off marks. Conventional heading/list commands keep
their paragraph styles and numbering together; styled lists remain editable.
End-of-paragraph Enter follows the configured next style. The vendored editor
now owns custom rules per document, avoiding cross-editor catalog replacement.

**235 Rust tests passed**, including new theme/literal font precedence, explicit
off run round trips and read-only cell preview rejection. **210 Flutter tests
passed with 2 optional AI-provider skips**, and analysis reported no issues.
The real-engine application test changes a base style, checks affected and
unrelated paragraphs, keyboard Undo/Redo, direct bold-off, Cancel, a styled table
draft and history, a styled numbered list and history, following-style typing,
stale rejection, New/Delete, local save/reopen and DOCX export. Separate tests
inspect effective editor/static-cell text styles, codec offsets and metadata,
two independent live document rule sets, input validation and heading/list
transitions. A scheduling issue in the test harness was fixed by pumping bridge
transactions and waiting for actual history revisions; no native success is
inferred from those test mechanics.

`verify_named_style_ui.py` independently opens `styled.docx`, `reopened.docx`
and `list-styled.docx`, checking stable style identity after rename, base
relationships, point size, explicit run-off formatting, following and cell
styles, numbering, and unrelated text. This uses python-docx and XML checks;
Microsoft Word has not rendered these files in this qualification.

Logs are in `../../work/styles-ui-{rust,flutter-full,analysis}.log`. Source
build/package/native verification remains separate. The normal running app and
reviewable release are still 0.52.0 build 2 while the isolated 0.53 candidate is
being prepared. No full parity family is accepted, and novel AI remains 0/20.

The isolated **0.53.0 build 1** candidate subsequently built and passed ZIP
signature verification. Its 273 compiled source files match the build stage;
the packaged engine matches the release engine after signature normalization.
Fifteen named-style/proofreading checks passed against that packaged engine.
The candidate ZIP is retained at `../../work/styles-ui-package/Folio Native.zip`;
it has not replaced the reviewable 0.52 release.

Native walkthrough evidence now covers importing the independent style fixture,
renaming the base style to `Native report base`, changing its font size from
14 to 20 points, dependent body/table rendering, preserved explicit bold-off,
keyboard Undo/Redo after the dialog, Enter followed by typing in `ReportBody`,
cell style preview without a workspace mutation, table Apply and keyboard
Undo/Redo, native DOCX export and restart at revision 9. The saved workspace was
byte-identical across restart. `verify_native_styles.py` checks the retained
workspace states and independently reads the exported DOCX. The normal user
workspace remains byte-identical to its pre-check backup.

Native snapshots and the report are in `../../work/native-styles`. Native
New/Delete/Cancel, first-style materialization in a new native document, the
remaining shortcut/input-method cases and Word visual comparison are still
unqualified. These passing cases do not accept the full named-style family.

## 2026-09-19 — first-style geometry correction (0.53 build 2 development)

Creating an unused custom style in the isolated build 1 preview visibly moved
the native second heading and following content upward. Native paragraphs used
additive spacing until a catalog was materialized, then inherited paragraphs
used collapsed spacing. Static native table cells also had a separate 14-pixel
font/compact paragraph default, and their editor/dialog defaults differed.

Build 2 source consistently applies the paragraph spacing rules and native
defaults in main text, static cells, cell drafts and their paragraph dialog.
The new real-engine regression measures 17 text regions across six heading
levels, bodies, lists and a table; text rectangles, effective fonts, line height
and alignment are identical before/after unused-style creation and Undo/Redo.
It also opens a cell paragraph draft, changes an indent, verifies the unchanged
native spacing, cancels and compares the restored cell geometry. The row-height
fixture now uses explicitly zero after-spacing and short labels that fit its
narrow cells, keeping its exact/minimum-height and vertical-alignment assertions
independent of document defaults. Ten focused layout checks and **211 full
Flutter checks pass with 2 optional AI skips**; analysis reports no issues.
Rust behavior is unchanged from the 235-test run.

Evidence logs: `../../work/styles-native-{geometry,full-flutter,analysis}.log`.
Native build 2 creation/deletion/Cancel verification is the next gate. The
reviewable package is still 0.52.0 build 2; this is not full W02 acceptance.

## 2026-09-19 — native style creation and cancellation, build 2

The signed isolated 0.53.0 build 2 archive matches 273 compiled sources and the
release engine. The native fixture has a first heading, body, second heading,
body and two bullets. New-style Cancel leaves revision 1 byte-identical; creating
`Native accent` at 18 points in green adds one revision without visibly moving
the existing content. Modify Cancel preserves revision 2 byte for byte. Apply
changes only the selected body paragraph. Apply Undo/Redo exactly restore the
retained before/after documents. In-use Delete is rejected without any change;
after Undo, unused Delete and its Undo/Redo restore the expected catalogs and
paragraphs exactly. This is not replacement-style deletion support.

The native export is independently read with python-docx: the stable custom
identity, 18-point green declaration, Normal base/following styles, target body,
unrelated heading/body styles and both bullet definitions are retained. Native
quit/reopen restores revision 13 byte-identically. The normal user workspace
also remains byte-identical. Retained states, export and the independently
recomputed report are in `../../work/native-style-creation`; run
`scripts/verification/verify_native_style_creation.py` against that directory.
Package identity is in `../../work/styles-native-package-verification.json`.

The walkthrough found an additional usability defect: Undo reset the caret to
the document start. It restored document content correctly, but required selecting
the intended paragraph again before further editing. Build 3 source corrects the
shared refresh behavior and has focused selection/real-app regressions. Build 3
native evidence and the full-suite outcome are still pending. Neither version
establishes complete W02 acceptance or Microsoft Word rendering equivalence.

## 2026-09-19 — document selection retention, build 3 development

The shared document refresh no longer resets every selection to offset zero.
Formatting and embedded-object property changes retain the current endpoints,
direction and affinity; text insertions/removals map the endpoints through the
change and clamp them to valid UTF-16 boundaries. Explicit Open/New and DOCX
import start a new selection context, including opening the same workspace ID.
Failed Open retains the existing selection. This maintains the current position;
restoring per-command original selections after unrelated navigation is a separate
unfinished history behavior.

Four focused selection tests, a real-engine workspace check and new assertions in
the named-style keyboard-history walkthrough cover these cases. The full Flutter
suite passes **216 checks with 2 optional AI-provider skips**; analysis reports no
issues. Three UI-generated DOCX artifacts independently retain styles, direct
overrides, following styles, table assignments and numbered headings. Rust source
is unchanged from the prior **235 passing checks**. Logs are
`../../work/styles-selection-{focused,full-flutter,analysis}.log`.
Build 3 packaging and native caret checks are still pending.

## 2026-09-19 — native line-selection failure and build 4 correction

The signed build 3 candidate launched with 274 source matches and the same
normalized engine. Native style Undo/Redo retained the visibly selected body.
However, Command-Right followed by Command-Shift-Left selected the preceding
paragraph break as well: Styles showed two paragraphs and replacement joined
the heading with the body. The failed native revision 17 is retained at
`../../work/native-style-selection/line-selection-failed.folio`. The fixture was
restored with Undo. Build 3 was not promoted.

A macOS real-workspace keyboard regression reproduced the off-by-one selection
(start 15 instead of 16). The vendored editor's expand-line action searched for
newline characters instead of rendered line boundaries. Build 4 uses the visible
line bounds and expanding selection semantics, preserving an existing range when
expanding its other end. Empty-line geometry no longer indexes an empty box list.
The regression now passes hard-line selection, wrapped-line boundaries in both
directions, Backspace retaining adjacent text, and empty-paragraph navigation.
Seven focused selection/style checks pass; build 4 full/native checks are pending.

Build 4 follow-up: **216 full Flutter checks pass, 2 optional AI checks skipped**,
analysis is clean, and all three UI-produced DOCX style artifacts pass the
independent reader. Logs are `../../work/styles-selection-build4-{focused,full,analysis}.log`.
The native build 4 selection/replacement check remains the release gate.

## 2026-09-19 — native line selection passes; paste caret corrected in build 5

Build 4 launched from its verified archive. Command-Right then Command-Shift-Left
now reports one selected paragraph. A 20-to-22-point style modification and
keyboard Undo/Redo retain that selected body. Pasting `Selection retained.`
replaces only the body, with every other document node unchanged. Native saved
states at revisions 20–24 are retained in `../../work/native-style-selection-final`.
However, the caret after replacement was beyond the inserted text, in another
paragraph. Build 4 was not promoted.

Plain paste had added the clipboard length to the selection's former end rather
than its start. Rich insertion also compared a whole Delta as one synthetic embed.
Build 5 uses the replacement start plus inserted length and compares actual rich
operations with normalized document edits. Six clipboard regressions cover plain,
internal rich and HTML replacement in both directions, preserved formatting,
Unicode and following typing without changing the next paragraph. Those six plus
the real-workspace bookmark clipboard and selection/history tests pass. Logs are
`../../work/styles-paste-focused.log`; full-suite and native build 5 checks remain
pending. Rapid interleaving of Undo and typing, native input methods, and restoring
an original per-command selection after later navigation remain separate gates.

Build 5 automated follow-up: **222 Flutter checks pass with 2 optional AI-provider
skips**. Analysis is clean after adding braces to two test-only conditionals.
Eight focused paste/bookmark/selection checks pass. Production Rust remains
unchanged from 235 passing checks. Native build 5 paste continuation/history and
recovery remain pending; the reviewable package stays 0.52.0 build 2 meanwhile.

## 2026-09-20 — build 5 native paste progress; desktop locked

The signed 0.53.0 build 5 candidate matches 274 compiled source files and the
normalized Rust engine. The isolated native app reached revision 25 with the
body `Paste stays here`, unchanged neighboring document nodes and the caret
visibly at the replacement's end. The desktop locked before the next `!` paste;
the recovery file confirms that action did not apply. The partial verifier passes
replacement and user-workspace preservation only. Continuation, keyboard history,
DOCX export and restart remain unperformed for this candidate, recorded as false
in `../../work/native-style-paste/verification-partial.json`.

The subsequent launch attempt found both the normal 0.52 app and isolated 0.53
candidate running, but desktop control again reported that the Mac is locked and
automatic unlock failed. No new visible-launch verification is claimed. The normal
user workspace remains byte-identical at revision 30, and the reviewable release
remains 0.52.0 build 2. Unlocking the desktop is required for the remaining native
checks; source-level work is not blocked.

## 2026-09-20 — asynchronous history/input repaired; 0.53 build 6 launched

A deterministic real-engine test reproduced two separate failures before the
change: typing immediately after Undo was itself undone, and typing while Undo
was pending disappeared from the editor. The original failures are retained in
`../../work/history-input-before.log`. Undo now establishes its input boundary
synchronously. Later rich edits remain visible, retain their order and bookmark
journals, and are transformed through the history result before committing.
Rejected history retains following input. Rejected writes retain their exact
operations for retry, display an unsaved status, and block misleading Save/Close
and further history until the draft commits.

**228 Flutter checks pass, 2 optional AI-provider checks skipped**; analysis is
clean. Three delayed/immediate timing scenarios include repeated history,
Unicode, caret placement, bookmarks, failure/retry, saved recovery and DOCX
export. Three pure editor checks cover formatting precedence, exact local rich
operations and table journals through surrounding changes. Existing selection,
clipboard, table and draft-field history checks pass in the full suite. Logs:
`../../work/history-input-{focused,full,analysis}.log`. The independent reader
verifies the three `.folio`/DOCX pairs in `../../work/history-input-artifacts`.
The named-style DOCX reader also passes. Production Rust is unchanged from the
235 passing checks and matches the previously verified normalized engine hash.

After desktop access returned, build 5 completed its interrupted native paste
checks: continuation, Undo/Redo, DOCX export and byte-identical revision-29
restart all passed (`../../work/native-style-paste/verification.json`). Build 6
then passed native Undo followed immediately by typing, Redo followed by typing,
two successive Undo commands followed by typing, keyboard history after the
native Save dialog, DOCX export and byte-identical revision-40 restart. Every
retained state keeps neighboring document nodes and workbook data unchanged.
The independent native verifier and report are in
`../../work/native-history-input`; this is native keyboard smoke evidence, while
the automated regressions separately force the asynchronous timing windows.

The signed build 6 archive matches 275 compiled source files and the unchanged
normalized engine. ZIP SHA-256:
`68d500d75ab30ec83ca22aaebe7ac53f5fbf6e3b02e4b960523a3f024325277d`.
The previous release was backed up to `../../work/release-before-0.53-20260920`.
The reviewable release was updated to 0.53.0 build 6, and the normal app reopened
**A little room to think**, revision 30, with the user workspace byte-identical
(SHA-256 `9cc4bc3eb007052a88c7da3e6aa0a6e0eef54a969c163bdc0b561181ec8ee8ec`).
The launch record is `../../work/folio-latest-launch.json`. Codex is disconnected.

Exact original history positions for ambiguous repeated text, per-command
selection restoration after navigation, native IME and broader concurrent
formatting/refresh cases remain unfinished. See `docs/DOCUMENT_HISTORY_INPUT.md`.
No entire parity family is accepted; the full Word/Excel goal remains active and
additional novel AI features accepted remain 0/20.

## 2026-09-20 — exact document history positions, build 7 source

The real-engine regression reproduced a positional defect in repeated text:
Undo of a leading insertion in `aaaaa`, followed immediately by `X`, produced
`aXaaa` instead of `Xaaaa`. The before log is `../../work/exact-history-before.log`.
Rust history now retains validated forward/inverse logical text journals; UI
capture supplies them even without bookmarks. Undo/Redo returns them only in its
response, scoped to workspace and revisions. The editor projects the exact
locations and inserted rich content before reconciling remaining formatting.
History limits include journal memory; saved documents contain no history hint.

**239 Rust checks and 233 Flutter checks pass**, with 2 optional AI-provider
skips; analysis is clean. The targeted tests cover UTF-16 inverse locations,
repeated text/paragraphs, rich insertion order, opaque-table boundaries, malformed
journals, mixed document/workbook history, measured AI proposals, history reset,
continued input, export and reopening. Logs are
`../../work/exact-history-{rust,rust-focused,projection,focused,full,analysis}.log`.
The independent reader verifies ASCII, emoji and repeated-paragraph `.folio`/DOCX
pairs in `../../work/exact-history-artifacts`. Existing history-input and named
style export readers also pass. Native build 7 checks and release promotion are
pending; the normal app and reviewable release remain build 6 at this point.

## 2026-09-20 — build 7 native history qualification and launch

The isolated signed build 7 passes keyboard Undo/Redo immediately followed by
typing within repeated letters and emoji, retaining the intended insertion
location. Native DOCX export, subsequent keyboard history and restart also pass.
The saved revision-56 workspace is byte-identical before and after restart.
The independent reader verifies the Unicode text, neighboring paragraphs, named
22-point green style, headings and bullets, with workbook sheets unchanged.
Snapshots and the verification report are in `../../work/native-exact-history`.

The package matches 276 compiled Dart/Swift sources and the rebuilt Rust engine;
Rust source hashes are retained in `../../work/exact-history-package-verification.json`.
The reviewable release is now **0.53.0 build 7**. Its archive SHA-256 is
`b5e7beb61eaf3c7de0f87d9622e6ae20b3399801d1437e6bb798d1a214f9a11d`.
The previous build 6 archive remains in `../../work/history-input-package`.
The normal build 7 app was launched and brought forward on the user's existing
revision-30 document. Its saved workspace remains byte-identical; launch evidence
is in `../../work/folio-latest-launch.json`. The assistant shows Codex disconnected.

Original selection restoration after later navigation, opaque-embed internals,
untracked changes, native IME and broader concurrent editing remain unfinished.
No Microsoft Word comparison or full parity family is accepted by these checks;
the separate novel-AI acceptance count remains 0/20.

## 2026-09-20 — in-use style replacement, build 8 source

Styles → Delete now offers a replacement style and Cancel. Replacement updates
body and nested-cell style assignments, base/following/linked declarations and
schema-defined references in preserved Word XML. Source packages participate in
the same Undo/Redo transaction and saved-workspace recovery; pending text and
direct formatting remain intact. Descendant replacements are excluded in the UI
and rejected by the engine to prevent inheritance cycles.

**241 Rust tests passed; 234 Flutter checks passed, with 2 optional AI-provider
skips; analysis is clean.** New regression workflows cover invalid/atomic
replacement, body/table/dependent styles, preserved header/numbering references,
foreign XML/text retention, pending body edits, Cancel, history and recovery.
Independent python-docx/lxml checks pass against engine and real-UI exports.
Artifacts are retained in `../../work/style-replacement-artifacts`; logs are
`../../work/style-replacement-{rust,flutter,analysis,independent}.log`.

At this stage native build 8 qualification is pending. Missing built-in heading
creation, comprehensive built-in deletion rules and the rest of W02 remain open.
No full parity family or new AI feature is accepted by these automated checks.

## 2026-09-20 — build 8 native replacement qualified and promoted

The signed build 8 candidate matches 276 compiled Dart/Swift files and the rebuilt
Rust engine; source hashes are retained in
`../../work/style-replacement-package-verification.json`. Native Styles → Delete
shows the replacement choice. Cancel leaves the revision-56 saved workspace
byte-identical. Replace and delete changes the used Native accent paragraph to
Normal at revision 57, retaining its Unicode text, headings, list and other
paragraphs. Command-Z/Command-Shift-Z restore the complete before/after document.
The same shortcuts work immediately after the native DOCX Save dialog. Restart
recovers revision 61 byte-for-byte unchanged. The independent native verifier
confirms exported styles/text and all retained snapshots. Evidence is in
`../../work/native-style-replacement/verification.json`.

The reviewable release is **0.53.0 build 8**, archive SHA-256
`891347ebad82f3fd9392ad8d2819c2b8ee51345328bba4c2267398c0f9a619d6`.
Build 7 remains available in `../../work/exact-history-package`. The normal app
was updated with a backup of its revision-30 workspace. Its final startup and
saved-workspace hash are recorded in `../../work/folio-latest-launch.json`.
The native replacement evidence concerns the isolated sample; full Word/Excel
parity and the 20 additional novel AI features remain unfinished.

## 2026-09-20 — missing built-in headings, build 9 source

Styles offers missing Heading 1–9 definitions without modifying the document
until Apply. Ordinary heading commands and table drafts use the same definitions;
following-style Enter works during queued typing. The engine retains its current
protected catalog for queued content edits while explicit style commands remain
revision checked. Built-in origin metadata survives renaming, legacy catalogs
and export; built-in/default styles cannot be deleted, and user copies are custom.

**246 Rust tests and 235 Flutter checks pass, with two optional AI-provider
skips; analysis is clean.** The new editor workflow covers missing-style Apply,
rapid heading/Enter/body typing, table Cancel/Apply, all nine rendered levels,
Undo and saved/reopened exports. Engine cases additionally cover absent style
parts, non-paragraph ID collisions, invalid input atomicity and legacy origin
interpretation. Independent python-docx/lxml readers pass for the new heading
files and the existing named-style and replacement artifacts. The heading reader
uses OOXML IDs because python-docx's display-name lookup normalizes built-in aliases.

Source behavior and limits are in `docs/BUILTIN_STYLES.md`; logs are
`../../work/builtin-styles-{rust,flutter-full,analysis,independent}.log`.
Native package qualification follows separately. Full Word/Excel parity and the
20 additional AI features are still incomplete.

## 2026-09-20 — build 9 native headings qualified and promoted

The locally signed candidate matches 277 compiled Dart/Swift files and the rebuilt
Rust engine. Archive SHA-256 is
`06cff350ae22f81668549d3de69df8a574cf2f283aa6f7f85ab4c6f9173c1e7d`
(141,806,943 bytes); source/engine hashes and signature checks are retained in
`../../work/builtin-styles-package-verification.json`.

The native check imported the independent DOCX with all nine paragraph-heading
declarations removed. Styles displayed Heading 9 as Added when applied; Cancel
left revision 62 byte-identical. Apply created Heading 9 and styled only the first
paragraph at revision 63. Built-in Delete was visibly disabled both before and
after materialization. Command-Z/Command-Shift-Z restored the full before/after
document at revisions 64/65. Table-cell Heading 7 remained an unsaved draft until
outer Apply (revision 66); native keyboard history restored/reapplied it at 67/68.

Native DOCX export retained semantic levels 9 and 7, base/following Normal, custom
Project body and the first run's explicit false bold override. Keyboard Undo/Redo
immediately after the Save panel worked at revisions 69/70. Quit/relaunch
recovered revision 70 byte-for-byte unchanged and rendered both headings. The
independent native reader checks these exports and saved snapshots; its report is
`../../work/native-builtin-styles/verification.json`. Table Cancel and rapid Enter were covered by the real-engine editor regression,
not separately exercised through physical desktop input in this pass.

The reviewable release is 0.53.0 build 9. Build 8 remains in
`../../work/style-replacement-package`; the normal workspace was backed up before
the update. Startup and workspace hash evidence is retained in
`../../work/folio-latest-launch.json`. No actual Microsoft Word comparison, full
parity family or new AI feature is accepted by this scoped qualification.

## 2026-09-20 — paragraph outline levels, build 10 development

Paragraph → Outline level edits levels 1–9, Body text and Use style in body and
cell editors. Direct outline metadata is separate from the visual style and its
inheritance. Outline changes preserve typography, rich marks, characters and list
settings; semantic headings and navigation use the effective level. Clearing an
override survives recovery, while legacy baselines hydrate previously unmodeled
source settings without discarding local text or format edits.

Focused engine/editor checks and independent DOCX reads pass for all ten values,
style Apply/Modify/replacement, explicit body overrides, clearing/recovery,
alternate namespace prefixes, unrelated extension attributes, mixed selections,
Enter, table Cancel/Apply and history. The renderer test checks identical font
size and weight across all outline levels for imported and native paragraphs.
See `docs/OUTLINE_LEVELS.md` and `scripts/verification/verify_outline_levels.py`.
Full final-source results and native qualification will be recorded separately.

The final functional suite passes **250 Rust tests and 240 Flutter checks**,
with two optional AI-provider skips; analysis is clean. A final source-identity
lookup optimization also passes the full 250-test Rust suite. Separate engine
and app-output readers pass. Logs are `../../work/outline-{rust,flutter-full,
analysis,independent,ui-reader}.log`. The final packaged engine and native UI
qualification follow separately.


## 2026-09-20 — build 10 normal-workspace launch

Promoted the verified outline-level package to `release/` and launched the extracted signed ZIP. Native UI shows “A little room to think”, 76 words, revision 30, and Saved on this Mac. The normal workspace is byte-identical before and after startup. Codex is disconnected. Package hash: `f7970568fe70afb99fc30ef215f916274641ecad56933f36f87eb3a3373f75ea`. Launch receipt: `../../work/folio-latest-launch.json`. Prior build 9 remains archived in `../../work/builtin-styles-package/`. This records startup and persistence only, not full Office parity or an AI-provider run.


## 2026-09-20 — build 10 native outline qualification

The isolated signed package passed native Body text / Use style, Cancel,
keyboard Undo/Redo, DOCX export, post-Save history and byte-identical restart at
revision 77. Other paragraphs and sheets stayed unchanged. Independent DOCX
inspection retained custom visual styles, 14-point text and explicit false bold,
removed only the cleared outline override, and preserved other direct/inherited
outline settings. The normal user workspace stayed byte-identical throughout.
Native table Apply was not separately exercised in this pass; the real-engine
editor flow covers it. Microsoft Word execution remains unverified.
Reports: `../../work/native-outline/verification.json` and
`../../work/outline-package-verification.json`. Full tests are the 250 Rust and
240 Flutter checks recorded above, plus 3 final packaged-engine editor flows.


## 2026-09-20 — build 11 native File commands

Added native macOS File commands and shared Flutter shortcut/ribbon/sidebar
routing: New Workspace, Open, Save Workspace Copy, Export Word/Excel and
Compatibility Report. The command gate blocks duplicate operations and commands
behind dialogs; New/Open retain the assistant-busy guard. Save Copy waits for the
full document/history/worksheet queue. A discovered rename-dialog controller
lifetime defect is fixed by letting its draft field own the controller.

**241 Flutter checks passed**, with two optional AI-provider skips; analysis is
clean. The real-engine file flow covers delayed Undo plus queued typing before
Save, duplicate commands, Open cancellation/failure, Flutter modal isolation,
snapshots before replacement, DOCX import/export, inline cell commit before XLSX
export, New and saved-workspace reopening. AppKit menu checks pass in memory
(order, shortcut modifiers, dispatch, repeat suppression, sheet/dialog/view state).
The keyboard adapter checks also pass. Independent python-docx/openpyxl reads
pass for the UI-produced files. The Rust engine is unchanged and matches build
10's independently qualified engine, whose full 250-test suite passed.

Isolated packaged native checks pass for menu Open and document Command-O,
Open Cancel, File-menu disabling during native and Flutter dialogs, Command-O
while Paragraph remains open, Command-S Save Copy, Command-N, reopening the saved
copy, and Command-Shift-E in both views. An active B4 edit to 4567 commits before
Excel export; independent file reads confirm B4=4567, retained SUM(B4:B7), and
cached B9=22167. The File menu changes between Export Word and Export Excel.
Post-export Command-Z/Shift-Z restored/reapplied the edit. DOCX retained custom
style, outline text and the table cell. The saved copy and snapshot before New
match exactly. Quit/restart retained revision 7 byte-for-byte.

The verified ZIP is 0.53.0 build 11, SHA-256
`9372de1d8a2526fa92e778ae4f4c0135f9b6ec70c6ebe2e6837a1653c0770096`.
278 Dart/Swift source files match the compiled staging tree; the packaged native
engine matches the retained build 10 engine. Promoted to `release/`; prior build
10 is retained in `../../work/outline-package/`. Normal app startup shows the
user's unchanged 76-word document at revision 30, Saved on this Mac, with the
new native File menu and Codex disconnected. The recovery-file hash is unchanged.

Logs and source/native evidence are retained in `release/validation/file-commands/`;
launch receipt: `../../work/folio-latest-launch.json`. Scope and remaining gaps:
`docs/FILE_COMMANDS.md`. No Microsoft Office execution, complete parity family,
or additional novel AI feature is accepted by this increment (0/20).


## 2026-09-20 — 0.54 Save and Save As development

Save/Save As now track separate active DOCX/XLSX locations and a native `.folio`
workspace location. Export and Save Workspace Copy retain their original meaning.
Local-only security-scoped bookmark records restore selected-file access; source
package identity prevents an undone import from retargeting unrelated files.
Association transitions reserve metadata before an external Save As; interrupted
updates restore no old destination. Corrupt metadata clears runtime associations.

The full suite passes **251 Rust tests and 243 Flutter checks**, with 2 optional
AI-provider skips. Analysis is clean. After the final corrupt-metadata handling
and formatting adjustment, all 3 affected editor/state/file-command flows pass
again. Native AppKit menu checks and 47 keyboard adapter checks pass in memory.
Independent readers confirm DOCX text across original/renamed/Save-As versions,
XLSX B4=5678, retained SUM(B4:B7), cached B9=23278, and the saved full workspace.

The engine rechecks expected file fingerprints after preparing output bytes,
before atomic replacement, and returns the SHA-256 and revision of its saved
bytes. Changed/missing files and invalid fingerprints reject writes without
changing the workspace or destination. These checks do not establish full
coordination with uncooperative concurrent writers. See `docs/WORKING_FILES.md`.
Logs: `../../work/working-files-{rust,flutter-full,final-focused,analysis,reader,
appkit-test,keyboard}.log`. Packaged native bookmark/restart qualification follows
separately. No complete parity family or new AI feature is accepted.


The additional real-application external-writer race check passes: a write arriving
after the UI preflight is rejected by the engine, and a write immediately after
the engine save cannot become the saved fingerprint baseline. The subsequent
Save detects the conflict, retains the external bytes and releases scoped access.
This is one additional check after the 243-check full suite.

The 0.54.0 build 1 signed ZIP has been built and extracted, with 281 Dart/Swift/
entitlement files matching staging and the native engine matching the build.
SHA-256: `39bafd68f6bc6bf42257e505daf6e5fc9fb326516abcc5dc5cc9682ad9adb4b3`.
The isolated Folio Save Check process is running, but macOS is locked; CUA could
not unlock it. No native startup, bookmark access, Save/restart acceptance or
release promotion is claimed. The normal app/release remains 0.53.0 build 11;
the user workspace hash remains unchanged. Resume from
`../../work/working-files-native-pending.json` after unlock, using the live process
and exact preview path; do not create another candidate solely because UI
observation failed while locked. Development evidence is retained in
`release/validation/working-files-development/`.


## 2026-09-20 — 0.54 build 2 file state and leave-file decisions

The engine now reports independent DOCX/XLSX/full-workspace content signatures.
DOCX resolves linked ranges, XLSX includes cached results/errors, and `.folio`
ignores the history revision counter. Saved responses carry the exact content
signature that produced their bytes. Source lineage remains stable when package
parts are revised, while new imports get separate identities. Native style
replacement, imported row insertion, linked values/number formats, unrelated edits,
Undo/Redo and save/reopen are exercised by `file_state_tests`.

Full regression: **256 Rust tests passed** and **246 Flutter tests passed**, with
**2 optional AI-provider skips**. The full Flutter run preceded only cache cleanup
on engine New/Load and lint-only test changes; affected workflows are rerun against
the final release engine below. Analysis is clean. UI save/race tests prove that
later typing during bookmark capture stays dirty and unrelated workbook edits do
not dirty the document. Source changes retain the selected Save location.

`working_files_leave_test.dart` uses the actual application, engine, editor and
native method handlers with OS file selection/grants mocked. It verifies two-file
Save before New, unrelated edits retained during Open, replacement Cancel, failed
access, conflict plus cancelled Save As, repeated close/menu rejection, native
Close Cancel and Save, Don’t Save preserving external bytes/outgoing snapshots,
and clean association restoration. Untitled workspaces continue local recovery;
this is not proof of a conventional discard/revert or first-save prompt.

The production Swift close method is extracted into an in-memory harness: false,
null and invalid replies do not approve closing; true does; the subsequent window
close/quit handoff avoids another prompt. Lifecycle/termination and AppKit File
menu checks also pass without posting desktop input. Independent python-docx and
openpyxl readers confirm Save-on-Close text, absence of discarded external edits,
B4=6789, retained SUM(B4:B7) and its matching cached total. Existing Save/Save As
reader checks also pass. openpyxl warns about a preserved unknown extension while
reading; these checks do not write the files back or establish full Excel fidelity.

Logs are `../../work/file-state-{rust-full,flutter-full,analysis,readers,package}.log`
and `../../work/file-state-flutter-workflows.log`. Native Save/bookmark/restart and
visual prompt checks remain pending; the normal release is still 0.53.0 build 11.
See `docs/WORKING_FILES.md` for current behavior and the remaining D02 requirements.
No full Word/Excel parity family or novel AI feature is accepted by this increment.

Final release-engine rerun: all **6 affected file-state/editor/leave/race/menu flows pass**. The signed build 2 ZIP was extracted and verified; 281 Dart/Swift/entitlement files match staging, and the normalized native engine matches the final release build.

ZIP SHA-256: `2e328ddc8999a13d9914c2423310bc46d6bb4e767514d903771df03995c6c9cd`. Package: `../../work/working-files-build2-package/Folio Native.zip`.
The app inventory became readable, but opening the new isolated preview still reported a locked Mac. No visual startup, native bookmark/restart or prompt acceptance is claimed. The normal user workspace SHA-256 is unchanged, and `release/` remains 0.53.0 build 11. Resume from `../../work/working-files-native-pending.json`; build 2 supersedes build 1 for qualification because its code changed, not because a UI timeout implied process termination.
Evidence bundle: `release/validation/working-file-state-development/`.


## 2026-09-20 — 0.54 build 3 unnamed saves and explicit discard

Initial document/workbook content signatures are persisted locally. Changed
unnamed views participate in New/Open/Close decisions independently; unchanged
new work does not prompt. Save uses the target part’s format even when the other
view is active. Cancelled first Save As cancels the transition. If DOCX succeeds
and XLSX is cancelled, the workspace remains open, the document stays saved, and
the next prompt covers only the workbook. Unknown legacy recoveries remain
conservatively unsaved. Outgoing content still gets a recovery snapshot.

Don’t Save on Close now atomically writes a fresh blank recovery workspace after
preserving the outgoing snapshot. It does not mutate live content/history before
the OS closes the window; failure cancels Close. Recovery persistence is sealed
inside the same engine transaction queue, preventing a flush queued during the
blank write from reviving discarded data. New/Open/Close temporarily disable the
editing surface and wait for focus changes before flushing; Cancel restores focus.
Normal Save continues to permit editing.

**257 Rust tests passed.** The new engine check verifies a failed discard write,
unchanged live content and Undo, a blank next-launch workspace, and recovery of
the discarded snapshot. **8 focused Flutter flows passed** before the additional
queued-autosave guard; the four affected leave/menu/race flows then passed again,
including a deliberately held blank write and an autosave queued before its result.
The full regression and package qualification are recorded below.

`untitled_workspace_test.dart` uses the real application, engine and editor with
OS file selection/grants mocked. It exercises first-save Cancel/Save, correct
per-part picker formats, a partly cancelled two-file save, restored focus, failed
Close with live history retained, external-file preservation, successful discard,
blank restart, stale grant rejection, and reopening the discarded snapshot through
ordinary Open. Independent python-docx/openpyxl readers verify both DOCX first saves,
B4=8888 in XLSX, no discarded text in the external DOCX, blank restart contents and
preserved discarded text in the recovery copy. openpyxl’s preserved-extension
warning does not imply a write-back; the verifier reads only.

See `docs/WORKING_FILES.md` for behavior and remaining D02 work. No native startup,
sandbox bookmark/restart, physical IME, complete Word/Excel parity, or novel AI
feature is implied by these checks. The normal release remains 0.53.0 build 11
until native qualification succeeds.

Final full Flutter regression after the queued-autosave/focus guard: **248 passed, 2 optional AI-provider skips**. Analysis is clean. Independent reader checks pass; no runtime source changed after this run. A test-only brace/lint adjustment has no behavior change.

Build 3 ZIP SHA-256: `70adc23b7988c9d1b2704bdf513d4edf1b2239685fca72dbbdfc90a9935d3033` (141909548 bytes). Fresh extraction passes strict/deep signature verification; 281 Dart/Swift/entitlement files match staging and the packaged native engine matches the release build. Archive: `../../work/working-files-build3-package/Folio Native.zip`.

The isolated build 3 process is live (49407), but both initial and repeated observation could not display a window; the repeated request explicitly reported the Mac locked. It has not been restarted on the basis of the observation failure. No native startup/Save/restart acceptance is claimed. User recovery SHA-256 remains `9cc4bc3eb007052a88c7da3e6aa0a6e0eef54a969c163bdc0b561181ec8ee8ec`, and the normal release remains 0.53.0 build 11.
Resume native checks from `../../work/working-files-native-pending.json`. Logs and source/artifacts: `release/validation/untitled-file-lifecycle-development/`. Full parity and novel AI acceptance remain incomplete (0/20).


## 0.55 recent files and recovery development — 2026-09-20

`docs/RECENT_FILES.md` describes the new File/sidebar browser, pinned recent
locations, migrated history, immutable recovery locations, preview and new-identity
restore. This is development evidence; the normal release remains 0.53.0 build 11.

- **259 Rust tests passed.** Preview leaves active content/history unchanged,
  invalid recovery cannot replace it, restore gives a fresh identity, both imported
  Office packages retain their DOCX/XLSX content signatures and exported ZIP parts,
  and recovery source bytes remain unchanged.
- **255 Flutter tests passed; 2 optional AI-provider checks skipped.** New coverage
  includes history migration before blank-restart reset, serialized metadata writes,
  pins/capping/renames, non-destructive removal, damaged metadata, legacy copies,
  stale/failed previews, whole-workspace drafts and real-engine browser flows.
  A startup timing failure was corrected by capturing the unnamed baseline before
  history migration; the original shortcut/save regression now passes again.
- The two affected browser workflows passed after the final preview-label edit.
  A separate real-font widget-render run also passed. The inspected PNG is an
  automated Flutter render with test content, not a native macOS screenshot.
- Independent python-docx/JSON reads confirmed the saved document, recovered
  document plus worksheet, fresh recovered identity, seven separate snapshots,
  moved-file pin and absence of an unsaved edit from the external Word file.
- Analysis is clean. Actual AppKit File-menu, close and lifecycle code passed
  in-memory checks; these send no desktop events. Pickers/bookmarks in Flutter
  tests are mocked, not proof of sandbox persistence or Finder movement.

The final archive is **141,986,277 bytes**, SHA-256
`ea635635957e376ac1d4aa5dc935f24740965c49cc23bfa6c10a798762948073`.
Strict/deep signatures passed after extraction. All **283 Dart/Swift/entitlement
source files** matched staging; the packaged engine matched the release engine
after normalizing signatures on temporary copies. Preview bundle identity is
`com.folio.history055.check`, named **Folio History Check**. It is prepared but
not launched; existing preview observation still reports Mac locked.

Native review, saved-file bookmark restart, moved-file access, physical keyboard/IME,
accessibility and actual Office comparison remain pending. The receipt is
`work/recent-files-native-pending.json`. Your normal recovery remains byte-identical
to the pre-check backup. Do not promote this package or claim D02/full parity from
these automated checks. No parity family is accepted; novel AI remains 0/20.


## 0.56 Word and Excel templates development — 2026-09-20

`docs/TEMPLATES.md` describes the six authored starters, personal template library,
Create versus Edit, correct Office package types and source protection. This is a
development candidate; the normal release remains 0.53.0 build 11.

- **263 Rust tests passed.** Four new scenarios cover every bundled template,
  isolated preview, fresh identity and first-save baselines, no inherited Undo,
  source bytes, recovery, regular/template content types, retained package parts,
  explicit template editing, namespace prefixes, ambiguous types, stale sources,
  corrupted files and disguised macro-bearing packages.
- **257 Flutter tests passed; 2 optional provider checks skipped.** New real-engine
  gallery coverage exercises first-save cancellation, DOCX/XLSX creation, copying
  templates without retargeting or marking the original saved, explicit DOTX
  in-place editing and Save As, custom workbook formulas, stale preview rejection,
  browsing/cancelling, source preservation and balanced mocked file grants. A
  gallery test verifies independent starter copies, stale/failed previews,
  disabled Create, search and bundled-template edit restrictions.
- Real-font visual review exposed raw formulas in workbook previews. The preview
  now reads the shared formatted-cell text. Four focused engine tests and the two
  affected Flutter workflows passed after that correction, including explicit
  `$0.00`/`$875.00` assertions. Inspected PNGs are automated widget renders, not
  native desktop screenshots. The real-font run also waits for scrolling to settle
  before clicking a personal-template row.
- Independent python-docx, openpyxl and XML reads verify edited DOTX versus its
  independent DOCX copies, base font style, XLTX versus XLSX package identities,
  retained formulas, cached 875 result, currency formats, frozen headers and the
  starter's status validation list. openpyxl reports an unsupported retained
  extension; the verifier only reads files and never resaves them.
- Analysis is clean. Production AppKit File-menu code passes an in-memory test,
  including new template commands and agent-busy/modal gating; no desktop events
  are sent by that check. Sandbox bookmark behavior is still mocked in Flutter.

The final ZIP is **142,153,753 bytes**, SHA-256
`693ab072907dafe133f763400c944a4a429d921bd8c3c46002dad707632992c8`. Strict/deep signatures pass after fresh extraction. All
**285 Dart/Swift/entitlement sources** match staging, all six template assets
match source/staging/the actual packaged Flutter assets, and normalized engine
bytes match the release binary. The isolated **Folio Template Check** bundle uses
`com.folio.templates056.check` and a blank test workspace. It is prepared but not
launched: existing-process observation still reports the Mac locked.

`work/templates-native-pending.json` carries forward file/recovery qualification
and adds template-native checks. User recovery SHA-256 remains `9cc4bc3eb007052a88c7da3e6aa0a6e0eef54a969c163bdc0b561181ec8ee8ec`.
No native template/restart/physical-keyboard/accessibility or Microsoft Office
application acceptance is claimed. Global/default/macro/legacy templates and the
other D02/W02 requirements remain unfinished; full parity and novel AI 0/20 remain
incomplete.


## 0.57 document review comments — development evidence (2026-09-20)

Added range/point comment threads, explicitly entered author names, replies,
plain-text editing, resolve/reopen, filters, deletion, Go to text, Undo/Redo and
recovery. Review controls, the document context menu and Command–Option–A /
Control–Alt–M expose the workflow. See `docs/COMMENTS.md` for remaining scope.

- Full Rust suite: **271 passed**, including eight new comment tests.
- Full Flutter suite: **258 passed, two optional AI skips**. This full run
  preceded the final imported-comment style preservation and filter adjustment;
  the final Rust suite includes that preservation fix, and the affected complete
  real-bridge widget workflow passed again after the filter adjustment. The
  final source analysis is clean.
- Exact-edit/history/recovery tests cover Unicode boundaries, repeated text,
  stale revisions, metadata injection, deletion/Undo and explicit AI whole-
  document replacement. DOCX cases include table/cross-paragraph anchors,
  overlapping bookmarks, threads/status, alternate prefixes and part targets,
  rich source preservation, modern metadata removal and retained plain-comment
  styles/unknown attributes when its text is edited.
- Microsoft Open XML SDK **3.5.1**, Microsoft 365 validation: **zero errors**
  for both the cross-paragraph/table fixture and the final Flutter-authored file.
  Its first run caught the wrong extended-comment content type; that defect was
  fixed before packaging. `scripts/verification/openxml` reproduces the check.
- Independent python-docx/lxml reads confirm both comment bodies and authors,
  the parent-paragraph link, resolved flags, paired body anchors, valid unique
  paragraph IDs and unchanged input bytes. Real-font widget renders were
  inspected; they contain synthetic review content and are not native screenshots.

The final ZIP is **142,252,091 bytes**, SHA-256
`c8c415a87712474e883b635fc2be3404cbc1d59ac5475ac1a0ff66889271464c`. Strict/deep signatures pass after fresh extraction.
All **286 Dart/Swift/entitlement sources** and all six template assets
match staging, and the packaged engine matches the release engine after signature
normalization (`22c74284e60eb0726f27d5246207c736541b8ab3db8897969157ccf21bdc3952`).

The isolated **Folio Comment Check** bundle (`com.folio.comments057.check`) is
prepared, signed and seeded with a blank test workspace, but **not launched**.
Desktop control rechecked the live normal app and still reports the Mac locked.
The normally running app remains **0.53.0 build 11**. User recovery is unchanged:
`9cc4bc3eb007052a88c7da3e6aa0a6e0eef54a969c163bdc0b561181ec8ee8ec`. No native UI or Microsoft Word desktop acceptance is claimed.
`work/comments-native-pending.json` retains all file/recovery/template gates and
adds comment-native checks. W13/D07, Excel comments, full parity and the novel
AI requirement remain incomplete; novel AI accepted is still **0/20**.

## 0.58 worksheet comments and notes — development evidence

The preceding 0.57 startup status has changed: on 2026-09-20, **Folio Comment
Check** launched and its Word editor was visibly brought forward through desktop
control. The normal user workspace hash remained unchanged. This verifies
startup only; it does not close the inherited file, template or Word review gates.

0.58 adds separate notes and threaded cell comments with reachable Review and
context-menu controls, workbook-wide navigation, corner markers/hover text,
Shift+F2, posting shortcuts, resolve/reopen, editing/deletion, draft protection,
Undo/Redo and annotation-only Paste Special. Details and remaining scope are in
`docs/SHEET_REVIEWS.md`.

- Final Rust suite: **276 passed**. Five new review tests cover XLSX threads and
  notes, author identities, unique copy identities, source preservation, exact
  no-edit bytes, structural edits, sort, paste, history and recovery. Imported
  omitted reply references and mismatched root/legacy anchors move and delete
  together; rich runs and VML types survive copying. Protected-sheet and
  preserved unsupported annotation collisions reject atomically.
- Full Flutter suite: **259 passed, two optional AI skips**. The complete
  real-bridge sheet review workflow covers posting, replies, editing, resolution,
  right-click, Shift+F2, marker presence, Paste Special without values, history,
  draft retention, stale rejection, cross-sheet navigation, filters, XLSX and
  recovery. The final small-window and unchanged-edit draft refinements were
  subsequently checked by rerunning this complete affected workflow. Analysis
  is clean. A synthetic real-font widget image was inspected; it is not a
  desktop screenshot.
- Microsoft Open XML SDK **3.5.1**, Microsoft 365 validation: **zero errors** in
  the Rust-authored thread/note fixture, copied rich-note fixture and final
  Flutter-authored workbook. Independent OpenPyXL/lxml reads confirm legacy
  records, messages, people, reply parents, shared cell refs and VML markers;
  all input bytes remain unchanged. OpenPyXL warns that it does not support an
  extension; it is used read-only and is not relied upon to save threaded data.
- Microsoft Word and Excel applications are absent from `/Applications` on this
  host. No actual Office desktop round-trip comparison is claimed.

Source changes and test evidence establish progress, not a completed parity
family. Full Word/Excel parity remains incomplete and additional novel AI
features accepted remain **0/20**. Packaging/native evidence follows below.

The isolated **0.58.0 build 1** preview was packaged, signature-verified and
launched as **Folio Sheet Review Check**. ZIP: **142,376,809 bytes**, SHA-256
`24e4ad937464f20c807634b7756b0542f26a688a99882ebc30b301333fbd3c31`.
All **287 Dart/Swift/entitlement sources** and six template assets match staging;
the packaged Rust engine matches after signature normalization
(`a2621cd5b9f918cf22c0ef1b00381bb57067bc2a6f5f3701a96c9d00e39294d5`).

Native checks passed on a disposable two-sheet workbook: note creation with
explicit attribution, Undo/Redo, adding a reply with Command+Return, resolving
an imported thread, right-click Edit note, Shift+F2, unchanged-note dismissal,
cell markers/hover text, All sheets and Go to cell navigation. Native Save As
created `review-native.xlsx`; native reopening showed the exact new note and
three-message resolved thread. The reopened workspace reached revision **7**.
Independent readers confirm five annotation cells across both sheets and four
threaded messages; the native output passes Microsoft 365 SDK validation with
**zero errors**. SHA-256:
`36423ee5168c948db90f46854ef8aeb57e79cf948ed3b42f2fc32dbc1b66cd06`.
The original input and normal user workspace remained byte-identical.

The native file picker needed its exposed Open accessibility action, and some
AX field values lagged the visible rendered text; screenshots plus posted and
saved values were used to check the actual result. Full accessibility acceptance
is still open. Native quit/restart, additional annotation editing/structural/
paste flows and the inherited file/recovery/template/Word gates remain pending.
The normal 0.53.0 build 11 release was not replaced. The 0.58 preview remains
open on the synthetic Forecast sheet. Evidence and exact pending scope are in
`release/validation/sheet-reviews-development` and
`work/sheet-reviews-native-verification.json`.


## 0.59 Track Changes review development — 2026-09-20

Previous goal turn: progress, with native 0.58 preview startup and visible
spreadsheet state reverified. This turn implements imported revision review;
the full parity objective and 20-feature AI phase remain incomplete (0/20).

- 284 Rust tests passed before the final direct-formatting preview refinement;
  all eight affected revision tests passed again afterward. Coverage includes
  namespace-safe text restoration, nested bulk operations, formatting snapshots,
  paragraph/row behavior, protected and unsupported records, crossing-range
  atomic failure, pending edits, source lineage, model/export guards, live
  workbook refresh and boundary failure, Undo/Redo and recovery.
- The real-engine Flutter workflow passes Accept/Reject, filtered bulk actions,
  history, export/recovery and stale-review failure. A 960x680 check exposed
  pre-existing fixed-width ribbon/status overflow with wide test fonts; those
  rows now retain accessible controls. Both Word and worksheet review workflows
  pass after that fix. The final full suite and package checks are recorded below.
- Independent lxml inspection passes Unicode/escaping, exact text, rich marks,
  paragraph joins, property reversion, nested edits, rows, workbook refresh,
  pending-edit retention and absence of temporary metadata.
- Microsoft Open XML SDK 3.5.1/Microsoft365 reports zero errors for 13 engine
  fixture packages, the real-engine UI export and the authored native demo input.
- `work/revisions-widget.png` is a synthetic widget capture, not a native
  screenshot. Preview markup is not a claim of precise Word typography/pagination.

Recording new tracked edits, all remaining revision types/contexts, annotation
semantics for complete subtree removal, nested live workbook bindings,
exact page navigation, display modes/balloons, Compare/Merge, native restart and
Microsoft Word differential qualification remain open. See docs/TRACK_CHANGES.md.
No Word/Excel family or novel AI feature is accepted complete.


0.59 final checks: **260 Flutter tests passed, two optional AI checks skipped**;
analysis is clean. The signed isolated 0.59.0 build 1 ZIP and fresh extraction
match 288 Dart/Swift/entitlement source files, six template assets and the built
Rust engine. Package hashes are in `work/revisions-package-verification.json`.

Native checks passed sample import; formatting comparison and rejection;
Undo/Redo; Accept All of the remaining four revisions; visible revised text,
paragraph join and editable table; native DOCX Save As; actual quit/relaunch
recovery; and bookmarked Save after restart. The native output passes independent
text/formatting/structure assertions and the Microsoft365 SDK with zero errors.
The original sample was reopened and the Track Changes window left visible with
five revisions. The normal user workspace fingerprint is unchanged.

`work/revisions-native-verification.json` records exact checks and remaining
boundaries. The normal 0.53.0 build 11 release was not replaced. No parity family
or new AI feature is accepted (0/20).

## Track Changes authoring engine/editor development — 2026-09-20

Previous goal turn: progress, reverified and brought forward the isolated 0.59
desktop preview. This turn adds source-preserving authored revisions and
connects them to ordinary engine document commits. The packaged 0.59 app was not
replaced, recording controls are not exposed yet, and native authoring has not
been verified. The goal remains active; no parity family or novel AI feature is
accepted (0/20).

- **305 Rust tests pass**, including exact Unicode/repeated-text replacements,
  multi-paragraph edits, edits inside table cells, author metadata, prior
  revision preservation, formatting snapshots, source-only properties, explicit
  font/theme changes, paragraph-mark typing format and protected/unsupported
  atomic failures. Recording settings, history, review, recovery and DOCX export
  pass through the actual engine command path.
- **261 Flutter tests pass; two optional AI tests skip.** The added real-engine
  workspace check queues rapid edits without waiting for native replies, then
  exercises formatting, Enter/Backspace, Undo/Redo, export, Reject All and
  reopening, followed by more typing. Existing imported revision review passes.
  The first attempt failed on a test-harness asynchronous expectation guard;
  changing that callback to expectSync fixed the harness without changing the
  product behavior. Static analysis is clean.
- Microsoft Open XML SDK 3.5.1 reports **zero errors for 21 DOCX packages**,
  including the real-editor export. A broader paragraph-mark fixture initially
  caught invalid marker ordering and paragraph snapshots containing mark
  properties; both were fixed and revalidated.
- Independent lxml inspection verifies exact visible text, revision authors and
  unique IDs, preserved media bytes, source-only run/paragraph formatting,
  custom settings-part paths and revision/formatting recording flags.

The release-mode native library was built for bridge tests. No desktop package
was built or promoted in this turn. `release/validation/tracked-authoring-development`
contains source hashes, synthetic fixtures/recovery and final test/reader logs;
`work/track-edit-verification.json` records scope and engine identity.
See docs/TRACK_CHANGES_AUTHORING.md for the unfinished UI, structural, annotation,
live-binding, performance, accessibility and Microsoft Word comparison work.

## Track Changes recording controls — 0.60 development, 2026-09-20

Previous goal turn: progress, the packaged 0.59 preview was brought forward and
its visible editor state verified. This turn exposes the recording controls,
explicit imported-file author setup and rejected-draft recovery. No parity
family or novel AI feature is accepted; the full goal remains active (0/20).

- **308 Rust tests passed.** An imported file's Word recording flag now requires
  an explicit author or recording-off choice before editing. Unsupported inline
  marks reject without silently dropping formatting from the tracked export.
- **261 Flutter tests passed; two optional AI checks skipped.** The real-engine
  workspace check drives author validation/Cancel/Apply, queued typing,
  paragraph and format edits, history, export, review, reopening, a rejected link
  edit with blocked Save, draft discard, resumed typing and imported author setup.
  Control+Shift+E is exercised with the editor focused. It previously conflicted
  with Export; the document shortcut map now reserves that combination while
  retaining Command+Shift+E for Export. The Review ribbon scrolls horizontally
  to avoid overflow at narrower widths. Static analysis is clean.
- **22 DOCX exports passed Microsoft365 Open XML SDK validation with zero
  errors.** The independent ZIP/XML reader verifies exact current content,
  revision metadata, preserved source properties/media and recording settings.

Reports are in `work/tracking-ui-*`. Packaged native verification is recorded
separately below when completed; these automated results alone do not establish
native behavior or equivalence with Microsoft Word.

The first packaged native check found an end-of-document case that the original
widget scenario missed. Native Enter can insert after the existing terminal
newline (the captured journal inserts at 108 in a document of length 108).
The region mapper had excluded that position. Build 2 includes the final
editable region's endpoint and maps terminal-newline appends to Word's final
paragraph-mark convention. The native source package and exact rejected
before/after/journal are now synthetic regression fixtures. Tests verify Enter,
continued typing by a second author, retained earlier authorship and rejection
of only the new author's two changes. A separate test covers both equivalent
terminal-newline insertion positions. The exported seven-change fixture passes
schema and independent current-text/author/identifier checks. Temporary native
diagnostic logging was removed before the final build.

Build 2 saved that native Enter, but the next native character exposed the
related editor-side issue: input after Quill's hidden terminal newline can
produce different tree and Delta contents in release mode (debug asserts).
Build 3 normalizes that specific platform insertion and its caret before the
sentinel. The real-engine macOS widget scenario now sends the native-shaped
editing value, verifies the new paragraph and caret, and types `Confirmed.` into
that paragraph after a rejected-link/discard sequence. Existing selection and
queued-history checks pass alongside it. File-level normalization remains useful
for exact journals already recorded at the terminal boundary.

Final isolated **0.60.0 build 3** native qualification passed:

- Cmd+Down, Enter places the caret in a new paragraph; typing `Confirmed.`
  stays there, with five visible paragraphs and 16 words. The review contains
  five earlier Alex changes and two new Morgan changes.
- Filter Morgan, Reject all shown removes only those two changes. Closing
  review and Command-Z restores the fifth paragraph and both changes.
- Native Save As writes `continued-native.docx`. Reopening that file retains
  the content and requires explicit author setup. Choosing Morgan enables
  editing without adopting Alex's identity.
- Save, quit (confirmed absent process), relaunch and post-restart Save retain
  Morgan, the working-file binding, all five paragraphs and seven revisions.
  The clean-quit and restarted recovery files are byte-identical, SHA-256
  `8c8eb3b18e715655555cae8ccd65ce3eb8a18fe8a75355b2feae94a3076439a9`.
- The three actual native-saved DOCX files pass Microsoft365 schema validation
  with zero errors. The independent reader verifies current text, per-author
  revision counts, distinct IDs, paragraph marks, formatting and settings.

An initial quit from the synthetic seeded workspace prompted about an unsaved
workbook. Selecting Don't Save correctly replaced the active workspace with a
blank one and retained recovery separately; that run is **not** counted as
unchanged recovery. The clean restart check above followed reopening the saved
DOCX, explicit author setup and Save, then quit without a discard prompt.

Build 1 native checks additionally passed selected replacement, bold,
Undo/Redo, recording/export shortcuts, Reject All/Undo, export/save-on-close and
rejected hyperlink draft handling. Save during that rejection left the source
DOCX byte-identical; Copy draft text reported success (clipboard contents were
not independently read), and explicit Discard restored the committed view.
Those earlier checks are distinguished from the final build 3 checks above.

The verified package ZIP SHA-256 is
`590c27a2bd1fd7014611c175eca5115591588b75a8fce0f0b1a6b3abd372bbda`;
its normalized native engine SHA-256 is
`51d67f14969c1033f0745c9ccd161ff6307a4e18a2d1535083223bed5d430365`.
The preview uses its own bundle and recovery store. The normal user workspace
hash remained unchanged. `release/validation/tracking-recording` retains source
snapshots, logs, fixtures and scoped verification reports. No normal release
promotion, Microsoft Word differential qualification, parity-family acceptance
or novel AI acceptance occurred. The full goal remains active.

## In-document revision display — 0.61 build 1, 2026-09-20

Previous goal turn: progress, native 0.60 recording checks completed. This turn
adds source-based Original/Current/All markup views inside the document workspace,
change bars and block navigation, reviewer filtering and individual Accept/Reject.
The view uses a transient package copy and never commits its display projection.
No Word/Excel parity family or novel AI feature is accepted; the goal remains
active and novel AI remains 0/20.

- **312 Rust tests passed.** New coverage verifies exact current/original text,
  previous formatting, paragraph-join anchors, row lifetimes, protected viewing
  with disabled mutation actions, explicit unsupported projections, unchanged
  source/revision/history and byte-identical exports after display requests.
- **263 Flutter tests passed; two optional AI checks skipped.** The real-engine
  workspace test exercises all display choices, review/Undo/Redo, narrow layout,
  return to editing, file identity and replacing the workspace. An additional
  navigation test reveals the 90th paragraph, checks protected review controls
  and verifies failed Original projection retains the prior displayed mode.
  The first workspace test run had a test-harness guarded-tap/pump overlap; the
  harness now awaits the tap before driving the engine. Static analysis is clean.
- The independent Python reader verifies source revisions, projected paragraph
  text/formatting and merged source anchors. The unchanged exported DOCX passes
  Microsoft365 Open XML SDK validation with zero errors.
- Native isolated **0.61.0 build 1** shows marked insertion/deletion text,
  three original paragraphs with 125000 and an unbold title, and five current
  paragraphs with 141000, bold title, Friday sentence and Confirmed. Switching
  display modes left the recovery file byte-identical and revision 2 unchanged.
- Native reviewer filtering shows Morgan's two changes; Next selects the text
  insertion. Reject removes Confirmed; Command-Z restores it, Command-Shift-Z
  removes it again, and another Undo restores all seven changes. Returning to
  all authors and accepting the title-format revision reduces the count to six;
  Undo restores seven. The page and revision pane refresh together.
- Native Save As from the review surface produces `review-view-native.docx`,
  byte-identical to the input after undone review actions (SHA-256
  `5d802710de46bd92843f746e4601c82d0500bd667a4b81b46ec165478005a27c`).
  Returning to editing restores the five paragraphs. Quit prompts to save the
  synthetic unbound workbook; Save writes its own XLSX, then the process exits.
  Restart retains Morgan, revision 8, text and working-file binding. Post-restart
  Save and reopening in-document review retain all seven revisions. Recovery
  before quit and after restart is byte-identical (SHA-256
  `e1f5dedea21e8a74b4655eb85174be10ba2049102f52adb42e31ef77cc2292a1`).

The package ZIP SHA-256 is
`f83736d6d31c6b73b79ea3d9ec15697453cd11f020f6ba717328de59b4710c1f`;
normalized engine SHA-256 is
`d58bd7c66926004b2eee7e107dbac55cbb5d0f69468c6368c28d1e2c8048dae9`.
The native saved DOCX also passes schema validation. The preview remains open
in All markup with seven revisions. The normal release was not promoted and
its workspace was unchanged. Evidence is retained in
`release/validation/review-display` and `work/review-display-*`.

See docs/REVIEW_DISPLAY.md for remaining editable-markup, formatting/table
rendering, per-character selection, story/page, accessibility, performance and
actual Microsoft Word comparison requirements. Current/Original rejects
unsupported revision structures explicitly; the last valid view remains visible.
Review display preferences are transient, and this turn did not exercise the AI
connection or accept any novel AI feature.

## Paired named-move review — 0.62 build 1, 2026-09-20

Previous goal turn: progress on in-document review. This turn adds linked source
and destination review for imported named moves. W13 and full Word/Excel parity
remain incomplete; no parity family is accepted and novel AI remains 0/20.
The user's unlocked desktop also allowed a fresh visual check of the earlier
0.61 imported document: Morgan is the selected author, recording and formatting
tracking are enabled, and all seven revisions remain. No settings were changed.

- **318 Rust tests passed**, including six move scenarios: either-side paired
  review, multiple members/nested deletions, whole-paragraph marks at story end
  and in cells, note-story isolation, hidden-marker retention during ordinary
  editing/export, malformed and text-box/cross-story rejection, atomic failure,
  view projections and history/recovery.
- **264 Flutter tests passed, with two optional AI checks skipped**, against the
  final engine. The new real-engine workspace test covers linked navigation in
  both review surfaces, source-only filtering, paired acceptance/rejection,
  Current/Original, native-sized layout, Undo and file recovery. Analysis is
  clean. Concurrent Flutter dependency setup initially collided; the final
  analysis/test runs used sequential setup and passed.
- The independent ZIP/XML reader checks exact resulting paragraph contents,
  source formatting, unrelated authors and package payload, nested deletion
  retention, cell-end paragraphs and retained move boundaries. Sixteen engine
  fixtures and the final widget-exported DOCX pass Microsoft365 Open XML SDK
  validation with zero errors.
- Isolated native **0.62.0 build 1** imported the sample, showed both move sides
  in All markup, navigated to the destination, and displayed Current/Original
  on their correct sides with formatting. View switching left recovery bytes
  unchanged (SHA-256
  `9e0d1066e217f98d3e8339b83ebd626cb08d856e9b65bb6780ae46d586521599`).
- Rejecting the destination resolved both move records while retaining Morgan's
  unrelated insertion. Native Save As wrote the reviewed DOCX. Command-Z and
  Command-Shift-Z restored/resolved both sides. A source-only type filter showed
  the paired-action notice; linked navigation cleared the filter and selected
  the destination. Accept all shown with just the source visible resolved both
  sides and retained Morgan's insertion.
- Undo plus native Save restored a DOCX byte-identical to the input. Quit was
  confirmed by process absence. Relaunch retained revision 7, all three source
  revisions and the working-file binding. Save after restart kept recovery
  byte-identical (SHA-256
  `8dbe8a5123e95ab9332002ce00a1575d6ce4812a465fb425d70f67f7df642113`).
  Independent inspection and SDK validation pass for all three native DOCX
  artifacts. The preview is left open in All markup with linked controls.

The ZIP SHA-256 is
`0629f5eacb74887a6c83571ee0b89d355cf26e413ca60c6ddbb404cd222aae2c`;
the normalized engine SHA-256 is
`58ac211c165eaea1a963e50efc8e2042b030f6d4fb63a97edc6a4c7e637d569b`.
Packaging matched 290 Dart/Swift/entitlement files and six template assets to
the build inputs. The normal release was not promoted and its workspace remains
unchanged. Evidence is retained in `release/validation/tracked-moves`.

Whole-paragraph and cell moves passed engine and independent file checks but
were not manually exercised in the native preview. Move authoring, custom XML,
column/displaced/text-box/cross-story resolution, complete markup layout,
performance/accessibility and Microsoft Word differential checks remain open;
see docs/TRACKED_MOVES.md. AI connection and generation were not exercised.

## 0.63 section page setup — development qualification

Layout > Page setup adds paper presets/custom dimensions, orientation, margins,
gutter, header/footer distances, section start, measurement units and selected
section/whole-document scope. Only changed settings apply. The continuous editor
uses Section 1 paper width and margins, with horizontal scrolling for wide paper.
Genuine pagination, per-section on-screen layout, columns, header/footer editing,
printing and differential Microsoft Word comparison remain incomplete.
See `docs/PAGE_SETUP.md`.

- **326 Rust tests pass**. Eight page-setup scenarios cover source selection,
  namespace and extension preservation, default/unit geometry, invalid/stale
  atomic behavior, continuous inheritance, boundary splitting/removal guards,
  recorded section formatting and header-link retention, protection/pending
  changes, history, recovery, pending ordinary text and live workbook bindings.
- **270 Flutter tests pass; two optional AI-provider checks skipped**. Six new
  page-setup tests cover dialog units/presets/scope/section selection, validation,
  Cancel, pending-change guards, physical width/asymmetric margins/scrolling,
  true text-offset section selection, real Enter behavior, Apply/Undo/Redo,
  narrow layouts, export and saved recovery. Static analysis is clean.
- Independent XML reading and the Microsoft Open XML SDK 3.5.1 Microsoft365
  validator pass on **14 engine DOCX artifacts and one application-produced
  DOCX**. They confirm section geometry, unchanged headers, reviewed formatting,
  text and live data. They do not run Microsoft Word.

The isolated **0.63.0 build 1** preview passes native Cancel, selected-section
Apply, edit-mode keyboard Undo/Redo, ordinary typing, Enter at a section-ending
paragraph, second-section selection, Save As, recording as Morgan, Accept and
Reject, toolbar Undo, and quit/restart. Six native DOCX artifacts also pass the
independent reader and Microsoft365 schema checks. At revision 20 the recovered
workspace is byte-identical to the saved pre-restart workspace; the normal user
workspace hash is unchanged. The preview is left open at Page setup.

**Remaining native issues:** Command-Z after Save in review mode did not change
the revision; toolbar Undo did. The native Edit menu showed Undo/Redo disabled.
Review comparisons for section geometry still display raw twip values and need
readable measurement units. These are not accepted workflows. Genuine pagination
and differential Microsoft Word validation remain open.

The ZIP is 142,731,476 bytes, SHA-256
`d53984f257bc918fb5c277fefeb13588c44f41a0a6c9814a8359249913f1404b`.
Strict/deep signatures pass after extraction; 291 Dart/Swift source files and
six template assets match staging, and the packaged engine matches the release
binary after normalizing signatures. Evidence is in
`release/validation/page-setup`. The normal release was not promoted. No parity
family is accepted as complete; the novel-AI milestone remains 0/20.

## 0.64 document review history — scoped native qualification

328 Rust tests and 272 Flutter tests pass; two optional AI-provider checks are
skipped. Static analysis is clean. The new regression first reproduced the
review focus failure and then passed after explicit workspace focus restoration.
Both native-channel and keyboard field-history paths pass across assistant,
search, dialog, formula, inline cell, document and grid contexts. Extracted Swift
history/File menus and 47 keyboard adapter checks pass without posting desktop
events.

The isolated **0.64.0 build 1** package passes imported-author setup (Morgan),
readable Gutter 0 in → 0.25 in review, Reject and Save As, Command-Z immediately
after Save (revision 4), keyboard Redo, actual Edit-menu Undo/Redo and restoration
of the original pending change. Assistant draft history, exhausted-draft no-op,
rename-dialog Undo/Redo/Cancel and formula draft Undo/Redo retain workspace state.
Committing B4=4321 recalculates D4=2471 and totals B9=21921/D9=7171; grid Undo
restores the exact original workspace content, and Redo retains the edit.
DOCX and XLSX saves, clean quit, process absence and restart pass. Revision 11
recovers byte-for-byte, including author and pending formatting; the normal user
workspace hash is unchanged. The preview is left open in document review.

An independent XML/relationship reader verifies the retained history snapshots,
tracked gutter removal/restoration, source text/header retention, cell values and
formula caches. Microsoft Open XML SDK 3.5.1 Microsoft365 validation reports no
errors in both saved DOCX files and the XLSX file. Microsoft Word/Excel have not
been run against these artifacts.

**Remaining native boundaries:** Edit-menu Undo/Redo in the AppKit Save filename
field work, but automated Command-Z/Command-Shift-Z sent to that panel did not
change its text. Cause and physical-keyboard behavior remain unverified. One
batched automated prompt typing-plus-Undo left the final character as a separate
undo step; another Undo cleared it without changing the workspace. Settled draft
history passes. Broader IME, accessibility and input-timing coverage remain open.

The ZIP is 142,747,003 bytes, SHA-256
`67e6e05a7b9448c45cee54fa33394d481e430543289f9548a2a14e648f422668`.
Strict/deep signatures pass after extraction; 292 Dart/Swift source files and six
template assets match staging, and the packaged engine matches the release
binary after normalized signatures. Evidence is in
`release/validation/review-history`. The normal release is not promoted. The goal
remains active; no parity family is complete and the novel-AI milestone is 0/20.


## 0.65 section breaks — development qualification

Layout > Section breaks adds Next page, Continuous, Even page and Odd page at
the insertion point. Remove explains the following-section formatting result,
retains inherited headers/footers and joins adjacent paragraphs. Unsupported
recorded section-break edits remain disabled; the user must explicitly turn
recording off. Full pagination and differential Microsoft Word validation remain
open. See `docs/SECTION_BREAKS.md`.

336 Rust tests and 275 Flutter tests pass; two optional AI-provider checks are
skipped. Static analysis is clean. Eight engine scenarios and three new Flutter
tests cover section breaks, including the real release engine. Independent XML checks and
Microsoft Open XML SDK 3.5.1 Microsoft365 validation pass for eight engine DOCX
artifacts plus one UI-produced DOCX. These verify file structure and retained
content; Microsoft Word was not executed. The isolated **0.65.0 build 1** preview passes native Cancel, Continuous
insertion after an emoji, preservation of bold text/hyperlink, automatic Section
2 selection, selected-section left-margin editing, Save As, removal with the
following section's margin, keyboard Undo/Redo and immediate post-Save Undo.
At revision 7 the app was cleanly quit, its process confirmed absent, and the
workspace recovered byte-for-byte after restart. The normal user workspace hash
is unchanged. The preview is left open on Layout.

Independent inspection verifies exact Undo documents, bookmark ranges, section
geometry, original header/footer parts, hyperlinks and text in the three native
saved DOCX files. All three plus the source fixture pass Microsoft365 schema
validation. This is a scoped section workflow; Microsoft Word was not run.
Native testing used Continuous; all four break types have engine/file checks.
AI connection and generation were not exercised in this isolated preview.

The ZIP is 142,784,429 bytes, SHA-256
`afa63804838f37fa8607b19a3e8b96a534699e4b178efa912705887a7d8e81b9`.
Strict/deep signatures pass after extraction; 293 Dart/Swift source files and six
template assets match staging, and the packaged engine matches the release
binary after normalized signatures. Evidence is archived in
`release/validation/section-breaks`. The normal release is not promoted. No parity
family is accepted as complete; the novel-AI milestone remains 0/20.

## 0.66 headers and footers — development qualification

`docs/HEADERS_FOOTERS.md` describes supported section/type-specific rich drafts,
linking and separate first/even-page content. **342 Rust tests and 280 Flutter
tests pass**, with two optional AI checks skipped; analysis is clean. Six focused
engine tests and five editor tests cover shared relationships, inheritance,
copying bookmark/field references, numbering, stale/protected/recording guards,
body drafts/live data/annotations, draft and workspace history, error recovery,
smaller-window controls, export and reopen.

Independent XML checks and the Microsoft Open XML SDK Microsoft365 validator
pass on six engine artifacts plus the actual editor-workflow DOCX. These checks
inspect content and schema; Microsoft Word was not executed. The isolated **0.66.0 build 1** candidate passed native checks:

- Draft keyboard Undo/Redo and Cancel leave revision 1 byte-identical.
- Editing Section 2's linked header updates Sections 1 and 2 at revision 2;
  Section 3's independent header remains unchanged.
- Unlinking, adding text and enabling first/odd/even settings commit revision 3.
  Save As produces separate shared-header and unlinked-header artifacts.
- A first-page footer and even-page header commit revisions 4 and 5. Ordinary
  keyboard Undo/Redo restores exact story packages at revisions 6 and 7.
- Save As, post-Save Command-Z and Command-Shift-Z restore the prior/final story
  packages at revisions 8 and 9. Save retains the final DOCX.
- Clean quit is confirmed by process absence. Restart restores revision 9
  byte-for-byte; the independent header and even-page content are visible again.

The native source plus three exported DOCX files pass independent content checks
and Microsoft365 schema validation. Header/footer parts, section linkage,
first/even settings, renamed copied bookmarks, hyperlinks and unchanged body text
are inspected independently. The final recovery SHA-256 is
`6828e030744be570fee9bece657459c949332b3111c06dfb7d8c7543f5252ce2`.

Native Command-Left moved to the start of the header line; two automated
Command-Up attempts did not reposition the header caret. Broader navigation and
physical keyboard qualification remain open. No pagination/Word-layout claim
follows from these file-content checks.

The package ZIP SHA-256 is
`086bfcffc948f1c01ced1d844dd393e14c0589afd6ab6d62d1a7ac7a7503caea`.
All 294 staged Dart/Swift source files and six template assets matched; the
packaged engine matches the current release binary after signature normalization.
Strict/deep signature verification passes. This is locally signed, not notarized.
Evidence is archived in `release/validation/headers-footers`. The normal user
release has not been replaced and its workspace hash is unchanged. Full W06 acceptance, physical page display, recorded
header/footer changes, page-number authoring and broad Word differential
qualification remain open. No entire parity family is accepted; novel AI is 0/20.

## 0.67 page fields and numbering — isolated native qualification

`docs/PAGE_NUMBERS.md` describes atomic inline PAGE/NUMPAGES/SECTIONPAGES/SECTION
fields and section-owned number format, continue/restart and chapter controls.
**348 Rust tests and 285 Flutter tests pass**, with two optional AI checks
skipped; static analysis is clean. Six new engine tests cover supported simple/complex fields,
split instructions, source preservation, unknown/nested/malformed fields,
authored kinds, invalid/forged values, custom numbering attributes, comments,
bookmarks, table fields, linked footer content with independent section numbering,
stale requests, Undo/Redo and recovery.

Independent XML and Microsoft365 schema checks pass on five engine artifacts
plus the actual editor-workflow DOCX.
The checks verify field codes, cached values, atomic bookmark placement,
comments/table fields, unchanged body/header content and section attributes.
The actual editor workflow passes against the release engine, covering both
field menus, Roman format, restart, chapter controls, deferred Apply, Cancel,
Undo/Redo, export and recovery. A stale Quill focus listener after read-only
transitions was exposed by this test and fixed.

The isolated **0.67.0 build 1** candidate passes these native checks:

- Open the three-section DOCX at revision 1. Change numbering in a nested
  draft, choose Done, then outer Cancel; recovery remains byte-identical.
- Unlink Section 2's default footer, retain its existing text, and use the
  toolbar to insert PAGE and NUMPAGES with intervening ordinary text.
- Set lowercase Roman format, restart at 5, Heading 2 chapter numbering and
  a colon separator. Outer Apply commits fields and settings at revision 2.
- Keyboard Undo/Redo reaches revisions 3/4 and exactly restores the prior/final
  document and Office package. Save As creates `numbers-final.docx`.
- Keyboard Undo/Redo immediately after Save reaches revisions 5/6, again
  restoring the exact packages. Save, quit and process absence are verified.
- Restart recovers revision 6 byte-for-byte. Both field tokens, the unlinked
  footer and all numbering settings are visibly present. Inspection dialogs
  are closed without changes, leaving the saved document open.

Independent native-file checks and the Microsoft365 schema validator pass on
the source and final DOCX. They confirm field instructions, unresolved caches
and dirty flags, section scope, unchanged body/header/original-footer content,
exact history and restart. Recovery SHA-256:
`c58cf4091ee5d73297395d20ba600b0f9ec14cd7d0b810088aa5e0a57fa7808a`.

The ZIP SHA-256 is
`bcd4492ad0e86c401a843f77c8245e3282911fdcebf10e2f0c02b2dd11f82c11`.
All 296 staged Dart/Swift/entitlement source files and six template assets
match; the packaged engine matches the release engine after signature
normalization. Strict/deep signature verification passes; this local candidate
is not notarized. Evidence is in `release/validation/page-numbers`.
The prior preview was saved and quit before launch, with process counts of zero
before and one after each launch. The normal release and user workspace remain
unchanged. Physical page calculation and direct Microsoft Word comparison
remain open; no parity family or novel AI feature is accepted by these scoped
checks. The full goal remains active.

## 0.68 physical page preview — development qualification

The immutable preview operation, page-flow renderer and native preview dialog
are implemented; see `docs/PAGINATION.md`. **351 Rust tests pass**. Engine
checks cover source-resolved linked stories, section numbering, pending edits,
read-only workspace/history behavior, manual break types and paragraph flow
flags. The dedicated source DOCX passes Microsoft365 schema validation.

Focused Flutter tests exercise line-fragment text and bounds, section geometry,
odd-page blanks, compatible continuous sections, field values across fragments,
merged table rows, keep-lines/page-break-before, manual page breaks, first/even
stories, body exclusion, locked field caches and field-format switches. The
application workflow covers menu entry, preview navigation, native minimum
window geometry, pending edits, cache preservation, Undo/Redo and recovery.
The full suite passes **295 Flutter tests**, with two optional AI-provider
checks skipped; static analysis is clean. Independent XML inspection and
Microsoft365 schema validation pass for the fixture and the application-edited
DOCX. The pending text prefix is the only body change; section geometry,
header/footer bytes, field instructions and saved cache values remain intact.

The isolated **0.68.0 build 1** candidate passes native qualification on the
prepared seven-page fixture. Every page was inspected: paragraphs 00–06, 07–13,
14–20, 21–27 and 28–34 occupy pages 1–5, paragraph 35 and the section ending
occupy page 6, and the landscape section occupies physical page 7. Headers
repeat, footers show the actual count, and the restarted final footer reads
`Page v of 7`. No blank page is required by this fixture; insertion to satisfy
an odd-page start is covered separately by the layout test.

Previous/next, 200% zoom, vertical scrolling and Fit page pass. Returning to the
document retains revision 1 and identical recovery bytes. Save As creates a
DOCX byte-identical to its source; independent inspection confirms the original
field caches of 99 are unchanged. Normal quit, verified process absence,
reopen and the restored seven-page preview pass. All four recovery snapshots
(baseline, after preview, before restart, after restart) share SHA-256
`631f8619b2c16043f5381e78dcd0b1c6f387c036ddb7124e7c0a2641a047dd97`.
The native source/final DOCX also pass Microsoft365 schema validation.

ZIP SHA-256:
`cd7aae262ef676fa8a093c863fbd33bbc42011a90d4f075e7ee95f723e1d42f9`.
All 298 staged Dart/Swift/entitlement source files and six template assets
match the tested source. The packaged engine matches the release engine after
signature normalization; strict/deep signature checks pass. This local
candidate is not notarized or promoted to the normal release. The normal user
workspace is unchanged. Process counts are zero before each launch and one
after; only the current preview is left running. Evidence is archived in
`release/validation/pagination`.

These checks do not execute Microsoft Word or prove matching Word pagination.
No complete parity family or additional AI feature is accepted; the full goal
stays active.

## 0.69 section columns — development qualification

Column settings and preview flow are implemented; see docs/COLUMNS.md.
**355 Rust tests pass**, including four new workflows for column definitions,
source preservation, invalid/stale requests, whole-document scope, recorded
formatting Accept/Reject, pending text, Undo and recovery. The targeted Flutter
layout/application run passes 17 tests: seven new column checks, one real-engine
column application workflow, and the nine existing pagination checks.
Independent OOXML inspection and Microsoft365 schema validation pass for the
source and custom-left/RTL application export, with unchanged unrelated parts,
other section settings and stored page-field caches. The complete Flutter suite
passes **303 tests**, with two optional AI-provider checks skipped; static
analysis is clean. The recorded, accepted, rejected and workspace-recovered
engine DOCX also pass the Microsoft365 validator, for six schema-checked
artifacts including the source and application export.

The isolated **0.69.0 build 1** package is prepared. ZIP SHA-256:
`c8efe96adeb22955e0ad40646987a12dab72662a2db364981d2fc00bd630f394`.
All 299 staged Dart/Swift/entitlement source files and six template assets match;
the packaged native engine matches the release build after signature
normalization. Strict/deep signature checks pass. It is a local ad-hoc candidate,
not notarized or promoted to the normal release.

Native qualification is pending because the Mac locked before the current
saved 0.68 preview could be inspected and quit. The 0.69 candidate has not been
launched; exactly one older preview remains running. Normal user recovery is
unchanged. Automated evidence and source hashes are in
`release/validation/columns-development`. Required next desktop checks are
Cancel, custom-left/RTL Apply, visible column flow, keyboard/post-Save Undo and
Redo, DOCX export and byte-identical restart, following the zero/one process
guard for every launch. Last-page balancing and full Word differential layout
qualification remain open. No complete parity family or novel AI feature is
accepted; the goal remains active.
## 0.70 continuous-section column balancing — automated qualification

The preview balances only the ending section's content on its final physical
page before a compatible continuous section, retaining prior pages and earlier
sections. Paragraphs resume at source offsets and tables at complete row groups;
manual column breaks and existing keep/widow constraints remain effective.
Word's related settings part supplies the noColumnBalance compatibility flag.
See docs/COLUMN_BALANCING.md for scope and remaining layout limitations.

All **356 Rust tests and 316 Flutter tests pass**, with two optional AI-provider
skips. Static analysis is clean. Twelve new layout tests and the real-engine
application workflow cover balancing/opt-out, unequal widths and RTL Unicode,
page fields, keep constraints, manual breaks, merged rows, shared-band prefixes,
unchanged preview/history, byte-identical export and recovery. The fixture pair
differs only in the intended compatibility setting. Independent ZIP/XML checks
and Microsoft365 schema validation pass for all four source/export DOCX files.
The two real-font headless page renders were visually inspected; these are not
native screenshots. Logs are in `../../work/balance-*.log`.

The desktop became accessible after packaging. The saved 0.68 preview was
inspected and quit normally; a zero-process check preceded launch of the signed
0.70.0 build 1. Its 299 staged application files, six templates and normalized
native engine match the tested source. The native app displays both balancing
variants correctly, retains exact recovery bytes at revisions 1/2, and saves
both DOCX copies byte-for-byte. Reopening the balanced export reaches revision
3; normal quit, zero-process confirmation, restart and restored preview retain
identical recovery bytes. Both native exports pass Microsoft365 schema checks.
The package SHA-256 is
`0c3ae85f3af4c7c3e1bd11d020bf898c394972e89083caa1a6ccb72781d338fc`.
Native 0.69 Columns dialog Apply/Cancel and keyboard-history checks remain
pending. Exactly one 0.70 preview remains running; normal user recovery and the
normal release are unchanged. Evidence is archived in
`release/validation/column-balancing-development`.
No full parity family or additional AI feature is accepted; the goal remains
active. This goal turn made independent implementation and verification progress.
## 0.70 native qualification of the Columns controls

Previous goal turn: progress. Column balancing implementation, automated tests,
packaging, native previews, source-identical exports and restart verification
completed. This turn closes the remaining native control workflow from 0.69
using the already-running 0.70 preview; no additional candidate was launched.

Import establishes revision 4. Three-column draft/Cancel leaves exact recovery
bytes unchanged. Left preset + separator + right-to-left Apply produces revision
5. Keyboard Undo/Redo reaches 6/7; immediate post-Save As Undo/Redo reaches 8/9.
Both Undo states exactly match the original document and Office package;
both Redo states match the applied state. Save, normal quit, zero-process
confirmation and restart leave revision-9 recovery byte-identical, with the
custom settings visibly recovered. SHA-256:
`26c1ca577c5d3224d7d2e7d5ff55c10882ff47412c0f36a9a8977998f5012c16`.

All seven preview pages were inspected. Paragraphs flow through the wide right
then narrow left column on pages 1–5; page 6 is deliberately blank for the
odd-page section; page 7 is landscape with footer `Page v of 7`. The native
DOCX passes the independent section/package verifier and Microsoft365 schema
validation. Evidence and source hashes are in `release/validation/columns-native`.

Application/engine source still matches the qualified 0.70 package and its
356 Rust / 316 Flutter test run (two optional AI checks skipped). This turn
changes only the verification helper and qualification records. Exactly one
saved preview remains open; the normal user workspace and release are unchanged.
Microsoft Word was not executed. Full Office parity and 20 novel AI features
remain incomplete; no family is accepted and the goal stays active.

## 0.71 development — repeated table headings — 2026-09-20

Table properties now exposes the repeat-heading row setting, including mixed
selection and draft behavior. Page preview repeats consecutive heading rows
without adding source rows or modifying saved field caches. Merged headings,
per-page fields, orphan prevention and column balancing have focused coverage.
See `docs/TABLE_HEADERS.md` for the remaining manual-break, merged-boundary,
oversized-header, tracked-format and direct Word comparison requirements.

Final source and release-engine validation:

- 360 Rust tests passed. Four new tests cover flags, prefix aliases, retained
  source data, invalid atomic edits, legacy hydration, native rich headings,
  page fields, save/reopen and Undo/Redo. Header-only changes also retain
  unchanged cell merge properties instead of normalizing their source syntax.
- 328 Flutter tests passed; two optional provider checks skipped. New layout,
  dialog and real-bridge workspace tests cover headings, mixed state, Cancel,
  Apply, history, export and recovery. Static analysis is clean.
- Four DOCX artifacts pass an independent XML verifier and Microsoft365 schema
  validation. Early schema validation rejected numeric flag values; the final
  exporter writes the required `on`/`off` values and all checks were rerun.
- Three real-font, headless renderings were inspected: rows 00–14, 15–29 and
  30–44, with the repeated merged title and correct per-page fields. These
  renderings are automated app evidence, not a native desktop inspection.
- The prepared package has strict/deep ad-hoc signature checks, 299 matching
  staged application files, six matching template assets and a matching engine.
  ZIP SHA-256: `7d0065b33667aceddea3dba91dd8032db616144340cf6caad18c45d020eed5ba`.

The only running desktop preview remains the saved 0.70 instance. The 0.71
candidate has not been launched; native Cancel/Apply, keyboard history, page
preview, export and restart qualification remain. The normal user workspace
hash is unchanged. Evidence: `release/validation/table-headers-development`.
No parity family or additional AI feature is accepted by this development pass.

### 0.71 native table-heading qualification — 2026-09-20

The same prepared 0.71 package passes native single-row Cancel/Apply,
mixed-selection display and cancellation, multi-row Apply with one-step Undo,
and keyboard Undo/Redo immediately after Apply and native Save. Both enabled
and disabled states were reviewed across all three preview pages. Derived
headings and per-page fields appear only in enabled continuations. Source rows,
cached field values and saved workspace bytes remain unchanged by preview.

Eighteen saved native checkpoints verify exact document/source-package/sheet
states, draft cancellation, history, and byte-identical restart at revision 6
(SHA-256 `4dee204e23383a18a723557fb944237c58ecf67d26e86af4e373e877ce4a3089`).
The visible checkbox is restored disabled after restart. The enabled DOCX
matches its original source; the reopened export matches the saved disabled
DOCX. Three native exports pass Microsoft365 schema validation. Later multi-row
Apply and Undo leave the final saved preview at revision 50.

An automation incident after the restart is recorded: the export picker
unexpectedly dismissed and filename text entered the disposable document.
Twenty-one native Undo actions restored every saved field except revision;
the verified export followed only after that restoration. The tests do not
attribute this observed picker/input issue to the application or the control
service without further evidence.

Process guards verified zero before replacing 0.70 and before restarting 0.71,
and exactly one afterward. The normal user workspace remains unchanged. Native
artifacts, checkpoint checks, schema results, source identity and these notes
are retained in `release/validation/table-headers-native`. No application or
engine code changed during this native qualification, and no parity family is
accepted by it.

### Table row-height source development before 0.72 — 2026-09-20

The editor and Page preview share content-driven automatic/minimum sizing and
fixed exact sizing. Merged cells grow only flexible rows. Preview clips nested
text, inline painting, fills and borders to every containing cell, retains the
source text, and excludes clipped lines/field boxes from its visible projection.

All 337 Flutter tests pass with two optional provider skips. Eight new layout
checks and one real-core/UI workflow cover tiny/exact/minimum/automatic heights,
merged cells, overflowing vertical alignment, nested clipping, repeated headers,
pagination, draft Cancel/Apply, preview immutability, Undo/Redo, DOCX export,
reimport and recovery. The final workspace fixture uses explicit OOXML line
breaks; that targeted test passes again after the full suite. Analysis is clean.
The rendered clipping image was visually inspected and the following row's
interior was checked for overflow at every pixel. Rust implementation did not
change and its suite was not rerun in this follow-up.

Independent XML comparison finds only the first row's height change (automatic
to 480 twips exact), retaining all seven clipped source lines and unrelated
package parts. Recovery export is byte-identical. Source, edited and recovered
DOCX files pass Microsoft365 schema checks. Evidence and source are retained in
`release/validation/table-row-heights-development`.

At this source-development checkpoint the changes had not been packaged or
qualified in the native preview, and 0.71 was the only running instance. Direct Word
comparison, row splitting, keep-together rules and complete cell geometry remain
open; see `docs/TABLE_ROW_HEIGHTS.md`. No whole parity family is accepted and
additional AI-feature acceptance remains 0/20.

### 0.72 native row-height qualification — 2026-09-20

The 0.72 build and extracted ZIP pass strict/deep ad-hoc signature checks. All
300 application-source files match both the build staging directory and the
tested source archive; six template assets match their packaged copies. The
packaged Rust library matches the rebuilt 0.72 library after signature
normalization. Only version metadata changed after the automated layout tests.
The normal release was not promoted and the package is not notarized.

Native checks pass on a two-row imported DOCX: automatic sizing, exact 24-point
Apply, outer Cancel, immediate and post-Save As keyboard Undo/Redo, minimum
24-point Apply and Undo, previewing all three modes, working Save and export.
Seventeen saved checkpoints verify only the intended row-height changes,
unchanged other document/source-package/workbook data, and byte-identical
preview/cancellation/restart. Final revision 8 has recovery SHA-256
`cb4c828049d225f0e35827721a6e84d19251a34ecc0dca82c242b3338662b2b0`.
The working Save and post-restart export match the original exact-height
export. Source and three native output files pass Microsoft365 schema checks.

The saved 0.71 preview was quit normally before launching 0.72, with a verified
zero/one process transition. The same guard was used for the restart. Exactly
one 0.72 preview remains open; both the normal user workspace and the closed
0.71 workspace are unchanged. A path-picker interaction required dismissing a
second path-entry sheet; no accidental document input occurred. Evidence is in
`release/validation/table-row-heights-native`.

This native fixture does not qualify nested clipping, vertically merged rows,
overflowing center/bottom alignment or repeating exact-height headings; their
coverage remains automated. Its line-break paragraph is preserved in the cell
editor, not yet editable as rich text. Direct Word differential testing and
the broader table-pagination requirements remain open. No complete parity
family or additional AI feature is accepted.


## Table row splitting — source development after 0.72, 2026-09-20

The new Allow row to break across pages setting reaches the table draft,
committed document, Word export and recovery. Ordinary automatic/minimum-height
rows continue each cell at its own source position; different column widths
reshape remaining paragraphs. See docs/TABLE_ROW_SPLITTING.md for supported
style inheritance, nested tables, page fields, balancing and remaining limits.

All **363 Rust tests** and **348 Flutter tests** pass, with two optional provider
skips; analysis is clean. Eleven new Flutter layout/render/dialog/real-engine
checks and three new Rust tests exercise the change. Independent XML checks and
Microsoft365 schema validation pass for three application artifacts. Four older
engine preservation artifacts retain three known shading-attribute schema errors
each from the original fixture; no clean-schema claim is made for those files.
The real-font canvas was inspected; packaged-native and Word comparison remain
pending. One existing 0.72 preview remains running (PID 87028), unchanged.

Evidence is under ../../work/table-row-flow-*; source hashes and final check
counts are recorded in ../../work/table-row-flow-source-status.json. No parity
family or new AI feature is accepted.


## Table row splitting — 0.73 native qualification, 2026-09-20

The 0.73 package matches the tested implementation (301 app source files,
107 Rust files, six templates); only version metadata changed after the
363-Rust/348-Flutter run. Signature and normalized engine comparisons pass.
Nineteen native checkpoints pass for import, both row page-break settings,
Cancel/Apply, two-page continuation, ordinary and immediate post-Save As keyboard
Undo/Redo, exports, working Save and byte-identical restart recovery. Source and
four native outputs pass Microsoft365 schema validation. Unrelated workspace
state, the previous preview workspace and the normal user workspace are unchanged.

The existing 0.72 process was quit normally before launching a single 0.73
preview; both replacement and restart passed explicit zero/one guards. The
final saved document is row-flow.docx, revision 8. Evidence is archived under
release/validation/table-row-flow-native. The package is ad-hoc signed, not
notarized or promoted to the normal release path. Complex native cases, the
remaining implementation limits and direct Word comparison are described in
docs/TABLE_ROW_SPLITTING.md. No parity family or novel AI feature is accepted.
