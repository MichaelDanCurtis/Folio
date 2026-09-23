"""Independent fixture and reader for lazy heading creation and style origins."""
from pathlib import Path
from tempfile import gettempdir
import argparse,json
from docx import Document
from docx.enum.style import WD_STYLE_TYPE
from docx.shared import Pt
from docx.oxml.ns import qn
ROOT=Path(__file__).resolve().parents[2]
SOURCE=ROOT/'test/fixtures/word-missing-headings.docx'
def fixture():
    d=Document()
    for level in range(1,10):
        style=d.styles[f'Heading {level}'];style.element.getparent().remove(style.element)
    normal=d.styles['Normal'];normal.font.name='Arial';normal.font.size=Pt(11)
    body=d.styles.add_style('Project body',WD_STYLE_TYPE.PARAGRAPH)
    body.element.set(qn('w:styleId'),'ProjectBody');body.base_style=normal;body.font.size=Pt(14)
    p=d.add_paragraph(style=body);p.add_run('Heading candidate').bold=False
    d.add_paragraph('Neighbor stays unchanged.')
    p=d.add_table(rows=1,cols=1).cell(0,0).paragraphs[0];p.style=body;p.add_run('Cell candidate')
    d.save(SOURCE);return SOURCE
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--fixture',action='store_true');parser.add_argument('folder',nargs='?',type=Path);args=parser.parse_args()
    if args.fixture: print(fixture())
    else:
        folder=args.folder or Path(gettempdir())/'folio-builtin-style-ui'
        for name in ['headings.docx','reopened.docx']:
            d=Document(folder/name)
            assert d.paragraphs[0].text=='Heading candidate'
            assert d.paragraphs[0].style.style_id=='Heading9'
            assert d.paragraphs[1].text=='Neighbor stays unchanged.'
            assert d.tables[0].cell(0,0).text=='Cell candidate'
            for level in range(1,10):
                # Resolve by OOXML identity: python-docx rewrites display-name
                # lookups to lowercase built-in aliases, while both name
                # spellings are legal and exposed correctly by iteration.
                s=next(s for s in d.styles if s.style_id==f'Heading{level}');assert s.builtin
                assert s.name.lower()==f'heading {level}'
                assert s.base_style.style_id=='Normal'
                assert s.next_paragraph_style.style_id=='Normal'
                assert s.element.pPr.find(qn('w:outlineLvl')).get(qn('w:val'))==str(level-1)
            assert not d.styles['Project body'].builtin
            assert d.paragraphs[0].runs[0].bold is False
        cell=Document(folder/'cell-heading.docx')
        assert cell.tables[0].cell(0,0).paragraphs[0].style.style_id=='Heading7'
        assert next(s for s in cell.styles if s.style_id=='Heading7').builtin
        report={'headings1Through9':True,'followingNormal':True,'directOverrideRetained':True,'tableDraftApplied':True,'customStyleRetained':True,'reopenedExport':True,'wordExecution':False}
        (folder/'independent-reader.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
