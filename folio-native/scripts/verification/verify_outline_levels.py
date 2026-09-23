"""Independent DOCX outline fixture and exported structure checks."""
from pathlib import Path
from tempfile import gettempdir
import argparse, json
from zipfile import ZipFile, ZIP_DEFLATED
from lxml import etree
from docx import Document
from docx.enum.style import WD_STYLE_TYPE
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Pt
ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT/'test/fixtures/word-outline-levels.docx'
def outline(p, level):
    pr=p._p.get_or_add_pPr(); n=OxmlElement('w:outlineLvl');n.set(qn('w:val'),str(level));pr.append(n)
def fixture():
    d=Document();normal=d.styles['Normal'];normal.font.name='Arial';normal.font.size=Pt(11)
    body=d.styles.add_style('Outline body',WD_STYLE_TYPE.PARAGRAPH);body.base_style=normal;body.font.size=Pt(14)
    structural=d.styles.add_style('Structure only',WD_STYLE_TYPE.PARAGRAPH);structural.base_style=body
    n=OxmlElement('w:outlineLvl');n.set(qn('w:val'),'1');structural.element.get_or_add_pPr().append(n)
    p=d.add_paragraph('Direct level one 😀',style=body);outline(p,0);p.runs[0].bold=False
    p=d.add_paragraph('Heading appearance, body structure',style='Heading 1');outline(p,9)
    p=d.add_paragraph('Direct level nine',style=structural);outline(p,8)
    d.add_paragraph('Inherited level two',style=structural)
    p=d.add_paragraph('Numbered structure',style='List Number');outline(p,3)
    p=d.add_table(rows=1,cols=1).cell(0,0).paragraphs[0];p.style=body;p.add_run('Cell structure');outline(p,7)
    d.save(SOURCE)
    with ZipFile(SOURCE) as z: parts={n:z.read(n) for n in z.namelist()}
    xml=etree.fromstring(parts['word/document.xml'])
    first=xml.find('.//'+qn('w:outlineLvl'))
    first.set('{urn:folio:outline}val','foreign-value')
    first.set('{urn:folio:outline}keep','retained')
    parts['word/document.xml']=etree.tostring(xml,xml_declaration=True,encoding='UTF-8').replace(b'xmlns:w=',b'xmlns:wx=').replace(b'<w:',b'<wx:').replace(b'</w:',b'</wx:').replace(b' w:',b' wx:')
    with ZipFile(SOURCE,'w',ZIP_DEFLATED) as z:
        for name,data in parts.items():z.writestr(name,data)
    return SOURCE
def check(folder):
    for level in range(10):
        d=Document(folder/f'level-{level}.docx');p=d.paragraphs[0]
        assert p.text=='Direct level one 😀' and p.style.name=='Outline body'
        assert p._p.pPr.find(qn('w:outlineLvl')).get(qn('w:val'))==str(level)
        assert p._p.pPr.find(qn('w:outlineLvl')).get('{urn:folio:outline}val')=='foreign-value'
        assert p._p.pPr.find(qn('w:outlineLvl')).get('{urn:folio:outline}keep')=='retained'
        assert p.runs[0].bold is False and p.style.font.size.pt==14
        assert d.paragraphs[1].style.style_id=='Heading1'
        assert d.paragraphs[1]._p.pPr.find(qn('w:outlineLvl')).get(qn('w:val'))=='9'
        assert d.tables[0].cell(0,0).paragraphs[0]._p.pPr.find(qn('w:outlineLvl')).get(qn('w:val'))=='7'
    for name in ['cleared.docx','cleared-reopened.docx']:
        d=Document(folder/name);assert d.paragraphs[0]._p.pPr.find(qn('w:outlineLvl')) is None
        assert d.paragraphs[0].style.name=='Outline body'
    report={'tenOutlineValues':True,'styleAndDirectFormattingRetained':True,
            'explicitBodyOverride':True,'tableOutlineRetained':True,
            'clearAndReopen':True,'microsoftWordExecution':False}
    (folder/'independent-reader.json').write_text(json.dumps(report,indent=2)+'\n');return report
def check_ui(folder):
    d=Document(folder/'body.docx')
    assert [p.style.style_id for p in d.paragraphs[:2]]==['Outlinebody','Heading1']
    assert [p._p.pPr.find(qn('w:outlineLvl')).get(qn('w:val')) for p in d.paragraphs[:2]]==['2','2']
    for name in ['cell.docx','reopened.docx']:
        d=Document(folder/name)
        assert d.paragraphs[0].text=='Direct level one 😀'
        assert d.paragraphs[0]._p.pPr.find(qn('w:outlineLvl')) is None
        assert d.paragraphs[0].style.style_id=='Outlinebody'
        p=d.tables[0].cell(0,0).paragraphs[0]
        assert p.text=='Cell structure' and p.style.style_id=='Outlinebody'
        assert p._p.pPr.find(qn('w:outlineLvl')).get(qn('w:val'))=='4'
    report={'mixedParagraphApply':True,'visualStylesUnchanged':True,'clearRetained':True,'tableApplied':True,'reopenedExport':True,'microsoftWordExecution':False}
    (folder/'independent-reader.json').write_text(json.dumps(report,indent=2)+'\n');return report
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--fixture',action='store_true');p.add_argument('--ui',action='store_true');p.add_argument('folder',nargs='?',type=Path);a=p.parse_args()
    print(fixture() if a.fixture else json.dumps((check_ui if a.ui else check)(a.folder or Path(gettempdir())/('folio-outline-ui' if a.ui else 'folio-outline-tests')),indent=2))
