# Section breaks

Version 0.65 adds Layout > Section breaks. Place the caret in a document
paragraph and choose Next page, Continuous, Even page or Odd page. The operation
splits that paragraph, retains its run formatting and hyperlinks, copies its
section settings and links the following section's headers and footers to the
previous section. Page setup can then change the selected section separately.

Remove identifies a boundary by its two section numbers and explains the result
before Apply: the combined section uses the following section's page settings.
Adjacent text paragraphs join. Before a table, the paragraph remains and only
its section property is removed. The final document section cannot be removed.
Effective headers and footers are retained when their explicit predecessor is
removed. Cancel has no effect; applied changes participate in Undo, Redo, saved
workspace recovery and DOCX export.

The behavior follows Microsoft's descriptions of [inserting section breaks](https://support.microsoft.com/en-us/word/insert-a-section-break),
[deleting a section break](https://support.microsoft.com/en-us/word/delete-a-section-break)
and [Link to Previous](https://support.microsoft.com/en-us/word/link-to-previous).
These references are a behavior target; they are not evidence of execution in
Microsoft Word.

## Boundaries

- The editor still uses a continuous view with Section 1's geometry. Physical
  pages, per-section on-screen geometry and full pagination are incomplete.
- Recording insertion or deletion of a section break is unsupported. Recording
  must be turned off explicitly; Folio never disables it as a side effect.
  Imported recording requests still require an explicit author choice.
- Protected documents, pending changes in affected paragraphs/section settings,
  unsupported inline objects, table cells and live workbook blocks cannot be
  split. Pending recorded content may require review before insertion.
- All four start types are stored in DOCX. Page-number restart, odd/even blank
  page behavior, different first-page headers and differential Word layout need
  separate acceptance. Columns, full header/footer authoring and continuous
  section rendering remain open.

## Verification

Focused engine checks cover styled text and hyperlinks, UTF-16 boundaries,
emoji, tabs and hard breaks, start/end/empty paragraphs, existing boundaries,
continuous geometry inheritance, inherited headers/footers and following-section
geometry. Workspace checks cover pending ordinary edits, exact Undo/Redo,
live workbook bindings, comments/bookmarks after the live block, point affinity,
file recovery and later live-cell export. Invalid positions, stale paragraph
keys, protection, recording and preserved objects are rejected.

Flutter tests exercise the actual Layout dialog with the Rust engine, selected
range rejection, Cancel, insertion, section selection in Page setup, selected
section margins, removal, Undo/Redo, export and recovery. Dialog checks cover
start choices, removal consequences, section selection and disabled states.

Native Continuous insertion/removal, second-section margin editing, Save As,
keyboard Undo/Redo, post-Save Undo and byte-identical restart pass in the isolated
0.65.0 build 1 preview. Native files retain source headers/footers, hyperlinks and
bookmark ranges. 336 Rust and 275 Flutter tests pass, with two optional AI checks
skipped; static analysis is clean. Independent XML and Microsoft365 schema
checks pass for the scoped artifacts. See VALIDATION.md and
`release/validation/section-breaks` for evidence. Full Word/Excel parity and the novel-AI milestone remain incomplete.
