"""Independent recovery/OOXML checks for the disposable Watch Window UI fixture."""
import argparse
import hashlib
import json
import posixpath
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

S = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'


def verify(folder):
    saved = json.loads((folder / 'workspace.folio').read_text())
    sheets = {s['name']: s for s in saved['sheets']}
    expected = {'Forecast': {'C1': True}, 'Sources': {'B4': True}}
    assert {n: s['cellWatches'] for n, s in sheets.items()} == expected
    assert sheets['Forecast']['cells']['C1'] == '=SUM(Sources!B3:B5)'
    assert [sheets['Sources']['cells'][a] for a in ['B3', 'B4', 'B5']] == ['10', '25', '30']
    with ZipFile(folder / 'watches.xlsx') as z:
        assert z.testzip() is None
        rels = {n.attrib['Id']: n.attrib['Target'] for n in ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))}
        for s in ET.fromstring(z.read('xl/workbook.xml')).find(S+'sheets'):
            name = s.attrib['name']
            target = rels[s.attrib[R+'id']]
            path = target[1:] if target.startswith('/') else posixpath.normpath(posixpath.join('xl', target))
            root = ET.fromstring(z.read(path))
            watches = root.findall(S+'cellWatches/'+S+'cellWatch')
            assert [n.attrib['r'] for n in watches] == list(expected[name])
            cells = {c.attrib['r']: c for c in root.findall(S+'sheetData/'+S+'row/'+S+'c')}
            if name == 'Forecast':
                assert cells['C1'].findtext(S+'f') == 'SUM(Sources!B3:B5)'
                assert float(cells['C1'].findtext(S+'v')) == 65
            else:
                assert [float(cells[a].findtext(S+'v')) for a in ['B3','B4','B5']] == [10,25,30]
    return {'passed': True, 'watchedCells': expected, 'calculatedTotal': 65,
            'structuralReferencesMatch': True, 'recoveryAndXlsxWatchesMatch': True,
            'sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(folder.iterdir()) if p.is_file()}}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
