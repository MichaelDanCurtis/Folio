"""Independent Excel fixture for directional fill and source-style retention."""
from pathlib import Path
from io import BytesIO
from zipfile import ZipFile, ZIP_DEFLATED
from openpyxl import Workbook
from openpyxl.styles import Font, PatternFill, Border, Side, Alignment, Protection
from openpyxl.cell.rich_text import CellRichText, TextBlock
from openpyxl.cell.text import InlineFont
from lxml import etree
out=Path(__file__).resolve().parents[2]/'test/fixtures/fill.xlsx'
w=Workbook(); s=w.active; s.title='Input'
for row in range(2,6):
    s.cell(row,1,row); s.cell(row,2,'old'); s.cell(row,3,'replace'); s.cell(row,4,'replace'); s.cell(row,5,'erase')
s['H1']=6
s['B2']='=A2*$H$1'; s['B3']='=A3*$H$1'
s['B2'].font=Font(name='Cambria',size=18,bold=True,italic=True,underline='double',strike=True,color='FF13579B')
s['B2'].fill=PatternFill('solid',fgColor='FFE2EFDA')
s['B2'].border=Border(left=Side(style='medium',color='FF224466'),bottom=Side(style='double',color='FF224466'))
s['B2'].alignment=Alignment(horizontal='right',vertical='center',wrap_text=True,text_rotation=15)
s['B2'].protection=Protection(locked=False,hidden=True)
s['B2'].number_format='#,##0.000'
s['C2']=CellRichText([TextBlock(InlineFont(b=True,color='FFAA0000'),'Rich'),' title'])
s['D2']='007'; s['E2']=None
s['E5'].fill=PatternFill('solid',fgColor='FFFF0000'); s['E5'].font=Font(name='Arial',size=22,bold=True)
s.row_dimensions[3].hidden=True; s.row_dimensions[5].height=41; s.column_dimensions['B'].width=25
s.column_dimensions['P'].font=Font(name='Georgia',size=16,italic=True)
s['P2']='Column style'; s['P3']='old'; s['P4']='old'
s.row_dimensions[7].font=Font(name='Verdana',size=14,bold=True)
s['R7']='Row style'; s['R8']='old'; s['R9']='old'
s.merge_cells('J2:K2'); s['J2']='Merged'; s['M2']='=ROW(M2:M4)'
t=w.create_sheet('Totals'); t['A1']='=SUM(Input!B2:B5)'
b=BytesIO(); w.save(b)
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with ZipFile(b) as source, ZipFile(out,'w',ZIP_DEFLATED) as target:
    for name in source.namelist():
        data=source.read(name)
        if name=='xl/worksheets/sheet1.xml':
            root=etree.fromstring(data)
            for a in ['B2','B3']:
                f=root.find(f'.//s:c[@r="{a}"]/s:f',ns); f.set('t','shared'); f.set('si','7')
                if a=='B2': f.set('ref','B2:B3')
                else: f.text=None
            f=root.find('.//s:c[@r="M2"]/s:f',ns); f.set('t','array'); f.set('ref','M2:M4')
            row=root.find('.//s:row[@r="2"]',ns)
            scoped=etree.Element(row.tag,attrib=dict(row.attrib),nsmap={'local':'urn:folio:fill:metadata'})
            for cell in list(row): scoped.append(cell)
            row.getparent().replace(row,scoped)
            scoped.find('s:c[@r="C2"]',ns).set('{urn:folio:fill:metadata}flag','retained')
            data=etree.tostring(root)
        target.writestr(name,data)
print(out)

# An equivalent package with explicit worksheet prefixes tests fragment copying.
import re
with ZipFile(out) as source, ZipFile(out.with_name('fill-prefixed.xlsx'),'w',ZIP_DEFLATED) as target:
    for name in source.namelist():
        data=source.read(name)
        if name=='xl/worksheets/sheet1.xml':
            data=data.replace(b'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"',b'xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"')
            data=re.sub(rb'(<[/]?)([A-Za-z][A-Za-z0-9]*)(?=[ > /])',rb'\1x:\2',data)
            etree.fromstring(data)
        target.writestr(name,data)
