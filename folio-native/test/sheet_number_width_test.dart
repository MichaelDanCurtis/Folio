import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_font.dart';
import 'package:folio_native/src/sheet_number_format.dart';
import 'package:folio_native/src/sheet_number_width.dart';

void main() {
  test(
    'General reduces decimals, then uses scientific notation without false zero',
    () {
      String? fit(num n, String full, int width) => generalWidthCandidates(
        n,
        full,
      ).where((s) => s.length <= width).firstOrNull;
      expect(fit(25.76, '25.76', 4), '25.8');
      expect(fit(25.76, '25.76', 2), '26');
      expect(fit(-25.76, '-25.76', 3), '-26');
      expect(fit(123456789, '123456789', 7), '1.2E+08');
      expect(fit(123456789, '123456789', 5), '1E+08');
      expect(fit(0.00001234, '0.00001234', 5), '1E-05');
      expect(fit(0.00001234, '0.00001234', 4), isNull);
      expect(fit(9.99e15, '9.99E+15', 5), '1E+16');
      expect(generalWidthCandidates(0, '0'), isEmpty);
      expect(generalWidthCandidates(double.infinity, 'Infinity'), isEmpty);
      final small = generalWidthCandidates(5e-324, '5E-324').toList();
      expect(small, isEmpty);
    },
  );

  testWidgets(
    'fixed numbers hash at narrow widths and restore full text and semantics',
    (tester) async {
      final semantics = tester.ensureSemantics();
      try {
        Future<void> frame(double width, {double scale = 1}) =>
            tester.pumpWidget(
              MaterialApp(
                home: MediaQuery(
                  data: MediaQueryData(textScaler: TextScaler.linear(scale)),
                  child: Align(
                    alignment: Alignment.topLeft,
                    child: SizedBox(
                      width: width,
                      height: 40,
                      child: SheetFontText(
                        '12345.67',
                        key: const Key('value'),
                        font: const {'fontName': 'Ahem', 'fontSize': 10},
                        layout: CellTextLayout({
                          '_numberWidth': 'fixed',
                        }, value: 12345.67),
                      ),
                    ),
                  ),
                ),
              ),
            );
        await frame(60);
        var box = tester.renderObject<SheetFontRenderBox>(
          find.byKey(const Key('value')),
        );
        expect(box.paintedText, '####');
        expect(box.textBounds.width, lessThanOrEqualTo(60));
        expect(
          tester.getSemantics(find.byKey(const Key('value'))).label,
          '12345.67',
        );
        await frame(130);
        expect(box.paintedText, '12345.67');
        await frame(130, scale: 2);
        expect(box.paintedText, '####');
        expect(box.runs.single.text, '12345.67');
      } finally {
        semantics.dispose();
      }
    },
  );

  testWidgets(
    'width fitting respects General, text, errors, shrink and natural AutoFit',
    (tester) async {
      Future<SheetFontRenderBox> frame(
        String text,
        dynamic value,
        Map style,
      ) async {
        await tester.pumpWidget(
          MaterialApp(
            home: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: 30,
                height: 40,
                child: SheetFontText(
                  text,
                  key: const Key('value'),
                  font: {'fontName': 'Ahem', 'fontSize': 10, ...style},
                  layout: CellTextLayout(style, value: value),
                ),
              ),
            ),
          ),
        );
        return tester.renderObject<SheetFontRenderBox>(
          find.byKey(const Key('value')),
        );
      }

      var box = await frame('25.76', 25.76, {'_numberWidth': 'general'});
      expect(box.paintedText, '26');
      expect(box.runs.single.text, '25.76');
      box = await frame('25.76', '25.76', {});
      expect(box.paintedText, '25.76');
      box = await frame('#DIV/0!', '#DIV/0!', {});
      expect(box.paintedText, '#DIV/0!');
      box = await frame('25.7600', 25.76, {
        '_numberWidth': 'fixed',
        'shrinkToFit': true,
      });
      expect(box.paintedText, '25.7600');
      expect(box.paintScale, lessThan(1));
      box = await frame('########', -1, {
        '_numberWidth': 'invalidDate',
        'shrinkToFit': true,
      });
      expect(box.paintedText, '##');
      box = await frame('', 25.76, {});
      expect(box.paintedText, isEmpty);
      final natural = SheetFontRenderBox(
        sheetFontRuns('12345.67', {'fontName': 'Ahem', 'fontSize': 10}),
        Colors.black,
        TextDirection.ltr,
        TextScaler.noScaling,
        CellTextLayout({'_numberWidth': 'fixed'}, value: 12345.67),
      );
      expect(natural.autoFitWidth(), greaterThan(100));
      expect(natural.runs.single.text, '12345.67');
      natural.dispose();
    },
  );

  testWidgets(
    'accounting reserves spacing but hashes remain visible in the effective color',
    (tester) async {
      final style = numberDisplayStyle(
        {'fontName': 'Ahem', 'fontSize': 10},
        {
          'numberWidth': 'fixed',
          'color': '#FF0000',
          'segments': [
            {'kind': 'skip', 'text': '('},
            {'kind': 'text', 'text': r'$'},
            {'kind': 'fill', 'text': ' '},
            {'kind': 'text', 'text': '1,234.50'},
            {'kind': 'skip', 'text': ')'},
          ],
        },
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 60,
              height: 40,
              child: SheetFontText(
                r' $1,234.50 ',
                font: style,
                key: const Key('value'),
                layout: CellTextLayout(style, value: 1234.5),
              ),
            ),
          ),
        ),
      );
      final box = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('value')),
      );
      expect(box.paintedText, '####');
      expect(box.textBounds.width, lessThanOrEqualTo(60));
      expect(box.runs[1].font['color'], '#FF0000');
    },
  );
}
