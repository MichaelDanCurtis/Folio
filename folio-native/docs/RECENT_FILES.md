# Recent files and recovery — 0.55 development

File → Recent Files and Recovery, and the sidebar history button, open a shared
browser for Folio workspaces, Word documents and Excel workbooks. Search matches
names and folders; file-type and pinned filters narrow the list. Successful Open,
Save, Save As, Save Copy and Export record a location. Pinning retains an entry;
Remove and Clear unpinned change only the list, never the files. There are up to
50 unpinned entries, plus all pinned entries. History is local to this installation.

Recent locations are separate from the active document/workbook save destinations.
New, importing another file and closing without saving do not clear the history.
The first migration reads the application-owned previous save locations before a
blank restart initializes its own metadata. Removed entries do not reappear on the
next load. Last-used dates are recorded only for actual successful file operations;
migrated entries show “Saved location.” Pins survive a bookmark-resolved rename
or an explicit Locate operation. A missing/inaccessible entry offers Locate or
Cancel. Cancel and unsuccessful opens leave its last-used time unchanged.

macOS bookmarks are resolved for the duration of an open and released afterward.
File contents cannot supply grants. Opening from history uses the same pending-save
decision and file transaction as Open; changed content and cancelled first saves
are protected. The editing surface is frozen while the browser and its follow-up
dialogs are open. A history write error does not invalidate an already completed
file save. Damaged metadata is retained and reported, rather than silently replaced.

Recovery copies are retained before New, Open and Close with Don't Save. Each new
copy goes into an OS-created unique directory; workspace IDs and titles never
become path components. This fixes the old fixed-name snapshot overwrite: selecting
an earlier copy cannot overwrite it while preserving the outgoing workspace.
Legacy `.folio` snapshots remain listed. Sidecar labels are optional; missing or
damaged labels do not hide valid copies. Symlinks are not followed by the catalog.
Selecting a copy validates/hydrates it in an isolated engine and previews the
document's first 2,000 characters and worksheet names. Stale preview results cannot
replace the current selection. A damaged selected file cannot be restored.

Restore validates again, preserves current work separately, and assigns the restored
workspace a fresh ID. No previous external Save destination is inherited. The
restored workspace is an unsaved whole-workspace draft: Save chooses a new `.folio`
destination, preserving both the document and all worksheets. Cancel keeps it
unsaved. Ordinary Open of a file within the recovery directory uses this same
restore behavior. Snapshots are never bound as active save destinations by Open.
Word/Excel exports remain available separately. The recovery source is unchanged.

The initial unnamed-content baseline is captured synchronously at startup, before
history migration; typing during metadata loading therefore remains unsaved.

## Requirements and remaining qualification

The local recent/pin/remove behavior follows Microsoft's documented
[recent-file workflow](https://support.microsoft.com/en-us/office/collab-files/customize-the-list-of-recently-used-files-in-office-apps).
Microsoft 365 also has Favorites and account-synchronized recent lists; these are
not implemented. Its configurable list length and disable-history option remain
open. Microsoft's [earlier-version recovery workflow](https://support.microsoft.com/en-us/word/recover-an-earlier-version-of-a-word-file)
also includes saved/unsaved versions. Folio currently browses leave-operation copies;
this is not full timed version history, cloud versioning or crash-session management.
There is no recovery retention/deletion UI yet, so snapshots accumulate.

Native sandbox persistence across process restart, actual moved-file bookmark
resolution, accessibility/keyboard review, small-window layouts, physical IME and
Microsoft Office comparison remain separate gates. Automated widget tests mock OS
pickers and bookmarks; in-memory AppKit tests do not operate the desktop. Advanced template behavior,
Finder associations, multi-window editing, cloud/provider coordination and other D02
requirements remain incomplete. No entire parity family or novel AI feature is
accepted by this increment.
