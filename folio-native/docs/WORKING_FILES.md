# Save and Save As

Version 0.54 adds Save (Command-S / Control-S) and Save As
(Command-Shift-S / Control-Shift-S) to the toolbar, File menu and native macOS
menu. These follow the opened or saved working file. Export and Save Workspace
Copy remain separate operations and do not change where the next Save writes.

A new document first saves as DOCX, and a new workbook first saves as XLSX.
Opening or saving an Office file associates it with that view; the document and
workbook can retain different file locations. Opening a `.folio` file associates
the full workspace with that location for both views, until an Office file is
opened/saved for a particular view. Save As preserves the original and changes
the active association only after writing the new file. Cancel retains the
previous location. An association change is reserved locally before writing a new destination; an
interrupted update cannot restore an old target after restart. New clears associations; opening another `.folio` replaces
them. Undoing an Office import cannot make Save overwrite that now-unrelated
Office file: associations must match the current source lineage. Package revisions
created by style replacement, paste/fill, table and worksheet structure edits retain
that lineage; importing a different file establishes a new lineage.

The existing local recovery file and outgoing-workspace snapshots continue to
hold the full workspace, even when the working file is DOCX or XLSX. File access
and location metadata are stored only in the app's local `working-files.json`;
imported documents, workspace files and AI proposals cannot supply these grants.
On macOS the app retains security-scoped bookmarks for user-selected files,
resolves them while saving, refreshes them after replacement, and balances scoped
access when the operation ends. A renamed file can resolve to its new location.

Save waits for pending document, worksheet and history edits. The core records
the revision, SHA-256 of the actual saved bytes, and content signature from the
same engine state, so later typing is not marked saved accidentally. The header distinguishes saved working files from changes
that are currently only in local recovery. A long filename is truncated visually
and available in the tooltip.

Before replacing a working file, Folio checks its SHA-256 against the last
opened/saved version. A changed file offers Cancel or Save As to keep both
versions. The engine checks again after constructing the output, immediately
before atomic replacement. Failed writes leave the previous data intact. Missing
or inaccessible files require choosing a location again with Save As.

[Microsoft's Word save instructions](https://support.microsoft.com/en-us/accessibility/word/use-a-screen-reader-to-save-a-document-in-word)
provide the Save/Save As shortcut baseline.
[Apple's sandbox file-access guidance](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox)
describes the user-selected grants and security-scoped bookmark lifecycle.
These references do not establish full Office file-workflow equivalence.

## Verification and remaining work

Engine, real-workspace editor, independent-reader and native package evidence is
recorded in `VALIDATION.md`. The application regression covers Cancel, repeated
Save, Save As preserving the original, Copy retaining the association, separate
Office files, recovery of associations, renamed-file resolution, outside changes,
failed saves, native-workspace saves, and New. OS access is mocked in the editor
test; the packaged sandbox restart check is a separate gate.

Build 2 replaces revision-based change indicators with separate content signatures.
DOCX includes the resolved document and preserved source parts: unrelated workbook
edits stay clean, while linked source values and number formats mark it changed.
XLSX includes worksheet content, settings, calculation results/errors and preserved
source parts. `.folio` includes the whole workspace except its monotonic revision.
Undo back to saved contents therefore shows Saved again. Immutable source packages
are hashed once and cached; content is streamed into the digest without constructing
another serialized copy. Older local associations without a signature use the
conservative revision comparison until the next Save or Open.

Before New, opening a replacement file, and Close/Quit, Folio asks whether to
save affected named files and changed unnamed documents/workbooks. Opening DOCX checks the document
and any whole-workspace file; opening XLSX checks the workbook and whole-workspace
file. Unrelated Office edits remain in the workspace. Save processes each affected
file, using its own path even if the other view is active. Cancel, cancelled Save As,
conflicts and failed saves stop the transition. Edits arriving during saving are
checked again. Repeated close/menu requests and dialogs cannot bypass the decision.
The macOS handler requires an explicit true result and propagates Cancel.

Build 3 tracks the initial contents of each unnamed view in local session metadata.
An unchanged new workspace does not trigger a first-save prompt. Typing in Word
prompts for DOCX; workbook edits prompt for XLSX, independently of which view is
visible. If both changed, Save handles both. Cancelling the second picker leaves
the first file saved and keeps the workspace open; the next attempt asks only for
the remaining unsaved part. Undo to the original contents clears the pending state.
Older recovered work without this baseline is conservatively treated as unsaved.
A named `.folio` covers both views. The header identifies locally recovered edits
that have not yet been saved to a file.

**Don’t Save discards the work being closed and leaves external files unchanged.**
New/Open keep an outgoing recovery snapshot while replacing the relevant content.
On Close/Quit, the entire workspace closes: a recovery snapshot is written, then
the automatic reopen file is atomically replaced by a new blank workspace. The
next launch therefore cannot silently revive discarded edits. The actual live
model and Undo history stay intact until the OS closes the window; if the blank
write fails, Close is cancelled and those edits remain available. A queued autosave
checks the closing state inside the engine transaction queue, so it cannot restore
the discarded workspace after that decision. Stale associations are rejected by
the new workspace ID. Saved files remain available through Open; the discarded
snapshot can also be restored through Recent Files and Recovery. Restoration creates a new unsaved workspace and never binds Save to the snapshot.

The editing surface is disabled while New/Open/Close is pending; dialogs remain
operable, and Cancel restores editor focus. Ordinary Save continues to accept
editing, with content signatures preserving later edits as unsaved. Focus changes
are allowed to finish before pending edits are flushed. Native sandbox, physical
IME and on-screen confirmation remain separate qualification gates.

These file associations do not complete D02 or file compatibility. Version 0.55 adds local recent-file and recovery browsing (see `RECENT_FILES.md`).
Advanced template behavior/full version browsing, Save As format conversion UI, file associations
and Finder Open With, multiple windows, and complete
native cloud/provider/file-lock coordination remain unfinished. A change by an
uncoordinated writer between the final fingerprint check and atomic replacement
is not transactionally excluded. Signatures represent editable/export source state,
not byte-equivalence of a future ZIP or Microsoft Office rendering. Windows/Linux
and physical IME qualification remain open.
The existing DOCX/XLSX supported/preserved-feature limits still apply.


Version 0.56 adds DOTX/XLTX Save as Template and explicit template editing.
See `TEMPLATES.md` for Create versus Edit, first-save behavior, gallery/library
scope and remaining template requirements.
