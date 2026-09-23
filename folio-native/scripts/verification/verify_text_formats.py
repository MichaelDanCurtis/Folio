"""Independent XLSX structure/type checks; not an Excel display/coercion oracle."""
from pathlib import Path
import json
import posixpath
import tempfile
import zipfile
from openpyxl import load_workbook
from lxml import etree

root = Path(tempfile.gettempdir()) / 'folio-text-format-tests'
ui = Path(tempfile.gettempdir()) / 'folio-text-format-ui-tests'
ns = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}

def worksheet_xml(z):
    workbook = etree.fromstring(z.read('xl/workbook.xml'))
    sheet = workbook.find('s:sheets/s:sheet', ns)
    rid = sheet.get('{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id')
    rels = etree.fromstring(z.read('xl/_rels/workbook.xml.rels'))
    target = next(r.get('Target') for r in rels if r.get('Id') == rid)
    path = target.lstrip('/') if target.startswith('/') else posixpath.normpath('xl/' + target)
    return etree.fromstring(z.read(path))

snapshot = json.loads((root / 'text-formulas.json').read_text())
values = snapshot['calculated']['s']
sources = snapshot['workspace']['sheets'][0]['cells']
formulas = load_workbook(root / 'text-formulas.xlsx').active
cached = load_workbook(root / 'text-formulas.xlsx', data_only=True).active
with zipfile.ZipFile(root / 'text-formulas.xlsx') as z:
    xml = worksheet_xml(z)
    for r in range(1, 13):
        a, b, c = f'A{r}', f'B{r}', f'C{r}'
        assert formulas[a].data_type == 'n' and formulas[a].value == values[a], a
        assert formulas[b].value == sources[b] and formulas[b].data_type == 'f', b
        assert isinstance(values[b], str) and values[c] == 2, b
        node = xml.xpath(f'//s:c[@r="{b}"]', namespaces=ns)[0]
        assert node.get('t') == 'str', b
        assert node.find('s:v', ns).text in (values[b], None if values[b] == '' else values[b]), b
        assert cached[b].value == (values[b] or None), b
        assert formulas[b].number_format == 'General', b
        assert formulas[b].font.color is None or formulas[b].font.color.type != 'rgb' or formulas[b].font.color.rgb != 'FFFF0000', b
assert values['B1'] == '6.0000'
assert values['B2'] == '$1,234.57'
assert values['B3'] == '28.5%'
assert values['B7'] == '1900-02-29'
assert values['B8'] == '12:00:00 PM'
assert values['B9'] == '72:00:00'
assert values['B10'] == '(1.25)'
assert (root / 'text-formulas.xlsx').read_bytes() == (root / 'text-reopened.xlsx').read_bytes()

filtered = load_workbook(root / 'formatted-filter.xlsx').active
assert filtered.auto_filter.ref == 'A1:B5'
rules = {f.colId: f.filters.filter for f in filtered.auto_filter.filterColumn}
assert rules == {0: ['Keep'], 1: ['0.125']}, rules
assert filtered['B2'].value == .125 and filtered['B2'].number_format == '0.0%'
assert [r for r in range(2, 6) if filtered.row_dimensions[r].hidden] == [3, 4, 5]

sheet = load_workbook(ui / 'formatted.xlsx').active
results = load_workbook(ui / 'formatted.xlsx', data_only=True).active
assert sheet['B1'].value == .5 and sheet['B1'].number_format == '0%'
assert sheet['B2'].value == .125 and sheet['B2'].number_format == '0.0%'
assert sheet['C2'].value == '=TEXT(B2,D2)' and sheet['D2'].value == '0.0000'
assert results['C2'].value == '0.1250' and results['C2'].data_type == 's'
assert results['C3'].value == '$0.25' and results['C4'].value == '1900-02-29'
assert sheet.auto_filter.ref == 'A1:D4'
assert sheet.auto_filter.filterColumn[0].colId == 1
assert sheet.auto_filter.filterColumn[0].filters.filter == ['0.25']
assert [r for r in range(2, 5) if sheet.row_dimensions[r].hidden] == [2, 4]
with zipfile.ZipFile(ui / 'formatted.xlsx') as z:
    xml = worksheet_xml(z)
    assert xml.xpath('string(//s:c[@r="A2"]/s:v)', namespaces=ns) == '60'
assert (ui / 'formatted.xlsx').read_bytes() == (ui / 'reopened.xlsx').read_bytes()
print('TEXT and formatted filters passed: numeric sources, string formula caches, retained format codes, raw filter keys, visibility and identical save/reopen XLSX (engine + widget).')
