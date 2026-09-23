"""Check the disposable packaged-app table export with an independent reader.

Usage: python verify_native_word_tables.py /path/to/Native-tables.docx
The native workflow edits R3C2, inserts row 4, and merges its first two cells.
"""
import sys
from pathlib import Path
from zipfile import ZipFile

from docx import Document
from lxml import etree

path = Path(sys.argv[1])
doc = Document(path)
table = doc.tables[0]
assert len(table.rows) == 4 and len(table.columns) == 4
assert table.cell(0, 0)._tc is table.cell(1, 1)._tc
assert table.cell(0, 2)._tc is table.cell(0, 3)._tc
assert table.cell(3, 0)._tc is table.cell(3, 1)._tc
cell = table.cell(2, 1)
assert cell.paragraphs[0].text == 'R3 C2 checked'
assert cell.paragraphs[0].runs[0].italic
assert cell.paragraphs[1].text == 'Second paragraph'
assert cell.paragraphs[1].runs[0].bold
assert table.cell(2, 3).tables[0].cell(0, 1).text == 'Nested right'
assert [p.text for p in doc.paragraphs] == ['Table verification', 'After the table']
assert doc.sections[0].header.paragraphs[0].text == 'Retained table header'
namespace = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
ns = {'w': namespace}
with ZipFile(path) as package:
    root = etree.fromstring(package.read('word/document.xml'))
grid = root.find('w:body/w:tbl/w:tblGrid', ns)
assert [int(c.get('{'+namespace+'}w')) for c in grid] == [1300, 1900, 2100, 1700]
assert root.find('.//w:tbl/w:tr/w:trPr/w:tblHeader', ns) is not None
assert root.find('.//w:shd', ns) is not None
assert root.find('.//w:vAlign', ns) is not None
print('Packaged export: rich cell edit, three merges, new row, nested table, widths, header and body verified.')
