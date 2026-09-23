# Paragraph outline levels

Paragraph → **Outline level** sets levels 1–9, **Body text**, or **Use style**.
The control is shared by the main document and table-cell editors. Only changed
settings apply to mixed selections. Cancel leaves the document unchanged; a
committed change is one Undo step. Table edits remain drafts until outer Apply.

The direct setting is stored separately from the paragraph's named style and
its inherited outline level. A direct Body text setting suppresses the inherited
structure; Use style removes the override. Style Apply, Modify and replacement
retain direct overrides. The document model and heading navigation use the
effective level. Enter and multiline insertion retain direct paragraph settings
while continuing the configured following style. This does not assign a list
number or change list nesting.

Outline changes preserve font, size, rich marks, paragraph layout and characters.
The renderer resolves named typography against ordinary paragraph defaults,
instead of letting the structural level supply a second heading appearance.
Native paragraphs without imported metadata retain their existing heading/body
appearance when first given an explicit outline setting. That first edit also
retains the native style catalog, so clearing the outline and later choosing a
visual heading continues to resolve the correct style. Source-identity matching
uses one lookup map per container rather than repeated sibling scans on typing.

DOCX uses `w:outlineLvl` values 0–8 for levels 1–9 and 9 for body text. Direct
values take precedence over inherited metadata. Clearing removes the direct
XML property. Namespace-qualified attributes distinguish Word values from
extension attributes; changed values retain unrelated attributes. The visual
paragraph style is retained through export even when the semantic node changes
between heading and paragraph. Older recovery files gain previously unmodeled
source outline settings together with their baseline, preserving local text and
format edits. A cleared setting in a modern recovery file is not reinstated.

The [Open XML outline definition](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.outlinelevel?view=openxml-3.0.1)
identifies this setting as structural rather than visual. The independent
fixture/reader is `scripts/verification/verify_outline_levels.py`; engine and UI
regressions cover precedence, all ten values, clearing, style changes, recovery,
rendered typography, Enter, mixed selection, table drafts and exports. Dated
results and native qualification are recorded in VALIDATION.md.

This does not implement Word's Outline view, outline-based TOC fields, collapsible
sections, structure reordering or promotion/demotion shortcuts. Word's UI rules
for built-in heading styles, complete table/navigation behavior, localized styles
and direct comparison with Microsoft Word remain to be qualified. No full parity
family is accepted by this feature.
