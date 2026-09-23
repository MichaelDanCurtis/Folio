"""Verify native Scenario Manager checkpoints and the saved OOXML independently."""
import argparse
import copy
import hashlib
import json
import posixpath
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

from verify_scenarios import R, S


def without_xml(value):
    if isinstance(value, dict):
        return {k: without_xml(v) for k, v in value.items() if k != '_xml'}
    if isinstance(value, list):
        return [without_xml(v) for v in value]
    return value


def verify(folder):
    paths = sorted(folder.glob('[0-9][0-9]-*.json'))
    assert len(paths) == 18
    states = [json.loads(p.read_text()) for p in paths]
    assert [s['revision'] for s in states] == [1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 12, 12, 13, 14, 15]
    assert paths[0].read_bytes() == paths[1].read_bytes()
    assert paths[12].read_bytes() == paths[13].read_bytes() == paths[14].read_bytes()
    base = states[0]
    assert [s['name'] for s in base['sheets']] == ['Inputs', 'Result', 'Scenario Summary']
    assert base['sheets'][0]['cells'] == {'A1': '15', 'B1': '30', 'C1': '=A1*B1'}
    assert base['sheets'][1]['cells'] == {'A1': '=Inputs!C1'}
    assert base['sheets'][0]['cellWatches'] == {'C1': True}
    assert [s['name'] for s in base['sheets'][0]['scenarios']['items']] == ['Base', 'Upside']
    added = {'name': 'Downside', 'comment': '', 'hidden': False, 'locked': True,
             'inputs': [{'address': 'A1', 'value': '5'}, {'address': 'B1', 'value': '8'}]}
    steady = copy.deepcopy(added)
    steady['name'] = 'Steady'
    steady['inputs'][1]['value'] = '12'
    summary = {
        'id': states[8]['sheets'][3]['id'], 'name': 'Scenario Summary 2',
        'cells': {'A1': 'Scenario Summary', 'A2': 'Static values for Inputs',
                  'A4': 'Changing cells', 'A5': "'A1", 'A6': "'B1", 'A8': 'Result cells', 'A9': "'C1",
                  'B3': "'Current values", 'B5': '5.0', 'B6': '12.0', 'B9': '60.0',
                  'C3': "'Base", 'C5': '10.0', 'C6': '20.0', 'C9': '200.0',
                  'D3': "'Upside", 'D5': '15.0', 'D6': '30.0', 'D9': '450.0',
                  'E3': "'Steady", 'E5': '5.0', 'E6': '12.0', 'E9': '60.0'},
        'cellStyles': {a: {'bold': True} for a in ['B3', 'C3', 'D3', 'E3']},
        'columnWidths': {'1': 24, '2': 22},
        'frozenPane': {'columns': 1, 'rows': 3, 'state': 'frozen', 'topLeftCell': 'B4'},
        'rowHeightModes': {str(r): 'auto' for r in [1, 2, 3, 4, 5, 6, 8, 9]},
        'rowHeights': {str(r): 24.0 for r in [1, 2, 3, 4, 5, 6, 8, 9]},
    }
    assert summary['id'].startswith('scenario-summary-')
    for i, actual in enumerate(states[:15]):
        expected = copy.deepcopy(base)
        expected['revision'] = actual['revision']
        source = expected['sheets'][0]
        if i >= 2:
            source['scenarios']['items'].append(copy.deepcopy(added if i < 6 else steady))
        if i == 3 or i >= 5:
            source['cells'].update(A1='5', B1='8' if i < 7 else '12')
            source['scenarios'].update(current=2, show=2)
        if i >= 8:
            expected['sheets'].append(copy.deepcopy(summary))
        if i >= 9:
            copied = [copy.deepcopy(source['scenarios']['items'][0]), copy.deepcopy(steady)]
            expected['sheets'][1]['scenarios'] = {'items': copied if i in (9, 11) else [copied[1]]}
        assert actual == expected, paths[i].name

    imported = states[15]
    expected = copy.deepcopy(states[14])
    # Import regenerates package records and source caches; content must survive.
    for key in ['revision', 'officePackages', 'officeWorkbook']:
        expected[key] = imported[key]
    for old, new in zip(expected['sheets'][:3], imported['sheets'][:3]):
        old['cached'] = new['cached']
        assert without_xml(old) == without_xml(new)
    assert imported['sheets'][0]['cached'] == {'A1': 5.0, 'B1': 12.0, 'C1': 60.0}
    assert imported['sheets'][1]['cached'] == {'A1': 60.0}
    assert imported['sheets'][2] == states[14]['sheets'][2]
    new_summary = imported['sheets'][3]
    assert new_summary['id'] == 'office-4'
    assert new_summary['cells'] == {a: v[1:] if v.startswith("'") else v for a, v in summary['cells'].items()}
    for key in ['name', 'columnWidths', 'frozenPane', 'rowHeights', 'rowHeightModes']:
        assert new_summary[key] == summary[key]
    normal = imported['sheets'][0]['styleDefaults']['default']
    assert new_summary['cellStyles'] == {a: dict(normal, bold=True) for a in summary['cellStyles']}
    assert new_summary['styleDefaults'] == imported['sheets'][0]['styleDefaults']
    for key in ['conditionalFormats', 'dataValidations', 'extendedValidationRanges', 'hiddenColumns', 'hiddenRows', 'tables']:
        assert new_summary[key] == []
    for key in ['conditionalRightToLeft', 'extendedConditionalFormats', 'extendedDataValidations', 'extendedValidationRangeUnknown']:
        assert new_summary[key] is False
    assert new_summary['autoFilter'] is None
    assert (new_summary['usedColumns'], new_summary['usedRows']) == (5, 9)
    assert new_summary['officePath'] == 'xl/worksheets/folio-4.xml'
    assert new_summary['officeStyles'] == {a: '1' for a in summary['cellStyles']}
    assert new_summary['cached'] == {a: float(v) if v.replace('.', '', 1).isdigit() else '' for a, v in new_summary['cells'].items()}
    assert set(new_summary) == set(summary) | {'autoFilter', 'cached', 'conditionalFormats', 'conditionalRightToLeft', 'dataValidations', 'extendedConditionalFormats', 'extendedDataValidations', 'extendedValidationRangeUnknown', 'extendedValidationRanges', 'hiddenColumns', 'hiddenRows', 'officePath', 'officeStyles', 'styleDefaults', 'tables', 'usedColumns', 'usedRows'}
    expected['sheets'] = imported['sheets']
    assert imported == expected  # Unrelated document and top-level data unchanged.
    for i in [16, 17]:
        expected = copy.deepcopy(imported)
        expected['revision'] = states[i]['revision']
        if i == 16:
            expected['sheets'][0]['cells'].update(A1='10', B1='20')
            expected['sheets'][0]['scenarios'].update(current=0, show=0)
        assert states[i] == expected, paths[i].name

    for filename in ['source.xlsx', 'scenarios-checked.xlsx']:
        with ZipFile(folder / filename) as z:
            assert z.testzip() is None
            rels = {n.attrib['Id']: n.attrib['Target'] for n in ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))}
            sheets = {}
            for n in ET.fromstring(z.read('xl/workbook.xml')).find(S + 'sheets'):
                target = rels[n.attrib[R + 'id']]
                part = target[1:] if target.startswith('/') else posixpath.normpath(posixpath.join('xl', target))
                sheets[n.attrib['name']] = ET.fromstring(z.read(part))
            assert list(sheets) == ['Inputs', 'Result', 'Scenario Summary', 'Scenario Summary 2']
            def cells(name):
                return {c.attrib['r']: c for c in sheets[name].findall(S + 'sheetData/' + S + 'row/' + S + 'c')}
            for name in sheets:
                watches = [n.attrib['r'] for n in sheets[name].findall(S + 'cellWatches/' + S + 'cellWatch')]
                assert watches == (['C1'] if name == 'Inputs' else [])
            for address, value in {'A1': 5, 'B1': 12, 'C1': 60}.items():
                assert float(cells('Inputs')[address].findtext(S + 'v')) == value
            assert cells('Inputs')['C1'].findtext(S + 'f') == 'A1*B1'
            assert cells('Result')['A1'].findtext(S + 'f') == 'Inputs!C1'
            assert float(cells('Result')['A1'].findtext(S + 'v')) == 60
            for name, names, values in [('Inputs', ['Base', 'Upside', 'Steady'], [('10', '20'), ('15', '30'), ('5', '12')]), ('Result', ['Steady'], [('5', '12')])]:
                group = sheets[name].find(S + 'scenarios')
                if name == 'Inputs':
                    assert group.attrib['current'] == group.attrib['show'] == '2'
                    assert group.attrib['sqref'] == 'C1'
                assert [n.attrib['name'] for n in group] == names
                for scenario, pair in zip(group, values):
                    assert scenario.attrib == {'name': scenario.attrib['name'], 'count': '2', 'comment': '', 'locked': '1', 'hidden': '0'}
                    assert [(c.attrib['r'], c.attrib['val']) for c in scenario] == list(zip(['A1', 'B1'], pair))
            for name, values in [('Scenario Summary', [450, 200, 450]), ('Scenario Summary 2', [60, 200, 450, 60])]:
                for col, value in zip('BCDE', values):
                    cell = cells(name)[col + '9']
                    assert cell.find(S + 'f') is None
                    assert float(cell.findtext(S + 'v')) == value
    assert (folder / 'source.xlsx').read_bytes() == (folder / 'scenarios-checked.xlsx').read_bytes()
    return {'passed': True, 'snapshots': len(paths), 'cancelByteIdentical': True,
            'historyAndUnrelatedContentVerified': True, 'definitionEditsDoNotApply': True,
            'showUndoRedoVerified': True, 'mergeAndDeleteUndoRedoVerified': True,
            'restartRecoveryByteIdentical': True, 'reimportShowUndoVerified': True,
            'exportAndInPlaceSaveByteIdentical': True, 'formulaDocumentAndWatchPreserved': True,
            'summaryCurrentAndScenarioResults': [60, 200, 450, 60],
            'microsoftExcelExecuted': False,
            'sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(folder.iterdir()) if p.is_file()}}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
