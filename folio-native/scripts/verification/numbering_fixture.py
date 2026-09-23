"""Small independent OOXML fixture for instance, level and style inheritance."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
from lxml import etree
from docx import Document

root = Path(__file__).resolve().parents[2]
target = root / 'test/fixtures/numbering.docx'
doc = Document()
doc.add_paragraph('placeholder')
doc.save(target)
with ZipFile(target) as z:
    parts = {n:z.read(n) for n in z.namelist()}
W = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
ns = {'w': W}
numbering = etree.fromstring(parts['word/numbering.xml'])
for n in list(numbering): numbering.remove(n)
numbering.extend(etree.fromstring(f'<root xmlns:w="{W}"><w:abstractNum w:abstractNumId="42"><w:multiLevelType w:val="multilevel"/><w:lvl w:ilvl="0"><w:start w:val="3"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl><w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="lowerLetter"/><w:lvlRestart w:val="0"/><w:lvlText w:val="%1.%2)"/></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId="43"><w:lvl w:ilvl="0"><w:start w:val="4"/><w:numFmt w:val="upperRoman"/><w:lvlText w:val="%1)"/></w:lvl></w:abstractNum><w:num w:numId="7"><w:abstractNumId w:val="42"/><w:lvlOverride w:ilvl="0"><w:startOverride w:val="5"/></w:lvlOverride></w:num><w:num w:numId="8"><w:abstractNumId w:val="42"/><w:lvlOverride w:ilvl="0"><w:startOverride w:val="2"/></w:lvlOverride></w:num><w:num w:numId="9"><w:abstractNumId w:val="43"/></w:num></root>'))
styles = etree.fromstring(parts['word/styles.xml'])
styles.extend(etree.fromstring(f'<root xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="ListBase"><w:name w:val="List base"/><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="8"/></w:numPr></w:pPr></w:style><w:style w:type="paragraph" w:styleId="DerivedList"><w:name w:val="Derived list"/><w:basedOn w:val="ListBase"/></w:style></root>'))
body = etree.fromstring(parts['word/document.xml'])
b = body.find('w:body', ns)
section = b.find('w:sectPr', ns)
for n in list(b): b.remove(n)
def paragraph(text, num=None, level=0, style=None, bold=False):
    p = etree.SubElement(b, f'{{{W}}}p')
    pr = etree.SubElement(p, f'{{{W}}}pPr')
    if style: etree.SubElement(pr, f'{{{W}}}pStyle', {f'{{{W}}}val':style})
    etree.SubElement(pr, f'{{{W}}}keepNext')
    if num is not None:
        np = etree.SubElement(pr, f'{{{W}}}numPr')
        etree.SubElement(np, f'{{{W}}}ilvl', {f'{{{W}}}val':str(level)})
        etree.SubElement(np, f'{{{W}}}numId', {f'{{{W}}}val':str(num)})
    etree.SubElement(pr, f'{{{W}}}spacing', {f'{{{W}}}after':'140'})
    r = etree.SubElement(p, f'{{{W}}}r')
    if bold: etree.SubElement(etree.SubElement(r, f'{{{W}}}rPr'), f'{{{W}}}b')
    etree.SubElement(r, f'{{{W}}}t').text = text
paragraph('First project',7,bold=True)
paragraph('First detail',7,1)
paragraph('Second detail',7,1)
paragraph('Intervening prose')
paragraph('Second project',7)
paragraph('Continued detail',7,1)
paragraph('Separate instance',8)
paragraph('Roman item',9)
paragraph('Inherited numbering',style='DerivedList')
paragraph('Explicitly unnumbered',0,style='DerivedList')
b.append(section)
for name, tree in [('word/document.xml',body),('word/styles.xml',styles),('word/numbering.xml',numbering)]:
    parts[name]=etree.tostring(tree,xml_declaration=True,encoding='UTF-8',standalone=True)
with ZipFile(target,'w',ZIP_DEFLATED) as z:
    for name,data in parts.items(): z.writestr(name,data)
# An alternate prefix exercises inherited namespace bindings during edits.
for name in ['word/document.xml','word/styles.xml','word/numbering.xml']:
    parts[name]=parts[name].replace(b'xmlns:w=',b'xmlns:folioW=').replace(b'<w:',b'<folioW:').replace(b'</w:',b'</folioW:').replace(b' w:',b' folioW:')
with ZipFile(target.with_name('numbering-prefixed.docx'),'w',ZIP_DEFLATED) as z:
    for name,data in parts.items(): z.writestr(name,data)
print(target)
