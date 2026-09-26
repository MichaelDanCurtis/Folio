# Several open files (STATUS rank 16, desktop)

**Goal:** several documents and workbooks open at once with independent save state (D01.07), and the open files and window restored after a restart (D01.09).

**Approach:**
- Tabs in the one window, as macOS apps do.
- Each tab is a full workspace screen over its own engine session, with its own recovery file. Inactive tabs stay alive but offstage, without focus or animation.

### Task 1: Engine sessions (Rust)
- [x] `dispatch` routes a request by its `session` to that session's engine. Requests without one use the first engine, so existing callers are unchanged.
- [x] `closeSession` frees an engine. At most 64 are open.
- [x] A long operation in one session doesn't block the others.

### Task 2: Tabs (app)
- [x] `CoreBridge(session:)`.
- [x] `WorkspaceTabs` hosts the tabs:
  - The first tab keeps the data folder's `workspace.folio`. Others use `tabs/<id>/workspace.folio`.
  - Recent files, templates, recovery copies and proofreading settings are shared by all tabs.
  - The tab strip shows only when more than one file is open.
- [x] New Tab (⌘T), Open in New Tab… (⌥⌘O) and Close Tab (⌥⌘W), in the File menu, the macOS menu bar and the strip.
  - A tab left blank by a cancelled Open closes.
  - Closing a tab asks to save its files first. The last tab stays open.
- [x] Only the tab in front owns the native File and Edit menus.
- [x] Quitting asks each tab with unsaved work in turn, bringing it to the front.

### Task 3: Restore (D01.09)
- [x] `tabs.json` lists the open tabs and the tab in front, and they reopen at launch. Unreadable or missing tabs are skipped.
- [x] The window frame is autosaved by AppKit (`setFrameAutosaveName`).

### Task 4: Close-out
- [x] Rust test for independent sessions.
- [x] Widget test: new tab, independent engines, menu ownership, restart restore, and close with Don't Save removing the tab's files.
- [x] Register: D01.07 and D01.09 to Automated verified.

**Left:**
- Separate windows for one file (D01.08).
- Dragging tabs to reorder.
- A native check of the tab strip, menus and window frame on the Mac.
