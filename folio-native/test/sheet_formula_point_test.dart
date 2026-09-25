import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_formula_point.dart';
import 'package:folio_native/src/sheet_view.dart';

void main() {
  test('where a reference can go', () {
    for (final (text, caret) in [
      ('=', 1),
      ('=A1+', 4),
      ('=SUM(', 5),
      ('=SUM(A1, ', 9),
      ('=A1:', 4),
    ]) {
      expect(canPoint(text, caret), isTrue, reason: text);
    }
    for (final (text, caret) in [
      ('A1+', 3),
      ('=A1', 3),
      ('=SUM', 4),
      ('="a+', 4),
      ('=', 0),
    ]) {
      expect(canPoint(text, caret), isFalse, reason: text);
    }
  });

  test('a pointed reference or range is written at its place', () {
    const value = TextEditingValue(
      text: '=SUM()',
      selection: TextSelection.collapsed(offset: 5),
    );
    final (one, placed) = applyPoint(
      value,
      const FormulaPoint(
        start: 5,
        end: 5,
        anchorRow: 2,
        anchorColumn: 0,
        row: 2,
        column: 0,
      ),
    );
    expect(one.text, '=SUM(A3)');
    expect(one.selection.baseOffset, 7);
    final (range, _) = applyPoint(
      one,
      FormulaPoint(
        start: placed.start,
        end: placed.end,
        anchorRow: 2,
        anchorColumn: 0,
        row: 0,
        column: 27,
      ),
    );
    expect(range.text, '=SUM(A1:AB3)');
  });

  Future<(SheetViewState, List<Map>)> mount(WidgetTester t) async {
    final edits = <Map>[];
    final sheet = <String, dynamic>{
      'id': 'data',
      'name': 'Data',
      'cells': <String, dynamic>{'A2': '4', 'B3': '5'},
    };
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SheetView(
            sheet: sheet,
            calculated: const {},
            onSelection: (_) {},
            onChanges: (changes) async => edits.addAll(changes),
          ),
        ),
      ),
    );
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('cell-A1')));
    await t.pumpAndSettle();
    return (t.state<SheetViewState>(find.byType(SheetView)), edits);
  }

  String editor(WidgetTester t) => t
      .widget<TextField>(find.byKey(const Key('cell-editor')))
      .controller!
      .text;

  Future<void> key(
    WidgetTester t,
    LogicalKeyboardKey k, {
    bool shift = false,
  }) async {
    if (shift) await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await t.sendKeyEvent(k);
    if (shift) await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await t.pumpAndSettle();
  }

  testWidgets('arrow keys point while a formula is typed', (t) async {
    final (state, edits) = await mount(t);
    await key(t, LogicalKeyboardKey.equal);
    expect(editor(t), '=');
    expect(state.shownEditMode, EditMode.enter);
    await key(t, LogicalKeyboardKey.arrowDown);
    expect(editor(t), '=A2');
    expect(state.shownEditMode, EditMode.point);
    await key(t, LogicalKeyboardKey.arrowDown);
    expect(editor(t), '=A3', reason: 'the reference moves');
    await key(t, LogicalKeyboardKey.arrowRight, shift: true);
    expect(editor(t), '=A3:B3', reason: 'Shift extends it to a range');
    await t.enterText(find.byKey(const Key('cell-editor')), '=A3:B3+');
    await t.pumpAndSettle();
    expect(editor(t), '=A3:B3+');
    expect(state.shownEditMode, EditMode.enter);
    await key(t, LogicalKeyboardKey.arrowRight);
    expect(editor(t), '=A3:B3+B1', reason: 'a new reference starts again');
    await key(t, LogicalKeyboardKey.enter);
    expect(edits.last['value'], '=A3:B3+B1');
    expect(edits.last['address'], 'A1');
    await t.pumpWidget(const SizedBox());
  });

  testWidgets('F2 switches to Edit mode, where arrows move the caret', (
    t,
  ) async {
    final (state, _) = await mount(t);
    await key(t, LogicalKeyboardKey.equal);
    await key(t, LogicalKeyboardKey.f2);
    expect(state.shownEditMode, EditMode.edit);
    await key(t, LogicalKeyboardKey.arrowDown);
    expect(editor(t), '=', reason: 'no reference in Edit mode');
    await key(t, LogicalKeyboardKey.f2);
    expect(state.shownEditMode, EditMode.enter);
    await key(t, LogicalKeyboardKey.arrowDown);
    expect(editor(t), '=A2');
    await t.pumpWidget(const SizedBox());
  });

  testWidgets('clicking a cell puts its reference into the formula', (t) async {
    final (state, edits) = await mount(t);
    await key(t, LogicalKeyboardKey.equal);
    await t.tap(find.byKey(const Key('cell-B3')));
    await t.pumpAndSettle();
    expect(editor(t), '=B3');
    expect(state.selected, 'A1', reason: 'the edited cell stays selected');
    await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await t.tap(find.byKey(const Key('cell-C4')));
    await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await t.pumpAndSettle();
    expect(editor(t), '=B3:C4');
    await key(t, LogicalKeyboardKey.enter);
    expect(edits.last['value'], '=B3:C4');

    // In a plain value, clicking another cell ends the entry as before.
    await t.tap(find.byKey(const Key('cell-D1')));
    await t.pumpAndSettle();
    await key(t, LogicalKeyboardKey.digit7);
    await t.tap(find.byKey(const Key('cell-E1')));
    await t.pumpAndSettle();
    expect(edits.last, {'sheetId': 'data', 'address': 'D1', 'value': '7'});
    expect(state.selected, 'E1');
    await t.pumpWidget(const SizedBox());
  });
}
