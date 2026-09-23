"""Independently read Folio template and created-file artifacts."""
from pathlib import Path
from zipfile import ZipFile
import json
import tempfile
from lxml import etree
from docx import Document
from openpyxl import load_workbook

root = Path(tempfile.gettempdir()) / 'folio-template-ui-output'
ct = {'ct': 'http://schemas.openxmlformats.org/package/2006/content-types'}
w = {'w': 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}

def main_type(path, part):
    with ZipFile(path) as package:
        types = etree.fromstring(package.read('[Content_Types].xml'))
        return types.xpath(f'//ct:Override[@PartName="/{part}"]/@ContentType', namespaces=ct)

assert main_type(root/'Reusable.dotx', 'word/document.xml') == ['application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml']
with ZipFile(root/'Reusable.dotx') as package:
    body = etree.fromstring(package.read('word/document.xml'))
    template_text = ''.join(body.xpath('//w:t/text()', namespaces=w))
    assert template_text.startswith('Edited template. Reusable greeting.')
created = Document(root/'created.docx')
assert '\n'.join(p.text for p in created.paragraphs).startswith('Edited template. Reusable greeting.')
letter = Document(root/'letter.docx')
assert '\n'.join(p.text for p in letter.paragraphs).startswith('Reusable greeting.')
assert not '\n'.join(p.text for p in letter.paragraphs).startswith('Edited template.')
assert created.styles['Normal'].font.name == 'Aptos'
for name in ['budget.xlsx','created.xlsx','Team budget.xltx']:
    book = load_workbook(root/name, data_only=False)
    sheet = book['Budget']
    assert sheet['B2'].value == 1000 and sheet['C2'].value == 125
    assert sheet['D2'].value == '=B2-C2'
    assert sheet['D8'].value == '=SUM(D2:D7)'
    assert sheet['B2'].number_format == '$#,##0.00;[Red]($#,##0.00)'
    assert sheet.freeze_panes == 'A2'
    assert book.template == name.endswith('xltx')
    cached = load_workbook(root/name, data_only=True)
    assert cached['Budget']['D2'].value == 875
    expected = 'template' if name.endswith('xltx') else 'sheet'
    assert main_type(root/name, 'xl/workbook.xml') == [f'application/vnd.openxmlformats-officedocument.spreadsheetml.{expected}.main+xml']
assets = Path(__file__).resolve().parents[2] / 'assets/templates'
tracker = load_workbook(assets/'tracker.xltx')
assert tracker.template
assert len(tracker.active.data_validations.dataValidation) == 1
assert 'Complete' in tracker.active.data_validations.dataValidation[0].formula1
state = json.loads((root/'workspace.folio').read_text())
assert state['sheets'][0]['cells']['B2'] == '1000'
print(json.dumps({'outputDirectory':str(root), 'wordTemplateAndSeparateDocuments':True,
    'wordBaseStyleRetained':True, 'excelTemplatesAndDocuments':True,
    'formulasCachedResultsCurrencyAndFrozenRows':True, 'starterValidationList':True,
    'officePackageTypesMatchExtensions':True}, indent=2))
