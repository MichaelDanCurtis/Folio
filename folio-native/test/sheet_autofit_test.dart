import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_autofit.dart';
import 'package:folio_native/src/sheet_font.dart';
import 'package:folio_native/src/table_controls.dart';

void main() {
  testWidgets(
    'large row measurement yields and abandons partial results when cancelled',
    (tester) async {
      var checkpoints = 0;
      final sheet = <String, dynamic>{'cells': <String, dynamic>{}};
      Map<String, double>? result;
      await expectLater(() async {
        result = await measureSheetRows(
          sheet: sheet,
          calculated: {},
          first: 0,
          count: 100000,
          columns: SheetAxisLayout(VisibleColumns(sheet), 140, {}),
          scaler: TextScaler.noScaling,
          direction: TextDirection.ltr,
          checkpoint: () async {
            if (++checkpoints == 2) throw StateError('Cancelled');
          },
        );
      }, throwsStateError);
      expect(checkpoints, 2);
      expect(result, isNull);
      expect(sheet, {'cells': {}});
    },
  );
  testWidgets(
    'AutoFit shares shaping for wrapping, rotation, scripts and shrink',
    (tester) async {
      const text =
          'A longer heading with enough words to wrap onto several lines';
      Future<double> fit(
        Map<String, dynamic> font,
        double width, {
        String value = text,
      }) async {
        final sheet = {
          'cells': {'A1': value},
          'cellStyles': {'A1': font},
          'defaultRowHeight': 15,
        };
        final sizes = await measureSheetRows(
          sheet: sheet,
          calculated: {},
          first: 0,
          count: 1,
          columns: SheetAxisLayout(VisibleColumns(sheet), width, {}),
          scaler: TextScaler.noScaling,
          direction: TextDirection.ltr,
          checkpoint: () async {},
        );
        await tester.pumpWidget(
          MaterialApp(
            home: Center(
              child: SizedBox(
                width: width - 21.2,
                height: sizes['1']! * 4 / 3 - 7.2,
                child: SheetFontText(
                  value,
                  font: font,
                  layout: CellTextLayout(font),
                  key: const Key('text'),
                ),
              ),
            ),
          ),
        );
        final box = tester.renderObject<SheetFontRenderBox>(
          find.byKey(const Key('text')),
        );
        expect(box.textBounds.top, greaterThanOrEqualTo(-.01));
        expect(box.textBounds.bottom, lessThanOrEqualTo(box.size.height + .01));
        return sizes['1']!;
      }

      final narrow = await fit({'wrapText': true}, 140);
      final wide = await fit({'wrapText': true}, 300);
      expect(narrow, greaterThan(wide));
      expect(
        await fit(
          {'textRotation': 90, 'wrapText': true},
          140,
          value: 'Quarter turn',
        ),
        greaterThan(70),
      );
      await fit(
        {'textRotation': -30, 'wrapText': true},
        260,
        value: 'Short angle',
      );
      await fit({'textRotation': 255}, 140, value: 'A🌊é');
      await fit(
        {'verticalAlign': 'superscript', 'fontSize': 24},
        140,
        value: 'ABC',
      );
      expect(await fit({'shrinkToFit': true}, 140), lessThan(narrow));
      await fit(
        {
          '_richText': [
            {
              'text': 'Big',
              'style': {'fontSize': 32},
            },
            {
              'text': ' small',
              'style': {'fontSize': 11},
            },
          ],
        },
        220,
        value: 'Big small',
      );
    },
  );

  testWidgets(
    'AutoFit measures offscreen cells and blank styled rows, skips hidden and filtered rows, caps height',
    (tester) async {
      final sheet = {
        'cells': {
          'Z1': 'one\ntwo\nthree\nfour',
          'B4': List.filled(1000, 'hidden').join('\n'),
          'A5': List.filled(1000, 'line').join('\n'),
        },
        'hiddenRows': [2],
        'hiddenColumns': [2],
        'autoFilter': {
          'filteredRows': [3],
        },
        'cellStyles': {
          'A6': {'fontSize': 50},
        },
        'styleDefaults': {
          'rows': {
            '7': {'fontSize': 60},
          },
        },
        'defaultRowHeight': 15,
      };
      var checkpoints = 0;
      final sizes = await measureSheetRows(
        sheet: sheet,
        calculated: {},
        first: 0,
        count: 200,
        columns: SheetAxisLayout(VisibleColumns(sheet), 140, {}),
        scaler: TextScaler.noScaling,
        direction: TextDirection.ltr,
        checkpoint: () async {
          checkpoints++;
        },
      );
      expect(sizes.keys, isNot(contains('2')));
      expect(sizes.keys, isNot(contains('3')));
      expect(sizes['1'], greaterThan(sizes['4']!));
      expect(sizes['5'], 409);
      expect(sizes['6'], greaterThan(sizes['4']!));
      expect(sizes['7'], greaterThan(sizes['6']!));
      expect(checkpoints, greaterThan(1));
      expect(sizes.length, 198);
    },
  );
}
