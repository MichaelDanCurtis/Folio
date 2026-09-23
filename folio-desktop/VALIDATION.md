# Prototype validation — September 17, 2026

- TypeScript check and production renderer build: passed.
- Six domain tests: passed, including formula recalculation, parser errors/cycles, bounded shared dependencies, atomic proposal application, stale revision rejection, workspace validation, and CSV escaping.
- Real Electron app: document and cell editing, formula recalculation, undo/redo, chart, autosave, and relaunch recovery passed without renderer errors.
- Native file handlers: `.folio` save/open, HTML export, and calculated CSV export passed using temporary files.
- Real Codex 0.154.0 app-server: connected using the existing account; a model read the sample document/workbook and proposed a cell change plus a paragraph. Content remained unchanged before Apply. Applying changed both editors; one Undo reversed both.
- The older global CLI 0.142.5 connected but was rejected by the model service as outdated. Folio now bundles 0.154.0; the global installation was not changed.
- Dependency audit after updating the packaging library: no known advisories reported.
- Final packaged adapter successfully launched its bundled Codex executable and detected the signed-in account.
- Apple Silicon `.app` packaged successfully and opened on this Mac. This is an unsigned local development build; signing/notarization and broader compatibility testing remain.

No DOCX/XLSX compatibility, complete Excel formula coverage, production security certification, or distribution readiness is claimed.
