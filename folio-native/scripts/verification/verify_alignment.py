"""Read exported alignment independently with openpyxl and source XML."""
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
from copy import copy
from lxml import etree
from openpyxl import load_workbook
import sys

folder=Path(gettempdir())/'folio-alignment-tests'
source=Path(__file__).resolve().parents[2]/'test/fixtures/alignment.xlsx'
before=load_workbook(source)['Input']['A30']
after=load_workbook(folder/'edited.xlsx')['Input']['A30']
assert before.value==after.value
for name in ['font','fill','border','protection','number_format']: assert copy(getattr(before,name))==copy(getattr(after,name)),name
a=after.alignment
assert (a.horizontal,a.vertical,a.wrapText,a.shrinkToFit,a.textRotation,a.indent,a.readingOrder,a.justifyLastLine)==('center','center',False,True,120,0,0,False)
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with ZipFile(folder/'edited.xlsx') as z,ZipFile(source) as old:
    style=etree.fromstring(z.read('xl/styles.xml')).findall('s:cellXfs/s:xf',ns)[after.style_id]
    alignment=style.find('s:alignment',ns)
    assert style.get('applyAlignment')=='1'
    assert alignment.get('{urn:folio:alignment}horizontal')=='keep-qualified'
    assert alignment.get('relativeIndent')=='-2'
    assert z.read('xl/sharedStrings.xml')==old.read('xl/sharedStrings.xml')
inherited=load_workbook(folder/'inherited.xlsx')['Input']['C30'].alignment
assert (inherited.horizontal,inherited.vertical,inherited.indent,inherited.textRotation)==('right','center',1,90)
with ZipFile(folder/'inherited.xlsx') as z:
    assert etree.fromstring(z.read('xl/styles.xml')).xpath('count(s:cellXfs/s:xf/s:alignment[@m:source="keep-ignored"])',namespaces={**ns,'m':'urn:folio:alignment'})==2
rotated=load_workbook(folder/'rotations.xlsx')['Input']
assert [rotated[f'{chr(65+i)}40'].alignment.textRotation for i in range(6)]==[180,91,0,1,90,255]
assert load_workbook(folder/'new.xlsx').worksheets[0]['A1'].alignment.horizontal=='center'
assert (folder/'undone.xlsx').read_bytes()==source.read_bytes()
for name in ['redone.xlsx','reopened.xlsx']:assert (folder/name).read_bytes()==(folder/'edited.xlsx').read_bytes()
if '--with-ui' in sys.argv:
    ui=Path(gettempdir())/'folio-font-ui-tests'
    cells=load_workbook(ui/'alignment-ui.xlsx')['Input']
    for name in ['A30','B30']:
        a=cells[name].alignment
        assert (a.horizontal,a.vertical,bool(a.wrapText),bool(a.shrinkToFit),a.textRotation,a.indent,a.readingOrder)==('center','top',True,False,0,0,0)
    assert copy(cells['A30'].font)==copy(before.font)
    assert (ui/'alignment-ui-undone.xlsx').read_bytes()==source.read_bytes()
    for name in ['alignment-ui-redone.xlsx','alignment-ui-reopened.xlsx']:
        assert (ui/name).read_bytes()==(ui/'alignment-ui.xlsx').read_bytes()
    print('Verified actual Alignment dialog exports and exact Undo/Redo/reopening.')
print('Independent alignment, signed rotations, inheritance, retained styles/metadata and exact history/reopen checks passed.')
