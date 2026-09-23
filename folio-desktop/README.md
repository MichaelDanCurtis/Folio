# Folio desktop — first prototype

A local Mac app with a rich-text document, a multi-sheet workbook, and an integrated Codex assistant. Built with Electron, TypeScript, Tiptap, and a bundled `codex app-server`.

## Run

```sh
npm install
npm start
```

The packaged Apple Silicon app is under `release/Folio-darwin-arm64/Folio.app`. It is a local development build, not a signed/notarized distribution.

## What works

- Document headings, bold/italic/underline, lists, tables, and editable workspace title.
- Workbook with multiple sheets, 60 rows × 12 columns per sheet, direct cell/formula-bar editing, tab-separated paste, and a simple column-B chart.
- Formula parser supporting arithmetic, parentheses, same-sheet cell references, SUM, AVERAGE, MIN, MAX, COUNT, ranges, error propagation and cycle detection. No JavaScript evaluation.
- Shared undo/redo across document edits, sheet edits, and atomic AI changes.
- Automatic local workspace recovery, native Save a copy/Open dialogs, HTML document export and calculated-value CSV export.
- Codex connection, existing account detection, ChatGPT sign-in, streamed responses, stop, live workspace reading and proposed editing tools.
- AI proposals show the actual text/cell changes. Apply checks the workspace revision, changes both editors atomically, and adds an undo step. Any intervening user edit invalidates a proposal.

## Codex integration

Click **Connect** in the assistant panel. Folio prefers its bundled Codex 0.154.0 native runtime, then falls back to `~/.local/bin`, Homebrew paths, or PATH. Set `FOLIO_CODEX_PATH` to override its location. Codex owns credentials; the renderer never receives tokens. Existing Codex sign-in is reused; otherwise, the app opens the managed ChatGPT login flow. Usage is subject to the signed-in account's Codex limits.

The app launches a child app-server over stdio, performs the initialize handshake, and starts an ephemeral conversation on the first request. It registers `read_workspace` and `propose_changes` dynamic tools and routes calls to the live editor. A ChatGPT request may send the document and workbook contents to OpenAI; the UI states this before use.

The prototype sets a read-only sandbox, disables execution environments for its thread, declines native command/file approvals, and instructs the assistant to use only its workspace tools. This is a development prototype, not a hardened multi-tenant isolation boundary. The installed user's Codex configuration may affect capabilities and login.

The integration was generated/checked against the bundled Codex CLI 0.154.0 protocol. Dynamic tools and app-server are experimental: pin/test a CLI version before shipping. The UI currently keeps its chat and pending proposals in memory; documents persist, but chat history resets after relaunch.

## Data and formats

Workspace files use a versioned JSON `.folio` format. Automatic recovery lives in Electron's Folio user-data directory (normally `~/Library/Application Support/Folio/workspace.folio`). Saves are serialized and use a temporary file plus rename. Export HTML preserves the supported rich-text structure; CSV exports the current sheet's calculated values, not formulas.

**Not implemented:** DOCX/XLSX import/export, Word-compatible pagination, images, advanced spreadsheet formatting, Excel's full formula language, cross-sheet references, macros, collaboration, or production distribution. Do not use the prototype as the only copy of important documents.

## Verify

```sh
npm test
npm run build
npm run test:desktop
FOLIO_LIVE_TEST=1 npm run test:desktop
npm run package:mac
```

The optional live test uses a temporary sample workspace and sends one actual Codex request, which consumes account usage. It verifies reading, proposing a cross-editor change, review-before-apply, and atomic undo. Native tests use temporary user data rather than the working app's documents.

## Layout

- `electron/main.cjs`: native shell, narrow IPC handlers, file dialogs and storage.
- `electron/preload.cjs`: isolated renderer bridge.
- `electron/agent.cjs`: bidirectional app-server transport and tool lifecycle.
- `src/model.ts`: workspace schema, formula evaluation, change validation.
- `src/main.ts`: editors, assistant panel, proposal review and undo.
- `tests/`: domain tests and real Electron integration checks.

## References

- [OpenAI app-server](https://learn.chatgpt.com/docs/app-server)
- [Electron context isolation](https://www.electronjs.org/docs/latest/tutorial/context-isolation)
- [Tiptap editor](https://tiptap.dev/docs/editor/getting-started/install/vanilla-javascript)
