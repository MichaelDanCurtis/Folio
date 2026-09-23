"""Read saved workspaces and DOCX exports from real-engine history/input tests."""
import argparse
import json
from pathlib import Path
from tempfile import gettempdir
from docx import Document
from docx.oxml.ns import qn

parser = argparse.ArgumentParser()
parser.add_argument('folder', nargs='?', type=Path,
                    default=Path(gettempdir()) / 'folio-history-input-tests')
args = parser.parse_args()
files = []
for name in ('immediate', 'saving', 'dispatch'):
    workspace = json.loads((args.folder / f'{name}.folio').read_text())
    content = workspace['document']['content']
    assert [''.join(run.get('text', '') for run in p.get('content', []))
            for p in content] == ['BodyBXY', 'After']
    bookmarks = workspace['document']['attrs']['bookmarks']
    assert len(bookmarks) == 1
    assert {key: bookmarks[0][key] for key in ('name', 'start', 'end')} == {
        'name': 'After', 'start': 8, 'end': 13,
    }
    document = Document(args.folder / f'{name}.docx')
    assert [p.text for p in document.paragraphs] == ['BodyBXY', 'After']
    after = document.paragraphs[1]._p
    starts = after.xpath('.//w:bookmarkStart[@w:name="After"]')
    assert len(starts) == 1
    identifier = starts[0].get(qn('w:id'))
    inside = False
    covered = ''
    for node in after.iter():
        if node.tag == qn('w:bookmarkStart') and node.get(qn('w:id')) == identifier:
            inside = True
        elif node.tag == qn('w:bookmarkEnd') and node.get(qn('w:id')) == identifier:
            inside = False
        elif inside and node.tag == qn('w:t'):
            covered += node.text or ''
    assert covered == 'After'
    files.extend([f'{name}.folio', f'{name}.docx'])
report = {
    'files': files,
    'expectedBody': 'BodyBXY',
    'untouchedNeighbor': 'After',
    'workspaceAndDocxBookmarksPreserved': True,
    'nativeExecution': False,
    'microsoftWordExecution': False,
}
(args.folder / 'independent-reader.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
