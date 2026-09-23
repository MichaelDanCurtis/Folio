"""Independently read search/replacement exports produced by search_office_test."""
import sys
from pathlib import Path
from zipfile import ZipFile
from docx import Document
from openpyxl import load_workbook
from lxml import etree

base = Path(sys.argv[1])
source = Path(__file__).resolve().parents[2] / 'test' / 'fixtures'
document = Document(base / 'replaced.docx')
assert document.paragraphs[1].text == 'Income grew by 12%.'
assert document.tables[0].cell(0, 1).text == 'Income'
assert document.sections[0].header.paragraphs[0].text == 'Keep this header'
assert len(document.inline_shapes) == 1
assert any(run.text == 'grew' and run.bold for run in document.paragraphs[1].runs)
workbook = load_workbook(base / 'replaced.xlsx')
sheet = workbook['Budget']
assert sheet['B2'].value == 3000
assert sheet['B2'].font.bold
assert sheet['B2'].number_format == '"$"#,##0.00'
assert sheet['B4'].value == '=SUM(B2:B3)'
assert sheet['AA1'].value == '=Budget!B2*Inputs!A1'
assert sheet['A2'].comment.text == 'Keep this comment'
assert len(sheet._charts) == 1
assert sheet.freeze_panes == 'B2'
assert sheet.auto_filter.ref == 'A1:C4'
assert 'E1:F1' in str(sheet.merged_cells)
for extension in ('docx', 'xlsx'):
    with ZipFile(base / f'replaced.{extension}') as result, ZipFile(source / f'source.{extension}') as original:
        for name in result.namelist():
            if name.endswith(('.xml', '.rels')):
                etree.fromstring(result.read(name))
        for name in original.namelist():
            if name.startswith(('word/header', 'word/media/', 'xl/charts/', 'xl/comments/')):
                assert result.read(name) == original.read(name), name
print('Search exports passed independent content, formatting, object and XML checks.')
