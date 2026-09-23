import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_view.dart';

import 'sheet_multiline_test.dart' show mount;

Future<void> digits(WidgetTester tester, String text) async {
  const keys = [
    LogicalKeyboardKey.digit0,
    LogicalKeyboardKey.digit1,
    LogicalKeyboardKey.digit2,
    LogicalKeyboardKey.digit3,
    LogicalKeyboardKey.digit4,
    LogicalKeyboardKey.digit5,
    LogicalKeyboardKey.digit6,
    LogicalKeyboardKey.digit7,
    LogicalKeyboardKey.digit8,
    LogicalKeyboardKey.digit9,
  ];
  for (final char in text.split('')) {
    await tester.sendKeyEvent(keys[int.parse(char)], character: char);
  }
}

void main() {
  testWidgets(
    'flush keeps text arriving while the final draft is being saved',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final first = Completer<void>(), finalDraft = Completer<void>();
      var calls = 0;
      final state = await mount(
        tester,
        changes,
        accept: () async {
          calls++;
          if (calls == 1) await first.future;
          if (calls == 2) await finalDraft.future;
        },
      );
      await digits(tester, '60');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await digits(tester, '4');
      final flushing = state.flush();
      first.complete();
      await tester.pumpAndSettle();
      expect(calls, 2);
      await digits(tester, '3');
      if (state.cellFocus.hasPrimaryFocus) {
        tester.testTextInput.updateEditingValue(
          const TextEditingValue(
            text: '43',
            selection: TextSelection.collapsed(offset: 2),
          ),
        );
      }
      finalDraft.complete();
      await tester.pumpAndSettle();
      await flushing;
      expect(state.raw('C4'), '43');
      expect(state.editing, isFalse);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'queued keys never replay into a replacement worksheet and remain recoverable',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final gate = Completer<void>();
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String;
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      var id = 'first';
      late StateSetter update;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                update = setState;
                return SheetView(
                  sheet: {'id': id, 'name': id, 'cells': <String, dynamic>{}},
                  workspaceId: 'workspace',
                  calculated: const {},
                  onSelection: (_) {},
                  onChanges: (items) async {
                    await gate.future;
                    changes.addAll(items);
                  },
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final state = tester.state<SheetViewState>(find.byType(SheetView));
      await tester.tap(find.byKey(const Key('cell-B4')));
      await tester.pumpAndSettle();
      await digits(tester, '60');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await digits(tester, '70');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      update(() => id = 'second');
      await tester.pump();
      gate.complete();
      await tester.pumpAndSettle();
      expect(changes, [
        {'sheetId': 'first', 'address': 'B4', 'value': '60'},
      ]);
      await state.finishEditing(1, 0);
      expect(changes, hasLength(1));
      await expectLater(state.flush(), throwsStateError);
      await tester.tap(find.byKey(const Key('copy-waiting-keys')));
      await tester.pumpAndSettle();
      expect(copied, startsWith('70'));
      await tester.tap(find.byKey(const Key('discard-queued-typing')));
      await tester.pumpAndSettle();
      expect(changes, hasLength(1));
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'new typing can join a second pending save and caret keys preserve graphemes',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final first = Completer<void>(), second = Completer<void>();
      var calls = 0;
      final state = await mount(
        tester,
        changes,
        accept: () async {
          calls++;
          if (calls == 1) await first.future;
          if (calls == 2) await second.future;
        },
      );
      await digits(tester, '60');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await digits(tester, '70');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      first.complete();
      await tester.pumpAndSettle();
      expect(calls, 2);
      expect(changes, hasLength(1));
      await digits(tester, '7');
      await tester.sendKeyEvent(LogicalKeyboardKey.keyU, character: '🦉');
      await digits(tester, '0');
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await digits(tester, '8');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      second.complete();
      await tester.pumpAndSettle();
      expect(changes.map((v) => (v['address'], v['value'])).toList(), [
        ('B4', '60'),
        ('C4', '70'),
        ('D4', '780'),
      ]);
      expect(state.selected, 'D6');
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'pointer navigation waits until queued cells have finished saving',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final first = Completer<void>();
      var calls = 0;
      final state = await mount(
        tester,
        changes,
        accept: () async {
          if (++calls == 1) await first.future;
        },
      );
      await digits(tester, '60');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await digits(tester, '70');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      final selected = state.select(0, 0);
      first.complete();
      await tester.pumpAndSettle();
      expect(await selected, isTrue);
      expect(changes.map((v) => (v['address'], v['value'])).toList(), [
        ('B4', '60'),
        ('C4', '70'),
      ]);
      expect(state.selected, 'A1');
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'failed save retains later entries and retry keeps the original direction',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final first = Completer<void>();
      var calls = 0;
      final state = await mount(
        tester,
        changes,
        accept: () async {
          if (++calls == 1) await first.future;
        },
      );
      await digits(tester, '60');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await digits(tester, '70');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      first.completeError(StateError('Test save failure'));
      await tester.pumpAndSettle();
      expect(changes, isEmpty);
      expect(state.cellController.text, '60');
      expect(state.selected, 'B4');
      expect(find.byKey(const Key('pending-sheet-entries')), findsOneWidget);
      expect(await state.select(0, 0), isFalse);
      await expectLater(state.flush(), throwsStateError);
      await tester.enterText(find.byKey(const Key('cell-editor')), '61');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pumpAndSettle();
      expect(changes.map((v) => (v['address'], v['value'])).toList(), [
        ('B4', '61'),
        ('C4', '70'),
      ]);
      expect(state.selected, 'C6');
      expect(find.byKey(const Key('pending-sheet-entries')), findsNothing);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('Escape skips a failed cell and preserves the queued next cell', (
    tester,
  ) async {
    final changes = <Map<String, dynamic>>[];
    final first = Completer<void>();
    var calls = 0;
    final state = await mount(
      tester,
      changes,
      accept: () async {
        if (++calls == 1) await first.future;
      },
    );
    await digits(tester, '60');
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await digits(tester, '70');
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    first.completeError(StateError('Test save failure'));
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    expect(changes.map((v) => (v['address'], v['value'])).toList(), [
      ('C4', '70'),
    ]);
    expect(state.raw('B4'), 'Original');
    expect(state.selected, 'C6');
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'discarding queued typing keeps the failed current draft unsaved',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final first = Completer<void>();
      var calls = 0;
      final state = await mount(
        tester,
        changes,
        accept: () async {
          if (++calls == 1) await first.future;
        },
      );
      await digits(tester, '60');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await digits(tester, '70');
      first.completeError(StateError('Test save failure'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('discard-queued-typing')));
      await tester.pumpAndSettle();
      expect(changes, isEmpty);
      expect(calls, 1);
      expect(state.cellController.text, '60');
      expect(state.editing, isTrue);
      expect(find.byKey(const Key('pending-sheet-entries')), findsNothing);
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(changes, isEmpty);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'queued Shift-Tab and Alt-Enter retain modifiers after key release',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final first = Completer<void>();
      var calls = 0;
      final state = await mount(
        tester,
        changes,
        accept: () async {
          if (++calls == 1) await first.future;
        },
      );
      await digits(tester, '1');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await digits(tester, '2');
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await digits(tester, '3');
      await tester.sendKeyDownEvent(LogicalKeyboardKey.altLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.altLeft);
      await digits(tester, '4');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      first.complete();
      await tester.pumpAndSettle();
      expect(changes.map((v) => (v['address'], v['value'])).toList(), [
        ('B4', '1'),
        ('C4', '2'),
        ('B4', '3\n4'),
      ]);
      expect(state.selected, 'B6');
      await tester.pumpWidget(const SizedBox());
    },
  );

  for (final formula in [false, true]) {
    for (final renderWhileSaving in [false, true]) {
      testWidgets(
        'attached ${formula ? 'formula' : 'cell'} editor retains an unfinished next draft (render while saving: $renderWhileSaving)',
        (tester) async {
          final changes = <Map<String, dynamic>>[];
          final first = Completer<void>();
          var calls = 0;
          final state = await mount(
            tester,
            changes,
            accept: () async {
              if (++calls == 1) await first.future;
            },
          );
          if (!formula) {
            await tester.sendKeyEvent(LogicalKeyboardKey.f2);
            await tester.pumpAndSettle();
          }
          await tester.enterText(
            find.byKey(Key(formula ? 'formula' : 'cell-editor')),
            '60',
          );
          await tester.sendKeyEvent(LogicalKeyboardKey.tab);
          if (renderWhileSaving) await tester.pump();
          await digits(tester, '70');
          first.complete();
          await tester.pumpAndSettle();
          expect(changes.map((v) => (v['address'], v['value'])).toList(), [
            ('B4', '60'),
          ]);
          expect(state.selected, 'C4');
          expect(state.editing, isTrue);
          expect(state.cellController.text, '70');
          expect(state.cellFocus.hasPrimaryFocus, isTrue);
          await tester.sendKeyEvent(LogicalKeyboardKey.enter);
          await tester.pumpAndSettle();
          expect(changes.last['value'], '70');
          expect(changes.last['address'], 'C4');
          await tester.pumpWidget(const SizedBox());
        },
      );
    }
  }

  testWidgets(
    'flush waits for all queued entries including an unfinished last draft',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final first = Completer<void>();
      var calls = 0, flushed = false;
      final state = await mount(
        tester,
        changes,
        accept: () async {
          if (++calls == 1) await first.future;
        },
      );
      await digits(tester, '60');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await digits(tester, '70');
      final pending = state.flush().then((_) => flushed = true);
      await tester.pump();
      expect(flushed, isFalse);
      first.complete();
      await tester.pumpAndSettle();
      await pending;
      expect(changes.map((v) => (v['address'], v['value'])).toList(), [
        ('B4', '60'),
        ('C4', '70'),
      ]);
      expect(state.editing, isFalse);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'continuous entries wait for the previous save and keep their cells',
    (tester) async {
      final changes = <Map<String, dynamic>>[];
      final first = Completer<void>();
      var calls = 0;
      final state = await mount(
        tester,
        changes,
        accept: () async {
          if (++calls == 1) await first.future;
        },
      );
      await digits(tester, '60');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await digits(tester, '70');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await digits(tester, '80');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      expect(changes, isEmpty);
      first.complete();
      await tester.pumpAndSettle();
      expect(changes.map((v) => (v['address'], v['value'])).toList(), [
        ('B4', '60'),
        ('C4', '70'),
        ('D4', '80'),
      ]);
      expect(state.selected, 'D6');
      expect(state.editing, isFalse);
      expect(state.gridFocus.hasPrimaryFocus, isTrue);
      await tester.pumpWidget(const SizedBox());
    },
  );
}
