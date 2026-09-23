"""Validate numbering exports with python-docx/lxml, independently of the Rust reader."""
from pathlib import Path
from zipfile import ZipFile
import tempfile
from lxml import etree
from docx import Document

folder=Path(tempfile.gettempdir())/'folio-numbering-tests'
W='http://schemas.openxmlformats.org/wordprocessingml/2006/main'
R='http://schemas.openxmlformats.org/package/2006/relationships'
ns={'w':W}
def xml(z,name):return etree.fromstring(z.read(name))
def check(name):
    path=folder/name
    doc=Document(path)
    with ZipFile(path) as z:
        body=xml(z,'word/document.xml')
        numbering=xml(z,'word/numbering.xml')
        nums={n.get(f'{{{W}}}numId'):n for n in numbering.findall('w:num',ns)}
        abstracts={n.get(f'{{{W}}}abstractNumId'):n for n in numbering.findall('w:abstractNum',ns)}
        assert len(nums)==len(numbering.findall('w:num',ns))
        assert len(abstracts)==len(numbering.findall('w:abstractNum',ns))
        # Abstract definitions must precede concrete instances.
        order=[etree.QName(n).localname for n in numbering]
        if 'num' in order: assert all(n!='abstractNum' for n in order[order.index('num'):])
        paragraphs=body.findall('w:body/w:p',ns)
        for p in paragraphs:
            pr=p.find('w:pPr',ns)
            if pr is not None: assert len(pr.findall('w:numPr',ns))<=1
            n=p.find('w:pPr/w:numPr',ns)
            if n is None:continue
            num=n.find('w:numId',ns).get(f'{{{W}}}val')
            if num=='0':continue
            assert num in nums,(name,num)
            abstract=nums[num].find('w:abstractNumId',ns).get(f'{{{W}}}val')
            assert abstract in abstracts
            level=n.find('w:ilvl',ns)
            idx='0' if level is None else level.get(f'{{{W}}}val')
            assert abstracts[abstract].find(f'w:lvl[@w:ilvl="{idx}"]',ns) is not None
        rels=xml(z,'word/_rels/document.xml.rels')
        assert any(n.get('Type','').endswith('/numbering') and n.get('Target')=='numbering.xml' for n in rels)
        content=xml(z,'[Content_Types].xml')
        assert any(n.get('PartName')=='/word/numbering.xml' and n.get('ContentType','').endswith('.numbering+xml') for n in content)
        if name in ('native.docx','appended.docx'):
            native=next(p for p in paragraphs if ''.join(p.xpath('.//w:t/text()',namespaces=ns))=='Native bold')
            assert native.find('w:hyperlink/w:r/w:rPr/w:b',ns) is not None
            nested=next(p for p in paragraphs if ''.join(p.xpath('.//w:t/text()',namespaces=ns))=='Nested bullet')
            assert nested.find('w:r/w:rPr/w:i',ns) is not None
            assert nested.find('w:pPr/w:numPr/w:ilvl',ns).get(f'{{{W}}}val')=='1'
            assert all(not p.text.startswith(('3. ','• ')) for p in doc.paragraphs)
        if name in ('restarted.docx','reopened.docx','legacy-reopened.docx'):
            assert paragraphs[8].find('w:pPr/w:numPr/w:numId',ns).get(f'{{{W}}}val')=='0'
            pr=paragraphs[8].find('w:pPr',ns)
            assert [etree.QName(n).localname for n in pr]==['pStyle','keepNext','numPr','spacing','jc']
            num=paragraphs[4].find('w:pPr/w:numPr/w:numId',ns).get(f'{{{W}}}val')
            abstract=nums[num].find('w:abstractNumId',ns).get(f'{{{W}}}val')
            assert abstracts[abstract].find('w:lvl/w:start',ns).get(f'{{{W}}}val')=='10'
        if name=='text-edited.docx':
            assert doc.paragraphs[0].text=='Edited project'
            assert doc.paragraphs[0].runs[0].bold
            with ZipFile(folder/'original.docx') as old:
                for part in old.namelist():
                    if part!='word/document.xml':assert old.read(part)==z.read(part),(name,part)
    print(f'{name}: {len(doc.paragraphs)} paragraphs; numbering, relationships, definitions and formatting verified')
for name in ['original.docx','text-edited.docx','restarted.docx','reopened.docx','legacy-reopened.docx','appended.docx','native.docx','prefixed.docx']: check(name)
assert (folder/'undone.docx').read_bytes()==(folder/'original.docx').read_bytes()
print('Exact original package restored by Undo.')
