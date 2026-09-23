"""Independently authored rich text for text/table conversion round trips."""
from pathlib import Path
from docx import Document
from docx.shared import Pt
from docx.enum.text import WD_ALIGN_PARAGRAPH

doc = Document()
doc.add_paragraph('Before conversion')
p = doc.add_paragraph()
p.paragraph_format.keep_with_next = True
p.paragraph_format.space_after = Pt(9)
p.alignment = WD_ALIGN_PARAGRAPH.RIGHT
p.add_run('Name').bold = True
p.add_run('\tAmount').italic = True
p = doc.add_paragraph()
p.paragraph_format.left_indent = Pt(18)
p.add_run('Research').italic = True
p.add_run('\t2400').bold = True
doc.add_paragraph('After conversion')
doc.sections[0].header.paragraphs[0].text = 'Conversion header retained'
path = Path(__file__).resolve().parents[2] / 'test/fixtures/word-conversion.docx'
doc.save(path)
print(path)
