"""Check files produced through the packaged macOS Save dialogs.

Usage: python verify_native_saves.py /path/to/disposable/verification/folder
The native workflow exports start-7 numbering, restarts it at 1 and replaces
the DOCX, saves a workspace copy, and exports the attached sample workbook.
This reader does not substitute for performing those native UI operations.
"""
import argparse
import json
from pathlib import Path
from zipfile import ZipFile

from docx import Document
from lxml import etree
from openpyxl import load_workbook

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("folder", type=Path)
folder = parser.parse_args().folder
ns = {"w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main"}
w = "{" + ns["w"] + "}"
for name, start in [("numbering-native-before-replace.docx", 7), ("numbering-native.docx", 1)]:
    path = folder / name
    document = Document(path)
    assert len(document.paragraphs) == 10
    assert document.paragraphs[0].text == "First project"
    assert document.paragraphs[0].runs[0].bold
    with ZipFile(path) as package:
        body = etree.fromstring(package.read("word/document.xml"))
        numbering = etree.fromstring(package.read("word/numbering.xml"))
        paragraphs = body.findall("w:body/w:p", ns)
        ids = [paragraphs[i].find("w:pPr/w:numPr/w:numId", ns).get(w + "val") for i in range(3)]
        assert len(set(ids)) == 1
        instance = numbering.find(f'w:num[@w:numId="{ids[0]}"]', ns)
        abstract = instance.find("w:abstractNumId", ns).get(w + "val")
        level = numbering.find(f'w:abstractNum[@w:abstractNumId="{abstract}"]/w:lvl[@w:ilvl="0"]', ns)
        assert level.find("w:start", ns).get(w + "val") == str(start)
        assert level.find("w:numFmt", ns).get(w + "val") == "upperLetter"
    print(f"{name}: start {start}, genuine numbering and bold text retained")

workspace = json.loads((folder / "numbering-native.folio").read_text())
assert workspace["document"]["content"][0]["attrs"]["numbering"]["levels"][0]["start"] == 1
workbook = load_workbook(folder / "numbering-native.xlsx")
sheet = workbook["Project budget"]
assert sheet["B4"].value == 2400
assert sheet["B9"].value == "=SUM(B4:B7)"
cached = load_workbook(folder / "numbering-native.xlsx", data_only=True)
assert cached["Project budget"]["B9"].value == 20000
print("Native workspace copy and Excel formula/cached total verified.")
