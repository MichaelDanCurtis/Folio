"""Independent OOXML checks for the repeated-table-header app workflow."""
import argparse
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET
from zipfile import ZipFile

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'


def parts(path):
    with ZipFile(path) as archive:
        return {name: archive.read(name) for name in archive.namelist()}


def signature(node):
    return (node.tag, sorted(node.attrib.items()), node.text or '',
            [signature(child) for child in node])


def verify(source, enabled, disabled, reopened):
    paths = [source, enabled, disabled, reopened]
    packages = [parts(path) for path in paths]
    assert source.read_bytes() == enabled.read_bytes(), 'Undo changed source package'
    assert disabled.read_bytes() == reopened.read_bytes(), 'Recovery changed export'
    documents = [ET.fromstring(p['word/document.xml']) for p in packages]
    for path, document in zip(paths, documents):
        tables = document.findall(f'{W}body/{W}tbl')
        assert len(tables) == 1, path
        rows = tables[0].findall(f'{W}tr')
        assert len(rows) == 47, 'Derived headings must not be inserted into the source'
        labels = [n.text for n in document.iter(W + 't')]
        assert labels.count('Quarterly accounts') == 1
        assert [v for v in labels if v and v.startswith('Account ')] == [
            f'Account {i:02}' for i in range(45)]
        assert rows[0].find(f'{W}tc/{W}tcPr/{W}gridSpan').get(W + 'val') == '2'
        assert rows[0].find(f'.//{W}b') is not None
        assert rows[0].find(f'{W}tc/{W}tcPr/{W}shd').get(W + 'fill') == 'D9EAF7'
        fields = list(document.iter(W + 'fldSimple'))
        assert [n.get(W + 'instr').strip() for n in fields] == ['PAGE', 'NUMPAGES']
        assert [''.join(n.itertext()) for n in fields] == ['1', '1']
        flags = [r.find(f'{W}trPr/{W}tblHeader') for r in rows]
        assert flags[0].get(W + 'val', 'on') == ('on' if path in [source, enabled] else 'off')
        assert flags[1].get(W + 'val', 'on') == 'on'
        assert all(n is None for n in flags[2:])
    original = packages[0]
    for package in packages[1:]:
        assert original.keys() == package.keys()
        assert all(original[name] == package[name] for name in original if name != 'word/document.xml')
    # Normalize only the intentionally edited flag; every other element,
    # attribute and text value must still match the source.
    for document in documents:
        document.find(f'{W}body/{W}tbl/{W}tr/{W}trPr/{W}tblHeader').set(W + 'val', 'on')
    assert all(signature(d) == signature(documents[0]) for d in documents[1:])
    return {
        'passed': True, 'rows': 47, 'sourceHeadingCopies': 1,
        'onlyFirstHeaderFlagChanged': True, 'savedFieldCachesUnchanged': True,
        'undoRestoresSourceBytes': True, 'recoveryExportByteIdentical': True,
        'artifacts': {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},
        'microsoftWordExecuted': False,
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['source', 'enabled', 'disabled', 'reopened']:
        parser.add_argument(name, type=Path)
    print(json.dumps(verify(**vars(parser.parse_args())), indent=2))
