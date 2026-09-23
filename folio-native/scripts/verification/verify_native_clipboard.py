from pathlib import Path
import copy, hashlib, json
from openpyxl import load_workbook
root=Path(__file__).resolve().parents[2]/'release/validation/native-clipboard'
def w(name):return json.loads((root/f'{name}.folio').read_text())
for a,b in [('imported','undone'),('pasted','redone'),('formatting','after-save-undone'),('transposed','after-save-redone')]:
 for field in ['sheets','document','officeWorkbook']:
  assert w(a).get(field)==w(b).get(field),(a,b,field)
p=w('pasted')['sheets'][0]
assert p['cells']['B5']=='=A5*3' and p['cells']['D5']=="'001"
assert p['cells']['C5']=='One\nTwo\t"quoted"' and p['rowHeights']['5']==31.5
v=w('values')['sheets'][0];f=w('formatting')['sheets'][0]
assert v['cells']['A8']=='6.0' and v['cellStyles']['A8']['italic']
assert f['cells']==v['cells'] and f['cellStyles']['A8']['bold'] and f['cellStyles']['A8']['fontName']=='Georgia'
assert f['rowHeights']['8']==24.75
assert (root/'after-save-redone.folio').read_bytes()==(root/'reopened.folio').read_bytes()
assert (root/'clipboard-native.xlsx').read_bytes()==(root/'clipboard-reopened.xlsx').read_bytes()
s=load_workbook(root/'clipboard-native.xlsx')['Copy source']
for a,v in [('A5',2),('B5','=A5*3'),('C5','One\nTwo\t"quoted"'),('D5','001'),('A8',6),('F2',2),('F3',6),('F4','One\nTwo\t"quoted"'),('F5','001'),('H1','Unselected private cell')]:
 assert s[a].value==v,(a,s[a].value,v)
for a,b in [('B2','B5'),('B2','A8'),('C2','C5')]:
 for field in ['font','fill','border','alignment','protection','number_format']:
  assert copy.copy(getattr(s[a],field))==copy.copy(getattr(s[b],field)),(a,b,field)
assert load_workbook(root/'clipboard-native.xlsx',data_only=True)['Copy source']['B5'].value==6
manifest={'version':'0.33.0','scope':'Native import, keyboard copy/paste and history, values/formatting menu commands, transpose dialog, native exports and byte-identical restart/export. Direct Excel differential qualification remains pending.','files':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.iterdir()) if p.suffix in ['.folio','.xlsx']}}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('Native clipboard passed:',len(manifest['files']),'retained snapshots and exports; formulas, formats, multiline values, exact undo/redo, save and restart.')
