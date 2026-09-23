"""Independent XML decoding checks; openpyxl does not decode all ST_Xstring
tokens in cell values. Use it for structure/fonts and a separate regex/UTF-16
reader for these text assertions, without importing Folio's implementation."""
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
import re
from lxml import etree
from openpyxl import load_workbook

folder = Path(gettempdir()) / 'folio-string-tests'
source = Path(__file__).resolve().parents[2] / 'test/fixtures/escaped-strings.xlsx'
ns = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
def decode(text):
    units = re.sub(r'_x([0-9a-fA-F]{4})_', lambda m: chr(int(m[1], 16)), text or '')
    return units.encode('utf-16-le', 'surrogatepass').decode('utf-16-le')

def cell_text(xml, address):
    cell = xml.find(f'.//s:c[@r="{address}"]', ns)
    content = cell.find('s:is', ns)
    if content is not None:
        return ''.join(decode(t.text) for t in content.xpath('s:t | s:r/s:t', namespaces=ns))
    return decode(cell.find('s:v', ns).text)

rich = 'A\rB🌊 _x0041_ & end'
with ZipFile(source) as before, ZipFile(folder/'characters.xlsx') as after:
    assert before.read('xl/sharedStrings.xml') == after.read('xl/sharedStrings.xml')
    assert before.read('xl/styles.xml') == after.read('xl/styles.xml')
    xml = etree.fromstring(after.read('xl/worksheets/sheet1.xml'))
    assert cell_text(xml, 'B30') == rich
    runs = xml.findall('.//s:c[@r="B30"]/s:is/s:r', ns)
    assert [decode(r.find('s:t', ns).text) for r in runs] == ['A\r', 'B🌊', ' _', 'x0041_ & end']
    for index, run in enumerate(runs):
        if index in (1, 2):
            assert run.find('s:rPr/s:b', ns).get('val') in ('0', 'false')
            assert run.find('s:rPr/s:sz', ns).get('val') == '16.5'
        if index < 2:
            assert run.get('{urn:folio:escaped}flag') == 'keep-run'
            assert run.find('s:t', ns).get('{urn:folio:escaped}flag') == 'keep-text'
            assert run.find('s:rPr/s:outline', ns) is not None
    assert xml.find('.//s:c[@r="B30"]/s:is/s:rPh/s:t', ns).text == 'PHONETIC'
    # Export refreshes formula caches in other sheets, retaining the formula.
    summary = etree.fromstring(after.read('xl/worksheets/sheet2.xml'))
    assert summary.find('.//s:c[@r="A1"]/s:f', ns).text == 'SUM(Input!B2:B5)'
    assert float(summary.find('.//s:c[@r="A1"]/s:v', ns).text) == 30

for name in ['characters.xlsx', 'values.xlsx', 'legacy.xlsx']:
    book = load_workbook(folder/name, rich_text=True)
    assert book['Input']['B30'].value[1].font.sz == 16.5
with ZipFile(folder/'values.xlsx') as saved:
    xml = etree.fromstring(saved.read('xl/worksheets/sheet1.xml'))
    assert cell_text(xml, 'M30') == '_x0041_\r\n\t🌊\0&<>'
    assert cell_text(xml, 'N30') == '_x005F_x000D_'
    assert cell_text(xml, 'I30') == '_x0041_'
    assert cell_text(xml, 'J30') == '_x0041_'
    assert cell_text(xml, 'K30') == '001'
    assert cell_text(xml, 'O30') == '_x0042_'
    assert xml.find('.//s:c[@r="J30"]/s:f', ns).text == '"_x0041_"'
    assert xml.find('.//s:c[@r="O30"]/s:f', ns).text == '"_x0042_"'
assert (folder/'original.xlsx').read_bytes() == source.read_bytes()
assert (folder/'undone.xlsx').read_bytes() == source.read_bytes()
for name in ['redone.xlsx','reopened.xlsx']:
    assert (folder/name).read_bytes() == (folder/'characters.xlsx').read_bytes()
print('Independent escaped-text, character offsets, cached strings, metadata and history checks passed.')
