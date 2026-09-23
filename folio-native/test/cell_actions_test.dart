import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/src/document_table_view.dart';
import 'package:folio_native/src/sheet_view.dart';
import 'cell_selection_accessibility_test.dart' show activate;
import 'document_table_sort_test.dart' show sortTable;
import 'sheet_multiline_test.dart' show mount;

Future<void> choose(WidgetTester t, String label) async {
  await activate(t, find.byKey(const Key('cell-actions')));
  await activate(t, find.text(label));
}

void main() {
  testWidgets('desktop menu and F8 extend, finish, clear and edit selections', (
    t,
  ) async {
    t.view.physicalSize = const Size(1440, 1100);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.resetPhysicalSize);
    addTearDown(t.view.resetDevicePixelRatio);
    final semantics = t.ensureSemantics();
    final edits = <Map<String, dynamic>>[];
    final state = await mount(t, edits);
    await choose(t, 'Extend selection');
    expect(state.extendSelectionMode, isTrue);
    expect(find.byKey(const Key('extend-selection-status')), findsOneWidget);
    await activate(t, find.byKey(const ValueKey('cell-semantics-C3')));
    expect(state.selectedRange, 'B3:C4');
    await t.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await t.pumpAndSettle();
    expect(state.selectedRange, 'B3:D4');
    await activate(t, find.byKey(const Key('finish-selection')));
    expect(state.extendSelectionMode, isFalse);
    expect(state.selectedRange, 'B3:D4');
    await activate(t, find.byKey(const ValueKey('cell-semantics-A4')));
    expect(state.selectedRange, 'A4');
    await t.sendKeyEvent(LogicalKeyboardKey.f8);
    await t.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await t.pumpAndSettle();
    expect(state.selectedRange, 'A4:A6'); // Row 5 is hidden.
    await t.sendKeyEvent(LogicalKeyboardKey.f8);
    expect(state.extendSelectionMode, isFalse);
    await activate(t, find.byKey(const ValueKey('cell-semantics-C4')));
    await choose(t, 'Edit cell');
    expect(state.selectedRange, 'C4');
    expect(state.editing, isTrue);
    expect(state.cellController.text, 'Neighbor');
    await t.sendKeyEvent(LogicalKeyboardKey.f8);
    expect(state.extendSelectionMode, isFalse);
    await t.sendKeyEvent(LogicalKeyboardKey.escape);
    await t.pumpAndSettle();
    await t.sendKeyEvent(LogicalKeyboardKey.f8);
    await t.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await t.pumpAndSettle();
    expect(state.selectedRange, 'C4:D4');
    await choose(t, 'Select active cell');
    expect(state.selectedRange, 'D4');
    expect(state.extendSelectionMode, isFalse);
    await t.sendKeyEvent(LogicalKeyboardKey.f8);
    await t.sendKeyEvent(LogicalKeyboardKey.escape);
    expect(state.extendSelectionMode, isFalse);
    expect(edits, isEmpty);
    await t.sendKeyEvent(LogicalKeyboardKey.f8);
    await t.tap(find.byKey(const Key('cell-C3')));
    await t.pumpAndSettle();
    expect(state.selectedRange, 'C3:D4');
    await t.sendKeyEvent(LogicalKeyboardKey.escape);
    semantics.dispose();
    await t.pumpWidget(const SizedBox());
    expect(t.takeException(), isNull);
  });

  testWidgets(
    'menu editing retains a rejected draft and retries once accepted',
    (t) async {
      final semantics = t.ensureSemantics();
      final edits = <Map<String, dynamic>>[];
      var reject = true;
      final state = await mount(
        t,
        edits,
        accept: () async {
          if (reject) throw StateError('Rejected test save');
        },
      );
      await t.sendKeyEvent(LogicalKeyboardKey.f2);
      await t.pumpAndSettle();
      await t.enterText(
        find.byKey(const Key('cell-editor')),
        'Keep this draft',
      );
      await choose(t, 'Edit cell');
      expect(state.editing, isTrue);
      expect(state.cellController.text, 'Keep this draft');
      expect(edits, isEmpty);
      reject = false;
      await choose(t, 'Edit cell');
      expect(state.editing, isTrue);
      expect(state.cellController.text, 'Keep this draft');
      expect(edits.single, {
        'sheetId': 'test',
        'address': 'B4',
        'value': 'Keep this draft',
      });
      await t.sendKeyEvent(LogicalKeyboardKey.escape);
      await t.pumpAndSettle();
      semantics.dispose();
      await t.pumpWidget(const SizedBox());
      expect(t.takeException(), isNull);
    },
  );

  testWidgets('an open cell menu cannot edit a different worksheet', (t) async {
    final semantics = t.ensureSemantics();
    final sheet = ValueNotifier<Map>({
      'id': 'first',
      'name': 'First',
      'cells': {'B4': 'First cell'},
    });
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ValueListenableBuilder<Map>(
            valueListenable: sheet,
            builder: (_, value, _) => SheetView(
              sheet: value,
              calculated: const {},
              onSelection: (_) {},
              onChanges: (_) async {},
            ),
          ),
        ),
      ),
    );
    await t.pumpAndSettle();
    final state = t.state<SheetViewState>(find.byType(SheetView));
    await t.tap(find.byKey(const Key('cell-B4')));
    await t.pumpAndSettle();
    await choose(t, 'Extend selection');
    await activate(t, find.byKey(const Key('cell-actions')));
    sheet.value = {
      'id': 'second',
      'name': 'Second',
      'cells': {'B4': 'Second cell'},
    };
    await t.pumpAndSettle();
    await activate(t, find.text('Edit cell'));
    expect(state.extendSelectionMode, isFalse);
    expect(state.editing, isFalse);
    expect(state.raw('B4'), 'Second cell');
    semantics.dispose();
    await t.pumpWidget(const SizedBox());
    sheet.dispose();
    expect(t.takeException(), isNull);
  });

  testWidgets('a late selection cannot commit a new worksheet formula draft', (
    t,
  ) async {
    final gate = Completer<void>();
    final edits = <Map<String, dynamic>>[];
    final sheet = ValueNotifier<Map>({
      'id': 'first',
      'name': 'First',
      'cells': {'B4': 'Original'},
    });
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ValueListenableBuilder<Map>(
            valueListenable: sheet,
            builder: (_, value, _) => SheetView(
              sheet: value,
              calculated: const {},
              onSelection: (_) {},
              onChanges: (changes) async {
                await gate.future;
                edits.addAll(changes);
                for (final change in changes) {
                  (value['cells'] as Map)[change['address']] = change['value'];
                }
              },
            ),
          ),
        ),
      ),
    );
    await t.pumpAndSettle();
    final state = t.state<SheetViewState>(find.byType(SheetView));
    await t.tap(find.byKey(const Key('cell-B4')));
    await t.sendKeyEvent(LogicalKeyboardKey.f2);
    await t.pumpAndSettle();
    await t.enterText(find.byKey(const Key('cell-editor')), 'First edit');
    final navigation = state.select(3, 2);
    await t.pump();
    sheet.value = {
      'id': 'second',
      'name': 'Second',
      'cells': {'B4': 'Second cell'},
    };
    await t.pump();
    await t.tap(find.byKey(const Key('formula')));
    await t.enterText(find.byKey(const Key('formula')), 'Unsaved second draft');
    gate.complete();
    expect(await navigation, isFalse);
    await t.pumpAndSettle();
    expect(state.selected, 'B4');
    expect(state.formulaController.text, 'Unsaved second draft');
    expect(state.formulaDirty, isTrue);
    expect(edits.single, {
      'sheetId': 'first',
      'address': 'B4',
      'value': 'First edit',
    });
    expect(sheet.value['cells']['B4'], 'Second cell');
    await t.pumpWidget(const SizedBox());
    sheet.dispose();
    expect(t.takeException(), isNull);
  });

  testWidgets('table Edit cell focuses its editor and ends range mode', (
    t,
  ) async {
    t.view.physicalSize = const Size(1440, 1100);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.resetPhysicalSize);
    addTearDown(t.view.resetDevicePixelRatio);
    final semantics = t.ensureSemantics();
    final original = sortTable([
      ['Name'],
      ['Alpha'],
      ['Beta'],
    ]);
    await t.pumpWidget(
      MaterialApp(
        localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
        home: Scaffold(
          body: DocumentTableDialog(node: original, embedBuilders: const []),
        ),
      ),
    );
    await t.pumpAndSettle();
    await activate(t, find.byKey(const ValueKey('table-cell-semantics-1-0')));
    await t.tap(find.byKey(const Key('table-select-cells')));
    await t.pumpAndSettle();
    await activate(t, find.byKey(const ValueKey('table-cell-semantics-2-0')));
    var selection = t
        .widget<DocumentTableView>(find.byType(DocumentTableView))
        .selection!;
    expect((selection.top, selection.bottom), (1, 2));
    await activate(t, find.byKey(const Key('table-edit-cell')));
    final editor = t.widget<q.QuillEditor>(find.byType(q.QuillEditor));
    expect(editor.focusNode.hasFocus, isTrue);
    expect(editor.controller.document.toPlainText(), 'Beta\n');
    selection = t
        .widget<DocumentTableView>(find.byType(DocumentTableView))
        .selection!;
    expect((selection.top, selection.bottom), (2, 2));
    expect(
      t
          .widget<FilterChip>(find.byKey(const Key('table-select-cells')))
          .selected,
      isFalse,
    );
    expect(
      t.widget<DocumentTableView>(find.byType(DocumentTableView)).node,
      original,
    );
    semantics.dispose();
    await t.pumpWidget(const SizedBox());
    expect(t.takeException(), isNull);
  });
}
