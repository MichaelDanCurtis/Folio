# Copying and filling conditional formats

Paste All and Paste Formats now transfer conditional rules with the copied
cells. Destination rules are removed only from cells being replaced; remaining
ranges and their formula anchors are retained. Paste Values and Paste Formulas
keep destination rules. Plain-text clipboard input carries no conditional-rule
payload. The clipboard continues to expose only text and an opaque token to the
operating system; rule and workbook snapshots stay inside the application.

Paste Special also offers **Cells and merged conditional formats**. It copies
contents and formatting while retaining destination conditional rules. Copied
rules appear above retained rules and preserve their source ordering and Stop
if true behavior. Nonconflicting source/destination styles can combine. This is
separate from replacing destination conditional rules with ordinary Paste All.

Rules follow the same cell map as values through overlap, repeated tiles,
transpose and Skip blank cells. Directional Fill copies from the immutable
source edge. Rules are grouped into rectangular or disjoint destination ranges;
copying a column does not create a rule for every destination cell. Percentile,
rank, duplicate, scale and icon rules use the new destination population.
Formula references shift from the original rule anchor, preserving absolute and
mixed references. Surviving destination fragments are reanchored as necessary.

Modeled standard and Office 2010 rules use the existing OOXML writer, including
color scales, data bars and custom/new icon sets. Copied extension rules receive
new identifiers reserved against every worksheet in the package, and data-bar
fallback links reference the copied rules. Known differential styles and scoped
XML metadata remain intact. Copied rules participate in the same measured edit,
Undo/Redo, recovery and export transaction as the cell changes. A measurement
failure cancels the complete paste.

## Compatibility boundaries

Unmodeled source rule types, unknown copied identifiers/attributes or group
content, and operations that would partially rewrite an unsupported destination
rule fail before publishing the paste. Unaffected opaque rules remain preserved.
Unmodeled extended-rule containers still prevent conditional-rule changes;
Values/Formulas remain available. Existing workbook-theme/metadata, protected
sheet, merged/array-formula, 100,000-cell and 4,096-rule limits still apply.

Missing explicit sheet references are checked as formula syntax nodes when the
source and destination sheet catalogs differ. Full defined-name, structured and
external-reference evaluation is not established. The independent corpus and
modeled tests do not substitute for direct Microsoft Excel behavior comparison.
In particular, Excel's rule consolidation, overlapping aggregate populations,
transpose/reference nuances, filter/hidden-row behavior and all merge-priority
cases need differential qualification. Cut/move, Format Painter and every paste
option beyond the implemented menu remain incomplete.

## References and evidence

- [Microsoft: paste options](https://support.microsoft.com/en-us/excel/paste-options)
- [Microsoft: PasteAllMergingConditionalFormats](https://learn.microsoft.com/en-us/office/vba/api/excel.xlpastetype)
- [Microsoft: rule precedence and copying/filling](https://support.microsoft.com/en-US/Excel/use-conditional-formatting-to-highlight-information-in-excel)

The replacement-versus-merge interpretation follows the separate documented
paste modes; exact Excel rule fragments are not claimed from that documentation.
Tests, package identity, independent readers and native qualification are
recorded in VALIDATION.md. No complete Word/Excel parity family is accepted.
