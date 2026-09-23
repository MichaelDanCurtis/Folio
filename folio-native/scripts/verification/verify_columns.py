"""Independent OOXML checks for the custom-left, RTL column workflow."""
import argparse
import json
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

p=argparse.ArgumentParser()
p.add_argument('source',type=Path)
p.add_argument('edited',type=Path)
p.add_argument('--body-prefix',default='Pending column edit. ')
a=p.parse_args()
W='{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
def package(path):
    with zipfile.ZipFile(path) as z:
        return {name:z.read(name) for name in z.namelist()}
def text(root):
    return ''.join(n.text or '' for n in root.iter(W+'t'))
source,edited=package(a.source),package(a.edited)
before,after=[ET.fromstring(p['word/document.xml']) for p in [source,edited]]
assert text(after)==a.body_prefix+text(before)
old,new=[list(root.iter(W+'sectPr')) for root in [before,after]]
assert len(old)==len(new)==2
assert ET.tostring(old[1])==ET.tostring(new[1])
cols=new[0].find(W+'cols')
assert cols is not None and cols.get(W+'num')=='2'
assert cols.get(W+'equalWidth')=='0' and cols.get(W+'sep')=='1'
assert new[0].find(W+'bidi').get(W+'val')=='1'
definitions=list(cols.findall(W+'col'))
assert len(definitions)==2
size,margin=old[0].find(W+'pgSz'),old[0].find(W+'pgMar')
available=int(size.get(W+'w'))-int(margin.get(W+'left'))-int(margin.get(W+'right'))-int(margin.get(W+'gutter','0'))
gap=min(720,available-2)
first=round((available-gap)/3)
assert [int(c.get(W+'w')) for c in definitions]==[first,available-gap-first]
assert [int(c.get(W+'space','0')) for c in definitions]==[gap,0]
for section in [old[0],new[0]]:
    for tag in ['cols','bidi']:
        for child in list(section.findall(W+tag)):
            section.remove(child)
assert ET.tostring(old[0])==ET.tostring(new[0])
for name in set(source)|set(edited):
    if name!='word/document.xml':
        assert source[name]==edited[name],name
footer=ET.fromstring(edited['word/footer1.xml'])
assert [text(n) for n in footer.iter(W+'fldSimple')]==['99','99']
print(json.dumps({'passed':True,'source':str(a.source),'edited':str(a.edited),
    'bodyPrefix':a.body_prefix,'columnWidths':[int(c.get(W+'w')) for c in definitions],
    'rightToLeft':True,'separator':True,'otherSectionUnchanged':True,
    'unrelatedPackagePartsUnchanged':True,'fieldCachesUnchanged':True,
    'microsoftWordExecuted':False},indent=2))
