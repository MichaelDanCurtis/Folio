# Document review and native history — 0.64 development

Command-Z and Command-Shift-Z retain document history after opening review,
accepting or rejecting a change, and returning from Save or Save Copy. The
workspace owns a focus scope, so removing a focused review button or closing a
file panel cannot leave shortcuts outside the workspace.

The Mac Edit menu follows the current history owner. A document or worksheet
uses the ordered workspace history, including ordinary typing, review decisions
and committed cell edits. An assistant prompt, formula bar, inline cell editor,
search field or dialog field uses its own draft history. Empty, read-only or
composing drafts do not fall through to the document. Menu commands carry their
focus-context generation; a delayed command from another field is rejected.
Attached native file panels restore AppKit's responder-chain Undo and Redo.

Page-formatting review lists only changed settings, including paper dimensions,
margins, gutter, header/footer distances, orientation and section start. Geometry
uses inches; unrecognized source values remain distinguishable. Printer-specific
paper settings retain their code when one changes or is removed. Microsoft's
[PageSize.Code definition](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.pagesize.code?view=openxml-3.0.1)
identifies this attribute as a printer paper code; no printer-specific code-to-name
mapping is assumed.

## Verification

A regression test reproduced the disappearing-review-control focus failure
before the focus change. Real-engine widget tests cover reject, keyboard Undo
and Redo, Save Copy and continued review history, native-channel Undo/Redo,
assistant drafts, stale focus commands and dialog Cancel. The field-history
suite exercises both keyboard and native-channel paths through search,
conversion dialogs, formula and inline cell drafts, grid history, Quill history,
cell commit and a fresh field after commit.

The extracted production Swift menu is checked for enabled states, context
forwarding, exact shortcut modifiers, native sheet fallback, hidden windows and
preservation of unrelated Edit commands. These automated checks do not replace
packaged native interaction. Native qualification and package identity are
recorded in VALIDATION.md. The isolated package passes review Undo/Redo after
Save, native menu Undo/Redo, focused assistant/dialog/formula histories, committed
workbook history, DOCX/XLSX saves and byte-identical restart.

Native filename edits undo and redo through Edit-menu clicks. Automated
Command-Z and Command-Shift-Z sent to the open AppKit Save panel did not change
its filename; the cause and physical-keyboard behavior remain unverified. A
batched automated prompt typing-plus-Undo call left its last character as a
separate undo step; a second Undo cleared it, with no workspace change. Settled
draft menu/keyboard history passed. These narrow input-timing cases remain open.

Full Word/Excel parity, complete pagination, broad IME and accessibility coverage,
multiple document windows and Microsoft Office differential validation remain
open. This work does not count toward the novel-AI milestone.
