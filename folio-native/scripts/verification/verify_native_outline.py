"""Independent checks of the packaged macOS outline workflow artifacts."""
from pathlib import Path
from hashlib import sha256
import copy,json,shutil
from docx import Document
from docx.oxml.ns import qn
root=Path(__file__).resolve().parents[4]/'work';folder=root/'native-outline'
meta_path=root/'outline-package-verification.json';meta=json.loads(meta_path.read_text())
shutil.copy2(meta['previewRecovery'],folder/'after-restart.folio')
user=Path.home()/'Library/Containers/com.folio.native/Data/Library/Application Support/com.folio.native/FolioNative/workspace.folio'
shutil.copy2(user,folder/'user-after-check.folio')
read=lambda name:json.loads((folder/(name+'.folio')).read_text())
before=read('before');body=read('body');cleared=read('cleared')
expected=copy.deepcopy(before['document']);p=expected['content'][0]
p['type']='paragraph';p['attrs'].pop('level');p['attrs']['paragraphFormat']['direct']['outlineLevel']=9
assert body['document']==expected
p['attrs']['paragraphFormat']['direct'].pop('outlineLevel')
assert cleared['document']==expected
assert (folder/'before.folio').read_bytes()==(folder/'cancelled.folio').read_bytes()
for name,rev,expected in [('undone',73,before),('redone',74,body),('post-save-undo',76,body),('before-restart',77,cleared),('after-restart',77,cleared)]:
    state=read(name);assert state['revision']==rev,name
    assert state['document']==expected['document'],name
    assert state['sheets']==before['sheets'],name
assert (folder/'before-restart.folio').read_bytes()==(folder/'after-restart.folio').read_bytes()
assert (folder/'user-before-check.folio').read_bytes()==(folder/'user-after-check.folio').read_bytes()
d=Document(folder/'native-outline.docx');p=d.paragraphs[0]
assert p.text=='Direct level one 😀' and p.style.style_id=='Outlinebody'
assert p._p.pPr.find(qn('w:outlineLvl')) is None
assert p.style.font.size.pt==14 and p.runs[0].bold is False
assert d.paragraphs[1].style.style_id=='Heading1'
assert [p._p.pPr.find(qn('w:outlineLvl')).get(qn('w:val')) for p in [d.paragraphs[1],d.paragraphs[2],d.paragraphs[4]]]==['9','8','3']
cell=d.tables[0].cell(0,0).paragraphs[0]
assert cell.text=='Cell structure' and cell.style.style_id=='Outlinebody'
assert cell._p.pPr.find(qn('w:outlineLvl')).get(qn('w:val'))=='7'
report={'build':'10','nativeCancelUnchanged':True,'nativeBodyTextAndClear':True,
        'nativeKeyboardUndoRedo':True,'nativePostSaveHistory':True,
        'independentDocxRead':True,'restartByteIdentical':True,'finalRevision':77,
        'userWorkspaceUnchanged':True,'userWorkspaceSha256':sha256(user.read_bytes()).hexdigest(),
        'microsoftWordExecution':False}
(folder/'verification.json').write_text(json.dumps(report,indent=2)+'\n')
meta.update(nativeStartupVerified=True,nativeScopedOutlineQualified=True,nativeLastRevision=77,userWorkspaceUnchanged=True)
meta_path.write_text(json.dumps(meta,indent=2)+'\n');print(json.dumps(report,indent=2))
