"""Independent checks of authored revision exports; no Folio engine imports."""
import json
import sys
from pathlib import Path
from zipfile import ZipFile
from lxml import etree

W = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
NS = {'w': W}


def visible(tree):
    result = []
    for paragraph in tree.xpath('//w:body//w:p[not(ancestor::w:pPrChange)]', namespaces=NS):
        for node in paragraph.iter():
            if any(etree.QName(a).localname in ('del', 'rPrChange', 'pPrChange') for a in node.iterancestors()):
                continue
            if node.tag == f'{{{W}}}t':
                result.append(node.text or '')
            elif node.tag == f'{{{W}}}tab':
                result.append('\t')
            elif node.tag in (f'{{{W}}}br', f'{{{W}}}cr'):
                result.append('\n')
        if not paragraph.xpath('w:pPr/w:rPr/w:del', namespaces=NS):
            result.append('\n')
    return ''.join(result)


def verify(folder):
    expected = {
        'folio-track-package-typing-split.docx': 'First \nlast\n',
        'folio-track-package-table-cell.docx': 'Title\nCell ONE\nCell two\nToday\nTail\n',
        'folio-track-package-table-cell-rejected.docx': 'Title\nCell one\nCell two\nToday\nTail\n',
        'folio-track-package-core-export.docx': 'First last\n',
        'folio-track-edit-projected-properties.docx': 'hello!\n',
        'folio-track-edit-projected-typing.docx': 'same!\n',
        'folio-track-package-native-append.docx': 'Quarterly planning update\nThe revenue forecast is 141000.\nDelivery remains our priority.\nReview due Friday.\nConfirmed.\n',
    }
    results = []
    for name, text in expected.items():
        with ZipFile(folder / name) as package:
            tree = etree.fromstring(package.read('word/document.xml'))
            assert visible(tree) == text, (name, visible(tree), text)
            revisions = tree.xpath('//w:ins | //w:del | //w:rPrChange | //w:pPrChange', namespaces=NS)
            ids = [n.get(f'{{{W}}}id') for n in revisions]
            assert len(ids) == len(set(ids)), (name, 'duplicate revision IDs')
            if name.endswith('native-append.docx'):
                assert len(revisions) == 7
                assert sum(n.get(f'{{{W}}}author') == 'Morgan' for n in revisions) == 2
                assert sum(n.get(f'{{{W}}}author') == 'Alex' for n in revisions) == 5
            else:
                assert all(n.get(f'{{{W}}}author') == 'Alex' for n in revisions), name
            if name.startswith('folio-track-package-') and not name.endswith('native-append.docx'):
                assert package.read('word/media/preserved.bin') == bytes([1, 2, 3, 4]), name
            if name.endswith('projected-properties.docx'):
                runs = tree.xpath('//w:r[not(ancestor::w:del) and not(ancestor::w:rPrChange)]', namespaces=NS)
                for run in runs:
                    assert run.xpath('w:rPr/w:smallCaps', namespaces=NS), name
                    assert run.xpath('w:rPr/w:b', namespaces=NS), name
                    fonts = run.find('w:rPr/w:rFonts', NS)
                    assert fonts.get(f'{{{W}}}ascii') == 'Arial'
                    assert fonts.get(f'{{{W}}}eastAsia') == 'Yu Gothic'
                spacing = tree.find('.//w:body/w:p/w:pPr/w:spacing', NS)
                assert spacing.get(f'{{{W}}}after') == '120'
                assert spacing.get(f'{{{W}}}before') == '360'
            if name.endswith('core-export.docx'):
                settings = etree.fromstring(package.read('word/settings.xml'))
                assert settings.find('w:trackRevisions', NS) is not None
            results.append({'file': name, 'visible': text, 'revisions': len(revisions)})
    for suffix, enabled in [('on', True), ('off', False)]:
        with ZipFile(folder / f'folio-track-package-settings-{suffix}.docx') as package:
            settings = etree.fromstring(package.read('word/preferences.xml'))
            assert (settings.find('w:trackRevisions', NS) is not None) == enabled
            assert (settings.find('w:doNotTrackFormatting', NS) is not None) == enabled
            assert settings.find('w:zoom', NS).get(f'{{{W}}}percent') == '125'
            assert settings.find('w:defaultTabStop', NS).get(f'{{{W}}}val') == '720'
    widget = folder / 'tracked-widget.docx'
    if widget.exists():
        with ZipFile(widget) as package:
            tree = etree.fromstring(package.read('word/document.xml'))
            assert visible(tree) == 'aaXY😀!\nTail\n', visible(tree)
            assert tree.xpath('//w:rPrChange', namespaces=NS)
            assert tree.xpath('//w:del/w:r/w:delText[text()="aa"]', namespaces=NS)
        results.append({'file': widget.name, 'visible': 'aaXY😀!\nTail\n'})
    return results


if __name__ == '__main__':
    print(json.dumps(verify(Path(sys.argv[1])), ensure_ascii=False, indent=2))
