"""Independent python-docx/lxml validation of the merged-table exports."""
from pathlib import Path
from zipfile import ZipFile
import tempfile
from docx import Document
from lxml import etree
folder=Path(tempfile.gettempdir())/'folio-word-table-tests'
ns={'w':'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}
w='{'+ns['w']+'}'
def xml(name):
 with ZipFile(folder/name) as z:return etree.fromstring(z.read('word/document.xml'))
def signature(node):return (node.tag,sorted(node.attrib.items()),node.text or '',[signature(c) for c in node])
original=xml('original.docx')
for name in ['edited.docx','reopened.docx','legacy.docx','prefixed.docx','ui-edited.docx']:
 d=Document(folder/name);table=d.tables[0]
 assert len(table.columns)==4
 assert table.cell(0,0)._tc is table.cell(1,1)._tc
 assert table.cell(0,2)._tc is table.cell(0,3)._tc
 assert table.cell(0,0).paragraphs[0].runs[0].bold
 assert len(table.cell(2,3).tables)==1
 assert table.cell(2,3).tables[0].cell(0,1).text=='Nested right'
 root=xml(name)
 for prop in ['tblPr','trPr','shd','vAlign','keepNext','ind']:
  assert [signature(n) for n in root.findall('.//w:'+prop,ns)]==[signature(n) for n in original.findall('.//w:'+prop,ns)],(name,prop)
 assert d.sections[0].header.paragraphs[0].text=='Retained table header'
 if name=='ui-edited.docx':
  assert len(table.rows)==4
  assert table.cell(3,0)._tc is table.cell(3,1)._tc
  assert table.cell(2,1).paragraphs[0].text.startswith('Updated cell')
  assert table.cell(2,1).paragraphs[0].runs[0].italic
  assert table.cell(2,1).paragraphs[1].runs[0].bold
  cells=root.find('w:body/w:tbl',ns).findall('w:tr',ns)[3].findall('w:tc',ns)
  assert cells[0].find('w:tcPr/w:tcW',ns).get(w+'w')=='3200'
 else:assert len(table.rows)==3
 # Every continuation has the same span as its restart and legal child ordering.
 for tc in root.findall('.//w:tc',ns):
  assert tc[-1].tag==w+'p'
  props=tc.find('w:tcPr',ns)
  if props is not None:
   names=[etree.QName(n).localname for n in props]
   if 'gridSpan' in names and 'vMerge' in names:assert names.index('gridSpan')<names.index('vMerge')
 print(name,': merged grid, rich text, nested table and source properties verified')
native=Document(folder/'native.docx').tables[0]
assert len(native.rows)==2 and len(native.columns)==2
assert native.cell(0,0)._tc is native.cell(1,0)._tc
assert native.cell(0,0).paragraphs[0].runs[0].bold
assert native.cell(0,0).paragraphs[1].runs[0].italic
assert native.cell(1,1).text=='Bottom right'
assert (folder/'undone.docx').read_bytes()==(folder/'original.docx').read_bytes()
print('Native vertical merge and exact original bytes after Undo verified.')
