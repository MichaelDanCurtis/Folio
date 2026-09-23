# Section page setup — 0.63 development

Layout > Page setup reads the selected section from the insertion point and
edits paper size, orientation, margins, gutter, header/footer distances and
section start. Letter, Legal, A4, A5 and Tabloid presets, margin presets and
inches/centimeters/points share whole-twip geometry with DOCX. Select This section
or Whole document; only changed properties apply, retaining other differences.

The document editing surface uses Section 1's physical width and margins at
96 pixels per inch. Wide paper scrolls horizontally rather than shrinking the
text. This is a continuous view, **not pagination**. Other sections' geometry is
stored and editable but does not yet change the editing surface within the
same document. Columns, headers/footers, page numbers, page breaking, printing,
PDF layout and a page ruler remain incomplete. Review display has its separate
continuous markup surface. No Word layout or parity family is complete.

## Preservation and review

Section settings are read from paragraph-ending `sectPr` records and the final
body `sectPr`. Continuous sections with omitted size or margins resolve those
properties from the following section. Missing final geometry uses Letter with
one-inch margins without writing defaults merely on inspection.

Apply checks the workspace revision and section source identity, then patches
only owned size, margin or section-start attributes. Other section properties,
header/footer links, relationships, ZIP parts and unknown attributes remain.
Protection and unresolved section-formatting revisions prevent Apply. Invalid
measurements leave the workspace unchanged. Whole-document requests are atomic.
An imported recording request needs an explicit author before changes.

When formatting recording is on, each changed section gets a `sectPrChange`
with its previous properties and the chosen author. Accept, Reject, Undo/Redo
and recovery use the existing review flow. Header/footer references stay in the
current section when its prior formatting is restored: the previous-section
schema excludes those references. Pending ordinary text is included in the
layout transaction; workbook blocks keep their live binding.

Enter in a section-ending paragraph retains its section boundary on the final
split paragraph. Ordinary document edits cannot silently remove or reorder that
boundary; direct section-break insertion/deletion remains unimplemented.

## Verification

Engine checks cover selected/all sections, default and alternate namespaces,
unit-form input, extensions, no-op/invalid/stale atomic behavior, continuous
inheritance, paragraph splitting, protected section preservation, recording,
Accept/Reject, pending text, history, save/load and live workbook updates.
Native qualification covers Cancel/Apply, editing-mode keyboard Undo/Redo,
ordinary typing/Enter, per-section settings, recording, Accept/Reject, toolbar
Undo, saves and byte-identical restart. The 0.63 post-Save review keyboard
failure and raw measurement display are fixed and natively qualified in 0.64;
see `HISTORY_MENUS.md` for focused history ownership and remaining native
file-panel keyboard/input-timing checks.

Flutter checks cover units, paper/orientation, section/scope selection,
validation, cancellation, exact paper geometry, scrolling and real-engine
editing/history/recovery. Independent XML and Open XML SDK checks are recorded
in `VALIDATION.md`; those checks do not run Microsoft Word.

## Source references

- [Section properties and placement](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.sectionproperties?view=openxml-3.0.1)
- [Page size](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.pagesize?view=openxml-3.0.1)
- [Page margins](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.pagemargin?view=openxml-3.0.1)
- [Continuous section inheritance](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.sectiontype?view=openxml-3.0.1)
- [Previous section properties](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.previoussectionproperties?view=openxml-3.0.1)
