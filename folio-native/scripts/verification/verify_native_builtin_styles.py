"""Independently verify retained native build 9 heading artifacts."""
from pathlib import Path
from hashlib import sha256
import json, shutil
from docx import Document
from docx.oxml.ns import qn

root = Path(__file__).resolve().parents[4] / 'work'
folder = root / 'native-builtin-styles'
meta_path = root / 'builtin-styles-package-verification.json'
meta = json.loads(meta_path.read_text())
shutil.copy2(meta['previewRecovery'], folder / 'after-restart.folio')
user = Path.home() / 'Library/Containers/com.folio.native/Data/Library/Application Support/com.folio.native/FolioNative/workspace.folio'
shutil.copy2(user, folder / 'user-after-check.folio')
read = lambda name: json.loads((folder / (name + '.folio')).read_text())
before, applied, cell = (read(name) for name in ['before', 'applied', 'cell-applied'])
assert (folder/'before.folio').read_bytes() == (folder/'cancelled.folio').read_bytes()
catalog = lambda state: state['document']['attrs']['wordStyles']['styles']
assert not any(s['id'] in ['Heading7', 'Heading9'] for s in catalog(before))
heading = applied['document']['content'][0]
assert heading['type'] == 'heading' and heading['attrs']['level'] == 9
assert heading['attrs']['paragraphFormat']['styleId'] == 'Heading9'
assert heading['content'] == before['document']['content'][0]['content']
assert applied['document']['content'][1:] == before['document']['content'][1:]
assert [s['id'] for s in catalog(applied) if s['id'] not in {s['id'] for s in catalog(before)}] == ['Heading9']
assert {s['id'] for s in catalog(cell)} - {s['id'] for s in catalog(applied)} == {'Heading7'}
assert (folder/'redone.folio').read_bytes() == (folder/'cell-draft.folio').read_bytes()
for name, rev, expected in [('undone',64,before),('redone',65,applied),
                           ('cell-undone',67,applied),('cell-redone',68,cell),
                           ('post-save-undo',69,applied),('before-restart',70,cell),
                           ('after-restart',70,cell)]:
    state=read(name)
    assert state['revision']==rev, name
    assert state['document']==expected['document'], name
    assert state['sheets']==before['sheets'], name
assert (folder/'before-restart.folio').read_bytes()==(folder/'after-restart.folio').read_bytes()
assert (folder/'user-before-check.folio').read_bytes()==(folder/'user-after-check.folio').read_bytes()
d=Document(folder/'native-builtin-styles.docx')
assert d.paragraphs[0].text=='Heading candidate'
assert d.paragraphs[0].style.style_id=='Heading9'
assert d.paragraphs[0].runs[0].bold is False
assert d.paragraphs[1].text=='Neighbor stays unchanged.'
assert d.tables[0].cell(0,0).text=='Cell candidate'
assert d.tables[0].cell(0,0).paragraphs[0].style.style_id=='Heading7'
for level in [7,9]:
    s=next(s for s in d.styles if s.style_id==f'Heading{level}')
    assert s.builtin and s.base_style.style_id=='Normal'
    assert s.next_paragraph_style.style_id=='Normal'
    assert s.element.pPr.find(qn('w:outlineLvl')).get(qn('w:val'))==str(level-1)
assert not d.styles['Project body'].builtin
report={'build':'9','nativeCancelUnchanged':True,'nativeMissingHeadingApplied':True,
        'nativeTableDraftAndApply':True,'nativeKeyboardUndoRedo':True,
        'nativePostSaveHistory':True,'independentDocxRead':True,
        'restartByteIdentical':True,'finalRevision':70,'userWorkspaceUnchanged':True,
        'userWorkspaceSha256':sha256(user.read_bytes()).hexdigest(),
        'microsoftWordExecution':False}
(folder/'verification.json').write_text(json.dumps(report,indent=2)+'\n')
meta.update(nativeStartupVerified=True,nativeScopedBuiltinStylesQualified=True,
            nativeLastRevision=70,userWorkspaceUnchanged=True)
meta_path.write_text(json.dumps(meta,indent=2)+'\n')
print(json.dumps(report,indent=2))
