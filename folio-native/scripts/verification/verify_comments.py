"""Independently read comment bodies, source anchors and thread metadata."""
from pathlib import Path
import hashlib, json, tempfile, zipfile
from docx import Document
from lxml import etree
root = Path(tempfile.gettempdir())
paths = [root / 'folio-comments-independent.docx']
ui = sorted(root.glob('folio-comments-ui-*/review.docx'), key=lambda p: p.stat().st_mtime)
if ui:
    paths.append(ui[-1])
ns = {'w': 'http://schemas.openxmlformats.org/wordprocessingml/2006/main',
      'w14': 'http://schemas.microsoft.com/office/word/2010/wordml',
      'w15': 'http://schemas.microsoft.com/office/word/2012/wordml'}
results = []
for path in paths:
    original = path.read_bytes()
    doc = Document(path)
    comments = list(doc.comments)
    assert len(comments) == 2
    with zipfile.ZipFile(path) as package:
        body = etree.fromstring(package.read('word/document.xml'))
        records = etree.fromstring(package.read('word/comments.xml'))
        extended = etree.fromstring(package.read('word/commentsExtended.xml'))
        ids = [c.get(f'{{{ns["w"]}}}id') for c in records]
        assert len(ids) == len(set(ids)) == 2
        refs = body.xpath('//w:commentReference/@w:id', namespaces=ns)
        starts = body.xpath('//w:commentRangeStart/@w:id', namespaces=ns)
        ends = body.xpath('//w:commentRangeEnd/@w:id', namespaces=ns)
        assert starts == ends == refs == [ids[0]]
        paras = [c.xpath('.//w:p', namespaces=ns)[-1].get(f'{{{ns["w14"]}}}paraId') for c in records]
        assert len(set(paras)) == 2 and all(0 < int(p, 16) < 0x80000000 for p in paras)
        replies = extended.xpath('//w15:commentEx/@w15:paraIdParent', namespaces=ns)
        assert replies == [paras[0]]
        states = extended.xpath('//w15:commentEx/@w15:done', namespaces=ns)
        if path.name == 'folio-comments-independent.docx':
            assert [c.author for c in comments] == ['Reviewer', 'Alex']
            assert comments[1].text == 'Verified & checked <all>'
            assert states == ['1', '1']
            assert 'Cell text' in doc.tables[0].cell(0, 0).text
        else:
            assert [c.author for c in comments] == ['Morgan', 'Morgan']
            assert comments[1].text == 'Confirmed against the attached workbook.'
            assert states == ['0', '0']
    assert path.read_bytes() == original
    results.append({'file': str(path), 'sha256': hashlib.sha256(original).hexdigest(),
                    'authors': [c.author for c in comments], 'messages': [c.text for c in comments],
                    'parentParagraph': replies[0], 'resolved': states, 'unchanged': True})
print(json.dumps(results, indent=2))
