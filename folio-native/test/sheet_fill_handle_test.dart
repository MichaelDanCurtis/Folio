import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_fill_handle.dart';
import 'package:folio_native/src/sheet_view.dart';
import 'package:folio_native/src/table_controls.dart';

void main() {
  const s = TableArea(1, 1, 2, 2); // B2:C3
  String plan(int r, int c) => planFillHandle(s, r, c).toString();

  test('releasing the handle: the larger overshoot picks the direction', () {
    expect(plan(6, 2), 'down 1,1:6,2');
    expect(plan(6, 4), 'down 1,1:6,2', reason: '4 rows beat 2 columns');
    expect(plan(2, 7), 'right 1,1:2,7');
    expect(plan(0, 1), 'up 0,1:2,2');
    expect(plan(1, 0), 'left 1,0:2,2');
    expect(plan(1, 2), 'clear 2,1:2,2', reason: 'dragged back inside');
    expect(plan(2, 1), 'clear 1,2:2,2');
    expect(planFillHandle(s, 2, 2), isNull);
  });

  test('double-click follows the adjacent column', () {
    final cells = {'A2': 'x', 'A3': 'x', 'A4': 'x', 'A5': 'x', 'D9': 'y'};
    expect(fillDownEnd(cells, s), 4, reason: 'A column runs to row 5');
    expect(fillDownEnd({'D4': 'y', 'D5': 'y'}, s), 4, reason: 'right side');
    expect(fillDownEnd({'A9': 'x'}, s), isNull);
  });

  test('positions map to cells, skipping hidden ones', () {
    final layout = SheetAxisLayout(VisibleAxis([2], 100), 20, {1: 40});
    expect(indexAt(layout, 0, 100), 0);
    expect(indexAt(layout, 19, 100), 0);
    expect(indexAt(layout, 20, 100), 1);
    expect(indexAt(layout, 59, 100), 1);
    expect(indexAt(layout, 60, 100), 3, reason: 'row 2 is hidden');
  });

  Future<(SheetViewState, List<Map>, List<Map>)> mount(WidgetTester t) async {
    final fills = <Map>[], clears = <Map>[];
    final sheet = <String, dynamic>{
      'id': 'data',
      'name': 'Data',
      'cells': <String, dynamic>{
        'A1': '1',
        'A2': '2',
        'B1': 'x',
        'B2': 'x',
        'B3': 'x',
        'B4': 'x',
      },
    };
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SheetView(
            sheet: sheet,
            calculated: const {},
            onSelection: (_) {},
            onChanges: (_) async {},
            onFill: (action) async => fills.add(action),
            onClear: (action) async => clears.add(action),
          ),
        ),
      ),
    );
    await t.pumpAndSettle();
    return (t.state<SheetViewState>(find.byType(SheetView)), fills, clears);
  }

  testWidgets('dragging the handle fills and selects the result', (t) async {
    final (state, fills, clears) = await mount(t);
    await state.goTo('A1:A2');
    await t.pumpAndSettle();
    final height = t.getRect(find.byKey(const Key('cell-A1'))).height;
    await t.drag(find.byKey(const Key('fill-handle')), Offset(0, height * 3));
    await t.pumpAndSettle();
    expect(fills.single, {
      'sheetId': 'data',
      'range': 'A1:A5',
      'source': 'A1:A2',
      'direction': 'down',
    });
    expect(state.selectedRange, 'A1:A5');

    // Back inside the selection: the released cells are cleared.
    await t.drag(find.byKey(const Key('fill-handle')), Offset(0, -height * 2));
    await t.pumpAndSettle();
    expect(clears.single, {
      'sheetId': 'data',
      'range': 'A4:A5',
      'what': 'contents',
    });
    await t.pumpWidget(const SizedBox());
  });

  testWidgets('double-clicking the handle fills down beside the data', (
    t,
  ) async {
    final (state, fills, _) = await mount(t);
    await state.goTo('A1:A2');
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('fill-handle')));
    await t.pump(const Duration(milliseconds: 50));
    await t.tap(find.byKey(const Key('fill-handle')));
    await t.pumpAndSettle();
    expect(fills.single['range'], 'A1:A4', reason: 'column B runs to row 4');
    expect(state.selectedRange, 'A1:A4');
    await t.pumpWidget(const SizedBox());
  });
}
