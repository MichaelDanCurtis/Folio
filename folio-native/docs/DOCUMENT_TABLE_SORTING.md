# Document table sorting — 0.75 preview

**Edit table → Sort…** adds up to three ordered column/field keys, each using
Text, Number or Date in ascending or descending order. The first/last row range
and heading count are editable; an existing multi-row selection supplies the
initial range. Consecutive repeating headings are retained by default when the
range starts at the top. Sorting moves complete rows, including their rich text,
cell contents, nested non-key content, fills, dimensions and source identities.

The dialog previews original row numbers and values. Changing settings clears
the old result immediately, and a late asynchronous comparison cannot replace
a newer preview. Cancel keeps the current table draft. Sort updates that draft;
outer **Apply changes** commits the draft and sort in one workspace history step.
The selected cell follows its row so subsequent typing edits the same content.

## Comparison and preservation

- Text collation uses macOS Foundation with the chosen locale and case option.
  Equal collation values receive equal ranks, allowing later keys to break ties.
  There is no silent code-point fallback when the native service is unavailable.
- Twelve language choices cover the current UI. Numeric and date parsing use
  the same locale. Decimal comparison retains large integer precision, signs,
  grouping, exponents, common leading currency symbols and percent values.
  Numbers and dates must parse completely; invalid values identify their row and
  column instead of silently sorting as zero. Empty fields stay last in either
  direction; all-equal keys preserve source order.
- A field can be a paragraph, or a tab/comma/semicolon-delimited value. Manual
  line breaks stay inside their paragraph field. Arbitrary custom separators
  and Word's full field/separator options are still outstanding.
- Vertically merged data rows cannot be sorted yet. A horizontal merge may be
  retained in a heading or non-key column; a merged key column reports an error.
- Created, inserted and converted rows receive durable identities. Identified
  row permutations move bookmarks and comments with their source rows, including
  local text edits and independent table formatting. Existing rows without IDs
  can be matched by unchanged content. A range whose endpoints would reverse
  across sorted rows is rejected rather than silently discarded.
- DOCX export reuses complete unchanged row XML when column widths and vertical
  geometry permit it. This retains wrapper attributes, unmodeled run properties
  and alternate namespace prefixes. Changed-row reconstruction continues through
  the existing table serializer and retains its remaining compatibility limits.

## Verification

All **368 Rust tests** and **364 Flutter tests** pass, with two optional AI
provider skips. Analysis is clean. Seven focused Dart checks cover multilevel
sorting, full-row preservation, ties/blanks, exact/localized numbers, dates,
partial ranges, field boundaries, merges, native response validation, late
preview results and Cancel. A Foundation executable verifies canonical accent
equivalence, case sensitivity, Swedish/German/Turkish ordering and empty/multiple
columns. This verifies the platform comparison function, not the packaged bridge.

The real-core application workflow edits Zulu to Zulu!, cancels both levels,
reopens the table, sorts Amount/Name/Date, continues typing in the moved cell,
and applies once. The resulting Zulu!? row keeps its bookmark and comment on
Zulu. Workspace Undo/Redo, unrelated data, export, recovery, byte-identical
recovered export and reimported anchors pass. Three additional Rust checks cover
row-local range rebasing, crossing-range rejection, and complete unchanged row
XML retention with standard and alternate Word namespace prefixes.

The independent reader checks row order and formats, actual DOCX bookmark/comment
ranges, comment text/author, commentsExtended paragraph links, other package
parts and recovered bytes. Three UI-produced files and four engine files pass
Microsoft365 Open XML schema validation. The source fixture initially omitted
the required shading `val` attribute; that fixture was corrected and its exact
row-retention test plus schema validation rerun. Production code did not change.
The development archive is `release/validation/document-table-sorting-development`.

## Native qualification

The isolated 0.75 package matches 306 app/native/project source files, 108 Rust
files and six template assets against the tested implementation. Version metadata
is the only production change from the development archive. The package uses
local ad-hoc signing; the normal release has not been promoted or notarized.
The saved 0.74 preview quit normally before the new app launched. Restart checks
also verified zero then one process; the normal user workspace hash is unchanged.

The native three-level Amount/Name/Date sort keeps the heading and moves the
edited Zulu!? row to row 5. Typing after sorting follows the moved cell. Sort
Cancel keeps the draft, outer Cancel preserves the original recovery bytes, and
Apply commits once. Ordinary and post-Save keyboard Undo/Redo restore exact
workspace contents. Bookmark and comment Go To both select only Zulu in row 5;
the imported comment still has its original author and message.

Sixteen captured fixture states pass `verify_native_table_sort.py`. Restart
preserves recovery bytes. Save As, recovered Save As, direct-mouse toolbar Export
and native-keyboard Export after DOCX reimport produce four byte-identical files.
Independent XML checks verify row content/properties, real annotation endpoints,
other package parts and comment paragraph links. All five source/output DOCX
files pass Microsoft365 schema validation. Reimport preserves document semantics;
source XML positions and equal-format run splits at annotation boundaries are
normalized for the JSON comparison, and its subsequent export is byte-identical.

A disposable draft also verifies native Swedish ordering Zulu, Åland, Ängelholm,
Öland and German ordering Åland, Ängelholm, Öland, Zulu. Cancelling both dialogs
leaves the saved document unchanged. Screenshot and accessibility observations
are in the qualification conversation. Evidence is archived in
`release/validation/document-table-sorting-native`.

Two input limitations remain explicit. An accessibility action opening Export
while the app was unfocused produced a panel that dismissed during automated
input; a direct mouse click produced a focused panel and completed, as did the
native export shortcut. Separately, native modifier shortcuts followed by table
cell clicks sometimes extended the selection unexpectedly. A Shift-arrow pair
in the editor restored ordinary selection in this run; the cause and native
modifier-state reconciliation still need investigation. Automated accented typing
also omitted an initial Ä; verified clipboard input supplied the exact text.
These observations do not qualify every input method or accessibility path.

## Remaining qualification and functionality

The sole running native preview is 0.75, with the saved sorted fixture at revision
7 and no open draft. Modifier/focus handling, input methods, screen readers,
large-table performance, all locale formats and actual Word differential
comparison remain unqualified beyond the specific native checks above.

Full table sorting also needs Word's column-only sorting and remaining field,
language and bidirectional options, vertically merged data behavior, nested-table
sorting, complex overlapping/cross-row annotations, and mixed structural drafts
or legacy/duplicated row identities. Sorting with Track Changes and preserved
revision structures requires direct qualification. No entire Word/Excel parity
family or feature toward the subsequent 20 novel AI requirement is accepted.

## Baseline sources

[Microsoft's table sorting workflow](https://support.microsoft.com/en-us/word/sort-the-contents-of-a-table)
describes three levels, Text/Number/Date, headings and sort options.
[Word Table.Sort](https://learn.microsoft.com/en-us/office/vba/api/word.table.sort)
also specifies language, case and bidirectional choices. These remain the target,
including options not yet implemented. The platform comparison is based on
[Foundation string comparison](https://developer.apple.com/documentation/foundation/nsstring/compare(_:options:range:locale:)).
