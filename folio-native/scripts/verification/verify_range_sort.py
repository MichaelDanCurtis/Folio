"""Check range exports using readers independent of the application."""
from pathlib import Path
from zipfile import ZipFile
import sys
from openpyxl import load_workbook
from openpyxl.cell.rich_text import CellRichText
from lxml import etree as ET

folder=Path(sys.argv[1]);root=Path(__file__).resolve().parents[2]
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
def book(name):
    with ZipFile(folder/name) as z:
        for part in z.namelist():
            if part.endswith(('.xml','.rels')): ET.fromstring(z.read(part))
    return load_workbook(folder/name,rich_text=True)
for name in ['range-sorted.xlsx','range-reopened.xlsx']:
    b=book(name);s=b['Records']
    assert not s.tables and s['A1'].value=='Label'
    assert str(s['A2'].value)=='Apple' and str(s['A4'].value)=='Hidden'
    assert s.row_dimensions[4].hidden and s['B4'].value==99 and s['D4'].value=='=B4*C4'
    assert (s['C3'].value,s['C5'].value)==(8,3)
    assert isinstance(s['A6'].value,CellRichText) and s['A6'].value[0].font.b
    assert s['A6'].comment.text=='First pear record' and s['A2'].comment is None
    assert s['A6'].hyperlink.location=="'Summary'!A1" and s['A2'].hyperlink is None
    assert s['B6'].font.bold and s['D6'].number_format=='$#,##0.00'
    assert s['D6'].value=='=B6*C6' and s['E6'].value=='fixed-6' and s.freeze_panes=='B2'
    assert b['Summary']['A1'].value=='=SUM(Records!D2:D7)'
    with ZipFile(folder/name) as z:
        d=ET.fromstring(z.read('xl/worksheets/sheet1.xml'))
        sort=d.find('s:sortState',ns);assert sort.attrib['ref']=='A2:D7'
        assert [k.attrib['ref'] for k in sort]==['A2:A7','B2:B7']
        assert list(d).index(sort)<list(d).index(d.find('s:hyperlinks',ns))
        assert not any(p.startswith('xl/tables/') for p in z.namelist())
b=book('range-headerless.xlsx');s=b['Records']
assert [s[f'H{r}'].value for r in [1,2,3,5]]==['b','a','B','A'] and s.row_dimensions[4].hidden
with ZipFile(folder/'range-headerless.xlsx') as z:
    d=ET.fromstring(z.read('xl/worksheets/sheet1.xml'));sort=d.find('s:sortState',ns)
    assert sort.attrib['ref']=='H1:H5' and sort.attrib['caseSensitive']=='1'
    assert sort[0].attrib['ref']=='H1:H5' and sort[0].attrib['descending']=='1'
assert (folder/'range-undone.xlsx').read_bytes()==(root/'test/fixtures/range-sort.xlsx').read_bytes()
assert (folder/'range-reopened.xlsx').read_bytes()==(folder/'range-sorted.xlsx').read_bytes()
print('Independent range sort checks passed: hidden rows, stable records, formulas, rich text, formatting, notes, links, source schema order, no synthetic tables, persistence and exact Undo.')
