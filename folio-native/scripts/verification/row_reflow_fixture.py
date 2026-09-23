"""Independent automatic/manual-height fixture, including OOXML-only flags."""
from pathlib import Path
from io import BytesIO
from zipfile import ZipFile, ZIP_DEFLATED
from lxml import etree
from openpyxl import Workbook
from openpyxl.styles import Alignment, Font
from openpyxl.comments import Comment
from openpyxl.cell.rich_text import CellRichText, TextBlock
from openpyxl.cell.text import InlineFont

root = Path(__file__).resolve().parents[2]
w = Workbook()
s = w.active
s.title = 'Notes'
s.append(['Row sizing', 'Content'])
s.column_dimensions['A'].width = 20
s.column_dimensions['B'].width = 22
s.column_dimensions['C'].width = 16
s.row_dimensions[1].height = 24
s['A2'], s['B2'] = 'Automatic', 'Short'
s['A3'], s['B3'] = 'Manual', 'Fixed height'
s.row_dimensions[3].height = 18
s['A4'], s['B4'] = 'Rich text', CellRichText('Small ', TextBlock(InlineFont(rFont='Georgia', sz=20, b=True), 'large'))
s['A5'], s['B5'] = 'Hidden', 'Hidden lines'
s.row_dimensions[5].height = 22
s.row_dimensions[5].hidden = True
s['A7'], s['B7'] = 'Formula', "=Inputs!A1"
s['A8'], s['B8'] = 'Merged', 'Do not refit merges'
s.merge_cells('B8:C8')
s.row_dimensions[8].height = 20
s['B10'] = 'A long unwrapped value stays on one line'
s['B200'] = 'Offscreen text wraps when the column gets narrower'
s['B1048576'] = 'Last row stays sparse'
for a in ['B2','B3','B4','B5','B7','B8','B200','B1048576']:
    s[a].alignment = Alignment(wrap_text=True, vertical='top')
    if a != 'B4': s[a].font = Font(name='Georgia', size=11)
s['B2'].comment = Comment('Retain the source note.', 'Fixture')
s.freeze_panes = 'A2'
i = w.create_sheet('Inputs')
i['A1'] = 'Start'
i['A1'].alignment = Alignment(wrap_text=True)
i.column_dimensions['A'].width = 24
buffer = BytesIO()
w.save(buffer)
ns = {'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with ZipFile(BytesIO(buffer.getvalue())) as z:
    parts = {name:z.read(name) for name in z.namelist()}
xml = etree.fromstring(parts['xl/worksheets/sheet1.xml'])
for row in [5,8]:
    xml.find(f's:sheetData/s:row[@r="{row}"]', ns).attrib.pop('customHeight', None)
parts['xl/worksheets/sheet1.xml'] = etree.tostring(xml)
with ZipFile(root/'test/fixtures/row-reflow.xlsx', 'w', ZIP_DEFLATED) as z:
    for name,data in parts.items(): z.writestr(name,data)
print('Created automatic, manual, hidden, merged, rich, formula and sparse-row fixture.')
