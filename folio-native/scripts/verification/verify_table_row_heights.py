"""Independent package checks for the exact-row-height editor workflow."""
import argparse
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET
from zipfile import ZipFile

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'


def signature(node):
    return (node.tag, sorted(node.attrib.items()), node.text or '',
            node.tail or '', [signature(child) for child in node])


def verify(folder):
    paths = [folder / name for name in ['source.docx', 'exact.docx', 'recovered.docx']]
    packages = []
    for path in paths:
        with ZipFile(path) as archive:
            packages.append({name: archive.read(name) for name in archive.namelist()})
    assert paths[1].read_bytes() == paths[2].read_bytes(), 'Recovered export changed'
    original = packages[0]
    documents = []
    for i, package in enumerate(packages):
        assert original.keys() == package.keys(), 'Package part set changed'
        assert all(original[n] == package[n] for n in original if n != 'word/document.xml'), 'Unrelated part changed'
        document = ET.fromstring(package['word/document.xml'])
        rows = document.findall(f'{W}body/{W}tbl/{W}tr')
        assert len(rows) == 2, 'Row count changed'
        heights = [r.find(f'{W}trPr/{W}trHeight') for r in rows]
        assert [(h.get(W + 'hRule'), h.get(W + 'val')) for h in heights] == [
            ('auto', '0') if i == 0 else ('exact', '480'), ('exact', '600')]
        text = [n.text for n in document.iter(W + 't')]
        assert text == ['Visible row'] + [f'Hidden row {n}' for n in range(7)] + ['Second row', 'After table'], 'Clipping removed saved text'
        # Only this one height was changed through the table properties UI.
        heights[0].set(W + 'hRule', 'auto')
        heights[0].set(W + 'val', '0')
        documents.append(document)
    assert all(signature(d) == signature(documents[0]) for d in documents[1:]), 'Unexpected document edit'
    return {
        'passed': True,
        'rows': 2,
        'onlyFirstRowHeightChanged': True,
        'clippedTextRetained': True,
        'unrelatedPackagePartsUnchanged': True,
        'recoveryExportByteIdentical': True,
        'artifacts': {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},
        'microsoftWordExecuted': False,
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
