"""Check Goal Seek's disposable UI recovery and OOXML outputs independently."""
import argparse
import hashlib
import json
import math
import posixpath
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

S = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'


def close(value, expected, tolerance=1e-6):
    assert math.isfinite(float(value)) and abs(float(value)-expected) <= tolerance, (value, expected)


def verify(folder):
    before = json.loads((folder/'before.folio').read_text())
    saved = json.loads((folder/'workspace.folio').read_text())
    old = {s['name']: s for s in before['sheets']}
    sheets = {s['name']: s for s in saved['sheets']}
    assert set(sheets) == set(old) == {'Summary', 'Inputs'}
    assert before['document'] == saved['document']
    assert before['id'] == saved['id'] and before['title'] == saved['title']
    assert saved['revision'] == before['revision']+3  # Apply, Undo, Redo.
    for name, sheet in sheets.items():
        expected = dict(old[name]['cells'])
        if name == 'Inputs':
            close(sheet['cells']['B3'], 50)
            expected['B3'] = sheet['cells']['B3']
        assert sheet['cells'] == expected
        assert {k: v for k, v in sheet.items() if k not in {'cells', 'rowHeights', 'rowHeightModes'}} == {
            k: v for k, v in old[name].items() if k not in {'cells', 'rowHeights', 'rowHeightModes'}}
    with ZipFile(folder/'goal-seek.xlsx') as z:
        assert z.testzip() is None
        rels = {n.attrib['Id']: n.attrib['Target'] for n in ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))}
        for sheet in ET.fromstring(z.read('xl/workbook.xml')).find(S+'sheets'):
            name = sheet.attrib['name']
            target = rels[sheet.attrib[R+'id']]
            part = target[1:] if target.startswith('/') else posixpath.normpath(posixpath.join('xl',target))
            root = ET.fromstring(z.read(part))
            cells = {n.attrib['r']:n for n in root.findall(S+'sheetData/'+S+'row/'+S+'c')}
            assert [n.attrib['r'] for n in root.findall(S+'cellWatches/'+S+'cellWatch')] == (['C1'] if name == 'Summary' else ['B3'])
            if name == 'Summary':
                assert cells['C1'].findtext(S+'f') == 'Inputs!B2*Inputs!B3'
                close(cells['C1'].findtext(S+'v'),500)
            else:
                close(cells['B2'].findtext(S+'v'),10)
                close(cells['B3'].findtext(S+'v'),50)
    return {'passed':True,'originalInput':20,'inputWithinToleranceOf':50,'resultWithinToleranceOf':500,
        'absoluteTolerance':1e-6,'formulasDocumentsAndWatchesPreserved':True,
        'historyRevisionDelta':3,'microsoftExcelExecuted':False,
        'sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(folder.iterdir()) if p.is_file()}}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder',type=Path)
    print(json.dumps(verify(parser.parse_args().folder),indent=2))
