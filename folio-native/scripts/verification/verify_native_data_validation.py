"""Validate artifacts retained from the isolated native validation workflow."""
import json
from pathlib import Path
import sys
import openpyxl

folder=Path(sys.argv[1])
def saved(name): return json.loads((folder/name).read_text())
seed=saved('seed.folio')
assert saved('undone.folio')['sheets'] == seed['sheets']
assert saved('redone.folio')['sheets'] == saved('edited.folio')['sheets']
assert saved('edited.folio')['sheets'][0]['cells']['A1'] == '4'
assert saved('dropdown.folio')['sheets'][0]['cells']['B1'] == "'Blue"
assert (folder/'before-restart.folio').read_bytes() == (folder/'after-restart.folio').read_bytes()
assert (folder/'native-validation.xlsx').read_bytes() == (folder/'native-validation-reopened.xlsx').read_bytes()
assert (folder/'native-validation.xlsx').read_bytes() == (folder/'native-validation-final.xlsx').read_bytes()
assert saved('warning-override.folio')['sheets'][0]['cells']['B1'] == 'Purple'
assert saved('before-restart.folio')['sheets'] == saved('dropdown.folio')['sheets']
assert any(r['formula1'] == '"Small,Medium,Large,XL"' for r in saved('rule-edited.folio')['sheets'][0]['dataValidations'])
for name in ['native-validation.xlsx','native-validation-reopened.xlsx','native-validation-final.xlsx']:
    w=openpyxl.load_workbook(folder/name);s=w.worksheets[0]
    assert s['A1'].value == 4 and s['B1'].value == 'Blue'
    rules=s.data_validations.dataValidation
    assert len(rules) == 3
    a=next(r for r in rules if 'A1' in r);b=next(r for r in rules if 'B1' in r);d=next(r for r in rules if 'D1' in r)
    assert a.type=='whole' and a.formula1=='1' and a.formula2=='5' and a.errorStyle=='stop'
    assert a.promptTitle=='Quantity' and a.showInputMessage
    assert b.type=='list' and b.errorStyle=='warning'
    assert d.formula1=='"Small,Medium,Large"' and not d.showDropDown
print('Native validation artifacts passed: entry/history/dropdown, three retained rules, values, hints, and byte-identical recovery and exports after restart. No Excel differential execution.')
