"""Independent checks of column sizes, localized Normal, source fidelity and history."""
from copy import copy
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
import json, sys
from openpyxl import load_workbook
from lxml import etree

folder=Path(gettempdir())/'folio-column-fit-tests'
source=Path(__file__).resolve().parents[2]/'test/fixtures/column-autofit.xlsx'
before=load_workbook(source,rich_text=True)
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
def verify(path,widths):
    after=load_workbook(path,rich_text=True)
    a,b=after.active,before.active
    for col,width in widths.items():assert a.column_dimensions[col].width==width,(col,width)
    assert a.column_dimensions['D'].hidden and a.column_dimensions['D'].width==17
    for row in [1,2,3,4,5,6,7]:
        assert a.row_dimensions[row].height==b.row_dimensions[row].height
        assert a.row_dimensions[row].hidden==b.row_dimensions[row].hidden
    assert a.freeze_panes==b.freeze_panes
    for row in b:
        for cell in row:
            other=a[cell.coordinate]
            assert str(other.value)==str(cell.value),cell.coordinate
            for prop in ['font','fill','border','alignment','protection']:
                assert copy(getattr(other,prop))==copy(getattr(cell,prop)),(cell.coordinate,prop)
    assert a['B4'].value==b['B4'].value
    assert a['B2'].comment.text==b['B2'].comment.text
    assert after._named_styles['Standard'].font.name=='Georgia'
    assert after._named_styles['Standard'].font.sz==14
    assert abs(load_workbook(path,data_only=True).active['B10'].value-24691.34)<1e-8
    with ZipFile(path) as z,ZipFile(source) as old:
        assert z.testzip() is None
        assert z.read('xl/styles.xml')==old.read('xl/styles.xml')
        xml=etree.fromstring(z.read('xl/worksheets/sheet1.xml'))
        for col in xml.findall('s:cols/s:col',ns):
            if int(col.get('min')) in [1,2,3]:
                assert col.get('customWidth')=='1'
                assert col.get('bestFit') not in ['1','true']
for name in ['edited','redone','reopened']:
    verify(folder/f'{name}.xlsx',{'A':12.625,'B':40.25,'C':20.75})
    assert (folder/f'{name}.xlsx').read_bytes()==(folder/'edited.xlsx').read_bytes()
for name in ['undone','legacy-upgraded']:
    assert (folder/f'{name}.xlsx').read_bytes()==source.read_bytes()
assert load_workbook(folder/'last-column.xlsx').active.column_dimensions['XFD'].width==255
if '--with-ui' in sys.argv:
    ui=Path(gettempdir())/'folio-column-fit-ui-tests'
    widths=json.loads((ui/'widths.json').read_text())
    verify(ui/'fitted.xlsx',{col:widths[str(i)] for i,col in enumerate(['A','B','C'],1)})
    assert (ui/'undone.xlsx').read_bytes()==source.read_bytes()
    for name in ['redone','reopened']:
        assert (ui/f'{name}.xlsx').read_bytes()==(ui/'fitted.xlsx').read_bytes()
    with ZipFile(ui/'fitted.xlsx') as z:
        xml=etree.fromstring(z.read('xl/worksheets/sheet1.xml'))
        for r in [10,11,200]:
            node=xml.find(f's:sheetData/s:row[@r="{r}"]',ns)
            assert node.get('customHeight') not in ['1','true']
            assert float(node.get('ht')) > 18
    print('Verified column AutoFit menu exports, preserved fixed rows, refitted automatic rows and exact Undo/Redo/reopen.')
print('Independent fitted widths, localized Normal font, rich content, formula caches, hidden metadata and original-file recovery checks passed.')
