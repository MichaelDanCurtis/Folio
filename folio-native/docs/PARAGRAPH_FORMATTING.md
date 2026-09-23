# Paragraph formatting

Home → **Paragraph…** edits selected paragraphs in the document. The cell editor
has the same control; its edits remain drafts until **Apply changes** commits the
table. The dialog provides left/right indents and before/after spacing in points,
plus Single, 1.5 lines, Double, Multiple, Exactly and At least line spacing.
Outline level adds levels 1–9, Body text and Use style without changing the
paragraph's appearance; see `OUTLINE_LEVELS.md`. Only changed settings apply across mixed selections. Cancel leaves the workspace
unchanged; document Apply is one Undo operation. A changed document/revision
invalidates an older preview.

The renderer keeps source characters, rich spans and selection offsets intact.
Positive horizontal indents constrain wrapping; neighboring paragraph gaps use
the larger of after/before spacing. Automatic line spacing uses 240ths of a line;
exact and minimum spacing use twips. Custom/default tabs remain relative to the
text margin when the left indent changes. Enter and multiline insertion carry
paragraph settings while preserving a unique source-paragraph identity.

DOCX import resolves document defaults, numbering-level properties and paragraph
style inheritance separately from direct formatting. Edits patch only changed
attributes, retain unmodeled XML, and keep source style parts unchanged. Legacy
workspaces hydrate missing paragraph metadata without replacing explicit edits.
New native paragraphs materialize their current spacing on first format edit.

First-line/hanging and character-unit indents, negative indents, automatic and
line-unit paragraph spacing, contextual spacing, bidi/mirrored indents and
document-grid behavior are retained but are not fully rendered. The dialog
identifies those preview limits and disables conflicting unit controls. Named-style editing refreshes the modeled inherited properties; full theme,
script, character/table/numbering-style cascades remain a separate track. Pagination, line breaking
against Word, font substitution and complete native input/accessibility coverage
remain unfinished. These controls do not establish Word parity.

The input fixture is generated independently with python-docx, then uses an
alternate main-document namespace prefix and foreign attributes. Rust tests,
real-core Flutter app tests and `scripts/verification/verify_paragraph_format.py`
cover import/export, model validation, layout, drafts, undo, recovery and reimport.
The Python reader does not execute Microsoft Word. See `VALIDATION.md` for dated
results and packaged/native qualification.

Reference semantics:

- [Word indentation properties](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.indentation?view=openxml-3.0.1)
- [Word paragraph and line spacing](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.spacingbetweenlines?view=openxml-3.0.1)
- [Tab positions relative to text margin](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.tabstop.position?view=openxml-3.0.1)
