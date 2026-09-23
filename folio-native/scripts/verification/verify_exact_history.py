"""Independently read repeat/Unicode history recovery and DOCX artifacts."""
import argparse
import json
from pathlib import Path
from tempfile import gettempdir
from docx import Document

parser = argparse.ArgumentParser()
parser.add_argument('folder', nargs='?', type=Path,
                    default=Path(gettempdir()) / 'folio-exact-history-tests')
root = parser.parse_args().folder
files = []
for name, expected in [
    ('ascii', ['XYaaaa', 'After']),
    ('unicode', ['XY😀😀', 'After']),
    ('paragraphs', ['XYLine', 'Line', 'After']),
]:
    raw = (root / f'{name}.folio').read_text()
    assert 'documentHistoryChange' not in raw
    workspace = json.loads(raw)
    assert [''.join(run.get('text', '') for run in paragraph.get('content', []))
            for paragraph in workspace['document']['content']] == expected
    doc = Document(root / f'{name}.docx')
    assert [paragraph.text for paragraph in doc.paragraphs] == expected
    files.extend([f'{name}.folio', f'{name}.docx'])
report = {'version': '0.53.0', 'build': '7', 'files': files,
          'unicodeAndRepeatedParagraphsPreserved': True,
          'historyJournalNotPersistedInDocument': True,
          'nativeExecution': False, 'microsoftWordExecution': False}
(root / 'independent-reader.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
