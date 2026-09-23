"""Verify retained artifacts from the isolated macOS date-edit/restart workflow."""
import json
from pathlib import Path
import sys
from verify_early_dates import verify

folder = Path(sys.argv[1])
def saved(name): return json.loads((folder / name).read_text())
assert saved('undone.folio')['sheets'] == saved('seed.folio')['sheets']
assert saved('redone.folio')['sheets'] == saved('edited.folio')['sheets']
assert saved('seed.folio')['sheets'][0]['cells']['A62'] == '60'
assert saved('edited.folio')['sheets'][0]['cells']['A62'] == '1900-02-28'
assert (folder / 'before-restart.folio').read_bytes() == (folder / 'after-restart.folio').read_bytes()
assert (folder / 'native-early.xlsx').read_bytes() == (folder / 'native-early-reopened.xlsx').read_bytes()
for name in ['native-early.xlsx', 'native-early-reopened.xlsx']:
    verify(folder / name, edited=True, typed=True)
print('Native early dates passed: date entry and dependent results, exact worksheet Undo/Redo, preserved raw serials/formulas/text, byte-identical recovery and exports after a verified process restart.')
