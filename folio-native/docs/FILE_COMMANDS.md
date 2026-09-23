# Desktop file commands

Build 11 adds a native macOS File menu alongside the existing Edit, View and
Window menus. New Workspace (Command-N), Open (Command-O), Save Workspace Copy
(Command-S), Export Word/Excel (Command-Shift-E), and Compatibility Report use
the same commands as the ribbon and sidebar. The export label follows the active
Word/Sheets view. Flutter also supplies Control equivalents for desktop hosts;
Windows/Linux native packaging and input qualification remain pending.

Open accepts `.folio`, `.docx` and `.xlsx`. New/Open save the outgoing workspace
in the existing local Snapshots folder before replacing it. Open cancellation
and failed reads leave the current workspace intact. Save Workspace Copy saves
the complete document/workbook workspace, while Export writes the active Office
file type. All queued document, worksheet and history edits finish before file
operations. Save Copy rechecks pending work after the native panel returns.

Only one file operation can run at a time. Repeated keys, native menu actions and
ribbon/sidebar clicks cannot stack dialogs or replace the workspace during a
pending file operation. Flutter dialogs disable file commands while their drafts
are open. The native menu also disables them while an AppKit sheet is attached.
New/Open are unavailable while the assistant is busy. Cancellation and errors
release the command gate. Workspace rename fields own their controllers through
the complete dialog lifetime, including the closing animation.

The menu uses AppKit's existing [NSMenuItem target/action and keyboard equivalents](https://developer.apple.com/documentation/appkit/nsmenuitem)
and [modifier masks](https://developer.apple.com/documentation/appkit/nsmenuitem/keyequivalentmodifiermask).
Existing native Edit actions and Flutter field-specific Undo remain in place.

## Evidence and limits

`test/workspace_file_commands_test.dart` drives the actual workspace and Rust
engine through keys and the native command channel. It covers pending Undo plus
new typing before Save, duplicate command suppression, Open Cancel and failure,
modal isolation, snapshot-before-replacement, DOCX import/export, inline cell
commit before XLSX export, New and exact saved-workspace recovery.
`scripts/test_macos_file_menu.py` exercises the production AppKit menu in memory;
`scripts/verification/verify_file_commands.py` independently reads editor exports.
Native package results are recorded in `VALIDATION.md`.

This records the build 11 release. Version 0.54 development adds associated-file
Save/Save As; see `WORKING_FILES.md` for its qualification and remaining gaps.
This does not establish full desktop file parity. Recent files, a snapshot/version browser, templates, Finder Open
With/file associations, multiple document windows, and complete File/Edit menus
remain incomplete. Native IME and physical-device/layout coverage also remain
open. No parity family or novel AI feature is accepted by this increment.
