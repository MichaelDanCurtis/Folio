"""Independent read-back of selected-character edits and untouched metadata."""
from copy import copy
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
from lxml import etree
from openpyxl import load_workbook
from openpyxl.styles import Color
from openpyxl.cell.text import InlineFont
import sys

folder = Path(gettempdir()) / 'folio-style-tests'
source = Path(__file__).resolve().parents[2] / 'test/fixtures/rich-text.xlsx'
original = load_workbook(source, rich_text=True)['Input']
edited = load_workbook(folder / 'characters.xlsx', rich_text=True)['Input']
def chars(cell):
    return [(ch, InlineFont() if isinstance(run, str) else run.font)
            for run in cell.value for ch in (run if isinstance(run, str) else run.text)]
old, new = chars(original['B20']), chars(edited['B20'])
assert len(old) == len(new)
for i, ((ch, font), (actual, fmt)) in enumerate(zip(old, new)):
    assert ch == actual
    wanted = copy(font)
    if 2 <= i < 8:
        wanted.b = False; wanted.sz = 16.5; wanted.color = Color(rgb='FF123456')
    assert fmt == wanted, (i, ch, fmt, wanted)
assert edited['C20'].value == original['C20'].value
ns = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with ZipFile(source) as before, ZipFile(folder / 'characters.xlsx') as after:
    assert before.read('xl/sharedStrings.xml') == after.read('xl/sharedStrings.xml')
    assert before.read('xl/styles.xml') == after.read('xl/styles.xml')
    xml = etree.fromstring(after.read('xl/worksheets/sheet1.xml'))
    cell = xml.find('.//s:c[@r="B20"]', ns)
    runs = cell.findall('s:is/s:r', ns)
    assert [r.find('s:t', ns).text for r in runs] == ['Ri', 'ch ', 'tex', 't 🌊']
    for run in runs[:2]:
        assert run.get('{urn:folio:rich}flag') == 'retain-run'
        assert run.find('s:t', ns).get('{urn:folio:rich}flag') == 'retain-text'
        assert run.find('s:rPr/s:outline', ns) is not None
        assert run.find('s:rPr/s:shadow', ns) is not None
    assert runs[0].find('s:rPr/s:sz', ns).get('{urn:folio:rich}val') == '900'
    for run in runs:
        assert run.find('s:t', ns).get('{http://www.w3.org/XML/1998/namespace}space') == 'preserve'
    assert cell.find('s:is/s:rPh/s:t', ns).text == 'PHONETIC ONLY'
assert (folder/'characters-undone.xlsx').read_bytes() == source.read_bytes()
for name in ['characters-redone.xlsx','characters-reopened.xlsx']:
    assert (folder/name).read_bytes() == (folder/'characters.xlsx').read_bytes()
literal = load_workbook(folder/'characters-literals.xlsx', rich_text=True)['Input']
for a, text in [('E20','001'),('F20','=literal'),('G20','TRUE'),('I20','Stored string'),('Z20','Plain text')]:
    assert str(literal[a].value) == text
assert chars(literal['A20'])[-1][0] == '🌊' and chars(literal['A20'])[-1][1].b
assert load_workbook(folder/'characters-replaced.xlsx')['Input']['Z20'].value == 'Different'
reformatted = load_workbook(folder/'characters-reformatted.xlsx', rich_text=True)['Input']['Z20']
assert str(reformatted.value) == 'Different'
assert all(font.b for _, font in chars(reformatted)[:4])
new = load_workbook(folder/'characters-new.xlsx', rich_text=True).worksheets[0]['A40']
assert str(new.value) == 'New rich text'
assert new.value[1].font.rFont == 'Georgia' and new.value[1].font.color.rgb == 'FF123456'
if '--with-ui' in sys.argv:
    ui = Path(gettempdir())/'folio-font-ui-tests'
    original = load_workbook(source.with_name('styles.xlsx'), rich_text=True)['Input']
    edited = load_workbook(ui/'characters-ui.xlsx', rich_text=True)['Input']
    for i, ((old, font), (new, actual)) in enumerate(zip(chars(original['S2']), chars(edited['S2']))):
        assert old == new
        wanted = copy(font)
        if 2 <= i < 9: wanted.rFont = 'Georgia'; wanted.b = False
        assert actual == wanted, (i, actual, wanted)
    assert edited['T2'].value == original['T2'].value
    assert (ui/'characters-ui-undone.xlsx').read_bytes() == source.with_name('styles.xlsx').read_bytes()
    assert (ui/'characters-ui-redone.xlsx').read_bytes() == (ui/'characters-ui.xlsx').read_bytes()
    print('Verified keyboard-selected character formatting from the actual Font dialog.')
print('Verified character boundaries, other fonts, source namespaces/effects/phonetics, unchanged style/shared-string parts, literal/new cells and recovery.')
