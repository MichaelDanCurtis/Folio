# Conditional-formatting icon sets (0.48)

In Sheets, choose **Conditional formatting → New rule → Icon sets**. Choose a
style and set category boundaries, listed highest first. Each boundary accepts
Number, Percent, Percentile, or Formula and either > or ≥. Click a category icon
to choose any glyph or **No cell icon**. **Reverse icon order** reverses the
selected glyph assignments without changing boundaries. **Show icon only** hides
the displayed value; calculations, cell editing, clipboard values and file data
retain it. Done stages the rule; Apply validates all staged rules and adds one
workspace Undo entry. Cancel does not apply the staged rule.

## Implementation and file behavior

- Twenty styles: 3/4/5 arrows in color and gray; flags; rimmed/unrimmed traffic
  lights; signs; circled/plain symbols; red-to-black circles; 4/5 ratings;
  quarters; stars; triangles; boxes. Vector glyphs follow the documented icon
  shapes, category order and fill fractions. They scale with the cell font.
- Percent uses the numeric minimum/maximum over the entire applied range,
  including offscreen populated cells. Percentile interpolates the sorted
  numeric population. Blank/text/Boolean/error cells receive no icon. Formula
  thresholds use absolute scalar references, and update with recalculation.
  Non-numeric or descending resolved thresholds warn instead of showing icons.
- Rules claim a separate display channel by priority, including No cell icon.
  A higher-priority ordinary Stop if true rule can prevent an icon. Styles,
  colors and data bars remain independent channels. Derived icon payloads are
  not ordinary cell styles or cell values. Icon-only numbers remain accessible.
- Standard OOXML stores the first 17 styles in ordinary `iconSet` rules. Newer
  styles and custom combinations use Office 2010 `x14` rules with independent
  IDs and priorities. A custom rule's category count comes from its thresholds,
  not its unused base style. The unused base can be NoIcons. Reverse is ignored
  by the file format for custom lists, so the UI reverses the list itself.
- Range and absolute threshold references move with supported structural edits.
  Rules persist through XLSX export/import and workspace recovery. Older opaque
  standard rules are upgraded by exact original source identity; previously
  locked extended rules gain newly modeled icon records. Deleted rules are not
  recreated in already upgraded workspaces.
- Unmodeled extension metadata keeps rules read-only and preserves its source.
  Promoting a standard rule to an extension-only representation fails if doing
  so would discard additional source metadata. This is a compatibility limit,
  not acceptance of arbitrary vendor extensions.

## Verification boundary

Core tests cover all 20 styles, standard/extended serialization, a separately
produced openpyxl import fixture, strict/inclusive thresholds, percentile and
formula updates, whole-column/offscreen populations, nonnumeric/error behavior,
custom categories, reverse/NoIcons, priority, history, recovery, structural edits,
and preservation or atomic rejection of unmodeled metadata. Flutter tests cover
the actual picker/apply workflow, custom icons, recalculation and Undo, column
AutoFit, narrow cells, accessibility, and distinct glyph rendering. An independent
openpyxl/ElementTree reader inspects exports without resaving extended workbooks.
See VALIDATION.md for the final executed results.

Native macOS input, direct Microsoft Excel visual/numerical comparison, printed
output, icon sorting/filtering, copy/fill/cut rule semantics, pivot/table-specific
rules, locale differences and remaining X07 conditional-formatting behavior are
not accepted complete. The 20 icon styles are Excel features; they do not count
as any of the requested 20 novel AI features.

## Primary references

- [Microsoft icon-set enumeration and images](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-xlsx/bfdf00a5-4e49-4b00-8803-32250e1683c2)
- [Office 2010 IconSet structure and custom/reverse rules](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.office2010.excel.iconset?view=openxml-3.0.1)
- [Threshold value objects](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-xlsx/15094f93-f587-4f4e-be74-1fbdc93da768)
- [Excel conditional-formatting workflow](https://support.microsoft.com/en-us/Excel/use-conditional-formatting-to-highlight-information-in-excel)
