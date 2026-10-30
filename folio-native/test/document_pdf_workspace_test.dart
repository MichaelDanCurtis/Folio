import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_page_layout.dart';
import 'package:folio_native/src/document_pdf.dart';

void main() {
  var initialized = false;
  Future<void> initialize() async {
    if (initialized) return;
    await CoreBridge.initialize();
    initialized = true;
  }

  test('the recorder keeps text, links, decorations and clips', () {
    final painter = TextPainter(
      text: const TextSpan(
        style: TextStyle(fontSize: 12, color: Color(0xff222222)),
        children: [
          TextSpan(text: 'Plain '),
          LinkTextSpan(
            href: 'https://example.com',
            text: 'link',
            style: TextStyle(decoration: TextDecoration.underline),
          ),
          TextSpan(text: '\tafter'),
        ],
      ),
      textDirection: TextDirection.ltr,
    )..layout(maxWidth: 400);
    final recorder = PdfRecorder();
    recorder.save();
    recorder.clipRect(const Rect.fromLTWH(0, 0, 100, 50));
    recorder.translate(10, 20);
    recorder.text(painter, const Offset(5, 5));
    recorder.restore();
    final texts = recorder.ops.where((o) => o['op'] == 'text').toList();
    expect(texts.map((t) => t['text']), ['Plain ', 'link', 'after']);
    expect(texts[1]['link'], 'https://example.com');
    expect(texts[0]['x'], greaterThanOrEqualTo(15));
    expect(texts.every((t) => (t['width'] as double) > 0), isTrue);
    // The underline is a rule under the link.
    expect(recorder.ops.where((o) => o['op'] == 'line'), hasLength(1));
    expect(recorder.ops.first['op'], 'save');
    expect(recorder.ops[1]['op'], 'clip');
    painter.dispose();
  });

  testWidgets('a document exports as a tagged PDF of its preview pages', (
    t,
  ) async {
    late Directory folder;
    late WorkspaceModel model;
    await t.runAsync(() async {
      await initialize();
      folder = await Directory.systemTemp.createTemp('folio-pdf-');
      final core = CoreBridge();
      await core.call({'op': 'new', 'blank': true});
      await core.call({
        'op': 'document',
        'document': {
          'type': 'doc',
          'content': [
            {
              'type': 'heading',
              'attrs': {'level': 1},
              'content': [
                {'type': 'text', 'text': 'Annual summary'},
              ],
            },
            for (var i = 0; i < 60; i++)
              {
                'type': 'paragraph',
                'content': [
                  {'type': 'text', 'text': 'Paragraph $i of the report. '},
                  if (i == 3)
                    {
                      'type': 'text',
                      'text': 'Source',
                      'marks': [
                        {
                          'type': 'link',
                          'attrs': {'href': 'https://example.com/source'},
                        },
                      ],
                    },
                ],
              },
            {
              'type': 'heading',
              'attrs': {'level': 2},
              'content': [
                {'type': 'text', 'text': 'Appendix'},
              ],
            },
          ],
        },
      });
      model = WorkspaceModel(core, '${folder.path}/workspace.folio');
      await model.initialize();
    });
    final path = '${folder.path}/summary.pdf';
    await t.runAsync(() => exportDocumentPdf(model, path));
    final bytes = await t.runAsync(() => File(path).readAsBytes());
    final pdf = String.fromCharCodes(bytes!);
    expect(pdf.startsWith('%PDF-'), isTrue);
    for (final needle in [
      '/StructTreeRoot',
      '/S /H1',
      '/S /H2',
      '/S /P',
      '/Outlines',
      '(Annual summary)',
      '(Appendix)',
      '(https://example.com/source)',
      '/FontFile2',
      '/ToUnicode',
    ]) {
      expect(pdf.contains(needle), isTrue, reason: needle);
    }
    // As many pages as the preview.
    final snapshot = await t.runAsync(
      () => model.core.call({'op': 'documentPagePreview'}),
    );
    final layout = DocumentPageLayout.build(snapshot!);
    final pages = layout.pages.length;
    layout.dispose();
    expect(pages, greaterThan(1));
    expect(RegExp(r'/Type /Page\b(?!s)').allMatches(pdf).length, pages);
    model.dispose();
  });

  testWidgets('a table of contents keeps its dot leaders and right-aligned '
      'page numbers in the PDF', (t) async {
    late CoreBridge core;
    await t.runAsync(() async {
      await initialize();
      core = CoreBridge();
      await core.call({'op': 'new', 'blank': true});
      await core.call({
        'op': 'document',
        'document': {
          'type': 'doc',
          'content': [
            {
              'type': 'tableOfContents',
              'attrs': {
                'levels': 3,
                'entries': [
                  {'level': 1, 'text': 'Method', 'page': '1'},
                  {
                    'level': 2,
                    'text': 'A longer sampling section',
                    'page': '1',
                  },
                ],
              },
            },
            {
              'type': 'heading',
              'attrs': {'level': 1},
              'content': [
                {'type': 'text', 'text': 'Method'},
              ],
            },
            {
              'type': 'heading',
              'attrs': {'level': 2},
              'content': [
                {'type': 'text', 'text': 'A longer sampling section'},
              ],
            },
          ],
        },
      });
    });
    final snapshot = await t.runAsync(
      () => core.call({'op': 'documentPagePreview'}),
    );
    final layout = DocumentPageLayout.build(snapshot!);
    final request = await t.runAsync(
      () => documentPdfRequest(snapshot, layout, title: 'Contents'),
    );
    layout.dispose();
    final texts = (request!['pages'] as List).first['ops']
        .where((o) => o['op'] == 'text')
        .toList()
        .cast<Map>();
    // Each entry's leader is a run of dots, as the preview paints it.
    final leaders = texts
        .where((o) => RegExp(r'^\.{3,}$').hasMatch(o['text'] as String))
        .toList();
    expect(leaders, hasLength(2));
    // Both page numbers end at the right tab stop (9350 twips from the
    // left margin), and each leader runs up to its number.
    final numbers = texts.where((o) => o['text'] == '1').toList();
    expect(numbers, hasLength(2));
    num end(Map o) => (o['x'] as num) + (o['width'] as num);
    expect(end(numbers[0]), closeTo(end(numbers[1]), .5));
    expect(end(numbers[0]), closeTo(96 + 9350 / 15, 1));
    for (var i = 0; i < 2; i++) {
      expect(end(leaders[i]), lessThanOrEqualTo(numbers[i]['x'] as num));
      expect(end(leaders[i]), greaterThan((numbers[i]['x'] as num) - 8));
    }
    // The engine writes them.
    final folder = await t.runAsync(
      () => Directory.systemTemp.createTemp('folio-pdf-toc-'),
    );
    await t.runAsync(
      () => core.call({
        'op': 'exportPdf',
        'path': '${folder!.path}/contents.pdf',
        ...request,
      }),
    );
    expect(File('${folder!.path}/contents.pdf').existsSync(), isTrue);
  });
}
