"""Reproducible Word tab fixture built with python-docx, plus relocated OPC parts."""
from pathlib import Path
from docx import Document
from docx.enum.style import WD_STYLE_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Pt
from zipfile import ZipFile, ZIP_DEFLATED

root = Path(__file__).resolve().parents[2]
def tabs(pr, stops):
    node = OxmlElement('w:tabs')
    for pos, alignment, leader in stops:
        stop = OxmlElement('w:tab')
        for key, value in [('pos', pos), ('val', alignment), ('leader', leader)]:
            stop.set(qn('w:' + key), str(value))
        node.append(stop)
    pr.append(node)

doc = Document()
settings = doc.settings.element
settings.find(qn('w:defaultTabStop')).set(qn('w:val'), '600')
pr = doc.styles.element.find(qn('w:docDefaults')).find(qn('w:pPrDefault')).find(qn('w:pPr'))
tabs(pr, [(360, 'left', 'none')])
base = doc.styles.add_style('TabBase', WD_STYLE_TYPE.PARAGRAPH)
base.base_style = doc.styles['Normal']
tabs(base.element.get_or_add_pPr(), [(720, 'center', 'none')])
derived = doc.styles.add_style('TabDerived', WD_STYLE_TYPE.PARAGRAPH)
derived.base_style = base
tabs(derived.element.get_or_add_pPr(), [(360, 'clear', 'none'), (1440, 'right', 'none')])
p = doc.add_paragraph(style=derived)
p.paragraph_format.space_after = Pt(7)
p.paragraph_format.keep_with_next = True
p.add_run('Project\t12.50\tReady').bold = True
tabs(p._p.get_or_add_pPr(), [(720, 'clear', 'none'), (2160, 'decimal', 'dot')])
doc.add_paragraph('Inherited\tCenter\tRight', style=derived)
doc.add_paragraph('Default\tSpacing')
t = doc.add_table(rows=1, cols=1)
t.cell(0, 0).paragraphs[0].style = derived
t.cell(0, 0).paragraphs[0].text = 'Nested\t12.50'
doc.sections[0].header.paragraphs[0].text = 'Tab compatibility fixture'
path = root / 'test/fixtures/word-tabs.docx'
doc.save(path)
with ZipFile(path) as z:
    entries = {n: z.read(n) for n in z.namelist()}
for kind in ['styles', 'settings']:
    old, new = f'word/{kind}.xml', f'word/options/{kind}-custom.xml'
    entries[new] = entries.pop(old)
    entries['word/_rels/document.xml.rels'] = entries['word/_rels/document.xml.rels'].replace(f'Target="{kind}.xml"'.encode(), f'Target="options/../options/{kind}-custom.xml"'.encode())
    entries['[Content_Types].xml'] = entries['[Content_Types].xml'].replace(f'/{old}'.encode(), f'/{new}'.encode())
for name, data in entries.items():
    if name.endswith('.xml'):
        entries[name] = data.replace(b'xmlns:w=', b'xmlns:wx=').replace(b'<w:', b'<wx:').replace(b'</w:', b'</wx:').replace(b' w:', b' wx:')
with ZipFile(root / 'test/fixtures/word-tabs-relocated.docx', 'w', ZIP_DEFLATED) as z:
    for name, data in entries.items(): z.writestr(name, data)
print(path)
