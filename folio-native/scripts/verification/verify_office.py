from pathlib import Path
from docx import Document
from openpyxl import load_workbook
from zipfile import ZipFile
from lxml import etree
import tempfile
base=Path(tempfile.gettempdir())
for name in ['folio-verified-edited.docx','folio-verified-new.docx','folio-verified-linked.docx']:
 p=base/name;d=Document(p);assert len(d.paragraphs)>0
 with ZipFile(p) as z:
  for part in z.namelist():
   if part.endswith(('.xml','.rels')):etree.fromstring(z.read(part))
 print(name, 'reopened independently:',len(d.paragraphs),'paragraphs,',len(d.tables),'tables')
d=Document(base/'folio-verified-edited.docx');assert d.paragraphs[1].text.startswith('Net revenue');assert d.sections[0].header.paragraphs[0].text=='Keep this header';assert len(d.inline_shapes)==1;assert d.paragraphs[1].runs[1].bold
for name in ['folio-verified-edited.xlsx','folio-verified-new.xlsx','folio-verified-styled.xlsx']:
 w=load_workbook(base/name);print(name,'reopened independently:',w.sheetnames)
 if 'Budget' in w:
  s=w['Budget'];assert s['B4'].value=='=SUM(B2:B3)';assert s['A2'].comment.text=='Keep this comment';assert len(s._charts)==1;assert s.freeze_panes=='B2';assert 'E1:F1' in str(s.merged_cells)
  if 'styled' in name:assert s['B2'].font.italic and not s['B2'].font.bold;assert s['B2'].number_format=='0.00%'
  else:assert s['B2'].value==3000;assert s['B2'].number_format=='"$"#,##0.00';assert s['B2'].font.bold
print('All independent package, object, formula, style and content assertions passed.')
