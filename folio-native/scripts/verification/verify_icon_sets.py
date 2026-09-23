#!/usr/bin/env python3
"""Read exported XLSX independently; never resave extension-bearing workbooks."""
import argparse, json, warnings
from pathlib import Path
from zipfile import ZipFile
import xml.etree.ElementTree as ET
import openpyxl

S='http://schemas.openxmlformats.org/spreadsheetml/2006/main'
X='http://schemas.microsoft.com/office/spreadsheetml/2009/9/main'
M='http://schemas.microsoft.com/office/excel/2006/main'
CATALOG=['3Arrows','3ArrowsGray','3Flags','3TrafficLights1','3TrafficLights2','3Signs','3Symbols','3Symbols2','4Arrows','4ArrowsGray','4RedToBlack','4Rating','4TrafficLights','5Arrows','5ArrowsGray','5Rating','5Quarters','3Stars','3Triangles','5Boxes']
def read(path):
    with warnings.catch_warnings():
        warnings.simplefilter('ignore', UserWarning)
        book=openpyxl.load_workbook(path, data_only=False)
    with ZipFile(path) as z:
        sheets=[ET.fromstring(z.read(n)) for n in z.namelist() if n.startswith('xl/worksheets/') and n.endswith('.xml')]
    rules=[]
    for sheet in sheets:
        for ns in [S,X]:
            for group in sheet.iter(f'{{{ns}}}conditionalFormatting'):
                for rule in group.findall(f'{{{ns}}}cfRule'):
                    if rule.get('type')!='iconSet':continue
                    icon=rule.find(f'{{{ns}}}iconSet');assert icon is not None
                    thresholds=[]
                    for p in icon.findall(f'{{{ns}}}cfvo'):
                        thresholds.append({'type':p.get('type'),'value':p.get('val') if ns==S else p.findtext(f'{{{M}}}f'), 'gte':p.get('gte','1')})
                    custom=[(p.get('iconSet'),int(p.get('iconId'))) for p in icon.findall(f'{{{X}}}cfIcon')]
                    name=icon.get('iconSet','3TrafficLights1')
                    assert name in CATALOG or name=='NoIcons'
                    if icon.get('custom')=='1':
                        assert len(custom)==len(thresholds) and 3<=len(custom)<=5
                    else:
                        assert not custom and len(thresholds)==int(name[0])
                    rules.append({'set':name,'namespace':ns,'range':group.get('sqref') if ns==S else group.findtext(f'{{{M}}}sqref'),'priority':int(rule.get('priority')),'id':rule.get('id'),'thresholds':thresholds,'icons':custom,'showValue':icon.get('showValue','1')})
    ids=[r['id'] for r in rules if r['namespace']==X];assert all(ids) and len(set(ids))==len(ids)
    return book,rules

def verify(core,ui):
    files=[]
    book,r=read(core/'all-icon-sets.xlsx')
    assert sorted(x['set'] for x in r)==sorted(CATALOG)
    assert sum(x['namespace']==S for x in r)==17
    assert sorted(x['priority'] for x in r)==list(range(1,21))
    # openpyxl separately recognizes all 17 standard icon sets.
    std=[rule for group in book.active.conditional_formatting for rule in book.active.conditional_formatting[group] if rule.type=='iconSet']
    assert sorted(x.iconSet.iconSet for x in std)==sorted(CATALOG[:17])
    for x in r:
        size=int(x['set'][0]);assert [t['value'] for t in x['thresholds']]==[str(int(i*100/size+0.5)) for i in range(size)]
    files.append('all-icon-sets.xlsx')
    book,r=read(core/'custom-icons.xlsx')
    custom=next(x for x in r if x['icons']);assert custom['namespace']==X
    assert custom['icons']==[('NoIcons',0),('5Quarters',2),('3Stars',2)] and custom['showValue']=='0'
    assert [book.active.cell(i,1).value for i in range(1,4)]==[0,50,100]
    files.append('custom-icons.xlsx')
    for filename,refs,formula,show in [('icon-formulas.xlsx','A1:A3 C1:C3',['$B$1','$B$2'],'1'),('icon-formulas-shifted.xlsx','A2:A4 C2:C4',['$B$2','$B$3'],'1'),('icon-formulas-edited.xlsx','A2:A4 C2:C4',['$B$2','$B$3'],'0')]:
        book,r=read(core/filename);assert len(r)==2
        for x in r:
            assert x['range']==refs and [t['value'] for t in x['thresholds'][1:]]==formula
            assert x['showValue']==(show if x['set']=='3Stars' else '1')
        files.append(filename)
    _,r=read(core/'icon-deleted.xlsx');assert r==[];files.append('icon-deleted.xlsx')
    for base in ['3TrafficLights1','NoIcons']:
        filename=f'icon-custom-{base}.xlsx'
        _,r=read(core/filename);assert len(r)==1
        assert r[0]['set']==base and len(r[0]['thresholds'])==4
        assert r[0]['icons']==[('NoIcons',0),('4Rating',1),('3Triangles',2),('5Boxes',4)]
        files.append(filename)
    book,r=read(core/'icon-openpyxl-edited.xlsx');assert len(r)==17
    for x in r:
        assert x['thresholds'][1]['gte']=='0' and x['thresholds'][1]['value']=='20'
        assert x['showValue']==('1' if x['set']=='3Arrows' else '0')
    assert book.active['A2'].value==25
    files.append('icon-openpyxl-edited.xlsx')
    if ui:
        book,r=read(ui/'icons.xlsx');assert len(r)==1
        assert r[0]['icons']==[('NoIcons',0),('5Quarters',2),('3TrafficLights1',2)]
        assert r[0]['showValue']=='0' and r[0]['range']=='A1:A1048576'
        assert [book.active.cell(i,1).value for i in range(1,6)]==[0,50,100,'text','=1/0']
        files.append('icons.xlsx')
    return {'passed':True,'files':files,'standardReader':'openpyxl','extensionReader':'ElementTree (no lossy resave)','nativeExcelVerified':False}

def fixture(path):
    from openpyxl.formatting.rule import Rule,IconSet,FormatObject
    wb=openpyxl.Workbook();ws=wb.active;ws.title='External icon sample'
    for i,name in enumerate(CATALOG[:17],1):
        for row,value in enumerate([0,25,50,75,100],1):ws.cell(row,i,value)
        size=int(name[0]);rule=Rule(type='iconSet',iconSet=IconSet(iconSet=name,showValue=False,reverse=True,cfvo=[FormatObject(type='num',val=j*20,gte=j%2==0) for j in range(size)]))
        col=openpyxl.utils.get_column_letter(i);ws.conditional_formatting.add(f'{col}1:{col}5',rule)
    wb.save(path)
    return {'fixture':str(path),'producer':'openpyxl','sets':17}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('core',type=Path,nargs='?');p.add_argument('--ui',type=Path);p.add_argument('--fixture',type=Path);a=p.parse_args()
    print(json.dumps(fixture(a.fixture) if a.fixture else verify(a.core,a.ui),indent=2))
