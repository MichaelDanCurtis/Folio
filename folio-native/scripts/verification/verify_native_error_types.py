"""Inspect retained files from the isolated, manually exercised macOS package."""
import json
from pathlib import Path
import sys
from openpyxl import load_workbook

folder = Path(sys.argv[1])
def saved(name): return json.loads((folder / name).read_text())
seed = saved('seed.folio')['sheets']
# Loading an imported package adds these empty source projections. They predate
# the paste; they are not edits made by history.
seed[0].update(tables=[], hiddenRows=[], frozenPane=None, autoFilter=None)
assert saved('undone.folio')['sheets'] == seed
assert saved('pasted.folio')['sheets'] == saved('redone.folio')['sheets']
sheet = saved('pasted.folio')['sheets'][0]
assert sheet['cells']['C2'] == '#N/A' and sheet['cells']['C3'] == "'#N/A"
assert saved('text-edited.folio')['sheets'][0]['cells']['B2'] == '="#N/A"'
assert (folder / 'before-restart.folio').read_bytes() == (folder / 'after-restart.folio').read_bytes()
assert (folder / 'native-errors.xlsx').read_bytes() == (folder / 'native-errors-reopened.xlsx').read_bytes()
formulas = load_workbook(folder / 'native-errors.xlsx').active
values = load_workbook(folder / 'native-errors.xlsx', data_only=True).active
assert formulas['B2'].value == '=NA()' and formulas['B3'].value == '="#N/A"'
assert formulas['C2'].data_type == 'e' and formulas['C3'].data_type == 's'
for address in ['B2', 'C2']:
    assert (values[address].data_type, values[address].value) == ('e', '#N/A')
for address in ['B3', 'C3']:
    assert (values[address].data_type, values[address].value) == ('s', '#N/A')
    assert values[address].number_format == '0;0;0;"TEXT: "@'
assert values['D2'].value is True and values['D3'].value is False
print('Native error-type artifacts passed: Paste Values, edit/history, error/text types, formulas/formats, exact saved recovery and identical exports after a verified process restart.')
