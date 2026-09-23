"""Excel escape sequences, authored independently of Folio's codec."""
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
row = etree.SubElement(sheet.find('s:sheetData', ns), '{' + S + '}row', r='30')
sheet.find('s:dimension', ns).set('ref', 'A1:T30')
content = etree.fromstring(('''<is xmlns="''' + S + '''" xmlns:m="urn:folio:escaped">
  <r m:flag="keep-run"><rPr><outline/><rFont val="Georgia"/><sz val="18"/><b/></rPr><t m:flag="keep-text" xml:space="preserve">A_x000D_B_xD83C__xDF0A_</t></r>
  <r><rPr><i/><color rgb="FF0044BB"/></rPr><t xml:space="preserve"> _x005F_x0041_ &amp; end</t></r>
  <rPh sb="0" eb="1"><t>PHONETIC</t></rPh>
</is>''').encode())
cell = etree.SubElement(row, '{' + S + '}c', r='A30', t='inlineStr')
cell.append(content)
shared = etree.fromstring(parts['xl/sharedStrings.xml'])
si = etree.SubElement(shared, '{' + S + '}si')
si[:] = [deepcopy(n) for n in content]
shared.set('count', '4'); shared.set('uniqueCount', '2')
for address in ['B30', 'C30']:
    cell = etree.SubElement(row, '{' + S + '}c', r=address, t='s')
    etree.SubElement(cell, '{' + S + '}v').text = '1'
for address, text in [
    ('D30', '_x0030_01'), ('E30', '_x003D_literal'),
    ('F30', '_x0054_RUE'), ('G30', 'a_x0000_b_x0001_c_x000D_\n\tend'),
    ('H30', '_xD800_X_xDC00_ _X0041_ _xZZZZ_'),
    ('I30', '_x005F_x0041_')
]:
    cell = etree.SubElement(row, '{' + S + '}c', r=address, t='str')
    etree.SubElement(cell, '{' + S + '}v').text = text
for address, formula, cache in [
    ('J30', '"_x0041_"', '_x005F_x0041_'),
    ('K30', '"001"', '001'), ('L30', '"🌊"', '_xD83C__xDF0A_')
]:
    cell = etree.SubElement(row, '{' + S + '}c', r=address, t='str')
    etree.SubElement(cell, '{' + S + '}f').text = formula
    etree.SubElement(cell, '{' + S + '}v').text = cache
parts['xl/worksheets/sheet1.xml'] = etree.tostring(sheet)
parts['xl/sharedStrings.xml'] = etree.tostring(shared)
with ZipFile(folder / 'escaped-strings.xlsx', 'w', ZIP_DEFLATED) as target:
    for part, value in parts.items(): target.writestr(part, value)
print('Created escaped-strings.xlsx')
