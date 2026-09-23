"""Independent DOCX reader checks; not a claim of native Microsoft Word parity."""
from pathlib import Path
from zipfile import ZipFile
import tempfile
from docx import Document
from lxml import etree

folder = Path(tempfile.gettempdir()) / 'folio-word-conversion-tests'
ns = {'w': 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}
w = '{' + ns['w'] + '}'
original = Path(__file__).resolve().parents[2] / 'test/fixtures/word-conversion.docx'
assert (folder / 'undone.docx').read_bytes() == original.read_bytes()
d = Document(folder / 'table.docx')
t = d.tables[0]
assert len(t.rows) == len(t.columns) == 2
assert [[c.text for c in r.cells] for r in t.rows] == [['Name', 'Amount'], ['Research', '2400']]
assert t.cell(0, 0).paragraphs[0].runs[0].bold
assert t.cell(0, 1).paragraphs[0].runs[0].italic
for cell in t.rows[0].cells:
    assert cell.paragraphs[0].paragraph_format.keep_with_next
    assert cell.paragraphs[0].paragraph_format.space_after.pt == 9
for cell in t.rows[1].cells:
    assert cell.paragraphs[0].paragraph_format.left_indent.pt == 18
assert [c.width.pt for c in t.columns] == [120, 120]
assert [p.text for p in d.paragraphs] == ['Before conversion', 'After conversion']
for name in ['text', 'reopened']:
    d = Document(folder / f'{name}.docx')
    assert not d.tables
    assert [p.text for p in d.paragraphs] == ['Before conversion', 'Name\tAmount', 'Research\t2400', 'After conversion']
    assert d.paragraphs[1].runs[0].bold
    assert d.paragraphs[1].runs[-1].italic
    assert d.paragraphs[2].runs[0].italic
    assert d.paragraphs[2].runs[-1].bold
    assert d.paragraphs[1].paragraph_format.keep_with_next
    assert d.paragraphs[2].paragraph_format.left_indent.pt == 18
    assert d.sections[0].header.paragraphs[0].text == 'Conversion header retained'
    with ZipFile(folder / f'{name}.docx') as z:
        root = etree.fromstring(z.read('word/document.xml'))
    assert len(root.findall('.//w:body/w:p//w:tab', ns)) == 2
    assert not any('\t' in (t.text or '') for t in root.findall('.//w:t', ns))
print('Verified text/table conversion, rich runs, paragraph properties, real Word tabs, header retention, Undo bytes and reopened exports.')
