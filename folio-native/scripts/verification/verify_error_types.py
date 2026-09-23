"""Independent OOXML/openpyxl checks for error values versus identical text.

No Folio parser is imported; inspect both formula and data-only workbook views.
This does not evaluate formulas or qualify the entire Excel error taxonomy.
"""
import argparse
from pathlib import Path
import posixpath
import tempfile
from zipfile import ZipFile
from lxml import etree as E
from openpyxl import load_workbook

parser = argparse.ArgumentParser()
parser.add_argument('--with-ui', action='store_true')
args = parser.parse_args()
root = Path(tempfile.gettempdir()) / 'folio-error-type-tests'
project = Path(__file__).resolve().parents[2]
ns = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
errors = ['#NULL!', '#DIV/0!', '#VALUE!', '#REF!', '#NAME?', '#NUM!', '#N/A']

def xml_sheet(path):
    with ZipFile(path) as z:
        book = E.fromstring(z.read('xl/workbook.xml'))
        rid = book.find('s:sheets/s:sheet', ns).get('{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id')
        rels = E.fromstring(z.read('xl/_rels/workbook.xml.rels'))
        target = next(r.get('Target') for r in rels if r.get('Id') == rid)
        name = target.lstrip('/') if target.startswith('/') else posixpath.normpath('xl/' + target)
        return E.fromstring(z.read(name))

def cell(xml, address):
    return xml.find(f'.//s:c[@r="{address}"]', ns)

source = load_workbook(project / 'test/fixtures/error-types.xlsx').active
for name in ['edited.xlsx', 'numeric.xlsx', 'error.xlsx', 'reopened.xlsx']:
    formulas = load_workbook(root / name).active
    values = load_workbook(root / name, data_only=True).active
    xml = xml_sheet(root / name)
    for row, error in enumerate(errors, 2):
        for column in ['B', 'D']:
            assert values[f'{column}{row}'].data_type == 'e', (name, column, row)
            assert values[f'{column}{row}'].value == error
        for column in ['C', 'E', 'K']:
            assert values[f'{column}{row}'].data_type == 's', (name, column, row)
            assert values[f'{column}{row}'].value == error
        for column, truth in [('F', True), ('G', False), ('H', True), ('I', False)]:
            assert values[f'{column}{row}'].data_type == 'b'
            assert values[f'{column}{row}'].value is truth
        assert values[f'J{row}'].value == 'handled'
        assert values[f'L{row}'].data_type == 'n' and values[f'L{row}'].value == row-1
        for column in 'DEFGHIJKL':
            assert formulas[f'{column}{row}'].value == source[f'{column}{row}'].value
    for address, text in [('C11', '#N/A'), ('C12', '#N/A'), ('C13', '#n/a')]:
        assert values[address].value == text and values[address].data_type == 's'
    n = cell(xml, 'N2')
    assert n.get('{urn:folio:error-types}retained') == 'yes'
    assert n.find('s:f', ns).get('ca') == '1'
    assert n.find('s:extLst/s:ext', ns).text == 'retained'
    expected = {'edited.xlsx': ('s', '#N/A'), 'numeric.xlsx': ('n', 42),
                'error.xlsx': ('e', '#N/A'), 'reopened.xlsx': ('e', '#N/A')}[name]
    assert (values['N2'].data_type, values['N2'].value) == expected

assert (root/'unchanged.xlsx').read_bytes() == (project/'test/fixtures/error-types.xlsx').read_bytes()
assert (root/'error.xlsx').read_bytes() == (root/'reopened.xlsx').read_bytes()
values = load_workbook(root/'new.xlsx', data_only=True).active
for row, error in enumerate(errors, 1):
    for col in 'AC':
        assert (values[f'{col}{row}'].data_type, values[f'{col}{row}'].value) == ('e', error)
    for col in 'BD':
        assert (values[f'{col}{row}'].data_type, values[f'{col}{row}'].value) == ('s', error)

for name, offset in [('pasted.xlsx', 0), ('shifted.xlsx', 1)]:
    values = load_workbook(root/name, data_only=True).active
    formulas = load_workbook(root/name).active
    for row, error in enumerate(errors, 2+offset):
        assert (values[f'R{row}'].data_type, values[f'R{row}'].value) == ('e', error)
        assert (values[f'S{row}'].data_type, values[f'S{row}'].value) == ('s', error)
        assert formulas[f'R{row}'].data_type != 'f' and formulas[f'S{row}'].data_type != 'f'
values = load_workbook(root/'filled.xlsx', data_only=True).active
for row in range(3, 11):
    assert (values[f'D{row}'].data_type, values[f'D{row}'].value) == ('e', '#NULL!')
    assert (values[f'E{row}'].data_type, values[f'E{row}'].value) == ('s', '#NULL!')
for name, expected in [('changed-formula.xlsx', ('e', '#DIV/0!')), ('changed-text.xlsx', ('s', '#DIV/0!'))]:
    values = load_workbook(root/name, data_only=True).active
    assert (values['N2'].data_type, values['N2'].value) == expected
    n = cell(xml_sheet(root/name), 'N2')
    assert n.get('{urn:folio:error-types}retained') == 'yes'
    assert n.find('s:extLst/s:ext', ns).text == 'retained'
n = cell(xml_sheet(root/'engine-diagnostic.xlsx'), 'N2')
assert n.find('s:v', ns) is None and n.get('t') is None
assert n.find('s:f', ns).text == 'N2'
values = load_workbook(root/'legacy.xlsx', data_only=True).active
assert values['C12'].value == 'actual edit'
assert values['C2'].data_type == values['C11'].data_type == 's'
if args.with_ui:
    ui = Path(tempfile.gettempdir()) / 'folio-error-type-ui-tests'
    values = load_workbook(ui/'errors.xlsx', data_only=True).active
    for address in ['B2', 'C2']:
        assert (values[address].data_type, values[address].value) == ('e', '#N/A')
    for address in ['B3', 'C3']:
        assert (values[address].data_type, values[address].value) == ('s', '#N/A')
    assert (ui/'errors.xlsx').read_bytes() == (ui/'reopened.xlsx').read_bytes()
print('Error types passed: all seven classic errors versus literal/formula text, typed caches, ISERROR/IFERROR/ERROR.TYPE, metadata, type changes, Paste Values, fill/structure, legacy repair, and save/reopen.' + (' Widget export also passed.' if args.with_ui else ''))
