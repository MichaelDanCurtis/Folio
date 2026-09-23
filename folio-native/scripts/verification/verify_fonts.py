"""Validate authored fonts and rich-run edits independently of the Rust writer."""
from copy import copy
from pathlib import Path
from tempfile import gettempdir
from zipfile import ZipFile
from lxml import etree
from openpyxl import load_workbook
from openpyxl.styles import Color
import sys

folder=Path(gettempdir())/'folio-style-tests'
source=Path(__file__).resolve().parents[2]/'test/fixtures/styles.xlsx'
original=load_workbook(source,rich_text=True)['Input']
edited=load_workbook(folder/'font.xlsx',rich_text=True)['Input']
font=copy(original['B2'].font)
font.name='Georgia';font.sz=16.5;font.b=False;font.i=False;font.strike=None
font.u='doubleAccounting';font.vertAlign='subscript';font.color=Color(auto=True)
font.scheme=None;font.family=None;font.charset=None
assert copy(edited['B2'].font)==font
for field in ['fill','border','alignment','protection','number_format']:
    assert copy(getattr(edited['B2'],field))==copy(getattr(original['B2'],field)),field
assert (folder/'font-original.xlsx').read_bytes()==(folder/'font-undone.xlsx').read_bytes()
for name in ['font-redone.xlsx','font-reopened.xlsx']:
    assert (folder/name).read_bytes()==(folder/'font.xlsx').read_bytes()
effects=load_workbook(folder/'font-effects.xlsx')['Input']
for i,u in enumerate([None,'single','double','singleAccounting','doubleAccounting']):
    assert effects[f'V{i+2}'].font.u==u
    assert effects[f'V{i+2}'].font.sz==1+i*.5
    assert effects[f'V{i+2}'].font.vertAlign==(None if i==0 else 'superscript')

ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
for name in ['rich-bold.xlsx','rich-font.xlsx']:
    cells=load_workbook(folder/name,rich_text=True)['Input']
    assert cells['T2'].value==original['T2'].value
    with ZipFile(folder/name) as package,ZipFile(source) as old:
        assert package.read('xl/sharedStrings.xml')==old.read('xl/sharedStrings.xml')
        xml=etree.fromstring(package.read('xl/worksheets/sheet1.xml'))
        assert xml.find('.//s:c[@r="C2"]',ns).get('{urn:folio:fill:metadata}flag')=='retained'
        for a,text in [('C2','Rich title'),('S2','Shared title')]:
            c=xml.find(f'.//s:c[@r="{a}"]',ns)
            assert c.get('t')=='inlineStr'
            assert ''.join(c.findall('s:is/s:r/s:t',ns)[i].text for i in range(2))==text
            for run in c.findall('s:is/s:r',ns):
                assert run.find('s:rPr/s:b',ns).get('val')=='0'
                if name=='rich-font.xlsx':
                    for tag,value in [('rFont','Georgia'),('sz','16.5'),('u','doubleAccounting'),('vertAlign','subscript')]:
                        assert run.find('s:rPr/s:'+tag,ns).get('val')==value
                    assert run.find('s:rPr/s:color',ns).get('auto')=='1'
        # The unrequested second-run italic and size survive the bold-only edit.
        if name=='rich-bold.xlsx':
            runs=xml.findall('.//s:c[@r="S2"]/s:is/s:r',ns)
            assert runs[0].find('s:rPr/s:rFont',ns).get('val')=='Times New Roman'
            assert runs[1].find('s:rPr/s:i',ns) is not None
            assert runs[1].find('s:rPr/s:sz',ns).get('val')=='22'
assert (folder/'rich-font.xlsx').read_bytes()==(folder/'rich-font-reopened.xlsx').read_bytes()

if '--with-ui' in sys.argv:
    ui=Path(gettempdir())/'folio-font-ui-tests'
    w=load_workbook(ui/'edited.xlsx',rich_text=True)['Input']
    for a in ['B2','C2','D2','E2']:
        assert w[a].font.name=='Georgia' and w[a].font.sz==16.5
        assert w[a].font.u=='doubleAccounting' and w[a].font.vertAlign=='subscript'
        assert w[a].font.color.rgb=='FF345678' and not w[a].font.strike
    assert w['B2'].font.b and not w['C2'].font.b
    assert w['C2'].value[0].font.b
    assert str(w['C2'].value)=='Rich title'
    for run in w['C2'].value:
        assert run.font.rFont=='Georgia' and run.font.sz==16.5
        assert run.font.u=='doubleAccounting' and run.font.vertAlign=='subscript'
        assert run.font.color.rgb=='FF345678' and not run.font.strike
    assert (ui/'undone.xlsx').read_bytes()==source.read_bytes()
    for name in ['redone.xlsx','reopened.xlsx']:
        assert (ui/name).read_bytes()==(ui/'edited.xlsx').read_bytes()
    print('Verified mixed-selection Font dialog exports and exact Undo/Redo/reopen files.')
print('Verified font authoring, all underline types, text positions, exact recovery and independently retained rich/shared-string formatting.')
