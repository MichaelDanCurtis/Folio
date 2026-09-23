import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_page_layout.dart';
import 'package:folio_native/src/document_table_format_dialog.dart';
import 'package:folio_native/src/document_tables.dart';
import 'package:folio_native/src/document_fields.dart';
import 'document_page_layout_test.dart' as f;
import 'document_table_row_height_preview_test.dart' as h;
import 'document_column_balance_test.dart' as columns;

Map lines(String prefix, int count, {int lineHeight = 300}) => f.paragraph(
  List.generate(
    count,
    (i) => '$prefix${i.toString().padLeft(2, '0')}',
  ).join('\n'),
  properties: {
    'before': 0,
    'after': 0,
    'line': lineHeight,
    'lineRule': 'exact',
    'widowControl': false,
  },
);
Map twoCells({bool? cantSplit, int count = 22}) => h.table([
  {
    ...h.row([
      h.cell([lines('A', count, lineHeight: 315)]),
      h.cell([lines('B', count, lineHeight: 435)]),
    ]),
    'attrs': {'cantSplit': ?cantSplit},
  },
], columns: 2);
DocumentPageLayout layout(
  List content, {
  Map? section,
  List? sections,
  bool balance = true,
}) {
  final input = {
    ...f.snapshot(
      content,
      sections ?? [section ?? f.section(0, content.length - 1)],
    ),
    'noColumnBalance': !balance,
  };
  final before = jsonEncode(input);
  final out = DocumentPageLayout.build(input);
  addTearDown(out.dispose);
  expect(out.converged, true);
  expect(jsonEncode(input), before);
  for (final page in out.pages) {
    for (final item in page.content.where(
      (p) => ['body', 'tableHeader'].contains(p.area),
    )) {
      expect(item.offset.dy, greaterThanOrEqualTo(page.top - .01));
      expect(
        item.offset.dy + item.height,
        lessThanOrEqualTo(page.bottom + .01),
      );
    }
  }
  return out;
}

String body(DocumentPageLayout out) => out.pages
    .expand((p) => p.content)
    .where((p) => p.area == 'body')
    .map((p) => p.text)
    .join('\n');
void exactlyOnce(DocumentPageLayout out, String prefix, int count) {
  final text = body(out).replaceAll(RegExp(r'\s+'), '');
  expect(
    RegExp('$prefix[0-9]{2}').allMatches(text).map((m) => m[0]).toList(),
    List.generate(count, (i) => '$prefix${i.toString().padLeft(2, '0')}'),
  );
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  testWidgets('mixed font sizes render complete continuation lines on canvas', (
    tester,
  ) async {
    await tester.runAsync(() async {
      final loader = FontLoader('RowFlowArial')
        ..addFont(
          File(
            '/System/Library/Fonts/Supplemental/Arial.ttf',
          ).readAsBytes().then((b) => ByteData.sublistView(b)),
        );
      await loader.load();
    });
    final node = twoCells(count: 19);
    for (var c = 0; c < 2; c++) {
      final p = node['content'][0]['content'][c]['content'][0];
      p['attrs']['paragraphFormat']['direct'] = {
        'before': 0,
        'after': 0,
        'widowControl': false,
      };
      p['content'] = [
        {
          ...p['content'][0],
          'marks': [
            {
              'type': 'textStyle',
              'attrs': {
                'fontFamily': 'RowFlowArial',
                'fontSize': c == 0 ? '11' : '19',
              },
            },
          ],
        },
      ];
    }
    final out = layout([node, f.paragraph('After table')]);
    exactlyOnce(out, 'A', 19);
    exactlyOnce(out, 'B', 19);
    expect(out.notices, isEmpty);
    final recorder = ui.PictureRecorder();
    final canvas = Canvas(recorder)
      ..drawColor(const Color(0xffe8e8e8), BlendMode.src);
    for (var i = 0; i < out.pages.length; i++) {
      canvas.save();
      canvas.translate(i * 312.0, 0);
      canvas.drawRect(
        const Rect.fromLTWH(0, 0, 300, 300),
        Paint()..color = Colors.white,
      );
      for (final p in out.pages[i].content) {
        p.paint(canvas);
      }
      canvas.restore();
    }
    final picture = recorder.endRecording();
    await tester.runAsync(() async {
      final img = await picture.toImage(312 * out.pages.length, 300);
      final png = await img.toByteData(format: ui.ImageByteFormat.png);
      final folder = await Directory.systemTemp.createTemp(
        'folio-row-flow-render-',
      );
      await File(
        '${folder.path}/mixed-fonts.png',
      ).writeAsBytes(png!.buffer.asUint8List());
      // ignore: avoid_print
      print('Row-flow rendering: ${folder.path}/mixed-fonts.png');
      img.dispose();
    });
    picture.dispose();
  });
  test(
    'unequal cell line heights continue independently without omissions or repeats',
    () {
      final out = layout([twoCells(), f.paragraph('After table')]);
      exactlyOnce(out, 'A', 22);
      exactlyOnce(out, 'B', 22);
      expect(out.pages.first.visibleText, contains('A10'));
      expect(out.pages.first.visibleText, isNot(contains('B10')));
      expect(out.pages.last.visibleText, contains('After table'));
      expect(out.notices, isEmpty);
    },
  );
  test(
    'allow splitting uses the current page; keep together moves a fitting row',
    () {
      for (final keep in [false, true]) {
        final node = twoCells(cantSplit: keep, count: 6);
        final out = layout([lines('P', 6), node]);
        expect(out.pages.first.visibleText.contains('A00'), !keep);
        expect(out.pages[1].visibleText, contains('A05'));
        exactlyOnce(out, 'A', 6);
        exactlyOnce(out, 'B', 6);
      }
    },
  );
  test(
    'a keep-together row taller than a page starts fresh and still continues',
    () {
      final out = layout([f.paragraph('Before'), twoCells(cantSplit: true)]);
      expect(out.pages.first.visibleText, 'Before');
      expect(out.pages.length, lessThan(8));
      exactlyOnce(out, 'A', 22);
      exactlyOnce(out, 'B', 22);
    },
  );
  test('direct false overrides an inherited keep-together setting', () {
    final node = twoCells(count: 6);
    node['content'][0]['attrs'] = {'inheritedCantSplit': true};
    expect(
      layout([lines('P', 6), node]).pages.first.visibleText,
      isNot(contains('A00')),
    );
    node['content'][0]['attrs']['cantSplit'] = false;
    expect(
      layout([lines('P', 6), node]).pages.first.visibleText,
      contains('A00'),
    );
  });
  test(
    'nested rows and several paragraphs each keep their own continuation',
    () {
      final nested = h.table([
        for (var i = 0; i < 17; i++)
          h.row([
            h.cell([
              lines('N', 1)
                ..['content'][0]['text'] = 'N${i.toString().padLeft(2, '0')}',
            ]),
          ]),
      ]);
      final node = h.table([
        h.row([
          h.cell([lines('S', 3), nested, lines('T', 3)]),
          h.cell([lines('Q', 25, lineHeight: 345)]),
        ]),
      ], columns: 2);
      final out = layout([node]);
      exactlyOnce(out, 'N', 17);
      exactlyOnce(out, 'S', 3);
      exactlyOnce(out, 'T', 3);
      exactlyOnce(out, 'Q', 25);
      expect(out.notices, isEmpty);
    },
  );
  test(
    'partial rows resume across unequal RTL columns and the final balanced page',
    () {
      for (final rtl in [false, true]) {
        final node = twoCells(count: 43);
        for (final cell in node['content'][0]['content']) {
          cell['content'][0]['content'] = [
            <String, dynamic>{
              ...cell['content'][0]['content'][0],
              'marks': [
                {
                  'type': 'textStyle',
                  'attrs': {'fontSize': '7.5'},
                },
              ],
            },
          ];
        }
        final out = layout(
          [node, f.paragraph('After')],
          sections: [
            columns.twoColumns(0, 0, custom: true, rtl: rtl),
            columns.oneColumn(1, 1),
          ],
        );
        exactlyOnce(out, 'A', 43);
        exactlyOnce(out, 'B', 43);
        expect(
          out.pages.expand((p) => p.columnBands).any((b) => b.balanced),
          true,
        );
        expect(out.notices, isEmpty);
      }
    },
  );
  test('long wrapped paragraphs reshape at a different destination width', () {
    final words = List.generate(
      90,
      (i) => 'W${i.toString().padLeft(2, '0')}',
    ).join(' ');
    final node = h.table([
      h.row([
        h.cell([
          f.paragraph(words, properties: {'widowControl': false}),
        ]),
      ]),
    ]);
    final out = layout(
      [node, f.paragraph('After')],
      sections: [
        columns.twoColumns(0, 0, custom: true),
        columns.oneColumn(1, 1),
      ],
    );
    exactlyOnce(out, 'W', 90);
    expect(out.pages.expand((p) => p.columnBands).any((b) => b.balanced), true);
  });
  test(
    'headers repeat over a split row and page fields belong to each fragment',
    () {
      final node = twoCells(count: 26);
      node['content'].insert(
        0,
        h.row([
          h.cell([f.paragraph('Heading')]),
          h.cell([f.paragraph('Title')]),
        ], header: true),
      );
      for (final cell in node['content'][1]['content']) {
        final content = <Map>[];
        for (var i = 0; i < 26; i++) {
          content.add({
            ...f.paragraph(''),
            'content': [
              {'type': 'text', 'text': 'L${i.toString().padLeft(2, '0')}: '},
              newPageField('PAGE'),
            ],
          });
        }
        cell['content'] = content;
      }
      final out = layout([node]);
      for (final page in out.pages) {
        expect(RegExp('Heading').allMatches(page.visibleText).length, 1);
        final text = page.content
            .where((p) => p.area == 'body')
            .map((p) => p.text)
            .join('\n');
        for (final m in RegExp(r'L\d{2}:\s*(\d+)').allMatches(text)) {
          expect(m[1], '${page.number}');
        }
      }
      expect(RegExp(r'L\d{2}').allMatches(body(out)).length, 52);
      expect(out.notices, isEmpty);
    },
  );
  testWidgets(
    'mixed row settings stay unchanged until the user toggles and applies',
    (tester) async {
      final node = twoCells();
      node['content'].add({
        ...h.row([
          h.cell([f.paragraph('One')]),
          h.cell([f.paragraph('Two')]),
        ]),
        'attrs': {'cantSplit': true},
      });
      final table = DocumentTable(node);
      Map? patch;
      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => TextButton(
              onPressed: () async {
                patch = await showDialog<Map>(
                  context: context,
                  builder: (_) => DocumentTableFormatDialog(
                    table: table,
                    area: const TableArea(0, 0, 1, 1),
                  ),
                );
              },
              child: const Text('Properties'),
            ),
          ),
        ),
      );
      await tester.tap(find.text('Properties'));
      await tester.pumpAndSettle();
      final checkbox = find.byKey(const Key('table-allow-row-break'));
      expect(tester.widget<CheckboxListTile>(checkbox).value, isNull);
      await tester.ensureVisible(checkbox);
      await tester.tap(checkbox);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('table-format-apply')));
      await tester.pumpAndSettle();
      expect(patch, {'cantSplit': false});
      final changed = table.format(
        const TableArea(0, 0, 1, 1),
        cantSplit: patch!['cantSplit'],
      );
      expect(
        changed['content'].every((r) => r['attrs']['cantSplit'] == false),
        true,
      );
      expect(node['content'][0]['attrs'].containsKey('cantSplit'), false);
    },
  );
}
