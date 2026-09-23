"""Read actual engine/widget XLSX outputs with independent openpyxl + lxml."""
from pathlib import Path
import tempfile, zipfile, copy
from openpyxl import load_workbook
from lxml import etree
root=Path(tempfile.gettempdir())/'folio-clipboard-tests'
def load(name): return load_workbook(root/name,rich_text=True)
a=load('overlap.xlsx');s=a['Source']
assert s['A1'].value==100 and s['A2'].value==2 and s['A3'].value==7
assert s['B2'].value=='=A2+$A2+A$1+$A$1+\'Other sheet\'!A2+LEN("A1")'
assert s['B2'].font.name=='Georgia' and s['B2'].font.bold and not s['B2'].font.italic
assert s['B2'].number_format=='0.0000' and s['B1'].number_format=='0.0%'
assert s['C2'].value=='One\nTwo\t"three"' and s['C3'].value is None
assert s.row_dimensions[3].hidden
assert load_workbook(root/'overlap.xlsx',data_only=True)['Source']['B2'].value==226
s=load('special.xlsx')['Source']
assert s['D8'].value=='destination' and s['D8'].font.bold and s['D8'].number_format=='0.0000'
assert s['G4'].value==7 and s['F6'].value=='One\nTwo\t"three"' and s['G6'].value is None
for a,v in [('J1',2),('K1',2),('J2',7),('K4',7)]: assert s[a].value==v
s=load('external.xlsx')['Source']
assert s['A10'].value=='one\ntwo' and s['B10'].value=='tab\tinside' and s['A11'].value=='a"b'
s=load('rich-shared.xlsx')['Input']
for a,b in [('C2','I11'),('D2','J11'),('C3','I12'),('S2','G20'),('T2','H20')]:
 assert s[a].value==s[b].value,(a,b,s[a].value,s[b].value)
 for field in ['font','fill','border','alignment','protection','number_format']:
  assert copy.copy(getattr(s[a],field))==copy.copy(getattr(s[b],field)),(a,b,field)
assert s['H11'].value=='=G11*$H$1'
# Source rich runs (including phonetic/extension children) survive exactly.
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with zipfile.ZipFile(root/'rich-shared.xlsx') as z:
 xml=etree.fromstring(z.read('xl/worksheets/sheet1.xml'))
 for a,b in [('C2','I11')]:
  left=xml.xpath(f'//s:c[@r="{a}"]/s:is/*',namespaces=ns)
  right=xml.xpath(f'//s:c[@r="{b}"]/s:is/*',namespaces=ns)
  def signature(n): return n.tag,dict(n.attrib),n.text,tuple(signature(c) for c in n)
  assert list(map(signature,left))==list(map(signature,right))
assert (root/'rich-shared.xlsx').read_bytes()==(root/'rich-reopened.xlsx').read_bytes()
s=load('inherited.xlsx')['Input']
assert copy.copy(s.column_dimensions['P'].font)==copy.copy(s['AA2'].font) and copy.copy(s.column_dimensions['P'].alignment)==copy.copy(s['AA2'].alignment)
ui=Path(tempfile.gettempdir())/'folio-clipboard-ui-tests'
if (ui/'clipboard.xlsx').exists():
 s=load_workbook(ui/'clipboard.xlsx')['Clipboard']
 for a,v in [('D4',2),('D5','=D4*3'),('E4','One\nTwo\ttab'),('E5','001'),('H4','keep'),('D10',99),('E10',297),('A16','quoted\nline'),('B16','tab\there'),('A17','a"b'),('D18','captured destination')]:
  assert s[a].value==v,(a,s[a].value,v)
 assert s['D5'].font.bold and s['H1'].font.italic and s['H4'].font.bold
 assert s.row_dimensions[4].height>18.75
 assert (ui/'clipboard.xlsx').read_bytes()==(ui/'reopened.xlsx').read_bytes()
print('Clipboard independent reader: formulas, full formats, rich runs, literals, quoted TSV, inherited styles, history outputs and saved/reopened XLSX passed.')
