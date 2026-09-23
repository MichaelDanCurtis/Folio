"""Check captured native preview/restart snapshots for the balancing fixtures."""
import argparse
import hashlib
import json
from pathlib import Path
from verify_column_balancing import verify

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('folder', type=Path)
folder = parser.parse_args().folder
report = verify(folder / 'balance-source.docx', folder / 'balance-disabled.docx',
                folder / 'balanced-native.docx', folder / 'disabled-native.docx')
snapshots = ['balanced-baseline', 'balanced-after-preview', 'disabled-baseline',
             'disabled-after-preview', 'before-restart', 'after-restart']
data = [(folder / f'{name}.folio').read_bytes() for name in snapshots]
assert [json.loads(value)['revision'] for value in data] == [1, 1, 2, 2, 3, 3]
for before, after in [(0, 1), (2, 3), (4, 5)]:
    assert data[before] == data[after], (snapshots[before], snapshots[after])
report.update(previewsByteIdentical=True, restartByteIdentical=True,
    revisions=[1, 1, 2, 2, 3, 3],
    snapshotHashes={name: hashlib.sha256(value).hexdigest()
                    for name, value in zip(snapshots, data)})
print(json.dumps(report, indent=2))
