"""Independently inspect named-move fixtures and resolved DOCX files as ZIP/XML."""
from pathlib import Path
from zipfile import ZipFile
import json
import sys
import tempfile
from lxml import etree

W = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
NS = {'w': W}


def read(path):
    with ZipFile(path) as package:
        assert package.read('word/media/retained.bin') == bytes([9, 8, 7, 6])
        doc = etree.fromstring(package.read('word/document.xml'))
        assert not any('folio-review' in key for n in doc.iter() for key in n.attrib)
        return doc


def visible_paragraphs(doc):
    result = []
    for p in doc.xpath('/w:document/w:body/w:p', namespaces=NS):
        result.append(''.join(n.text or '' for n in p.xpath('.//w:t', namespaces=NS)
                              if not n.xpath('ancestor::w:del | ancestor::w:moveFrom', namespaces=NS)))
    return result


def verify(folder):
    source = read(folder / 'folio-moves-input.docx')
    assert source.xpath('count(//w:moveFrom | //w:moveTo)', namespaces=NS) == 2
    assert source.xpath('//w:moveFromRangeStart/@w:name', namespaces=NS) == ['budgetMove']
    assert source.xpath('//w:moveToRangeStart/@w:name', namespaces=NS) == ['budgetMove']
    expected = {
        'accepted': ['Planning notes', 'Before:  stays.', 'After: Budget 😀 here.', 'Unrelated note.'],
        'rejected': ['Planning notes', 'Before: Budget 😀 stays.', 'After:  here.', 'Unrelated note.'],
        'core-export': ['Planning notes', 'Before: Budget 😀 stays.', 'After:  here.', 'Unrelated note.'],
        'paragraph-accepted': ['Before.', 'Stay.', 'Moving paragraph.'],
        'paragraph-rejected': ['Before.', 'Moving paragraph.', 'Stay.'],
        'nested-accept': ['', 'One two'],
        'nested-reject': ['One two', ''],
    }
    result = {}
    for name, texts in expected.items():
        doc = read(folder / f'folio-moves-{name}.docx')
        assert not doc.xpath('//*[starts-with(local-name(), "move")]', namespaces=NS), name
        assert visible_paragraphs(doc) == texts, (name, visible_paragraphs(doc))
        if name in {'accepted', 'rejected', 'core-export'}:
            assert doc.xpath('//w:ins/@w:author', namespaces=NS) == ['Morgan']
            assert doc.xpath('//w:rPr/w:color/@w:val', namespaces=NS) == ['185ABD']
        if name.startswith('nested-'):
            assert doc.xpath('//w:del/w:r/w:delText/text()', namespaces=NS) == ['earlier']
            assert doc.xpath('count(//w:del)', namespaces=NS) == 1
        result[name] = texts
    for name in ['cells-accepted', 'cells-rejected']:
        doc = read(folder / f'folio-moves-{name}.docx')
        cells = doc.xpath('//w:tc', namespaces=NS)
        assert len(cells) == 2
        assert all(cell[-1].tag == f'{{{W}}}p' for cell in cells)
        assert ''.join(doc.xpath('//w:t/text()', namespaces=NS)) == 'Moving paragraph.'
    retained = read(folder / 'folio-moves-hidden-export.docx')
    assert retained.xpath('count(//w:moveFrom | //w:moveTo)', namespaces=NS) == 4
    assert len(retained.xpath('//*[contains(local-name(), "RangeStart") or contains(local-name(), "RangeEnd")]', namespaces=NS)) == 4
    assert 'Stay here.' in retained.xpath('//w:t/text()', namespaces=NS)
    return {'paragraphs': result, 'cellEndParagraphs': True, 'nestedDeletionRetained': True,
            'unrelatedAuthorRetained': True, 'boundariesSurviveOrdinaryEdit': True,
            'unrelatedPackagePayloadRetained': True}


if __name__ == '__main__':
    print(json.dumps(verify(Path(sys.argv[1]) if len(sys.argv) > 1 else Path(tempfile.gettempdir())), indent=2))
