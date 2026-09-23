"""Independent DOCX check for the real-bridge row page-break workflow."""
import argparse
import hashlib
import json
from pathlib import Path
from zipfile import ZipFile
import xml.etree.ElementTree as ET

p = argparse.ArgumentParser()
p.add_argument('folder', type=Path)
args = p.parse_args()
W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'


def signature(n):
    return [n.tag, sorted(n.attrib.items()), n.text or '', [signature(c) for c in n]]


packages, signatures, records = [], [], []
for name, keep in [('source.docx', None), ('kept.docx', 'on'), ('recovered.docx', 'on')]:
    path = args.folder / name
    with ZipFile(path) as z:
        parts = {n: z.read(n) for n in z.namelist()}
    root = ET.fromstring(parts['word/document.xml'])
    rows = root.findall(f'{W}body/{W}tbl/{W}tr')
    assert len(rows) == 1
    flag = rows[0].find(f'{W}trPr/{W}cantSplit')
    assert (flag.get(W + 'val') if flag is not None else None) == keep
    text = [n.text for n in root.iter(W + 't')]
    assert text == ([f'Before{i:02}' for i in range(30)] +
                    [f'A{i:02}' for i in range(18)] +
                    [f'B{i:02}' for i in range(18)] + ['After table'])
    assert len(list(root.iter(W + 'br'))) == 63
    # Ignore only the requested property and an otherwise-empty property wrapper.
    if flag is not None:
        pr = rows[0].find(W + 'trPr')
        pr.remove(flag)
        if not len(pr) and not pr.attrib:
            rows[0].remove(pr)
    signatures.append(signature(root))
    packages.append(parts)
    records.append({'file': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                    'cantSplit': keep, 'textRuns': len(text), 'lineBreaks': 63})
assert signatures[0] == signatures[1] == signatures[2], 'Unrelated document content changed'
for parts in packages[1:]:
    assert parts.keys() == packages[0].keys()
    for name, data in parts.items():
        if name != 'word/document.xml':
            assert data == packages[0][name], name
assert (args.folder / 'kept.docx').read_bytes() == (args.folder / 'recovered.docx').read_bytes()
print(json.dumps({'files': records, 'sourceContentRetained': True,
                  'unrelatedPartsUnchanged': True, 'recoveredBytesIdentical': True}, indent=2))
