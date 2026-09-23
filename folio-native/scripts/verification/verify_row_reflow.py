"""Independent Office checks; do not use Folio to validate its own export."""
from copy import copy
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
import warnings
from lxml import etree
from openpyxl import load_workbook
warnings.filterwarnings('ignore', message='Unknown extension is not supported and will be removed')

root = Path(__file__).resolve().parents[2]
folder = Path(gettempdir())/'folio-row-reflow-ui-tests'
source = root/'test/fixtures/row-reflow.xlsx'
before = load_workbook(source, rich_text=True)
ns = {'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
lines = 'First line\nSecond line\nThird line'
wrapped = 'A longer sentence that changes height when its column becomes narrower.'
formula = 'Formula first\nFormula second\nFormula third'
assert (folder/'undone.xlsx').read_bytes() == source.read_bytes()
assert (folder/'edited.xlsx').read_bytes() == (folder/'redone.xlsx').read_bytes()
for name in ['reopened','reimported']:
    assert (folder/f'{name}.xlsx').read_bytes() == (folder/'final.xlsx').read_bytes()
for name in ['edited','redone','narrow','formula','final','reopened','reimported','queued']:
    path = folder/f'{name}.xlsx'
    w = load_workbook(path, rich_text=True)
    notes = w['Notes']
    initial = name in ['edited','redone']
    has_formula = name in ['formula','final','reopened','reimported','queued']
    assert notes['B2'].value == (lines if initial or name == 'queued' else wrapped)
    assert notes['B3'].value == ('Fixed height' if initial else lines)
    assert notes['B7'].value == '=Inputs!A1'
    assert w['Inputs']['A1'].value == (formula if has_formula else 'Start')
    cached = load_workbook(path, data_only=True)
    assert cached['Notes']['B7'].value == (formula if has_formula else 'Start')
    assert notes.row_dimensions[2].height > 40
    assert notes.row_dimensions[3].height == 18
    assert notes.row_dimensions[5].height == 22 and notes.row_dimensions[5].hidden
    assert notes.row_dimensions[8].height == 20
    if not initial:
        assert notes.column_dimensions['B'].width == 12
        assert notes.row_dimensions[200].height > 15
        assert notes.row_dimensions[1048576].height > 15
    if has_formula:
        assert notes.row_dimensions[7].height > 40
        assert w['Inputs'].row_dimensions[1].height > 30
    for s in before:
        current = w[s.title]
        # Sparse iteration matters: this fixture intentionally reaches row 1048576.
        for cell in s._cells.values():
            other = current[cell.coordinate]
            if (s.title,cell.coordinate) not in [('Notes','B2'),('Notes','B3'),('Inputs','A1')]:
                assert other.value == cell.value,(name,s.title,cell.coordinate)
            for prop in ['font','fill','border','alignment','protection','number_format']:
                assert copy(getattr(other,prop)) == copy(getattr(cell,prop)),(name,cell.coordinate,prop)
        assert current.freeze_panes == s.freeze_panes
        assert str(current.merged_cells) == str(s.merged_cells)
    assert notes['B2'].comment.text == before['Notes']['B2'].comment.text
    with ZipFile(path) as z, ZipFile(source) as original:
        assert z.testzip() is None
        assert set(original.namelist()).issubset(z.namelist())
        for added in set(z.namelist()) - set(original.namelist()):
            assert added == 'xl/worksheets/_rels/sheet2.xml.rels'
            relationships = etree.fromstring(z.read(added))
            assert relationships.tag == '{http://schemas.openxmlformats.org/package/2006/relationships}Relationships'
            assert len(relationships) == 0
        for part in original.namelist():
            if part not in ['xl/workbook.xml','xl/worksheets/sheet1.xml','xl/worksheets/sheet2.xml']:
                assert z.read(part) == original.read(part),(name,part)
        xml=etree.fromstring(z.read('xl/worksheets/sheet1.xml'))
        # openpyxl's customHeight property is derived from ht, so inspect the
        # actual OOXML flag to distinguish cached automatic and fixed heights.
        for r in ([2] if initial else [2,7,200,1048576]):
            node=xml.find(f's:sheetData/s:row[@r="{r}"]',ns)
            assert node.get('customHeight') not in ['1','true'],(name,r)
        assert xml.find('s:sheetData/s:row[@r="3"]',ns).get('customHeight') == '1'
print('Automatic row XLSX checks passed: multiline/wrap/formulas, manual/hidden/merged preservation, sparse last row,')
print('persistent sizing modes, unchanged rich styles/notes/views/parts and exact Undo/Redo/reopen/reimport.')
