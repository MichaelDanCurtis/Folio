"""Verify saved native header/footer artifacts independently; does not execute Word."""
import argparse, hashlib, json, posixpath, zipfile
from pathlib import Path
from xml.etree import ElementTree as ET
p=argparse.ArgumentParser();p.add_argument('folder',type=Path);root=p.parse_args().folder
snap=lambda name:json.loads((root/(name+'.folio')).read_text())
base,canceled,shared,unlinked,footer,even,undo,post,final=[snap(n) for n in ['baseline','canceled','shared','unlinked','footer','even','undone','post-save-undo','after-restart']]
assert [v['revision'] for v in [base,canceled,shared,unlinked,footer,even,undo,post,final]]==[1,1,2,3,4,5,6,8,9]
assert (root/'baseline.folio').read_bytes()==(root/'canceled.folio').read_bytes()
assert (root/'before-restart.folio').read_bytes()==(root/'after-restart.folio').read_bytes()
assert footer['document']==undo['document']==post['document']
assert even['document']==final['document']
assert footer['officeDocument']==undo['officeDocument']==post['officeDocument']
assert even['officeDocument']==final['officeDocument']
W='{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
R='{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'
text=lambda node:''.join(t.text or '' for t in node.iter(W+'t'))
def read(name):
 with zipfile.ZipFile(root/name) as z:
  pkg={n:z.read(n) for n in z.namelist()}
 body=ET.fromstring(pkg['word/document.xml']);sections=list(body.iter(W+'sectPr'))
 rels={r.get('Id'):r for r in ET.fromstring(pkg['word/_rels/document.xml.rels'])}
 def story(i,kind='header',variant='default'):
  for s in reversed(sections[:i+1]):
   refs=[r for r in s.findall(W+kind+'Reference') if r.get(W+'type')==variant]
   if refs:
    path=posixpath.normpath(posixpath.join('word',rels[refs[0].get(R+'id')].get('Target'))).lstrip('/')
    return ET.fromstring(pkg[path])
  return ET.fromstring('<blank/>')
 return pkg,body,sections,story
source,body,_,_=read('headers-source.docx')
for name in ['shared','unlinked','final']:
 pkg,doc,sections,story=read('headers-'+name+'.docx')
 assert text(doc)==text(body)=='Body one.Body two.Body three.'
 assert len(sections)==3
 assert text(story(0))=='Shared 😀 linkShared edit '
 assert text(story(1))==('Shared 😀 linkShared edit ' if name=='shared' else 'Chapter Shared 😀 linkShared edit ')
 assert text(story(2))=='Independent'
 assert all(text(story(i,'footer'))=='Footer retained' for i in range(3))
 assert pkg['word/header2.xml']==source['word/header2.xml']
 assert pkg['word/footer1.xml']==source['word/footer1.xml']
 if name!='shared':
  assert sections[1].find(W+'titlePg') is not None
  assert sections[0].find(W+'titlePg') is None
  assert ET.fromstring(pkg['word/settings.xml']).find(W+'evenAndOddHeaders') is not None
  assert next(story(1).iter(W+'bookmarkStart')).get(W+'name')=='HeaderName_Copy1'
  assert any(r.get('Target')=='https://example.org/header' for r in ET.fromstring(pkg['word/_rels/folio-header-1.xml.rels']))
 if name=='final':
  assert text(story(0,'footer','first'))==''
  assert text(story(1,'footer','first'))==text(story(2,'footer','first'))=='Chapter footer'
  assert text(story(0,'header','even'))==''
  assert text(story(1,'header','even'))==text(story(2,'header','even'))=='Even chapter'
print(json.dumps({'passed':True,'revision':9,'cancelExact':True,'undoExact':True,'postSaveUndoExact':True,
 'restartByteIdentical':True,'independentOfficeRead':True,'nativeOfficeArtifacts':3,
 'recoverySha256':hashlib.sha256((root/'after-restart.folio').read_bytes()).hexdigest(),
 'boundary':'Saved artifacts inspected independently. Native UI actions observed separately; Microsoft Word not executed.'},indent=2))
