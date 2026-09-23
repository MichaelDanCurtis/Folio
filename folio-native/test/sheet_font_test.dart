import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_font.dart';

void main() {
  test(
    'character previews preserve neighboring fonts and resolve reversed Unicode selections',
    () {
      final source = {
        'fontName': 'Georgia',
        '_fontOverrides': {'bold': true},
        '_richText': [
          {
            'text': 'A🌊',
            'style': {'fontSize': 12, 'color': '#AA0000'},
          },
          {
            'text': 'BC',
            'style': {'fontSize': 22, 'italic': true},
          },
        ],
      };
      const range = TextSelection(baseOffset: 4, extentOffset: 1);
      final selected = selectedSheetFontRuns(
        sheetFontRuns('A🌊BC', source),
        range,
      );
      expect(selected.map((r) => r.text).join(), '🌊B');
      expect(commonSheetFont(selected.map((r) => r.font))['fontSize'], isNull);
      final preview = previewCharacterFont('A🌊BC', source, range, {
        'bold': false,
        'underline': 'double',
      });
      final runs = sheetFontRuns('A🌊BC', preview);
      expect(runs.map((r) => r.text), ['A', '🌊', 'B', 'C']);
      expect(runs.map((r) => r.font['bold']), [true, false, false, true]);
      expect(runs.map((r) => r.font['fontSize']), [12, 12, 22, 22]);
      expect(runs.map((r) => r.font['underline']), [
        'none',
        'double',
        'double',
        'none',
      ]);
      expect(runs[3].font['italic'], true);
      expect(source['_fontOverrides'], {'bold': true});
    },
  );
  test(
    'rich runs inherit, override, summarize and reject stale character positions',
    () {
      final style = {
        'fontName': 'Georgia',
        'fontSize': 18,
        'bold': true,
        '_richText': [
          {'text': 'A', 'style': <String, dynamic>{}},
          {
            'text': '🌊B',
            'style': {'bold': false, 'fontSize': 12, 'color': '#FF0000'},
          },
        ],
      };
      var runs = sheetFontRuns('A🌊B', style);
      expect(runs[0].font['bold'], true);
      expect(runs[1].font['fontName'], 'Georgia');
      expect(runs[1].font['bold'], false);
      final common = commonSheetFont(runs.map((r) => r.font));
      expect(common['fontName'], 'Georgia');
      expect(common['fontSize'], isNull);
      expect(common['bold'], isNull);
      runs = sheetFontRuns('A🌊B', {
        ...style,
        '_fontOverrides': {'bold': false},
      });
      expect(commonSheetFont(runs.map((r) => r.font))['bold'], false);
      expect(runs[1].font['color'], '#FF0000');
      runs = sheetFontRuns('A replacement', style);
      expect(runs.length, 1);
      expect(runs.single.text, 'A replacement');
      expect(runs.single.font['bold'], true);
    },
  );
  testWidgets(
    'mixed runs paint individual sizes, colors, underline and raised or lowered text',
    (tester) async {
      tester.view.physicalSize = const Size(800, 600);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      Future<void> frame(
        String vertical, {
        String underline = 'doubleAccounting',
        double scale = 1,
      }) => tester.pumpWidget(
        MaterialApp(
          home: MediaQuery(
            data: MediaQueryData(textScaler: TextScaler.linear(scale)),
            child: Align(
              alignment: Alignment.topLeft,
              child: RepaintBoundary(
                key: const Key('rich-pixels'),
                child: ColoredBox(
                  color: Colors.white,
                  child: SizedBox(
                    width: 240,
                    height: 100,
                    child: SheetFontText(
                      'A2B',
                      key: const Key('rich'),
                      font: {
                        'fontName': 'Ahem',
                        'fontSize': 24,
                        '_richText': [
                          {
                            'text': 'A',
                            'style': {'color': '#000000'},
                          },
                          {
                            'text': '2',
                            'style': {
                              'color': '#FF0000',
                              'verticalAlign': vertical,
                            },
                          },
                          {
                            'text': 'B',
                            'style': {
                              'color': '#0000FF',
                              'fontSize': 12,
                              'underline': underline,
                            },
                          },
                        ],
                      },
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      Future<({Rect red, Rect black, Rect blue})> bounds() async {
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('rich-pixels')),
        );
        final image = await boundary.toImage(pixelRatio: 1);
        final bytes = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        final points = [<Offset>[], <Offset>[], <Offset>[]];
        for (var y = 0; y < image.height; y++) {
          for (var x = 0; x < image.width; x++) {
            final i = (y * image.width + x) * 4;
            final r = bytes.getUint8(i),
                g = bytes.getUint8(i + 1),
                b = bytes.getUint8(i + 2);
            final group = r > 180 && g < 80 && b < 80
                ? 0
                : r < 80 && g < 80 && b < 80
                ? 1
                : b > 180 && g < 180 && r < 180
                ? 2
                : -1;
            if (group >= 0) {
              points[group].add(Offset(x.toDouble(), y.toDouble()));
            }
          }
        }
        image.dispose();
        Rect rect(List<Offset> p) {
          expect(p, isNotEmpty);
          return p
              .map((v) => Rect.fromLTWH(v.dx, v.dy, 1, 1))
              .reduce((a, b) => a.expandToInclude(b));
        }

        return (
          red: rect(points[0]),
          black: rect(points[1]),
          blue: rect(points[2]),
        );
      }

      await frame('superscript');
      final box = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('rich')),
      );
      final raised = (await tester.runAsync(bounds))!;
      expect(raised.red.center.dy, lessThan(raised.black.center.dy));
      expect(raised.red.height, lessThan(raised.black.height));
      // A fractional underline edge can antialias into the adjacent pixel.
      expect(raised.blue.left, greaterThanOrEqualTo(raised.red.right - 1));
      final layouts = box.layoutCount;
      box.markNeedsPaint();
      await tester.pump();
      expect(box.layoutCount, layouts);
      await frame('subscript');
      final lowered = (await tester.runAsync(bounds))!;
      expect(lowered.red.top, greaterThan(raised.red.top));
      expect(lowered.black, raised.black);
      expect(lowered.blue, raised.blue);
      await frame('subscript', underline: 'none');
      final plain = (await tester.runAsync(bounds))!;
      expect(plain.blue.bottom, lessThan(lowered.blue.bottom));
      expect(plain.red, lowered.red);
      expect(plain.black, lowered.black);
      await frame('subscript', scale: 2);
      expect(box.paintedTextWidth, greaterThan(raised.blue.right));
      final semantics = tester.ensureSemantics();
      expect(find.bySemanticsLabel('A2B'), findsOneWidget);
      semantics.dispose();
      expect(tester.takeException(), isNull);
    },
  );
  test(
    'effective cell fonts resolve defaults, column and row styles and direct overrides',
    () {
      final sheet = {
        'styleDefaults': {
          'default': {'fontName': 'Calibri', 'fontSize': 11},
          'columns': [
            {
              'min': 2,
              'max': 3,
              'style': {'fontName': 'Georgia', 'fontSize': 18, 'italic': true},
            },
          ],
          'rows': {
            '7': {'fontName': 'Verdana', 'bold': true},
          },
        },
        'officeStyles': {'B7': '0'},
        'cellStyles': {
          'B7': {'fontName': 'Calibri', 'bold': false},
          'C7': {'italic': false},
        },
      };
      expect(cellStyle(sheet, 'B2')['fontName'], 'Georgia');
      expect(cellStyle(sheet, 'A7')['fontName'], 'Verdana');
      expect(cellStyle(sheet, 'B7')['fontName'], 'Calibri');
      expect(cellStyle(sheet, 'C7')['fontName'], 'Verdana');
      expect(cellStyle(sheet, 'C7')['italic'], false);
    },
  );
  testWidgets(
    'clipped script runs do not allocate invisible paragraph layers',
    (tester) async {
      final text = 'Normal ' * 100;
      await tester.pumpWidget(
        MaterialApp(
          home: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 80,
              height: 32,
              child: SheetFontText(
                '$text${'x' * 100}',
                key: const Key('long-rich'),
                font: {
                  'fontName': 'Ahem',
                  '_richText': [
                    {'text': text, 'style': <String, dynamic>{}},
                    for (var i = 0; i < 100; i++)
                      {
                        'text': 'x',
                        'style': {
                          'fontSize': 10 + i / 2,
                          'verticalAlign': 'superscript',
                        },
                      },
                  ],
                },
              ),
            ),
          ),
        ),
      );
      final box = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('long-rich')),
      );
      expect(box.paintLayerCount, 1);
      final count = box.layoutCount;
      box.markNeedsPaint();
      await tester.pump();
      expect(box.layoutCount, count);
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets(
    'font drawing scales, clips, paints double accounting lines and reuses layout on repaint',
    (tester) async {
      tester.view.physicalSize = const Size(800, 600);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      Future<void> frame(Map font, {double scale = 1, double width = 200}) =>
          tester.pumpWidget(
            MaterialApp(
              home: MediaQuery(
                data: MediaQueryData(textScaler: TextScaler.linear(scale)),
                child: Align(
                  alignment: Alignment.topLeft,
                  child: RepaintBoundary(
                    key: const Key('pixels'),
                    child: ColoredBox(
                      color: Colors.white,
                      child: SizedBox(
                        width: width,
                        height: 64,
                        child: SheetFontText(
                          'A',
                          font: font,
                          key: const Key('text'),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          );
      final font = {
        'fontName': 'Ahem',
        'fontSize': 12,
        'underline': 'doubleAccounting',
        'strike': true,
        'color': '#000000',
      };
      await frame(font);
      final box = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('text')),
      );
      expect(box.textStyle.fontSize, 16);
      expect(box.textStyle.decoration, TextDecoration.lineThrough);
      expect(box.paintedTextWidth, greaterThan(0));
      final layouts = box.layoutCount;
      box.markNeedsPaint();
      await tester.pump();
      expect(box.layoutCount, layouts);
      Future<List<int>> darkRows() async {
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('pixels')),
        );
        final image = await boundary.toImage(pixelRatio: 1);
        final bytes = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        final result = [
          for (var y = 0; y < 64; y++)
            if (bytes.getUint8((y * image.width + 190) * 4) < 128) y,
        ];
        image.dispose();
        return result;
      }

      final doubleRows = (await tester.runAsync(darkRows))!;
      expect(doubleRows.length, greaterThanOrEqualTo(2));
      expect(doubleRows.last - doubleRows.first, greaterThan(1));
      await frame({...font, 'underline': 'single'});
      expect((await tester.runAsync(darkRows))!, isEmpty);
      await frame({...font, 'verticalAlign': 'superscript'}, scale: 2);
      expect(box.fontPixels, 32);
      expect(box.baselineShift, lessThan(0));
      expect(box.layoutCount, greaterThan(layouts));
      await frame({
        ...font,
        'verticalAlign': 'subscript',
        'fontSize': 409,
      }, width: 20);
      expect(box.size, const Size(20, 64));
      expect(box.baselineShift, greaterThan(0));
      expect(tester.takeException(), isNull);
      final semantics = tester.ensureSemantics();
      expect(find.bySemanticsLabel('A'), findsOneWidget);
      semantics.dispose();
    },
  );
}
