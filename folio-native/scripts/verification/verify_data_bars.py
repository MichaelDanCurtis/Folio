#!/usr/bin/env python3
"""Read actual exports with openpyxl and namespace-aware XML. No Excel rendering oracle."""
import json, tempfile, warnings
from pathlib import Path
from zipfile import ZipFile
from xml.etree import ElementTree as ET
import openpyxl
NS={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main','x14':'http://schemas.microsoft.com/office/spreadsheetml/2009/9/main','xm':'http://schemas.microsoft.com/office/excel/2006/main'}
ROOT=Path(tempfile.gettempdir())/'folio-conditional-tests'
UI=Path(tempfile.gettempdir())/'folio-data-bars-ui-tests'
checked=[]
def read(path, count):
    with warnings.catch_warnings():
        warnings.filterwarnings('ignore',message='Conditional Formatting extension is not supported.*')
        book=openpyxl.load_workbook(path)
    with ZipFile(path) as z:
        name=next(n for n in z.namelist() if n.startswith('xl/worksheets/') and n.endswith('.xml') and '/_rels/' not in n)
        root=ET.fromstring(z.read(name))
    ordinary=[(g.attrib['sqref'],r) for g in root.findall('s:conditionalFormatting',NS) for r in g.findall('s:cfRule',NS) if r.attrib['type']=='dataBar']
    extended=[(g.find('xm:sqref',NS).text,r) for g in root.findall('.//x14:conditionalFormatting',NS) for r in g.findall('x14:cfRule',NS)]
    assert len(ordinary)==count
    links={}
    for sqref,r in ordinary:
        b=r.find('s:dataBar',NS);assert len(b.findall('s:cfvo',NS))==2
        minimum,maximum=int(b.get('minLength','10')),int(b.get('maxLength','90'));assert 0<=minimum<=maximum<=100
        link=r.find('s:extLst/s:ext/x14:id',NS)
        if link is not None:
            assert link.text not in links;links[link.text]=(sqref,b)
    assert len(extended)==len(links)
    for sqref,r in extended:
        base_ref,base_bar=links[r.attrib['id']];assert sqref==base_ref
        b=r.find('x14:dataBar',NS);points=b.findall('x14:cfvo',NS);assert len(points)==2
        for p in points:
            if p.attrib['type'] in {'min','max','autoMin','autoMax'}: assert p.find('xm:f',NS) is None
            else: assert p.find('xm:f',NS).text is not None
        lo,hi=int(b.get('minLength','10')),int(b.get('maxLength','90'));assert 0<=lo<=hi<=100
        if (lo,hi)==(0,100): assert (int(base_bar.get('minLength')),int(base_bar.get('maxLength')))==(10,90)
        assert b.find('x14:fillColor',NS) is None
        assert (b.find('x14:borderColor',NS) is not None)==(b.get('border','0') in {'1','true'})
        assert (b.find('x14:negativeFillColor',NS) is not None)==(b.get('negativeBarColorSameAsPositive','0') not in {'1','true'})
        assert (b.find('x14:axisColor',NS) is not None)==(b.get('axisPosition','automatic')!='none')
    checked.append(str(path));return book.worksheets[0],ordinary,extended
s,rs,x=read(ROOT/'data-bars.xlsx',1);assert rs[0][0]=='A1:A2 C1:C2';assert s['A1'].value==-20 and s['A2'].value==80
s,rs,x=read(ROOT/'data-bars-shifted.xlsx',1);assert rs[0][0]=='A2:A3 C2:C3';assert [p.find('xm:f',NS).text for p in x[0][1].findall('x14:dataBar/x14:cfvo',NS)]==['$B$2','$B$3'];assert s['A2'].value==-20
s,rs,x=read(ROOT/'data-bars-edited.xlsx',1);assert x[0][1].find('x14:dataBar',NS).get('showValue')=='0';assert x[0][1].find('x14:dataBar',NS).get('direction')=='rightToLeft'
s,rs,x=read(ROOT/'data-bars-deleted.xlsx',0);assert not x
s,rs,x=read(ROOT/'data-bars-xlsxwriter-edited.xlsx',6);assert s.sheet_view.rightToLeft
b=next(r for ref,r in x if ref=='B1:B4').find('x14:dataBar',NS);assert b.find('x14:negativeFillColor',NS).get('rgb')=='FF8800AA';assert b.get('showValue')=='0' and b.get('gradient')=='0'
s,rs,x=read(UI/'data-bars.xlsx',1);assert rs[0][0]=='A1:A1048576';assert s['A1'].value==-20 and s['A3'].value==40 and s['B1'].value=='=A3*2'
assert x[0][1].find('x14:dataBar',NS).get('showValue')=='0';assert rs[0][1].find('s:dataBar/s:color',NS).get('rgb')=='FF63BE7B'
print(json.dumps({'passed':True,'files':checked,'reader':'openpyxl + ElementTree','limitation':'x14 inspected directly because openpyxl does not render or retain these extensions. No Microsoft Excel comparison.'},indent=2))
