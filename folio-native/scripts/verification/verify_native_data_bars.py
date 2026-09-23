#!/usr/bin/env python3
"""Checks the persisted results of the separately observed native workflow."""
import hashlib,json,sys
from pathlib import Path
from zipfile import ZipFile
from xml.etree import ElementTree as ET
p=Path(sys.argv[1]);ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main','x14':'http://schemas.microsoft.com/office/spreadsheetml/2009/9/main','xm':'http://schemas.microsoft.com/office/excel/2006/main'}
def w(name):return json.loads((p/name).read_text())
seed=w('seed.folio');edited=w('edited.folio');undone=w('undone.folio');rule=w('rule-edited.folio');rule_undo=w('rule-undone.folio');rule_redo=w('rule-redone.folio')
assert seed['sheets'][0]['cells']['A3']=='40'
assert edited['sheets'][0]['cells']['A3']=='60'
assert undone['sheets']==seed['sheets']
b=rule['sheets'][0]['conditionalFormats'][0]['dataBar'];assert b['showValue'] and b['axisPosition']=='middle'
hidden=w('rule-hidden.folio');assert not hidden['sheets'][0]['conditionalFormats'][0]['dataBar']['showValue']
assert rule_undo['sheets']==rule['sheets'];assert rule_redo['sheets']==hidden['sheets']
assert (p/'before-restart.folio').read_bytes()==(p/'after-restart.folio').read_bytes()
assert (p/'native-data-bars.xlsx').read_bytes()==(p/'native-data-bars-reopened.xlsx').read_bytes()
with ZipFile(p/'native-data-bars.xlsx') as z:
 name=next(n for n in z.namelist() if n.startswith('xl/worksheets/') and n.endswith('.xml') and '/_rels/' not in n)
 root=ET.fromstring(z.read(name));base=root.find('s:conditionalFormatting/s:cfRule',ns);ext=root.find('.//x14:conditionalFormatting/x14:cfRule',ns)
 assert base.find('s:extLst/s:ext/x14:id',ns).text==ext.attrib['id']
 b=ext.find('x14:dataBar',ns);assert b.attrib['axisPosition']=='middle' and b.attrib['showValue']=='1'
 assert root.find('.//s:c[@r="A3"]/s:v',ns).text=='40'
 assert root.find('.//s:c[@r="B1"]/s:f',ns).text=='A3*2'
print(json.dumps({'passed':True,'directory':str(p),'files':[{'file':f.name,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()} for f in sorted(p.iterdir()) if f.suffix in {'.folio','.xlsx'}],'scope':'Saved native workflow artifacts, complementing UI observations; not a Microsoft Excel comparison.'},indent=2))
