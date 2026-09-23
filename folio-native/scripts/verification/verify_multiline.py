"""Independent reader for native-bridge multiline editor exports."""

from copy import copy
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
import warnings

from lxml import etree
from openpyxl import load_workbook

warnings.filterwarnings("ignore", message="Unknown extension is not supported and will be removed")
folder = Path(gettempdir()) / "folio-multiline-ui-tests"
source = Path(__file__).resolve().parents[2] / "test/fixtures/column-autofit.xlsx"
before = load_workbook(source, rich_text=True).active
text = str(before["B3"].value) + "\nCoordination 🦉"
ns = {"s": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}

assert (folder / "undone.xlsx").read_bytes() == source.read_bytes()
assert (folder / "redone.xlsx").read_bytes() == (folder / "edited.xlsx").read_bytes()
assert (folder / "reopened.xlsx").read_bytes() == (folder / "fitted.xlsx").read_bytes()
assert (folder / "reimported.xlsx").read_bytes() == (folder / "fitted.xlsx").read_bytes()

for name in ["edited", "redone", "formula", "fitted", "reopened", "reimported"]:
    path = folder / f"{name}.xlsx"
    book = load_workbook(path, rich_text=True)
    after = book.active
    assert after["B3"].value == text
    formula = "=A10*2" if name in ["edited", "redone"] else "=SUM(A10,\nA10)"
    assert after["B10"].value == formula
    assert load_workbook(path, data_only=True).active["B10"].value == 24691.34
    for source_row in before:
        for cell in source_row:
            other = after[cell.coordinate]
            if cell.coordinate not in ["B3", "B10"]:
                assert other.value == cell.value, (name, cell.coordinate)
            for prop in ["font", "fill", "border", "alignment", "number_format", "protection"]:
                assert copy(getattr(other, prop)) == copy(getattr(cell, prop)), (name, cell.coordinate, prop)
    assert after["B2"].comment.text == before["B2"].comment.text
    assert after.freeze_panes == before.freeze_panes
    for col in ["A", "B", "C", "D", "Z"]:
        assert after.column_dimensions[col].width == before.column_dimensions[col].width
        assert after.column_dimensions[col].hidden == before.column_dimensions[col].hidden
    for row in range(1, 12):
        assert after.row_dimensions[row].hidden == before.row_dimensions[row].hidden
        if row == 3 and name in ["fitted", "reopened", "reimported"]:
            assert after.row_dimensions[row].height > 50
        elif row == 10 and name not in ["edited", "redone"]:
            assert 15 <= after.row_dimensions[row].height <= 409
        else:
            assert after.row_dimensions[row].height == before.row_dimensions[row].height
    with ZipFile(source) as old, ZipFile(path) as new:
        assert new.testzip() is None
        assert set(new.namelist()) == set(old.namelist())
        for part in old.namelist():
            if part not in {"xl/worksheets/sheet1.xml", "xl/workbook.xml"}:
                assert new.read(part) == old.read(part), (name, part)
        xml = etree.fromstring(new.read("xl/worksheets/sheet1.xml"))
        cell = xml.find('s:sheetData/s:row/s:c[@r="B3"]', ns)
        assert "".join(cell.itertext()) == text
        if name not in ["edited", "redone"]:
            assert xml.find('s:sheetData/s:row/s:c[@r="B10"]/s:f', ns).text == formula[1:]
            assert xml.find('s:sheetData/s:row[@r="10"]', ns).get('customHeight') not in ['1','true']

print("Multiline editor XLSX checks passed: text/Unicode/newlines, formula and cache, exact Undo/Redo/reopen/reimport,")
print("explicit row fit, original formatting/rich runs/comments/views and retained package parts.")
