"""Reopen Folio's exports with independent Office readers and inspect retained objects."""
from pathlib import Path
from zipfile import ZipFile
import sys
from openpyxl import load_workbook
from openpyxl.cell.rich_text import CellRichText
from lxml import etree as ET

folder=Path(sys.argv[1]); root=Path(__file__).resolve().parents[2]
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}

def book(name):
    with ZipFile(folder/name) as z:
        for path in z.namelist():
            if path.endswith(('.xml','.rels')): ET.fromstring(z.read(path))
    return load_workbook(folder/name,rich_text=True)

b=book('filtered.xlsx');s=b['Orders'];t=s.tables['Sales']
assert s.row_dimensions[3].hidden and s.row_dimensions[9].hidden
assert not s.row_dimensions[2].hidden
assert t.autoFilter.filterColumn[0].filters.filter==['Pear']
assert t.ref=='A1:D5' and t.autoFilter.ref=='A1:D4' and t.totalsRowCount==1
assert t.tableColumns[3].calculatedColumnFormula.attr_text=='[@Qty]*[@Price]'
assert s['D5'].value=='=SUBTOTAL(109,Sales[Amount])'
b=book('sorted.xlsx');s=b['Orders'];t=s.tables['Sales']
assert str(s['A2'].value)=='Apple' and str(s['A4'].value)=='Pear'
assert s['B4'].value==3 and s['B4'].font.bold
assert s['A4'].comment.text=='Record one' and s['A2'].comment is None
assert s['A4'].hyperlink.location=="'Summary'!A1" and s['A2'].hyperlink is None
assert isinstance(s['A4'].value,CellRichText) and s['A4'].value[0].font.b
assert s.row_dimensions[2].hidden and not s.row_dimensions[3].hidden and s.row_dimensions[9].hidden
assert [c.ref for c in t.sortState.sortCondition]==['A2:A4','B2:B4']
assert s['D4'].value=='=[@Qty]*[@Price]' and s.freeze_panes=='B2'
for name in ['renamed.xlsx','reopened.xlsx']:
    b=book(name);s=b['Orders'];t=s.tables['OrdersTable']
    assert t.tableColumns[3].name=='Net amount'
    assert b['Summary']['A1'].value=='=SUM(OrdersTable[Net amount])'
    assert b.defined_names['SalesAmounts'].attr_text=='OrdersTable[Net amount]'
    assert s['A4'].comment.text=='Record one'
b=book('structure.xlsx');s=b['Orders'];t=s.tables['Sales']
assert t.ref=='A1:E6' and len(t.tableColumns)==5
assert [c.name for c in t.tableColumns]==['Product','Column2','Qty','Price','Amount']
assert s['E3'].value=='=[@Qty]*[@Price]' and s['A3'].value=='Peach'
assert s['C3'].value==2 and s['D3'].value==10
assert s.row_dimensions[10].hidden
b=book('deleted-column.xlsx');s=b['Orders'];t=s.tables['Sales']
assert len(t.tableColumns)==4 and '#REF!' in s['D2'].value and '#REF!' in t.tableColumns[3].calculatedColumnFormula.attr_text
b=book('deleted-table.xlsx');assert not b['Orders'].tables and b['Summary']['A1'].value=='=SUM(#REF!)'
b=book('added-table.xlsx');assert len(b.sheetnames)==3
s=b.worksheets[2];t=s.tables['NewSales'];assert t.ref=='A1:B3' and s['A3'].value==7
assert 'Units' in s['B3'].value and 'Units' in t.tableColumns[1].calculatedColumnFormula.attr_text
b=book('cleared.xlsx');assert not b['Orders'].row_dimensions[3].hidden and b['Orders'].row_dimensions[9].hidden
b=book('filtered-column-deletion.xlsx');s=b['Orders'];t=s.tables['Sales']
assert t.ref=='B1:E5' and [c.name for c in t.tableColumns]==['Qty','Column2','Price','Amount']
assert [c.ref for c in t.sortState.sortCondition]==['D2:D4','B2:B4']
assert t.sortState.sortCondition[0].descending and not t.sortState.sortCondition[1].descending
assert t.autoFilter.filterColumn[0].colId==0 and s.row_dimensions[2].hidden
book('custom-filter.xlsx')
b=book('totals.xlsx');s=b['Orders'];assert s['D5'].value=='=SUBTOTAL(101,Sales[Amount])'
assert s.tables['Sales'].tableColumns[3].totalsRowFunction=='average'
b=book('prefixed.xlsx');assert b['Orders'].tables['PrefixedSales'].tableColumns[1].name=='Units'
assert 'Units' in b['Orders']['D2'].value
b=book('deleted-table-sheet.xlsx');assert b.sheetnames==['Summary'] and b.defined_names['SalesAmounts'].attr_text=='#REF!' 
assert (folder/'undone.xlsx').read_bytes()==(root/'test/fixtures/tables.xlsx').read_bytes()
for name in ['resized-expanded.xlsx','resized-reopened.xlsx','resized-undone.xlsx']:
    b=book(name);s=b['Orders'];t=s.tables['Sales']
    assert t.ref=='A1:E8' and t.autoFilter.ref=='A1:E7' and t.totalsRowCount==1
    assert [c.name for c in t.tableColumns]==['Product','Qty','Price','Amount','Qty2']
    assert s['D6'].value=='=[@Qty]*[@Price]' and s['D7'].value==777
    assert s['D8'].value=='=SUBTOTAL(109,Sales[Amount])' and s['D8'].font.bold
    assert s['D8'].comment.text=='Visible order total' and s['D5'].comment is None
    assert s['D8'].hyperlink.location=="'Summary'!A2" and s['D5'].hyperlink is None
    assert s['A2'].comment.text=='Record one' and isinstance(s['A2'].value,CellRichText)
    assert all(s.row_dimensions[r].hidden for r in [3,5,7,9])
    assert not s.row_dimensions[6].hidden and not s.row_dimensions[8].hidden
b=book('resized-shrunk.xlsx');s=b['Orders'];t=s.tables['Sales']
assert t.ref=='A1:C3' and len(t.tableColumns)==3 and not t.totalsRowCount
assert s['D2'].value=='=B2*C2' and s['D6'].value=='=B6*C6' and s['D7'].value==777
assert s['A6'].value=='Pear' and not s.row_dimensions[5].hidden and not s.row_dimensions[7].hidden
assert b.defined_names['SalesAmounts'].attr_text=='#REF!'
b=book('resized-left.xlsx');s=b['Orders'];t=s.tables['Sales']
assert t.ref=='B1:D5' and [c.id for c in t.tableColumns]==[2,3,4]
assert [c.ref for c in t.sortState.sortCondition]==['C2:C4','B2:B4']
assert t.autoFilter.filterColumn[0].colId==0 and s.row_dimensions[2].hidden
assert str(s['A2'].value)=='Pear' and s['A3'].comment.text=='Record one'
assert (folder/'resized-undone.xlsx').read_bytes()==(folder/'resized-expanded.xlsx').read_bytes()
b=book('resize-custom-sort-expanded.xlsx');t=b['Orders'].tables['Sales']
assert t.sortState.ref=='A2:E7'
assert [(c.ref,c.customList) for c in t.sortState.sortCondition]==[('A2:A7','Pear,Apple'),('B2:B7','3,2,1')]
b=book('resize-custom-sort-narrowed.xlsx');t=b['Orders'].tables['Sales']
assert t.sortState.ref=='B2:D7' and len(t.sortState.sortCondition)==1
assert t.sortState.sortCondition[0].customList=='3,2,1' and t.sortState.sortCondition[0].descending
b=book('resize-a1-total.xlsx');s=b['Orders']
assert s['D8'].value=='=SUM($B$2:B4)' and s['D8'].comment.text=='Visible order total'
assert s.tables['Sales'].tableColumns[3].totalsRowFunction=='custom'
assert s.tables['Sales'].tableColumns[3].totalsRowFormula.attr_text=='SUM($B$2:B4)'
for name in ['custom-totals.xlsx','custom-totals-reopened.xlsx']:
    b=book(name);s=b['Orders'];t=s.tables['OrdersTable'];c=t.tableColumns[4]
    assert c.totalsRowFunction=='custom' and c.totalsRowFormula.attr_text=='SUM(OrdersTable[Net amount])+C2'
    assert s['E5'].value=='=SUM(OrdersTable[Net amount])+C2'
    with ZipFile(folder/name) as z:
        tree=ET.fromstring(z.read('xl/tables/table1.xml'))
        col=tree.findall('s:tableColumns/s:tableColumn',ns)[4]
        assert [ET.QName(child).localname for child in col][:2]==['calculatedColumnFormula','totalsRowFormula']
b=book('custom-totals-standard.xlsx');c=b['Orders'].tables['OrdersTable'].tableColumns[4]
assert c.totalsRowFunction=='average' and c.totalsRowFormula is None
b=book('custom-totals-label.xlsx');c=b['Orders'].tables['Sales'].tableColumns[3]
assert c.totalsRowFunction is None and c.totalsRowLabel=='Range of amounts' and c.totalsRowFormula is None
with ZipFile(folder/'custom-totals-label.xlsx') as z:
    col=ET.fromstring(z.read('xl/tables/table1.xml')).findall('s:tableColumns/s:tableColumn',ns)[3]
    assert col.attrib['totalsRowFunction']=='none'
with ZipFile(folder/'sorted.xlsx') as changed, ZipFile(root/'test/fixtures/tables.xlsx') as original:
    assert changed.read('xl/theme/theme1.xml')==original.read('xl/theme/theme1.xml')
print('Independent table checks passed: resize, sorting, filters, totals, rich text, notes, links, styles, names, structural edits, new tables, save/reload and exact Undo.')
