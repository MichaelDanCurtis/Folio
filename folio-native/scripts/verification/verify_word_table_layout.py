"""Independent checks for editable Word table dimensions and cell appearance."""
from pathlib import Path
from zipfile import ZipFile
import tempfile
import sys
from docx import Document
from docx.enum.table import WD_TABLE_ALIGNMENT, WD_ROW_HEIGHT_RULE, WD_CELL_VERTICAL_ALIGNMENT
from lxml import etree
folder=Path(tempfile.gettempdir())/'folio-word-table-tests'
ns={'w':'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}
w='{'+ns['w']+'}'
names = [sys.argv[1]] if len(sys.argv)>1 else ['layout-word-tables.docx','layout-word-tables-prefixed.docx','layout-word-tables-reopened.docx','layout-word-tables-prefixed-reopened.docx','layout-ui.docx']
for name in names:
 is_ui = len(sys.argv)>1 or name=='layout-ui.docx'
 d=Document(folder/name);t=d.tables[0]
 assert not t.autofit and t.alignment==WD_TABLE_ALIGNMENT.RIGHT
 assert [c.width.twips for c in t.columns]==[1800,1800,2100,1700]
 assert t.rows[0].height.twips==480 and t.rows[0].height_rule==WD_ROW_HEIGHT_RULE.EXACTLY
 assert t.rows[1].height.twips==(480 if is_ui else 900)
 assert t.rows[1].height_rule==(WD_ROW_HEIGHT_RULE.EXACTLY if is_ui else WD_ROW_HEIGHT_RULE.AT_LEAST)
 assert t.cell(0,0)._tc is t.cell(1,1)._tc
 assert t.cell(0,0).vertical_alignment==WD_CELL_VERTICAL_ALIGNMENT.BOTTOM
 assert len(t.cell(0,0).paragraphs)==4
 assert t.cell(0,0).paragraphs[0].runs[0].bold
 assert t.cell(2,3).tables[0].cell(0,1).text=='Nested right'
 assert d.sections[0].header.paragraphs[0].text=='Retained table header'
 with ZipFile(folder/name) as z:root=etree.fromstring(z.read('word/document.xml'))
 table=root.find('w:body/w:tbl',ns)
 assert table.find('w:tblPr/w:tblW',ns).get(w+'w')=='7400'
 for row in table.findall('w:tr',ns)[:2]:
  pr=row.find('w:tc/w:tcPr',ns)
  assert pr.find('w:shd',ns).get(w+'fill')=='FFF2CC'
  assert pr.find('w:vAlign',ns).get(w+'val')=='bottom'
  assert pr.find('w:tcW',ns).get(w+'w')=='3600'
 assert table.find('w:tr/w:trPr/w:tblHeader',ns) is not None
 print(name,': fixed dimensions, row rules, fills, vertical alignment, merges and retained content verified')
for suffix in ([] if len(sys.argv)>1 else ['word-tables','word-tables-prefixed']):
 reset=Document(folder/f'layout-{suffix}-reset.docx').tables[0]
 assert reset.rows[0].height_rule==WD_ROW_HEIGHT_RULE.AUTO
 assert (folder/f'layout-{suffix}-undone.docx').read_bytes()==(folder/f'layout-{suffix}-original.docx').read_bytes()
if len(sys.argv)==1: print('Automatic height reset and byte-exact Undo verified.')
