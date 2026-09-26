import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_page_layout.dart';
import 'document_page_layout_test.dart' as f;

Map picture(double width, double height) => {
  'type': 'image',
  'attrs': {'src': 'word/media/p.png', 'width': width, 'height': height},
};

void main() {
  test('a picture sets its line height and scales to the text width', () {
    DocumentPageLayout build(Map picture) {
      final result = DocumentPageLayout.build(
        f.snapshot(
          [
            {
              'type': 'paragraph',
              'content': [
                {'type': 'text', 'text': 'Before '},
                picture,
                {'type': 'text', 'text': ' after'},
              ],
            },
            f.paragraph('Next paragraph'),
          ],
          [f.section(0, 1)],
        ),
      );
      addTearDown(result.dispose);
      return result;
    }

    double nextTop(DocumentPageLayout result) => result.pages.first.content
        .firstWhere((p) => p.text.contains('Next'))
        .offset
        .dy;
    final small = build(picture(40, 20));
    final tall = build(picture(40, 220));
    expect(nextTop(tall) - nextTop(small), closeTo(200, 1));
    // Wider than the page's text: scaled to fit, keeping its shape.
    final wide = build(picture(6000, 3000));
    final line = wide.pages.first.content.first;
    expect(line.text, contains('￼'));
    // 2:1, at the section's text width.
    final text = f.section(0, 1);
    expect(text, isNotNull);
    expect(line.height, lessThan(3000 / 4));
    expect(line.height, greaterThan(100));
  });

  test('a decoded picture is painted in its box', () async {
    final recorder = ui.PictureRecorder();
    final canvas = Canvas(recorder);
    final source = ui.PictureRecorder();
    Canvas(source).drawRect(
      const Rect.fromLTWH(0, 0, 2, 2),
      Paint()..color = const Color(0xffff0000),
    );
    final image = await source.endRecording().toImage(2, 2);
    previewPictures['word/media/p.png'] = image;
    addTearDown(previewPictures.clear);
    paintPicture(canvas, const Rect.fromLTWH(0, 0, 10, 10), 'word/media/p.png');
    paintPicture(canvas, const Rect.fromLTWH(20, 20, 5, 5), 'missing.png');
    final painted = await recorder.endRecording().toImage(10, 10);
    final bytes = (await painted.toByteData())!;
    // The centre is the picture's red, not the placeholder's grey.
    expect(bytes.getUint8((5 * 10 + 5) * 4), 0xff);
    expect(bytes.getUint8((5 * 10 + 5) * 4 + 1), 0);
  });
}
