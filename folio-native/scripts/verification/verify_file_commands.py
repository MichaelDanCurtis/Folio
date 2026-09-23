#!/usr/bin/env python3
"""Independently read the Office files produced by file-command editor checks."""
from pathlib import Path
import json, tempfile
from docx import Document
from openpyxl import load_workbook
root = Path(tempfile.gettempdir())/'folio-file-commands-ui'
saved = json.loads((root/'saved.folio').read_text())
assert saved['document']['content'][0]['content'][0]['text'].startswith('Fresh Good ideas')
doc = Document(root/'document.docx')
assert doc.paragraphs[0].text == 'Direct level one 😀'
assert doc.paragraphs[0].style.name == 'Outline body'
assert doc.paragraphs[0]._p.xpath('./w:pPr/w:outlineLvl')[0].get('{http://schemas.openxmlformats.org/wordprocessingml/2006/main}val') == '0'
book = load_workbook(root/'workbook.xlsx')
assert book.active['B4'].value == 4567
assert book.active['B9'].value == '=SUM(B4:B7)'
print('Independent DOCX/XLSX and workspace reads passed for file-command outputs.')
