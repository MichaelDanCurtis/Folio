"""Independently inspect revision fixture output; does not run Microsoft Word."""
import argparse, json
from pathlib import Path
from zipfile import ZipFile
from lxml import etree

parser=argparse.ArgumentParser()
parser.add_argument('directory',type=Path)
args=parser.parse_args()
ns={'w':'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}

def root(name):
    path=args.directory/f'folio-revisions-{name}.docx'
    with ZipFile(path) as z:
        for part in z.namelist():
            if part.endswith('.xml'):
                xml=z.read(part)
                assert b'folioReviewTemporary' not in xml,(path,part)
                etree.fromstring(xml)
        return etree.fromstring(z.read('word/document.xml'))

def texts(d): return d.xpath('//w:t/text()',namespaces=ns)
def no_revisions(d): assert not d.xpath('//w:ins|//w:del|//w:rPrChange|//w:pPrChange',namespaces=ns)

a=root('accepted');assert texts(a)==['Keep ','new'];no_revisions(a)
r=root('rejected');assert texts(r)==['Keep ','old😀'];no_revisions(r)
assert r.xpath('boolean(//w:r[w:t="old😀"]/w:rPr/w:b)',namespaces=ns)
assert a.xpath('boolean(//w:r[w:t="new"]/w:rPr/w:i)',namespaces=ns)
for name in ['break-del','break-ins']:
    d=root(name);assert len(d.xpath('//w:body/w:p',namespaces=ns))==2
    assert texts(d)==['First','Second','Third'];no_revisions(d)
    assert d.xpath('string(//w:body/w:p[1]/w:pPr/w:jc/@w:val)',namespaces=ns)=='right'
f=root('format-rejected');assert f.xpath('string(//w:jc/@w:val)',namespaces=ns)=='left'
assert f.xpath('boolean(//w:rPr/w:i)',namespaces=ns);assert not f.xpath('//w:rPr/w:b',namespaces=ns)
assert texts(root('nested'))==['A','B']
assert len(root('row-accepted').xpath('//w:tr',namespaces=ns))==1
assert not root('row-rejected').xpath('//w:tbl',namespaces=ns)
assert texts(root('default-namespace'))==[' kept & restored ']
assert 'New live value' in texts(root('linked'))
assert texts(root('core-reviewed'))==['Added','Pending edit']
print(json.dumps({'result':'pass','checks':['exact Unicode text and escaping','rich run properties','paragraph joins and following content','formatting rejection','nested bulk resolution','table row acceptance/rejection','default namespaces','live workbook refresh','pending edits retained','no temporary review metadata in packages']},indent=2))
