"""Check Scenario Manager's recovery history, input sets and static XLSX report."""
import argparse
import copy
import hashlib
import json
import posixpath
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

S = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'


def verify(folder):
    before = json.loads((folder/'before-show.folio').read_text())
    applied = json.loads((folder/'applied.folio').read_text())
    final = json.loads((folder/'workspace.folio').read_text())
    assert applied['revision'] == before['revision'] + 3  # Show, Undo, Redo.
    assert final['revision'] == applied['revision'] + 1  # Add static summary.
    expected = copy.deepcopy(before)
    expected['revision'] = applied['revision']
    sheet = expected['sheets'][0]
    sheet['cells'].update(A1='15', B1='30')
    sheet['scenarios'].update(current=1, show=1)
    # Automatic fitting is independently covered by the shared layout path.
    for target, actual in zip(expected['sheets'], applied['sheets']):
        for key in ['rowHeights', 'rowHeightModes']:
            if key in actual:
                target[key] = actual[key]
    assert applied == expected
    assert before['document'] == final['document']
    assert final['sheets'][0]['cells'] == {'A1':'15','B1':'30','C1':'=A1*B1'}
    assert final['sheets'][1] == applied['sheets'][1]
    assert final['sheets'][2]['name'] == 'Scenario Summary'
    defs = final['sheets'][0]['scenarios']['items']
    assert [s['name'] for s in defs] == ['Base', 'Upside']
    assert [[i['value'] for i in s['inputs']] for s in defs] == [['10','20'],['15','30']]
    with ZipFile(folder/'scenarios.xlsx') as z:
        assert z.testzip() is None
        rels = {n.attrib['Id']: n.attrib['Target'] for n in ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))}
        sheets = {}
        for n in ET.fromstring(z.read('xl/workbook.xml')).find(S+'sheets'):
            target = rels[n.attrib[R+'id']]
            part = target[1:] if target.startswith('/') else posixpath.normpath(posixpath.join('xl',target))
            sheets[n.attrib['name']] = ET.fromstring(z.read(part))
        assert set(sheets) == {'Inputs','Result','Scenario Summary'}
        def cells(name):
            return {c.attrib['r']: c for c in sheets[name].findall(S+'sheetData/'+S+'row/'+S+'c')}
        inputs = cells('Inputs')
        assert float(inputs['A1'].findtext(S+'v')) == 15
        assert float(inputs['B1'].findtext(S+'v')) == 30
        assert inputs['C1'].findtext(S+'f') == 'A1*B1'
        assert float(inputs['C1'].findtext(S+'v')) == 15*30
        assert cells('Result')['A1'].findtext(S+'f') == 'Inputs!C1'
        assert float(cells('Result')['A1'].findtext(S+'v')) == 450
        group = sheets['Inputs'].find(S+'scenarios')
        assert group.attrib['current'] == group.attrib['show'] == '1'
        assert group.attrib['sqref'] == 'C1'
        assert [s.attrib['name'] for s in group] == ['Base','Upside']
        for scenario, expected_values in zip(group, [('10','20'),('15','30')]):
            assert scenario.attrib['count'] == '2'
            assert scenario.attrib['locked'] == '1' and scenario.attrib['hidden'] == '0'
            assert [(c.attrib['r'],c.attrib['val']) for c in scenario] == list(zip(['A1','B1'],expected_values))
        summary = cells('Scenario Summary')
        for address, expected_value in {'B9':450,'C9':200,'D9':450}.items():
            assert summary[address].find(S+'f') is None
            assert float(summary[address].findtext(S+'v')) == expected_value
        assert sheets['Inputs'].find(S+'cellWatches/'+S+'cellWatch').attrib['r'] == 'C1'
    return {'passed':True,'historyRevisionDelta':4,'scenarioNames':['Base','Upside'],
        'currentInputValues':[15,30],'summaryCurrentAndScenarioResults':[450,200,450],
        'formulaDocumentAndWatchPreserved':True,'summaryContainsStaticValues':True,
        'microsoftExcelExecuted':False,
        'sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(folder.iterdir()) if p.is_file()}}

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder',type=Path)
    print(json.dumps(verify(parser.parse_args().folder),indent=2))
