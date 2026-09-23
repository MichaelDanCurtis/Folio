"""Read style-engine artifacts independently using python-docx and XML.

This proves declaration/link preservation in the emitted packages. It does not
run Microsoft Word or validate Folio rendering, theme resolution or typing.
"""
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
import json
from docx import Document
from lxml import etree

ROOT = Path(gettempdir()) / 'folio-style-tests'
W = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
NS = {'w': W}

def check():
    opened = []
    for name in ['edited.docx', 'recovered.docx', 'undone.docx', 'created.docx',
                 'deleted.docx', 'native-heading.docx', 'native-heading-updated.docx',
                 'created-style-part.docx', 'legacy-recovered.docx', 'outline-before.docx',
                 'outline-after.docx', 'outline-removed.docx', 'tab-clears.docx']:
        doc = Document(ROOT / name)
        opened.append(name)
        if name in ['edited.docx', 'recovered.docx']:
            base = doc.styles['Body base renamed']
            child = doc.styles['Body child']
            assert base.style_id == 'StyleBase'
            assert child.base_style.style_id == 'StyleBase'
            assert child.next_paragraph_style.style_id == 'Normal'
            assert base.font.size.pt == 16
            assert base.font.name == 'Arial'
            assert base.font.bold is True and child.font.bold is True
            assert base.paragraph_format.space_after.pt == 18
            assert base.paragraph_format.right_indent.pt == 4.5
            assert base.paragraph_format.alignment == 2  # RIGHT
            for p in [doc.paragraphs[0], doc.paragraphs[1], doc.tables[0].cell(0, 0).paragraphs[0]]:
                assert p.style.style_id == 'StyleChild'
            assert doc.paragraphs[0].paragraph_format.space_after.pt == 12
            assert doc.paragraphs[0].paragraph_format.left_indent.pt == 36
            assert doc.paragraphs[0].runs[0].bold is True
            assert doc.paragraphs[0].runs[1].italic is True
            assert doc.sections[0].header.paragraphs[0].text == 'Retained header'
            with ZipFile(ROOT / name) as z:
                xml = etree.fromstring(z.read('word/styles/named.xml'))
                node = xml.xpath('./w:style[@w:styleId="StyleBase"]', namespaces=NS)[0]
                assert node.get('{urn:folio:styles-test}flag') == 'style'
                assert node.xpath('./w:rPr/w:lang/@w:val', namespaces=NS) == ['en-NZ']
                assert node.xpath('./w:pPr/w:keepNext', namespaces=NS)
                assert node.find('{urn:folio:styles-test}payload').text == 'retain me'
        elif name == 'created.docx':
            assert doc.paragraphs[1].style.style_id == 'ProjectHeading'
            assert doc.styles['Project heading'].base_style.style_id == 'StyleBase'
            assert doc.styles['Project heading'].element.xpath('./w:pPr/w:outlineLvl/@w:val') == ['1']
        elif name == 'deleted.docx':
            assert not any(s.style_id == 'ProjectHeading' for s in doc.styles)
            assert doc.paragraphs[1].style.style_id == 'Normal'
        elif name.startswith('native-heading'):
            assert doc.paragraphs[0].style.style_id == 'NativeHeading'
            level = '2' if 'updated' in name else '1'
            assert doc.paragraphs[0].style.element.xpath('./w:pPr/w:outlineLvl/@w:val') == [level]
        elif name.startswith('outline-'):
            assert doc.paragraphs[1].style.style_id == 'StyleChild'
            expected = {'outline-before.docx': ['1'], 'outline-after.docx': ['2'], 'outline-removed.docx': []}[name]
            assert doc.paragraphs[1].style.element.xpath('./w:pPr/w:outlineLvl/@w:val') == expected
        elif name == 'tab-clears.docx':
            child = doc.styles['Body child']
            assert child.element.xpath('./w:pPr/w:tabs/w:tab/@w:val') == ['clear', 'right']
            assert doc.styles['Body base'].font.size is None
            assert doc.styles['Body base'].paragraph_format.space_after is None
            assert doc.paragraphs[0].paragraph_format.tab_stops[0].position.pt == 72
        elif name == 'created-style-part.docx':
            assert doc.styles['First style'].font.bold is True
            assert doc.styles['First style'].paragraph_format.space_after.pt == 10
    report = {'artifacts': opened, 'count': len(opened), 'reader': 'python-docx and lxml',
              'wordExecution': False, 'nativeRendering': False}
    (ROOT / 'independent-reader.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))

if __name__ == '__main__':
    check()
