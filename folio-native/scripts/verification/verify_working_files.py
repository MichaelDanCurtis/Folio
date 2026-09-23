#!/usr/bin/env python3
"""Read the actual Office/workspace files produced by Save/Save As UI flows."""
from pathlib import Path
import tempfile,json
from docx import Document
from openpyxl import load_workbook
root=Path(tempfile.gettempdir())/'folio-working-files-ui'
first=Document(root/'first.docx')
assert first.paragraphs[0].text.startswith('Second First Good ideas')
assert not (root/'second.docx').exists(), 'The renamed original must not be recreated'
renamed=Document(root/'renamed.docx')
assert renamed.paragraphs[0].text.startswith('Moved Third Second First Good ideas')
latest=Document(root/'both-versions.docx')
assert latest.paragraphs[0].text.startswith('Failure retained Local Moved Third Second First Good ideas')
book=load_workbook(root/'book.xlsx')
assert book.active['B4'].value == 5678
assert book.active['B9'].value == '=SUM(B4:B7)'
cache=load_workbook(root/'book.xlsx',data_only=True)
assert cache.active['B9'].value == 23278
copy=json.loads((root/'copy.folio').read_text())
assert copy['document']['content'][0]['content'][0]['text'].startswith('Workspace Third Second First Good ideas')
assert copy['sheets'][0]['cells']['B4'] == '2400'
assert 'workingFiles' not in copy and 'fileAccess' not in copy
print('Independent Save/Save As DOCX, XLSX values/formulas/caches and full-workspace reads passed.')
