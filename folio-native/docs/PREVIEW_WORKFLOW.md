# Native preview hygiene

The user requires at most one Folio preview process. Build and test artifacts
may be prepared without launching them; headless Flutter tests are separate
from the desktop preview.

Before any native launch or restart:

1. Run `python3 scripts/verification/check_preview_processes.py --expected 1`
   to identify the existing preview. If the count differs, inspect the listed
   applications before proceeding.
2. Inspect that exact application through desktop controls. Preserve unfinished
   edits, verify its saved state, and quit through the normal application menu.
3. Confirm `--expected 0` before opening one verified candidate.
4. Confirm `--expected 1` afterward and record the exact app path and bundle ID.

If the zero-process check fails after Quit, return to that same application
and inspect its Save/Cancel dialog. Finish saving through the normal UI and
check zero again. A locally saved recovery or completed Export can still leave
the working file unsaved; neither permits bypassing the normal quit dialog.

Use the same candidate for a restart check. A timeout or missing accessibility
tree is not evidence that the process exited. Do not create or launch another
copy merely because observation failed. If the Mac is locked, leave the
existing preview untouched and continue independent implementation or automated
checks. Resume the saved-state inspection when desktop access returns.
