"""Verify native cross-workbook scenario history and custom-format transfer."""
import argparse
import copy
import hashlib
import json
import posixpath
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile
from verify_scenarios import R, S
from verify_native_scenarios import without_xml


def verify(folder):
    paths = sorted(folder.glob('[0-9][0-9]-*.json'))
    assert len(paths) == 15
    states = [json.loads(p.read_text()) for p in paths]
    assert [s['revision'] for s in states] == [1, 1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 9, 10, 10]
    assert paths[0].read_bytes() == paths[1].read_bytes() == paths[2].read_bytes()
    assert paths[10].read_bytes() == paths[11].read_bytes() == paths[12].read_bytes()
    assert paths[13].read_bytes() == paths[14].read_bytes()
    base = states[0]
    assert [s['name'] for s in base['sheets']] == ['Inputs', 'Result']
    assert base['sheets'][0]['cells'] == {'A1': '10', 'B1': '20', 'C1': '=A1*B1'}
    assert base['sheets'][1]['cells'] == {'A1': '=Inputs!C1'}
    assert base['sheets'][0]['cellStyles']['B1']['numFmt'] == '0.00000'
    item = copy.deepcopy(states[3]['sheets'][0]['scenarios']['items'][0])
    simple = without_xml(item)
    portable = simple['inputs'][0].pop('_numberFormat')
    assert ET.fromstring(portable).attrib == {'numFmtId': '164', 'formatCode': '0.0000'}
    assert simple == {'name': 'Formatted', 'comment': 'sample', 'locked': True, 'hidden': False,
                      'inputs': [{'address': 'A1', 'value': '7', 'numFmtId': 164}, {'address': 'B1', 'value': '9'}]}
    assert ET.fromstring(item['_xml']).attrib == {'name': 'Formatted', 'comment': 'sample', 'count': '2', 'locked': '1', 'hidden': '0'}
    for i, val in enumerate(['7', '9']):
        node = ET.fromstring(item['inputs'][i]['_xml'])
        assert node.attrib['r'] == ['A1', 'B1'][i] and node.attrib['val'] == val
    duplicate = copy.deepcopy(item)
    duplicate['name'] = 'Formatted (2)'
    summary = {
        'id': states[10]['sheets'][2]['id'], 'name': 'Scenario Summary',
        'cells': {'A1': 'Scenario Summary', 'A2': 'Static values for Inputs', 'A4': 'Changing cells',
                  'A5': "'A1", 'A6': "'B1", 'A8': 'Result cells', 'A9': "'C1"},
        'columnWidths': {'1': 24, '2': 22},
        'frozenPane': {'columns': 1, 'rows': 3, 'state': 'frozen', 'topLeftCell': 'B4'},
        'cellStyles': {},
        'rowHeightModes': {str(r): 'auto' for r in [1, 2, 3, 4, 5, 6, 8, 9]},
        'rowHeights': {str(r): 24.0 for r in [1, 2, 3, 4, 5, 6, 8, 9]},
    }
    for col, name in zip('BCD', ['Current values', 'Formatted', 'Formatted (2)']):
        summary['cells'].update({col+'3': "'"+name, col+'5': '7.0', col+'6': '9.0', col+'9': '63.0'})
        summary['cellStyles'][col+'3'] = {'bold': True}
        summary['cellStyles'][col+'6'] = copy.deepcopy(base['sheets'][0]['cellStyles']['B1'])
    for i, actual in enumerate(states[:13]):
        expected = copy.deepcopy(base)
        expected['revision'] = actual['revision']
        source = expected['sheets'][0]
        if i >= 3:
            source['scenarios'] = {'items': [copy.deepcopy(item)]}
            if i == 7 or i >= 9:
                source['scenarios']['items'].append(copy.deepcopy(duplicate))
        if i == 4 or i >= 6:
            source['cells'].update(A1='7', B1='9')
            source['scenarios'].update(current=0, show=0)
            for sheet in expected['sheets']:
                sheet.setdefault('rowHeights', {})['1'] = 24.0
                sheet.setdefault('rowHeightModes', {})['1'] = 'auto'
        if i >= 10:
            source['scenarios']['resultCells'] = 'C1'
            expected['sheets'].append(copy.deepcopy(summary))
        assert actual == expected, paths[i].name

    imported = states[13]
    expected = copy.deepcopy(states[12])
    for key in ['revision', 'officeWorkbook', 'officePackages']:
        expected[key] = imported[key]
    for old, new in zip(expected['sheets'][:2], imported['sheets'][:2]):
        old['cached'] = new['cached']
        if 'scenarios' in old:
            for scenario in old['scenarios']['items']:
                scenario['inputs'][0].pop('_numberFormat')
                scenario['inputs'][0]['numFmtId'] = 165
        assert without_xml(old) == without_xml(new)
    assert imported['sheets'][0]['cached'] == {'A1': 7.0, 'B1': 9.0, 'C1': 63.0}
    assert imported['sheets'][1]['cached'] == {'A1': 63.0}
    new = imported['sheets'][2]
    expected_summary = copy.deepcopy(summary)
    expected_summary['id'] = 'office-3'
    expected_summary['cells'] = {a: v[1:] if v.startswith("'") else v for a, v in summary['cells'].items()}
    normal = imported['sheets'][0]['styleDefaults']['default']
    expected_summary['cellStyles'] = {a: dict(normal, **v) for a, v in summary['cellStyles'].items()}
    expected_summary.update({
        'autoFilter': None, 'conditionalFormats': [], 'conditionalRightToLeft': False,
        'dataValidations': [], 'extendedConditionalFormats': False, 'extendedDataValidations': False,
        'extendedValidationRangeUnknown': False, 'extendedValidationRanges': [], 'hiddenColumns': [], 'hiddenRows': [],
        'tables': [], 'usedColumns': 4, 'usedRows': 9, 'officePath': 'xl/worksheets/folio-3.xml',
        'officeStyles': {col+str(row): str(style) for col in 'BCD' for row, style in [(3, 2), (6, 1)]},
        'styleDefaults': imported['sheets'][0]['styleDefaults'],
        'cached': {a: float(v) if v.replace('.', '', 1).isdigit() else '' for a, v in expected_summary['cells'].items()},
    })
    assert new == expected_summary
    expected['sheets'] = imported['sheets']
    assert imported == expected  # Document and unrelated top-level content preserved.

    for filename in ['donor.xlsx', 'source.xlsx', 'merged-checked.xlsx']:
        donor = filename == 'donor.xlsx'
        with ZipFile(folder/filename) as z:
            assert z.testzip() is None
            styles = ET.fromstring(z.read('xl/styles.xml'))
            formats = {n.attrib['numFmtId']: n.attrib['formatCode'] for n in styles.find(S+'numFmts')}
            assert formats == ({'164': '0.0000'} if donor else {'164': '0.00000', '165': '0.0000'})
            rels = {n.attrib['Id']: n.attrib['Target'] for n in ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))}
            sheets = {}
            for n in ET.fromstring(z.read('xl/workbook.xml')).find(S+'sheets'):
                t = rels[n.attrib[R+'id']]
                part = t[1:] if t.startswith('/') else posixpath.normpath(posixpath.join('xl', t))
                sheets[n.attrib['name']] = ET.fromstring(z.read(part))
            assert list(sheets) == (['Inputs', 'Result'] if donor else ['Inputs', 'Result', 'Scenario Summary'])
            def cells(name):
                return {n.attrib['r']: n for n in sheets[name].findall(S+'sheetData/'+S+'row/'+S+'c')}
            inputs = cells('Inputs')
            for a, value in zip(['A1', 'B1', 'C1'], [10, 20, 200] if donor else [7, 9, 63]):
                assert float(inputs[a].findtext(S+'v')) == value
            assert inputs['C1'].findtext(S+'f') == 'A1*B1'
            assert cells('Result')['A1'].findtext(S+'f') == 'Inputs!C1'
            assert float(cells('Result')['A1'].findtext(S+'v')) == (200 if donor else 63)
            group = sheets['Inputs'].find(S+'scenarios')
            assert [n.attrib['name'] for n in group] == (['Formatted'] if donor else ['Formatted', 'Formatted (2)'])
            for scenario in group:
                assert scenario.attrib['comment'] == 'sample'
                assert scenario.attrib['locked'] == '1' and scenario.attrib['hidden'] == '0'
                assert [(n.attrib['r'], n.attrib['val']) for n in scenario] == [('A1', '7'), ('B1', '9')]
                assert list(scenario)[0].attrib['numFmtId'] == ('164' if donor else '165')
            if not donor:
                assert group.attrib['current'] == group.attrib['show'] == '0'
                assert group.attrib['sqref'] == 'C1'
                xf = list(styles.find(S+'cellXfs'))[int(inputs['B1'].attrib['s'])]
                assert xf.attrib['numFmtId'] == '164'
                for col in 'BCD':
                    assert cells('Scenario Summary')[col+'9'].find(S+'f') is None
                    assert float(cells('Scenario Summary')[col+'9'].findtext(S+'v')) == 63
    fixtures = json.loads((folder.parent/'scenarios-merge-native-fixtures.json').read_text())
    assert hashlib.sha256((folder/'donor.xlsx').read_bytes()).hexdigest() == fixtures['donor.xlsx']
    assert (folder/'source.xlsx').read_bytes() == (folder/'merged-checked.xlsx').read_bytes()
    return {'passed': True, 'snapshots': len(paths), 'previewCancelByteIdentical': True,
            'copyChangesOnlyScenarioDefinitions': True, 'applyAndCopyUndoRedoVerified': True,
            'restartRecoveryByteIdentical': True, 'reimportAndInspectByteIdentical': True,
            'donorUnchanged': True, 'destinationFormatPreserved': '0.00000',
            'copiedFormatCode': '0.0000', 'copiedFormatRemappedFrom': 164, 'copiedFormatRemappedTo': 165,
            'duplicateCopiesReuseFormatRecord': True, 'summaryResults': [63, 63, 63],
            'exportAndInPlaceSaveByteIdentical': True, 'documentAndFormulasPreserved': True,
            'microsoftExcelExecuted': False,
            'sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(folder.iterdir()) if p.is_file()}}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    print(json.dumps(verify(parser.parse_args().folder), indent=2))
