import 'dart:async';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/src/document_table_view.dart';
import 'package:folio_native/src/document_tables.dart';
import 'document_table_sort_test.dart' show sortTable;
import 'sheet_multiline_test.dart' show mount;

Future<void> activate(
  WidgetTester tester,
  Finder target, {
  String? custom,
}) async {
  await tester.pumpAndSettle();
  final node = tester.getSemantics(target);
  final owner = node.owner!;
  if (custom == null) {
    expect(node.getSemanticsData().hasAction(ui.SemanticsAction.tap), isTrue);
    owner.performAction(node.id, ui.SemanticsAction.tap);
  } else {
    final id = CustomSemanticsAction.getIdentifier(
      CustomSemanticsAction(label: custom),
    );
    expect(node.getSemanticsData().customSemanticsActionIds, contains(id));
    owner.performAction(node.id, ui.SemanticsAction.customAction, id);
  }
  await tester.pumpAndSettle();
}

void main() {
  testWidgets(
    'table accessible activation ignores stale Shift but preserves explicit ranges',
    (t) async {
      t.view.physicalSize = const Size(1440, 1100);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      final semantics = t.ensureSemantics();
      final original = sortTable([
        ['Name', 'Amount'],
        ['Alpha', '1'],
        ['Beta', '2'],
        ['Gamma', '3'],
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
      Finder cell(int row) =>
          find.byKey(ValueKey('table-cell-semantics-$row-0'));
      TableArea selected() => t
          .widget<DocumentTableView>(find.byType(DocumentTableView))
          .selection!;
      await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await activate(t, cell(2));
      expect((selected().top, selected().bottom), (2, 2));
      // A real Shift-click still extends selection.
      await t.ensureVisible(cell(3));
      await t.tap(find.byKey(const ValueKey('table-cell-3-0')));
      await t.pumpAndSettle();
      expect((selected().top, selected().bottom), (2, 3));
      await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await activate(t, cell(1));
      await activate(t, cell(3), custom: 'Extend selection to this cell');
      expect((selected().top, selected().bottom), (1, 3));
      // Explicit Select cells mode also works for accessible activation.
      await activate(t, cell(0));
      await t.tap(find.byKey(const Key('table-select-cells')));
      await t.pumpAndSettle();
      await activate(t, cell(2));
      expect((selected().top, selected().bottom), (0, 2));
      expect(
        t.widget<DocumentTableView>(find.byType(DocumentTableView)).node,
        original,
      );
      semantics.dispose();
      await t.pumpWidget(const SizedBox());
      expect(t.takeException(), isNull);
    },
  );

  testWidgets(
    'sheet accessible activation and range actions do not inherit keyboard modifiers',
    (t) async {
      t.view.physicalSize = const Size(1440, 1100);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      final semantics = t.ensureSemantics();
      final edits = <Map<String, dynamic>>[];
      final state = await mount(t, edits);
      Finder cell(String name) => find.byKey(ValueKey('cell-semantics-$name'));
      expect(t.getSemantics(cell('B4')).label, contains('B4 cell'));
      expect(t.getSemantics(cell('B4')).label, contains('Original'));
      await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await activate(t, cell('C3'));
      expect(state.selectedRange, 'C3');
      await t.tap(find.byKey(const Key('cell-B4')));
      await t.pumpAndSettle();
      expect(state.selectedRange, 'B3:C4');
      await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await activate(t, cell('B4'));
      await activate(t, cell('C3'), custom: 'Extend selection to this cell');
      expect(state.selectedRange, 'B3:C4');
      await activate(t, cell('C4'), custom: 'Edit cell');
      expect(state.selectedRange, 'C4');
      expect(state.editing, isTrue);
      expect(state.cellController.text, 'Neighbor');
      await t.sendKeyEvent(LogicalKeyboardKey.escape);
      await t.pumpAndSettle();
      expect(edits, isEmpty);
      semantics.dispose();
      await t.pumpWidget(const SizedBox());
      expect(t.takeException(), isNull);
    },
  );

  testWidgets('accessible editing preserves a rejected draft before retry', (
    t,
  ) async {
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
    await t.enterText(find.byKey(const Key('cell-editor')), 'Keep this draft');
    final destination = find.byKey(const ValueKey('cell-semantics-C4'));
    await activate(t, destination, custom: 'Edit cell');
    expect(state.selectedRange, 'B4');
    expect(state.editing, isTrue);
    expect(state.cellController.text, 'Keep this draft');
    expect(edits, isEmpty);

    reject = false;
    await activate(t, destination, custom: 'Edit cell');
    expect(edits.single, {
      'sheetId': 'test',
      'address': 'B4',
      'value': 'Keep this draft',
    });
    expect(state.selectedRange, 'C4');
    expect(state.editing, isTrue);
    expect(state.cellController.text, 'Neighbor');
    await t.sendKeyEvent(LogicalKeyboardKey.escape);
    await t.pumpAndSettle();
    expect(edits, hasLength(1));
    semantics.dispose();
    await t.pumpWidget(const SizedBox());
    expect(t.takeException(), isNull);
  });

  testWidgets(
    'pointer selection retains Shift state while a pending cell commit finishes',
    (t) async {
      final edits = <Map<String, dynamic>>[];
      final gate = Completer<void>();
      final state = await mount(t, edits, accept: () => gate.future);
      await t.sendKeyEvent(LogicalKeyboardKey.f2);
      await t.pumpAndSettle();
      await t.enterText(find.byKey(const Key('cell-editor')), 'Changed');
      await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await t.tap(find.byKey(const Key('cell-C3')));
      await t.pump();
      await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      gate.complete();
      await t.pumpAndSettle();
      expect(edits.single['value'], 'Changed');
      expect(state.selectedRange, 'B3:C4');
      await t.pumpWidget(const SizedBox());
    },
  );
}
