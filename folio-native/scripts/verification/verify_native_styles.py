"""Check saved evidence from the native named-style walkthrough."""
import argparse
import hashlib
import json
from pathlib import Path
from docx import Document

parser = argparse.ArgumentParser()
parser.add_argument('folder', type=Path)
root = parser.parse_args().folder
names = ['imported', 'applied', 'undone', 'redone', 'following',
         'table-draft-uncommitted', 'table-applied', 'table-undone', 'table-redone']
values = {n: json.loads((root / f'{n}.folio').read_text()) for n in names}
for name, revision in zip(names, [1, 2, 3, 4, 6, 6, 7, 8, 9]):
    assert values[name]['revision'] == revision, name
for left, right in [('undone', 'imported'), ('redone', 'applied'),
                    ('table-draft-uncommitted', 'following'),
                    ('table-undone', 'following'), ('table-redone', 'table-applied')]:
    assert values[left]['document'] == values[right]['document'], (left, right)
doc = Document(root / 'native-styles.docx')
base = doc.styles['Native report base']
assert base.style_id == 'ReportBase' and base.font.size.pt == 20
assert doc.styles['Report body'].base_style.style_id == 'ReportBase'
follow = next(p for p in doc.paragraphs if p.text == 'Native follow-up')
assert follow.style.style_id == 'ReportBody'
revenue = next(p for p in doc.paragraphs if p.text.startswith('Revenue'))
assert any(r.text == 'stable' and r.bold is False for r in revenue.runs)
assert doc.tables[0].cell(0, 0).paragraphs[0].style.style_id == 'ReportHeading'
assert doc.paragraphs[-1].text == 'Unrelated paragraph.'
assert doc.paragraphs[-1].style.style_id == 'Normal'
assert (root / 'before-restart.folio').read_bytes() == (root / 'after-restart.folio').read_bytes()
assert (root / 'user-before-check.folio').read_bytes() == (root / 'user-after-check.folio').read_bytes()
report = {
    'nativeStyleModify': True, 'nativeKeyboardUndoRedo': True,
    'nativeFollowingStyle': True, 'nativeTableDraftAndHistory': True,
    'independentDocxExport': True, 'restartByteIdentical': True,
    'finalRevision': 9, 'userWorkspaceUnchanged': True,
    'userWorkspaceSha256': hashlib.sha256((root / 'user-after-check.folio').read_bytes()).hexdigest(),
    'wordExecution': False,
    'notQualified': ['native style creation/deletion and Cancel',
                     'native document first-style materialization',
                     'all heading/list shortcuts and native input methods',
                     'full styles/themes/templates/organizer parity'],
}
(root / 'verification.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
