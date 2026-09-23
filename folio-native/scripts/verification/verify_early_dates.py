"""Inspect the raw OOXML date serials and cached results, plus independent parsing.

The fixture is a modeled serial calendar, not a captured Microsoft Excel oracle.
openpyxl normalizes serials 59 and 60 to the same Gregorian date, so assertions
for date values use raw worksheet XML instead of its datetime conversion.
"""
from pathlib import Path
import json
import sys
import tempfile
import zipfile
from lxml import etree
from openpyxl import load_workbook

project = Path(__file__).resolve().parents[2]
expected = json.loads((project / 'test/fixtures/early-dates.json').read_text())
source = load_workbook(project / 'test/fixtures/early-dates.xlsx').active
ns = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
errors = {'#NUM!', '#VALUE!', '#N/A', '#REF!', '#DIV/0!', '#NAME?', '#NULL!'}

def verify(path, edited=False, typed=False):
    formulas = load_workbook(path).active
    with zipfile.ZipFile(path) as z:
        # The generated fixture has one sheet with this standard part name.
        xml = etree.fromstring(z.read('xl/worksheets/sheet1.xml'))
        cells = {c.get('r'): c for c in xml.findall('s:sheetData/s:row/s:c', ns)}
        values = dict(expected['expected'])
        if edited:
            values.update(A62=59, B62=28, E62=3, F62=59, G62=59,
                          H62='1900-02-28', I62=88)
        if typed: values.update(P2=1, P3=59, P4=60, Q2=2)
        for a, v in values.items():
            cell = cells[a]
            cache = cell.find('s:v', ns)
            assert cache is not None, (path, a, 'missing cache/value')
            if isinstance(v, (float, int)):
                assert cell.get('t', 'n') == 'n' and float(cache.text) == v, (path, a, cache.text, v)
            else:
                assert cell.get('t') == ('e' if v in errors else 'str'), (path, a, 'wrong type')
                assert cache.text == v, (path, a, cache.text, v)
            if source[a].data_type == 'f':
                assert formulas[a].value == source[a].value, (path, a, 'formula changed')
        for row in range(2, 74):
            assert formulas[f'A{row}'].number_format == 'yyyy-mm-dd'
        for a, v in expected['explicitText'].items():
            assert formulas[a].data_type == 's' and formulas[a].value == v
        if typed: assert formulas['Q2'].value == '=COUNTIF(P2:P4,">=1900-02-28")'

if __name__ == '__main__':
    if len(sys.argv) > 1:
        # Packaged UI export after editing A62; seeded from the engine workspace.
        for name in sys.argv[1:]: verify(Path(name), edited=True, typed=True)
        print(f'Native early-date exports passed all {len(expected["expected"])} modeled results, edited date/formulas, typed input serials and explicit text retention.')
    else:
        for folder_name, edited, typed in [('folio-early-date-tests', False, True), ('folio-early-date-ui-tests', True, False)]:
            folder = Path(tempfile.gettempdir()) / folder_name
            for name in ['early.xlsx', 'reopened.xlsx']: verify(folder / name, edited, typed)
            assert (folder / 'early.xlsx').read_bytes() == (folder / 'reopened.xlsx').read_bytes()
        print(f'Early dates passed: {len(expected["expected"])} modeled serial/calendar/formula results, explicit text types, native-bridge entry/display/history/clipboard, typed error caches and exact save/reopen exports.')
