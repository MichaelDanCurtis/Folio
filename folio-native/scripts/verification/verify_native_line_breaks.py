"""Independently qualify captured native manual-break edits and Office exports.

Run against work/native-line-breaks or the archived artifacts directory. The
checkpoints contain the disposable fixture only, not the normal user workspace.
Keyboard/mouse and visual claims require the accompanying native observations.
"""
import argparse
import base64
import hashlib
import io
import json
import posixpath
from pathlib import Path
from zipfile import ZipFile
import xml.etree.ElementTree as ET

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
R = '{http://schemas.openxmlformats.org/package/2006/relationships}'
ODR = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def node_text(node):
    if node.get('type') == 'hardBreak':
        return '\n'
    return node.get('text', ''.join(node_text(c) for c in node.get('content', [])))


def paragraphs(node):
    if node.get('type') in ('paragraph', 'heading'):
        yield node
    else:
        for child in node.get('content', []):
            yield from paragraphs(child)


def xml_text(node):
    return ''.join('\n' if n.tag == W + 'br' else n.text or ''
                   for n in node.iter() if n.tag in (W + 't', W + 'br'))


def office_parts(raw):
    with ZipFile(io.BytesIO(raw)) as archive:
        assert archive.testzip() is None
        return {name: archive.read(name) for name in archive.namelist()}


def verify_headers(parts, expected):
    root = ET.fromstring(parts['word/document.xml'])
    rels = ET.fromstring(parts['word/_rels/document.xml.rels'])
    targets = {n.get('Id'): posixpath.normpath(
        posixpath.join('word', n.get('Target'))).lstrip('/')
        for n in rels.findall(R + 'Relationship')
        if n.get('Type').endswith('/header')}
    refs = root.findall(f'{W}body/{W}sectPr/{W}headerReference')
    assert len(targets) == len(refs) == int(expected)
    for ref in refs:
        target = targets[ref.get(ODR + 'id')]
        header = ET.fromstring(parts[target])
        assert len(header.findall(W + 'p')) == 1
        assert xml_text(header) == 'Header\nSecond'
        assert len(list(header.iter(W + 'br'))) == 1
        assert all('\n' not in (n.text or '') and '\u2028' not in (n.text or '')
                   for n in header.iter(W + 't'))


def main(folder):
    names = [
        'imported', 'body-break', 'body-typed', 'spelling-applied',
        'spelling-undone', 'spelling-redone', 'paragraph-enter',
        'paragraph-undone', 'cell-draft', 'cell-cancelled', 'cell-applied',
        'cell-undone', 'cell-redone', 'header-draft', 'header-cancelled',
        'header-applied', 'header-undone', 'header-redone', 'previewed',
        'exported', 'saved', 'postsave-undone', 'postsave-redone',
        'before-restart', 'restarted', 'recovered-export', 'reimported',
        'reimport-inspected', 'reimport-exported',
    ]
    revisions = [1, 2, 17, 18, 19, 20, 21, 22, 22, 22, 23, 24, 25, 25,
                 25, 26, 27, 28, 28, 28, 28, 29, 30, 30, 30, 30, 31, 31, 31]
    raw = {i: (folder / f'{i:02d}-{name}.json').read_bytes()
           for i, name in enumerate(names, 1)}
    states = {i: json.loads(value) for i, value in raw.items()}
    corrected = 'Alpha\nBeta\nWe received it.'
    has_header = {16, 18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 29}
    short_cell = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12}
    unchanged_keys = set(states[1]) - {'document', 'officeDocument', 'officePackages', 'revision'}
    original_paragraphs = list(paragraphs(states[1]['document']))
    for i, state in states.items():
        assert state['revision'] == revisions[i - 1], i
        actual = list(paragraphs(state['document']))
        body = {1: 'Alpha\nBeta', 2: 'Alpha\nBeta\n',
                3: 'Alpha\nBeta\nWe recieved it.',
                5: 'Alpha\nBeta\nWe recieved it.'}.get(i, corrected)
        cell = 'Left\nRight' if i in short_cell else 'Left\nRight\nBottom'
        assert [node_text(p) for p in actual] == ([body, '', cell] if i == 7 else [body, cell]), i
        # Paragraph properties, source identities and all non-document content
        # must survive the edits, not merely the visible text.
        assert actual[0]['attrs'] == original_paragraphs[0]['attrs'], i
        assert actual[-1]['attrs'] == original_paragraphs[-1]['attrs'], i
        for key in unchanged_keys:
            assert state[key] == states[1][key], (i, key)
        source = state['officePackages'][state['officeDocument']['packageId']]
        verify_headers(office_parts(base64.b64decode(source)), i in has_header)

    byte_groups = [[8, 9, 10], [13, 14, 15], [18, 19, 20, 21],
                   [23, 24, 25, 26], [27, 28, 29]]
    for group in byte_groups:
        assert all(raw[i] == raw[group[0]] for i in group), group
    history_groups = [[3, 5], [4, 6, 8, 9, 10, 12],
                      [11, 13, 14, 15, 17, 22],
                      [16, 18, 19, 20, 21, 23, 24, 25, 26]]
    for group in history_groups:
        compared = [{k: v for k, v in states[i].items() if k != 'revision'} for i in group]
        assert all(value == compared[0] for value in compared), group
    assert states[26]['document'] == states[27]['document']

    records = []
    outputs = []
    for name in ['source.docx', 'edited.docx', 'line-breaks.docx', 'recovered.docx', 'reimported.docx']:
        data = (folder / name).read_bytes()
        edited = name != 'source.docx'
        parts = office_parts(data)
        root = ET.fromstring(parts['word/document.xml'])
        body = root.find(W + 'body')
        ps = body.findall(W + 'p')
        tables = body.findall(W + 'tbl')
        assert len(ps) == len(tables) == 1
        rows = tables[0].findall(W + 'tr')
        assert len(rows) == 1
        cells = rows[0].findall(W + 'tc')
        assert len(cells) == 1
        cps = cells[0].findall(W + 'p')
        assert len(cps) == 1
        assert xml_text(ps[0]) == (corrected if edited else 'Alpha\nBeta')
        assert xml_text(cps[0]) == ('Left\nRight\nBottom' if edited else 'Left\nRight')
        assert len(list(root.iter(W + 'br'))) == (4 if edited else 2)
        assert all('\n' not in (n.text or '') and '\u2028' not in (n.text or '')
                   for n in root.iter(W + 't'))
        verify_headers(parts, edited)
        records.append({'file': name, 'sha256': digest(data), 'bodyParagraphs': 1,
                        'cellParagraphs': 1, 'bodyAndCellLineBreaks': 4 if edited else 2,
                        'headerLineBreaks': int(edited)})
        if edited:
            outputs.append(data)
    assert all(data == outputs[0] for data in outputs)
    return {'passed': True, 'checkpoints': [
        {'file': f'{i:02d}-{name}.json', 'revision': revisions[i - 1], 'sha256': digest(raw[i])}
        for i, name in enumerate(names, 1)],
        'byteIdenticalCheckpointGroups': byte_groups,
        'exactHistoryGroupsExcludingRevision': history_groups,
        'unrelatedWorkspaceDataUnchanged': True, 'reimportedDocumentIdentical': True,
        'files': records, 'exportWorkingSaveAndRestartExportByteIdentical': True}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(main(parser.parse_args().folder), indent=2))
