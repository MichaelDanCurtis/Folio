"""Verify the native row-splitting history and exported document checkpoints."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from zipfile import ZipFile
import xml.etree.ElementTree as ET


def verify(folder):
    names = ['01-imported', '02-allow-previewed', '03-cancelled', '04-kept',
             '05-undone', '06-redone', '07-keep-previewed', '08-exported',
             '09-saved', '10-postsave-undone', '11-postsave-redone',
             '12-allow-applied', '13-allow-previewed', '14-allow-exported',
             '15-keep-undone', '16-saved', '17-before-restart', '18-restarted',
             '19-recovered-export']
    raw = [(folder / (n + '.json')).read_bytes() for n in names]
    states = [json.loads(r) for r in raw]
    assert [s['revision'] for s in states] == [1, 1, 1, 2, 3, 4, 4, 4, 4, 5, 6, 7, 7, 7, 8, 8, 8, 8, 8]
    original = states[0]['document']
    kept, allowed = copy.deepcopy(original), copy.deepcopy(original)
    kept['content'][1]['content'][0]['attrs']['cantSplit'] = True
    allowed['content'][1]['content'][0]['attrs']['cantSplit'] = False
    for i, s in enumerate(states):
        expected = original if i in {0, 1, 2, 4, 9} else allowed if i in {11, 12, 13} else kept
        assert s['document'] == expected, names[i]
        assert {k: v for k, v in s.items() if k not in {'document', 'revision'}} == {
            k: v for k, v in states[0].items() if k not in {'document', 'revision'}
        }, names[i]
    for group in [(0, 1, 2), (5, 6, 7, 8), (11, 12, 13), (14, 15, 16, 17, 18)]:
        assert all(raw[i] == raw[group[0]] for i in group), group
    assert (folder / 'row-flow.docx').read_bytes() == (folder / 'kept.docx').read_bytes()
    independent = subprocess.run([sys.executable, str(Path(__file__).with_name('verify_table_row_flow.py')), str(folder)], check=True, capture_output=True, text=True)
    w = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
    with ZipFile(folder / 'allowed.docx') as z:
        allowed_parts = {n: z.read(n) for n in z.namelist()}
    with ZipFile(folder / 'kept.docx') as z:
        kept_parts = {n: z.read(n) for n in z.namelist()}
    assert kept_parts.keys() == allowed_parts.keys()
    a, k = [ET.fromstring(parts['word/document.xml']) for parts in [allowed_parts, kept_parts]]
    flag = a.find(f'{w}body/{w}tbl/{w}tr/{w}trPr/{w}cantSplit')
    assert flag is not None and flag.get(w + 'val') == 'off'
    flag.set(w + 'val', 'on')
    assert ET.tostring(a) == ET.tostring(k)
    assert all(b == kept_parts[n] for n, b in allowed_parts.items() if n != 'word/document.xml')
    return {'passed': True, 'snapshots': len(states), 'cancelByteIdentical': True,
            'previewByteIdentical': True, 'ordinaryAndPostSaveHistoryExact': True,
            'explicitAllowApplyExportAndUndoExact': True,
            'allUnrelatedWorkspaceStateUnchanged': True,
            'workingSaveMatchesExport': True, 'restartByteIdentical': True,
            'restartSha256': hashlib.sha256(raw[17]).hexdigest(),
            'exports': json.loads(independent.stdout),
            'checkpoints': {n: hashlib.sha256(b).hexdigest() for n, b in zip(names, raw)},
            'microsoftWordExecuted': False}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('folder', type=Path)
    print(json.dumps(verify(p.parse_args().folder), indent=2))
