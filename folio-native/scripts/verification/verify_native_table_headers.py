"""Verify saved checkpoints from the native repeated-header workflow."""
import argparse
import hashlib
import json
from pathlib import Path
from verify_table_headers import verify as verify_exports


def verify(folder):
    names = [
        '01-imported', '02-previewed', '03-cancelled', '04-disabled',
        '05-undone', '06-redone', '07-mixed-cancelled', '08-disabled-previewed',
        '09-saved', '10-postsave-undone', '11-postsave-redone',
        '12-before-restart', '13-restarted', '14-picker-input-undone', '15-exported',
        '16-multirow-applied', '17-multirow-undone', '18-final-saved',
    ]
    raw = [(folder / (name + '.json')).read_bytes() for name in names]
    states = [json.loads(value) for value in raw]
    assert [s['revision'] for s in states] == [1, 1, 1, 2, 3, 4, 4, 4, 4, 5, 6, 6, 6, 48, 48, 49, 50, 50]
    enabled = states[0]['document']
    disabled = json.loads(json.dumps(enabled))
    disabled['content'][0]['content'][0]['attrs']['header'] = False
    both_disabled = json.loads(json.dumps(disabled))
    both_disabled['content'][0]['content'][1]['attrs']['header'] = False
    enabled_indices = {0, 1, 2, 4, 9}
    for i, state in enumerate(states):
        expected = both_disabled if i == 15 else enabled if i in enabled_indices else disabled
        assert state['document'] == expected, names[i]
        assert {k: v for k, v in state.items() if k not in {'document', 'revision'}} == {
            k: v for k, v in states[0].items() if k not in {'document', 'revision'}
        }, names[i]
    for group in [(0, 1, 2), (5, 6, 7, 8), (10, 11, 12), (13, 14), (16, 17)]:
        assert all(raw[i] == raw[group[0]] for i in group), group
    exports = verify_exports(
        folder / 'headers-source.docx', folder / 'headers-enabled.docx',
        folder / 'headers-disabled.docx', folder / 'headers-reopened.docx',
    )
    return {
        'passed': True,
        'snapshots': len(states),
        'enabledPreviewByteIdentical': True,
        'disabledPreviewByteIdentical': True,
        'cancelByteIdentical': True,
        'mixedDraftCancelByteIdentical': True,
        'multirowApplyAndOneStepUndoExact': True,
        'ordinaryAndPostSaveHistoryExact': True,
        'restartByteIdentical': True,
        'restartSha256': hashlib.sha256(raw[12]).hexdigest(),
        'pickerInputIncident': {
            'description': 'After restart, a dismissed export picker routed a 21-character filename into the disposable test document.',
            'restoration': '21 native Undo commands restored the document, Office source and workbook exactly; only revision advanced to 48.',
            'restoredStateVerified': True,
            'postRestoreExportMatchesSavedBytes': True,
        },
        'exports': exports,
        'checkpoints': {name: hashlib.sha256(value).hexdigest() for name, value in zip(names, raw)},
        'microsoftWordExecuted': False,
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
