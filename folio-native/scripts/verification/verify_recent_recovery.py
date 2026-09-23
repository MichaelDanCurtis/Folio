"""Read files produced by recent_files_workspace_test independently of Folio."""
from pathlib import Path
import json
import sys
from docx import Document

root = Path(sys.argv[1])
restored = json.loads((root / 'recovered.folio').read_text())

def text(node):
    return node.get('text', '') + ''.join(text(child) for child in node.get('content', []))

assert text(restored['document']).startswith('Keep this recovery copy. ')
assert 'A later version.' not in text(restored['document'])
assert len(restored['sheets']) == 1
assert restored['sheets'][0]['cells'] == {}
report = Document(root / 'moved.docx')
assert '\n'.join(p.text for p in report.paragraphs).startswith('A later version. Keep this recovery copy.')
assert 'Unsaved local edit.' not in '\n'.join(p.text for p in report.paragraphs)
copies = list((root / 'Snapshots').rglob('*.folio'))
assert len(copies) >= 4
originals = [json.loads(p.read_text()) for p in copies if text(json.loads(p.read_text())['document']).startswith('Keep this recovery copy.')]
assert any(w['id'] != restored['id'] for w in originals)
history = json.loads((root / 'recent-files.json').read_text())['files']
assert len(history) == 2
assert history[0]['path'] == str(root / 'moved.docx') and history[0]['pinned']
assert not any('/Snapshots/' in e['path'] for e in history)
assert all(p.name == 'Workspace.folio' for p in copies)
print(json.dumps({'fixtureDirectory': str(root), 'snapshots': len(copies),
    'recoveredDocumentAndWorkbook': True, 'savedDocumentReadByPythonDocx': True,
    'unsavedEditAbsentFromExternalDocument': True, 'freshRecoveredIdentity': True,
    'recentEntries': len(history), 'movedFilePinRetained': True}, indent=2))
