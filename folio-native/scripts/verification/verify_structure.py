"""Validate structural exports with openpyxl and lxml, outside Folio's reader."""
import sys
from pathlib import Path
from zipfile import ZipFile
from openpyxl import load_workbook
from lxml import etree as ET

root = Path(sys.argv[1])
fixture = Path(__file__).resolve().parents[2] / 'test/fixtures/structure.xlsx'
book = load_workbook(root / 'rows.xlsx')
s = book['Budget']
assert s['B4'].value == 2400 and s['B6'].value == '=SUM(B4:B5)'
assert s['D5'].value == '=C5*2'  # materialized shared follower
assert s['B4'].font.bold and s['B4'].number_format == '"$"#,##0.00'
assert s.row_dimensions[4].height == 30
assert s.column_dimensions['B'].width == 22
assert s['A4'].comment is not None and s['A2'].comment is None
assert s.freeze_panes == 'B4' and s.auto_filter.ref == 'A1:C6'
assert str(s.merged_cells) == 'E1:F1'
assert str(s.data_validations.dataValidation[0].sqref) == 'B4:C5'
assert [str(key.sqref) for key in s.conditional_formatting] == ['B4:C5']
assert s.conditional_formatting[next(iter(s.conditional_formatting))][0].formula == ['$B$4']
assert book.defined_names['Costs'].attr_text == 'Budget!$B$4:$C$5'
assert s.defined_names['LocalCosts'].attr_text == 'Budget!$B$4:$B$5'
assert s.print_title_rows == '$1:$1' and s.print_title_cols == '$A:$A'
assert s._charts[0].series[0].val.numRef.f == 'Budget!$B$4:$B$5'
assert s._charts[0].anchor._from.row == 3
assert book['Inputs']['B1'].value == '=SUM(Budget!B4:B5)'
assert book['Inputs']['C1'].hyperlink.location == 'Budget!B4'

book = load_workbook(root / 'renamed-moved.xlsx')
assert book.sheetnames == ['Inputs', "O'Brien budget"]
assert book['Inputs']['B1'].value == "=SUM('O''Brien budget'!B4:B5)"
assert 'LocalCosts' in book["O'Brien budget"].defined_names
assert book.defined_names['Costs'].attr_text == "'O''Brien budget'!$B$4:$C$5"
for file in ['deleted-row.xlsx', 'reopened.xlsx']:
    book = load_workbook(root / file)
    s = book["O'Brien budget"]
    assert s['B4'].value == 7600 and s['B5'].value == '=SUM(B4:B4)'
    assert s['A4'].comment is None
    assert s.freeze_panes == 'B4'
    assert s.auto_filter.ref == 'A1:C5'
    assert str(s.data_validations.dataValidation[0].sqref) == 'B4:C4'
    assert s._charts[0].series[0].val.numRef.f == "'O''Brien budget'!$B$4:$B$4"
    rules = s.conditional_formatting[next(iter(s.conditional_formatting))]
    assert rules[0].formula == ['#REF!'] and rules[1].formula == ['B4>0']

book = load_workbook(root / 'columns.xlsx')
s = book['Budget']
assert s['D2'].value == 2400 and s['D4'].value == '=SUM(D2:D3)'
assert s['AC1'].value == '=Budget!D2*Inputs!A1'
assert s.column_dimensions['D'].width == 22
assert s.freeze_panes == 'D2' and s.auto_filter.ref == 'A1:E4'
assert str(s.merged_cells) == 'G1:H1'
assert s._charts[0].anchor._from.col == 9
assert s['A2'].comment is not None
book = load_workbook(root / 'deleted-column.xlsx')
assert book['Budget']['AB1'].value == '=#REF!*Inputs!A1'
book = load_workbook(root / 'deleted-sheet.xlsx')
assert book.sheetnames == ['Inputs']
assert book['Inputs']['B1'].value == '=SUM(#REF!)'
assert book.defined_names['Costs'].attr_text == '#REF!'
assert 'LocalCosts' not in book['Inputs'].defined_names
book = load_workbook(root / 'shared-edit.xlsx')
assert book['Budget']['D2'].value == 7 and book['Budget']['D3'].value == '=C3*2'
book = load_workbook(root / 'added-after-delete.xlsx')
assert len(book.sheetnames) == 3
assert book.worksheets[0]['A1'].value == 'survives'
assert book.worksheets[2]['B2'].value == 'new data'
assert book.calculation.fullCalcOnLoad and book.calculation.forceFullCalc

assert (root / 'undone.xlsx').read_bytes() == fixture.read_bytes()
with ZipFile(fixture) as original:
    for file in ['rows.xlsx', 'columns.xlsx', 'renamed-moved.xlsx', 'reopened.xlsx', 'deleted-sheet.xlsx']:
        with ZipFile(root / file) as edited:
            assert edited.testzip() is None
            for name in edited.namelist():
                if name.endswith(('.xml', '.rels', '.vml')):
                    ET.fromstring(edited.read(name))
            for name in ['xl/styles.xml', 'xl/theme/theme1.xml']:
                assert edited.read(name) == original.read(name), (file, name)
print('Independent structural export checks passed: cells, formulas, names, styles, comments, charts, panes, filters, validations, merges, Undo and saved-file recovery.')
