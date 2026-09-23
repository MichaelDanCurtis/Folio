"""Read Rust-generated tab exports independently with python-docx and lxml."""
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
from docx import Document
from docx.oxml.ns import qn
from docx.enum.text import WD_TAB_ALIGNMENT, WD_TAB_LEADER
from lxml import etree
folder = Path(gettempdir()) / 'folio-word-tabs-tests'
source = Path(__file__).resolve().parents[2] / 'test/fixtures/word-tabs-relocated.docx'
assert (folder / 'undone.docx').read_bytes() == source.read_bytes()
assert (folder / 'reimported.docx').read_bytes() == (folder / 'edited.docx').read_bytes()
for name in ['edited', 'reopened', 'reimported']:
    path = folder / f'{name}.docx'
    doc = Document(path)
    assert doc.paragraphs[0].text == 'Revised\t24.50\tReady'
    assert doc.paragraphs[0].runs[0].bold
    assert doc.paragraphs[0].paragraph_format.space_after.pt == 7
    assert doc.paragraphs[0].paragraph_format.keep_with_next
    assert doc.settings.element.find(qn('w:defaultTabStop')).get(qn('w:val')) == '900'
    tabs = doc.paragraphs[0].paragraph_format.tab_stops
    assert [t.position.pt for t in tabs] == [36, 72, 144]
    assert [t.alignment for t in tabs] == [WD_TAB_ALIGNMENT.CLEAR, WD_TAB_ALIGNMENT.CLEAR, WD_TAB_ALIGNMENT.RIGHT]
    assert tabs[-1].leader == WD_TAB_LEADER.DASHES
    assert doc.tables[0].cell(0,0).text == 'Nested\t12.50'
    assert doc.paragraphs[1].style.style_id == 'TabDerived'
    assert [t.position.pt for t in doc.styles['TabBase'].paragraph_format.tab_stops] == [36]
    with ZipFile(path) as package, ZipFile(source) as original:
        for part in ['word/options/styles-custom.xml','word/header1.xml','word/_rels/document.xml.rels','[Content_Types].xml']:
            assert package.read(part) == original.read(part), part
        for part in package.namelist():
            if part.endswith('.xml') or part.endswith('.rels'): etree.fromstring(package.read(part))
new = Document(folder / 'new-default.docx')
assert new.settings.element.find(qn('w:defaultTabStop')).get(qn('w:val')) == '480'
print('Verified custom stops and clears, leaders, default spacing, relocated settings/styles, retained formatting, table text, reopened/reimported exports, new settings relationships and byte-identical Undo.')

leader_values = ['none','dot','hyphen','underscore','heavy','middleDot']
leader_enums = [WD_TAB_LEADER.SPACES, WD_TAB_LEADER.DOTS, WD_TAB_LEADER.DASHES, WD_TAB_LEADER.LINES, WD_TAB_LEADER.HEAVY, WD_TAB_LEADER.MIDDLE_DOT]
for name in ['all-leaders','all-leaders-reopened','all-leaders-reimported']:
    doc = Document(folder / f'{name}.docx')
    assert [p.text for p in doc.paragraphs] == [f'{leader}\t123.45' for leader in leader_values]
    for p, leader in zip(doc.paragraphs, leader_enums):
        assert len(p.paragraph_format.tab_stops) == 1
        stop = p.paragraph_format.tab_stops[0]
        assert stop.leader == leader and stop.alignment == WD_TAB_ALIGNMENT.RIGHT and stop.position.pt == 216
        assert p.runs[0].bold
        assert len(p._p.findall('.//' + qn('w:r') + '/' + qn('w:tab'))) == 1
assert (folder/'all-leaders-reimported.docx').read_bytes() == (folder/'all-leaders.docx').read_bytes()
print('Verified all six leader choices through new, reopened and reimported DOCX files, each with one real tab and retained bold text.')
