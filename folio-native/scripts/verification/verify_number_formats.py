"""Read generated formatting outputs independently; openpyxl is not a display oracle."""
from pathlib import Path
import tempfile, json, zipfile
from copy import copy
from openpyxl import load_workbook
from lxml import etree
root = Path(tempfile.gettempdir()) / 'folio-number-format-tests'
project = Path(__file__).resolve().parents[2]
ns = {'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
source = load_workbook(project/'test/fixtures/number-formats.xlsx')['Formats']
book = load_workbook(root/'formatted.xlsx')
sheet = book['Formats']
assert sheet['B2'].value == 6 and sheet['B2'].number_format == '0.0000000000'
assert sheet['B21'].value == '=B2+1'
assert load_workbook(root/'formatted.xlsx',data_only=True)['Formats']['B21'].value == 7
assert sheet.row_dimensions[2].height == 40.5
for row in range(2,25):
    address=f'B{row}'
    if row != 2: assert sheet[address].number_format == source[address].number_format, address
    for field in ['font','fill','border','alignment','protection']:
        assert copy(getattr(sheet[address],field)) == copy(getattr(source[address],field)), (address,field)
assert sheet.column_dimensions['E'].number_format == '0.0000'
assert sheet.row_dimensions[25].number_format == '0.00%'
with zipfile.ZipFile(root/'formatted.xlsx') as z:
    xml=etree.fromstring(z.read('xl/worksheets/sheet1.xml'))
    def raw(a): return xml.xpath(f'string(//s:c[@r="{a}"]/s:v)',namespaces=ns)
    assert float(raw('B8')) == 60 # openpyxl intentionally normalizes the fictional leap day.
    assert float(raw('B9')) == .9999999
    assert float(raw('B7')) == 123 # display hidden; number retained.
    assert not xml.xpath('//s:row[@r="2"]/@customHeight',namespaces=ns)
assert (root/'formatted.xlsx').read_bytes() == (root/'reopened.xlsx').read_bytes()
mac=load_workbook(root/'date1904.xlsx')
assert mac.epoch.year == 1904 and mac['Formats']['B23'].value.year == 1904
snapshot=json.loads((root/'display.json').read_text())
assert snapshot['formatted']['office-1']['B8']['text'] == '1900-02-29'
assert snapshot['formatted']['office-1']['B9']['text'] == '0:00:00'
ui=Path(tempfile.gettempdir())/'folio-number-format-ui-tests'
if (ui/'formatted.xlsx').exists():
    target=load_workbook(ui/'formatted.xlsx')['Formats']
    assert target['B2'].value == 6 and target['B3'].value == 1234.5
    assert target['B2'].number_format == target['B3'].number_format
    assert '* ' in target['B2'].number_format
    assert target['A30'].value == 'newer edit'
    assert (ui/'formatted.xlsx').read_bytes() == (ui/'reopened.xlsx').read_bytes()
else:
    raise AssertionError('Run the real-bridge number-format widget test first')
print('Number-format XLSX, cached formula, styles, date system and save/reopen checks passed (engine + widget).')
