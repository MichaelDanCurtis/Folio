"""Independent Office reader for multi-row AutoFit transactions and UI output."""
import json
import sys
import warnings
from copy import copy
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
from openpyxl import load_workbook
from lxml import etree

warnings.filterwarnings('ignore', message='Unknown extension is not supported and will be removed')
folder = Path(gettempdir())/'folio-geometry-tests'
root = Path(__file__).resolve().parents[2]
ns = {'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
original = root/'test/fixtures/geometry.xlsx'
assert (folder/'autofit-undone.xlsx').read_bytes() == original.read_bytes()
for name in ['autofit-edited.xlsx', 'autofit-redone.xlsx', 'autofit-reopened.xlsx']:
    path = folder/name
    assert path.read_bytes() == (folder/'autofit-edited.xlsx').read_bytes()
    sheet = load_workbook(path, rich_text=True)['Records']
    for row,height in [(2,78.75),(3,24),(5,90),(25,44)]:
        assert sheet.row_dimensions[row].height == height
    assert sheet.row_dimensions[4].hidden
    assert sheet['A2'].comment is not None and sheet['A2'].hyperlink is not None
    assert sheet['D2'].value == '=B2*C2'
    with ZipFile(path) as z, ZipFile(original) as old:
        assert z.testzip() is None
        assert z.read('xl/styles.xml') == old.read('xl/styles.xml')
        assert z.read('xl/theme/theme1.xml') == old.read('xl/theme/theme1.xml')
        xml = etree.fromstring(z.read('xl/worksheets/sheet1.xml'))
        for row in [2,3,5]:
            assert xml.find(f's:sheetData/s:row[@r="{row}"]',ns).get('customHeight') in (None, '0', 'false')

if '--with-ui' in sys.argv:
    folder = Path(gettempdir())/'folio-autofit-ui-tests'
    original = root/'test/fixtures/autofit.xlsx'
    before = load_workbook(original, rich_text=True).active
    after = load_workbook(folder/'fitted.xlsx', rich_text=True).active
    heights = json.loads((folder/'heights.json').read_text())
    assert after.row_dimensions[2].height > 45  # Includes offscreen Z2's four lines.
    assert after.row_dimensions[3].height > 35
    assert after.row_dimensions[6].hidden and after.row_dimensions[6].height == 75
    assert after.freeze_panes == before.freeze_panes == 'A2'
    for row in range(1,7): assert after.row_dimensions[row].height == heights[str(row)]
    for row in before:
        for c in row:
            other = after[c.coordinate]
            assert str(other.value) == str(c.value)
            for prop in ['font','fill','border','alignment','protection']:
                assert copy(getattr(other,prop)) == copy(getattr(c,prop)), (c.coordinate,prop)
    assert after['B4'].value == before['B4'].value  # Rich run sizes and styles.
    assert after['B2'].comment.text == before['B2'].comment.text
    for col in ['A','B','C','Z']:
        assert after.column_dimensions[col].width == before.column_dimensions[col].width
    with ZipFile(folder/'fitted.xlsx') as z, ZipFile(original) as old:
        assert z.testzip() is None
        assert z.read('xl/styles.xml') == old.read('xl/styles.xml')
    assert (folder/'undone.xlsx').read_bytes() == original.read_bytes()
    for name in ['redone.xlsx','reopened.xlsx']:
        assert (folder/name).read_bytes() == (folder/'fitted.xlsx').read_bytes()
    print('Verified real-menu AutoFit XLSX, offscreen/multiline row heights, retained rich styles/notes/views, exact Undo/Redo and reopen.')
print('Independent row-height transaction, visibility, metadata and recovery checks passed.')
