"""Read exported AutoFilter packages with independent XLSX and XML readers."""
from pathlib import Path
from zipfile import ZipFile
import json,sys,warnings
from openpyxl import load_workbook
from openpyxl.cell.rich_text import CellRichText
from lxml import etree as ET
warnings.filterwarnings('ignore',message='Unknown extension is not supported and will be removed')
folder=Path(sys.argv[1]);ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main','f':'urn:folio:worksheet-visibility:1'}
def inspect(name):
    with ZipFile(folder/name) as z:
        for part in z.namelist():
            if part.endswith(('.xml','.rels')):ET.fromstring(z.read(part))
        d=ET.fromstring(z.read('xl/worksheets/sheet1.xml'))
        wb=ET.fromstring(z.read('xl/workbook.xml'))
    return load_workbook(folder/name,rich_text=True),d,wb
for name in ['filtered-range.xlsx','filtered-reopened.xlsx']:
    b,d,wb=inspect(name);s=b['Records']
    assert s.auto_filter.ref=='A1:D7' and not s.tables
    f=s.auto_filter.filterColumn[0];assert f.colId==0 and f.filters.filter==['Pear','Hidden']
    assert [r for r in range(1,8) if s.row_dimensions[r].hidden]==[3,4,6]
    assert isinstance(s['A2'].value,CellRichText) and s['A2'].value[0].font.b
    assert s['A2'].comment.text=='First pear record' and s['A2'].hyperlink.location=="'Summary'!A1"
    assert s['B2'].font.bold and s['D2'].number_format=='$#,##0.00' and s.freeze_panes=='B2'
    assert all(s[f'E{r}'].value==f'fixed-{r}' for r in range(2,8))
    assert d.find('s:sheetPr',ns).get('filterMode')=='1'
    assert list(d).index(d.find('s:autoFilter',ns))<list(d).index(d.find('s:hyperlinks',ns))
    name_node=wb.find("s:definedNames/s:definedName[@name='_xlnm._FilterDatabase']",ns)
    assert name_node.text=="'Records'!$A$1:$D$7" and name_node.get('localSheetId')=='0' and name_node.get('hidden')=='1'
    state=json.loads(d.find('.//f:visibility',ns).text)
    assert state['manual']==[4] and state['range']['rows']==[3,6]
    cached=load_workbook(folder/name,data_only=True)
    assert [cached['Summary'][a].value for a in ['A1','C1','D1','E1','F1']]==[239,224,26,140,110]
    with ZipFile(folder/name) as z:assert not any(p.startswith('xl/tables/') for p in z.namelist())
b,d,wb=inspect('filtered-conditions.xlsx')
f=b['Records'].auto_filter.filterColumn[1];assert f.colId==1 and d.find("s:autoFilter/s:filterColumn[@colId='1']/s:customFilters",ns).get('and')=='1'
assert [(c.operator,c.val) for c in f.customFilters.customFilter]==[('greaterThan','1'),('lessThan','90')]
for name in ['filtered-cleared.xlsx','filtered-removed.xlsx']:
    b,d,wb=inspect(name);s=b['Records']
    assert [r for r in range(1,8) if s.row_dimensions[r].hidden]==[4]
    assert d.find('s:sheetPr',ns).get('filterMode') is None
    if 'cleared' in name:assert s.auto_filter.ref=='A1:D7' and not s.auto_filter.filterColumn
    else:
        assert s.auto_filter.ref is None
        assert wb.find("s:definedNames/s:definedName[@name='_xlnm._FilterDatabase']",ns) is None
b,d,wb=inspect('filtered-sorted.xlsx');s=b['Records']
assert [s[f'B{r}'].value for r in [2,3,4,5,6,7]]==[1,2,99,1,7,3]
assert s.auto_filter.ref=='A1:D7' and d.find('s:sortState',ns).get('ref')=='A2:D7'
assert list(d).index(d.find('s:autoFilter',ns))<list(d).index(d.find('s:sortState',ns))
b,d,wb=inspect('filtered-structure.xlsx');s=b["Order's data"]
assert s.auto_filter.ref=='B1:E8' and s.auto_filter.filterColumn[0].colId==0
assert [r for r in range(1,9) if s.row_dimensions[r].hidden]==[4,5,7]
assert wb.find("s:definedNames/s:definedName[@name='_xlnm._FilterDatabase']",ns).text=="'Order''s data'!$B$1:$E$8"
b,d,wb=inspect('filtered-header-deleted.xlsx');assert b["Order's data"].auto_filter.ref is None
assert wb.find("s:definedNames/s:definedName[@name='_xlnm._FilterDatabase']",ns) is None
b,d,wb=inspect('filtered-table-manual.xlsx');state=json.loads(d.find('.//f:visibility',ns).text)
assert state['manual']==[4] and state['tables'][0]['rows']==[3,6]
assert (folder/'filter-before.xlsx').read_bytes()==(folder/'filter-undone.xlsx').read_bytes()
assert (folder/'filtered-range.xlsx').read_bytes()==(folder/'filtered-reopened.xlsx').read_bytes()
b,d,wb=inspect('filter-prefixed.xlsx');s=b['Records']
assert s['G15'].value=='New cell' and s['D2'].value=='=B2*C2' and str(s['A2'].value)=='Pear'
assert s.auto_filter.ref=='A1:D7' and s.auto_filter.filterColumn[0].filters.filter==['Pear','Hidden']
assert s['A2'].comment.text=='First pear record' and s['B2'].font.bold
assert load_workbook(folder/'filter-prefixed.xlsx',data_only=True)['Summary']['C1'].value==224
assert all(ET.QName(n).namespace==ns['s'] for n in d.iter() if ET.QName(n).localname in ['c','f','v','row','autoFilter','filterColumn'])
print('Independent AutoFilter checks passed: range criteria, visibility, subtotal caches, hidden-row provenance, source formatting/notes/links, schema order, defined names, structure, clearing/removal, save/reopen and exact Undo bytes.')
