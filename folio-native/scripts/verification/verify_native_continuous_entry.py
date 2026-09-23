"""Independently check the saved native multi-cell entry and history trail."""
from pathlib import Path
import copy
import json
import sys

project = Path(__file__).resolve().parents[2]
folder = Path(sys.argv[1]) if len(sys.argv) > 1 else project.parents[1] / 'work/native-continuous-entry-shortcut'
seed = json.loads((folder / 'seed.folio').read_text())
assert seed['revision'] == 13
assert seed['sheets'][0]['cells']['A3'] == '40'
cases = [
    ('after-rapid-quit', 15, {}),
    ('after-restart', 15, {}),
    ('entered', 18, {'A3': '60', 'B3': '70', 'C3': '80'}),
    ('undo-1', 19, {'A3': '60', 'B3': '70'}),
    ('undo-2', 20, {'A3': '60'}),
    ('undo-3', 21, {}),
    ('redo-1', 22, {'A3': '60'}),
    ('redo-2', 23, {'A3': '60', 'B3': '70'}),
    ('redo-3', 24, {'A3': '60', 'B3': '70', 'C3': '80'}),
    ('after-second-rapid-quit', 26, {'A3': '60', 'B3': '70', 'C3': '80', 'A6': '87', 'B6': '88'}),
]
for name, revision, cells in cases:
    actual = json.loads((folder / f'{name}.folio').read_text())
    expected = copy.deepcopy(seed)
    expected['revision'] = revision
    expected['sheets'][0]['cells'].update({'A5': '42', 'B5': '43', **cells})
    # Entering the first populated cell in a row establishes automatic sizing.
    for row in ['5', *(['6'] if name == 'after-second-rapid-quit' else [])]:
        expected['sheets'][0]['rowHeightModes'][row] = 'auto'
        expected['sheets'][0]['rowHeights'][row] = 24.0
    assert actual == expected, name
assert (folder / 'after-rapid-quit.folio').read_bytes() == (folder / 'after-restart.folio').read_bytes()
print('PASS: ten native snapshots contain exactly the expected cell/revision changes; both immediate quit runs flushed both entries and restart is byte-identical.')
