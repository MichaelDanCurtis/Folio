"""Read formatting exports independently through openpyxl and lxml."""
from copy import copy
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
from openpyxl import load_workbook
from lxml import etree
import sys

folder = Path(gettempdir()) / 'folio-style-tests'
fixtures = Path(__file__).resolve().parents[2] / 'test/fixtures'
ns = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
def cell(path, address='B2'):
    return load_workbook(path, rich_text=True)['Input'][address]
def equal_components(actual, expected, skip=()):
    for key in ['font', 'fill', 'border', 'alignment', 'protection', 'number_format']:
        if key not in skip:
            assert copy(getattr(actual, key)) == copy(getattr(expected, key)), key
source = cell(fixtures / 'fill.xlsx')
for name in ['bold', 'redone', 'reopened', 'reimported']:
    actual = cell(folder / f'{name}.xlsx')
    expected_font = copy(source.font); expected_font.b = False
    assert copy(actual.font) == expected_font, name
    equal_components(actual, source, ['font'])
    assert actual.value == source.value
assert (folder / 'original.xlsx').read_bytes() == (folder / 'undone.xlsx').read_bytes()
for name in ['redone', 'reopened', 'reimported']:
    assert (folder / 'bold.xlsx').read_bytes() == (folder / f'{name}.xlsx').read_bytes()

source = cell(fixtures / 'styles.xlsx')
for name, changed in [('italic', 'font'), ('color', 'font'), ('fill', 'fill'), ('number', 'number_format')]:
    actual = cell(folder / f'{name}.xlsx')
    equal_components(actual, source, [changed])
    expected = copy(getattr(source, changed))
    if name == 'italic': expected.i = False
    elif name == 'color': expected.color = 'FF112233'
    elif name == 'fill':
        from openpyxl.styles import PatternFill, Color
        expected = PatternFill('solid', fgColor='FFAABBCC', bgColor=Color(indexed=64))
    elif name == 'number': expected = '0.00%'
    assert copy(getattr(actual, changed)) == expected, name

inherited = load_workbook(folder / 'inherited.xlsx')['Input']
for address, name, size in [('L2', 'Cambria', 18), ('P2', 'Georgia', 16), ('P10', 'Georgia', 16), ('R7', 'Verdana', 14)]:
    actual = inherited[address].font
    assert (actual.name, actual.sz, actual.b, actual.i) == (name, size, False, False), address
assert inherited['F2']._style.numFmtId == 14
assert inherited['G2']._style.numFmtId == 27
assert cell(folder / 'explicit-general.xlsx', 'G2').number_format == 'General'
legacy = load_workbook(folder / 'legacy.xlsx')['Input']
assert legacy['B2'].font.name == 'Cambria' and not legacy['B2'].font.b
assert legacy['P2'].font.name == 'Georgia' and not legacy['P2'].font.i
assert legacy['F2']._style.numFmtId == 14 and legacy['G2']._style.numFmtId == 27

for name in ['bold', 'italic', 'fill', 'color', 'number', 'prefixed', 'inherited', 'legacy', 'deduplicated']:
    with ZipFile(folder / f'{name}.xlsx') as package:
        for part in package.namelist():
            if part.endswith(('.xml', '.rels')): etree.fromstring(package.read(part))
        with ZipFile(fixtures / 'fill.xlsx') as original:
            for part in ['xl/theme/theme1.xml', 'docProps/core.xml']:
                assert package.read(part) == original.read(part)
with ZipFile(folder / 'prefixed.xlsx') as package:
    styles = etree.fromstring(package.read('xl/styles.xml'))
    cells = etree.fromstring(package.read('xl/worksheets/sheet1.xml'))
    xf = styles.find('s:cellXfs', ns)[int(cells.find('.//s:c[@r="B2"]', ns).get('s'))]
    assert xf.get('{urn:folio:style:metadata}fontId') == 'keep-id'
    assert xf.get('quotePrefix') == '1'
    assert xf.find('.//{urn:folio:style:metadata}payload').get('value') == 'keep-extension'

ui = Path(gettempdir()) / 'folio-style-ui-tests'
if '--with-ui' in sys.argv:
    actual = cell(ui / 'edited.xlsx')
    expected_font = copy(source.font); expected_font.b = False; expected_font.i = False
    assert copy(actual.font) == expected_font
    equal_components(actual, source, ['font'])
    assert actual.value == '=A2*$H$1+1'
    assert (ui / 'undone.xlsx').read_bytes() == (fixtures / 'styles.xlsx').read_bytes()
    for name in ['redone.xlsx', 'reopened.xlsx']:
        assert (ui / name).read_bytes() == (ui / 'edited.xlsx').read_bytes()
    print('Verified actual toolbar edits, pending formula commit, Undo/Redo and workspace reload.')
print('Verified fonts, formatting components, inheritance, date/locale IDs, old-workspace migration, namespaces, unchanged parts and exact recovery bytes.')
