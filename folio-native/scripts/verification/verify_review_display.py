"""Check review projections and their source without importing the Folio engine."""
import json
import sys
from pathlib import Path
from zipfile import ZipFile
from lxml import etree
from verify_docx_tracking import NS, W, visible


def text(node):
    if node.get('type') == 'text':
        return node['text']
    return ''.join(text(n) for n in node.get('content', []))


def verify(source, projections):
    views = json.loads(projections.read_text())
    with ZipFile(source) as package:
        doc = etree.fromstring(package.read('word/document.xml'))
        changes = doc.xpath('//w:ins | //w:del | //w:rPrChange | //w:pPrChange', namespaces=NS)
        assert len(changes) == 5
        assert {n.get(f'{{{W}}}author') for n in changes} == {'Alex'}
        assert visible(doc) == 'Quarterly planning update\nThe revenue forecast is 141000.\nDelivery remains our priority.\nReview due Friday.\n'
    current = views['current']['document']['content']
    original = views['original']['document']['content']
    assert [text(n) for n in current] == ['Quarterly planning update', 'The revenue forecast is 141000.', 'Delivery remains our priority.', 'Review due Friday.']
    assert [text(n) for n in original] == ['Quarterly planning update', 'The revenue forecast is 125000.', 'Delivery remains our priority.']
    assert current[0]['content'][0]['marks'] == [{'type': 'bold'}]
    assert not any(m['type'] == 'bold' for n in original[0]['content'] for m in n.get('marks', []))
    assert original[2]['attrs']['reviewBlocks'] == ['word-2', 'word-3']
    assert [n['attrs']['reviewBlocks'] for n in current] == [[f'word-{i}'] for i in range(4)]
    return {'currentParagraphs': [text(n) for n in current], 'originalParagraphs': [text(n) for n in original], 'sourceRevisions': len(changes), 'joinedSourceAnchors': original[2]['attrs']['reviewBlocks'], 'formattingBeforeAfterVerified': True}


if __name__ == '__main__':
    print(json.dumps(verify(Path(sys.argv[1]), Path(sys.argv[2])), indent=2))
