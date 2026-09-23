#!/usr/bin/env python3
"""Independently inspect files written (and left untouched) by leave-file flows."""
from pathlib import Path
import tempfile
from docx import Document
from openpyxl import load_workbook
root = Path(tempfile.gettempdir()) / 'folio-file-leave-ui'
document = Document(root / 'document.docx')
assert document.paragraphs[0].text.startswith('Saved on close. Saved on leave. Good ideas')
assert 'Retained local.' not in '\n'.join(p.text for p in document.paragraphs)
workbook = load_workbook(root / 'book.xlsx')
cache = load_workbook(root / 'book.xlsx', data_only=True)
sheet = workbook.active
assert sheet['B4'].value == 6789, 'Don’t Save must not publish the later 7777 edit'
assert sheet['B9'].value == '=SUM(B4:B7)'
assert cache.active['B9'].value == sum(sheet[f'B{r}'].value for r in range(4, 8))
print('Save before New, Save on Close, and Don’t Save DOCX/XLSX results passed independent readers.')
