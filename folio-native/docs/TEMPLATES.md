# Word and Excel templates — 0.56 development

File → New from Template opens a searchable gallery for Word and Excel. Six
Folio-authored starters are included: Business letter, Project report, Meeting
notes, Budget, Invoice and Project tracker. They contain editable text/placeholders,
styles and tables, or formulas, totals, currency formats, frozen headers and status
validation. The gallery previews document text or the first worksheet's upper-left
seven rows/four columns. It is a content preview, not a page-layout thumbnail.

Browse Templates adds user-selected `.dotx` and `.xltx` files to a persistent
personal library. The library records local sandbox grants, separately from saved
workspace contents. Missing or inaccessible locations offer Locate. Removing an
entry removes only the library reference; the source file stays intact. Search and
Word/Excel filters work across bundled and personal entries. Concurrent starter
previews use separate temporary copies, released after each read. Late preview
results cannot replace the currently selected template.

**Create** starts an independent workspace with a new identity and no external
Save association. Current pending changes are handled by the existing Save/Don't
Save/Cancel flow; outgoing work is preserved in recovery. Only the template's part
starts as unsaved: a Word template asks for a `.docx` first save, and an Excel
template asks for `.xlsx`. The other part starts blank. New document/workbook
metadata uses a normal Office content type. The template remains untouched.
Template bytes are checked against the preview fingerprint before replacement;
an intervening source change requires another preview. Invalid templates leave
the current model, Undo and save locations intact.

**Save as Template** writes the active view to `.dotx` or `.xltx` through the native
Save picker and adds it to the personal library. This writes a separate template;
it neither retargets a saved document nor marks that document's pending changes
saved. **Edit template**, available for personal templates, explicitly opens it
for editing. Ordinary Open also accepts these formats for editing. Save writes to
the template, Save As retains its format, and active content signatures, external
change checks, bookmark refresh, recent-file history and Undo behavior are shared
with ordinary files. A Save as Template operation targeting the already-active
template refreshes that association rather than creating a spurious conflict.

The engine changes the main-part content type through parsed namespace-aware XML.
Other package entries remain unchanged by that conversion. A `.folio` recovery
retains the normalized source package for a newly created file, or the template
package for an explicitly edited template. Renaming an extension alone is never
used as conversion. Macro-enabled content, disguised VBA/macrosheets and ambiguous
main-part types are rejected; original files remain untouched.

## Requirements and remaining work

Microsoft documents distinct [create, save and edit template workflows](https://support.microsoft.com/en-us/word/create-a-template),
including `.dotx` and `.xltx` for ordinary templates and separate macro formats.
Its Open XML guidance requires the [package type to match the extension](https://learn.microsoft.com/en-us/office/open-xml/spreadsheet/how-to-create-a-spreadsheet-document-by-providing-a-file-name).
These guide the workflows above; they are not a full template specification.

Template inheritance/attached-template updates, global and Normal templates,
building blocks/content controls, `.dotm`/`.xltm`, older binary template formats,
Excel sheet templates and startup defaults, online catalogs and shared/cloud
libraries remain unimplemented. Source objects retain the existing DOCX/XLSX
editable-versus-preserved limits; a source part retained in an exported template
does not prove editable parity. Multi-window creation, full layout thumbnails,
native sandbox/keyboard/accessibility review and actual Microsoft Office comparison
remain separate qualification gates. No entire D02/W02 family is accepted here;
the full Word/Excel goal and subsequent novel-AI phase remain incomplete.
