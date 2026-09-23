import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_view.dart';

Future<void> key(
  WidgetTester tester,
  LogicalKeyboardKey value, {
  bool alt = false,
  bool control = false,
  bool shift = false,
  bool meta = false,
}) async {
  final modifiers = [
    if (alt) LogicalKeyboardKey.altLeft,
    if (control) LogicalKeyboardKey.controlLeft,
    if (shift) LogicalKeyboardKey.shiftLeft,
    if (meta) LogicalKeyboardKey.metaLeft,
  ];
  for (final modifier in modifiers) {
    await tester.sendKeyDownEvent(modifier);
  }
  await tester.sendKeyEvent(value);
  for (final modifier in modifiers.reversed) {
    await tester.sendKeyUpEvent(modifier);
  }
  await tester.pumpAndSettle();
}

Future<SheetViewState> mount(
  WidgetTester tester,
  List<Map<String, dynamic>> edits, {
  Future<void> Function()? accept,
}) async {
  final sheet = {
    'id': 'test',
    'name': 'Drafts',
    'cells': <String, dynamic>{'B4': 'Original', 'C4': 'Neighbor'},
    'hiddenRows': [5],
  };
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: SheetView(
          sheet: sheet,
          calculated: const {},
          onSelection: (_) {},
          onChanges: (changes) async {
            await accept?.call();
            edits.addAll(changes);
            for (final change in changes) {
              (sheet['cells'] as Map)[change['address']] = change['value'];
            }
          },
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  final state = tester.state<SheetViewState>(find.byType(SheetView));
  await tester.tap(find.byKey(const Key('cell-B4')));
  await tester.pumpAndSettle();
  return state;
}

void main() {
  testWidgets(
    'pasting multiline text in either editor stays in one cell',
    (tester) async {
      const pasted = 'First 🦉\nSecond\nThird';
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.getData') return {'text': pasted};
          if (call.method == 'Clipboard.hasStrings') return {'value': true};
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      final changes = <Map<String, dynamic>>[];
      final state = await mount(tester, changes);
      for (final formula in [false, true]) {
        if (!formula) {
          await key(tester, LogicalKeyboardKey.f2);
        } else {
          await tester.tap(find.byKey(const Key('formula')));
          await tester.pumpAndSettle();
          await key(tester, LogicalKeyboardKey.keyA, meta: true);
        }
        await key(tester, LogicalKeyboardKey.keyV, meta: true);
        expect(
          (formula ? state.formulaController : state.cellController).text,
          pasted,
        );
        expect(changes, isEmpty);
        await key(tester, LogicalKeyboardKey.escape);
        expect(state.selected, 'B4');
        expect(changes, isEmpty);
      }
      await tester.pumpWidget(const SizedBox());
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );

  testWidgets(
    'cell line-break shortcuts replace selections, retain local history and commit once',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final state = await mount(tester, changes);
      await key(tester, LogicalKeyboardKey.f2);
      await tester.enterText(
        find.byKey(const Key('cell-editor')),
        'First 🦉 replace last',
      );
      state.cellController.selection = const TextSelection(
        baseOffset: 16,
        extentOffset: 9,
      );
      await tester.pump();
      await key(
        tester,
        LogicalKeyboardKey.enter,
        alt: true,
        control: defaultTargetPlatform == TargetPlatform.macOS,
      );
      expect(state.cellController.text, 'First 🦉 \n last');
      expect(state.cellController.selection.baseOffset, 10);
      expect(state.editing, isTrue);
      expect(changes, isEmpty);
      final mac = defaultTargetPlatform == TargetPlatform.macOS;
      await key(tester, LogicalKeyboardKey.keyZ, meta: mac, control: !mac);
      expect(state.cellController.text, 'First 🦉 replace last');
      await key(
        tester,
        LogicalKeyboardKey.keyZ,
        meta: mac,
        control: !mac,
        shift: true,
      );
      expect(state.cellController.text, 'First 🦉 \n last');
      // Both the regular Return and keypad Enter use the commit route.
      await key(
        tester,
        defaultTargetPlatform == TargetPlatform.windows
            ? LogicalKeyboardKey.enter
            : LogicalKeyboardKey.numpadEnter,
      );
      expect(changes, hasLength(1));
      expect(changes.single['value'], 'First 🦉 \n last');
      expect(state.selected, 'B6'); // Hidden row 5 is skipped.
      expect(state.editing, isFalse);
      await tester.pumpWidget(const SizedBox());
    },
    variant: TargetPlatformVariant({
      TargetPlatform.macOS,
      TargetPlatform.windows,
      TargetPlatform.linux,
    }),
  );

  testWidgets(
    'formula bar expands without committing, retains newlines and cancels a draft',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final state = await mount(tester, changes);
      await tester.enterText(find.byKey(const Key('formula')), 'First\nSecond');
      final selection = state.formulaController.selection;
      await tester.tap(find.byKey(const Key('expand-formula')));
      await tester.pumpAndSettle();
      expect(state.formulaExpanded, isTrue);
      expect(state.formulaFocus.hasFocus, isTrue);
      expect(state.formulaController.selection, selection);
      expect(changes, isEmpty);
      await key(tester, LogicalKeyboardKey.enter, alt: true);
      expect(state.formulaController.text, 'First\nSecond\n');
      expect(changes, isEmpty);
      await key(tester, LogicalKeyboardKey.escape);
      expect(changes, isEmpty);
      expect(state.formulaController.text, 'Original');
      expect(state.gridFocus.hasFocus, isTrue);
      await tester.enterText(find.byKey(const Key('formula')), '=SUM(1,\n2)');
      await key(tester, LogicalKeyboardKey.enter, shift: true);
      expect(changes.single['value'], '=SUM(1,\n2)');
      expect(state.selected, 'B3');
      // Reopening a multiline value and changing one character retains all breaks.
      await state.select(3, 1);
      await tester.pumpAndSettle();
      await key(tester, LogicalKeyboardKey.f2);
      expect(state.cellController.text, '=SUM(1,\n2)');
      await tester.enterText(
        find.byKey(const Key('cell-editor')),
        '=SUM(1,\n3)',
      );
      await key(tester, LogicalKeyboardKey.tab, shift: true);
      expect(changes.last['value'], '=SUM(1,\n3)');
      expect(state.selected, 'A4');
      await tester.pumpWidget(const SizedBox());
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );

  testWidgets(
    'composition owns Enter and Escape and failed saves retain the cell draft',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      var reject = true;
      final state = await mount(
        tester,
        changes,
        accept: () async {
          if (reject) throw StateError('Rejected test cell');
        },
      );
      await key(tester, LogicalKeyboardKey.f2);
      const composing = TextEditingValue(
        text: '日本',
        selection: TextSelection.collapsed(offset: 2),
        composing: TextRange(start: 0, end: 2),
      );
      tester.testTextInput.updateEditingValue(composing);
      await tester.pump();
      await key(tester, LogicalKeyboardKey.enter);
      await key(tester, LogicalKeyboardKey.escape);
      expect(state.cellController.value, composing);
      expect(changes, isEmpty);
      expect(state.editing, isTrue);
      tester.testTextInput.updateEditingValue(
        composing.copyWith(composing: TextRange.empty),
      );
      await key(tester, LogicalKeyboardKey.enter);
      expect(changes, isEmpty);
      expect(state.selected, 'B4');
      expect(state.editing, isTrue);
      expect(state.cellController.text, '日本');
      expect(state.cellFocus.hasFocus, isTrue);
      reject = false;
      await key(tester, LogicalKeyboardKey.tab);
      expect(changes.single['value'], '日本');
      expect(state.selected, 'C4');
      await tester.pumpWidget(const SizedBox());
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );

  testWidgets(
    'repeated commit keys wait for one save and failure prevents click navigation',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final gate = Completer<void>();
      final state = await mount(tester, changes, accept: () => gate.future);
      await tester.enterText(find.byKey(const Key('formula')), 'First\nSecond');
      await key(tester, LogicalKeyboardKey.enter);
      await key(tester, LogicalKeyboardKey.enter);
      expect(state.selected, 'B4');
      expect(changes, isEmpty);
      gate.completeError(StateError('Rejected pending formula'));
      await tester.pumpAndSettle();
      expect(state.selected, 'B4');
      expect(state.formulaDirty, isTrue);
      expect(state.formulaController.text, 'First\nSecond');
      await tester.tap(find.byKey(const Key('cell-C4')));
      await tester.pumpAndSettle();
      expect(state.selected, 'B4');
      expect(state.formulaController.text, 'First\nSecond');
      expect(await state.select(0, 0), isFalse);
      expect(state.selectedRange, 'B4');
      await expectLater(state.flush(), throwsStateError);
      await tester.pumpWidget(const SizedBox());
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
