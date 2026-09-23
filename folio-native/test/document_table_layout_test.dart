import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_table_view.dart';

void main() {
  testWidgets(
    'fixed width, exact/minimum row heights, alignment and clipping render',
    (tester) async {
      tester.view.physicalSize = const Size(600, 400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      Map cell(String text, {String align = 'top'}) => {
        'type': 'tableCell',
        'attrs': {'fill': 'FFF2CC', 'verticalAlign': align},
        'content': [
          {
            'type': 'paragraph',
            // Keep this geometry fixture independent of document-default
            // paragraph after-spacing. B/C fit the deliberately narrow cells.
            'attrs': {
              'paragraphFormat': {
                'inherited': {},
                'direct': {'after': 0},
              },
            },
            'content': [
              {'type': 'text', 'text': text},
            ],
          },
        ],
      };
      final node = {
        'type': 'table',
        'attrs': {
          'layout': 'fixed',
          'alignment': 'right',
          'columnWidths': [900, 900],
        },
        'content': [
          {
            'type': 'tableRow',
            'attrs': {
              'height': {'rule': 'exact', 'value': 300},
            },
            'content': [
              cell('This text is deliberately much taller than the exact row.'),
              cell('Top'),
            ],
          },
          {
            'type': 'tableRow',
            'attrs': {
              'height': {'rule': 'atLeast', 'value': 1200},
            },
            'content': [cell('B', align: 'bottom'), cell('C', align: 'center')],
          },
        ],
      };
      final boundary = GlobalKey();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: 300,
                child: RepaintBoundary(
                  key: boundary,
                  child: DocumentTableView(node: node),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final first = tester.getRect(
        find.byKey(const ValueKey('table-cell-0-0')),
      );
      final last = tester.getRect(find.byKey(const ValueKey('table-cell-1-1')));
      expect(first.left, closeTo(180, .1));
      expect(first.width, 60);
      expect(first.height, 20);
      expect(last.top, 20);
      expect(last.height, 80);
      final bottom = tester.getRect(find.text('B', findRichText: true));
      final center = tester.getRect(find.text('C', findRichText: true));
      expect(bottom.bottom, inExclusiveRange(last.bottom - 15, last.bottom));
      expect(center.center.dy, closeTo(last.center.dy, 3));
      expect(bottom.center.dy, greaterThan(center.center.dy + 10));
      final image = await tester.runAsync(
        () =>
            (boundary.currentContext!.findRenderObject()
                    as RenderRepaintBoundary)
                .toImage(),
      );
      final data = await tester.runAsync(
        () => image!.toByteData(format: ui.ImageByteFormat.rawRgba),
      );
      final offset = (35 * image!.width + 190) * 4;
      expect(data!.buffer.asUint8List().sublist(offset, offset + 4), [
        255,
        242,
        204,
        255,
      ]);
      image.dispose();
      expect(tester.takeException(), isNull);
      // Wide fixed tables remain reachable through horizontal scrolling.
      node['attrs'] = {
        'layout': 'fixed',
        'columnWidths': [4500, 4500],
      };
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SizedBox(width: 300, child: DocumentTableView(node: node)),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final scroll = find.byType(SingleChildScrollView).first;
      expect(
        tester.getRect(find.byKey(const ValueKey('table-cell-0-0'))).width,
        300,
      );
      await tester.drag(scroll, const Offset(-250, 0));
      await tester.pumpAndSettle();
      expect(
        tester.getRect(find.byKey(const ValueKey('table-cell-0-1'))).left,
        lessThan(300),
      );
      expect(tester.takeException(), isNull);
    },
  );
}
