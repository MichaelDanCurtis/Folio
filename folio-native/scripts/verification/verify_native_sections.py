"""Inspect native section-break saves and recovery without Folio's engine."""
import argparse, hashlib, json, zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

p = argparse.ArgumentParser()
p.add_argument('folder', type=Path)
root = p.parse_args().folder
read = lambda name: json.loads((root/(name+'.folio')).read_text())
base, split, merged, undo, final = [read(n) for n in ['baseline','split','merged','undone','after-restart']]
assert [v['revision'] for v in [base,split,merged,undo,final]] == [1,3,4,5,7]
assert undo['document'] == final['document'] == split['document']
assert (root/'before-restart.folio').read_bytes() == (root/'after-restart.folio').read_bytes()
for value, margins, bookmark_end in [(base,[1440],13),(split,[1440,720],14),(merged,[720],13)]:
    assert [s['geometry']['left'] for s in value['document']['attrs']['pageLayout']['sections']] == margins
    bookmark = next(b for b in value['document']['attrs']['bookmarks'] if b['name']=='All')
    assert bookmark['start'] == 0 and bookmark['end'] == bookmark_end
W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
def read_doc(name):
    with zipfile.ZipFile(root/name) as z:
        return ET.fromstring(z.read('word/document.xml')).find(W+'body'), z.read('word/header1.xml'), z.read('word/footer1.xml')
source, header, footer = read_doc('sections-source.docx')
for name, margins, paragraphs in [
    ('sections-split.docx',[1440,720],['Alpha 😀',' beta','Second paragraph.']),
    ('sections-merged.docx',[720],['Alpha 😀 beta','Second paragraph.']),
    ('sections-restored.docx',[1440,720],['Alpha 😀',' beta','Second paragraph.']),
]:
    body, h, f = read_doc(name)
    assert h == header and f == footer, name
    sections = list(body.iter(W+'sectPr'))
    assert [int(s.find(W+'pgMar').get(W+'left')) for s in sections] == margins
    assert [''.join(t.text or '' for t in n.iter(W+'t')) for n in body.findall(W+'p')] == paragraphs
    assert len(list(body.iter(W+'hyperlink'))) == 1
    assert len(list(body.iter(W+'bookmarkStart'))) == len(list(body.iter(W+'bookmarkEnd'))) == 1
    assert len(list(body.iter(W+'headerReference'))) == len(list(body.iter(W+'footerReference'))) == 1
    if len(sections) == 2:
        assert sections[1].find(W+'type').get(W+'val') == 'continuous'
print(json.dumps({'passed':True,'revision':7,'undoExact':True,'postSaveUndoExact':True,
    'restartByteIdentical':True,'independentOfficeRead':True,'nativeOfficeArtifacts':3,
    'recoverySha256':hashlib.sha256((root/'after-restart.folio').read_bytes()).hexdigest(),
    'boundary':'Verifies retained artifacts. Native actions are separately observed; Microsoft Word was not run.'},indent=2))
