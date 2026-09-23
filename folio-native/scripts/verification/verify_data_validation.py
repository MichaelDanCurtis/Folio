#!/usr/bin/env python3
"""Independent XLSX structure/reader checks; does not emulate Excel validation."""
import json
import tempfile
from pathlib import Path
from zipfile import ZipFile
from xml.etree import ElementTree as ET
import openpyxl

ROOT = Path(tempfile.gettempdir()) / 'folio-validation-tests'
UI = Path(tempfile.gettempdir()) / 'folio-validation-ui-tests'
NS = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
checked = []
def read(path):
    book = openpyxl.load_workbook(path)
    with ZipFile(path) as package:
        for name in package.namelist():
            if name.startswith('xl/worksheets/') and name.endswith('.xml'):
                root = ET.fromstring(package.read(name))
                parent = root.find('s:dataValidations', NS)
                if parent is not None:
                    rules = parent.findall('s:dataValidation', NS)
                    assert int(parent.attrib['count']) == len(rules), name
                    order = [n.tag.split('}')[-1] for n in root]
                    for later in ['hyperlinks','pageMargins','tableParts','extLst']:
                        if later in order:
                            assert order.index('dataValidations') < order.index(later)
                    for rule in rules:
                        assert rule.attrib['sqref']
                        assert len(rule.findall('s:formula1', NS)) <= 1
    checked.append(str(path))
    return book

def at(sheet, address):
    matches = [r for r in sheet.data_validations.dataValidation if address in r]
    assert len(matches) == 1, (address, len(matches))
    return matches[0]

w = read(ROOT/'authored.xlsx'); a = w.worksheets[0]
assert at(a,'A2').type == 'custom'
assert at(a,'A2').formula1 == 'A2=ROW()'
assert at(a,'A2').prompt == "Use this row's number"
assert at(a,'B8').formula1 == '"Red,Green,Blue"'
assert at(a,'B8').showDropDown is False
w = read(ROOT/'edited.xlsx'); a = w.worksheets[0]
assert at(a,'A6').formula1 == 'A3=ROW()'
assert at(a,'A4').type == 'whole'
assert at(a,'A4').formula2 == '10'
assert all(r.imeMode == 'disabled' for r in a.data_validations.dataValidation)
assert (ROOT/'edited.xlsx').read_bytes() == (ROOT/'reopened.xlsx').read_bytes()
read(ROOT/'reopened.xlsx')
w = read(ROOT/'copied.xlsx'); a = w.worksheets[0]
assert at(a,'C5').formula1 == 'C3=ROW()'
assert at(a,'A5').formula1 == 'A2=ROW()'
assert at(a,'B2').type == 'list' and a['B2'].value == 1
assert not any('B1' in r for r in a.data_validations.dataValidation)
assert at(w.worksheets[1],'D4').formula1 == 'D4=ROW()'
w = read(ROOT/'calendar.xlsx'); a = w.worksheets[0]
assert w.epoch.year == 1904
assert at(a,'Z1').formula1 == 'DATE(1904,1,1)'
w = read(UI/'authored.xlsx'); a = w.worksheets[0]
assert a['A1'].value == 3 and a['B1'].value == 'Green' and a['D1'].value == 'Medium'
assert at(a,'A1').type == 'whole' and at(a,'A1').errorStyle == 'stop'
assert at(a,'B1').errorStyle == 'warning'
assert at(a,'D1').formula1 == '"Small,Medium,Large"'
assert at(a,'D1').showDropDown is False and at(a,'D1').showInputMessage is True
assert at(a,'D1').promptTitle == 'Size' and at(a,'D1').prompt == 'Choose a size.'
print(json.dumps({'passed':True,'reader':'openpyxl + independent XML assertions','files':checked,'limitation':'No live Microsoft Excel comparison or formula evaluation.'},indent=2))
