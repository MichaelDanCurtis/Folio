import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_column_metrics.dart';
import 'package:folio_native/src/sheet_autofit.dart';
import 'package:folio_native/src/sheet_font.dart';
import 'package:folio_native/src/table_controls.dart';

void main() {
  test(
    'Excel width conversions match the published 96dpi example and round upward',
    () {
      const metrics = SheetColumnMetrics(7);
      expect(metrics.fromCharacters(8), 8.7109375);
      expect(metrics.pixels(8.7109375), 61);
      expect(metrics.characters(8.7109375), 8);
      for (final digit in [5.0, 7.0, 11.0, 40.0]) {
        final m = SheetColumnMetrics(digit);
        for (final px in [1.0, 3.5, 17.0, 61.1, 245.0, 1000.0]) {
          final width = m.fitPixels(px);
          expect(m.pixels(width), greaterThanOrEqualTo(px));
          if (width > 1 / 256) {
            expect(m.pixels(width - 1 / 256), lessThan(px.ceil()));
          }
        }
        expect(m.fitPixels(100000), 255);
        expect(m.fromCharacters(m.maxCharacters), lessThanOrEqualTo(255));
      }
      expect(metrics.pixels(0), 0);
      expect(metrics.fromCharacters(0), 0);
    },
  );
  testWidgets(
    'font-dependent columns measure natural lines, rich fonts, indents, rotation and number formats',
    (tester) async {
      final small = SheetColumnMetrics.fromFont({'fontSize': 11});
      final large = SheetColumnMetrics.fromFont({'fontSize': 22});
      expect(large.digitWidth, greaterThan(small.digitWidth));
      Future<double> width(String text, Map<String, dynamic> style) async {
        final box = SheetFontRenderBox(
          sheetFontRuns(text, style),
          Colors.black,
          TextDirection.ltr,
          TextScaler.noScaling,
          CellTextLayout(style),
        );
        final result = box.autoFitWidth();
        box.dispose();
        return result;
      }

      final natural = await width('A long line of text', {});
      expect(
        await width('A long line of text', {
          'wrapText': true,
          'shrinkToFit': true,
        }),
        natural,
      );
      expect(
        await width('short\nA long line of text', {}),
        closeTo(natural, .001),
      );
      expect(
        await width('mixed text', {
          '_richText': [
            {
              'text': 'mixed ',
              'style': {'fontSize': 30},
            },
            {
              'text': 'text',
              'style': {'fontSize': 11},
            },
          ],
        }),
        greaterThan(await width('mixed text', {})),
      );
      expect(
        await width('A long line of text', {'textRotation': 90}),
        lessThan(natural),
      );
      expect(
        await width('A long line of text', {'textRotation': 255}),
        lessThan(natural),
      );
      expect(
        await width('text', {'horizontalAlignment': 'left', 'indent': 2}),
        greaterThan(await width('text', {})),
      );
      final sheet = {
        'cells': {
          'A1': 'x',
          'A200': 'offscreen content',
          'B1': 'x',
          'B2': 'Hidden ' * 200,
          'C1': '=1+1',
          'D1': 'hidden',
        },
        'hiddenRows': [2],
        'hiddenColumns': [4],
        'cellStyles': {
          'C1': {'numFmt': r'$#,##0.00'},
        },
      };
      final sizes = await measureSheetColumns(
        sheet: sheet,
        calculated: {'C1': 123456.7},
        first: 0,
        count: 5,
        columns: SheetAxisLayout(VisibleColumns(sheet), 122, {}),
        metrics: small,
        scaler: TextScaler.noScaling,
        direction: TextDirection.ltr,
        checkpoint: () async {},
      );
      expect(sizes['1'], greaterThan(sizes['2']!));
      expect(sizes['3'], greaterThan(sizes['2']!));
      expect(sizes.containsKey('4'), isFalse);
      expect(sizes['5'], small.fitPixels(122));
      expect(small.pixels(sizes['2']!), lessThan(122));
    },
  );
}
