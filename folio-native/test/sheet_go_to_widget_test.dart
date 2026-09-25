import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_go_to.dart';
import 'package:folio_native/src/sheet_view.dart';

void main() {
  testWidgets('name box and Go To reach cells, ranges, names and sheets', (
    t,
  ) async {
    final elsewhere = <SheetTarget>[];
    final sheet = {
      'id': 'data',
      'name': 'Data',
      'cells': <String, dynamic>{'A1': 'Item'},
    };
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SheetView(
            sheet: sheet,
            calculated: const {},
            onSelection: (_) {},
            onChanges: (_) async {},
            onDefinedNames: () async => [
              {'name': 'Rate', 'sheetId': null, 'refersTo': r'Data!$B$4'},
              {'name': 'Plan', 'sheetId': null, 'refersTo': r'Other!$A$1:$C$3'},
            ],
            onGoToSheet: (target) async {
              elsewhere.add(target);
              return target.sheetName == 'Other';
            },
          ),
        ),
      ),
    );
    await t.pumpAndSettle();
    final state = t.state<SheetViewState>(find.byType(SheetView));

    await state.goTo('C5:D6');
    await t.pumpAndSettle();
    expect(state.selectedRange, 'C5:D6');
    await state.goTo(r'data!$B$2');
    await t.pumpAndSettle();
    expect(state.selectedRange, 'B2', reason: 'this sheet, named');
    await state.goTo('rate');
    await t.pumpAndSettle();
    expect(state.selectedRange, 'B4', reason: 'a defined name');
    await state.goTo('E:F');
    await t.pumpAndSettle();
    expect(state.selectedRange, 'E1:F1048576');
    expect(state.wholeColumns, isTrue);

    await state.goTo('Plan');
    await t.pumpAndSettle();
    expect(elsewhere.single.sheetName, 'Other');
    expect(state.selectedRange, 'E1:F1048576', reason: 'left to the host');

    await state.goTo('Nowhere!A1');
    await t.pumpAndSettle();
    expect(find.textContaining('no sheet named Nowhere'), findsOneWidget);
    ScaffoldMessenger.of(t.element(find.byType(SheetView))).clearSnackBars();
    await t.pumpAndSettle();
    await state.goTo('not a place');
    await t.pumpAndSettle();
    expect(find.textContaining('is not a cell, range or name'), findsOneWidget);

    // F5 opens Go To, with the names and recent targets listed.
    await t.tap(find.byKey(const Key('cell-A1')));
    await t.pumpAndSettle();
    await t.sendKeyEvent(LogicalKeyboardKey.f5);
    await t.pumpAndSettle();
    expect(find.text('Go To'), findsOneWidget);
    expect(find.byKey(const Key('go-to-Rate')), findsOneWidget);
    expect(find.byKey(const Key('go-to-E:F')), findsOneWidget);
    await t.enterText(find.byKey(const Key('go-to-reference')), 'A1:B2x');
    await t.tap(find.byKey(const Key('go-to-ok')));
    await t.pumpAndSettle();
    expect(find.textContaining('Enter a cell'), findsOneWidget);
    await t.enterText(find.byKey(const Key('go-to-reference')), 'G7');
    await t.tap(find.byKey(const Key('go-to-ok')));
    await t.pumpAndSettle();
    expect(find.text('Go To'), findsNothing);
    expect(state.selectedRange, 'G7');

    // Ctrl+G too.
    await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await t.sendKeyEvent(LogicalKeyboardKey.keyG);
    await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await t.pumpAndSettle();
    expect(find.text('Go To'), findsOneWidget);
    await t.tap(find.text('Cancel'));
    await t.pumpAndSettle();
    expect(state.selectedRange, 'G7');
    await t.pumpWidget(const SizedBox());
  });
}
