"""Read exported files independently: overflow never populates adjacent cells."""
import argparse
import copy
import json
from pathlib import Path
import tempfile
import openpyxl


def verify_export(path):
    formulas = openpyxl.load_workbook(path, data_only=False)
    values = openpyxl.load_workbook(path, data_only=True)
    try:
        sheet = formulas.active
        assert sheet['A1'].value == 'A long project heading that extends across empty worksheet cells'
        assert sheet['B1'].value is None and sheet['C1'].value is None
        assert sheet['A2'].value == '=1/0'
        assert values.active['A2'].value == '#DIV/0!' and values.active['A2'].data_type == 'e'
        assert sheet['A3'].value == '="A formula can also return a long text label"'
        assert values.active['A3'].value == 'A formula can also return a long text label'
        assert sheet['A4'].value == '123456789012345678901234567890' and sheet['A4'].data_type == 's'
        populated = {cell.coordinate for row in sheet for cell in row if cell.value is not None}
        assert populated == {'A1', 'A2', 'A3', 'A4'}, populated
    finally:
        formulas.close()
        values.close()


def verify_native(folder):
    seed = json.loads((folder / 'seed.folio').read_text())
    assert seed['revision'] == 5
    for name, revision, blocked in [
        ('blocked', 6, True), ('undo', 7, False), ('redo', 8, True),
        ('before-restart', 9, False), ('after-restart', 9, False),
    ]:
        expected = copy.deepcopy(seed)
        expected['revision'] = revision
        if blocked:
            expected['sheets'][0]['cells']['B1'] = 'x'
            expected['sheets'][0]['rowHeightModes'] = {'1': 'auto'}
            expected['sheets'][0]['rowHeights'] = {'1': 24.0}
        actual = json.loads((folder / f'{name}.folio').read_text())
        assert actual == expected, name
    assert (folder / 'before-restart.folio').read_bytes() == (folder / 'after-restart.folio').read_bytes()
    verify_export(folder / 'overflow-native.xlsx')
    print('PASS: native edit/Undo/Redo, all saved workspace fields, exact restart bytes and native XLSX values/types.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-dir', type=Path, help='Read the isolated native qualification artifacts instead of automated fixtures.')
    args = parser.parse_args()
    if args.native_dir:
        verify_native(args.native_dir)
    else:
        verify_export(Path(tempfile.gettempdir()) / 'folio-overflow-ui-tests/overflow.xlsx')
        merged = openpyxl.load_workbook(Path(tempfile.gettempdir()) / 'folio-geometry-tests/overflow-merged-roundtrip.xlsx')
        try:
            assert 'I20:J21' in {str(r) for r in merged.active.merged_cells.ranges}
        finally:
            merged.close()
        print('PASS: source text, blank neighbors, text formula cache, numeric text, typed error and transformed merge survive XLSX export.')
