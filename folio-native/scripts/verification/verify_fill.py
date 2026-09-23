"""Check fill exports through openpyxl and XML, independently of Folio."""
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
from openpyxl import load_workbook
from lxml import etree
folder=Path(gettempdir())/'folio-fill-tests'
source=load_workbook(folder/'original.xlsx',rich_text=True)
for filename in ['filled.xlsx','reopened.xlsx','reimported.xlsx','redone.xlsx']:
    w=load_workbook(folder/filename,rich_text=True); s=w['Input']
    for row in range(2,6):
        assert s[f'B{row}'].value==f'=A{row}*$H$1'
        assert s[f'B{row}']._style==source['Input']['B2']._style
        assert s[f'C{row}'].value==source['Input']['C2'].value
        assert s[f'D{row}'].value=='007' and s[f'D{row}'].data_type=='s'
        assert s[f'E{row}'].value is None and s[f'E{row}'].style_id==0
    assert s.row_dimensions[3].hidden and s.row_dimensions[5].height==41
    assert s.column_dimensions['B'].width==25
    assert str(s.merged_cells)=='J2:K2'
    assert 'M2' in s.array_formulae
    cached=load_workbook(folder/filename,data_only=True)
    assert cached['Input']['B5'].value==30
    assert cached['Totals']['A1'].value==84
    with ZipFile(folder/filename) as package:
        for part in package.namelist():
            if part.endswith('.xml') or part.endswith('.rels'): etree.fromstring(package.read(part))
        ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
        xml=etree.fromstring(package.read('xl/worksheets/sheet1.xml'))
        assert not xml.findall('.//s:f[@t="shared"]',ns)
        assert xml.find('.//s:c[@r="B5"]/s:f',ns).text=='A5*$H$1'
        assert xml.find('.//s:c[@r="C5"]',ns).get('{urn:folio:fill:metadata}flag')=='retained'
assert (folder/'original.xlsx').read_bytes()==(folder/'undone.xlsx').read_bytes()
for name in ['reopened.xlsx','reimported.xlsx','redone.xlsx']:
    assert (folder/name).read_bytes()==(folder/'filled.xlsx').read_bytes()
print('Verified translated formulas and caches, complete source formats, rich text, literal numeric strings, blank/style clearing, geometry, untouched objects, reopen/reimport/Redo and exact Undo bytes.')
inherited=load_workbook(folder/'inherited.xlsx',rich_text=True)['Input']
for row in [3,4]:
    assert inherited[f'P{row}'].font.name=='Georgia' and inherited[f'P{row}'].font.sz==16 and inherited[f'P{row}'].font.i
for row in [8,9]:
    assert inherited[f'R{row}'].font.name=='Verdana' and inherited[f'R{row}'].font.sz==14 and inherited[f'R{row}'].font.b
assert inherited['B5']._style==source['Input']['B2']._style
assert inherited['C5'].value==source['Input']['C2'].value
print('Verified prefixed worksheet records and copied inherited row/column styles.')
