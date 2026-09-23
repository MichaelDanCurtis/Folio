"""Independent fixtures for editing a property without rebuilding the style."""
from copy import deepcopy
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from lxml import etree
import re

folder = Path(__file__).resolve().parents[2] / 'test/fixtures'
S = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'
ns = {'s': S}
with ZipFile(folder / 'fill.xlsx') as source:
    parts = {name: source.read(name) for name in source.namelist()}
sheet = etree.fromstring(parts['xl/worksheets/sheet1.xml'])
styles = etree.fromstring(parts['xl/styles.xml'])
fonts, xfs, parents = (styles.find('s:' + tag, ns) for tag in ['fonts', 'cellXfs', 'cellStyleXfs'])
source_id = int(sheet.find('.//s:c[@r="B2"]', ns).get('s'))
base = xfs[source_id]
font = fonts[int(base.get('fontId'))]
for tag, val in [('family', '2'), ('charset', '1'), ('scheme', 'minor'),
                 ('vertAlign', 'superscript'), ('outline', '1'), ('shadow', '1')]:
    etree.SubElement(font, '{' + S + '}' + tag, val=val)
def cell(address, xf):
    xfs.append(xf)
    row = sheet.find('.//s:row[@r="2"]', ns)
    c = etree.SubElement(row, '{' + S + '}c', r=address, s=str(len(xfs) - 1), t='n')
    etree.SubElement(c, '{' + S + '}v').text = '45292'
for address, code in [('F2', '14'), ('G2', '27')]:
    xf = deepcopy(base); xf.set('numFmtId', code); cell(address, xf)
# Explicit false values must not be interpreted as true by presence alone.
false_font = deepcopy(font)
for tag in ['b', 'i', 'strike']:
    false_font.find('s:' + tag, ns).set('val', '0' if tag != 'i' else 'false')
fonts.append(false_font)
xf = deepcopy(base); xf.set('fontId', str(len(fonts) - 1)); cell('I2', xf)
# A direct XF can explicitly defer font selection to a named cell style.
parent = deepcopy(base); parents.append(parent)
xf = deepcopy(base); xf.set('xfId', str(len(parents) - 1)); xf.set('fontId', '0'); xf.set('applyFont', '0')
cell('L2', xf)
for address in ['S2','T2']:
    c = etree.SubElement(sheet.find('.//s:row[@r="2"]',ns), '{'+S+'}c', r=address, t='s')
    etree.SubElement(c,'{'+S+'}v').text='0'
parts['xl/sharedStrings.xml'] = ('<sst xmlns="'+S+'" count="2" uniqueCount="1"><si><r><rPr><rFont val="Times New Roman"/><b/><color rgb="FFAA0000"/></rPr><t>Shared</t></r><r><rPr><i/><sz val="22"/></rPr><t xml:space="preserve"> title</t></r></si></sst>').encode()
rels = etree.fromstring(parts['xl/_rels/workbook.xml.rels'])
etree.SubElement(rels,'{http://schemas.openxmlformats.org/package/2006/relationships}Relationship',Id='styleTestStrings',Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings',Target='sharedStrings.xml')
parts['xl/_rels/workbook.xml.rels'] = etree.tostring(rels)
types = etree.fromstring(parts['[Content_Types].xml'])
etree.SubElement(types,'{http://schemas.openxmlformats.org/package/2006/content-types}Override',PartName='/xl/sharedStrings.xml',ContentType='application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml')
parts['[Content_Types].xml'] = etree.tostring(types)
for container in [fonts, xfs, parents]: container.set('count', str(len(container)))
for row in sheet.findall('.//s:row', ns):
    # Excel cell records follow column order.
    def col(c):
        result = 0
        for letter in re.match('[A-Z]+', c.get('r')).group(): result = result * 26 + ord(letter) - 64
        return result
    row[:] = sorted(row, key=col)
parts['xl/worksheets/sheet1.xml'] = etree.tostring(sheet)
parts['xl/styles.xml'] = etree.tostring(styles)
def save(name, data):
    with ZipFile(folder / name, 'w', ZIP_DEFLATED) as target:
        for part, value in data.items(): target.writestr(part, value)
save('styles.xlsx', parts)

# Explicit prefixes and extension attributes scoped to the record's parent.
# Same-local-name extension attributes must remain distinct from OOXML IDs.
data = parts['xl/styles.xml'].replace(('xmlns="' + S + '"').encode(), ('xmlns:x="' + S + '"').encode())
data = re.sub(rb'(<[/]?)([A-Za-z][A-Za-z0-9]*)(?=[ > /])', rb'\1x:\2', data)
data = data.replace(b'<x:fonts ', b'<x:fonts xmlns:m="urn:folio:style:metadata" m:count="keep-fonts" ', 1)
data = data.replace(b'<x:cellXfs ', b'<x:cellXfs xmlns:m="urn:folio:style:metadata" m:count="keep-xfs" ', 1)
data = data.replace(b'<x:font>', b'<x:font m:flag="keep-font">')
root = etree.fromstring(data)
xf = root.find('s:cellXfs', ns)[source_id]
# Deliberately place the qualified attribute before the real fontId.
original_attributes = dict(xf.attrib)
xf.attrib.clear()
xf.set('{urn:folio:style:metadata}fontId', 'keep-id')
xf.attrib.update(original_attributes)
xf.set('quotePrefix', '1')
ext = etree.SubElement(xf, '{' + S + '}extLst')
entry = etree.SubElement(ext, '{' + S + '}ext', uri='urn:folio:style:test')
etree.SubElement(entry, '{urn:folio:style:metadata}payload', value='keep-extension')
parts['xl/styles.xml'] = etree.tostring(root)
save('styles-prefixed.xlsx', parts)
print('Created styles.xlsx and styles-prefixed.xlsx')
