"""Independent worksheet with a localized Normal style distinct from cell XF 0."""
from copy import deepcopy
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from openpyxl import load_workbook
from openpyxl.styles import Font, Alignment
from lxml import etree as E

folder=Path(__file__).resolve().parents[2]/'test/fixtures'
w=load_workbook(folder/'autofit.xlsx',rich_text=True);s=w.active
s['B2']='Review the field notes together.'
s['C200']='Offscreen reviewer'
s['C200'].font=Font(name='Georgia',size=11)
s['B7']='A hidden record that must not widen this column. '*100
s.row_dimensions[7].hidden=True
s.column_dimensions['D'].hidden=True;s.column_dimensions['D'].width=17
s['D1']='Hidden column'
s['A10']=12345.67;s['B10']='=A10*2';s['C10']=True
for a in ['A10','B10','C10']:
    s[a].font=Font(name='Georgia',size=11)
    s[a].number_format='$#,##0.00' if a!='C10' else 'General'
s['A11']='Vertical';s['A11'].alignment=Alignment(textRotation=90)
s['A11'].font=Font(name='Georgia',size=11)
path=folder/'column-autofit.xlsx';w.save(path)
with ZipFile(path) as z:parts={n:z.read(n) for n in z.namelist()}
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'};S='{'+ns['s']+'}'
root=E.fromstring(parts['xl/styles.xml']);fonts=root.find('s:fonts',ns)
font=E.SubElement(fonts,S+'font');E.SubElement(font,S+'sz',val='14');E.SubElement(font,S+'name',val='Georgia')
fonts.set('count',str(len(fonts)))
bases=root.find('s:cellStyleXfs',ns);xf=deepcopy(bases[0]);xf.set('fontId',str(len(fonts)-1));xf.set('applyFont','1')
bases.append(xf);bases.set('count',str(len(bases)))
normal=root.find('s:cellStyles/s:cellStyle[@builtinId="0"]',ns);normal.set('name','Standard');normal.set('xfId',str(len(bases)-1))
normal.set('{urn:folio:normal}xfId','999')
parts['xl/styles.xml']=E.tostring(root)
with ZipFile(path,'w',ZIP_DEFLATED) as z:
    for name,data in parts.items():z.writestr(name,data)
print('Created column-autofit.xlsx')
