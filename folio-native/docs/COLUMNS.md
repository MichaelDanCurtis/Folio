# Section columns — 0.69 development

Layout > Columns opens a section-scoped draft. Choose One, Two, Three, Left or
Right, or set a count, spacing and individual widths in inches, centimetres or
points. Equal column width and Line between columns are separate controls.
Reading order can start from either side of the page. Apply to This section or
Whole document; custom widths must fit every target section. To format only
part of a document, first create section boundaries around that text.

Apply is one workspace history step. Cancel retains the original document.
Pending body text is flushed separately before opening the dialog. Protected
documents, stale drafts and unresolved section-formatting reviews are guarded
by the existing page-setup transaction. With formatting recording enabled,
column changes produce a section-formatting revision. Review lists each custom
width and spacing in readable units; Accept/Reject retains header references.

The Office reader resolves equal-width definitions from the section count and
spacing, and unequal definitions from the actual column children, even if a
stored count differs. Dimensions support the existing measurement-unit parser.
Read-only import does not materialize defaults. Explicit editing retains
unrelated column/section attributes, other sections and package parts. Equal
columns ignore retained custom definitions as required by the format. Custom
definitions preserve existing per-column attributes by index. Editing supports
1–45 columns; unsupported/malformed imported layouts display a preview notice.

Page preview fills columns in the selected reading order before advancing to
the next physical page. Headers/footers remain across the page's text area.
Separator rules appear between physical columns. A paragraph crossing into a
different width is reshaped from the first unconsumed source character, keeping
rich spans and field identities. Tables remeasure complete row groups at the
new width while retaining vertical-merge boundaries. Manual column breaks
advance a column; page breaks and page-break-before advance the physical page.
Page fields continue to count physical pages rather than columns.

Compatible new-column sections share the next column on the page. A continuous
section with different columns begins below preceding content in a new column
area when there is room. Changed page geometry, and new-column sections that
also change column geometry, currently start on a new page with a notice.

## Remaining work

Version 0.70 balances final-page content before compatible continuous section
boundaries; see COLUMN_BALANCING.md. This is not complete Word column or
pagination parity. Automatic section creation around selected text, direct inline
column-break insertion/removal and full paginated editing remain open. The
existing limitations on full keep-with-next chains, oversized table rows,
floating objects, footnotes/endnotes, first-line/hanging indents, complex text
layout and Office font matching still apply. Microsoft Word layout comparison and large-document
performance qualification remain outstanding.

## Evidence

All 355 Rust tests pass. Four added engine workflows cover custom/default
definitions, source scope and extension retention, invalid measurements and
whole-document targets, recorded formatting review, pending body text,
workspace history, export and recovery. Seven focused Flutter checks cover
equal/unequal columns, rich Unicode text, table row groups, RTL order, distinct
break types and fields, continuous/new-column sections, and dialog drafts.
A real-engine application check exercises ribbon entry, pending text, Cancel,
custom-left columns, separator/RTL settings, minimum window geometry, preview,
Undo/Redo, export and recovery. The 17-test targeted layout/application run
passes, including the nine pre-existing pagination checks.

The full suite passes 303 Flutter tests, with two optional AI-provider checks
skipped; static analysis is clean. Independent OOXML inspection passes on the
source and application-produced DOCX. Microsoft365 schema validation also
passes on the engine's recorded, accepted, rejected and recovered exports
(six DOCX artifacts total).

The isolated 0.69.0 candidate was superseded before native launch. Its automated
evidence remains archived in `release/validation/columns-development`. Native
qualification runs in the signed 0.70.0 build 1, whose 299 application files,
six templates and native engine match the tested source.

Native Cancel retains exact recovery bytes. Applying the Left preset, separator
and right-to-left reading order changes only Section 1 in one history step.
Keyboard Undo/Redo, including immediately after Save As, restore the document
and Office package state exactly. The exported DOCX passes independent XML and
Microsoft365 schema checks; other package parts, the landscape section, linked
stories and stored field results remain unchanged. Normal quit and restart
preserve byte-identical revision-9 recovery, and the dialog restores its custom
widths, separator and reading order. The verifier takes an explicit baseline
revision so this workflow can use the existing preview instead of a new app.

All seven native preview pages were inspected: Section 1 flows from the wide
right column into the narrow left column over pages 1–5; page 6 is the blank
page needed for the next odd-page section; physical page 7 is landscape with
the restarted footer `Page v of 7`. Text continues across changes in column
width, with the shared header and physical-page values retained.

Evidence is in `release/validation/columns-native`. There was exactly one Folio
process throughout, with zero confirmed before the same preview was restarted.
The normal user workspace and release are unchanged. These remain scoped
Folio checks; Microsoft Word was not executed, and no complete parity family
or novel AI feature is accepted.

Primary references:

- [Word column definitions](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.columns?view=openxml-3.0.1)
- [Equal-width and custom columns](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.columns.equalwidth?view=openxml-3.0.1)
- [Column separator placement](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.columns.separator?view=openxml-3.0.1)
- [Word column controls](https://support.microsoft.com/en-us/word/training/create-multiple-columns-in-word)
