"""Exercise valid SpreadsheetML with only a prefixed worksheet namespace."""
from pathlib import Path
from zipfile import ZipFile,ZIP_DEFLATED
from lxml import etree as ET
root=Path(__file__).resolve().parents[2]
ns='http://schemas.openxmlformats.org/spreadsheetml/2006/main'
with ZipFile(root/'test/fixtures/range-sort.xlsx') as original, ZipFile(root/'test/fixtures/filter-prefixed.xlsx','w',ZIP_DEFLATED) as out:
    for name in original.namelist():
        data=original.read(name)
        if name.startswith('xl/worksheets/sheet') and name.endswith('.xml'):
            old=ET.fromstring(data)
            new=ET.Element(old.tag,dict(old.attrib),nsmap={'x':ns,'r':'http://schemas.openxmlformats.org/officeDocument/2006/relationships'})
            new.extend(old)
            if name=='xl/worksheets/sheet1.xml':
                filt=ET.Element('{'+ns+'}autoFilter',ref='A1:D7')
                new.insert(list(new).index(new.find('{'+ns+'}sheetData'))+1,filt)
            data=ET.tostring(new)
            assert b'<x:worksheet' in data and b'<x:c ' in data
        out.writestr(name,data)
print('Wrote filter-prefixed.xlsx')
