"""Read style replacement exports independently with python-docx and lxml."""
import json
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
from docx import Document
from lxml import etree
NS = {'w': 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}
base = Path(gettempdir())
engine = base / 'folio-style-tests'
ui = base / 'folio-style-replacement-ui'
with ZipFile(engine / 'replace-style-after.docx') as archive:
    styles = etree.fromstring(archive.read('word/styles/named.xml'))
    assert not styles.xpath('./w:style[@w:styleId="StyleBase"]', namespaces=NS)
    assert styles.xpath('./w:style[@w:styleId="StyleChild"]/w:basedOn/@w:val', namespaces=NS) == ['Normal']
    header = etree.fromstring(archive.read('word/header-style-test.xml'))
    assert header.xpath('//w:pStyle/@w:val', namespaces=NS) == ['Normal']
    assert header.xpath('//w:pStyle/@k:val', namespaces={'k': 'urn:keep', **NS}) == ['StyleBase']
    assert header.xpath('//w:t/text()', namespaces=NS) == ['StyleBase header']
    assert header.xpath('//w:color/@w:val', namespaces=NS) == ['ABCDEF']
    assert archive.read('word/document.xml').find(b'Changed before style deletion') >= 0
for name in ['replaced.docx', 'reopened.docx']:
    d = Document(ui / name)
    assert 'ReportBody' not in [s.style_id for s in d.styles]
    assert d.styles['Report heading'].next_paragraph_style.style_id == 'Normal'
    for prefix in ['Revenue summary:', 'Second paragraph']:
        p = next(p for p in d.paragraphs if p.text.startswith(prefix))
        assert p.style.style_id == 'Normal'
    p = next(p for p in d.paragraphs if p.text.startswith('Revenue summary:'))
    assert any(r.text == 'stable' and r.bold is False for r in p.runs)
    assert d.tables[0].cell(0, 0).paragraphs[0].style.style_id == 'Normal'
    assert d.tables[0].cell(0, 0).text == 'Table report text'
    assert d.paragraphs[0].text == 'Executive summary'
    assert d.paragraphs[0].style.style_id == 'ReportHeading'
report = {'engineSourceReferences': True, 'bodyAndTableReplacement': True,
          'relatedFollowingStyle': True, 'directFormattingRetained': True,
          'reopenedExport': True, 'microsoftWordExecution': False}
(ui / 'independent-reader.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report, indent=2))
