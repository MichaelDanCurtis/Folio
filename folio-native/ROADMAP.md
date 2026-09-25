# Folio product roadmap and acceptance contract

Planning revision: 2026-09-21. Implementation baseline: 0.82.0 preview.

## 1. Product destination

Build a desktop-first Word and Excel alternative with Flutter for the interface, Rust for document/workbook engines, and the existing Codex App Server integration for AI. A person should be able to finish a real document and its supporting workbook in Folio, exchange them with Office users, and understand and control every AI change.

The destination remains full Word and Excel parity, including Windows-only capabilities, followed by at least 20 distinct, implemented, verified AI capabilities unavailable in the agreed Microsoft comparison baseline. Milestones below stage delivery; they do not remove difficult requirements from that destination.

**Current evidence:** [README](README.md) and [VALIDATION](VALIDATION.md) record 405 passing Rust tests, 395 passing Flutter tests, two optional AI skips, and focused native verification through cross-workbook Scenario Merge. These are recorded results, not new test runs for this roadmap. [PARITY](PARITY.md) has 46 broad families: 10 desktop, 17 Word, 19 Excel. **Accepted whole families: 0/46. Accepted additional AI features: 0/20.** Counts are inventory counts, not effort estimates or a percentage of product completeness.

The M0 status register is [docs/STATUS.md](docs/STATUS.md) (created 2026-09-25): 391 child requirements across the 46 families, a ranked gap list and a risk register.

The next priority is to close complete workflows and establish actual Office comparisons. Adding another isolated command is useful only when it advances a named acceptance unit.

## 2. What “done” means

### Versioned comparison baseline

At M0, record actual Word and Excel version/build, update channel, OS, locale, fonts, installed extensions, licenses and enabled Copilot capabilities. Use Microsoft 365 desktop on Windows as the breadth reference and macOS as an additional native interaction reference. Linux remains a Folio delivery target. Record platform-specific differences explicitly; an alternative UI may qualify when behavior and results are equivalent.

Freeze that baseline for each milestone so tests are reproducible. Review changes at milestone boundaries and again before the final parity claim. Newly discovered requirements enter a visible change log. Never silently narrow the goal to a selected subset or claim indefinite parity with future Office releases.

### Acceptance states

Use **Specified → Implemented → Automated verified → Native verified → Office compared → Accepted**. A non-Office feature may mark Office comparison “not applicable” with a reason. A blocked requirement retains its last achieved state and names the missing dependency. Preserved-only and partially editable content are capabilities, not acceptance states.

Every acceptance unit has:

- A stable ID, parent parity family, user outcome, exact commands/options, scope and dependencies.
- A reproducible input file and expected content, layout/calculation and error behavior.
- Positive, boundary and failure cases, including keyboard and accessibility paths.
- Applicable Apply/Cancel, Undo/Redo, save/reopen, export/reimport and restart/recovery evidence.
- Exact candidate/source identity, reference Office build, result artifacts and unresolved differences.
- A pass/fail decision. Every child requirement must pass before its parent family is accepted.

Schema validity proves file structure. An independent reader proves another interpretation. Neither proves Word layout, Excel calculation, editable fidelity or actual Office interoperability. A skipped live-provider test cannot accept AI behavior.

### File compatibility contract

Maintain a per-format and per-feature matrix for **open, display, edit, calculate, save, round-trip, print and automation**. Label each intersection Unsupported, Preserved only, Partial or Verified. Never use a single “supported” label for an opaque object.

For each qualified feature, test Office-created → Folio edit → Office reopen/edit → Folio reopen, and Folio-created → Office edit → Folio reopen. Require no unexpected repair dialog, missing relationships, silent content loss, incorrect cached results or unrelated changes. Compare document structure and text, reference calculations and rendered pages; byte equality is required only where preservation or no-op behavior calls for it.

Unsupported content must be identified before a destructive conversion, with a concrete impact preview and a safe copy/read-only path. This is an interim protection, not full compatibility. Include macros, signatures, links, protection, embedded objects and unfamiliar extension parts in the matrix.

## 3. Delivery sequence

Dates follow measured throughput after M0; no invented completion date for a full office suite. Each milestone is a usable release gate. Security, accessibility, compatibility and performance are part of each gate, not deferred cleanup.

| Milestone | User-visible outcome | Exit gate |
| --- | --- | --- |
| M0 — Honest baseline and release discipline | One dependable preview, a finite acceptance inventory, reproducible Office comparison fixtures | All 46 families decomposed into bounded child requirements; exact Office baseline and benchmark machine recorded; evidence linked; one preview process; highest-risk dependencies evaluated |
| M1 — Daily writing and spreadsheet work | Create a report and budget, edit efficiently, proofread, save, recover and exchange the files | Complete daily-workflow scenario below passes natively and in reference Office; local proofing and baseline AI selection → proposal → Apply → Undo work end to end |
| M2 — Professional documents and connected reports | Publish long documents and produce a traceable report from a workbook | Pagination, styles, tables, references, review and document publishing requirements pass their corpus; linked-report scenario passes; unresolved advanced objects stay visibly open |
| M3 — Reliable analytical workbooks | Build and audit a model, analyze data, make charts and produce a printable workbook | Formula catalog has explicit coverage and reference results; dynamic formulas, tables, auditing, pivots and what-if workflows pass; large-file budgets met for the declared corpus |
| M4 — Advanced Office breadth | Use specialist formats, automation, external data, protection and shared editing | Remaining automation/data/object/format/collaboration requirements implemented and compared; architecture decisions validated with working proofs |
| M5 — Full parity release | Replace Word/Excel for the complete agreed inventory on supported Folio platforms | 46/46 families accepted, no unaccepted discovered baseline requirement, distribution and recovery gates pass, current-baseline delta reviewed |
| M6 — Verified AI differentiation | At least 20 additional distinct AI workflows beyond the comparison baseline | Each feature passes live-model, usability, persistence, safety and dated novelty gates; accepted count reaches at least 20 |

M1–M3 may refine existing assistant functionality requested for the baseline product. M6 is the implementation phase for the additional 20 features. Candidate research can happen earlier. Do not count routine AI rewriting, grammar review, formulas or charts as novel features.

### First three delivery packages

1. **Acceptance foundation (M0):** extract current evidence into a concise status register; enumerate child requirements; capture the Office reference and representative files; measure current startup/edit/save behavior; verify the one-preview launch procedure. Produce a ranked gap list with effort ranges and confidence, based on inspection and small technical proofs.
2. **Daily editing closure (M1):** finish the report-and-budget scenario, prioritizing broken or inconsistent typing, selection, keyboard focus, spelling replacement, save lifecycle, Undo and recovery. Compare the saved files in Office. Close named child requirements rather than incrementing versions for every control.
3. **Connected report closure (M1/M2):** qualify the actual Codex request with selected prose and an attached range, source inspection, native A2UI review, stale-source handling, Apply/Undo and persistence. Then close long-document and calculation dependencies identified by those workflows.

## 4. Exact feature expectations

These packages cover every current parity family. Each package must be decomposed further at M0; grouping does not imply all features have equal effort or share an implementation.

### Shared desktop product

| Package / families | Required behavior and acceptance examples | Target |
| --- | --- | --- |
| S01 — Workspace and shell; D01, D02 | Document/workbook tabs and windows; recent/pinned files; templates; native open/save/save-as; file associations; dirty state; autosave/version recovery; missing/renamed file handling. Saving has an unambiguous destination and no silent overwrite of a template. Recover independent windows and unsaved work after interruption. | M1 core; M4 full |
| S02 — Office-like interaction; D01, D03 | Familiar ribbon groups, contextual tabs, quick commands, status bar, document ruler/page canvas, formula bar/name box/sheet tabs, keyboard shortcuts and command search. Cut/copy/paste across Folio and Office retains supported rich content. Context menus separate spelling suggestions with a visible divider. Native dialogs return focus and keyboard history to the correct editor. | M1 |
| S03 — Accessibility and languages; D04 | Every command reachable by keyboard; visible focus; meaningful control names, selection/range announcements and errors; VoiceOver and NVDA task tests; high contrast and 200% UI scale; CJK composition and RTL text. Locale-specific input, dates and formatting are explicit and tested. Audit supported Linux assistive technology before Linux release. | Every milestone; full M5 |
| S04 — Publishing; D05 | Real print preview and system printing, paper/margins/orientation, ranges and scaling, searchable tagged PDF with reading order, links and accessible tables. Printed output matches the qualified page model; no clipped/omitted content. | M2 documents; M3 sheets |
| S05 — Security and shared work; D06, D07 | Password/encryption/protection/signature semantics; trusted automation and external-link handling; file permissions; comments and author identity; sharing/coauthoring, offline edits and conflict resolution. Concurrent or failed saves do not silently lose work. | M4 |
| S06 — Distribution and speed; D08, D09 | macOS first, then Windows and Linux; signed/notarized artifacts where applicable; install/update/rollback/uninstall; migration backups; measured startup, input, memory and cancellation; sleep/lock/display and crash recovery. | Continuous; full M5 |
| S07 — Integration; D10 | Keep Flutter UI, Rust engine and existing local workspace continuity. Route assistant edits through typed application commands and normal history. Define and test Office automation/add-in compatibility separately from Folio's own extension interfaces. | Baseline M1; full M4 |

### Word workstreams

| Package / families | Required behavior and acceptance examples | Target |
| --- | --- | --- |
| W-A — Editing and navigation; W01, W03, W04 | Rich text, font fallback, Unicode/RTL, paragraph spacing/indents/tabs, list definitions/restarts/continuation; literal/wildcard/format/special-code search, scoped replacement, Go To and bookmarks. Preserve selection and annotations through repeated text, emoji, IME, paste and structural edits. | M1–M2 |
| W-B — Styles and views; W02, W17 | Named/linked/base/next styles, direct overrides, themes, style organizer and templates; print/draft/web/outline/read/focus views and accessibility checking. Changing a base style refreshes all dependents without flattening direct formatting. | M2 |
| W-C — Page layout; W05, W06 | Section geometry, columns, continuous/next/odd/even breaks, widow/orphan and keep controls, hyphenation; linked/first/even headers and footers, numbering, watermarks/backgrounds. No duplicated or omitted lines at page/column boundaries; matching page count and intended break locations in the reference corpus. | M2 |
| W-D — Tables and objects; W07, W08, W09 | Nested tables, merges/splits, AutoFit, repeat headers, row flow, sorting and text conversion; editable image crop/wrap/anchors, shapes, text boxes, charts, SmartArt, equations, embedded/linked objects, media and content controls. Objects must render and remain editable; retaining ZIP parts alone fails. | Tables M2; full M4 |
| W-E — References; W10, W11 | Fields and updating, captions/cross-references, TOC/index/table of authorities, footnotes/endnotes, citations and bibliography. Moving a cited figure updates labels/references; changing pagination updates page references; missing targets produce visible errors. | M2 |
| W-F — Proofing; W12 | Continuous local spelling/grammar, custom dictionaries, ignore/learn/undo, language selection, thesaurus and translation; AI grammar on selection or document with tracked proposals. Detailed quality contract below. | English M1; full M4 |
| W-G — Review and production; W13, W14, W15 | Threaded comments, author attribution, text/format/structure tracking, accept/reject, compare/merge; mail merge from workbooks/data sources, labels/envelopes; forms, restricted editing/protection and macros. Review edits in body, tables, headers and notes without losing anchors. | Review M2; full M4 |
| W-H — Format fidelity; W16 | DOCX, DOCM, DOTX, DOTM, DOC, RTF, ODT, HTML and text: explicit per-format matrix and round-trip evidence. Templates retain template semantics; macro formats retain and eventually execute supported automation with correct trust controls. | DOCX every milestone; full M4–M5 |

### Excel workstreams

| Package / families | Required behavior and acceptance examples | Target |
| --- | --- | --- |
| X-A — Grid editing; X01, X02, X03 | Multi-area/range selection, edit/commit/cancel, fill/series/flash-fill-equivalent behavior, Paste Special and external clipboard; Find/Replace, Go To/Special; row/column/sheet structure and movement. Transform formulas, names, comments, rules, charts and links correctly after insertion/deletion/move. | M1; full M3 |
| X-B — Formatting and views; X04, X05 | Fonts/rich runs, themes/styles, borders/merges, number formats, locale/date systems, wrap/rotation/overflow/AutoFit; freeze/split panes, zoom and page layout. Render displayed values consistently in grid, clipboard, filters, charts, linked report and print. | M1; full M3 |
| X-C — Tables and rules; X06, X07 | Structured references/calculated columns/totals, sort/filter/advanced filter/slicers; all validation and conditional formatting rule types, precedence, relative references and visual outputs. Sorting moves complete records; paste/fill/structure updates rule scope correctly; Undo restores it. | M1; full M3 |
| X-D — Calculation; X08, X09 | Enumerated function catalog, error/coercion/precision semantics, locales/date systems; shared/array/dynamic formulas, spill, LET/LAMBDA, names and external references. Reference tests include circular/iterative calculation, volatile functions, spill collisions and structural updates. Unknown/unsupported formulas are explicit, never guessed. | M3; remaining breadth M4 |
| X-E — Auditing and what-if; X10, X14 | Trace precedents/dependents, Evaluate Formula, watches and navigation; Goal Seek, scenarios/merge/reports, one/two-variable data tables, multivariable Solver and Analysis ToolPak. Compare numerical outcomes and non-convergence behavior; cancellation leaves the original workbook unchanged. | M3 |
| X-F — Visualization and analysis; X11, X12 | Editable Office chart types/styles/axes/labels, sparklines and drawings; PivotTables/PivotCharts, refresh, grouping, calculated fields/items, slicers and data models. Source edits/refresh update outputs; saved files remain editable in Excel. | M3; specialist breadth M4 |
| X-G — Data and automation; X13, X16 | Import/connectors, credential handling, refresh/error/cancel, Power Query transformations, relationships, Power Pivot/DAX; VBA, add-ins and Office Scripts behavior/API compatibility. Native alternatives do not automatically establish compatibility with existing user automation. | M4 |
| X-H — Review and protection; X15 | Notes/threaded comments with identity, history, protected cells/sheets/workbooks, sharing/coauthoring and conflict recovery. Protection applies consistently to UI, paste, automation and AI commands. | Local review M1; full M4 |
| X-I — Formats and publishing; X17, X18 | XLSX, XLSM, XLTX, XLTM, XLS, XLSB, CSV and ODS matrix; print areas/titles, page breaks, scaling, headers/footers and PDF. Qualify encodings, delimiters and locale ambiguity for CSV, and charts/rules/formulas/objects for package formats. | XLSX M1; printing M3; full M4 |
| X-J — Scale and scheduling; X19 | Excel address/size limits, dense and sparse workbooks, incremental calculation, progress/cancellation and bounded resource use. Remove or explicitly track existing 30-sheet/100k-populated-cell restrictions; displaying the last cell is not large-workbook acceptance. | M3–M5 |

## 5. Proofing quality contract

Use a pluggable local proofing service behind Flutter so engine choices can be benchmarked and improved without rewriting editor behavior. Evaluate the current engine first. A library's advertised feature list does not establish accuracy.

- A right-click suggestion replaces exactly the intended word, preserves surrounding formatting and punctuation, updates the underline and creates one reversible edit. Works after scrolling, wrapping, selection changes, native dialogs, Unicode text and in document tables/header/footer stories.
- A clear divider separates suggestions and dictionary actions from Cut/Copy/Paste and other commands. Keyboard users can invoke every suggestion; screen readers announce the target and replacement.
- Local checks run without a network connection. Learned words and language preferences persist. Suggestions for an old revision are rejected or recalculated instead of replacing nearby text.
- AI grammar accepts selection, paragraph or full-document scope. Show original/proposed text and an explanation; preserve names, figures, citations and author intent; uncertain substantive changes require explicit review. Support individual and grouped acceptance, cancellation and Undo.
- Establish a held-out, human-reviewed corpus with at least 1,000 spelling errors plus 1,000 clean passages and 500 grammar issues plus 500 clean passages per initially qualified language. Include names, technical vocabulary, contractions, dialect variants and mixed formatting. Keep tuning examples separate.
- Initial release targets: spelling top-three correction recall ≥95%, flag precision ≥98%; local grammar precision ≥95% and recall ≥80% for declared rule categories. Report numerators, denominators and uncertainty per category/language. AI grammar must introduce zero meaning-changing or numeric changes in the protected-fact acceptance corpus. These are proposed targets, not current measurements or guarantees.
- First local underline within 500 ms after typing pauses on the reference document; accepted correction visible within 100 ms at p95. Broad multilingual parity remains open until each baseline language and proofing category has equivalent evidence.

## 6. Baseline AI-human interface

These requirements implement the assistant already requested; they are not automatically additional novel features.

1. **Precise context:** select a word, sentence, paragraph, table, cell/range, chart or multiple sources. Show attached context chips with file/sheet/range, revision and scope. A simple paragraph click can anchor Ask AI without losing the caret or changing text.
2. **Workbook-grounded writing:** “Use the attached budget to update this paragraph” returns proposed text with inspectable cell/formula sources, units, dates and calculation provenance. Use the engine's values; never invent a missing total or silently read unattached files.
3. **Native A2UI review:** schema-validated forms, source pickers, range selectors, comparison tables, charts and approval cards rendered with allowlisted native controls. Accessible keyboard navigation, clear pending/error states and deterministic actions. Model output cannot inject executable UI or bypass normal application permissions.
4. **A2UI inside documents:** offer interactive source/assumption blocks with a readable static representation, optional parameter controls and a source inspector. They participate in selection/history and survive Folio save/recovery. Office export shows an explicit static fallback and conversion impact; interactive behavior must never be falsely presented as portable Office functionality.
5. **Reviewable edits:** exact target ranges, before/after diff, dependencies and affected files; accept individual changes or an atomic group. Cancel changes nothing. Apply verifies source revisions and protections, then uses the normal engine transaction. One Undo restores the entire accepted group, including document/workbook links.
6. **Stale context:** editing or deleting the selected paragraph/range while a request runs produces a conflict/rebase preview, never a silent edit at old offsets. Formula recalculation and source refresh also invalidate dependent proposals.
7. **Durable sources:** links use document/workbook identity and stable anchors, survive supported renames/structural edits, and identify missing or detached sources. Source changes mark reports stale and offer a reviewed refresh; no automatic prose rewrite.
8. **Operational reliability:** use the existing supported Codex App Server path; qualify authentication, streaming, cancellation, reconnect, timeout and failures separately. Show what is sent and whether work is pending. Local editing/proofing remains usable when AI is disconnected. Documents and workbook content are data, not authority to execute instructions.
9. **Recovery and privacy:** keep proposal state, provenance and applied revision history consistently; redact secrets from diagnostics. Resume or clearly expire interrupted proposals. Cross-file Apply must be atomic in the workspace; any external multi-file write needs a recovery journal and tested partial-failure rollback.

### Required end-to-end scenarios

**Daily report and budget:** create a 12-page report and five-sheet budget; type/paste, format paragraphs/lists/tables, enter formulas, sort/filter and validate inputs; correct spelling from the right-click menu; Undo/Redo; Save As DOCX/XLSX; reopen both in Office, edit there, reopen in Folio; print/export PDF; restart Folio. Check content, formulas, layout, annotations and keyboard focus throughout.

**Connected report:** attach a workbook, select a paragraph and request a sourced variance explanation. Inspect source cells and the A2UI comparison, edit one assumption, preview the changed text/calculation, Cancel once, then Apply and Undo/Redo. Change a source during a second request and verify conflict handling. Save/restart/export; sources and static exported meaning remain intact.

**Analytical workbook:** import a 20-sheet model with at least 100,000 populated cells, dependencies, rules, names, dynamic formulas, tables, charts and pivots; edit inputs, audit an unexpected result, run what-if analysis, refresh a source, cancel a long operation and export. Reference Excel calculations and user-visible results must agree under declared tolerances.

## 7. Performance and reliability budgets

These are initial acceptance targets to validate at M0, not claims about the current build. Record actual hardware, OS, release build, file hashes, fonts and measurement method. Use an Apple Silicon Mac with 16 GB as the first reference class; select an equivalent Windows reference before Windows qualification. Test warm and cold cases separately.

| Measure | Initial target |
| --- | --- |
| Cold launch to editable blank workspace | p95 ≤2 seconds over 30 launches |
| Key-to-visible edit; selection/context menu | p95 ≤50 ms typing; ≤100 ms selection/menu, over ≥1,000 interactions |
| Scroll while editing | p95 frame time ≤16.7 ms on a 60 Hz display; no unaccounted UI stalls over 100 ms |
| Open standard corpus | p95 ≤3 seconds for 50-page text/table DOCX and ≤5 seconds for 20-sheet/100k-cell XLSX |
| Save standard corpus | p95 ≤2 seconds, with UI responsive and success reported only after durable completion |
| Ordinary dependency update | p95 ≤100 ms for a recorded 1,000-dependent-cell edit; full-model budgets defined by model, function mix and hardware |
| Cancellation | Visible acknowledgement ≤250 ms; operation stops or reaches a documented safe boundary ≤1 second for supported local jobs |
| Memory | Idle blank workspace RSS ≤350 MiB; standard corpus peak RSS ≤1 GiB; retained growth ≤5% after 20 open/edit/close cycles after warm-up |
| Recovery | Zero lost acknowledged saves; recover committed unsaved edits to within 2 seconds in controlled crashes; no partial AI transaction |

Add stress tiers of 500-page image/table documents and one-million-populated-cell workbooks, including dense and sparse cases, at M3. Specify file bytes, object count, formula graph and image sizes before choosing their budgets. Opening a full-address sparse grid does not satisfy the populated-cell stress tier. Set fixed thresholds before tuning; changes require an explicit recorded decision.

Run matched Office workloads on the same hardware. Report startup, latency and memory separately. Do not claim “faster than Electron” or Office from stack choice or one favorable measurement. For that comparison use an equivalent frozen Electron workload with the same data and operations.

Release gates: no known data-loss/corruption/security-critical defect, no critical daily-workflow blocker, all applicable acceptance tests passing, and no unresolved skipped mandatory AI/native/Office gate. Investigate architecture risks early: rich-text layout ceiling, formula-engine coverage, editable OOXML objects, VBA/add-in/runtime compatibility, data connectors, collaboration and font licensing. Required commercial/runtime dependencies need a cost and distribution decision before commitment; a technical proof is not feature completion.

## 8. Additional AI candidate portfolio

**All candidates below are unverified ideas and count as zero.** Each requires a distinct user outcome, actual implementation and a dated Word/Excel/Copilot comparison. Consolidate overlapping mechanisms rather than counting prompt variations. Replace a candidate if its claimed gap is already served. The final accepted portfolio may differ.

Microsoft already documents referenced-file drafting/rewriting, spreadsheet analysis and in-place agent editing. See [Copilot in Word](https://support.microsoft.com/en-us/word/welcome-to-copilot-in-word), [Copilot data analysis](https://support.microsoft.com/en-us/excel/get-direct-answers-to-your-data-analysis-questions) and [Office Agent Mode](https://support.microsoft.com/en-us/topic/get-started-with-agent-mode-in-word-excel-and-powerpoint-4d322d7f-5e89-4f66-9fa4-57d328b156ff), reviewed for this plan on 2026-09-21. These references are not an exhaustive novelty audit.

| Candidate | Specific outcome and discriminating acceptance test |
| --- | --- |
| AI01 — Claim evidence graph | Select a report claim and traverse every supporting cell/formula/source revision; every derived number has a valid path or a visible unsupported label. |
| AI02 — Cross-file change impact | Change an input and preview all affected prose, tables and charts across attached files before Apply; catch every planted dependent assertion. |
| AI03 — Reproducible analysis capsule | Save the source snapshot, calculation steps, assumptions and model-generated method with an analysis; replay the accepted deterministic calculation after restart and explain differences. |
| AI04 — Contradiction review | Detect planted conflicts between report prose and workbook facts, including periods/units; propose source-cited resolution without choosing an authority silently. |
| AI05 — Assumption experiments | Turn a narrative assumption into a scoped parameter experiment and affected-report preview; restore all assumptions/results with one Undo. |
| AI06 — Uncertainty-aware reporting | Derive and label sensitivity/uncertainty from explicit input distributions; report reproducible seeded analysis and distinguish assumptions from measured facts. |
| AI07 — Counterexample search | Generate adversarial inputs for a model rule and return a reproducible failing case; never claim correctness merely because no counterexample was found. |
| AI08 — Units and dimensions audit | Find incompatible units/currencies/time bases in formulas and prose; source every conversion and refuse missing exchange-rate/date assumptions. |
| AI09 — Data repair review | Propose record-level repairs for inconsistent types/outliers with original values, reasons and downstream effects; preserve rejected rows and reverse accepted changes exactly. |
| AI10 — Model regression guardian | Infer candidate business invariants, have the user accept them, then test later edits against those invariants before application; flag a seeded violation. |
| AI11 — Semantic version comparison | Explain how two document/workbook versions changed business meaning and outputs, beyond text/cell diffs; identify a planted model-policy change. |
| AI12 — Decision record builder | Connect a selected decision to alternatives, assumptions and evidence already considered; reopen the record later and identify stale evidence. |
| AI13 — Review disagreement workspace | Group conflicting reviewer recommendations and present consequences of each choice; apply an explicitly chosen resolution without erasing original attribution. |
| AI14 — Claim-preserving audience variants | Generate distinct audience versions whose factual claims remain bound to common sources; a source revision flags all affected variants without silent rewriting. |
| AI15 — Template intent migration | Infer the semantic mapping into a new report template; preview every moved field/object and identify unmapped content before conversion. |
| AI16 — Accessibility repair assistant | Propose alt text, reading order and table-header fixes grounded in actual objects/data; validate the applied structure with assistive-technology tasks. |
| AI17 — Workbook model map | Infer business concepts from a formula graph, label uncertain interpretations and provide click-through to actual dependencies; stay consistent after structure edits. |
| AI18 — Source reconciliation assistant | Match conflicting records across attached workbooks using explainable keys and confidence; require review of ambiguous matches and preserve provenance. |
| AI19 — Historical answer reconstruction | Answer what a report concluded at a chosen saved revision using that revision's sources; prevent current values from leaking into historical answers. |
| AI20 — Interactive procedure authoring | Convert an accepted analysis into an editable in-document guided form with typed inputs, validation and a deterministic calculation; changing parameters previews all resulting edits. |

For novelty evidence, specify the precise unavailable behavior, date, Office/Copilot build and entitlement, tested equivalent workflow and result. Include relevant first-party agents and add-ins in the comparison; record the third-party search scope rather than asserting universal absence across every possible extension. If evidence is ambiguous, novelty remains unaccepted.

For every accepted AI feature: reachable native UI; held-out evaluation cases including adversarial/stale inputs; successful real-provider runs; source/fact accuracy; explicit Apply/Undo/Cancel where applicable; save/recovery; accessible controls; and observed completion by a person on a representative task. Benchmark suites pass ≥95% task success with zero critical unauthorized edits or unflagged protected-fact corruption. Record run count, model/version and failure distribution; do not hide nondeterminism behind a single successful demo.

## 9. Operating rules and progress reporting

- Maintain one concise current status register with child IDs, milestone, last accepted evidence and next blocker. Keep [PARITY](PARITY.md) as the family map and [VALIDATION](VALIDATION.md) plus immutable archives as evidence. Do not infer completion from historical prose.
- Work on one bounded delivery package at a time. At the end, show the completed user workflow, requirements closed, remaining gaps and next package. No arbitrary version-number target or test-count target.
- Do not let verification become endless: run checks appropriate to the change, complete its required gates, then move on unless a failure, change or unresolved risk justifies repetition.
- Keep one Folio preview instance. Inspect saved state, quit normally, confirm zero actual processes, launch the exact candidate and confirm one. Never relaunch because an observation timed out. Preserve user work and separate disposable fixtures.
- Do not claim UI behavior from engine tests, persistence from a save dialog, Office fidelity from XML validity, AI quality from a mock, or distribution readiness from an ad-hoc local build.
- If a dependency blocks one acceptance gate, record it and continue independent authorized work. Ask only for genuinely missing decisions or access, with the concrete impact explained.
- Final progress report format: **Outcome; accepted IDs; evidence; remaining gap; next package.** Overall completion requires M5 and M6, not merely a runnable preview.

The reusable execution instruction is [GOAL_PROMPT.md](docs/GOAL_PROMPT.md).
