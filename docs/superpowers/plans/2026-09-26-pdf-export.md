# PDF export (STATUS rank 15, documents)

**Goal:** a PDF that matches the page preview (D05.04), with real text and an accessible structure (D05.05).

**Approach:**
- The preview's layout paints each page onto a recording canvas (`PdfRecorder`) instead of the screen.
- Text is recorded as positioned fragments, one per style, line and tab stop, with the width the layout gave it. Rectangles, rules, pictures and clips are recorded too.
- The engine's `exportPdf` op writes them with `pdf-writer`, embedding font subsets (`subsetter`, `ttf-parser`) found on the system.

### Task 1: The PDF writer (Rust)
- [x] Fonts are found by family, with Word's families mapped to sans, serif and mono substitutes. Bold and italic are synthesized when a face is missing.
- [x] Per-character fallback to any installed font with the glyph, such as CJK.
- [x] TrueType outlines are embedded as Identity-H CID fonts with ToUnicode maps, so text can be copied and searched.
- [x] Each fragment is scaled horizontally to the layout's width, so lines break and align as in the preview.
- [x] Pictures are written as RGB with an alpha mask. Rectangles, rules and clips are drawn.
- [x] Web and mail links become link annotations. Headings become outline entries.
- [x] Tags: Document, then H1–H6, P and Figure with alt text over marked content. Headers, footers and column rules are artifacts. Language, title and the structure-order tab order are set.

### Task 2: The app
- [x] `PdfRecorder` implements `Canvas`. The layout hands it text painters and pictures directly, and link text carries its target (`LinkTextSpan`).
- [x] File → Export PDF… (and the macOS File menu) lays the document out as the preview does and writes the PDF.

### Task 3: Close-out
- [x] Rust tests for the writer. Checked with pypdf and pdfminer: text extraction (accented and CJK), fonts, outline and link.
- [x] Widget tests for the recorder and the export.
- [x] Register: D05.04 and D05.05; new D05.07 for tables and lists.

**Left:** the system print dialog (D05.02); tagged tables and lists (D05.07); shaping for complex scripts; CFF-outline fonts are substituted; workbooks (X18).
