# Repeated table header fixture

Synthetic native DOCX emitted by the Rust test
`word_table_headers_native_document_reimports_rich_headings_and_page_fields`.
It contains a two-column, 47-row table: a blue merged title, one column-heading
row with PAGE and NUMPAGES fields, and accounts 00–44. Both heading rows have
`w:tblHeader w:val="on"`. Saved field caches are deliberately `1`; preview
calculates its own values without changing those caches.

This is a Folio-generated test document, not a Word-authored comparison file.
See `docs/TABLE_HEADERS.md` for acceptance boundaries and regeneration.
