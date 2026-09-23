"""Independently read numeric-width exports; display changes must not alter data."""
import argparse
import copy
from datetime import datetime
import json
import math
import posixpath
from pathlib import Path
import tempfile
import xml.etree.ElementTree as ET
import zipfile

import openpyxl


def verify(path):
    formulas = openpyxl.load_workbook(path, data_only=False)
    values = openpyxl.load_workbook(path, data_only=True)
    try:
        s, v = formulas.active, values.active
        assert s['A1'].value == 12345.6789
        assert s['A1'].number_format == '0.0000'
        assert s['A2'].value == '=A1/7'
        assert math.isclose(v['A2'].value, 12345.6789 / 7, abs_tol=1e-12)
        assert s['A3'].value == datetime(2023, 3, 15)
        assert s['A3'].number_format == s['A6'].number_format == 'yyyy-mm-dd'
        assert s['A4'].value == '123456789.012345' and s['A4'].data_type == 's'
        assert s['A5'].value == '=1/0'
        assert v['A5'].value == '#DIV/0!' and v['A5'].data_type == 'e'
        assert s['A7'].value == '=TEXT(A1,"0.0000")'
        assert v['A7'].value == '12345.6789' and v['A7'].data_type == 's'
        assert s['A8'].value == 12345.6789 and s['A8'].alignment.shrinkToFit
        assert s.column_dimensions['A'].width == 7
        assert s.column_dimensions['B'].width == 28
        assert {c.coordinate for row in s for c in row if c.value is not None} == {
            f'{col}{row}' for col in ['A', 'B'] for row in range(1, 9)
        }
        with zipfile.ZipFile(path) as package:
            book = ET.fromstring(package.read('xl/workbook.xml'))
            rels = ET.fromstring(package.read('xl/_rels/workbook.xml.rels'))
            relationship = book.find('{*}sheets/{*}sheet').attrib[
                '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id'
            ]
            target = next(r.attrib['Target'] for r in rels if r.attrib['Id'] == relationship)
            part = target.lstrip('/') if target.startswith('/') else posixpath.normpath(posixpath.join('xl', target))
            root = ET.fromstring(package.read(part))
        ns = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
        assert root.find(".//s:c[@r='A3']/s:v", ns).text == '45000'
        assert root.find(".//s:c[@r='A6']/s:v", ns).text == '-1'
    finally:
        formulas.close()
        values.close()
    print(f'PASS: {path.name}: full values, formulas/caches, dates, text/error types, Shrink to Fit and narrow widths are intact.')


def verify_native(folder):
    seed = json.loads((folder / 'seed.folio').read_text())
    assert seed['revision'] == 6
    for name, revision, width in [
        ('wide', 7, 30.625), ('undo', 8, 7), ('redo', 9, 30.625),
        ('autofit', 11, 18.5625), ('before-restart', 12, 7), ('after-restart', 12, 7),
    ]:
        expected = copy.deepcopy(seed)
        expected['revision'] = revision
        if width != 7:
            sheet = expected['sheets'][0]
            sheet['columnWidths']['1'] = width
            sheet['rowHeightModes'] = {str(r): 'auto' for r in range(1, 9)}
            sheet['rowHeights'] = {str(r): 24 for r in range(1, 9)}
        assert json.loads((folder / f'{name}.folio').read_text()) == expected, name
    assert (folder / 'before-restart.folio').read_bytes() == (folder / 'after-restart.folio').read_bytes()
    verify(folder / 'number-width-native.xlsx')
    print('PASS: native width, Undo/Redo, AutoFit, all saved workspace fields and exact restart bytes.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', nargs='?', type=Path,
                        default=Path(tempfile.gettempdir()) / 'folio-number-width-ui-tests/number-width.xlsx')
    parser.add_argument('--native-dir', type=Path)
    args = parser.parse_args()
    if args.native_dir:
        verify_native(args.native_dir)
    else:
        verify(args.path)
