#!/usr/bin/env python3
"""Read files from an observed isolated desktop workflow; do not drive the UI."""
import json, sys
from pathlib import Path
from zipfile import ZipFile
from xml.etree import ElementTree as ET
import openpyxl
root=Path(sys.argv[1]); ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
for name in ['seed','edited','undone','rule-edited','rule-undone','before-restart','after-restart']:
    w=json.loads((root/f'{name}.folio').read_text());s=w['sheets'][0]
    assert len(s['conditionalFormats'])==2
    if name=='edited': assert s['cells']['A2']=='6'
    else: assert s['cells']['A2']=='20'
    rule=next(r for r in s['conditionalFormats'] if r['type']=='cellIs')
    assert rule['formulas']==(['25'] if name=='rule-edited' else ['10'])
    blank=next(r for r in s['conditionalFormats'] if r['type']=='containsBlanks')
    assert blank['sqref']=='D1:D1048576'
assert (root/'before-restart.folio').read_bytes()==(root/'after-restart.folio').read_bytes()
a=root/'native-conditional.xlsx';b=root/'native-conditional-reopened.xlsx';assert a.read_bytes()==b.read_bytes()
book=openpyxl.load_workbook(a);sheet=book.worksheets[0];assert sheet['A2'].value==20 and sheet['B2'].value=='=A2*2'
rules=[r for cf in sheet.conditional_formatting for r in sheet.conditional_formatting[cf]]
assert len(rules)==2
value=next(r for r in rules if r.type=='cellIs');assert value.formula==['10'] and value.operator=='greaterThan'
assert value.dxf.fill.fgColor.rgb=='FFFFC7CE' and sheet['A2'].fill.patternType is None
blank=next(r for r in rules if r.type=='containsBlanks');assert blank.formula==['LEN(TRIM(D1))=0']
print(json.dumps({'passed':True,'nativeFiles':9,'restartedWorkspaceIdentical':True,'restartedExportIdentical':True,'reader':'openpyxl + saved-workspace assertions'},indent=2))
