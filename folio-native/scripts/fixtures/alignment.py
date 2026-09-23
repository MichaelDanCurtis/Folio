"""Independent alignment/inheritance fixture with retained Office metadata."""
from pathlib import Path
from copy import deepcopy
from zipfile import ZipFile,ZIP_DEFLATED
from lxml import etree

folder=Path(__file__).resolve().parents[2]/'test/fixtures'
S='http://schemas.openxmlformats.org/spreadsheetml/2006/main'
ns={'s':S}
with ZipFile(folder/'styles.xlsx') as f: parts={n:f.read(n) for n in f.namelist()}
sheet=etree.fromstring(parts['xl/worksheets/sheet1.xml'])
styles=etree.fromstring(parts['xl/styles.xml'])
xfs=styles.find('s:cellXfs',ns); parents=styles.find('s:cellStyleXfs',ns)
base=int(sheet.find('.//s:c[@r="B2"]',ns).get('s'))
def add_xf(xf):
    xfs.append(xf);xfs.set('count',str(len(xfs)));return str(len(xfs)-1)
def align(xf,**attrs):
    old=xf.find('s:alignment',ns)
    if old is not None: xf.remove(old)
    node=etree.Element('{'+S+'}alignment',**attrs)
    xf.insert(0,node);return node
xf=deepcopy(xfs[base])
fonts=styles.find('s:fonts',ns)
font=deepcopy(fonts[int(xf.get('fontId'))]);font.find('s:name',ns).set('val','Georgia')
for name in ['scheme','family','charset']:
    old=font.find('s:'+name,ns)
    if old is not None:font.remove(old)
fonts.append(font);fonts.set('count',str(len(fonts)));xf.set('fontId',str(len(fonts)-1))
a=align(xf,horizontal='right',vertical='top',indent='2',textRotation='135',wrapText='1',shrinkToFit='0',readingOrder='2',justifyLastLine='1')
a.set('{urn:folio:alignment}horizontal','keep-qualified');a.set('relativeIndent','-2')
xf.set('applyAlignment','1');direct=add_xf(xf)
parent=deepcopy(parents[0]);align(parent,horizontal='left',vertical='center',indent='1',textRotation='90')
parents.append(parent);parents.set('count',str(len(parents)))
xf=deepcopy(xfs[base]);xf.set('xfId',str(len(parents)-1));xf.set('applyAlignment','0')
align(xf,horizontal='right',vertical='bottom',textRotation='15').set('{urn:folio:alignment}source','keep-ignored')
inherited=add_xf(xf)
xf=deepcopy(xfs[0]);xf.set('xfId','0');xf.set('applyAlignment','0')
align(xf,horizontal='right',vertical='top');ignored=add_xf(xf)
row=etree.SubElement(sheet.find('s:sheetData',ns),'{'+S+'}row',r='30',ht='90',customHeight='1',s=inherited,customFormat='1')
for address,style,text in [('A30',direct,'A longer heading\nSecond line'),('B30','0','Short'),('C30',inherited,'Inherited'),('D30',ignored,'Defaults')]:
    c=etree.SubElement(row,'{'+S+'}c',r=address,s=style,t='inlineStr')
    etree.SubElement(etree.SubElement(c,'{'+S+'}is'),'{'+S+'}t').text=text
etree.SubElement(sheet.find('s:cols',ns),'{'+S+'}col',min='20',max='20',style=direct,width='25',customWidth='1')
sheet.find('s:dimension',ns).set('ref','A1:T30')
parts['xl/worksheets/sheet1.xml']=etree.tostring(sheet)
parts['xl/styles.xml']=etree.tostring(styles)
with ZipFile(folder/'alignment.xlsx','w',ZIP_DEFLATED) as f:
    for name,data in parts.items():f.writestr(name,data)
print('Created alignment.xlsx')
