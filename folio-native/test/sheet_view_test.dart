import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_view.dart';

void main() {
  testWidgets(
    'filter button stays under the pointer when table is not selected',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SheetView(
              sheet: const {
                'id': 's',
                'name': 'Sheet',
                'cells': {'A1': 'Name', 'B1': 'Value', 'A2': 'Pear', 'B2': '1'},
                'tables': [
                  {
                    'id': 't',
                    'name': 'Sales',
                    'range': 'A1:B2',
                    'headerRows': 1,
                    'totalsRows': 0,
                    'columns': [
                      {'name': 'Name'},
                      {'name': 'Value'},
                    ],
                    'filters': {},
                  },
                ],
              },
              calculated: const {'A2': 'Pear', 'B2': 1},
              onSelection: (_) {},
              onChanges: (_) async {},
              onTable: (_) async {},
            ),
          ),
        ),
      );
      final button = find.byKey(const Key('table-filter-t-0'));
      final position = tester.getCenter(button);
      final pointer = await tester.startGesture(position);
      await tester.pump();
      expect(tester.getCenter(button), position);
      await pointer.up();
      await tester.pump(const Duration(milliseconds: 350));
      await tester.pumpAndSettle();
      expect(find.text('Filter Name'), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      await tester.pumpWidget(const SizedBox());
    },
  );
  testWidgets(
    'cell selection is immediate and double-click still edits',
    (tester) async {
      const channel = MethodChannel('folio/keyboard');
      final focused = <bool>[];
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
        call,
      ) async {
        expect(call.method, 'setGridFocused');
        focused.add(call.arguments as bool);
        return null;
      });
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          channel,
          null,
        ),
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SheetView(
              sheet: const {
                'id': 's',
                'name': 'Sheet',
                'cells': {'A1': 'one', 'B1': 'two'},
              },
              calculated: const {},
              onSelection: (_) {},
              onChanges: (_) async {},
            ),
          ),
        ),
      );
      final state = tester.state<SheetViewState>(find.byType(SheetView));
      await tester.tap(find.byKey(const Key('cell-A1')));
      await tester.pump();
      expect(state.selected, 'A1');
      expect(state.gridFocus.hasPrimaryFocus, isTrue);
      expect(focused.last, isTrue);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pump();
      expect(state.selected, 'B1');
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('cell-B1')));
      await tester.pump(const Duration(milliseconds: 60));
      await tester.tap(find.byKey(const Key('cell-B1')));
      await tester.pumpAndSettle();
      expect(state.editing, isTrue);
      // Text input is delivered by the platform after the key event. The grid
      // must not replace the editor's contents while that event bubbles out.
      final beforeTyping = state.cellController.text;
      await tester.sendKeyEvent(LogicalKeyboardKey.keyK, character: 'k');
      await tester.pump();
      expect(state.cellController.text, beforeTyping);
      await tester.tap(find.byKey(const Key('cell-editor')));
      await tester.pumpAndSettle();
      expect(state.editing, isTrue);
      expect(focused.last, isFalse);
      await tester.tap(find.byKey(const Key('formula')));
      await tester.pumpAndSettle();
      expect(state.formulaFocus.hasPrimaryFocus, isTrue);
      expect(focused.last, isFalse);
      await tester.tap(find.byKey(const Key('cell-A1')));
      await tester.pump();
      expect(focused.last, isTrue);
      await tester.pump(const Duration(milliseconds: 350));
      await tester.pumpWidget(const SizedBox());
      expect(focused.last, isFalse);
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );
  testWidgets(
    'grid stays virtualized and moving after an edit commits exactly once',
    (tester) async {
      tester.view.physicalSize = const Size(1000, 700);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final changes = <Map<String, dynamic>>[];
      final sheet = {
        'id': 'budget',
        'cells': <String, dynamic>{'B4': '2400'},
      };
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SheetView(
              sheet: sheet,
              calculated: const {},
              onSelection: (_) {},
              onChanges: (items) async {
                changes.addAll(items);
                for (final item in items) {
                  (sheet['cells'] as Map)[item['address']] = item['value'];
                }
              },
            ),
          ),
        ),
      );
      final count = find
          .byWidgetPredicate(
            (w) =>
                w.key is ValueKey<String> &&
                (w.key as ValueKey<String>).value.startsWith('cell-'),
          )
          .evaluate()
          .length;
      expect(count, lessThan(500));
      expect(find.byKey(const Key('cell-Z10000')), findsNothing);
      await tester.tap(find.byKey(const Key('cell-B4')));
      await tester.pump(const Duration(milliseconds: 350));
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pumpAndSettle();
      expect(
        tester.state<SheetViewState>(find.byType(SheetView)).editing,
        isTrue,
      );
      await tester.enterText(find.byKey(const Key('cell-editor')), '3000');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();
      expect(changes, hasLength(1));
      expect(changes.single['value'], '3000');
      expect(sheet['cells'], containsPair('B4', '3000'));
      await tester.enterText(find.byKey(const Key('formula')), '42');
      await tester.tap(find.byKey(const Key('cell-A1')));
      await tester.pumpAndSettle();
      expect(changes, hasLength(2));
      expect(changes.last['address'], 'B5');
      expect(changes.last['value'], '42');
      await tester.pumpWidget(const SizedBox());
    },
  );
}
