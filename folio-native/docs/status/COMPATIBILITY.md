# Compatibility matrix

The file compatibility contract from ROADMAP §2 (W16.09, X17.09): for each feature in a Word or Excel file, what Folio does when it opens, shows, edits, calculates, saves, round-trips, prints or automates it.

The states:
- **Verified**: automated round-trip evidence.
- **Partial**: some of the feature is edited, and the rest is kept.
- **Preserved only**: kept unchanged, but not shown or edited.
- **Unsupported**: refused, dropped or invalidated.
- **—**: does not apply.

Folio has no printing yet, so Print is Unsupported throughout.

The tables below are generated from `rust/src/compatibility.rs`. The app's Compatibility Report uses the same table. Regenerate them with:

```
FOLIO_WRITE_COMPATIBILITY=1 cargo test --lib compatibility_matrix
```

A test fails when the tables and the code differ.

## What exporting does beyond the table

- **Signed files:** exporting a signed file that has changed removes its digital signatures, because a signature no longer matches a changed file. Exporting an unchanged file keeps them.
- **Edited paragraphs:** an edited paragraph is written from Folio's formatting. Character formatting Folio doesn't edit is lost in that paragraph, such as small caps, character spacing or hidden text.
- **Preview:** before an export that would do either of these, Folio lists what will be removed or changed. It offers to save a Folio copy instead.

<!-- generated -->

## Word documents (DOCX)

| Feature | Open | Display | Edit | Calculate | Save | Round trip | Print | Automation | Saving an edited file | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|
| Text and character formatting | Verified | Verified | Verified | — | Verified | Verified | Unsupported | Partial | Written from Folio's edits; formatting Folio doesn't edit is lost in edited paragraphs | W01, W16.01; corpus round trip |
| Styles | Verified | Verified | Verified | — | Verified | Verified | Unsupported | — | Kept, with Folio's changes | W02 |
| Numbered and bulleted lists | Verified | Verified | Verified | — | Verified | Verified | Unsupported | — | Kept, with Folio's changes | W03 |
| Tables | Verified | Verified | Verified | — | Verified | Verified | Unsupported | — | Kept, with Folio's changes | W07 |
| Inline pictures | Verified | Verified | Partial | — | Verified | Verified | Unsupported | — | Kept; new pictures added | W08.01–W08.02; corpus pictures |
| Floating pictures and shapes | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | Unsupported | — | Kept unchanged | W08.04, W08.08 |
| Fields | Verified | Partial | Partial | — | Preserved only | Verified | Unsupported | — | Page fields written; other fields kept unchanged | W10 |
| Comments | Verified | Verified | Verified | — | Verified | Verified | Unsupported | — | Kept, with Folio's changes | W13.01 |
| Tracked changes | Verified | Verified | Partial | — | Verified | Verified | Unsupported | — | Kept; Folio's tracked edits recorded | W13.03–W13.06 |
| Headers and footers | Verified | Verified | Verified | — | Verified | Verified | Unsupported | — | Kept, with Folio's changes | W06 |
| Footnotes and endnotes | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | Unsupported | — | Kept unchanged | W11.01 |
| Equations | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | Unsupported | — | Kept unchanged | W09.01 |
| Content controls | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | Unsupported | — | Kept unchanged | W09.05 |
| Charts and SmartArt | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | Unsupported | — | Kept unchanged | W08.06–W08.07 |
| Embedded objects | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | Unsupported | — | Kept unchanged | W09.02 |
| Bookmarks and hyperlinks | Verified | Verified | Verified | — | Verified | Verified | Unsupported | — | Kept, with Folio's changes | W04.07, W01.14 |
| Sections, page setup and columns | Verified | Partial | Verified | — | Verified | Verified | Unsupported | — | Kept, with Folio's changes | W05 |
| Macros | Preserved only | — | Unsupported | — | Unsupported | Unsupported | — | Unsupported | Refused: Folio doesn't write files that contain macros | D06 |
| Digital signatures | Verified | — | Unsupported | — | Unsupported | Preserved only | — | — | Removed when the file has changed, as a signature no longer matches it; kept on an unchanged export | D06 |
| Protection | Verified | — | Partial | — | Preserved only | Preserved only | — | — | Kept unchanged; not enforced in Folio | D06, W15, X16 |
| Custom XML data | Verified | — | — | — | Preserved only | Preserved only | — | — | Kept unchanged | W08.08 |
| Other package parts | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | — | — | Kept unchanged | W08.08 |

## Excel workbooks (XLSX)

| Feature | Open | Display | Edit | Calculate | Save | Round trip | Print | Automation | Saving an edited file | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|
| Cells, values and formulas | Verified | Verified | Verified | Partial | Verified | Verified | Unsupported | Partial | Written from Folio's edits with recalculated values | X01, X08; corpus: 99.87% of formulas match Excel |
| Cell styles and number formats | Verified | Verified | Verified | — | Verified | Verified | Unsupported | — | Kept, with Folio's changes | X04 |
| Merged cells | Verified | Verified | Verified | — | Verified | Verified | Unsupported | — | Kept, with Folio's changes | X04 |
| Tables | Verified | Verified | Verified | Verified | Verified | Verified | Unsupported | — | Kept, with Folio's changes | X06 |
| Conditional formatting | Verified | Verified | Verified | Verified | Verified | Verified | Unsupported | — | Kept, with Folio's changes | X07 |
| Data validation | Verified | Verified | Verified | Verified | Verified | Verified | Unsupported | — | Kept, with Folio's changes | X10 |
| Comments and notes | Verified | Verified | Verified | — | Verified | Verified | Unsupported | — | Kept, with Folio's changes | X15 |
| Defined names | Verified | — | Partial | Verified | Verified | Verified | — | — | Kept | X09.08 |
| Charts | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | Unsupported | — | Kept unchanged | X11 |
| PivotTables | Verified | Preserved only | Preserved only | Preserved only | Preserved only | Preserved only | Unsupported | — | Kept unchanged; not refreshed | X12 |
| Sparklines | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | Unsupported | — | Kept unchanged | X11 |
| Slicers | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | Unsupported | — | Kept unchanged | X12 |
| Links to other workbooks | Verified | Preserved only | Unsupported | Unsupported | Preserved only | Preserved only | Unsupported | — | Kept unchanged; values not refreshed | X13 |
| Data connections and queries | Verified | Preserved only | Unsupported | Unsupported | Preserved only | Preserved only | Unsupported | — | Kept unchanged; not refreshed | X13 |
| Macros | Preserved only | — | Unsupported | — | Unsupported | Unsupported | — | Unsupported | Refused: Folio doesn't write files that contain macros | D06 |
| Digital signatures | Verified | — | Unsupported | — | Unsupported | Preserved only | — | — | Removed when the file has changed, as a signature no longer matches it; kept on an unchanged export | D06 |
| Protection | Verified | — | Partial | — | Preserved only | Preserved only | — | — | Kept unchanged; not enforced in Folio | D06, W15, X16 |
| Custom XML data | Verified | — | — | — | Preserved only | Preserved only | — | — | Kept unchanged | W08.08 |
| Other package parts | Verified | Preserved only | Preserved only | — | Preserved only | Preserved only | — | — | Kept unchanged | W08.08 |

