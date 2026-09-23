"""Independent rich text, phonetic, explicit-false and Unicode XLSX fixture."""
from copy import deepcopy
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from lxml import etree

folder = Path(__file__).resolve().parents[2] / 'test/fixtures'
S = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'
ns = {'s': S}
with ZipFile(folder / 'styles.xlsx') as source:
    parts = {name: source.read(name) for name in source.namelist()}
sheet = etree.fromstring(parts['xl/worksheets/sheet1.xml'])
data = sheet.find('s:sheetData', ns)
row = etree.SubElement(data, '{' + S + '}row', r='20')
content = etree.fromstring(('''<is xmlns="''' + S + '''" xmlns:m="urn:folio:rich">
  <r m:flag="retain-run"><rPr><outline/><shadow/><rFont val="Georgia"/><sz m:val="900" val="18"/><b/><color rgb="FFAA0000"/></rPr><t m:flag="retain-text" xml:space="preserve">Rich </t></r>
  <r><rPr><rFont val="Arial"/><sz val="12"/><b val="false"/><i/><strike val="0"/><u/><vertAlign val="baseline"/><color rgb="FF0044BB"/></rPr><t>text 🌊</t></r>
  <rPh sb="0" eb="4"><t>PHONETIC ONLY</t></rPh>
  <phoneticPr fontId="0"/>
</is>''').encode())
cell = etree.SubElement(row, '{' + S + '}c', r='A20', t='inlineStr')
cell.append(content)
shared = etree.fromstring(parts['xl/sharedStrings.xml'])
si = etree.SubElement(shared, '{' + S + '}si')
si[:] = [deepcopy(n) for n in content]
shared.set('count', '4'); shared.set('uniqueCount', '2')
for address in ['B20', 'C20']:
    cell = etree.SubElement(row, '{' + S + '}c', r=address, t='s')
    etree.SubElement(cell, '{' + S + '}v').text = '1'
# Rich runs with no properties inherit the cell font. Explicit false and
# baseline values override that font. Empty runs do not create visible text.
style = sheet.find('.//s:c[@r="B2"]', ns).get('s')
cell = etree.SubElement(row, '{' + S + '}c', r='D20', s=style, t='inlineStr')
cell.append(etree.fromstring(('''<is xmlns="''' + S + '''"><r><t>A</t></r>
<r><rPr><b val="0"/><i val="false"/><strike val="0"/><u val="none"/><vertAlign val="baseline"/><color auto="1"/></rPr><t>B</t></r><r><t/></r></is>''').encode()))
for address, text in [('E20', '001'), ('F20', '=literal'), ('G20', 'TRUE')]:
    cell = etree.SubElement(row, '{' + S + '}c', r=address, t='inlineStr')
    content = etree.SubElement(cell, '{' + S + '}is')
    run = etree.SubElement(content, '{' + S + '}r')
    etree.SubElement(run, '{' + S + '}t').text = text
# Plain text with pronunciation annotations also excludes the annotation.
cell = etree.SubElement(row, '{' + S + '}c', r='H20', t='inlineStr')
cell.append(etree.fromstring(('''<is xmlns="''' + S + '''"><t>東京</t><rPh sb="0" eb="2"><t>とうきょう</t></rPh></is>''').encode()))
cell = etree.SubElement(row, '{' + S + '}c', r='I20', t='str')
etree.SubElement(cell, '{' + S + '}v').text = 'Stored string'
parts['xl/worksheets/sheet1.xml'] = etree.tostring(sheet)
parts['xl/sharedStrings.xml'] = etree.tostring(shared)
with ZipFile(folder / 'rich-text.xlsx', 'w', ZIP_DEFLATED) as target:
    for part, value in parts.items(): target.writestr(part, value)
print('Created rich-text.xlsx')
