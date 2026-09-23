"""Independent WordprocessingML reader for the page-setup qualification fixtures."""
import argparse, json, zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
R = '{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'

def read(path):
    with zipfile.ZipFile(path) as z:
        root = ET.fromstring(z.read('word/document.xml'))
        body = root.find(W+'body')
        sections = []
        for i, block in enumerate(body):
            nodes = [block] if block.tag == W+'sectPr' else block.findall('./'+W+'pPr/'+W+'sectPr')
            for node in nodes:
                size, margins = node.find(W+'pgSz'), node.find(W+'pgMar')
                sections.append({
                    'endParagraph': None if block.tag == W+'sectPr' else i,
                    'size': {k.removeprefix(W): v for k,v in size.attrib.items()} if size is not None else {},
                    'margins': {k.removeprefix(W): v for k,v in margins.attrib.items()} if margins is not None else {},
                    'headers': [h.get(R+'id') for h in node.findall(W+'headerReference')],
                    'recorded': node.find(W+'sectPrChange') is not None,
                })
        return {'file':str(path),'sections':sections,'text':'|'.join(t.text or '' for t in body.iter(W+'t')),
                'header':z.read('word/header1.xml').decode() if 'word/header1.xml' in z.namelist() else None}

def verify(folder, extra):
    records = {p.stem.removeprefix('folio-page-'):read(p) for p in folder.glob('folio-page-*.docx')}
    source=records['source']
    assert len(source['sections'])==2
    for name in ['selected','all','tracked','tracked-accept','tracked-reject','core-export','split','core-tracked-reject','live']:
        assert records[name]['header']==source['header'],name
        assert len(records[name]['sections'])==2,name
        assert records[name]['sections'][0]['headers']==['headerRef'],name
    assert records['selected']['sections'][0]==source['sections'][0]
    assert records['selected']['sections'][1]['margins']['left']=='1800'
    assert all(s['margins']['top']=='1080' for s in records['all']['sections'])
    assert records['tracked']['sections'][0]['recorded']
    assert records['tracked-accept']['sections'][0]['size']['w']=='15840'
    assert records['tracked-reject']['sections'][0]['size']['w']=='12240'
    assert records['core-export']['sections'][0]['margins']['left']=='1080'
    assert 'Pending ordinary edit.' in records['core-export']['text']
    assert records['core-tracked-reject']['sections'][0]['margins']['left']=='1440'
    assert records['split']['sections'][0]['endParagraph']==2
    assert 'Still live after page setup' in records['live']['text']
    for file in extra:
        record=read(file)
        assert len(record['sections'])==2,file
        assert record['sections'][0]['headers']==['headerRef'],file
        records[file.stem]=record
    return {'passed':True,'count':len(records),'artifacts':list(records.values()),'boundary':'Independent XML checks; does not run Microsoft Word.'}

if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('folder',type=Path)
    p.add_argument('extra',nargs='*',type=Path)
    a=p.parse_args()
    print(json.dumps(verify(a.folder,a.extra),indent=2))
