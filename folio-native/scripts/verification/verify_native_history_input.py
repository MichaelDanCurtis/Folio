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
assert before['revision'] == 29
original = 'Paste stays here! Next'
for name, revision, text in [
    ('inserted', 30, original + ' old'),
    ('undo-typed', 32, original + 'X'),
    ('undone', 33, original),
    ('redo-typed', 35, original + 'XY'),
    ('repeated-undo-typed', 38, original + 'Z'),
    ('post-save-undo', 39, original),
    ('post-save-redo', 40, original + 'Z'),
    ('after-restart', 40, original + 'Z'),
]:
    value = read(name)
    assert value['revision'] == revision, name
    expected = json.loads(json.dumps(before['document']))
    expected['content'][1]['content'] = [{'type': 'text', 'text': text}]
    assert value['document'] == expected, name
    assert value['sheets'] == before['sheets'], name
assert (root/'before-restart.folio').read_bytes() == (root/'after-restart.folio').read_bytes()
assert (root/'user-before-check.folio').read_bytes() == (root/'user-after-check.folio').read_bytes()
doc = Document(root/'native-history-input.docx')
assert [p.text for p in doc.paragraphs[:4]] == [
    'Native title', original + 'Z', 'Section title', 'Final body text.',
]
assert [doc.paragraphs[i].style.style_id for i in (0, 2)] == ['Heading1', 'Heading2']
style = doc.styles['Native accent']
assert doc.paragraphs[1].style.style_id == style.style_id
assert style.font.size.pt == 22
assert str(style.font.color.rgb) == '176B4D'
assert all(p._p.xpath('./w:pPr/w:numPr') for p in doc.paragraphs[4:6])
report = {
    'version': '0.53.0', 'build': '6',
    'nativeUndoThenTyping': True, 'nativeRedoThenTyping': True,
    'nativeRepeatedUndoThenTyping': True, 'nativePostSaveHistory': True,
    'independentDocxRead': True, 'restartByteIdentical': True,
    'userWorkspaceUnchanged': True,
    'userWorkspaceSha256': hashlib.sha256((root/'user-after-check.folio').read_bytes()).hexdigest(),
    'finalRevision': 40, 'microsoftWordExecution': False,
    'scope': 'Native keyboard smoke checks plus separate deterministic delayed-engine regressions',
    'remaining': ['Native IME and broader formatting/history input combinations',
                  'Exact historical edit locations for ambiguous repeated text',
                  'Per-command original selection restoration after later navigation',
                  'Full Word/Excel parity and Microsoft application comparison'],
}
(root/'verification.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report, indent=2))
