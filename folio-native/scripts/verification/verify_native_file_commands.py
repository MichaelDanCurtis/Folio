#!/usr/bin/env python3
"""Read saved artifacts from the isolated packaged File menu exercise."""
from pathlib import Path
import hashlib, json
from docx import Document
from openpyxl import load_workbook
project = Path(__file__).resolve().parents[2]
work = project.parents[1]/'work'
root = work/'native-file-commands'
read = lambda name: json.loads((root/f'{name}.folio').read_text())
original, imported = read('before-open'), read('imported')
saved, snapshot = read('native-file-copy'), read('snapshot-before-new')
assert saved == imported == snapshot
assert read('new')['id'] != imported['id']
assert imported['sheets'] == original['sheets']
assert original['document']['content'][0]['content'][0]['text'].startswith('Fresh Good ideas')
assert read('undone')['sheets'] == imported['sheets']
assert (root/'before-restart.folio').read_bytes() == (root/'after-restart.folio').read_bytes()
final = read('after-restart')
assert final['document'] == imported['document']
assert final['sheets'][0]['cells']['B4'] == '4567'
assert final['revision'] == 7
book = load_workbook(root/'native-file-workbook.xlsx')
assert book.active['B4'].value == 4567
assert book.active['B9'].value == '=SUM(B4:B7)'
cache = load_workbook(root/'native-file-workbook.xlsx',data_only=True)
assert cache.active['B9'].value == 22167
word = Document(root/'native-file-document.docx')
assert word.paragraphs[0].text == 'Direct level one 😀'
assert word.paragraphs[0].style.name == 'Outline body'
assert word.tables[0].cell(0,0).text == 'Cell structure'
user = Path.home()/'Library/Containers/com.folio.native/Data/Library/Application Support/com.folio.native/FolioNative/workspace.folio'
assert user.read_bytes() == (root/'user-before-check.folio').read_bytes()
report = {'build':'11','nativeSaveCopyAndSnapshotExact':True,
          'nativeNewAndReopen':True,'nativePostExportHistory':True,
          'nativeCellCommitAndXlsxCache':True,'independentDocxRead':True,
          'restartByteIdentical':True,'finalRevision':7,
          'userWorkspaceUnchanged':True,'microsoftOfficeExecution':False}
(root/'verification.json').write_text(json.dumps(report,indent=2)+'\n')
p=work/'file-commands-package-verification.json'; verification=json.loads(p.read_text())
verification.update(nativeStartupVerified=True,nativeFileCommandsQualified=True,
                    nativeLastRevision=7,userWorkspaceUnchanged=True)
p.write_text(json.dumps(verification,indent=2)+'\n')
print(json.dumps(report,indent=2))
