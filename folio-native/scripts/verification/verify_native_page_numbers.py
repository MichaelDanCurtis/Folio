"""Inspect saved native page-number artifacts; does not execute Microsoft Word."""
import argparse
import hashlib
import json
import posixpath
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

parser = argparse.ArgumentParser()
parser.add_argument('folder', type=Path)
folder = parser.parse_args().folder
names = ['baseline', 'canceled', 'applied', 'undone', 'redone',
         'post-save-undo', 'post-save-redo', 'before-restart', 'after-restart']
snapshots = {name: json.loads((folder / f'{name}.folio').read_text()) for name in names}
assert [snapshots[n]['revision'] for n in names] == [1, 1, 2, 3, 4, 5, 6, 6, 6]
assert (folder / 'baseline.folio').read_bytes() == (folder / 'canceled.folio').read_bytes()
assert (folder / 'before-restart.folio').read_bytes() == (folder / 'after-restart.folio').read_bytes()
for key in ['document', 'officeDocument']:
    for name in ['undone', 'post-save-undo']:
        assert snapshots[name][key] == snapshots['baseline'][key], (name, key)
    for name in ['redone', 'post-save-redo', 'before-restart', 'after-restart']:
        assert snapshots[name][key] == snapshots['applied'][key], (name, key)

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'

def package(name):
    with zipfile.ZipFile(folder / name) as z:
        return {name: z.read(name) for name in z.namelist()}

def text(node):
    return ''.join(t.text or '' for t in node.iter(W + 't'))

source = package('numbers-source.docx')
final = package('numbers-final.docx')
original = ET.fromstring(source['word/document.xml'])
document = ET.fromstring(final['word/document.xml'])
assert text(document) == text(original) == 'Body one.Body two.Body three.'
sections = list(document.iter(W + 'sectPr'))
old_sections = list(original.iter(W + 'sectPr'))
assert len(sections) == len(old_sections) == 3
for index in [0, 2]:
    assert ET.tostring(sections[index]) == ET.tostring(old_sections[index])
numbering = sections[1].find(W + 'pgNumType')
assert numbering is not None
assert {k.removeprefix(W): v for k, v in numbering.attrib.items()} == {
    'fmt': 'lowerRoman', 'start': '5', 'chapStyle': '2', 'chapSep': 'colon'}
references = sections[1].findall(W + 'footerReference')
assert len(references) == 1 and references[0].get(W + 'type') == 'default'
rels = {n.get('Id'): n.get('Target')
        for n in ET.fromstring(final['word/_rels/document.xml.rels'])}
part = posixpath.normpath(posixpath.join('word', rels[references[0].get(R + 'id')])).lstrip('/')
footer = ET.fromstring(final[part])
fields = list(footer.iter(W + 'fldSimple'))
assert [f.get(W + 'instr').strip() for f in fields] == ['PAGE', 'NUMPAGES']
assert all(f.get(W + 'dirty') == '1' and text(f) == '?' for f in fields)
assert text(footer) == 'Footer retained Page ? of ?'
for part in ['word/nested/header.xml', 'word/header2.xml', 'word/footer1.xml']:
    assert final[part] == source[part], part
print(json.dumps({
    'passed': True, 'revision': 6, 'cancelExact': True, 'undoExact': True,
    'postSaveUndoExact': True, 'restartByteIdentical': True,
    'independentOfficeRead': True, 'nativeOfficeArtifacts': 2,
    'sectionNumbering': 'lowerRoman, start 5, chapter Heading 2, colon separator',
    'footerFields': ['PAGE', 'NUMPAGES'], 'pageValuesCalculated': False,
    'recoverySha256': hashlib.sha256((folder / 'after-restart.folio').read_bytes()).hexdigest(),
    'boundary': 'Saved files inspected independently. Native UI observed separately; Microsoft Word not executed.'
}, indent=2))
