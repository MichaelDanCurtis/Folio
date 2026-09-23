"""Qualify captured native table-sort history, recovery and DOCX artifacts.

The files contain the disposable annotated fixture, not the user's workspace.
Mouse, keyboard, focus and locale-preview observations are recorded separately.
"""
import argparse
from copy import deepcopy
import hashlib
import json
from pathlib import Path

from verify_document_table_sort import verify as verify_office


def semantic_document(document):
    """Import assigns XML positions and splits equal-format runs at anchors."""
    def normalize(value):
        if isinstance(value, list):
            result = []
            for item in map(normalize, value):
                if (result and isinstance(item, dict) and item.get('type') == 'text'
                        and isinstance(result[-1], dict)
                        and result[-1].get('type') == 'text'
                        and {k: v for k, v in item.items() if k != 'text'}
                        == {k: v for k, v in result[-1].items() if k != 'text'}):
                    result[-1]['text'] += item['text']
                else:
                    result.append(item)
            return result
        if isinstance(value, dict):
            return {k: normalize(v) for k, v in value.items() if k != 'officeId'}
        return value
    result = normalize(document)
    for bookmark in result.get('attrs', {}).get('bookmarks', []):
        bookmark.pop('sourceId', None)
    return result


def verify(folder):
    names = ['imported', 'draft-preview', 'cancelled', 'sorted-draft',
             'applied', 'undone', 'redone', 'saved', 'postsave-undone',
             'before-restart', 'restarted', 'recovered-save', 'exported',
             'reimported', 'reimport-exported', 'locale-cancelled']
    revisions = [1, 1, 1, 1, 2, 3, 4, 4, 5, 6, 6, 6, 6, 7, 7, 7]
    raw = {i: (folder / f'{i:02d}-{name}.json').read_bytes()
           for i, name in enumerate(names, 1)}
    states = {i: json.loads(value) for i, value in raw.items()}
    for i, state in states.items():
        assert state['revision'] == revisions[i - 1], i
    for group in [[1, 2, 3, 4], [7, 8], [10, 11, 12, 13], [14, 15, 16]]:
        assert all(raw[i] == raw[group[0]] for i in group), group
    for group in [[1, 6, 9], [5, 7, 8, 10, 11, 12, 13]]:
        snapshots = [{k: v for k, v in states[i].items() if k != 'revision'}
                     for i in group]
        assert all(value == snapshots[0] for value in snapshots), group

    original = states[1]['document']
    expected = deepcopy(original)
    rows = expected['content'][1]['content']
    rows[1]['content'][0]['content'][0]['content'][0]['text'] = 'Zulu!?'
    expected['content'][1]['content'] = [rows[i] for i in [0, 2, 4, 3, 1]]
    for kind in ['bookmarks', 'comments']:
        assert len(expected['attrs'][kind]) == 1
        expected['attrs'][kind][0].update(start=86, end=90)
    assert states[5]['document'] == expected
    assert semantic_document(states[14]['document']) == semantic_document(expected)
    unchanged = set(states[1]) - {'revision', 'document', 'officePackages', 'officeDocument'}
    for i, state in states.items():
        for key in unchanged:
            assert state[key] == states[1][key], (i, key)

    office = verify_office(folder)
    edited = (folder / 'sorted.docx').read_bytes()
    output_names = ['sorted.docx', 'recovered.docx', 'exported.docx', 'reimported.docx']
    assert all((folder / name).read_bytes() == edited for name in output_names)
    return {'passed': True, 'checkpoints': len(states),
            'draftCancelAndLocaleDraftLeaveRecoveryUnchanged': True,
            'oneStepApplyAndOrdinaryAndPostSaveHistoryExact': True,
            'completeRowsAndAnnotationsPreserved': True,
            'restartByteIdentical': True, 'reimportSemanticsPreserved': True,
            'reimportNormalization': 'XML source positions and equal-format run boundaries only',
            'unrelatedWorkspaceStateUnchanged': True,
            'byteIdenticalNativeOutputs': output_names,
            'nativeOutputSha256': hashlib.sha256(edited).hexdigest(),
            'office': office}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
