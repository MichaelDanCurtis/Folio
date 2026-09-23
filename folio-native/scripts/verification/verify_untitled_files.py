#!/usr/bin/env python3
"""Independently read first-save outputs, discarded recovery and blank restart."""
from pathlib import Path
import json,tempfile
from docx import Document
from openpyxl import load_workbook
root = Path(tempfile.gettempdir()) / 'folio-untitled-ui'
first = Document(root / 'first.docx')
assert first.paragraphs[0].text.startswith('First document. Good ideas')
assert 'Discarded close edit.' not in '\n'.join(p.text for p in first.paragraphs)
second = Document(root / 'second.docx')
assert second.paragraphs[0].text == 'Document and workbook saved separately.'
book = load_workbook(root / 'book.xlsx')
assert book.active['B4'].value == 8888
blank = json.loads((root / 'blank-restart.folio').read_text())
assert blank['title'] == 'Untitled workspace'
assert blank['document'] == {'type': 'doc', 'content': [{'type': 'paragraph'}]}
assert blank['sheets'] == [{'id': 'sheet-1', 'name': 'Sheet 1', 'cells': {}}]
assert 'officePackages' not in blank
recovery = json.loads((root / 'discarded-recovery.folio').read_text())
text = lambda node: node.get('text', '') + ''.join(text(n) for n in node.get('content', []))
assert text(recovery['document']).startswith('Discarded close edit. First document. Good ideas')
assert recovery['id'] != blank['id']
print('Independent first-save DOCX/XLSX, unchanged external file, blank restart, and discarded recovery reads passed.')
