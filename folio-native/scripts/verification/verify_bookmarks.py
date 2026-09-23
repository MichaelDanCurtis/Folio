#!/usr/bin/env python3
"""Read core-generated bookmark fixtures with independent XML and DOCX readers."""
import argparse
import hashlib
import json
from pathlib import Path
from zipfile import ZipFile
from xml.etree import ElementTree as ET
from docx import Document

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'


def inspect(path):
    with ZipFile(path) as package:
        parts = {name: package.read(name) for name in package.namelist()}
    body = ET.fromstring(parts['word/document.xml']).find(W + 'body')
    position = 0
    text = []
    starts = {}
    ends = {}

    def visit(node):
        nonlocal position
        if node.tag == W + 'bookmarkStart':
            key = node.attrib[W + 'id']
            assert key not in starts, f'Duplicate start {key}'
            starts[key] = (node.attrib[W + 'name'], position)
        elif node.tag == W + 'bookmarkEnd':
            key = node.attrib[W + 'id']
            assert key not in ends, f'Duplicate end {key}'
            ends[key] = position
        elif node.tag == W + 't':
            value = node.text or ''
            text.append(value)
            position += len(value.encode('utf-16-le')) // 2
        elif node.tag in (W + 'tab', W + 'br', W + 'cr'):
            text.append('\t' if node.tag == W + 'tab' else '\n')
            position += 1
        else:
            for child in node:
                if child.tag not in (W + 'pPr', W + 'rPr'):
                    visit(child)
            if node.tag == W + 'p':
                text.append('\n')
                position += 1

    visit(body)
    assert set(starts) == set(ends), 'Unpaired bookmark markers'
    ranges = {name: [start, ends[key]] for key, (name, start) in starts.items()}
    assert all(start <= end for start, end in ranges.values())
    # This reader is separate from Folio's serializer and range projector.
    doc = Document(path)
    assert len(doc.paragraphs) == 3
    assert doc.paragraphs[-1].text == 'Unrelated'
    assert 'Next\nline' == doc.paragraphs[1].text
    return parts, ''.join(text), ranges


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    source, original_text, original_ranges = inspect(args.directory / 'source.docx')
    assert original_text == 'A😀 sales\ttoday end\nNext\nline\nUnrelated\n'
    assert original_ranges == {'Revenue': [4, 15], 'Across': [15, 29], '_Hidden': [29, 29]}
    outputs = []
    for name in ('edited.docx', 'reopened.docx'):
        parts, text, ranges = inspect(args.directory / name)
        assert text == 'Prefix ' + original_text
        assert ranges == {'Revenue': [11, 22], '_Hidden': [36, 36], 'Start': [0, 0]}
        for part in source:
            if part != 'word/document.xml':
                assert source[part] == parts[part], f'Changed untouched part: {part}'
        dom = ET.fromstring(parts['word/document.xml'])
        assert any(n.attrib.get(W + 'anchor') == 'Revenue' for n in dom.iter(W + 'hyperlink'))
        assert not any(n.tag.startswith(W) and 'BookmarkEvents' in n.tag for n in dom.iter())
        outputs.append(parts)
    assert outputs[0] == outputs[1], 'Export changed after workspace recovery'
    report = {
        'result': 'passed',
        'checks': ['UTF-16 positions', 'paired unique markers', 'range text',
                   'point and hidden bookmarks', 'deletion', 'internal hyperlink',
                   'unchanged other package parts', 'recovered export equality',
                   'python-docx paragraph text'],
        'files': {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in sorted(args.directory.iterdir()) if p.is_file()},
        'limits': 'These are independent package checks, not Microsoft Word or native UI qualification.',
    }
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
