"""Independently inspect the disposable formula-audit recovery and XLSX fixture."""
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
    recovery = json.loads((folder / 'workspace.folio').read_text())
    by_name = {s['name']: s for s in recovery['sheets']}
    assert set(by_name) == {'Forecast', 'Sources'}
    assert by_name['Forecast']['cells'] == {
        'A1': 'Total', 'C1': '=SUM(Sources!B2:B4)', 'D1': '=INDIRECT("C1")',
    }
    assert by_name['Sources']['cells'] == {
        'A1': 'Values', 'B2': '10', 'B3': '20', 'B4': '30', 'F1': '=Forecast!C1',
    }
    with ZipFile(folder / 'audit.xlsx') as z:
        assert z.testzip() is None
        rels = {e.attrib['Id']: e.attrib['Target'] for e in ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))}
        sheets = {}
        for s in ET.fromstring(z.read('xl/workbook.xml')).find(S+'sheets'):
            target = rels[s.attrib[R+'id']]
            path = posixpath.normpath(posixpath.join('xl', target)) if not target.startswith('/') else target[1:]
            root = ET.fromstring(z.read(path))
            sheets[s.attrib['name']] = {c.attrib['r']: c for c in root.findall('.//'+S+'sheetData/'+S+'row/'+S+'c')}
        expected = sum([10, 20, 30])
        forecast, sources = sheets['Forecast'], sheets['Sources']
        assert forecast['C1'].findtext(S+'f') == 'SUM(Sources!B2:B4)'
        assert forecast['D1'].findtext(S+'f') == 'INDIRECT("C1")'
        assert sources['F1'].findtext(S+'f') == 'Forecast!C1'
        for c in [forecast['C1'], forecast['D1'], sources['F1']]:
            assert float(c.findtext(S+'v')) == expected
        for address, value in [('B2',10),('B3',20),('B4',30)]:
            assert float(sources[address].findtext(S+'v')) == value
    return {'passed': True, 'sheets': 2, 'formulas': 3, 'expectedTotal': expected,
            'undoRestoredSourceInputs': True, 'recoveryFormulasPreserved': True,
            'xlsxFormulasAndCachedValuesPreserved': True,
            'sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(folder.iterdir()) if p.is_file()}}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
