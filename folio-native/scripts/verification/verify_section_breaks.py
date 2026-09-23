"""Inspect generated section fixtures independently of Folio's Rust engine."""
import argparse, json, zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

parser = argparse.ArgumentParser()
parser.add_argument('folder', type=Path)
parser.add_argument('--ui', type=Path)
args = parser.parse_args()
W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
def package(path):
    with zipfile.ZipFile(path) as z:
        return {n: z.read(n) for n in z.namelist()}
def read(path):
    p = package(path)
    return p, ET.fromstring(p['word/document.xml']).find(W+'body')
def text(node):
    return ''.join((n.text or '') if n.tag == W+'t' else '\t' if n.tag == W+'tab'
                   else '\n' if n.tag in {W+'br', W+'cr'} else '' for n in node.iter())
source, original = read(args.folder/'folio-section-source.docx')
for kind in ['nextPage','continuous','evenPage','oddPage']:
    p, body = read(args.folder/f'folio-section-{kind}.docx')
    sections = list(body.iter(W+'sectPr'))
    assert len(sections) == 2
    assert sections[1].find(W+'type').get(W+'val') == kind
    assert [text(n) for n in body.findall(W+'p')] == ['Alpha 😀',' beta','Second paragraph.']
    assert len(list(body.iter(W+'bookmarkStart'))) == len(list(body.iter(W+'bookmarkEnd'))) == 1
    assert len(list(sections[0].iter(W+'headerReference'))) == 1
    assert not list(sections[1].iter(W+'headerReference'))
    assert body.findall(W+'p')[1].find(W+'hyperlink') is not None
    for part in ['word/header1.xml','word/footer1.xml','word/_rels/document.xml.rels']:
        assert p[part] == source[part], (kind,part)
_, merged = read(args.folder/'folio-section-merged.docx')
sections = list(merged.iter(W+'sectPr'))
assert len(sections) == 1
assert sections[0].find(W+'pgSz').get(W+'orient') == 'landscape'
assert sections[0].find(W+'pgMar').get(W+'left') == '720'
assert text(merged) == text(original)
_, inherited = read(args.folder/'folio-section-inherited.docx')
sections = list(inherited.iter(W+'sectPr'))
assert len(sections) == 3
assert sections[1].find(W+'pgSz').get(W+'w') == '15840'
assert sections[1].find(W+'pgSz').get(W+'orient') == 'landscape'
p, live = read(args.folder/'folio-section-core.docx')
assert 'Live after removing a section' in text(live)
assert 'Pending Alpha 😀 beta' in text(live)
assert len(list(live.iter(W+'sectPr'))) == 1
assert {'AfterLive','SplitPoint'} <= {n.get(W+'name') for n in live.iter(W+'bookmarkStart')}
comments = ET.fromstring(p['word/comments.xml'])
assert 'Check beta' in text(comments)
assert len(list(live.iter(W+'commentRangeStart'))) == len(list(live.iter(W+'commentRangeEnd'))) == 1
if args.ui:
    _, body = read(args.ui)
    assert [text(p) for p in body.findall(W+'p')] == ['New Alpha 😀 beta','Second paragraph.']
    sections = list(body.iter(W+'sectPr'))
    assert len(sections) == 1 and sections[0].find(W+'pgMar').get(W+'left') == '720'
print(json.dumps({'passed':True,'engineArtifacts':8,'uiArtifact':str(args.ui) if args.ui else None,
  'checks':['four start types','rich paragraph split','bookmark pairing','header and footer links',
            'following-section geometry','continuous inheritance','live data and comments','UI merge and margin'],
  'microsoftWordExecuted':False}, indent=2))
