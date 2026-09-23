"""Independently check captured native Goal Seek history and exported OOXML."""
import argparse
import copy
import hashlib
import json
import posixpath
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

from verify_goal_seek import R, S, close


def verify(folder):
    paths = sorted(folder.glob('[0-9][0-9]-*.json'))
    assert len(paths) == 11
    states = [json.loads(p.read_text()) for p in paths]
    base = states[0]
    assert [x['revision'] for x in states] == [1, 1, 1, 2, 3, 4, 4, 4, 4, 4, 5]
    assert paths[0].read_bytes() == paths[1].read_bytes() == paths[2].read_bytes()
    assert all(p.read_bytes() == paths[5].read_bytes() for p in paths[6:10])
    assert [s['name'] for s in base['sheets']] == ['Summary', 'Inputs']
    close(base['sheets'][1]['cells']['B3'], 50)
    for i, state in enumerate(states):
        expected = copy.deepcopy(base)
        expected['revision'] = state['revision']
        if i in (3, 5, 6, 7, 8, 9, 10):
            value = state['sheets'][1]['cells']['B3']
            close(value, 75)
            expected['sheets'][1]['cells']['B3'] = value
        if i == 10:
            # Reimport replaces source-package metadata and source caches.
            expected['officePackages'] = state['officePackages']
            expected['officeWorkbook'] = state['officeWorkbook']
            for old, new in zip(expected['sheets'], state['sheets']):
                old['cached'] = new['cached']
            close(state['sheets'][0]['cached']['C1'], 750)
            close(state['sheets'][1]['cached']['B3'], 75)
        assert state == expected, paths[i].name
    for filename in ['goal-checked.xlsx', 'source.xlsx']:
        with ZipFile(folder / filename) as z:
            assert z.testzip() is None
            rels = {n.attrib['Id']: n.attrib['Target'] for n in ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))}
            sheets = ET.fromstring(z.read('xl/workbook.xml')).find(S + 'sheets')
            assert [s.attrib['name'] for s in sheets] == ['Summary', 'Inputs']
            for sheet in sheets:
                name = sheet.attrib['name']
                target = rels[sheet.attrib[R + 'id']]
                part = target[1:] if target.startswith('/') else posixpath.normpath(posixpath.join('xl', target))
                root = ET.fromstring(z.read(part))
                cells = {n.attrib['r']: n for n in root.findall(S + 'sheetData/' + S + 'row/' + S + 'c')}
                watches = [n.attrib['r'] for n in root.findall(S + 'cellWatches/' + S + 'cellWatch')]
                assert watches == (['C1'] if name == 'Summary' else ['B3'])
                if name == 'Summary':
                    assert cells['C1'].findtext(S + 'f') == 'Inputs!B2*Inputs!B3'
                    close(cells['C1'].findtext(S + 'v'), 750)
                else:
                    close(cells['B2'].findtext(S + 'v'), 10)
                    close(cells['B3'].findtext(S + 'v'), 75)
    assert (folder/'goal-checked.xlsx').read_bytes() == (folder/'source.xlsx').read_bytes()
    return {
        'passed': True, 'snapshots': len(paths), 'previewAndCancelByteIdentical': True,
        'onlyChosenInputChanged': True, 'undoRestoresOriginalInput': True,
        'restartRecoveryByteIdentical': True, 'exportAndInPlaceSaveByteIdentical': True,
        'targetResult': 750, 'inputResult': 75, 'absoluteTolerance': 1e-6,
        'formulaDocumentAndWatchesPreserved': True, 'microsoftExcelExecuted': False,
        'sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(folder.iterdir()) if p.is_file()},
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
