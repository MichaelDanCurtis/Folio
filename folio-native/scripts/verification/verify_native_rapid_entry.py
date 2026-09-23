"""Check saved native typing fixtures without using the Folio engine."""
from pathlib import Path
import copy
import json
import sys

project = Path(__file__).resolve().parents[2]
folder = Path(sys.argv[1]) if len(sys.argv) > 1 else project.parents[1] / 'work/native-rapid-entry'
seed = json.loads((folder / 'seed.folio').read_text())
assert seed['revision'] == 13
assert seed['sheets'][0]['cells']['A3'] == '40'
assert seed['sheets'][0]['cells']['B1'] == '=A3*2'
cases = [
    ('entered', 14, {'A3': '60'}),
    ('undone', 15, {}),
    ('redone', 16, {'A3': '60'}),
    ('tab', 17, {'A3': '60', 'B3': '12345'}),
    ('shift-tab', 18, {'A3': '60', 'B3': '12345', 'C3': '24'}),
    ('cancelled', 18, {'A3': '60', 'B3': '12345', 'C3': '24'}),
    ('before-restart', 18, {'A3': '60', 'B3': '12345', 'C3': '24'}),
    ('after-restart', 18, {'A3': '60', 'B3': '12345', 'C3': '24'}),
]
for name, revision, cells in cases:
    actual = json.loads((folder / f'{name}.folio').read_text())
    expected = copy.deepcopy(seed)
    expected['revision'] = revision
    expected['sheets'][0]['cells'].update(cells)
    assert actual == expected, name
assert (folder / 'cancelled.folio').read_bytes() == (folder / 'shift-tab.folio').read_bytes()
assert (folder / 'before-restart.folio').read_bytes() == (folder / 'after-restart.folio').read_bytes()
print('PASS: eight native snapshots contain only the expected cell/revision changes; cancellation and restart are byte-identical.')
