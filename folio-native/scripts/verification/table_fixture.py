"""Independent Excel table fixture; no application-generated source data."""
from pathlib import Path
from openpyxl import Workbook
from openpyxl.worksheet.table import Table, TableColumn, TableFormula, TableStyleInfo
from openpyxl.worksheet.filters import AutoFilter
from openpyxl.worksheet.hyperlink import Hyperlink
from openpyxl.comments import Comment
from openpyxl.styles import Font
from openpyxl.workbook.defined_name import DefinedName
from openpyxl.cell.rich_text import CellRichText, TextBlock
from openpyxl.cell.text import InlineFont

root = Path(__file__).resolve().parents[2]
b = Workbook(); s = b.active; s.title = 'Orders'
for row in [('Product','Qty','Price','Amount'),('Pear',3,5,'=[@Qty]*[@Price]'),('Apple',2,4,'=[@Qty]*[@Price]'),('Pear',1,8,'=[@Qty]*[@Price]'),('Total',None,None,'=SUBTOTAL(109,Sales[Amount])')]: s.append(row)
s['A2'] = CellRichText(TextBlock(InlineFont(b=True), 'Pe'), 'ar')
s['A2'].comment = Comment('Record one', 'Folio test')
s['A2'].hyperlink = Hyperlink(ref='A2', location="'Summary'!A1")
s['B2'].font = Font(bold=True, color='FF003399')
s['D5'].font = Font(bold=True, color='FF006633')
s['D5'].comment = Comment('Visible order total', 'Folio test')
s['D5'].hyperlink = Hyperlink(ref='D5', location="'Summary'!A2")
s.row_dimensions[9].hidden = True
cols = [TableColumn(id=i+1,name=n) for i,n in enumerate(['Product','Qty','Price','Amount'])]
cols[3].calculatedColumnFormula = TableFormula(attr_text='[@Qty]*[@Price]')
cols[3].totalsRowFunction = 'sum'; cols[0].totalsRowLabel = 'Total'
t = Table(displayName='Sales',ref='A1:D5',tableColumns=cols,totalsRowCount=1,autoFilter=AutoFilter(ref='A1:D4'),tableStyleInfo=TableStyleInfo(name='TableStyleMedium2',showRowStripes=True))
s.add_table(t); s.freeze_panes='B2'
summary=b.create_sheet('Summary');summary['A1']='=SUM(Sales[Amount])';summary['A2']='=SUBTOTAL(109,Sales[Amount])'
b.defined_names.add(DefinedName('SalesAmounts',attr_text='Sales[Amount]'))
b.save(root/'test/fixtures/tables.xlsx')
print('Wrote tables.xlsx')
