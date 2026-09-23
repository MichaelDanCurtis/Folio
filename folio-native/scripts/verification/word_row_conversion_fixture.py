"""Independent Word fixture with middle rows, rich content and retained merges."""
from pathlib import Path
from docx import Document
from docx.shared import Pt
from docx.enum.table import WD_TABLE_ALIGNMENT, WD_ROW_HEIGHT_RULE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn

doc = Document()
doc.add_paragraph('Before rows')
table = doc.add_table(rows=5, cols=3)
table.autofit = False
table.alignment = WD_TABLE_ALIGNMENT.RIGHT
table.style = 'Table Grid'
for col, width in zip(table.columns, [72, 108, 144]):
    col.width = Pt(width)
for r, row in enumerate(table.rows):
    for c, cell in enumerate(row.cells):
        cell.width = Pt([72, 108, 144][c])
        paragraph = cell.paragraphs[0]
        paragraph.paragraph_format.space_after = Pt(7)
        paragraph.paragraph_format.left_indent = Pt(9)
        run = paragraph.add_run(f'R{r + 1}C{c + 1}')
        run.italic = c == 0
        run.bold = c == 1
        shading = OxmlElement('w:shd')
        shading.set(qn('w:fill'), 'FFF2CC' if c == 0 else 'E8F0F8')
        cell._tc.get_or_add_tcPr().append(shading)
header = OxmlElement('w:tblHeader')
table.rows[0]._tr.get_or_add_trPr().append(header)
table.cell(1, 0).paragraphs[0].paragraph_format.keep_with_next = True
table.cell(1, 1).add_paragraph().add_run('Second paragraph').bold = True
nested = table.cell(1, 2).add_table(rows=1, cols=2)
nested.cell(0, 0).text = 'Nested A'
nested.cell(0, 1).text = 'Nested B'
table.cell(3, 0).merge(table.cell(4, 0))
table.cell(3, 1).merge(table.cell(3, 2))
table.rows[3].height = Pt(24)
table.rows[3].height_rule = WD_ROW_HEIGHT_RULE.EXACTLY
doc.add_paragraph('After rows')
doc.sections[0].header.paragraphs[0].text = 'Row conversion header'
path = Path(__file__).resolve().parents[2] / 'test/fixtures/word-row-conversion.docx'
doc.save(path)
print(path)
