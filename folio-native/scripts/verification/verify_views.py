"""Read exported panes with openpyxl and independently inspect OOXML metadata."""
import sys
from pathlib import Path
from zipfile import ZipFile
from openpyxl import load_workbook
from lxml import etree as ET

p=Path(sys.argv[1])
root=Path(__file__).resolve().parents[2]
fixture=root/'test/fixtures/views.xlsx'
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with ZipFile(fixture) as z:
    original=ET.fromstring(z.read('xl/worksheets/sheet1.xml'))
    secondary=original.find('s:sheetViews',ns)[1]
    split=ET.fromstring(z.read('xl/worksheets/sheet2.xml')).find('s:sheetViews',ns)
for name, rows, cols, top in [('view-edited.xlsx',2,4,'E3'),('view-reopened.xlsx',2,4,'E3'),('view-structure.xlsx',5,3,'H24')]:
    w=load_workbook(p/name);s=w['Frozen'];pane=s.sheet_view.pane
    assert pane.ySplit==rows and pane.xSplit==cols and pane.topLeftCell==top
    assert pane.activePane=='bottomRight' and s.sheet_view.zoomScale==90
    assert len(s.views.sheetView)==2
    assert s.views.sheetView[1].pane.xSplit==(2 if name=='view-structure.xlsx' else 1)
    assert s.views.sheetView[1].zoomScale==70
    with ZipFile(p/name) as z:
        for path in z.namelist():
            if path.endswith(('.xml','.rels','.vml')): ET.fromstring(z.read(path))
        xml=ET.fromstring(z.read('xl/worksheets/sheet1.xml'))
        views=xml.find('s:sheetViews',ns)
        if name!='view-structure.xlsx':
            assert ET.tostring(views[1])==ET.tostring(secondary)
        assert ET.tostring(ET.fromstring(z.read('xl/worksheets/sheet2.xml')).find('s:sheetViews',ns))==ET.tostring(split)
        tags=[ET.QName(n).localname for n in xml]
        assert tags.index('sheetViews')<tags.index('sheetData')
        if 'sheetFormatPr' in tags: assert tags.index('sheetViews')<tags.index('sheetFormatPr')
        view_tags=[ET.QName(n).localname for n in views[0]]
        assert view_tags.index('pane')<view_tags.index('selection')
    if name!='view-structure.xlsx':
        assert s['A1'].font.bold and s['A1'].comment.text=='Preserve this note'
        assert s.row_dimensions[2].hidden and s.column_dimensions['B'].hidden
        assert s.row_dimensions[3].height==40 and s['D25'].value=='=B25+C25'
for name in ['view-unfrozen.xlsx','view-deleted-prefix.xlsx']:
    s=load_workbook(p/name)['Frozen']
    assert s.sheet_view.pane is None
    assert all(selection.pane is None for selection in s.sheet_view.selection)
columns=load_workbook(p/'view-columns-only.xlsx')['Frozen']
assert columns.sheet_view.pane.ySplit in (None,0)
assert columns.sheet_view.pane.xSplit==3 and columns.sheet_view.pane.activePane=='topRight'
assert all(selection.pane in (None,'topLeft','topRight') for selection in columns.sheet_view.selection)
assert load_workbook(p/'view-deleted-prefix.xlsx')['Frozen'].views.sheetView[1].pane is None
native=load_workbook(p/'view-native.xlsx').worksheets[0]
assert native.sheet_view.pane.ySplit==1 and native.sheet_view.pane.xSplit is None
prefixed=load_workbook(p/'view-prefixed.xlsx').worksheets[0]
assert prefixed.sheet_view.pane.xSplit==1 and prefixed.sheet_view.pane.ySplit is None
assert (p/'view-undone.xlsx').read_bytes()==fixture.read_bytes()
assert (p/'view-legacy.xlsx').read_bytes()==fixture.read_bytes()
print('Independent pane checks passed: counts, scroll origins, other views, ordinary splits, XML order, source objects, unfreeze, Undo and reopen.')
