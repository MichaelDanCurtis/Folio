"""Independent validation of the retained, actual macOS interaction artifacts."""
import argparse, json, warnings
from copy import deepcopy, copy
from pathlib import Path
from zipfile import ZipFile
from lxml import etree
from openpyxl import load_workbook

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('folder',type=Path)
p=parser.parse_args().folder
warnings.filterwarnings('ignore',message='Unknown extension is not supported and will be removed')
names=['before','grown','undone','redone','manual','fitted','shrunk','narrowed',
       'width-undone','width-redone','formula','formula-undone','before-restart',
       'after-restart','reimported','reimport-edited']
states={n:json.loads((p/f'{n}.folio').read_text()) for n in names}
def content(w): return {k:v for k,v in w.items() if k!='revision'}
for a,b in [('undone','before'),('redone','grown'),('width-undone','shrunk'),
            ('width-redone','narrowed'),('formula-undone','width-redone'),('before-restart','formula')]:
    assert content(states[a])==content(states[b]),a
assert [states[n]['revision'] for n in names] == list(range(1,14))+[13,14,15]
assert (p/'before-restart.folio').read_bytes()==(p/'after-restart.folio').read_bytes()

expected=deepcopy(states['before'])
s=expected['sheets'][0]
s['cells']['B2']='Native first\nNative second\nNative third'
s['rowHeights']['2']=44.25
s['rowHeightModes']['2']='auto'
assert content(expected)==content(states['grown'])
s['cells']['B3']='Manual first\nManual second\nManual third'
assert content(expected)==content(states['manual'])
s['rowHeights']['3']=44.25
s['rowHeightModes']['3']='auto'
assert content(expected)==content(states['fitted'])
s['cells']['B3']='Short'
s['rowHeights']['3']=18.75
assert content(expected)==content(states['shrunk'])
s['columnWidths']['2']=11.3125
for row,height in {'2':82.5,'3':18.75,'4':51,'7':18.75,'10':18.75,'200':82.5,'1048576':44.25}.items():
    s['rowHeights'][row]=height
    s['rowHeightModes'][row]='auto'
assert content(expected)==content(states['narrowed'])
formula='Formula first\nFormula second\nFormula third'
expected['sheets'][1]['cells']['A1']=formula
expected['sheets'][1]['rowHeightModes']={'1':'auto'}
source_height=states['formula']['sheets'][1]['rowHeights']['1']
assert source_height>30
expected['sheets'][1]['rowHeights']={'1':source_height}
s['rowHeights']['7']=82.5
assert content(expected)==content(states['formula'])
for original,reimported in zip(states['formula']['sheets'],states['reimported']['sheets']):
    assert {k:v for k,v in original.items() if k!='cached'} == {k:v for k,v in reimported.items() if k!='cached'}
assert states['reimported']['sheets'][0]['cached']['B7']==formula
edited=deepcopy(states['reimported'])
edited['sheets'][0]['cells']['B2']='Short'
edited['sheets'][0]['rowHeights']['2']=18.75
assert content(edited)==content(states['reimport-edited'])

export=p/'reflow-native.xlsx'
assert export.read_bytes()==(p/'reflow-reopened.xlsx').read_bytes()
source=Path(__file__).resolve().parents[2]/'test/fixtures/row-reflow.xlsx'
before=load_workbook(source,rich_text=True)
after=load_workbook(export,rich_text=True)
values=load_workbook(export,data_only=True)
for i,sheet in enumerate(after):
    spec=expected['sheets'][i]
    for row,height in spec['rowHeights'].items(): assert sheet.row_dimensions[int(row)].height==height
    for cell in before[sheet.title]._cells.values():
        other=sheet[cell.coordinate]
        target=spec['cells'].get(cell.coordinate)
        if (sheet.title,cell.coordinate) in [('Notes','B2'),('Notes','B3'),('Inputs','A1')]:
            assert other.value==target
        else: assert other.value==cell.value,(sheet.title,cell.coordinate)
        for prop in ['font','fill','border','alignment','protection','number_format']:
            assert copy(getattr(other,prop))==copy(getattr(cell,prop)),(cell.coordinate,prop)
    assert sheet.freeze_panes==before[sheet.title].freeze_panes
    assert str(sheet.merged_cells)==str(before[sheet.title].merged_cells)
assert values['Notes']['B7'].value==formula
assert after['Notes']['B2'].comment.text==before['Notes']['B2'].comment.text
assert after['Notes'].row_dimensions[5].hidden
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with ZipFile(export) as z, ZipFile(source) as old:
    assert z.testzip() is None
    assert set(old.namelist()).issubset(z.namelist())
    for part in old.namelist():
        if part not in ['xl/workbook.xml','xl/worksheets/sheet1.xml','xl/worksheets/sheet2.xml']:
            assert old.read(part)==z.read(part),part
    for part in set(z.namelist())-set(old.namelist()):
        assert part=='xl/worksheets/_rels/sheet2.xml.rels'
        assert len(etree.fromstring(z.read(part)))==0
    for i,sheet in enumerate(expected['sheets'],1):
        xml=etree.fromstring(z.read(f'xl/worksheets/sheet{i}.xml'))
        for row,mode in sheet['rowHeightModes'].items():
            node=xml.find(f's:sheetData/s:row[@r="{row}"]',ns)
            assert (node.get('customHeight') in ['1','true']) == (mode=='manual')
print('Native automatic growth/shrink, fixed-height preservation, context-menu AutoFit, column drag,')
print('cross-sheet formulas, keyboard/post-Save history, exact restart/export and reimport/edit checks passed.')
print('Independent Excel reader confirms row modes, content/caches, rich styles, notes, views and retained parts.')
