"""Verify retained artifacts from the native macOS paragraph UI sessions."""
import argparse,json
from pathlib import Path
from docx import Document

parser=argparse.ArgumentParser()
parser.add_argument('folder',type=Path)
args=parser.parse_args();folder=args.folder
def read(name): return json.loads((folder/name).read_text())
assert read('native-undone.folio')['document']==read('seed.folio')['document']
assert read('native-redone.folio')['document']==read('native-applied.folio')['document']
assert read('final-table-undone.folio')['document']==read('before-final-build.folio')['document']
assert read('final-table-redone.folio')['document']==read('final-table-applied.folio')['document']
for name,revision in [('seed.folio',0),('native-applied.folio',1),('native-undone.folio',2),
                      ('native-redone.folio',3),('native-table-applied.folio',4),
                      ('final-table-applied.folio',5),('final-table-undone.folio',6),('final-table-redone.folio',7)]:
    assert read(name)['revision']==revision
for name,table_before in [('native-paragraphs.docx',18),('native-table-paragraphs.docx',30)]:
    doc=Document(folder/name);p=doc.paragraphs[0];f=p.paragraph_format
    assert p.text=='First\tparagraph' and p.runs[0].bold and p.runs[-1].italic
    assert p.style.style_id=='ParagraphDerived'
    assert f.left_indent.pt==72 and f.space_after.pt==18 and f.line_spacing==2.0
    assert f.keep_with_next and f.tab_stops[0].position.pt==72
    assert doc.paragraphs[2].paragraph_format.first_line_indent.pt==-12
    assert doc.tables[0].cell(0,0).paragraphs[0].paragraph_format.space_before.pt==table_before
assert (folder/'before-final-restart.folio').read_bytes()==(folder/'after-final-restart.folio').read_bytes()
print('Verified native Paragraph Apply/Undo/Redo/Cancel, final table keyboard history, two independent DOCX exports and byte-identical final-build restart at revision 7.')
