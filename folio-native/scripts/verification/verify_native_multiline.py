"""Read-only native multiline qualification: pass the retained evidence folder."""
import argparse
from copy import copy, deepcopy
import json
from pathlib import Path
import warnings
from zipfile import ZipFile
from openpyxl import load_workbook

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('folder', type=Path)
folder = parser.parse_args().folder
warnings.filterwarnings('ignore', message='Unknown extension is not supported and will be removed')
source = Path(__file__).resolve().parents[2] / 'test/fixtures/column-autofit.xlsx'
names = ['before', 'text-applied', 'text-undone', 'text-redone', 'formula-applied',
         'formula-undone', 'fitted', 'before-restart', 'after-restart']
states = {n: json.loads((folder / f'{n}.folio').read_text()) for n in names}

def content(w):
    return {k: v for k, v in w.items() if k != 'revision'}

for actual, expected in [('text-undone', 'before'), ('text-redone', 'text-applied'),
                         ('formula-undone', 'text-redone'), ('before-restart', 'fitted')]:
    assert content(states[actual]) == content(states[expected]), actual
expected = deepcopy(states['before'])
expected['sheets'][0]['cells']['B3'] = 'Native 🦉 first\nNative second'
assert content(expected) == content(states['text-applied'])
expected['sheets'][0]['cells']['B10'] = '=SUM(A10,\nA10)'
assert content(expected) == content(states['formula-applied'])
height = states['fitted']['sheets'][0]['rowHeights']['3']
assert height > 18
expected['sheets'][0]['rowHeights']['3'] = height
assert content(expected) == content(states['fitted'])
assert [states[n]['revision'] for n in names] == [1, 2, 3, 4, 5, 8, 6, 10, 10]
assert (folder / 'after-restart.folio').read_bytes() == (folder / 'before-restart.folio').read_bytes()
exported = folder / 'multiline-native.xlsx'
assert exported.read_bytes() == (folder / 'multiline-reopened.xlsx').read_bytes()
before = load_workbook(source, rich_text=True).active
after = load_workbook(exported, rich_text=True).active
assert after['B3'].value == expected['sheets'][0]['cells']['B3']
assert after['B10'].value == expected['sheets'][0]['cells']['B10']
assert after.row_dimensions[3].height == height
assert load_workbook(exported, data_only=True).active['B10'].value == 24691.34
for row in before:
    for c in row:
        other = after[c.coordinate]
        if c.coordinate not in ['B3', 'B10']:
            assert other.value == c.value, c.coordinate
        for prop in ['font', 'fill', 'border', 'alignment', 'number_format', 'protection']:
            assert copy(getattr(other, prop)) == copy(getattr(c, prop)), (c.coordinate, prop)
assert after['B2'].comment.text == before['B2'].comment.text
assert after.freeze_panes == before.freeze_panes
assert after.row_dimensions[6].hidden and after.row_dimensions[7].hidden
assert after.column_dimensions['D'].hidden
with ZipFile(source) as old, ZipFile(exported) as new:
    assert new.testzip() is None
    assert set(old.namelist()) == set(new.namelist())
    for part in old.namelist():
        if part not in ['xl/worksheets/sheet1.xml', 'xl/workbook.xml']:
            assert new.read(part) == old.read(part), part
print('Native multiline cell/formula edits, independent history steps, fitted rows and exact restart/export checks passed.')
print('Independent XLSX reader verifies Unicode/newlines, formula cache, retained styles, rich text, comments and views.')
