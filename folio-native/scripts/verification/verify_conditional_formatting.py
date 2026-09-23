#!/usr/bin/env python3
"""Independent OOXML/openpyxl checks, not Microsoft Excel rendering evidence."""
import json, tempfile
from pathlib import Path
from zipfile import ZipFile
from xml.etree import ElementTree as ET
import openpyxl
ROOT=Path(tempfile.gettempdir())/'folio-conditional-tests'
UI=Path(tempfile.gettempdir())/'folio-conditional-ui-tests'
NS={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
checked=[]
def read(path):
    book=openpyxl.load_workbook(path)
    with ZipFile(path) as z:
        styles=ET.fromstring(z.read('xl/styles.xml'))
        dxfs=styles.find('s:dxfs',NS)
        if dxfs is not None: assert int(dxfs.attrib['count'])==len(dxfs)
        for name in z.namelist():
            if name.startswith('xl/worksheets/') and name.endswith('.xml'):
                root=ET.fromstring(z.read(name)); order=[n.tag.split('}')[-1] for n in root]
                for cf in root.findall('s:conditionalFormatting',NS):
                    for rule in cf.findall('s:cfRule',NS):
                        assert int(rule.attrib['priority'])>0
                        if 'dxfId' in rule.attrib: assert 0<=int(rule.attrib['dxfId'])<len(dxfs)
                    for later in ['dataValidations','hyperlinks','pageMargins','tableParts','extLst']:
                        if later in order: assert order.index('conditionalFormatting')<order.index(later)
    checked.append(str(path));return book.worksheets[0]
def rules(sheet):
    return [(str(cf.sqref),r) for cf in sheet.conditional_formatting for r in sheet.conditional_formatting[cf]]
s=read(ROOT/'rules.xlsx');r=rules(s)[0][1]
assert r.type=='expression' and r.formula==['A1>10'] and r.stopIfTrue
assert r.dxf.font.b is False
assert r.dxf.font.color.rgb=='FF9C0006'
assert r.dxf.fill.fgColor.rgb=='FFFFC7CE'
assert r.dxf.numFmt.formatCode=='0.00%'
assert s['A2'].value==20 and s['A2'].fill.patternType is None
s=read(ROOT/'rules-shifted.xlsx');sqref,r=rules(s)[0]
assert sqref=='A2:A3 C2:C3' and r.formula==['A2>10']
s=read(UI/'conditional.xlsx');rs=rules(s)
assert len(rs)==2
blank=next(r for _,r in rs if r.type=='containsBlanks')
assert blank.formula==['LEN(TRIM(D1))=0']
assert blank.dxf.fill.fgColor.rgb=='FFEEEEEE'
value=next(r for _,r in rs if r.type=='cellIs')
assert value.operator=='greaterThan' and value.formula==['10']
assert s['A2'].value==20 and s['B2'].value=='=A2*2'
s=read(ROOT/'metadata-edited.xlsx');r=rules(s)[0][1]
assert r.dxf.font.b is False and r.dxf.fill.fgColor.rgb=='FF00FF00'
with ZipFile(ROOT/'metadata-edited.xlsx') as z:
    styles=ET.fromstring(z.read('xl/styles.xml'))
    dxf=styles.find('s:dxfs',NS)[r.dxfId]
    assert dxf.attrib['{urn:folio:future}keep']=='root'
    assert dxf.find('s:font/s:b',NS).attrib['{urn:folio:future}keep']=='font'
    assert dxf.find('s:fill/s:patternFill/s:fgColor',NS).attrib['{urn:folio:future}keep']=='fill'
s=read(ROOT/'metadata-shifted.xlsx');assert rules(s)[0][0]=='A2:A3'
print(json.dumps({'passed':True,'files':checked,'reader':'openpyxl + XML','limitation':'No live Microsoft Excel rendering or behavior comparison.'},indent=2))
