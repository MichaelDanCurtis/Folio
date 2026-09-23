#!/usr/bin/env python3
"""Independent conditional-rule copy fixtures and output checks."""
from pathlib import Path
from zipfile import ZipFile
import argparse,json,warnings,xml.etree.ElementTree as ET
import openpyxl
from openpyxl.formatting.rule import Rule,ColorScale,IconSet,DataBar,FormatObject
from openpyxl.styles import Color,Font,PatternFill
from openpyxl.styles.differential import DifferentialStyle
S='http://schemas.openxmlformats.org/spreadsheetml/2006/main'
X='http://schemas.microsoft.com/office/spreadsheetml/2009/9/main'
TYPES=['cellIs','expression','containsText','notContainsText','beginsWith','endsWith','containsBlanks','notContainsBlanks','containsErrors','notContainsErrors','duplicateValues','uniqueValues','top10','aboveAverage','colorScale','dataBar','iconSet','timePeriod']
def fixture(path):
    wb=openpyxl.Workbook();ws=wb.active;ws.title='Rule source'
    style=DifferentialStyle(font=Font(bold=True),fill=PatternFill('solid',fgColor='FFEEDDAA'))
    for i,kind in enumerate(TYPES,1):
        col=openpyxl.utils.get_column_letter(i)
        for row,value in enumerate([0,25,75,100],1):ws.cell(row,i,value)
        rule=Rule(type=kind,dxf=style)
        if kind=='cellIs':rule.operator='greaterThan';rule.formula=['20']
        if kind=='expression':rule.formula=[f'{col}1>20']
        if kind in ['containsText','notContainsText','beginsWith','endsWith']:
            rule.text='yes';rule.operator='notContains' if kind=='notContainsText' else kind
            rule.formula=[f'ISNUMBER(SEARCH("yes",{col}1))']
        if kind=='top10':rule.rank=2
        if kind=='aboveAverage':rule.aboveAverage=True
        if kind in ['dataBar','colorScale','iconSet']:rule.dxf=None
        if kind=='colorScale':rule.colorScale=ColorScale(cfvo=[FormatObject(type='min'),FormatObject(type='max')],color=[Color('FFFF0000'),Color('FF00FF00')])
        if kind=='dataBar':rule.dataBar=DataBar(cfvo=[FormatObject(type='min'),FormatObject(type='max')],color=Color('FF3388CC'))
        if kind=='iconSet':rule.iconSet=IconSet(iconSet='3TrafficLights1',cfvo=[FormatObject(type='percent',val=n) for n in [0,33,67]])
        if kind=='timePeriod':rule.timePeriod='today';rule.formula=[f'INT({col}1)=TODAY()']
        ws.conditional_formatting.add(f'{col}1:{col}4',rule)
    path.parent.mkdir(parents=True,exist_ok=True);wb.save(path)
    return {'fixture':str(path),'producer':'openpyxl','ruleTypes':TYPES}
def read(path):
    with warnings.catch_warnings():
        warnings.simplefilter('ignore',UserWarning)
        book=openpyxl.load_workbook(path,data_only=False)
    rows=[];ids=[];links=[]
    with ZipFile(path) as z:
        for name in z.namelist():
            if not name.startswith('xl/worksheets/') or not name.endswith('.xml'):continue
            root=ET.fromstring(z.read(name))
            for ns in [S,X]:
                for group in root.findall(f'.//{{{ns}}}conditionalFormatting'):
                    for r in group.findall(f'{{{ns}}}cfRule'):
                        if r.get('id'):ids.append(r.get('id').upper())
                        for link in r.findall(f'.//{{{X}}}id'):links.append(link.text.upper())
                        rows.append((name,group,r))
    assert len(ids)==len(set(ids)), 'Extension ids must be unique across worksheets'
    assert all(link in ids for link in links),'Every fallback link must resolve'
    return book,rows

def verify(core,ui):
    checked=[]
    book,rows=read(core/'independent-copied.xlsx')
    ordinary=[(g.get('sqref'),r) for _,g,r in rows if r.tag==f'{{{S}}}cfRule']
    assert len(ordinary)==36
    assert sorted(int(r.get('priority')) for _,r in ordinary)==list(range(1,37))
    for i,kind in enumerate(TYPES,1):
        col=openpyxl.utils.get_column_letter(i)
        r=next(r for area,r in ordinary if area==f'{col}10:{col}13')
        assert r.get('type')==kind
        if kind=='expression':assert r.findtext(f'{{{S}}}formula')==f'{col}10>20'
        assert [book.active.cell(row,i).value for row in range(10,14)]==[0,25,75,100]
    checked.append('independent-copied.xlsx')
    for name in ['replace.xlsx','mode-all.xlsx','mode-formats.xlsx','mode-allMergeConditional.xlsx','mode-values.xlsx','mode-formulas.xlsx','mapped.xlsx','graphics.xlsx','graphics-recovered.xlsx','graphics-cross-sheet.xlsx','fill-down.xlsx','fill-up.xlsx','fill-left.xlsx','fill-right.xlsx','opaque-unrelated.xlsx','threshold-formulas.xlsx']:
        book,rows=read(core/name)
        if name=='replace.xlsx':
            r=next(r for _,g,r in rows if g.get('sqref')=='D3:D4')
            assert r.findtext(f'{{{S}}}formula')=='D3>$Z$1'
            assert book.active['D3'].value==25 and book.active['D4'].value==75
        if name=='mode-formats.xlsx':assert book.active['D1'].value==5
        if name=='mode-allMergeConditional.xlsx':assert any(g.get('sqref')=='D1:D4' for _,g,_ in rows)
        if name=='opaque-unrelated.xlsx':assert any(r.get('future')=='keep' for _,_,r in rows)
        if name=='threshold-formulas.xlsx':
            r=next(r for _,g,r in rows if g.get('sqref')=='D1:D2' and r.get('type')=='colorScale')
            assert [x.get('val') for x in r.findall(f'{{{S}}}colorScale/{{{S}}}cfvo')]==['D1','$Z$2']
            assert book.active['Z2'].value==100
        checked.append(name)
    if ui:
        book,rows=read(ui/'conditional-paste.xlsx');assert book.active['D1'].value==25
        assert any(g.get('sqref')=='D1:D4' for _,g,_ in rows)
        checked.append('conditional-paste.xlsx')
    return {'passed':True,'files':checked,'readers':['openpyxl','ElementTree'],'nativeExcelVerified':False}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('core',type=Path,nargs='?');p.add_argument('--ui',type=Path);p.add_argument('--fixture',type=Path);a=p.parse_args()
    print(json.dumps(fixture(a.fixture) if a.fixture else verify(a.core,a.ui),indent=2))
