"""Verify native row-height history, read-only preview and recovery checkpoints."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
from verify_table_row_heights import verify as verify_exports


def verify(folder):
    names = [
        '01-imported', '02-auto-previewed', '03-cancelled', '04-exact',
        '05-undone', '06-redone', '07-exact-previewed', '08-exported',
        '09-saved', '10-postsave-undone', '11-postsave-redone', '12-minimum',
        '13-minimum-previewed', '14-minimum-undone', '15-before-restart',
        '16-restarted', '17-recovered-export',
    ]
    raw = [(folder/(name+'.json')).read_bytes() for name in names]
    states = [json.loads(value) for value in raw]
    assert [s['revision'] for s in states] == [1, 1, 1, 2, 3, 4, 4, 4, 4, 5, 6, 7, 7, 8, 8, 8, 8]
    original = states[0]['document']
    exact = copy.deepcopy(original)
    exact['content'][0]['content'][0]['attrs']['height'] = {'rule': 'exact', 'value': 480}
    minimum = copy.deepcopy(exact)
    minimum['content'][0]['content'][0]['attrs']['height']['rule'] = 'atLeast'
    automatic_indices = {0, 1, 2, 4, 9}
    for i, state in enumerate(states):
        expected = original if i in automatic_indices else minimum if i in {11, 12} else exact
        assert state['document'] == expected, names[i]
        assert {k: v for k, v in state.items() if k not in {'document', 'revision'}} == {
            k: v for k, v in states[0].items() if k not in {'document', 'revision'}
        }, names[i]
    for group in [(0, 1, 2), (5, 6, 7, 8), (11, 12), (13, 14, 15, 16)]:
        assert all(raw[i] == raw[group[0]] for i in group), group
    assert (folder/'row-height.docx').read_bytes() == (folder/'exact.docx').read_bytes()
    return {
        'passed': True, 'snapshots': len(states),
        'cancelByteIdentical': True, 'previewByteIdenticalInAllThreeModes': True,
        'ordinaryAndPostSaveHistoryExact': True,
        'minimumApplyAndUndoExact': True,
        'allUnrelatedWorkspaceStateUnchanged': True,
        'workingSaveMatchesExport': True, 'restartByteIdentical': True,
        'restartSha256': hashlib.sha256(raw[15]).hexdigest(),
        'exports': verify_exports(folder),
        'checkpoints': {n: hashlib.sha256(b).hexdigest() for n, b in zip(names, raw)},
        'microsoftWordExecuted': False,
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
