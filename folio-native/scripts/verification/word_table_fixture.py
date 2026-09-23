"""A Word table with horizontal/vertical merges and independently authored properties."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from docx import Document
from docx.shared import Inches, Pt
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
out=Path(__file__).resolve().parents[2]/'test/fixtures'
doc=Document();doc.add_paragraph('Table verification')
t=doc.add_table(rows=3,cols=4);t.style='Table Grid'
for r,row in enumerate(t.rows):
 for c,cell in enumerate(row.cells):cell.text=f'R{r+1} C{c+1}'
t.cell(0,0).merge(t.cell(1,1));t.cell(0,2).merge(t.cell(0,3))
p=t.cell(0,0).paragraphs[0];p.runs[0].bold=True
p.paragraph_format.keep_with_next=True;p.paragraph_format.space_after=Pt(9)
p=t.cell(2,1).paragraphs[0];p.runs[0].italic=True;p.paragraph_format.left_indent=Inches(.15)
extra=t.cell(2,1).add_paragraph('Second paragraph');extra.runs[0].bold=True
cell=t.cell(2,3);nested=cell.add_table(rows=1,cols=2);nested.cell(0,0).text='Nested left';nested.cell(0,1).text='Nested right'
for cell in [t.cell(0,0),t.cell(1,2),t.cell(2,1)]:
 pr=cell._tc.get_or_add_tcPr();shd=OxmlElement('w:shd');shd.set(qn('w:fill'),'D9EAF7');pr.append(shd)
 va=OxmlElement('w:vAlign');va.set(qn('w:val'),'center');pr.append(va)
row=t.rows[0]._tr;pr=row.get_or_add_trPr();repeat=OxmlElement('w:tblHeader');pr.append(repeat)
widths=[1300,1900,2100,1700]
for c,w in zip(t._tbl.tblGrid.gridCol_lst,widths):c.set(qn('w:w'),str(w))
# Empty continuation paragraph carries a property that must survive edits elsewhere.
cont=t.rows[1]._tr.tc_lst[0];p=cont.p_lst[0];pr=p.get_or_add_pPr();keep=OxmlElement('w:keepNext');pr.append(keep)
doc.sections[0].header.paragraphs[0].text='Retained table header'
doc.add_paragraph('After the table')
doc.save(out/'word-tables.docx')
with ZipFile(out/'word-tables.docx') as z,ZipFile(out/'word-tables-prefixed.docx','w',ZIP_DEFLATED) as p:
 for name in z.namelist():
  data=z.read(name)
  if name=='word/document.xml':data=data.replace(b'xmlns:w=',b'xmlns:folioW=').replace(b'<w:',b'<folioW:').replace(b'</w:',b'</folioW:').replace(b' w:',b' folioW:')
  p.writestr(name,data)
print('Created merged Word table fixtures.')
