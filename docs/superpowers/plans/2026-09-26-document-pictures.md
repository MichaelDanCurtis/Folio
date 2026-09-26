# Pictures in documents (STATUS rank 7)

**Goal:** inline pictures in Word documents render in the editor and the page preview (W08.01), and a picture can be inserted from a file (W08.02). Floating pictures (`wp:anchor`) stay preserved blocks until the page-aware layout decision (risk R1, W08.04).

**Model:**
- An inline picture is an inline `image` node in the document: `{"type":"image","attrs":{"src","width","height","alt"}}`, sized in pixels at 96 dpi. The node has no text.
- `src` is the picture's part in the document's Office package (`word/media/image1.png`), and `rId` its relationship.
- An imported picture keeps its source drawing as `officeXml`, so an unchanged picture is written back byte for byte.
- An inserted picture is added to the document's package (`word/media/folio-<hash>.png`, with its relationship and content type), which gets a new package id; a native document gets a package on its first picture. History records package ids, so Undo restores the previous package.

### Task 1: Import and export inline pictures

- [x] Paragraphs whose only special content is inline pictures (`w:drawing/wp:inline` with an embedded blip) import as editable paragraphs with `image` nodes. Floating and linked pictures stay preserved blocks.
- [x] Export writes an imported picture's source run back unchanged, and a new picture as a DrawingML inline with its media part, relationship and content type.
- [x] DOCX round trip, and reimport of an exported new picture.

### Task 2: Pictures in the editor

- [ ] An engine op returns a picture's bytes (`documentImage`); the editor shows the picture at its size, scaled down to the text width.
- [ ] The picture is one character: the caret moves over it, and it can be selected, deleted, copied within the document and undone.
- [ ] Alt text is read out by screen readers.

### Task 3: Pictures in the page preview

- [ ] The page layout measures a picture's line by its height and paints it on the page.

### Task 4: Insert a picture from a file (W08.02)

- [ ] Insert › Picture chooses a PNG, JPEG or GIF file. The picture is stored in the media store and inserted at the caret, scaled to the text width, as one Undo step.
- [ ] Files over 10 MB, or whose contents aren't a supported image, are refused with a message.
- [ ] Save as `.folio`, reopen, export to DOCX and reimport all keep the picture.

### Task 5: Close-out

- [ ] Rerun the Office corpus: pictures render and documents still round-trip.
- [ ] Update the register rows, STATUS.md counts and the change log.
- [ ] Run the local CI mirror. The Flutter baseline is 7 failures that happen only on Linux.
