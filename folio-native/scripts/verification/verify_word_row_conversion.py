"""Independent reader for selected-row conversion; native Word comparison is separate."""
from pathlib import Path
import tempfile
from zipfile import ZipFile
from docx import Document
from docx.enum.table import WD_TABLE_ALIGNMENT, WD_ROW_HEIGHT_RULE
from lxml import etree

folder = Path(tempfile.gettempdir()) / 'folio-word-row-conversion-tests'
source = Path(__file__).resolve().parents[2] / 'test/fixtures/word-row-conversion.docx'
assert (folder / 'undone.docx').read_bytes() == source.read_bytes()
assert (folder / 'reimported.docx').read_bytes() == (folder / 'rows.docx').read_bytes()
ns = {'w': 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}
w = '{' + ns['w'] + '}'
expected = [
    'Before rows', 'Revised R2C1\tR2C2', 'Second paragraph\tR2C3',
    'Nested A\tNested B', '', 'R3C1\tR3C2\tR3C3', 'After rows',
]
documents = []
for name in ['rows', 'reopened', 'reimported']:
    path = folder / f'{name}.docx'
    doc = Document(path)
    assert [p.text for p in doc.paragraphs] == expected
    assert len(doc.tables) == 2
    first, last = doc.tables
    assert [[c.text for c in r.cells] for r in first.rows] == [['R1C1', 'R1C2', 'R1C3']]
    assert len(last.rows) == 2 and len(last.columns) == 3
    assert last.cell(0, 0).text == 'R4C1\nR5C1'
    assert last.cell(0, 0)._tc is last.cell(1, 0)._tc
    assert last.cell(0, 1).text == 'R4C2\nR4C3'
    assert last.cell(0, 1)._tc is last.cell(0, 2)._tc
    assert last.cell(1, 1).text == 'R5C2' and last.cell(1, 2).text == 'R5C3'
    for table in doc.tables:
        assert [col.width.pt for col in table.columns] == [72, 108, 144]
        assert table.autofit is False and table.alignment == WD_TABLE_ALIGNMENT.RIGHT
        assert table.style.style_id == 'TableGrid'
    assert last.rows[0].height.pt == 24
    assert last.rows[0].height_rule == WD_ROW_HEIGHT_RULE.EXACTLY
    assert doc.paragraphs[1].runs[0].italic
    assert doc.paragraphs[1].runs[-1].bold
    assert doc.paragraphs[2].runs[0].bold
    assert doc.paragraphs[1].paragraph_format.keep_with_next
    assert doc.paragraphs[1].paragraph_format.space_after.pt == 7
    assert doc.paragraphs[1].paragraph_format.left_indent.pt == 9
    assert doc.sections[0].header.paragraphs[0].text == 'Row conversion header'
    with ZipFile(path) as archive, ZipFile(source) as original:
        assert archive.read('word/header1.xml') == original.read('word/header1.xml')
        root = etree.fromstring(archive.read('word/document.xml'))
    tables = root.findall('w:body/w:tbl', ns)
    assert len(tables) == 2
    assert tables[0].find('w:tr/w:trPr/w:tblHeader', ns) is not None
    assert tables[1].find('w:tr/w:tc/w:tcPr/w:shd', ns).get(w + 'fill') == 'FFF2CC'
    assert len(root.findall('w:body/w:p//w:tab', ns)) == 5
    assert not any('\t' in (t.text or '') for t in root.findall('.//w:t', ns))
    documents.append(etree.tostring(root, method='c14n'))
assert documents[0] == documents[1] == documents[2]
print('Verified selected-row text, two table fragments, source formatting, merged cells, nested content, real Word tabs, exact original Undo, reopened exports and byte-identical Office reimport/export.')
