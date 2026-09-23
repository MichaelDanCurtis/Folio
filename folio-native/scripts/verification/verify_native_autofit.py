"""Read-only verification of the native AutoFit qualification artifacts.

Usage: python verify_native_autofit.py /path/to/autofit-native
The snapshots and XLSX files must come from the packaged native UI run;
this script does not create them or call Folio's engine.
"""

import argparse
from copy import copy, deepcopy
import json
from pathlib import Path
import warnings
from zipfile import ZipFile

from lxml import etree
from openpyxl import load_workbook
from openpyxl.utils import get_column_letter


def verify(folder: Path) -> None:
    warnings.filterwarnings(
        "ignore", message="Unknown extension is not supported and will be removed"
    )
    source = Path(__file__).resolve().parents[2] / "test/fixtures/column-autofit.xlsx"
    snapshots = {
        name: json.loads((folder / f"{name}.folio").read_text())
        for name in [
            "before-fit", "columns-fitted", "columns-undone", "columns-redone",
            "rows-fitted", "rows-undone-after-export", "before-restart", "after-restart",
            "column-dragged", "column-double-clicked", "row-dragged", "row-double-clicked",
            "xlsx-reimported",
        ]
    }
    baseline = snapshots["before-fit"]
    columns = snapshots["columns-fitted"]
    rows = snapshots["rows-fitted"]

    # Revisions advance during Undo/Redo; the remainder must be exact.
    def without_revision(workspace):
        return {key: value for key, value in workspace.items() if key != "revision"}

    for actual, expected in [
        ("columns-undone", "before-fit"),
        ("columns-redone", "columns-fitted"),
        ("rows-undone-after-export", "columns-redone"),
        ("before-restart", "rows-fitted"),
        ("column-double-clicked", "rows-fitted"),
        ("row-double-clicked", "rows-fitted"),
    ]:
        assert without_revision(snapshots[actual]) == without_revision(snapshots[expected]), actual
    assert columns["revision"] == baseline["revision"] + 1
    assert snapshots["columns-undone"]["revision"] == columns["revision"] + 1
    assert snapshots["columns-redone"]["revision"] == columns["revision"] + 2
    assert rows["revision"] == snapshots["columns-redone"]["revision"] + 1
    assert snapshots["rows-undone-after-export"]["revision"] == rows["revision"] + 1
    assert snapshots["before-restart"]["revision"] == rows["revision"] + 2
    assert (folder / "before-restart.folio").read_bytes() == (folder / "after-restart.folio").read_bytes()

    # Only the requested dimension metadata may change, including in source packages.
    expected = deepcopy(baseline)
    expected["sheets"][0]["columnWidths"] = columns["sheets"][0]["columnWidths"]
    assert without_revision(expected) == without_revision(columns)
    expected = deepcopy(columns)
    expected["sheets"][0]["rowHeights"] = rows["sheets"][0]["rowHeights"]
    assert without_revision(expected) == without_revision(rows)
    sheet = rows["sheets"][0]
    dragged_column = snapshots["column-dragged"]["sheets"][0]
    dragged_row = snapshots["row-dragged"]["sheets"][0]
    assert dragged_column["columnWidths"]["2"] > sheet["columnWidths"]["2"]
    assert dragged_row["rowHeights"]["4"] > sheet["rowHeights"]["4"]
    for axis in ["column", "row"]:
        assert snapshots[f"{axis}-double-clicked"]["revision"] == snapshots[f"{axis}-dragged"]["revision"] + 1
    reimported = snapshots["xlsx-reimported"]["sheets"][0]
    assert {k: v for k, v in reimported.items() if k != "cached"} == {
        k: v for k, v in sheet.items() if k != "cached"
    }
    expected_cache = {**sheet["cached"], "B10": 24691.34}
    assert reimported["cached"] == expected_cache
    assert sheet["columnWidths"]["4"] == 17
    assert sheet["rowHeights"]["6"] == 75
    assert sheet["rowHeights"]["2"] > 45  # Offscreen Z2's four lines.
    assert sheet["rowHeights"]["3"] > 35  # Three manual text lines.
    assert sheet["rowHeights"]["11"] > 30  # Quarter-turn text.
    assert sheet["columnWidths"]["3"] > sheet["columnWidths"]["1"]  # Offscreen C200.

    exported = folder / "autofit-native-fitted.xlsx"
    assert exported.read_bytes() == (folder / "autofit-native-reopened.xlsx").read_bytes()
    before = load_workbook(source, rich_text=True)
    after = load_workbook(exported, rich_text=True)
    a, b = after.active, before.active
    assert a.title == b.title == "Delivery plan"
    for index, width in sheet["columnWidths"].items():
        assert a.column_dimensions[get_column_letter(int(index))].width == width
    for index, height in sheet["rowHeights"].items():
        assert a.row_dimensions[int(index)].height == height
    assert a.column_dimensions["D"].hidden
    assert a.row_dimensions[6].hidden and a.row_dimensions[7].hidden
    assert a.freeze_panes == b.freeze_panes == "A2"
    for source_row in b:
        for cell in source_row:
            other = a[cell.coordinate]
            assert other.value == cell.value, cell.coordinate
            for prop in ["font", "fill", "border", "alignment", "protection", "number_format"]:
                assert copy(getattr(other, prop)) == copy(getattr(cell, prop)), (cell.coordinate, prop)
    assert a["B2"].comment.text == b["B2"].comment.text
    assert after._named_styles["Standard"].font.name == "Georgia"
    assert after._named_styles["Standard"].font.sz == 14
    assert load_workbook(exported, data_only=True).active["B10"].value == 24691.34
    ns = {"s": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}
    with ZipFile(exported) as new, ZipFile(source) as old:
        assert new.testzip() is None
        assert set(new.namelist()) == set(old.namelist())
        for name in old.namelist():
            if name not in {"xl/worksheets/sheet1.xml", "xl/workbook.xml"}:
                assert new.read(name) == old.read(name), name
        # Export explicitly asks Excel to refresh its formula calculation state.
        # Namespace-prefix/empty-element serialization is not workbook content.
        def semantic(node):
            return (node.tag, dict(node.attrib), node.text or "", [semantic(child) for child in node])

        old_book = etree.fromstring(old.read("xl/workbook.xml"))
        new_book = etree.fromstring(new.read("xl/workbook.xml"))
        calculation = new_book.find("s:calcPr", ns)
        assert dict(calculation.attrib) == {"calcId": "0", "fullCalcOnLoad": "1", "forceFullCalc": "1"}
        old_book.remove(old_book.find("s:calcPr", ns))
        new_book.remove(calculation)
        assert semantic(old_book) == semantic(new_book)
        xml = etree.fromstring(new.read("xl/worksheets/sheet1.xml"))
        for col in xml.findall("s:cols/s:col", ns):
            if int(col.get("min")) in [1, 2, 3]:
                assert col.get("customWidth") == "1"
                assert col.get("bestFit") not in ["1", "true"]
        for row in [1, 2, 3, 4, 5, 8, 9, 10, 11]:
            assert xml.find(f's:sheetData/s:row[@r="{row}"]', ns).get("customHeight") == "1"
    print("Native column/row AutoFit: exact keyboard Undo/Redo, boundary clicks, restart and XLSX reimport passed.")
    print("Native Save/reopened XLSX: widths, heights, rich text, formulas, comments, hidden dimensions,")
    print("frozen view, Normal font, recalculation request and retained package parts verified independently.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folder", type=Path)
    verify(parser.parse_args().folder)
