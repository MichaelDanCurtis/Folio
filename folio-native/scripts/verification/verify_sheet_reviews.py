"""Independent readers for Folio's synthetic worksheet review fixtures.

Run after the Rust sheet_review tests and the real-bridge Flutter workflow.
OpenPyXL reads legacy note/placeholder records; lxml checks thread relationships,
people, references and VML independently of Folio's importer. This is not an
Excel desktop interoperability test.
"""
import hashlib
import json
import os
from pathlib import Path
import sys
from zipfile import ZipFile
from lxml import etree
from openpyxl import load_workbook

S = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'
T = 'http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments'
X = 'urn:schemas-microsoft-com:office:excel'
V = 'urn:schemas-microsoft-com:vml'


def verify(path):
    before = hashlib.sha256(path.read_bytes()).hexdigest()
    wb = load_workbook(path)
    legacy = {f'{s.title}!{c.coordinate}': {'author': c.comment.author, 'text': c.comment.text}
              for s in wb for row in s for c in row if c.comment}
    assert legacy, 'No notes or compatibility placeholders found'
    people, threads, shapes = {}, [], []
    with ZipFile(path) as z:
        for name in z.namelist():
            if not name.endswith(('.xml', '.vml')):
                continue
            root = etree.fromstring(z.read(name))
            for person in root.iter(f'{{{T}}}person'):
                person_id = person.get('id').upper()
                assert person_id not in people, 'Duplicate person identifier'
                people[person_id] = person.get('displayName')
            threads.extend(root.iter(f'{{{T}}}threadedComment'))
            for shape in root.iter(f'{{{V}}}shape'):
                client = shape.find(f'{{{X}}}ClientData')
                if client is not None and client.get('ObjectType') == 'Note':
                    shapes.append((int(client.findtext(f'{{{X}}}Row')), int(client.findtext(f'{{{X}}}Column'))))
        ids = {n.get('id').upper(): n for n in threads}
        assert len(ids) == len(threads), 'Duplicate thread identifiers'
        for n in threads:
            assert n.get('personId').upper() in people
            assert n.get('ref'), 'This authored fixture should emit references on all replies'
            if n.get('parentId'):
                parent = ids[n.get('parentId').upper()]
                assert parent.get('ref') == n.get('ref')
            else:
                assert any(v['author'] == f'tc={n.get("id")}' and a.split('!')[-1] == n.get('ref')
                           for a, v in legacy.items()), 'Missing matching compatibility placeholder'
        assert len(shapes) == len(legacy), 'VML note marker count does not match annotations'
    if path.name == 'folio-sheet-reviews.xlsx':
        assert legacy['Review!B1']['author'] == 'Reviewer'
        assert len(threads) == 2
        assert all(n.get('done') == '1' for n in threads)
        assert threads[0].findtext(f'{{{T}}}text').startswith('Check 😀')
        assert threads[1].findtext(f'{{{T}}}text') == 'Confirmed'
    elif path.name == 'folio-sheet-review-rich.xlsx':
        assert legacy['Copied!C3']['text'] == 'Rich note'
        assert not threads
        assert shapes == [(2, 2)]
    elif path.name == 'reviews.xlsx':
        assert legacy['Forecast!B2']['text'] == 'Costs include delivery and setup.'
        assert legacy['Forecast!C3']['text'] == legacy['Forecast!B2']['text']
        assert len(threads) == 3
        assert 'Confirmed the numbers and source.' in [t.findtext(f'{{{T}}}text') for t in threads]
    wb.close()
    assert hashlib.sha256(path.read_bytes()).hexdigest() == before
    return {'file': str(path), 'legacyCells': list(legacy), 'messages': len(threads),
            'people': people, 'vmlShapes': len(shapes), 'sha256': before, 'inputUnchanged': True}


paths = list(map(Path, sys.argv[1:])) or [Path(os.environ['TMPDIR']) / n for n in
    ['folio-sheet-reviews.xlsx', 'folio-sheet-review-rich.xlsx']]
print(json.dumps([verify(p) for p in paths], indent=2, ensure_ascii=False))
