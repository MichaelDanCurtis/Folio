import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'sheet_multiline_test.dart' show mount;

void main() {
  testWidgets('rapid Tab and Shift-Tab save then navigate once', (
    tester,
  ) async {
    final changes = <Map<String, dynamic>>[];
    final state = await mount(tester, changes);
    for (final reverse in [false, true]) {
      await tester.sendKeyEvent(LogicalKeyboardKey.keyA, character: 'a');
      await tester.sendKeyEvent(LogicalKeyboardKey.keyB, character: 'b');
      if (reverse) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      if (reverse) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await tester.pumpAndSettle();
      expect(state.selected, reverse ? 'B4' : 'C4');
      expect(state.gridFocus.hasPrimaryFocus, isTrue);
    }
    expect(changes.map((v) => (v['address'], v['value'])).toList(), [
      ('B4', 'ab'),
      ('C4', 'ab'),
    ]);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('cancel before attachment discards the draft and old focus work', (
    tester,
  ) async {
    final changes = <Map<String, dynamic>>[];
    final state = await mount(tester, changes);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyA, character: 'a');
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    expect(state.editing, isFalse);
    expect(state.gridFocus.hasPrimaryFocus, isTrue);
    expect(changes, isEmpty);
    // A second edit in the same frame must own the deferred focus request.
    await tester.sendKeyEvent(LogicalKeyboardKey.keyA, character: 'a');
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyB, character: 'b');
    await tester.pumpAndSettle();
    expect(state.cellFocus.hasPrimaryFocus, isTrue);
    expect(state.cellController.text, 'b');
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    // Disposal before the deferred callback must not access a dead focus node.
    await tester.sendKeyEvent(LogicalKeyboardKey.keyA, character: 'a');
    await tester.pumpWidget(const SizedBox());
    expect(tester.takeException(), isNull);
    expect(changes, isEmpty);
  });

  testWidgets(
    'F2 replacement, repeats, grapheme deletion and line breaks work before a frame',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final state = await mount(tester, changes);
      await tester.sendKeyEvent(LogicalKeyboardKey.f2);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.keyA, character: 'a');
      await tester.sendKeyRepeatEvent(LogicalKeyboardKey.keyA, character: 'a');
      await tester.sendKeyUpEvent(LogicalKeyboardKey.keyA);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyU, character: '👩🏽‍💻');
      await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
      expect(state.cellController.text, 'aa');
      await tester.sendKeyDownEvent(LogicalKeyboardKey.altLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.altLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyE, character: 'é');
      await tester.sendKeyEvent(LogicalKeyboardKey.numpadEnter);
      await tester.pumpAndSettle();
      expect(changes.single['value'], 'aa\né');
      expect(state.selected, 'B6');
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('a failed rapid save retains its whole draft for retry', (
    tester,
  ) async {
    final changes = <Map<String, dynamic>>[];
    var reject = true;
    final state = await mount(
      tester,
      changes,
      accept: () async {
        if (reject) throw StateError('Test save failure');
      },
    );
    await tester.sendKeyEvent(LogicalKeyboardKey.digit6, character: '6');
    await tester.sendKeyEvent(LogicalKeyboardKey.digit0, character: '0');
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    expect(changes, isEmpty);
    expect(state.selected, 'B4');
    expect(state.editing, isTrue);
    expect(state.cellController.text, '60');
    expect(state.cellFocus.hasPrimaryFocus, isTrue);
    reject = false;
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    expect(changes.single['value'], '60');
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('rapid commit key repeats do not reopen an editor while saving', (
    tester,
  ) async {
    final changes = <Map<String, dynamic>>[];
    final gate = Completer<void>();
    final state = await mount(tester, changes, accept: () => gate.future);
    await tester.sendKeyEvent(LogicalKeyboardKey.digit6, character: '6');
    await tester.sendKeyDownEvent(LogicalKeyboardKey.enter);
    await tester.sendKeyRepeatEvent(LogicalKeyboardKey.enter);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.enter);
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    gate.complete();
    await tester.pumpAndSettle();
    expect(changes, hasLength(1));
    expect(changes.single['value'], '6');
    expect(state.selected, 'B6');
    expect(state.editing, isFalse);
    expect(state.gridFocus.hasPrimaryFocus, isTrue);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'rapid grid entry and Enter before a frame preserve every character',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final state = await mount(tester, changes);
      await tester.sendKeyEvent(LogicalKeyboardKey.digit6, character: '6');
      await tester.sendKeyEvent(LogicalKeyboardKey.digit0, character: '0');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pumpAndSettle();
      expect(changes, [
        {'sheetId': 'test', 'address': 'B4', 'value': '60'},
      ]);
      expect(state.selected, 'B6');
      expect(state.editing, isFalse);
      expect(state.gridFocus.hasPrimaryFocus, isTrue);
      expect(state.cellFocus.hasFocus, isFalse);
      await tester.pumpWidget(const SizedBox());
    },
    variant: TargetPlatformVariant({
      TargetPlatform.macOS,
      TargetPlatform.windows,
      TargetPlatform.linux,
    }),
  );

  testWidgets(
    'first typed character leaves the caret ready for platform input',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final state = await mount(tester, changes);
      await tester.sendKeyEvent(LogicalKeyboardKey.digit6, character: '6');
      await tester.pumpAndSettle();
      expect(state.cellFocus.hasPrimaryFocus, isTrue);
      expect(state.cellController.text, '6');
      expect(
        state.cellController.selection,
        const TextSelection.collapsed(offset: 1),
      );
      // Once focused, the platform owns text input. The hardware event must not
      // also insert the character or a real key press would be applied twice.
      await tester.sendKeyEvent(LogicalKeyboardKey.digit0, character: '0');
      expect(state.cellController.text, '6');
      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: '60',
          selection: TextSelection.collapsed(offset: 2),
        ),
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pumpAndSettle();
      expect(changes.single['value'], '60');
      await tester.pumpWidget(const SizedBox());
    },
  );
}
