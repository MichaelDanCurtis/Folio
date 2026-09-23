"""Independent calendar metadata, raw serial, formula/cache and text-type checks.

Python dates are a Gregorian oracle; neither openpyxl nor these comparisons
qualify all Excel coercions. Classic error caches are checked as typed errors.
"""
from pathlib import Path
from datetime import datetime
import json
import math
import posixpath
import tempfile
import zipfile
from lxml import etree
from openpyxl import load_workbook

project = Path(__file__).resolve().parents[2]
root = Path(tempfile.gettempdir()) / 'folio-date-system-tests'
ui = Path(tempfile.gettempdir()) / 'folio-date-system-ui-tests'
expected = json.loads((project / 'test/fixtures/date-systems.json').read_text())
ns = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}

def first_sheet(z):
    book = etree.fromstring(z.read('xl/workbook.xml'))
    first = book.find('s:sheets/s:sheet', ns)
    rid = first.get('{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id')
    rels = etree.fromstring(z.read('xl/_rels/workbook.xml.rels'))
    target = next(r.get('Target') for r in rels if r.get('Id') == rid)
    return target.lstrip('/') if target.startswith('/') else posixpath.normpath('xl/' + target)

def compare_reopened(before, after):
    with zipfile.ZipFile(before) as a, zipfile.ZipFile(after) as b:
        assert set(a.namelist()) == set(b.namelist())
        sheet = first_sheet(a)
        for name in a.namelist():
            if name != sheet:
                assert a.read(name) == b.read(name), name
                continue
            parts = []
            for z in (a, b):
                xml = etree.fromstring(z.read(name))
                # This is the only intentionally volatile formula in the fixture.
                for v in xml.xpath('//s:c[@r="L1"]/s:v', namespaces=ns): v.text = 'VOLATILE'
                parts.append(etree.tostring(xml, method='c14n'))
            assert parts[0] == parts[1], 'nonvolatile worksheet changed on reopen'

for year in (1900, 1904):
    name = f'calendar-{year}.xlsx'
    formulas = load_workbook(root / name)
    cache = load_workbook(root / name, data_only=True).active
    source = load_workbook(project / f'test/fixtures/date-system-{year}.xlsx').active
    assert (formulas.epoch.year == 1904) == (year == 1904)
    sheet = formulas.active
    e = expected[f'date-system-{year}']
    with zipfile.ZipFile(root / name) as z:
        xml = etree.fromstring(z.read(first_sheet(z)))
        for a, serial in e['dateSerials'].items():
            assert float(xml.xpath(f'string(//s:c[@r="{a}"]/s:v)', namespaces=ns)) == serial, (year, a)
            assert sheet[a].number_format == source[a].number_format
        assert float(xml.xpath('string(//s:c[@r="D2"]/s:v)', namespaces=ns)) == e['dateSerials']['A3']
    for a, value in e['expected'].items():
        assert sheet[a].value == source[a].value, (year, a, 'formula')
        actual = cache[a].value
        if isinstance(value, (int, float)):
            assert cache[a].data_type == 'n' and math.isclose(actual, value, rel_tol=1e-10, abs_tol=1e-10), (year, a, actual, value)
        else:
            assert actual == value, (year, a, actual, value)
            if value in ('#NUM!', '#VALUE!', '#N/A', '#REF!', '#DIV/0!', '#NAME?', '#NULL!'):
                assert cache[a].data_type == 'e', (year, a, 'error cache type')
    assert sheet['D2'].value == datetime(1904, 2, 29) and sheet['D2'].data_type == 'd'
    assert sheet['D4'].value == .25 and sheet['D4'].data_type == 'n'
    assert sheet['D5'].value is True and sheet['D5'].data_type == 'b'
    for a, text in [('M1', '1904-01-01'), ('M2', '25%'), ('M3', '$12.50')]:
        assert sheet[a].value == text and sheet[a].data_type == 's', (year, a)
    compare_reopened(root / name, root / f'calendar-{year}-reopened.xlsx')

for kind in ('table', 'autoFilter'):
    book = load_workbook(root / f'calendar-{kind}-structure.xlsx')
    assert book.epoch.year == 1904
    sheet = book.active
    rule = next(iter(sheet.tables.values())).autoFilter if kind == 'table' else sheet.auto_filter
    assert rule.ref == 'P1:Q5'
    assert len(rule.filterColumn) == 1 and rule.filterColumn[0].colId == 1
    assert rule.filterColumn[0].filters.filter == ['1904']
    assert [r for r in range(2, 6) if sheet.row_dimensions[r].hidden] == [3, 4, 5]
    cached = load_workbook(root / f'calendar-{kind}-structure.xlsx', data_only=True).active
    assert cached['Q2'].value == 1904 and cached['Q3'].value == 1905

book = load_workbook(ui / 'calendar.xlsx'); sheet = book.active
assert book.epoch.year == 1904 and sheet['D2'].value == datetime(1904, 2, 29)
assert load_workbook(ui / 'calendar.xlsx', data_only=True).active['F5'].value == 0
assert sheet.auto_filter.filterColumn[0].filters.filter == ['0.0']
assert [r for r in range(2, 8) if sheet.row_dimensions[r].hidden] == [3, 4, 5, 6, 7]
compare_reopened(ui / 'calendar.xlsx', ui / 'reopened.xlsx')
print('Date systems passed: paired calendar metadata/raw serials, 94 formula expectations, typed dates/numbers/booleans, preserved literal text, table/worksheet structural filtering, widget export, and save/reopen (only NOW cache excluded).')
