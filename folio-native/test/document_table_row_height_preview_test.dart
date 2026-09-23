import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_fields.dart';
import 'package:folio_native/src/document_page_layout.dart';
import 'package:folio_native/src/document_table_view.dart';
import 'document_page_layout_test.dart' as f;

Map paragraph(String text) => f.paragraph(
  text,
  properties: {'before': 0, 'line': 300, 'lineRule': 'exact'},
);
Map cell(List content, {int rows = 1, String? fill, String align = 'top'}) => {
  'type': 'tableCell',
  'attrs': {
    'rowspan': rows,
    'colspan': 1,
    'verticalAlign': align,
    'fill': ?fill,
  },
  'content': content,
};
Map row(
  List cells, {
  String rule = 'auto',
  int value = 0,
  bool header = false,
}) => {
  'type': 'tableRow',
  'attrs': {
    'height': {'rule': rule, 'value': value},
    'header': header,
  },
  'content': cells,
};
Map table(List rows, {int columns = 1}) => {
  'type': 'table',
  'attrs': {'columnWidths': List.filled(columns, 2400)},
  'content': rows,
};
DocumentPageLayout layout(List content, {Map? section}) {
  final snapshot = f.snapshot(content, [
    section ?? f.section(0, content.length - 1),
  ]);
  final before = jsonEncode(snapshot);
  final result = DocumentPageLayout.build(snapshot);
  addTearDown(result.dispose);
  expect(result.converged, true);
  expect(jsonEncode(snapshot), before);
  return result;
}

double tableHeight(DocumentPageLayout result) => result.pages
    .expand((p) => p.content)
    .where((p) => p.bodyIndex == 0 && p.area == 'body')
    .fold<double>(0, (a, p) => a + p.height);

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'exact, minimum and automatic heights follow content and stored rules',
    () {
      for (final (rule, value, expected) in [
        ('exact', 15, 1.0),
        ('exact', 450, 30.0),
        ('atLeast', 150, 36.0),
        ('atLeast', 1200, 80.0),
        ('auto', 9000, 36.0),
        ('auto', 0, 36.0),
      ]) {
        final result = layout([
          table([
            row(
              [
                cell([paragraph('One')]),
              ],
              rule: rule,
              value: value,
            ),
          ]),
          paragraph('After table'),
        ]);
        expect(result.pages.length, 1, reason: '$rule $value');
        expect(
          tableHeight(result),
          closeTo(expected, .01),
          reason: '$rule $value',
        );
        final after = result.pages.single.content.singleWhere(
          (p) => p.bodyIndex == 1,
        );
        expect(
          after.offset.dy,
          closeTo(result.pages.single.top + expected, .01),
        );
        expect(result.notices, isEmpty);
      }
    },
  );

  test(
    'overflow in exact rows cannot create extra pages or visible hidden lines',
    () {
      final result = layout([
        table([
          for (var i = 0; i < 8; i++)
            row(
              [
                cell([
                  paragraph(
                    'Row$i\nContinued\n${List.filled(50, 'Hidden$i').join('\n')}',
                  ),
                ]),
              ],
              rule: 'exact',
              value: 600,
            ),
        ]),
        paragraph('After table'),
      ]);
      expect(result.pages.length, 2);
      expect(tableHeight(result), 320);
      expect(result.pages.first.content.single.height, 240);
      final text = result.pages.map((p) => p.visibleText).join('\n');
      for (var i = 0; i < 8; i++) {
        expect('Row$i'.allMatches(text).length, 1);
        expect(text, isNot(contains('Hidden$i')));
      }
      expect(result.pages.last.visibleText, contains('After table'));
      expect(result.notices, isEmpty);
    },
  );

  test(
    'automatic and small minimum rows have no artificial 36-pixel floor',
    () {
      for (final rule in ['auto', 'atLeast']) {
        final result = layout([
          table([
            row(
              [
                cell([
                  f.paragraph(
                    'Tiny',
                    properties: {'before': 0, 'line': 150, 'lineRule': 'exact'},
                  ),
                ]),
              ],
              rule: rule,
              value: rule == 'auto' ? 9000 : 75,
            ),
          ]),
        ]);
        expect(tableHeight(result), closeTo(26, .01));
      }
    },
  );

  testWidgets(
    'merged overflow grows only flexible rows in editor and preview',
    (tester) async {
      for (final allExact in [false, true]) {
        final node = table([
          row(
            [
              cell([paragraph('M1\nM2\nM3\nM4\nM5')], rows: 3),
              cell([paragraph('A')]),
            ],
            rule: 'exact',
            value: 300,
          ),
          row(
            [
              cell([paragraph('B')]),
            ],
            rule: allExact ? 'exact' : 'atLeast',
            value: 450,
          ),
          row(
            [
              cell([paragraph('C\nD')]),
            ],
            rule: allExact ? 'exact' : 'auto',
            value: 600,
          ),
        ], columns: 2);
        final result = layout([node]);
        expect(tableHeight(result), closeTo(allExact ? 90 : 116, .01));
        if (allExact) {
          expect(result.pages.single.visibleText, isNot(contains('M5')));
        }
        await tester.pumpWidget(
          MaterialApp(
            home: SizedBox(
              width: 260,
              child: Align(
                alignment: Alignment.topLeft,
                child: DocumentTableView(node: node),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final fixed = tester.getRect(
          find.byKey(const ValueKey('table-cell-0-1')),
        );
        final merged = tester.getRect(
          find.byKey(const ValueKey('table-cell-0-0')),
        );
        expect(fixed.height, 20);
        if (allExact) {
          expect(merged.height, 90);
        } else {
          expect(merged.height, greaterThan(90));
          expect(
            tester.getRect(find.byKey(const ValueKey('table-cell-1-1'))).height,
            greaterThanOrEqualTo(30),
          );
          expect(
            tester.getRect(find.byKey(const ValueKey('table-cell-2-1'))).height,
            greaterThan(40),
          );
        }
        expect(tester.takeException(), isNull);
      }
    },
  );

  test(
    'center and bottom alignment clip the same part of overflowing content as the editor',
    () {
      final result = layout([
        table([
          row(
            [
              for (final align in ['top', 'center', 'bottom'])
                cell([paragraph('One\nTwo\nThree')], align: align),
            ],
            rule: 'exact',
            value: 540,
          ),
        ], columns: 3),
      ], section: f.section(0, 0, page: f.geometry(width: 9000)));
      expect(tableHeight(result), 36);
      expect(result.pages.single.visibleText.split('\n'), [
        'One',
        'Two',
        'Three',
      ]);
    },
  );

  test(
    'nested clipping survives intervening paragraphs and ancestor offsets',
    () {
      final nested = table([
        row(
          [
            cell([paragraph('Visible\nHidden\nHidden')]),
          ],
          rule: 'exact',
          value: 540,
        ),
      ]);
      final node = table([
        row([
          cell([paragraph('Prefix'), nested, paragraph('Suffix')]),
        ]),
      ]);
      final result = layout([node]);
      expect(
        result.pages.single.visibleText,
        contains('Prefix\nVisible\nSuffix'),
      );
      expect(result.pages.single.visibleText, isNot(contains('Hidden')));
      expect(tableHeight(result), 92);
    },
  );

  test(
    'exact repeating headings keep their height and derived page fields',
    () {
      final heading = paragraph('Heading');
      heading['content'] = [...heading['content'], newPageField('PAGE')];
      final result = layout([
        table([
          row(
            [
              cell([heading, paragraph('Clipped heading')]),
            ],
            rule: 'exact',
            value: 540,
            header: true,
          ),
          for (var i = 0; i < 15; i++)
            row(
              [
                cell([paragraph('Body$i')]),
              ],
              rule: 'exact',
              value: 540,
            ),
        ]),
      ]);
      expect(result.pages.length, 3);
      for (var i = 0; i < result.pages.length; i++) {
        final page = result.pages[i];
        expect(
          page.visibleText.replaceAll(RegExp(r'\s+'), ''),
          contains('Heading${i + 1}'),
        );
        expect(page.visibleText, isNot(contains('Clipped')));
        if (i > 0) expect(page.content.first.height, 36);
      }
      expect(result.notices, isEmpty);
    },
  );

  testWidgets(
    'nested fill, borders and glyphs stay inside the exact parent cell on canvas',
    (tester) async {
      await tester.runAsync(() async {
        final font = File('/System/Library/Fonts/Supplemental/Arial.ttf');
        if (await font.exists()) {
          await (FontLoader(
            'Aptos',
          )..addFont(font.readAsBytes().then(ByteData.sublistView))).load();
        }
      });
      final nested = table([
        row([
          cell([
            paragraph(List.filled(12, 'Overflow').join('\n')),
          ], fill: 'FF0000'),
        ]),
      ]);
      final node = table([
        row(
          [
            cell([nested], fill: 'D9EAF7'),
          ],
          rule: 'exact',
          value: 450,
        ),
        row(
          [
            cell([paragraph('')], fill: 'FFF2CC'),
          ],
          rule: 'exact',
          value: 900,
        ),
      ]);
      final result = layout([node, paragraph('After table')]);
      final page = result.pages.single;
      final recorder = ui.PictureRecorder();
      final canvas = Canvas(recorder)..drawColor(Colors.white, BlendMode.src);
      for (final placement in page.content) {
        placement.paint(canvas);
      }
      final picture = recorder.endRecording();
      final image = await tester.runAsync(() => picture.toImage(300, 300));
      final bytes = await tester.runAsync(
        () => image!.toByteData(format: ui.ImageByteFormat.rawRgba),
      );
      final rgba = bytes!.buffer.asUint8List();
      // Scan the following row, including where overflowing nested borders and
      // red fill would have been painted without ancestor clipping.
      for (var y = 52; y < 108; y++) {
        for (var x = 22; x < 278; x++) {
          final offset = (y * image!.width + x) * 4;
          expect(rgba.sublist(offset, offset + 4), [
            255,
            242,
            204,
            255,
          ], reason: '$x,$y');
        }
      }
      expect('Overflow'.allMatches(page.visibleText).length, 1);
      // Retain the rendered result for visual review, outside the source tree.
      await tester.runAsync(() async {
        final folder = await Directory.systemTemp.createTemp(
          'folio-row-heights-',
        );
        final png = await image!.toByteData(format: ui.ImageByteFormat.png);
        await File(
          '${folder.path}/nested-clipping.png',
        ).writeAsBytes(png!.buffer.asUint8List());
        // ignore: avoid_print
        print('Row-height rendering: ${folder.path}/nested-clipping.png');
      });
      image!.dispose();
      picture.dispose();
    },
  );
}
