import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_view.dart';
import 'package:folio_native/src/sheet_text_overflow.dart';
import 'package:folio_native/src/table_controls.dart';

Future<int> overflowInk(WidgetTester tester, String address) async {
  return (await tester.runAsync(() async {
    final boundary = tester.renderObject<RenderRepaintBoundary>(
      find.byKey(const Key('capture')),
    );
    final rect = tester
        .getRect(find.byKey(Key('cell-$address')))
        .shift(-tester.getTopLeft(find.byKey(const Key('capture'))))
        .deflate(5);
    final image = await boundary.toImage();
    final bytes = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
    var count = 0;
    for (
      var y = rect.top.ceil().clamp(0, image.height);
      y < rect.bottom.floor().clamp(0, image.height);
      y++
    ) {
      for (
        var x = rect.left.ceil().clamp(0, image.width);
        x < rect.right.floor().clamp(0, image.width);
        x++
      ) {
        final at = (y * image.width + x) * 4;
        if (bytes.getUint8(at) < 120 &&
            bytes.getUint8(at + 1) < 120 &&
            bytes.getUint8(at + 2) < 120) {
          count++;
        }
      }
    }
    image.dispose();
    return count;
  }))!;
}

void main() {
  test(
    'sparse overflow stops at values, empty-result formulas, merges and frozen panes',
    () {
      final index = SheetOverflowIndex(
        {
          'A1': 'long',
          'D1': '=IF(TRUE,"","")',
          'XFD1048576': 'last',
          'B2': 'hidden',
          'A2': 'visible',
        },
        VisibleColumns({
          'hiddenColumns': [2],
        }),
        [TableArea.parse('E1:F1')],
      );
      expect(index.candidates(0, 2, 0), [0, 3]);
      expect(index.candidates(0, 3, 0), [3]);
      expect(index.candidates(0, 6, 0), isEmpty);
      expect(index.candidates(0, 2, 1), [3]);
      expect(index.candidates(1, 2, 0), [0]);
      expect(index.candidates(1048575, 16382, 0), [16383]);
    },
  );

  testWidgets(
    'overflow paints blank neighbors, keeps hit testing and clears when blocked or wrapped',
    (tester) async {
      final sheet = <String, dynamic>{
        'id': 's',
        'name': 'Labels',
        'cells': <String, dynamic>{
          'A1': 'A long label extending across several empty columns',
        },
        'cellStyles': <String, dynamic>{},
      };
      late StateSetter refresh;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: RepaintBoundary(
              key: const Key('capture'),
              child: StatefulBuilder(
                builder: (context, setState) {
                  refresh = setState;
                  return SheetView(
                    sheet: sheet,
                    calculated: Map<String, dynamic>.from(sheet['cells']),
                    onSelection: (_) {},
                    onChanges: (changes) async {
                      refresh(() {
                        for (final change in changes) {
                          (sheet['cells'] as Map)[change['address']] =
                              change['value'];
                        }
                      });
                    },
                  );
                },
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(await overflowInk(tester, 'B1'), greaterThan(30));
      final state = tester.state<SheetViewState>(find.byType(SheetView));
      await tester.tap(find.byKey(const Key('cell-B1')));
      await tester.pumpAndSettle();
      expect(state.selected, 'B1');
      expect(state.raw('B1'), '');
      refresh(() {
        (sheet['cells'] as Map)['B1'] = 'x';
      });
      await tester.pumpAndSettle();
      expect(await overflowInk(tester, 'C1'), 0);
      refresh(() {
        (sheet['cells'] as Map).remove('B1');
        (sheet['cellStyles'] as Map)['A1'] = {'wrapText': true};
      });
      await tester.pumpAndSettle();
      expect(await overflowInk(tester, 'B1'), 0);
      refresh(() {
        (sheet['cellStyles'] as Map).clear();
      });
      await tester.pumpAndSettle();
      expect(await overflowInk(tester, 'B1'), greaterThan(30));
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('offscreen owners continue painting after horizontal scroll', (
    tester,
  ) async {
    final text = 'Continuous label ' * 40;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RepaintBoundary(
            key: const Key('capture'),
            child: SheetView(
              sheet: {
                'id': 's',
                'name': 'Scrolling',
                'cells': {'A1': text},
              },
              calculated: {'A1': text},
              onSelection: (_) {},
              onChanges: (_) async {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final state = tester.state<SheetViewState>(find.byType(SheetView));
    state.horizontal.jumpTo(900);
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('cell-A1')), findsNothing);
    expect(await overflowInk(tester, 'J1'), greaterThan(30));
    await tester.pumpWidget(const SizedBox());
  });
  testWidgets(
    'alignment, typed numbers, errors and merged barriers control overflow',
    (tester) async {
      final long = 'abcdefghij' * 5;
      final cells = {
        'C1': long,
        'C2': long,
        'C3': long,
        'C4': '1234567890123456789',
        'C5': '1234567890123456789',
        'C6': '#DIV/0!',
        'A7': long,
      };
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: RepaintBoundary(
              key: const Key('capture'),
              child: SheetView(
                sheet: {
                  'id': 's',
                  'name': 'Alignment',
                  'cells': cells,
                  'mergedRanges': ['B7:C7'],
                  'cellStyles': {
                    'C1': {'horizontalAlignment': 'center'},
                    'C2': {'horizontalAlignment': 'right'},
                    'C3': {'horizontalAlignment': 'left'},
                  },
                },
                calculated: {...cells, 'C4': 1234567890123456789},
                formattedCells: {
                  'C6': {'text': '#DIV/0!', 'isError': true},
                },
                onSelection: (_) {},
                onChanges: (_) async {},
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(await overflowInk(tester, 'B1'), greaterThan(30));
      expect(await overflowInk(tester, 'D1'), greaterThan(30));
      expect(await overflowInk(tester, 'B2'), greaterThan(30));
      expect(await overflowInk(tester, 'D2'), 0);
      expect(await overflowInk(tester, 'B3'), 0);
      expect(await overflowInk(tester, 'D3'), greaterThan(30));
      expect(await overflowInk(tester, 'D4'), 0);
      expect(await overflowInk(tester, 'D5'), greaterThan(30));
      expect(await overflowInk(tester, 'D6'), 0);
      expect(await overflowInk(tester, 'B7'), 0);
      expect(await overflowInk(tester, 'D7'), 0);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'frozen columns and empty-result formulas block painting without consuming cells',
    (tester) async {
      final long = 'abcdefghij' * 5;
      final cells = {'A1': long, 'A2': long, 'B2': '=IF(TRUE,"","")'};
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: RepaintBoundary(
              key: const Key('capture'),
              child: SheetView(
                sheet: {
                  'id': 's',
                  'name': 'Frozen',
                  'cells': cells,
                  'frozenPane': {'rows': 0, 'columns': 1, 'state': 'frozen'},
                },
                calculated: {...cells, 'B2': ''},
                onSelection: (_) {},
                onChanges: (_) async {},
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(await overflowInk(tester, 'B1'), 0);
      expect(await overflowInk(tester, 'C2'), 0);
      final state = tester.state<SheetViewState>(find.byType(SheetView));
      expect(state.raw('B1'), '');
      expect(state.raw('B2'), '=IF(TRUE,"","")');
      await tester.pumpWidget(const SizedBox());
    },
  );
}
