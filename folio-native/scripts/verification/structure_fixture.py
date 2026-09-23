"""Independent OOXML fixture for structural-edit acceptance."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from io import BytesIO
from openpyxl import load_workbook
from openpyxl.workbook.defined_name import DefinedName
from openpyxl.worksheet.datavalidation import DataValidation
from openpyxl.formatting.rule import CellIsRule, FormulaRule
from openpyxl.worksheet.hyperlink import Hyperlink
from openpyxl.styles import PatternFill
from lxml import etree as ET

root = Path(__file__).resolve().parents[2]
book = load_workbook(root / 'test/fixtures/source.xlsx')
sheet = book['Budget']
sheet.row_dimensions[2].height = 30
sheet.column_dimensions['B'].width = 22
sheet['D1'] = 'Doubled'
sheet['D2'] = '=C2*2'
sheet['D3'] = '=C3*2'
book['Inputs']['B1'] = '=SUM(Budget!B2:B3)'
book['Inputs']['B2'] = '=Budget!B2+Budget!$B$3'
book['Inputs']['C1'] = 'Budget link'
book['Inputs']['C1'].hyperlink = Hyperlink(ref='C1', location="'Budget'!B2")
book.defined_names.add(DefinedName('Costs', attr_text="'Budget'!$B$2:$C$3"))
sheet.defined_names.add(DefinedName('LocalCosts', attr_text="'Budget'!$B$2:$B$3"))
sheet.print_area = 'A1:D4'
sheet.print_title_rows = '1:1'
sheet.print_title_cols = 'A:A'
validation = DataValidation(type='whole', operator='between', formula1='1', formula2='10000')
sheet.add_data_validation(validation)
validation.add('B2:C3')
sheet.conditional_formatting.add('B2:C3', CellIsRule(operator='greaterThan', formula=['$B$2'], fill=PatternFill('solid', fgColor='FFFF0000')))
sheet.conditional_formatting.add('B2:C3', FormulaRule(formula=['B2>0'], fill=PatternFill('solid', fgColor='FF00FF00')))
buffer = BytesIO()
book.save(buffer)
ns = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with ZipFile(BytesIO(buffer.getvalue())) as original, ZipFile(root / 'test/fixtures/structure.xlsx', 'w', ZIP_DEFLATED) as out:
    for name in original.namelist():
        data = original.read(name)
        if name == 'xl/worksheets/sheet1.xml':
            doc = ET.fromstring(data)
            master = doc.find('.//s:c[@r="D2"]/s:f', ns)
            master.set('t', 'shared'); master.set('si', '0'); master.set('ref', 'D2:D3')
            follower = doc.find('.//s:c[@r="D3"]/s:f', ns)
            follower.text = None; follower.set('t', 'shared'); follower.set('si', '0')
            data = ET.tostring(doc)
        out.writestr(name, data)
print('Wrote structure.xlsx')
