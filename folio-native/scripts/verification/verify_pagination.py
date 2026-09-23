"""Check preview fixtures and UI exports without importing the application."""
import argparse
import json
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

p = argparse.ArgumentParser()
p.add_argument('source', type=Path)
p.add_argument('edited', type=Path)
p.add_argument('--unchanged', action='store_true')
a = p.parse_args()
W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
def read(path):
    with zipfile.ZipFile(path) as z:
        return {name: z.read(name) for name in z.namelist()}
def text(node):
    return ''.join(n.text or '' for n in node.iter(W+'t'))
source, edited = read(a.source), read(a.edited)
before, after = [ET.fromstring(pkg['word/document.xml']) for pkg in [source, edited]]
assert text(after) == ('' if a.unchanged else 'Pending preview edit. ') + text(before)
sections = list(after.iter(W+'sectPr'))
assert len(sections) == 2
assert [ET.tostring(s) for s in sections] == [ET.tostring(s) for s in before.iter(W+'sectPr')]
assert sections[1].find(W+'pgSz').get(W+'orient') == 'landscape'
assert sections[1].find(W+'type').get(W+'val') == 'oddPage'
assert sections[1].find(W+'pgNumType').get(W+'fmt') == 'lowerRoman'
assert sections[1].find(W+'pgNumType').get(W+'start') == '5'
for name in ['word/nested/header.xml', 'word/footer1.xml']:
    assert source[name] == edited[name], name
footer = ET.fromstring(edited['word/footer1.xml'])
fields = list(footer.iter(W+'fldSimple'))
assert [n.get(W+'instr').strip() for n in fields] == ['PAGE', 'NUMPAGES']
assert [text(n) for n in fields] == ['99', '99']
assert all(n.get(W+'dirty') == '1' for n in fields)
if a.unchanged:
    assert a.source.read_bytes() == a.edited.read_bytes()
print(json.dumps({'passed': True, 'source':str(a.source), 'edited':str(a.edited),
    'bodyChange': 'none' if a.unchanged else 'Pending preview edit. prefix only',
    'sectionGeometryUnchanged': True, 'fieldCachesUnchanged': True,
    'originalCache':99, 'documentBytesUnchanged':a.unchanged,
    'microsoftWordExecuted':False}, indent=2))
