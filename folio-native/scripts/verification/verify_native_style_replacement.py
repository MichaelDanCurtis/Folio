"""Check retained artifacts from the packaged macOS style-replacement flow."""
from pathlib import Path
from hashlib import sha256
import json, shutil
from docx import Document
root = Path(__file__).resolve().parents[4] / 'work'
folder = root / 'native-style-replacement'
meta = json.loads((root / 'style-replacement-package-verification.json').read_text())
shutil.copy2(meta['previewRecovery'], folder / 'after-restart.folio')
user = Path.home() / 'Library/Containers/com.folio.native/Data/Library/Application Support/com.folio.native/FolioNative/workspace.folio'
shutil.copy2(user, folder / 'user-after-check.folio')
read = lambda name: json.loads((folder / name).read_text())
before, replaced = read('before.folio'), read('replaced.folio')
assert (folder / 'before.folio').read_bytes() == (folder / 'cancelled.folio').read_bytes()
style = next(s for s in before['document']['attrs']['wordStyles']['styles'] if s['name']=='Native accent')['id']
assert all(s['id']!=style for s in replaced['document']['attrs']['wordStyles']['styles'])
assert replaced['revision']==57
for index,(a,b) in enumerate(zip(before['document']['content'], replaced['document']['content'])):
    if index == 1:
        assert a['content']==b['content']
        assert b['attrs']['paragraphFormat']['styleId']=='Normal'
        assert b['attrs']['paragraphFormat']['direct']==a['attrs']['paragraphFormat']['direct']
        assert b['attrs']['paragraphFormat']['inheritedRun']['sz']==24
    else: assert a==b, index
for name, revision, expected in [('undone.folio',58,before),('redone.folio',59,replaced),('post-save-undo.folio',60,before),('before-restart.folio',61,replaced),('after-restart.folio',61,replaced)]:
    state=read(name);assert state['revision']==revision
    assert state['document']==expected['document']
    assert state['sheets']==before['sheets']
assert (folder / 'before-restart.folio').read_bytes()==(folder / 'after-restart.folio').read_bytes()
assert (folder / 'user-before-check.folio').read_bytes()==(folder / 'user-after-check.folio').read_bytes()
d=Document(folder / 'native-style-replacement.docx')
assert d.paragraphs[1].text=='XY😀😀'
assert d.paragraphs[1].style.style_id=='Normal'
assert 'Native accent' not in [s.name for s in d.styles]
assert d.paragraphs[0].text=='Native title' and d.paragraphs[0].style.style_id=='Heading1'
assert d.paragraphs[2].text=='Section title' and d.paragraphs[2].style.style_id=='Heading2'
assert d.paragraphs[3].text=='Final body text.'
assert [p.text for p in d.paragraphs if p._p.pPr is not None and p._p.pPr.numPr is not None]==['First bullet','Second bullet']
report={'build':'8','nativeCancelUnchanged':True,'nativeReplaceInUseStyle':True,'nativeKeyboardUndoRedo':True,'nativePostSaveHistory':True,'independentDocxRead':True,'restartByteIdentical':True,'finalRevision':61,'userWorkspaceUnchanged':True,'userWorkspaceSha256':sha256(user.read_bytes()).hexdigest(),'microsoftWordExecution':False}
(folder/'verification.json').write_text(json.dumps(report,indent=2)+'\n')
meta.update(nativeStartupVerified=True,nativeScopedStyleReplacementQualified=True,nativeLastRevision=61,userWorkspaceUnchanged=True)
(root/'style-replacement-package-verification.json').write_text(json.dumps(meta,indent=2)+'\n')
print(json.dumps(report,indent=2))
