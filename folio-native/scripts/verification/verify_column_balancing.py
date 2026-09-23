"""Independently verify the balancing fixture pair and unchanged UI exports."""
import argparse
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET
from zipfile import ZipFile


def parts(path):
    with ZipFile(path) as archive:
        return {name: archive.read(name) for name in archive.namelist()}


def verify(source, disabled, source_export, disabled_export):
    left, right = parts(source), parts(disabled)
    assert left.keys() == right.keys()
    changed = [name for name in left if left[name] != right[name]]
    assert changed == ['word/settings.xml'], changed
    ns = {'w': 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}
    enabled_root = ET.fromstring(left['word/settings.xml'])
    disabled_root = ET.fromstring(right['word/settings.xml'])
    assert enabled_root.find('w:compat/w:noColumnBalance', ns) is None
    flag = disabled_root.find('w:compat/w:noColumnBalance', ns)
    assert flag is not None and not flag.attrib
    doc = ET.fromstring(left['word/document.xml'])
    sections = doc.findall('.//w:sectPr', ns)
    assert len(sections) == 2
    w = '{' + ns['w'] + '}'
    assert sections[0].find('w:cols', ns).get(w + 'num') == '2'
    assert sections[1].find('w:type', ns).get(w + 'val') == 'continuous'
    content = [n.text for n in doc.findall('.//w:t', ns)]
    assert content == [f'Line {i:02}.' for i in range(20)] + [
        'End of columns.', 'Full-width text follows the balanced columns.']
    for original, exported in [(source, source_export), (disabled, disabled_export)]:
        assert original.read_bytes() == exported.read_bytes(), exported
    return {
        'passed': True,
        'onlyCompatibilitySettingDiffers': True,
        'sections': 2,
        'exportsByteIdentical': True,
        'artifacts': {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                      for p in [source, disabled, source_export, disabled_export]},
        'microsoftWordExecuted': False,
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['source', 'disabled', 'source_export', 'disabled_export']:
        parser.add_argument(name, type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(**vars(args)), indent=2))
