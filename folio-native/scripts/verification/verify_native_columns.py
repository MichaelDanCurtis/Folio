"""Verify captured native column history after the desktop workflow is complete."""
import argparse
import hashlib
import json
from pathlib import Path

p=argparse.ArgumentParser()
p.add_argument('folder',type=Path)
p.add_argument('--base-revision',type=int,default=1)
args=p.parse_args()
folder=args.folder
names=['baseline','canceled','applied','undone','redone','post-save-undo','post-save-redo','before-restart','after-restart']
snapshots={name:json.loads((folder/f'{name}.folio').read_text()) for name in names}
assert [snapshots[name]['revision'] for name in names]==[args.base_revision+n for n in [0,0,1,2,3,4,5,5,5]]
assert (folder/'baseline.folio').read_bytes()==(folder/'canceled.folio').read_bytes()
assert (folder/'before-restart.folio').read_bytes()==(folder/'after-restart.folio').read_bytes()
for key in ['document','officeDocument']:
    for name in ['undone','post-save-undo']:
        assert snapshots[name][key]==snapshots['baseline'][key],(name,key)
    for name in ['redone','post-save-redo','before-restart','after-restart']:
        assert snapshots[name][key]==snapshots['applied'][key],(name,key)
settings=snapshots['applied']['document']['attrs']['pageLayout']['sections'][0]['columnLayout']
assert settings['count']==2 and not settings['equalWidth'] and settings['rtl'] and settings['separator']
assert settings['columns'][0]['width']<settings['columns'][1]['width']
print(json.dumps({'passed':True,'revision':args.base_revision+5,'cancelExact':True,'undoExact':True,
    'postSaveUndoExact':True,'restartByteIdentical':True,'columnLayout':settings,
    'recoverySha256':hashlib.sha256((folder/'after-restart.folio').read_bytes()).hexdigest(),
    'boundary':'Saved native snapshots inspected. Run verify_columns.py separately for the DOCX; Microsoft Word was not executed.'},indent=2))
