"""Independent checks of retained native history/input and restart evidence."""
import argparse
import hashlib
import json
from pathlib import Path
from docx import Document

parser = argparse.ArgumentParser()
parser.add_argument('folder', type=Path)
root = parser.parse_args().folder
read = lambda name: json.loads((root / f'{name}.folio').read_text())
before = read('before')
assert before['revision'] == 40
for name, revision, text in [
    ('ascii-base', 41, 'aaaa'),
    ('ascii-inserted', 42, 'aaaaa'),
    ('ascii-undo-typed', 44, 'Xaaaa'),
    ('ascii-undone', 45, 'aaaa'),
    ('ascii-redo-typed', 47, 'XYaaaa'),
    ('unicode-base', 48, '😀😀'),
    ('unicode-inserted', 49, '😀😀😀'),
    ('unicode-undo-typed', 51, 'X😀😀'),
    ('unicode-undone', 52, '😀😀'),
    ('unicode-redo-typed', 54, 'XY😀😀'),
    ('post-save-undo', 55, 'X😀😀'),
    ('post-save-redo', 56, 'XY😀😀'),
    ('after-restart', 56, 'XY😀😀'),
]:
    value = read(name)
    assert value['revision'] == revision, name
    expected = json.loads(json.dumps(before['document']))
    expected['content'][1]['content'] = [{'type': 'text', 'text': text}]
    assert value['document'] == expected, name
    assert value['sheets'] == before['sheets'], name
assert (root/'before-restart.folio').read_bytes() == (root/'after-restart.folio').read_bytes()
assert (root/'user-before-check.folio').read_bytes() == (root/'user-after-check.folio').read_bytes()
doc = Document(root/'native-exact-history.docx')
assert [p.text for p in doc.paragraphs[:4]] == [
    'Native title', 'XY😀😀', 'Section title', 'Final body text.',
]
assert [doc.paragraphs[i].style.style_id for i in (0, 2)] == ['Heading1', 'Heading2']
style = doc.styles['Native accent']
assert doc.paragraphs[1].style.style_id == style.style_id
assert style.font.size.pt == 22
assert str(style.font.color.rgb) == '176B4D'
assert all(p._p.xpath('./w:pPr/w:numPr') for p in doc.paragraphs[4:6])
report = {
    'version': '0.53.0', 'build': '7',
    'nativeRepeatedTextUndoRedoTyping': True,
    'nativeUnicodeUndoRedoTyping': True,
    'nativePostSaveHistory': True, 'independentDocxRead': True,
    'restartByteIdentical': True, 'userWorkspaceUnchanged': True,
    'userWorkspaceSha256': hashlib.sha256((root/'user-after-check.folio').read_bytes()).hexdigest(),
    'finalRevision': 56, 'microsoftWordExecution': False,
    'remaining': ['Per-command original selection after later navigation',
                  'Opaque embed internals and changes without exact journals',
                  'Native IME, broader concurrent edits and full Word/Excel parity'],
}
(root/'verification.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report, indent=2))
