"""Independent dimension, visibility, metadata and recovery verification."""
import sys, warnings
from pathlib import Path
from zipfile import ZipFile
from openpyxl import load_workbook
from lxml import etree as ET
warnings.filterwarnings('ignore',message='Unknown extension is not supported and will be removed')
p=Path(sys.argv[1])
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
fixture=Path(__file__).resolve().parents[2]/'test/fixtures/geometry.xlsx'
assert (p/'geometry-undone.xlsx').read_bytes() == fixture.read_bytes()
for name in ['geometry-edited.xlsx','geometry-reopened.xlsx']:
 w=load_workbook(p/name,rich_text=True);s=w['Records']
 assert s.row_dimensions[2].height==36 and s.row_dimensions[3].height==36
 assert s.row_dimensions[25].height==44
 assert s.row_dimensions[4].hidden and s.row_dimensions[5].hidden
 assert s.column_dimensions['B'].width==18 and s.column_dimensions['B'].hidden
 assert s.column_dimensions['C'].width==28.5 and not s.column_dimensions['C'].hidden
 assert s.column_dimensions['D'].width==18 and s.column_dimensions['D'].hidden
 for col in ['B','C','D']:
  d=s.column_dimensions[col]
  assert d.outlineLevel==1 and d.collapsed and d.font.bold and d.font.color.rgb=='00773355'
 assert str(s['A2'].value)=='Pear' and s['A2'].comment is not None
 assert s['B2'].font.bold and s['D2'].number_format=='$#,##0.00'
 assert s['A2'].hyperlink.location == "'Summary'!A1"
 with ZipFile(p/name) as z, ZipFile(fixture) as source:
  assert z.testzip() is None
  for path in z.namelist():
   if path.endswith(('.xml','.rels','.vml')): ET.fromstring(z.read(path))
  for path in ['xl/styles.xml','xl/theme/theme1.xml']:
   assert z.read(path)==source.read(path)
  xml=ET.fromstring(z.read('xl/worksheets/sheet1.xml'))
  tags=[ET.QName(n).localname for n in xml]
  assert tags.index('cols')<tags.index('sheetData')
  for n in xml.xpath('.//s:cols/s:col | .//s:sheetData/s:row',namespaces=ns): assert ET.QName(n).namespace==ns['s']
w=load_workbook(p/'geometry-structure.xlsx');s=w['Records']
assert s.row_dimensions[4].height==30 and s.row_dimensions[27].height==44
assert s.column_dimensions['D'].width==18 and s.column_dimensions['D'].hidden
w=load_workbook(p/'geometry-native.xlsx');s=w.worksheets[0]
assert s.row_dimensions[30].height==50 and s.column_dimensions['H'].width==22
w=load_workbook(p/'geometry-filtered.xlsx');s=w['Records']
assert not s.row_dimensions[2].hidden and not s.row_dimensions[4].hidden
assert s.row_dimensions[3].hidden and s.row_dimensions[6].hidden
assert s.auto_filter.ref=='A1:D7'
print('Independent geometry checks passed: column-group splitting, styles, row sizes, visibility, filters, structural movement, native export, Undo and reopen.')
