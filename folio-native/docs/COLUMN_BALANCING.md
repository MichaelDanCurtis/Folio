# Column balancing — 0.70 development

Page preview balances an ending multi-column section before a continuous
section that can share its page geometry. The next section begins below the
tallest preceding column, including when both sections use identical columns.
Normal document endings and next-page sections retain sequential column flow.
Word's `noColumnBalance` compatibility setting disables balancing. The reader
uses the actual related settings part and the format's on/off values without
rewriting source XML or adding default settings.

Only the ending section's content on its final physical page is replayed.
Earlier pages, headers/footers and preceding sections retain their placements.
Paragraphs resume from the first unconsumed source character, including rich
Unicode text and page fields. Table content resumes at complete row groups;
vertical merges stay together. Unequal widths are remeasured in reading order.
Manual column breaks and existing paragraph keep/widow constraints remain in
effect. These constraints can leave columns uneven or keep a short tail entirely
in a wider column.

A bounded search tests shorter column heights without permitting an extra
physical page or artificial clipping. Failed trial painters are disposed;
the best fitting layout is retained. Previewing never commits document changes,
field caches or an Undo step. Oversized fragments that cannot safely resume at
a source line or row boundary retain their existing layout with a notice.

## Qualification boundary

All 356 Rust and 316 Flutter tests pass, with two optional AI-provider skips;
static analysis is clean. The engine test resolves absent/on/off compatibility
values and a relocated settings relationship without changing source bytes.
Twelve layout tests cover continuous boundaries, earlier-page preservation,
Unicode/RTL and unequal widths, fields, paragraph constraints, manual breaks,
merged table rows, and earlier content in a shared column area.

The real-engine application test imports both fixture variants, opens the
ribbon preview, checks column placement and following-section position, returns
to editing, exports and reloads recovery. Preview and recovery bytes remain
unchanged; each exported DOCX is byte-identical to its source. An independent
ZIP/XML reader confirms the fixtures differ only in the compatibility setting,
and Microsoft365 schema validation passes for all four DOCX artifacts.
The two test-rendered page images use a real font and were visually inspected;
they are headless Flutter evidence, not native screenshots.

The isolated 0.70.0 build 1 passes native import, both preview variants and
Save As. Balanced mode places lines 00–10 on the left and 11–19 plus the section
ending on the right; the next section follows beneath them. Disabled mode keeps
all section content in the left column. Both native exports remain identical
to their sources. Previewing retains exact recovery bytes at revisions 1 and 2;
reopening the balanced export and restarting the same app retain revision 3
and identical recovery bytes, with the balanced page restored. The native
exports also pass Microsoft365 schema validation.

The prior saved preview was closed normally before replacement. Process counts
were zero before each launch and one afterward. Exactly one 0.70 preview is
left running. The package, six validated source/export DOCX artifacts, test
renders, native recovery snapshots and hashes are recorded under
`release/validation/column-balancing-development`. The Columns dialog's native
Apply/Cancel, keyboard history and restart workflow subsequently passed in
this same 0.70 build; see COLUMNS.md and `release/validation/columns-native`.

This is a scoped layout implementation. Microsoft Word has not been executed
for differential layout comparison. Full keep-with-next chains, exact Office
font metrics, complex bidi, floating content, footnotes, table header repetition,
oversized rows and large-document performance remain open. Bounded balancing
does not establish globally optimal column heights for every combination of
unequal widths and keep constraints. No parity family or new AI feature is
accepted by these checks.

Primary references:

- [Word compatibility: continuous-section column balancing](https://learn.microsoft.com/en-us/office/vba/api/word.wdcompatibility)
- [Open XML noColumnBalance setting](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.nocolumnbalance?view=openxml-3.0.1)
