"""Verify native paste evidence; partial mode never accepts unperformed checks."""
import argparse
import hashlib
import json
from pathlib import Path
from docx import Document

parser = argparse.ArgumentParser()
parser.add_argument('folder', type=Path)
parser.add_argument('--partial', action='store_true')
args = parser.parse_args()
root = args.folder

def read(name):
    return json.loads((root / f'{name}.folio').read_text())

before = read('before')
replaced = read('replaced')
assert before['revision'] == 24 and replaced['revision'] == 25

def check_body(state, expected):
    doc = state['document']
    body = doc['content'][1]
    assert ''.join(run.get('text', '') for run in body['content']) == expected
    assert body['attrs'] == before['document']['content'][1]['attrs']
    assert doc['attrs'] == before['document']['attrs']
    assert len(doc['content']) == len(before['document']['content'])
    for i, node in enumerate(doc['content']):
        if i != 1:
            assert node == before['document']['content'][i], i

check_body(replaced, 'Paste stays here')
report = {
    'version': '0.53.0', 'build': '5', 'nativeReplacementPreservesNeighbors': True,
    'nativeContinuationAndHistory': False, 'nativeRestart': False,
    'independentDocxRead': False, 'wordExecution': False,
}
if args.partial:
    user_after = root / 'user-at-lock.folio'
    report['status'] = 'partial: Mac locked before continuation/history/restart'
else:
    for name, revision, body in [
        ('continued', 26, 'Paste stays here!'),
        ('undone', 27, 'Paste stays here'),
        ('redone', 28, 'Paste stays here!'),
        ('following', 29, 'Paste stays here! Next'),
    ]:
        value = read(name)
        assert value['revision'] == revision, name
        check_body(value, body)
    assert read('undone')['document'] == replaced['document']
    assert read('redone')['document'] == read('continued')['document']
    assert (root / 'before-restart.folio').read_bytes() == (root / 'after-restart.folio').read_bytes()
    assert read('after-restart')['document'] == read('following')['document']
    doc = Document(root / 'native-paste.docx')
    assert [p.text for p in doc.paragraphs[:4]] == [
        'Native title', 'Paste stays here! Next', 'Section title', 'Final body text.',
    ]
    style = doc.styles['Native accent']
    assert style.font.size.pt == 22
    assert str(style.font.color.rgb) == '176B4D'
    assert doc.paragraphs[1].style.style_id == style.style_id
    assert doc.paragraphs[0].style.style_id == 'Heading1'
    assert doc.paragraphs[2].style.style_id == 'Heading2'
    assert all(p._p.xpath('./w:pPr/w:numPr') for p in doc.paragraphs[4:6])
    user_after = root / 'user-after-check.folio'
    report.update(status='scoped checks passed', nativeContinuationAndHistory=True,
                  nativeRestart=True, independentDocxRead=True, finalRevision=29)
assert user_after.read_bytes() == (root / 'user-before-check.folio').read_bytes()
report['userWorkspaceUnchanged'] = True
report['userWorkspaceSha256'] = hashlib.sha256(user_after.read_bytes()).hexdigest()
report['remaining'] = [
    'Rapid history/typing interleaving and native IME',
    'Per-history-entry original selection after later navigation',
    'Complete Word/Excel parity and Microsoft application comparison',
]
(root / ('verification-partial.json' if args.partial else 'verification.json')).write_text(
    json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
