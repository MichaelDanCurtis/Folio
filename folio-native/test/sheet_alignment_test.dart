import 'dart:math' as math;
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_font.dart';
import 'package:folio_native/src/sheet_alignment_dialog.dart';
import 'package:folio_native/src/sheet_view.dart';

void main() {
  testWidgets(
    'wide and compact ribbons expose alignment without clipping other controls',
    (tester) async {
      tester.view.physicalSize = const Size(1200, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      Map<String, dynamic>? change;
      Future<void> show(double width) async {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: Center(
                child: SizedBox(
                  width: width,
                  child: SheetView(
                    sheet: const {
                      'id': 's',
                      'name': 'Sheet',
                      'cells': {'A1': 'Text'},
                    },
                    calculated: const {'A1': 'Text'},
                    onSelection: (_) {},
                    onChanges: (_) async {},
                    onFormat: (addresses, style, {baseRevision}) async {
                      change = style;
                    },
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
      }

      await show(1100);
      expect(find.byTooltip('Align right'), findsOneWidget);
      await tester.tap(find.byTooltip('Align right'));
      await tester.pumpAndSettle();
      expect(change, {'horizontalAlignment': 'right'});
      expect(tester.takeException(), isNull);
      await show(700);
      expect(find.byTooltip('Align right'), findsNothing);
      expect(
        find.byKey(const Key('cell-font-button')).hitTestable(),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('cell-alignment-button')).hitTestable(),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('cell-alignment-button')));
      await tester.pumpAndSettle();
      expect(find.text('Alignment'), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    },
  );
  testWidgets(
    'top-aligned raised text paints complete glyphs without edge clipping',
    (tester) async {
      Future<int> ink(String vertical) async {
        await tester.pumpWidget(
          MaterialApp(
            home: Center(
              child: RepaintBoundary(
                key: const Key('pixels'),
                child: ColoredBox(
                  color: Colors.white,
                  child: SizedBox(
                    width: 180,
                    height: 100,
                    child: SheetFontText(
                      'ABC',
                      font: const {
                        'fontSize': 20,
                        'verticalAlign': 'superscript',
                      },
                      layout: CellTextLayout({
                        'verticalAlignment': vertical,
                        'horizontalAlignment': 'center',
                      }),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('pixels')),
        );
        return (await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(
            format: ui.ImageByteFormat.rawRgba,
          );
          image.dispose();
          var count = 0;
          for (var at = 0; at < bytes!.lengthInBytes; at += 4) {
            if (bytes.getUint8(at) < 160) count++;
          }
          return count;
        }))!;
      }

      final centered = await ink('center'), top = await ink('top');
      expect(centered, greaterThan(100));
      expect(top, closeTo(centered, centered * .04));
      expect(tester.takeException(), isNull);
    },
  );
  Future<SheetFontRenderBox> cell(
    WidgetTester tester,
    String text,
    Map style, {
    dynamic value,
    double width = 180,
    double height = 100,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Center(
          child: SizedBox(
            width: width,
            height: height,
            child: SheetFontText(
              text,
              key: const Key('cell'),
              font: style,
              layout: CellTextLayout(style, value: value),
            ),
          ),
        ),
      ),
    );
    return tester.renderObject<SheetFontRenderBox>(
      find.byKey(const Key('cell')),
    );
  }

  testWidgets(
    'cell alignment resolves value types, direction, indent and vertical position',
    (tester) async {
      var box = await cell(tester, '123', {}, value: 123);
      expect(box.horizontalAlignment, 'right');
      expect(box.textBounds.right, closeTo(180, .01));
      expect(box.textBounds.bottom, closeTo(100, .01));
      box = await cell(tester, '123', {}, value: '123');
      expect(box.horizontalAlignment, 'left');
      expect(box.textBounds.left, 0);
      box = await cell(tester, 'TRUE', {}, value: true);
      expect(box.textBounds.center.dx, closeTo(90, .01));
      box = await cell(tester, 'abc', {
        'horizontalAlignment': 'center',
        'verticalAlignment': 'top',
      });
      expect(box.textBounds.center.dx, closeTo(90, .01));
      expect(box.textBounds.top, 0);
      box = await cell(tester, 'abc', {
        'horizontalAlignment': 'right',
        'verticalAlignment': 'center',
        'indent': 2,
      });
      expect(box.textBounds.center.dy, closeTo(50, .01));
      expect(box.textBounds.right, lessThan(180));
      expect(box.textBounds.right, greaterThan(50));
      box = await cell(tester, 'שלום', {});
      expect(box.readingDirection, TextDirection.rtl);
      expect(box.horizontalAlignment, 'right');
      box = await cell(tester, 'שלום', {'readingOrder': 1});
      expect(box.readingDirection, TextDirection.ltr);
      expect(box.horizontalAlignment, 'left');
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets(
    'wrapping follows width, manual breaks remain and shrink preserves font size',
    (tester) async {
      const text = 'A longer heading that needs several lines';
      var box = await cell(
        tester,
        text,
        {'wrapText': true, 'verticalAlignment': 'top'},
        width: 140,
        height: 150,
      );
      final narrow = box.lineCount;
      expect(narrow, greaterThan(1));
      expect(box.paintScale, 1);
      box = await cell(
        tester,
        text,
        {'wrapText': true},
        width: 300,
        height: 150,
      );
      expect(box.lineCount, lessThan(narrow));
      box = await cell(tester, 'First\nSecond', {});
      expect(box.lineCount, 2);
      box = await cell(tester, text, {
        'shrinkToFit': true,
        'horizontalAlignment': 'right',
      }, width: 80);
      expect(box.paintScale, lessThan(1));
      expect(box.paintScale, greaterThan(0));
      expect(box.textBounds.width, closeTo(80, .01));
      expect(box.runs.single.font['fontSize'], 11);
      box = await cell(tester, 'First\nSecond', {
        'shrinkToFit': true,
      }, width: 30);
      expect(box.paintScale, 1);
      box = await cell(
        tester,
        text,
        {'wrapText': true, 'shrinkToFit': true},
        width: 80,
        height: 200,
      );
      expect(box.paintScale, 1);
      expect(box.lineCount, greaterThan(1));
      final count = box.layoutCount;
      box.markNeedsPaint();
      await tester.pump();
      expect(box.layoutCount, count);
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets(
    'rotations use signed angles and stacked text keeps grapheme clusters',
    (tester) async {
      var box = await cell(tester, 'AB', {
        'textRotation': 90,
        'horizontalAlignment': 'center',
        'verticalAlignment': 'center',
      });
      expect(box.paintRotation, closeTo(-math.pi / 2, .0001));
      expect(box.textBounds.center.dx, closeTo(90, .01));
      expect(box.textBounds.center.dy, closeTo(50, .01));
      final first = box.textBounds.size;
      box = await cell(tester, 'AB', {'textRotation': -90});
      expect(box.paintRotation, closeTo(math.pi / 2, .0001));
      expect(box.textBounds.width, closeTo(first.width, .0001));
      expect(box.textBounds.height, closeTo(first.height, .0001));
      box = await cell(tester, 'AB', {'textRotation': 45});
      expect(box.paintRotation, closeTo(-math.pi / 4, .0001));
      box = await cell(tester, 'A🌊e\u0301', {'textRotation': 255});
      expect(box.lineCount, 3);
      expect(box.paintRotation, 0);
      expect(box.runs.single.text, 'A🌊e\u0301');
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets(
    'alignment dialog keeps mixed settings and invalid drafts cannot apply',
    (tester) async {
      Map<String, dynamic>? result;
      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => TextButton(
              onPressed: () async {
                result = await showDialog<Map<String, dynamic>>(
                  context: context,
                  builder: (_) => const SheetAlignmentDialog(
                    styles: [
                      {'horizontalAlignment': 'left', 'wrapText': true},
                      {'horizontalAlignment': 'right', 'wrapText': false},
                    ],
                    previewStyle: {},
                    sample: 'Preview text',
                  ),
                );
              },
              child: const Text('Open'),
            ),
          ),
        ),
      );
      await tester.tap(find.text('Open'));
      await tester.pumpAndSettle();
      expect(
        find.textContaining('Mixed settings stay unchanged'),
        findsOneWidget,
      );
      await tester.enterText(find.byKey(const Key('alignment-degrees')), '91');
      await tester.pump();
      await tester.tap(find.byKey(const Key('alignment-wrapText')));
      await tester.pump();
      expect(
        tester
            .widget<FilledButton>(find.byKey(const Key('apply-cell-alignment')))
            .onPressed,
        isNull,
      );
      await tester.enterText(find.byKey(const Key('alignment-degrees')), '-45');
      await tester.pump();
      await tester.tap(find.byKey(const Key('apply-cell-alignment')));
      await tester.pumpAndSettle();
      expect(result, {
        'wrapText': true,
        'shrinkToFit': false,
        'textRotation': -45,
      });
      expect(tester.takeException(), isNull);
    },
  );
}
