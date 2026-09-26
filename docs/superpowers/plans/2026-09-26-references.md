# References: notes, captions, cross-references and contents (STATUS rank 14)

**Goal:** footnotes and endnotes that can be edited (W11.01); captions (W10.03), cross-references (W10.04, W10.08) and tables of contents (W10.05) that Folio numbers and Word keeps.

**Finding:** footnote and endnote references were dropped on import. A paragraph holding one stayed editable, and editing it deleted the reference from the saved file.

### Task 1: Footnotes and endnotes
- [x] `w:footnoteReference` and `w:endnoteReference` import as inline `noteRef` nodes, with the note's text.
- [x] An unchanged reference writes its source run.
- [x] Before export, new references get ids and a note in `footnotes.xml` or `endnotes.xml`. The part, its relationship, content type and separators are created when missing.
- [x] Edited notes are rewritten as plain paragraphs, keeping the first paragraph's properties.
- [x] Validation refuses a source run that isn't exactly its reference.
- [x] In the editor: References → Insert footnote or endnote. The mark numbers itself live (1, 2, 3; i, ii, iii); clicking it edits the note.
- [x] The page preview shows the mark. Placing notes at the page bottom is W11.05.

### Task 2: Fields for captions and cross-references
- [x] `SEQ`, `REF` and `PAGEREF` instructions, with their switches, import as editable fields like page fields.
- [x] Update Fields numbers SEQ fields: formats, `\r`, `\c`, `\s` and `\h`.
- [x] REF shows the bookmarked text, and PAGEREF its page. A missing bookmark shows “Error! Reference source not found.”
- [x] Page numbers come from the app's layout. Headings map by order, and bookmarks by name, since export keeps names.

### Task 3: Inserting references
- [x] `documentReferences` inserts a caption paragraph (Caption style, `Figure 1: …`) after the cursor's paragraph.
- [x] It inserts a REF or PAGEREF at the cursor, adding Word's hidden `_Ref` bookmark around a heading or a caption's label and number.
- [x] It inserts a table of contents before the cursor's block.
- [x] Each is one undo step. Bookmarks and comments are rebased by text.
- [x] A table of contents is a `tableOfContents` block. It is written as Word's TOC field in a content control, with TOC 1–3 styles, and read back from one.
- [x] The editor shows it with Update table. The preview lays out its entries with dot leaders.

### Task 4: Close-out
- [x] Rust tests for notes, fields, insertion, round trips and the op. Widget test for the menu flows.
- [x] Compatibility table: notes editable; fields and tables of contents updated.
- [x] Register and corpus.
