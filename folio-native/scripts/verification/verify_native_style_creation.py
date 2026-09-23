"""Inspect retained native build-2 style creation, cancellation and history."""
import argparse
import hashlib
import json
from pathlib import Path

from docx import Document

parser = argparse.ArgumentParser()
parser.add_argument('folder', type=Path)
root = parser.parse_args().folder
revisions = {
    'before': 1, 'new-cancelled': 1, 'created': 2, 'modify-cancelled': 2,
    'applied': 3, 'apply-undo': 4, 'apply-redo': 5,
    'used-delete-rejected': 5, 'before-delete': 6, 'deleted': 7,
    'delete-undo': 8, 'delete-redo': 9,
    'before-restart': 13, 'after-restart': 13,
}
raw = {n: (root / f'build2-{n}.folio').read_bytes() for n in revisions}
states = {n: json.loads(v) for n, v in raw.items()}
for name, revision in revisions.items():
    assert states[name]['revision'] == revision, name
for left, right in [('new-cancelled', 'before'), ('modify-cancelled', 'created'),
                    ('used-delete-rejected', 'apply-redo'),
                    ('before-restart', 'after-restart')]:
    assert raw[left] == raw[right], (left, right)
for left, right in [('apply-undo', 'created'), ('apply-redo', 'applied'),
                    ('before-delete', 'created'), ('delete-undo', 'created'),
                    ('delete-redo', 'deleted'), ('after-restart', 'applied')]:
    assert states[left]['document'] == states[right]['document'], (left, right)

created = states['created']['document']
style = next(s for s in created['attrs']['wordStyles']['styles']
             if s['name'] == 'Native accent')
sid = style['id']
assert style['basedOn'] == style['next'] == 'Normal'
assert style['properties']['run'] == {'color': {'val': '176B4D'}, 'sz': 36}
assert all(s['id'] != sid for s in states['deleted']['document']['attrs']['wordStyles']['styles'])
assert states['deleted']['document']['content'] == created['content']
applied = states['applied']['document']
assert applied['attrs'] == created['attrs']
for index, node in enumerate(created['content']):
    if index != 1:
        assert applied['content'][index] == node, index
target = applied['content'][1]
assert target['content'] == created['content'][1]['content']
assert target['attrs']['paragraphFormat']['styleId'] == sid
assert target['attrs']['paragraphFormat']['inheritedRun']['sz'] == 36

doc = Document(root / 'build2-native-body.docx')
exported = doc.styles['Native accent']
assert exported.style_id == sid
assert exported.font.size.pt == 18 and str(exported.font.color.rgb) == '176B4D'
assert exported.base_style.style_id == exported.next_paragraph_style.style_id == 'Normal'
assert [(p.text, p.style.style_id) for p in doc.paragraphs[:4]] == [
    ('Native title', 'Heading1'), ('Plain body text.', sid),
    ('Section title', 'Heading2'), ('Final body text.', 'Normal'),
]
assert [p.text for p in doc.paragraphs[4:6]] == ['First bullet', 'Second bullet']
assert all(p._p.xpath('./w:pPr/w:numPr') for p in doc.paragraphs[4:6])
user_before = (root / 'user-before-check.folio').read_bytes()
assert user_before == (root / 'user-after-check.folio').read_bytes()
report = {
    'version': '0.53.0', 'build': '2', 'nativeStyleCreation': True,
    'nativeNewAndModifyCancelByteIdentical': True,
    'nativeTargetedStyleApply': True, 'nativeUnusedStyleDelete': True,
    'nativeApplyAndDeleteHistoryExact': True,
    'inUseStyleDeleteRejectedWithoutMutation': True,
    'independentDocxExport': True, 'restartByteIdentical': True,
    'finalRevision': 13, 'userWorkspaceUnchanged': True,
    'userWorkspaceSha256': hashlib.sha256(user_before).hexdigest(),
    'geometryEvidence': 'Native screenshots visually inspected; exact geometry covered by document_native_styles_workspace_test.dart.',
    'wordExecution': False,
    'remaining': ['Undo resets the caret in build 2',
                  'Replacing an in-use style during deletion',
                  'Remaining heading/list shortcuts and native input methods',
                  'Complete styles/themes/templates/organizer parity'],
}
(root / 'verification-build2.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
