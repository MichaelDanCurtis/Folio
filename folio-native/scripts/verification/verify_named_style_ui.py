"""Independent input and output checks for the named-style editor."""
from pathlib import Path
from tempfile import gettempdir
import argparse, json
from docx import Document
from docx.shared import Pt, RGBColor
from docx.enum.style import WD_STYLE_TYPE
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
ROOT=Path(__file__).resolve().parents[2]
SOURCE=ROOT/'test/fixtures/word-named-styles.docx'
def fixture():
    d=Document()
    normal=d.styles['Normal'];normal.font.name='Arial';normal.font.size=Pt(12)
    normal.paragraph_format.space_after=Pt(9);normal.paragraph_format.line_spacing=1.4
    base=d.styles.add_style('Report base',WD_STYLE_TYPE.PARAGRAPH);base.element.set(qn('w:styleId'),'ReportBase');base.base_style=normal
    base.font.name='Georgia';base.font.size=Pt(14);base.font.bold=True;base.font.color.rgb=RGBColor.from_string('185ABD');base.paragraph_format.alignment=WD_ALIGN_PARAGRAPH.LEFT
    body=d.styles.add_style('Report body',WD_STYLE_TYPE.PARAGRAPH);body.element.set(qn('w:styleId'),'ReportBody');body.base_style=base;body.next_paragraph_style=normal
    heading=d.styles.add_style('Report heading',WD_STYLE_TYPE.PARAGRAPH);heading.element.set(qn('w:styleId'),'ReportHeading');heading.base_style=base;heading.next_paragraph_style=body
    heading.font.size=Pt(18);heading.font.bold=True
    outline=OxmlElement('w:outlineLvl');outline.set(qn('w:val'),'1');heading.element.get_or_add_pPr().append(outline)
    d.add_paragraph('Executive summary',style=heading)
    p=d.add_paragraph(style=body);p.add_run('Revenue summary: ');p.add_run('stable').bold=False;p.add_run(' and clear.')
    d.add_paragraph('Second paragraph follows the report style.',style=body)
    cell=d.add_table(rows=1,cols=1).cell(0,0).paragraphs[0];cell.style=body;cell.add_run('Table report text')
    d.add_paragraph('Unrelated paragraph.',style=normal)
    d.save(SOURCE)
    return SOURCE
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--fixture',action='store_true');parser.add_argument('folder',nargs='?',type=Path);args=parser.parse_args()
    if args.fixture: print(fixture())
    else:
        folder=args.folder or Path(gettempdir())/'folio-named-style-ui'
        d=Document(folder/'styled.docx')
        base=d.styles['Report base renamed'];assert base.style_id=='ReportBase';assert base.font.size.pt==18
        assert d.styles['Report body'].base_style.style_id=='ReportBase'
        p=next(p for p in d.paragraphs if p.text.startswith('Revenue summary:'))
        assert p.style.style_id=='ReportBody';assert any(r.text=='stable' and r.bold is False for r in p.runs)
        p=next(p for p in d.paragraphs if p.text=='New section detail')
        assert p.style.style_id=='ReportBody'
        assert d.tables[0].cell(0,0).paragraphs[0].style.style_id=='ReportHeading'
        assert d.tables[0].cell(0,0).text=='Table report text added'
        assert next(p for p in d.paragraphs if p.text=='Unrelated paragraph.').style.style_id=='Normal'
        assert Document(folder/'reopened.docx').paragraphs[0].text=='Executive summary'
        listed=Document(folder/'list-styled.docx')
        p=next(p for p in listed.paragraphs if p.text.startswith('Revenue summary:'))
        assert p.style.style_id=='ReportHeading'
        assert p._p.pPr.numPr is not None
        report={'files':['styled.docx','reopened.docx','list-styled.docx'],'styleId':'ReportBase','bodyOverridesPreserved':True,'followingStyle':'ReportBody','cellStyle':'ReportHeading','numberedHeadingPreserved':True,'wordExecution':False}
        (folder/'independent-reader.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
