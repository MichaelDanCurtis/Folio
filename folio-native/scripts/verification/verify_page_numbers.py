"""Independent XML checks for page fields and section numbering; does not run Word."""
import argparse,json,posixpath,zipfile
from pathlib import Path
from xml.etree import ElementTree as ET
p=argparse.ArgumentParser();p.add_argument('folder',type=Path);p.add_argument('--ui',type=Path);a=p.parse_args()
W='{http://schemas.openxmlformats.org/wordprocessingml/2006/main}';R='{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'
def read(path):
 with zipfile.ZipFile(path) as z:return {n:z.read(n) for n in z.namelist()}
def text(n):return ''.join(t.text or '' for t in n.iter(W+'t'))
def field_codes(n):
 codes=[f.get(W+'instr').strip() for f in n.iter(W+'fldSimple')]
 complex_code=''.join(f.text or '' for f in n.iter(W+'instrText'))
 return codes+([complex_code] if complex_code else [])
f=a.folder
p=read(f/'folio-page-fields-edited.docx');root=ET.fromstring(p['word/document.xml'])
assert field_codes(root)==['PAGE','NUMPAGES \\* Arabic']
assert text(root)=='Report page 7 of 123 😀 end'
paragraph=root.find('.//'+W+'p');children=list(paragraph)
last_field_end=max(i for i,n in enumerate(children) if n.find('.//'+W+'fldChar') is not None and n.find('.//'+W+'fldChar').get(W+'fldCharType')=='end')
bookmark_end=next(i for i,n in enumerate(children) if n.tag==W+'bookmarkEnd')
assert bookmark_end>last_field_end
p=read(f/'folio-page-fields-new.docx');root=ET.fromstring(p['word/document.xml'])
assert field_codes(root)==['PAGE','NUMPAGES','SECTIONPAGES','SECTION']
assert all(n.get(W+'dirty')=='1' for n in root.iter(W+'fldSimple'))
assert text(root)=='????'
p=read(f/'folio-page-fields-comments.docx');root=ET.fromstring(p['word/document.xml'])
assert field_codes(root)==['NUMPAGES','PAGE']
assert len(list(root.iter(W+'commentRangeStart')))==len(list(root.iter(W+'commentRangeEnd')))==1
assert 'Keep the count' in text(ET.fromstring(p['word/comments.xml']))
source=read(f/'folio-page-numbers-source.docx')
for path,start,separator,unlinked in [(f/'folio-page-numbers-core.docx',0,'hyphen',False)]+([(a.ui,5,'colon',True)] if a.ui else []):
 p=read(path);root=ET.fromstring(p['word/document.xml']);sections=list(root.iter(W+'sectPr'))
 assert len(sections)==3
 assert text(root)==text(ET.fromstring(source['word/document.xml']))
 assert sections[0].find(W+'pgNumType') is None
 assert sections[2].find(W+'pgNumType') is None
 numbers=sections[1].find(W+'pgNumType');assert numbers.get(W+'fmt')=='lowerRoman'
 assert numbers.get(W+'start')==str(start) and numbers.get(W+'chapStyle')=='2' and numbers.get(W+'chapSep')==separator
 target=sections[1 if unlinked else 0].find(W+'footerReference').get(R+'id')
 rels={n.get('Id'):n.get('Target') for n in ET.fromstring(p['word/_rels/document.xml.rels'])}
 story=posixpath.normpath(posixpath.join('word',rels[target])).lstrip('/')
 assert field_codes(ET.fromstring(p[story]))==['PAGE','NUMPAGES']
 for part in ['word/nested/header.xml','word/header2.xml']:assert p[part]==source[part]
print(json.dumps({'passed':True,'engineArtifacts':5,'uiArtifact':str(a.ui) if a.ui else None,
 'checks':['field codes and cached results','atomic bookmark boundaries','comment and table fields','section-scoped format and restart','linked and independent footer stories','unchanged body and headers'],
 'pageValuesCalculated':False,'microsoftWordExecuted':False},indent=2))
