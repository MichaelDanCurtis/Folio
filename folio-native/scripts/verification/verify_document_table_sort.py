"""Inspect table sort DOCX fixtures independently of Folio's parsers."""
import argparse
import hashlib
import json
from pathlib import Path
from zipfile import ZipFile
import xml.etree.ElementTree as ET

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
W14 = '{http://schemas.microsoft.com/office/word/2010/wordml}'
W15 = '{http://schemas.microsoft.com/office/word/2012/wordml}'


def parts(path):
    with ZipFile(path) as archive:
        assert archive.testzip() is None
        return {n: archive.read(n) for n in archive.namelist()}


def canonical(node):
    return (node.tag, sorted(node.attrib.items()), node.text or '',
            [canonical(n) for n in node], node.tail or '')


def text(node):
    return ''.join(n.text or '' for n in node.iter(W + 't'))


def ranges(root, name):
    offset = 0
    opened = {}
    result = {}
    def visit(node):
        nonlocal offset
        if node.tag == W + name + 'Start':
            opened[node.get(W + 'id')] = offset
        elif node.tag == W + name + 'End':
            ident = node.get(W + 'id')
            assert ident in opened
            result[ident] = (opened.pop(ident), offset)
        elif node.tag == W + 't':
            offset += len((node.text or '').encode('utf-16-le')) // 2
        elif node.tag in (W + 'tab', W + 'br'):
            offset += 1
        for child in node:
            visit(child)
        if node.tag == W + 'p':
            offset += 1
    visit(root.find(W + 'body'))
    assert not opened
    return result


def verify(folder):
    source = parts(folder / 'source.docx')
    original = ET.fromstring(source['word/document.xml'])
    source_rows = original.findall(f'{W}body/{W}tbl/{W}tr')
    assert len(source_rows) == 5
    records = []
    for filename in ['sorted.docx', 'recovered.docx']:
        exported = parts(folder / filename)
        root = ET.fromstring(exported['word/document.xml'])
        body = root.find(W + 'body')
        assert [text(n) for n in body.findall(W + 'p')] == ['Before table', 'After table']
        tables = body.findall(W + 'tbl')
        assert len(tables) == 1
        rows = tables[0].findall(W + 'tr')
        assert [[text(c) for c in row.findall(W + 'tc')] for row in rows] == [
            ['Name', 'Amount', 'Date'], ['Alpha', '2', '2026-01-01'],
            ['Alpha', '2', '2026-02-01'], ['Beta', '2', '2026-03-01'], ['Zulu!?', '10', '2026-02-01']]
        for new, old in enumerate([0, 2, 4, 3, 1]):
            assert canonical(rows[new].find(W + 'trPr')) == canonical(source_rows[old].find(W + 'trPr'))
            for cell, prior in zip(rows[new].findall(W + 'tc'), source_rows[old].findall(W + 'tc')):
                assert canonical(cell.find(W + 'tcPr')) == canonical(prior.find(W + 'tcPr'))
            if old != 1:
                assert canonical(rows[new]) == canonical(source_rows[old])
        assert len(rows[0].findall(f'{W}trPr/{W}tblHeader')) == 1
        assert all(n.find(f'{W}rPr/{W}b') is not None for n in rows[-1].iter(W + 'r') if n.find(W + 't') is not None)
        bookmarks = ranges(root, 'bookmark')
        comments = ranges(root, 'commentRange')
        logical = ''.join(text(p) + '\n' for p in body.iter(W + 'p'))
        assert len(bookmarks) == len(comments) == 1
        for start, end in [*bookmarks.values(), *comments.values()]:
            assert logical.encode('utf-16-le')[start*2:end*2].decode('utf-16-le') == 'Zulu'
        names = [n.get(W + 'name') for n in root.iter(W + 'bookmarkStart')]
        assert names == ['ZuluMark']
        comment_xml = ET.fromstring(exported['word/comments.xml'])
        entries = comment_xml.findall(W + 'comment')
        assert len(entries) == 1 and text(entries[0]) == 'Keep this row'
        assert entries[0].get(W + 'author') == 'Test reviewer'
        assert set(comments) == {entries[0].get(W + 'id')}
        pids = {n.get(W14 + 'paraId') for n in comment_xml.iter(W + 'p')}
        extended = ET.fromstring(exported['word/commentsExtended.xml'])
        assert all(n.get(W15 + 'paraId') in pids for n in extended)
        assert source.keys() == exported.keys()
        for part in source.keys() - {'word/document.xml'}:
            if part.endswith('.xml'):
                assert canonical(ET.fromstring(source[part])) == canonical(ET.fromstring(exported[part])), part
            else:
                assert source[part] == exported[part], part
        records.append({'file': filename, 'sha256': hashlib.sha256((folder / filename).read_bytes()).hexdigest(),
                        'rows': 5, 'headerRows': 1, 'bookmarks': bookmarks, 'comments': comments})
    assert (folder / 'sorted.docx').read_bytes() == (folder / 'recovered.docx').read_bytes()
    return {'passed': True, 'files': records, 'recoveredExportByteIdentical': True,
            'rowFormatsPreserved': True, 'unrelatedPackagePartsSemanticallyUnchanged': True}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
