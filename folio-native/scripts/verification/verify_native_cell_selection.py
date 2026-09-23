"""Compare the disposable native selection fixture before/after UI checks.

UI selection geometry and keyboard observations are recorded separately. These
checks establish that selection/cancellation/export/restart did not edit data.
"""
import argparse
import hashlib
import json
from pathlib import Path


def verify(folder):
    names = [
        '01-imported.json', '02-table-cancelled.json',
        '03-sheet-cancelled.json', '04-exported.json', '05-restarted.json',
    ]
    raw = (folder / names[0]).read_bytes()
    original = json.loads(raw)
    assert original['revision'] == 1
    for name in names:
        assert (folder / name).read_bytes() == raw, name
    source = (folder / 'source.docx').read_bytes()
    assert source == (folder / 'selection-checked.docx').read_bytes()
    return {
        'passed': True,
        'checkpointCount': len(names),
        'selectionAndCancelledDraftsLeaveEntireWorkspaceUnchanged': True,
        'restartByteIdentical': True,
        'unchangedRecoverySha256': hashlib.sha256(raw).hexdigest(),
        'sourceAndExportByteIdentical': True,
        'docxSha256': hashlib.sha256(source).hexdigest(),
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
