"""Read header/footer outputs independently of Folio's engine (does not run Word)."""
import argparse, json, posixpath, zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

parser = argparse.ArgumentParser()
parser.add_argument('folder', type=Path)
parser.add_argument('--ui', type=Path)
args = parser.parse_args()
W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'

def package(path):
    with zipfile.ZipFile(path) as z:
        return {n: z.read(n) for n in z.namelist()}

def text(node):
    return ''.join(n.text or '' for n in node.iter(W+'t'))

def read(path):
    p = package(path)
    root = ET.fromstring(p['word/document.xml'])
    sections = list(root.iter(W+'sectPr'))
    rels = {r.get('Id'): r for r in ET.fromstring(p['word/_rels/document.xml.rels'])}
    def story(index, kind='header', variant='default'):
        for section in reversed(sections[:index+1]):
            refs = [r for r in section.findall(W+kind+'Reference') if r.get(W+'type') == variant]
            if refs:
                rel = rels[refs[0].get(R+'id')]
                assert rel.get('Type').endswith('/'+kind)
                path = posixpath.normpath(posixpath.join('word',rel.get('Target'))).lstrip('/')
                return ET.fromstring(p[path])
        return ET.fromstring('<blank/>')
    return p,root,sections,story

source, original, _, _ = read(args.folder/'folio-headers-source.docx')
for suffix in ['unlinked','linked','variants','bookmarks','core']:
    p, root, sections, story = read(args.folder/f'folio-headers-{suffix}.docx')
    assert len(sections) == 3
    assert p['word/header2.xml'] == source['word/header2.xml']
    assert text(story(2)) == 'Independent'
    if suffix != 'core':
        assert text(root) == text(original)
    if suffix in ['unlinked','linked']:
        assert text(story(0)) == 'Changed 😀 link'
        assert text(story(1)) == ('Only section two 😀 link' if suffix=='unlinked' else 'Changed 😀 link')
        assert len(sections[1].findall(W+'headerReference')) == (1 if suffix=='unlinked' else 0)
        copied = ET.fromstring(p['word/folio-header-1.xml'])
        assert next(copied.iter(W+'bookmarkStart')).get(W+'name') == 'HeaderName_Copy1'
        assert any(n.get('Target')=='https://example.org/header' for n in ET.fromstring(p['word/_rels/folio-header-1.xml.rels']))
    if suffix=='variants':
        assert text(story(0,'footer','first')) == ''
        assert text(story(1,'footer','first')) == text(story(2,'footer','first')) == 'Chapter opening'
        assert text(story(1,'footer')) == 'Footer retained'
        assert sections[0].find(W+'titlePg') is None
        assert sections[1].find(W+'titlePg') is not None
        assert ET.fromstring(p['word/settings.xml']).find(W+'evenAndOddHeaders') is not None
        assert text(story(0,'header','even')) == 'Even list'
        assert next(story(0,'header','even').iter(W+'numPr'),None) is not None
        assert 'word/numbering.xml' in p
    if suffix=='bookmarks':
        copied=story(1)
        assert next(copied.iter(W+'bookmarkStart')).get(W+'name')=='HeaderName_Copy1'
        assert any(n.get(W+'anchor')=='HeaderName_Copy1' for n in copied.iter(W+'hyperlink'))
        assert 'HeaderName_Copy1' in next(copied.iter(W+'fldSimple')).get(W+'instr')
        assert 'PAGEREF "HeaderName_Copy1"' in ''.join(n.text or '' for n in copied.iter(W+'instrText'))
        assert p['word/nested/header.xml'] != ET.tostring(copied)
    if suffix=='core':
        assert text(story(1))=='Native chapter link'
        assert 'Draft body one.' in text(root)
        assert any(n.get(W+'name')=='BodyDraft' for n in root.iter(W+'bookmarkStart'))
        assert 'Body comment stays' in text(ET.fromstring(p['word/comments.xml']))
if args.ui:
    p, root, sections, story = read(args.ui)
    assert text(root)==text(original)
    assert text(story(0))=='Shared 😀 link'
    assert text(story(1))=='Chapter Shared 😀 link'
    assert text(story(2))=='Independent'
    assert text(story(1,'footer','first'))=='Chapter footer'
    assert text(story(2,'footer','first'))=='Chapter footer'
    assert sections[1].find(W+'titlePg') is not None
print(json.dumps({'passed':True,'engineArtifacts':6,'uiArtifact':str(args.ui) if args.ui else None,
  'checks':['linked ownership','independent copies','body preservation','first and even settings','list numbering','bookmark and field retargeting','body comments','UI persistence'],
  'microsoftWordExecuted':False},indent=2))
