#!/usr/bin/env python3
"""Independently check the real-engine rich-paste/table editing fixture."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

from docx import Document

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
EXPECTED_TEXT = 'aaaaaa\naaaaaaaa\nbbbbb\nTail\n'
EXPECTED_RANGES = {'Head': [2, 4], 'First': [11, 13], 'Second': [17, 19], 'Tail': [22, 26]}


def read_docx(path):
    with ZipFile(path) as archive:
        assert archive.testzip() is None
        parts = {name: archive.read(name) for name in archive.namelist()}
    root = ET.fromstring(parts['word/document.xml'])
    text, starts, ends = [], {}, {}
    offset = 0

    def visit(node):
        nonlocal offset
        if node.tag == W + 'bookmarkStart':
            key = node.attrib[W + 'id']
            assert key not in starts
            starts[key] = (node.attrib[W + 'name'], offset)
        elif node.tag == W + 'bookmarkEnd':
            key = node.attrib[W + 'id']
            assert key not in ends
            ends[key] = offset
        elif node.tag == W + 't':
            text.append(node.text or '')
            offset += len((node.text or '').encode('utf-16-le')) // 2
        elif node.tag in (W + 'tab', W + 'br', W + 'cr'):
            text.append('\t' if node.tag == W + 'tab' else '\n')
            offset += 1
        else:
            for child in node:
                if child.tag not in (W + 'pPr', W + 'rPr'):
                    visit(child)
            if node.tag == W + 'p':
                text.append('\n')
                offset += 1

    visit(root.find(W + 'body'))
    assert set(starts) == set(ends), 'Unpaired bookmarks'
    assert len({value[0] for value in starts.values()}) == len(starts)
    ranges = {name: [start, ends[key]] for key, (name, start) in starts.items()}
    assert ranges == EXPECTED_RANGES, ranges
    assert ''.join(text) == EXPECTED_TEXT
    doc = Document(path)
    assert [p.text for p in doc.paragraphs] == ['aaaaaa', 'Tail']
    assert len(doc.tables) == 1
    assert [[c.text for c in row.cells] for row in doc.tables[0].rows] == [['aaaaaaaa', 'bbbbb']]
    paragraphs = doc.paragraphs + [p for c in doc.tables[0].rows[0].cells for p in c.paragraphs]
    assert all(run.bold is True for p in paragraphs for run in p.runs if run.text)
    return parts


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('directory', type=Path)
    directory = parser.parse_args().directory
    before = json.loads((directory / 'original-workspace.folio').read_text())
    after = json.loads((directory / 'workspace.folio').read_text())
    expected = copy.deepcopy(before['document'])
    expected['content'][1]['content'][0]['content'][0]['content'][0]['content'][0]['text'] = 'aaaaaaaa'
    expected['content'][1]['content'][0]['content'][1]['content'][0]['content'][0]['text'] = 'bbbbb'
    for bookmark in expected['attrs']['bookmarks']:
        bookmark['start'], bookmark['end'] = EXPECTED_RANGES[bookmark['name']]
    assert after['document'] == expected, 'Unexpected document change'
    assert after['id'] == before['id'] + '-other', 'Stale-workspace test did not run'
    assert after['revision'] > before['revision']
    excluded = {'document', 'id', 'revision'}
    assert {k: v for k, v in before.items() if k not in excluded} == {
        k: v for k, v in after.items() if k not in excluded
    }, 'Non-document workspace content changed'
    edited, reopened = [read_docx(directory / name) for name in ('edited.docx', 'reopened.docx')]
    assert edited == reopened, 'Export differs after recovery'
    files = ['original-workspace.folio', 'original-document.json', 'workspace.folio', 'edited.docx', 'reopened.docx']
    print(json.dumps({
        'result': 'passed',
        'checks': ['exact saved content and ranges', 'unchanged workbook and workspace settings',
                   'paired unique DOCX markers', 'bookmark text and positions', 'table layout and cell text',
                   'bold formatting', 'identical package entries after recovery'],
        'files': {name: hashlib.sha256((directory / name).read_bytes()).hexdigest() for name in files},
        'limits': 'Independent file checks of automated UI fixtures; native interaction and Microsoft Word comparison remain unverified.',
    }, indent=2))


if __name__ == '__main__':
    main()
