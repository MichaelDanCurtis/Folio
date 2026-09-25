import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_view.dart';

void main() {
  Future<(SheetViewState, List<Map>, List<Map>)> mount(WidgetTester t) async {
    final cleared = <Map>[], edits = <Map>[];
    final sheet = {
      'id': 'data',
      'name': 'Data',
      'cells': <String, dynamic>{'A1': 'Item', 'B1': '4', 'B2': '9'},
    };
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SheetView(
            sheet: sheet,
            calculated: const {},
            onSelection: (_) {},
            onChanges: (changes) async => edits.addAll(changes),
            onClear: (action) async => cleared.add(action),
          ),
        ),
      ),
    );
    await t.pumpAndSettle();
    return (t.state<SheetViewState>(find.byType(SheetView)), cleared, edits);
  }

  testWidgets('the Clear menu clears the selection', (t) async {
    final (state, cleared, _) = await mount(t);
    await state.goTo('A1:B2');
    await t.pumpAndSettle();
    for (final (key, what) in [
      ('clear-all', 'all'),
      ('clear-formats', 'formats'),
      ('clear-contents', 'contents'),
      ('clear-notes', 'notes'),
      ('clear-hyperlinks', 'hyperlinks'),
    ]) {
      await t.tap(find.byKey(const Key('clear-cells-menu')));
      await t.pumpAndSettle();
      await t.tap(find.byKey(Key(key)));
      await t.pumpAndSettle();
      expect(cleared.last, {'sheetId': 'data', 'range': 'A1:B2', 'what': what});
    }
    // Whole columns are passed as a range, not cell by cell.
    await state.goTo('B:B');
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('clear-cells-menu')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('clear-all')));
    await t.pumpAndSettle();
    expect(cleared.last['range'], 'B1:B1048576');
    await t.pumpWidget(const SizedBox());
  });

  testWidgets(
    'Delete and Backspace clear the contents of every selected cell',
    (t) async {
      final (state, _, edits) = await mount(t);
      await state.goTo('A1:B2');
      await t.pumpAndSettle();
      await t.sendKeyEvent(LogicalKeyboardKey.delete);
      await t.pumpAndSettle();
      expect(edits.map((e) => e['address']).toSet(), {'A1', 'B1', 'A2', 'B2'});
      expect(edits.every((e) => e['value'] == ''), isTrue);
      edits.clear();
      await state.goTo('B2');
      await t.pumpAndSettle();
      await t.sendKeyEvent(LogicalKeyboardKey.backspace);
      await t.pumpAndSettle();
      expect(edits, [
        {'sheetId': 'data', 'address': 'B2', 'value': ''},
      ]);
      await t.pumpWidget(const SizedBox());
    },
  );
}
