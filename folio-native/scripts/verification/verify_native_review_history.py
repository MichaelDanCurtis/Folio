"""Read saved native history artifacts without using Folio's engine."""
import argparse, hashlib, json, zipfile, posixpath
from pathlib import Path
from decimal import Decimal
from xml.etree import ElementTree as ET

p = argparse.ArgumentParser()
p.add_argument('folder', type=Path)
root = p.parse_args().folder
read = lambda name: json.loads((root / (name + '.folio')).read_text())
base = read('baseline')
assert base['revision'] == 2
assert base['officeDocument']['tracking']['author'] == 'Morgan'
for name, revision in [('post-save-undo', 4), ('menu-restored', 8), ('cell-undone', 10)]:
    value = read(name)
    assert value['revision'] == revision, name
    for key in ['document', 'officeDocument', 'officePackages', 'sheets', 'title']:
        assert value[key] == base[key], (name, key)
rejected = read('rejected')
assert rejected['revision'] == 3
section = rejected['document']['attrs']['pageLayout']['sections'][0]
assert section['geometry']['gutter'] == 0 and not section['hasTrackedSettings']
committed = read('cell-committed')
assert committed['revision'] == 9
assert committed['sheets'][0]['cells']['B4'] == '4321'
assert committed['document'] == base['document']
assert (root/'before-restart.folio').read_bytes() == (root/'after-restart.folio').read_bytes()
final = read('after-restart')
assert final['revision'] == 11 and final['sheets'] == committed['sheets']
assert final['document'] == base['document']
W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
def document(name):
    with zipfile.ZipFile(root/name) as z:
        return ET.fromstring(z.read('word/document.xml')), z.read('word/header1.xml')
source, header = document('history-source.docx')
for name, gutter, tracked in [('history-rejected.docx','0',False), ('history-restored.docx','360',True)]:
    doc, other_header = document(name)
    assert other_header == header
    assert [e.text for e in doc.iter(W+'t')] == [e.text for e in source.iter(W+'t')]
    section = doc.find('.//'+W+'pPr/'+W+'sectPr')
    assert section.find(W+'pgMar').get(W+'gutter') == gutter
    assert (section.find(W+'sectPrChange') is not None) == tracked
S = '{http://schemas.openxmlformats.org/spreadsheetml/2006/main}'
with zipfile.ZipFile(root/'history-workbook.xlsx') as z:
    relns = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'
    book = ET.fromstring(z.read('xl/workbook.xml'))
    selected = next(s for s in book.iter(S+'sheet') if s.get('name') == 'Project budget')
    relations = ET.fromstring(z.read('xl/_rels/workbook.xml.rels'))
    target = next(r.get('Target') for r in relations if r.get('Id') == selected.get(relns+'id'))
    path = target.lstrip('/') if target.startswith('/') else posixpath.normpath(posixpath.join('xl',target))
    sheet = ET.fromstring(z.read(path))
    cells = {c.get('r'):c for c in sheet.iter(S+'c')}
    for ref, expected in [('B4','4321'), ('D4','2471'), ('B9','21921'), ('D9','7171')]:
        assert Decimal(cells[ref].find(S+'v').text) == Decimal(expected), ref
    assert cells['D4'].find(S+'f') is not None
report = {'passed':True, 'revision':11, 'reviewHistoryExact':True,
          'draftAndDialogWorkspaceUnchanged':True, 'cellUndoExact':True,
          'restartByteIdentical':True, 'independentOfficeRead':True,
          'recoverySha256':hashlib.sha256((root/'after-restart.folio').read_bytes()).hexdigest(),
          'boundary':'Checks retained artifacts; native interactions are separately observed. Does not execute Microsoft Office.'}
print(json.dumps(report, indent=2))
