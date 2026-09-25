# Document editing essentials (STATUS rank 6)

**Goal:** the everyday Word commands a daily-workflow user reaches for, built and automated-verified here, with native checks joining the Mac sweep (rank 3).

**Children:** W01.13, W01.14, W12.09, W17.02, W17.07, W07.06, D03.10 and X04.12.

**Approach:** each command is a pure function over the Quill document or controller where possible, with widget tests through the real editor and DOCX round-trip tests where the file format is involved.

### Task 1: Change Case and Clear Formatting (W01.13)

- [x] **Change Case:**
  - Sentence case, lowercase, UPPERCASE, Capitalize Each Word and tOGGLE cASE, on the selection;
  - Shift+F3 cycles UPPERCASE, lowercase, then Capitalize Each Word, as Word does.
- [x] **Clear All Formatting:**
  - removes character formatting from the selection and returns its paragraphs to Normal;
  - keeps text, links, comments and tracked-change marks.
- [x] Each command is one Undo step and round-trips through DOCX.

### Task 2: Hyperlink UI (W01.14)

- [x] **Insert and edit** with Ctrl/Cmd+K: text to display, and an address or a place in this document (a heading or bookmark).
- [x] **Remove the link**, keeping its text.
- [x] **Follow** with Ctrl/Cmd+click, or from the context menu:
  - an external link asks the platform to open it;
  - an internal link moves the caret to the target.
- [x] Only http, https and mailto addresses are followed.

### Task 3: Word Count dialog (W12.09)

- [x] Pages (from the page layout), words, characters with and without spaces, paragraphs and lines, for the document or the selection.
- [x] A checkbox includes text boxes, footnotes and endnotes.
- [x] The status-bar word count opens the dialog.

### Task 4: Editor zoom (W17.02)

- [x] **Zoom range and controls:**
  - 10–500% in the page view (Folio has no separate Draft view yet);
  - a status-bar slider, Command/Ctrl with + and −, and presets including Page Width and Whole Page.
- [x] Hit testing, selection and the caret stay correct at every zoom.
- [x] The zoom is kept per document for the session. Word doesn't save it in the file.

### Task 5: Formatting marks (W17.07)

- [x] **Show/Hide ¶** (Ctrl/Cmd+Shift+8) shows:
  - paragraph marks;
  - spaces as middle dots;
  - tabs as arrows;
  - line breaks.
- [x] The marks are display-only: they are never copied, saved or counted.

### Task 6: Table drag-resize (W07.06)

- [x] Drag a column border to resize the columns on either side.
- [x] Drag a row's bottom border to set its height.
- [x] Each resize is one Undo step, and round-trips through DOCX.

### Task 7: Format Painter (D03.10, X04.12)

- [x] **Documents:**
  - copy the character and paragraph formatting at the selection;
  - the next selection receives it;
  - a double-click keeps the painter on until Esc.
- [x] **Workbooks:** the copied cell formats (font, fill, borders, number format, alignment) apply from the next clicked cell over the copied range's size, as Paste Formatting does; a double-click keeps the painter on until Esc.

### Task 8: Close-out

- [x] Update the register rows, STATUS.md counts and the change log.
- [ ] Run the local CI mirror. The Flutter baseline is 7 failures that happen only on Linux.
