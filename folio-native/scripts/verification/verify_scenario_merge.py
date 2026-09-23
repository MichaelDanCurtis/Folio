"""Independently verify the real-app cross-workbook scenario merge artifacts."""
import argparse
import copy
import hashlib
import json
import posixpath
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

from verify_scenarios import R, S


def verify(folder):
    before = json.loads((folder / 'before.folio').read_text())
    merged = json.loads((folder / 'merged.folio').read_text())
    applied = json.loads((folder / 'workspace.folio').read_text())
    expected = copy.deepcopy(before)
    expected['revision'] += 1
    scenario = merged['sheets'][0]['scenarios']
    assert set(scenario) == {'items'}
    assert len(scenario['items']) == 1
    item = scenario['items'][0]
    assert item['name'] == 'Sales'
    assert item['comment'] == '' and item['locked'] is True and item['hidden'] is False
    assert [(v['address'], v['value']) for v in item['inputs']] == [('A1', '12'), ('B1', '25')]
    expected['sheets'][0]['scenarios'] = scenario
    assert merged == expected
    expected['revision'] += 3  # Show, Undo, Redo.
    expected['sheets'][0]['cells'].update(A1='12', B1='25')
    expected['sheets'][0]['scenarios'].update(current=0, show=0)
    # Native row measurement adds automatic fitting for changed rows.
    for key in ['rowHeights', 'rowHeightModes']:
        if key in applied['sheets'][0]:
            expected['sheets'][0][key] = applied['sheets'][0][key]
    assert applied == expected
    for name, values in [('sales.xlsx', (1, 2, 2)), ('merged.xlsx', (12, 25, 300))]:
        with ZipFile(folder / name) as z:
            assert z.testzip() is None
            rels = {n.attrib['Id']: n.attrib['Target'] for n in ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))}
            sheets = list(ET.fromstring(z.read('xl/workbook.xml')).find(S + 'sheets'))
            assert len(sheets) == 1 and sheets[0].attrib['name'] == 'Inputs'
            target = rels[sheets[0].attrib[R + 'id']]
            path = target[1:] if target.startswith('/') else posixpath.normpath(posixpath.join('xl', target))
            root = ET.fromstring(z.read(path))
            cells = {c.attrib['r']: c for c in root.findall(S + 'sheetData/' + S + 'row/' + S + 'c')}
            assert cells['C1'].findtext(S + 'f') == 'A1*B1'
            for address, value in zip(['A1', 'B1', 'C1'], values):
                assert float(cells[address].findtext(S + 'v')) == value
            scenarios = list(root.find(S + 'scenarios'))
            assert len(scenarios) == 1 and scenarios[0].attrib['name'] == 'Sales'
            assert [(n.attrib['r'], n.attrib['val']) for n in scenarios[0]] == [('A1', '12'), ('B1', '25')]
            watches = [n.attrib['r'] for n in root.findall(S + 'cellWatches/' + S + 'cellWatch')]
            assert watches == (['C1'] if name == 'merged.xlsx' else [])
    return {'passed': True, 'mergeChangesOnlyScenarioCollection': True,
            'applyUndoRedoRevisionDelta': 3, 'sourceResult': 2, 'mergedResult': 300,
            'documentFormulaAndWatchPreserved': True, 'microsoftExcelExecuted': False,
            'sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(folder.iterdir()) if p.is_file()}}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
