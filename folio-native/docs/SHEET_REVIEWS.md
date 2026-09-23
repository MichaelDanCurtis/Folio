# Worksheet comments and notes — 0.58 development

Review > New comment starts a cell conversation. New note creates a standalone
annotation with no replies or resolved state. Comments & notes lists either kind
on the current sheet or all sheets, with All, Comments, Notes, Active and Resolved
filters. Go to cell changes sheets and selects the annotation's address.

Cell corners have purple comment or red note markers. Hover text includes the
annotation and authors. Right-click offers New comment/New note or Show comment/
Edit note, separated from clipboard actions. Shift+F2 opens the selected cell's
note; Command–Option–A or Control–Alt–M opens comments. Command/Control+Enter posts
from the review dialog. Plain text can be edited, threads resolved/reopened,
replies deleted, and either kind deleted. Workspace Undo/Redo restores changes.
Draft navigation or dismissal offers Keep writing/Discard; stale dialogs reject
changes rather than writing to a changed workbook.

Names are explicitly entered local attribution, not authenticated cloud
identity or ownership. The interface does not send invitations or notifications.
Imported names/dates are preserved. Editing a message preserves its attribution.

## Cells and saved files

`sheet.cellReviews` maps physical addresses to annotations. Ordinary typing and
clearing cell values retain them. Insert/delete rows or columns, sheet moves,
range/table sorts, Fill, ordinary native cell paste, and Paste Special > Comments
and notes follow the mapped cells. The latter copies annotations without values
or formatting and replaces destination annotations. Copies get fresh message and
thread identities while retaining source author attribution. Value/formula/
format-only paste leaves destination annotations intact. Copies require Folio's
internal clipboard snapshot; plain external clipboard text has no annotations.

XLSX/XLTX use legacy comments/authors and VML for notes, and Excel's threaded
comments/person parts for conversations. Each conversation also writes the
required legacy placeholder whose author identifies the root thread. All reply
references are emitted explicitly. Imported omitted reply references and legacy
placeholder/root reference disagreement are normalized before structural edits
or sorting; a deleted anchor removes the complete conversation. A no-edit export
returns the original bytes. `.folio` recovery retains both annotation data and
source Office parts.

Imported rich note runs, VML shape definitions, person records and unknown
thread metadata are retained. Unsupported rich text/mention editing is disabled.
Copied rich notes retain runs and their shape definition with unique destination
identifiers. Copying embedded VML relationships or comments containing mentions
is currently rejected. Conflicting author identities, preserved unsupported
annotation collisions and protected-sheet annotation edits are rejected before
committing a change. These limitations are acceptance gaps, not parity.

## Evidence and remaining work

See `VALIDATION.md` and `release/validation/sheet-reviews-development` for the
specific source, test and package evidence. The independent reader script is
`scripts/verification/verify_sheet_reviews.py`; the Microsoft 365 schema check
is `scripts/verification/openxml`. Both operate on synthetic fixtures, and
neither establishes actual Microsoft Excel desktop behavior.

Remaining review work includes an Excel-style persistent task pane and on-cell
note editor, movable/resizable/always-visible note boxes, annotation display
preferences and printing, rich text/image authoring, mentions and authenticated
ownership, cloud coauthoring/reactions/tasks, robust copying of mention/image
relationships, and comprehensive native accessibility/localization/large-file
and Excel-desktop comparison. Protection and tracked history remain separate
unfinished capabilities. No Word/Excel parity family is accepted by this slice.

References checked 2026-09-20:

- [Microsoft: Insert comments and notes](https://support.microsoft.com/en-us/excel/insert-comments-and-notes-in-excel)
- [Threaded comment schema](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-xlsx/42f9b03d-9662-4204-9783-dbeb324a691c)
- [Legacy placeholder requirements](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-xlsx/6383f002-c90b-401c-a1d7-66b97b14cb3e)
- [Person and threaded-comment roots](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-xlsx/adb84732-9fc8-48b6-bddc-6b0bcdaad940)
