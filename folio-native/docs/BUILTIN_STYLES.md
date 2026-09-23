# Missing heading styles and built-in protection

Build 9 exposes Heading 1–9 in Styles even when an imported document has no
corresponding declarations. Missing entries say **Added when applied**. Opening
or cancelling the picker does not change the document. Applying one creates only
that definition and its missing dependencies, then applies it in one history
transaction. Existing paragraph styles matching the conventional heading ID or
name retain their declarations and identity. An ID occupied by another style
type receives a collision-free heading identity.

The fallback definitions use Folio's heading typography and the document's
existing default paragraph style as their base and following style. Documents
without a default style receive an explicit Normal definition when needed,
without changing document-wide defaults. This does not reproduce every Word
template's theme-specific default heading appearance.

The editor supports semantic heading levels 7–9 as well as 1–6. Import, style
application, rich-text conversion, paragraph rendering and following-style Enter
retain the actual level. Conventional heading formatting refreshes the named
style metadata rather than retaining the old body style. Immediate Enter uses
the following body style even while the heading definition is being saved.

Queued editor input asks the engine to retain its current protected style catalog.
The engine discards any declarations supplied with that request; explicit style
edits still require the Styles transaction and its revision check. This permits
automatic heading creation followed immediately by typing without stale-catalog
failures. Normal typing does not clone the preserved Office package merely to
look for missing definitions.

Table-cell style previews stay in the table draft. Cancel leaves both content and
the global catalog unchanged. Outer Apply materializes the required definitions
and cell changes together; one Undo restores both.

## Style identity

The catalog retains the Word `customStyle` flag. A false or omitted source flag
identifies a built-in declaration for deletion protection. New user styles are
always custom, including copies of built-in styles. Existing origins cannot be
changed through Modify. Built-in and default styles can be modified but cannot
be deleted; this remains true after renaming. Older saved catalogs lacking origin
metadata are interpreted using their preserved source (or Folio's native source
for native documents). Styles absent from that older source remain custom.

Microsoft documents [built-in identity separately from localized names](https://learn.microsoft.com/en-us/office/vba/api/word.style.builtin).
The independent reader uses [python-docx's metadata-based built-in property](https://python-docx.readthedocs.io/en/latest/api/style.html#docx.styles.style.BaseStyle.builtin).
This metadata-based protection is not a complete registry of all historical Word
style identities or localized aliases.

## Verification and limits

Engine checks cover all nine levels, missing style parts, ID collisions, cell
drafts, direct-format retention, origin protection/rename, legacy interpretation,
queued typing, Undo/Redo, local recovery and DOCX reimport. The real-engine editor
workflow checks the gallery, rapid heading/Enter/typing, table Cancel/Apply,
rendering all nine levels and reopened exports. The independent reader is
`scripts/verification/verify_builtin_styles.py`; dated results are in VALIDATION.md.

Direct outline overrides, comprehensive localized/legacy identity resolution,
linked-character style authoring, full theme/script cascades, templates, organizer
and Microsoft Word comparison remain open. This does not accept the entire W02
named-styles family or the overall parity goal.
