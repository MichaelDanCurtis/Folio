"""Create an independent, explicitly typed OOXML error/text regression workbook."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from openpyxl import Workbook
from lxml import etree as E

S = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'
ns = {'s': S}
out = Path(__file__).resolve().parents[2] / 'test/fixtures/error-types.xlsx'
book = Workbook()
sheet = book.active
sheet.title = 'Error types'
sheet.append(['Case', 'Error value', 'Literal text', 'Formula error', 'Formula text',
              'Error?', 'Text error?', 'Formula error?', 'Formula text error?',
              'Handled', 'Unchanged text', 'Error code'])
errors = ['#NULL!', '#DIV/0!', '#VALUE!', '#REF!', '#NAME?', '#NUM!', '#N/A']
formulas = ['=#NULL!', '=1/0', '=VALUE("bad")', '=#REF!', '=FolioMissingName', '=SQRT(-1)', '=NA()']
for row, (error, formula) in enumerate(zip(errors, formulas), 2):
    sheet.append([error, error, error, formula, f'="{error}"', f'=ISERROR(B{row})',
                  f'=ISERROR(C{row})', f'=ISERROR(D{row})', f'=ISERROR(E{row})',
                  f'=IFERROR(D{row},"handled")', f'=IFERROR(E{row},"handled")',
                  f'=ERROR.TYPE(D{row})'])
    sheet[f'C{row}'].data_type = 's'
    for column in 'BCDE':
        sheet[f'{column}{row}'].number_format = '0;0;0;"TEXT: "@'
sheet['N1'] = 'Switch result type'
sheet['N2'] = '=IF(P2=0,NA(),IF(P2=1,"#N/A",42))'
sheet['N2'].number_format = '0;0;0;"TEXT: "@'
sheet['P2'] = 0
for column in 'ABCDEFGHIJKLMNOP':
    sheet.column_dimensions[column].width = 20
sheet['A11'] = 'Shared rich text'
sheet['A12'] = 'String value'
sheet['A13'] = 'Lowercase text'
for a in ['C11', 'C12', 'C13']:
    sheet[a] = '#N/A'
    sheet[a].data_type = 's'
sheet['C13'] = '#n/a'
book.save(out)
with ZipFile(out) as archive:
    parts = {n: archive.read(n) for n in archive.namelist()}
xml = E.fromstring(parts['xl/worksheets/sheet1.xml'])
for row, error in enumerate(errors, 2):
    for column, kind, value in [('D', 'e', error), ('E', 'str', error),
                                ('F', 'b', '1'), ('G', 'b', '0'),
                                ('H', 'b', '1'), ('I', 'b', '0'),
                                ('J', 'str', 'handled'), ('K', 'str', error),
                                ('L', 'n', str(row - 1))]:
        c = xml.find(f'.//s:c[@r="{column}{row}"]', ns)
        c.set('t', kind)
        c.find('s:v', ns).text = value
for a, kind, value in [('C11', 's', '0'), ('C12', 'str', '#N/A'), ('N2', 'e', '#N/A')]:
    c = xml.find(f'.//s:c[@r="{a}"]', ns)
    for child in list(c):
        if child.tag != f'{{{S}}}f': c.remove(child)
    c.set('t', kind)
    E.SubElement(c, f'{{{S}}}v').text = value
    if a == 'N2':
        c.set('{urn:folio:error-types}retained', 'yes')
        c.find('s:f', ns).set('ca', '1')
        ext = E.SubElement(c, f'{{{S}}}extLst')
        E.SubElement(ext, f'{{{S}}}ext', uri='urn:folio:error-types').text = 'retained'
parts['xl/worksheets/sheet1.xml'] = E.tostring(xml)
parts['xl/sharedStrings.xml'] = f'<sst xmlns="{S}" count="1" uniqueCount="1"><si><r><rPr><b/></rPr><t>#N</t></r><r><t>/A</t></r></si></sst>'.encode()
rels = E.fromstring(parts['xl/_rels/workbook.xml.rels'])
E.SubElement(rels, '{http://schemas.openxmlformats.org/package/2006/relationships}Relationship',
             Id='errorStrings', Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings', Target='sharedStrings.xml')
parts['xl/_rels/workbook.xml.rels'] = E.tostring(rels)
content = E.fromstring(parts['[Content_Types].xml'])
E.SubElement(content, '{http://schemas.openxmlformats.org/package/2006/content-types}Override',
             PartName='/xl/sharedStrings.xml', ContentType='application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml')
parts['[Content_Types].xml'] = E.tostring(content)
with ZipFile(out, 'w', ZIP_DEFLATED) as archive:
    for name, data in parts.items(): archive.writestr(name, data)
print(out)
# Structural edits intentionally reject opaque extension ranges. Exercise those
# edits separately, without removing the metadata-retention case above.
for extension in xml.findall('.//s:extLst', ns): extension.getparent().remove(extension)
parts['xl/worksheets/sheet1.xml'] = E.tostring(xml)
with ZipFile(out.with_name('error-types-structure.xlsx'), 'w', ZIP_DEFLATED) as archive:
    for name, data in parts.items(): archive.writestr(name, data)
