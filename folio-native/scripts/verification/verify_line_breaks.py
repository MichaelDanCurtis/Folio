"""Read exported Word XML independently of the Folio codec and engine."""
import argparse
import hashlib
import json
import posixpath
from pathlib import Path
from zipfile import ZipFile
import xml.etree.ElementTree as ET

p = argparse.ArgumentParser()
p.add_argument('folder', type=Path)
args = p.parse_args()
W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
R = '{http://schemas.openxmlformats.org/package/2006/relationships}'


def text(paragraph):
    return ''.join('\n' if n.tag == W + 'br' else n.text or ''
                   for n in paragraph.iter() if n.tag in (W + 't', W + 'br'))


records = []
for name, edited in [('source.docx', False), ('edited.docx', True), ('recovered.docx', True)]:
    path = args.folder / name
    with ZipFile(path) as z:
        parts = {n: z.read(n) for n in z.namelist()}
    root = ET.fromstring(parts['word/document.xml'])
    body = root.find(W + 'body')
    paragraphs = body.findall(W + 'p')
    cells = body.findall(f'{W}tbl/{W}tr/{W}tc')
    assert len(paragraphs) == len(cells) == 1
    assert text(paragraphs[0]) == ('Alpha\nBeta\nTail' if edited else 'Alpha\nBeta')
    cell_paragraphs = cells[0].findall(W + 'p')
    assert len(cell_paragraphs) == 1
    assert text(cell_paragraphs[0]) == ('Left\nRight\nBottom' if edited else 'Left\nRight')
    assert len(list(root.iter(W + 'br'))) == (4 if edited else 2)
    # A line break must be a real element, never a newline hidden in text runs.
    assert all('\n' not in (n.text or '') and '\u2028' not in (n.text or '')
               for n in root.iter(W + 't'))
    rels = ET.fromstring(parts['word/_rels/document.xml.rels'])
    targets = {n.get('Id'): posixpath.normpath(posixpath.join('word', n.get('Target'))).lstrip('/')
               for n in rels.findall(R + 'Relationship')
               if n.get('Type').endswith('/header')}
    headers = [ET.fromstring(parts[target]) for target in targets.values()]
    if edited:
        assert len(headers) == 1
        assert len(headers[0].findall(W + 'p')) == 1
        assert text(headers[0]) == 'Header\nSecond'
        refs = root.findall(f'{W}body/{W}sectPr/{W}headerReference')
        assert len(refs) == 1
        rid = refs[0].get('{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id')
        assert targets[rid] in parts
    else:
        assert not headers
    records.append({'file': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                    'bodyParagraphs': 1, 'cellParagraphs': 1,
                    'bodyLineBreaks': 4 if edited else 2, 'headerLineBreaks': int(edited)})
assert (args.folder / 'edited.docx').read_bytes() == (args.folder / 'recovered.docx').read_bytes()
print(json.dumps({'passed': True, 'files': records, 'recoveryByteIdentical': True}, indent=2))
