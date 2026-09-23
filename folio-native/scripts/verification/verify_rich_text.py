"""Read displayed-rich-text exports with openpyxl and inspect preserved XML."""
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
from lxml import etree
from openpyxl import load_workbook
import sys

source = Path(__file__).resolve().parents[2] / 'test/fixtures/rich-text.xlsx'
folder = Path(gettempdir()) / 'folio-style-tests'
original = load_workbook(source, rich_text=True)['Input']
edited = load_workbook(folder / 'rich-display.xlsx', rich_text=True)['Input']
for a in ['A20', 'B20', 'C20']:
    assert str(original[a].value) == str(edited[a].value) == 'Rich text 🌊'
    assert edited[a].value[0].font.rFont == 'Georgia'
    assert edited[a].value[0].font.sz == 18
    assert edited[a].value[1].font.rFont == 'Arial'
    assert edited[a].value[1].font.sz == 12
    assert edited[a].value[1].font.i
assert edited['A20'].value == original['A20'].value
assert edited['C20'].value == original['C20'].value
for run in edited['B20'].value:
    assert not run.font.b and run.font.u == 'double'
for a, value in [('E20', '001'), ('F20', '=literal'), ('G20', 'TRUE'), ('H20', '東京')]:
    assert str(edited[a].value) == value
ns = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with ZipFile(source) as old, ZipFile(folder / 'rich-display.xlsx') as new:
    assert old.read('xl/sharedStrings.xml') == new.read('xl/sharedStrings.xml')
    xml = etree.fromstring(new.read('xl/worksheets/sheet1.xml'))
    annotation = xml.find('.//s:c[@r="B20"]/s:is/s:rPh/s:t', ns)
    assert annotation.text == 'PHONETIC ONLY'
    assert xml.find('.//s:c[@r="C20"]', ns).get('t') == 's'
assert (folder / 'rich-display-undone.xlsx').read_bytes() == source.read_bytes()
for name in ['rich-display-redone.xlsx', 'rich-display-reopened.xlsx']:
    assert (folder / name).read_bytes() == (folder / 'rich-display.xlsx').read_bytes()
if '--with-ui' in sys.argv:
    ui = Path(gettempdir()) / 'folio-font-ui-tests'
    source = source.with_name('styles.xlsx')
    original = load_workbook(source, rich_text=True)['Input']
    edited = load_workbook(ui / 'rich-render.xlsx', rich_text=True)['Input']
    assert edited['T2'].value == original['T2'].value
    assert str(edited['S2'].value) == 'Shared title'
    assert all(not run.font.b for run in edited['S2'].value)
    assert edited['S2'].value[0].font.rFont == 'Times New Roman'
    assert edited['S2'].value[1].font.sz == 22
    assert edited['S2'].value[1].font.i
    assert (ui / 'rich-render-undone.xlsx').read_bytes() == source.read_bytes()
    print('Verified the mixed-cell Font dialog export and exact Undo.')
print('Verified rich/shared text, Unicode, phonetic retention and exact recovery with independent readers.')
