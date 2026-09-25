import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_view.dart';

Future<SheetViewState> mount(
  WidgetTester t,
  List<Map<String, dynamic>> edits,
) async {
  final sheet = {
    'id': 'nav',
    'name': 'Data',
    'cells': <String, dynamic>{
      'A1': 'Item',
      'B1': 'Qty',
      'A2': 'Pens',
      'B2': '4',
      'A3': 'Ink',
      'B3': '9',
      'A6': 'Notes',
      'A10': 'Total',
    },
  };
  await t.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: SheetView(
          sheet: sheet,
          calculated: const {},
          onSelection: (_) {},
          onChanges: (changes) async {
            edits.addAll(changes);
            for (final change in changes) {
              (sheet['cells'] as Map)[change['address']] = change['value'];
            }
          },
        ),
      ),
    ),
  );
  await t.pumpAndSettle();
  final state = t.state<SheetViewState>(find.byType(SheetView));
  await t.tap(find.byKey(const Key('cell-A1')));
  await t.pumpAndSettle();
  return state;
}

Future<void> press(
  WidgetTester t,
  LogicalKeyboardKey key, {
  bool control = false,
  bool shift = false,
}) async {
  if (control) await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  if (shift) await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
  await t.sendKeyEvent(key);
  if (shift) await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  if (control) await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
  await t.pumpAndSettle();
}

void main() {
  testWidgets('Ctrl+arrow, Ctrl+Home/End and Home move like Excel', (t) async {
    final state = await mount(t, []);
    await press(t, LogicalKeyboardKey.arrowDown, control: true);
    expect(state.selectedRange, 'A3', reason: 'end of the first run');
    await press(t, LogicalKeyboardKey.arrowDown, control: true);
    expect(state.selectedRange, 'A6', reason: 'next filled cell');
    await press(t, LogicalKeyboardKey.arrowDown, control: true, shift: true);
    expect(state.selectedRange, 'A6:A10', reason: 'Shift extends');
    await press(t, LogicalKeyboardKey.arrowDown, control: true);
    expect(state.selectedRange, 'A1048576', reason: 'no data below');
    await press(t, LogicalKeyboardKey.home, control: true);
    expect(state.selectedRange, 'A1');
    await press(t, LogicalKeyboardKey.arrowRight, control: true);
    expect(state.selectedRange, 'B1');
    await press(t, LogicalKeyboardKey.end, control: true);
    expect(state.selectedRange, 'B10', reason: 'last used row and column');
    await press(t, LogicalKeyboardKey.home);
    expect(state.selectedRange, 'A10');
    await t.pumpWidget(const SizedBox());
    expect(t.takeException(), isNull);
  });

  testWidgets('whole rows, columns, the current region and the sheet', (
    t,
  ) async {
    final state = await mount(t, []);
    await press(t, LogicalKeyboardKey.space, shift: true);
    expect(state.selectedRange, 'A1:XFD1');
    expect(state.selected, 'A1', reason: 'the active cell stays');
    await press(t, LogicalKeyboardKey.space, control: true);
    expect(state.selectedRange, 'A1:XFD1048576');
    await press(t, LogicalKeyboardKey.arrowRight);
    expect(state.selectedRange, 'B1', reason: 'moving clears it');
    await press(t, LogicalKeyboardKey.keyA, control: true);
    expect(state.selectedRange, 'A1:B3', reason: 'current region');
    expect(state.selected, 'B1');
    await press(t, LogicalKeyboardKey.keyA, control: true);
    expect(state.selectedRange, 'A1:XFD1048576', reason: 'then the sheet');
    await t.tap(find.byKey(const Key('row-header-2')));
    await t.pumpAndSettle();
    expect(state.selectedRange, 'A3:XFD3');
    expect(state.selected, 'A3');
    await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await t.tap(find.byKey(const Key('row-header-5')));
    await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await t.pumpAndSettle();
    expect(state.selectedRange, 'A3:XFD6', reason: 'Shift-click extends');
    await t.tap(find.byKey(const Key('column-header-1')));
    await t.pumpAndSettle();
    expect(state.selectedRange, 'B1:B1048576');
    await t.pumpWidget(const SizedBox());
    expect(t.takeException(), isNull);
  });

  testWidgets('Delete on a whole column clears only its populated cells', (
    t,
  ) async {
    final edits = <Map<String, dynamic>>[];
    final state = await mount(t, edits);
    await t.tap(find.byKey(const Key('column-header-0')));
    await t.pumpAndSettle();
    expect(state.selectedRange, 'A1:A1048576');
    await press(t, LogicalKeyboardKey.delete);
    expect(edits.map((e) => e['address']).toSet(), {
      'A1',
      'A2',
      'A3',
      'A6',
      'A10',
    });
    expect(edits.every((e) => e['value'] == ''), isTrue);
    await t.pumpWidget(const SizedBox());
    expect(t.takeException(), isNull);
  });
}
