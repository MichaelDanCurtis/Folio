"""Independently inspect the synthetic files saved through the native 0.60 UI."""
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path
from zipfile import ZipFile

from lxml import etree
from verify_docx_tracking import NS, W, visible


def verify(folder):
    original = (
        'Quarterly planning update\nThe revenue forecast is 141000.\n'
        'Delivery remains our priority.\nReview due Friday.\n'
    )
    results = []
    for name in ['tracked-native.docx', 'working-native.docx', 'continued-native.docx']:
        continued = name == 'continued-native.docx'
        expected = original + ('Confirmed.\n' if continued else '')
        authors = Counter({'Alex': 5, 'Morgan': 2} if continued else {'Alex': 5})
        path = folder / name
        with ZipFile(path) as package:
            tree = etree.fromstring(package.read('word/document.xml'))
            assert visible(tree) == expected, (name, visible(tree))
            changes = tree.xpath('//w:ins | //w:del | //w:rPrChange | //w:pPrChange', namespaces=NS)
            assert Counter(n.get(f'{{{W}}}author') for n in changes) == authors, name
            ids = [n.get(f'{{{W}}}id') for n in changes]
            assert len(set(ids)) == len(ids), name
            assert tree.xpath('//w:del/w:r/w:delText[text()="25000"]', namespaces=NS), name
            assert tree.xpath('//w:ins/w:r/w:t[text()="41000"]', namespaces=NS), name
            assert tree.xpath('//w:rPr/w:b/following-sibling::w:rPrChange', namespaces=NS), name
            marks = tree.xpath('//w:pPr/w:rPr/w:ins', namespaces=NS)
            assert Counter(n.get(f'{{{W}}}author') for n in marks) == Counter(
                {'Alex': 1, 'Morgan': 1} if continued else {'Alex': 1}
            ), name
            if continued:
                assert tree.xpath('//w:ins[@w:author="Morgan"]/w:r/w:t[text()="Confirmed."]', namespaces=NS), name
            settings = etree.fromstring(package.read('word/settings.xml'))
            assert settings.find('w:trackRevisions', NS) is not None, name
            assert settings.find('w:doNotTrackFormatting', NS) is None, name
        results.append({
            'file': name, 'currentText': expected, 'revisionsByAuthor': dict(authors),
            'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
        })
    return results


if __name__ == '__main__':
    print(json.dumps(verify(Path(sys.argv[1])), indent=2))
