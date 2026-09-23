"""Independently check the disposable 0.79 native Watch Window artifacts."""
import argparse
import hashlib
import json
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

S = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'


def semantic(node):
    """Compare XML meaning without depending on redundant namespace declarations."""
    return (node.tag, sorted(node.attrib.items()), node.text or '',
            tuple(semantic(child) for child in node))


def verify(folder, original):
    states = {p.stem: json.loads(p.read_text()) for p in folder.glob('[0-9][0-9]-*.json')}
    expected = [
        ('01-imported', 1, 2, '25'),
        ('02-cancelled-edit', 1, 2, '25'),
        ('03-added-range', 2, 3, '25'),
        ('04-removed-watch', 3, 2, '25'),
        ('05-undo-remove', 4, 3, '25'),
        ('06-redo-remove', 5, 2, '25'),
        ('07-restored-and-inspected', 6, 3, '25'),
        ('08-typed-source-edit', 7, 3, '26'),
        ('09-cancelled-keyboard-draft', 7, 3, '26'),
        ('10-final-before-export', 8, 3, '25'),
        ('11-exported', 8, 3, '25'),
        ('12-quit-saved', 8, 3, '25'),
        ('13-restarted', 8, 3, '25'),
        ('14-export-reopened', 9, 3, '25'),
    ]
    assert set(states) == {row[0] for row in expected}
    baseline = states['01-imported']
    for name, revision, count, value in expected:
        data = states[name]
        assert data['revision'] == revision, name
        sheets = {s['name']: s for s in data['sheets']}
        assert set(sheets) == {'Forecast', 'Sources'}, name
        assert data['document'] == baseline['document'], name
        assert sheets['Forecast']['cells'] == {'A1': 'Total', 'C1': '=SUM(Sources!B3:B5)'}, name
        assert sheets['Sources']['cells'] == {'A1': 'Costs', 'B3': '10', 'B4': value, 'B5': '30'}, name
        assert sheets['Forecast']['cellWatches'] == {'C1': True}, name
        assert sheets['Sources']['cellWatches'] == ({'B4': True, 'B5': True} if count == 3 else {'B4': True}), name
        for sheet, initial in zip(data['sheets'], baseline['sheets']):
            assert {k: v for k, v in sheet.items() if k not in {'cells', 'cellWatches'}} == {
                k: v for k, v in initial.items() if k not in {'cells', 'cellWatches'}}, name
    for first, second in [
        ('01-imported', '02-cancelled-edit'),
        ('08-typed-source-edit', '09-cancelled-keyboard-draft'),
        ('10-final-before-export', '11-exported'),
        ('11-exported', '12-quit-saved'),
        ('12-quit-saved', '13-restarted'),
    ]:
        assert (folder / (first+'.json')).read_bytes() == (folder / (second+'.json')).read_bytes()
    exported = folder / 'watch-checked.xlsx'
    assert exported.read_bytes() == (folder / 'source.xlsx').read_bytes()
    changed = []
    with ZipFile(original) as before, ZipFile(exported) as after:
        assert before.testzip() is None and after.testzip() is None
        assert set(before.namelist()) == set(after.namelist())
        for part in before.namelist():
            left, right = before.read(part), after.read(part)
            if left == right:
                continue
            changed.append(part)
            assert part in {'xl/worksheets/folio-1.xml', 'xl/worksheets/folio-2.xml'}, part
            a, b = ET.fromstring(left), ET.fromstring(right)
            if part.endswith('folio-2.xml'):
                watches = b.find(S+'cellWatches')
                assert [n.attrib['r'] for n in watches] == ['B4', 'B5']
                watches.remove(next(n for n in watches if n.attrib['r'] == 'B5'))
            assert semantic(a) == semantic(b), part
        forecast = ET.fromstring(after.read('xl/worksheets/folio-1.xml'))
        c1 = forecast.find('.//'+S+'c[@r="C1"]')
        assert c1.findtext(S+'f') == 'SUM(Sources!B3:B5)'
        assert float(c1.findtext(S+'v')) == 65
    return {
        'passed': True, 'snapshots': len(states), 'changedXlsxParts': changed,
        'onlySemanticXlsxChange': 'Sources!B5 added to cellWatches',
        'typedDraftCancellationByteIdentical': True,
        'restartRecoveryByteIdentical': True,
        'exportAndInPlaceSaveByteIdentical': True,
        'finalWatchedCells': {'Forecast': ['C1'], 'Sources': ['B4', 'B5']},
        'finalCachedTotal': 65,
        'sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                   for p in sorted(folder.iterdir()) if p.is_file()},
        'nativeUiObservations': 'Recorded separately in the qualification report and conversation',
        'microsoftExcelExecuted': False,
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    parser.add_argument('original', type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.folder, args.original), indent=2))
