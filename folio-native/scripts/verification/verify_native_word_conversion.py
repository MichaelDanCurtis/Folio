"""Read exports and saved state produced by native conversion interaction."""
from pathlib import Path
from zipfile import ZipFile
import json
from docx import Document
from lxml import etree

work = Path(__file__).resolve().parents[4] / 'work'
ns = {'w': 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}
table_doc = Document(work / 'Native-conversion-table.docx')
table = table_doc.tables[0]
assert [[c.text for c in r.cells] for r in table.rows] == [
    ['Name', 'Amount'], ['Research', '2400']]
assert [c.width.pt for c in table.columns] == [120, 120]
assert table.cell(0, 0).paragraphs[0].runs[0].bold
assert table.cell(0, 1).paragraphs[0].runs[0].italic
for cell in table.rows[0].cells:
    assert cell.paragraphs[0].paragraph_format.keep_with_next
    assert cell.paragraphs[0].paragraph_format.space_after.pt == 9
for cell in table.rows[1].cells:
    assert cell.paragraphs[0].paragraph_format.left_indent.pt == 18
assert [p.text for p in table_doc.paragraphs] == ['Before conversion', 'After conversion']
text_doc = Document(work / 'Native-conversion-text.docx')
assert not text_doc.tables
assert [p.text for p in text_doc.paragraphs] == [
    'Before conversion', 'Name\tAmount', 'Research\t2400', 'After conversion']
assert text_doc.paragraphs[1].runs[0].bold
assert text_doc.paragraphs[1].runs[-1].italic
assert text_doc.paragraphs[2].runs[0].italic
assert text_doc.paragraphs[2].runs[-1].bold
assert text_doc.paragraphs[1].paragraph_format.keep_with_next
assert text_doc.paragraphs[2].paragraph_format.left_indent.pt == 18
for doc in [table_doc, text_doc]:
    assert doc.sections[0].header.paragraphs[0].text == 'Conversion header retained'
with ZipFile(work / 'Native-conversion-text.docx') as archive:
    xml = etree.fromstring(archive.read('word/document.xml'))
assert len(xml.findall('.//w:body/w:p//w:tab', ns)) == 2
before = (work / 'conversion-native-before-reopen.folio').read_bytes()
after = (work / 'conversion-native-after-reopen.folio').read_bytes()
assert before == after
saved = json.loads(after)
assert saved['revision'] == 7
assert all(n['type'] == 'paragraph' for n in saved['document']['content'])
print('Native table/text exports: rich runs, widths, paragraph properties and Word tabs verified; reopened revision 7 is byte-identical.')
