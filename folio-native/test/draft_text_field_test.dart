import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/draft_text_field.dart';

Future<void> historyKey(WidgetTester tester, {bool redo = false}) async {
  final modifier = defaultTargetPlatform == TargetPlatform.macOS
      ? LogicalKeyboardKey.metaLeft
      : LogicalKeyboardKey.controlLeft;
  await tester.sendKeyDownEvent(modifier);
  if (redo) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
  if (redo) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyUpEvent(modifier);
  await tester.pump();
}

void main() {
  testWidgets(
    'moving between fields keeps drafts separate and external values start fresh',
    (tester) async {
      final first = TextEditingController(text: 'one');
      final second = TextEditingController(text: 'two');
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Column(
              children: [
                draftTextField(key: const Key('one'), controller: first),
                draftTextField(key: const Key('two'), controller: second),
              ],
            ),
          ),
        ),
      );
      await tester.tap(find.byKey(const Key('one')));
      await tester.enterText(find.byKey(const Key('one')), 'first draft');
      await tester.tap(find.byKey(const Key('two')));
      // Flutter can update selection after a field loses focus.
      first.selection = const TextSelection.collapsed(offset: 0);
      await tester.enterText(find.byKey(const Key('two')), 'second draft');
      await historyKey(tester);
      expect(second.text, 'two');
      expect(first.text, 'first draft');
      await tester.tap(find.byKey(const Key('one')));
      await historyKey(tester);
      expect(first.text, 'one');
      await historyKey(tester, redo: true);
      expect(first.text, 'first draft');
      await tester.tap(find.byKey(const Key('two')));
      first.text = 'new supplied value';
      await tester.tap(find.byKey(const Key('one')));
      await historyKey(tester);
      expect(first.text, 'new supplied value');
      await tester.enterText(find.byKey(const Key('one')), 'updated');
      await historyKey(tester);
      expect(first.text, 'new supplied value');
      await tester.pumpWidget(const SizedBox.shrink());
      first.dispose();
      second.dispose();
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );

  testWidgets(
    'the first edit can be undone and redone immediately or after typing pauses',
    (tester) async {
      for (final initial in ['', 'pré 🦉']) {
        for (final wait in [Duration.zero, const Duration(milliseconds: 600)]) {
          final controller = TextEditingController(text: initial);
          final changes = <String>[];
          await tester.pumpWidget(
            MaterialApp(
              home: Scaffold(
                body: draftTextField(
                  key: const Key('field'),
                  controller: controller,
                  onChanged: changes.add,
                ),
              ),
            ),
          );
          await tester.tap(find.byKey(const Key('field')));
          await tester.pump();
          final originalSelection = controller.selection;
          await tester.enterText(find.byKey(const Key('field')), 'first draft');
          await tester.pump(wait);
          await historyKey(tester);
          expect(controller.text, initial);
          expect(controller.selection, originalSelection);
          await historyKey(tester, redo: true);
          expect(controller.text, 'first draft');
          expect(
            controller.selection,
            const TextSelection.collapsed(offset: 11),
          );
          expect(changes, ['first draft', initial, 'first draft']);
          await tester.pumpWidget(const SizedBox.shrink());
          controller.dispose();
        }
      }
    },
    variant: TargetPlatformVariant({
      TargetPlatform.macOS,
      TargetPlatform.windows,
      TargetPlatform.linux,
    }),
  );

  testWidgets(
    'typing groups, moved selections and new branches retain the right text',
    (tester) async {
      final controller = TextEditingController(text: 'abc');
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: draftTextField(controller: controller)),
        ),
      );
      await tester.tap(find.byType(TextField));
      await tester.enterText(find.byType(TextField), 'abcd');
      await tester.pump(const Duration(milliseconds: 100));
      await tester.enterText(find.byType(TextField), 'abcde');
      await historyKey(tester);
      expect(controller.text, 'abc');
      await historyKey(tester, redo: true);
      expect(controller.text, 'abcde');
      // Moving the caret separates the next edit from the preceding typing group.
      controller.selection = const TextSelection.collapsed(offset: 0);
      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: 'Xabcde',
          selection: TextSelection.collapsed(offset: 1),
        ),
      );
      await tester.pump();
      await historyKey(tester);
      expect(controller.text, 'abcde');
      expect(controller.selection, const TextSelection.collapsed(offset: 0));
      await historyKey(tester);
      expect(controller.text, 'abc');
      await tester.enterText(find.byType(TextField), 'another branch');
      await historyKey(tester, redo: true);
      expect(controller.text, 'another branch');
      await historyKey(tester);
      expect(controller.text, 'abc');
      await tester.pumpWidget(const SizedBox.shrink());
      controller.dispose();
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );

  testWidgets(
    'live IME composition stays intact and commits as one undoable edit',
    (tester) async {
      final controller = TextEditingController(text: 'Start ');
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: draftTextField(controller: controller)),
        ),
      );
      await tester.tap(find.byType(TextField));
      const composing = TextEditingValue(
        text: 'Start に',
        selection: TextSelection.collapsed(offset: 7),
        composing: TextRange(start: 6, end: 7),
      );
      tester.testTextInput.updateEditingValue(composing);
      await tester.pump();
      await historyKey(tester);
      expect(controller.value, composing);
      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: 'Start 日本',
          selection: TextSelection.collapsed(offset: 8),
        ),
      );
      await tester.pump();
      await historyKey(tester);
      expect(controller.text, 'Start ');
      await historyKey(tester, redo: true);
      expect(controller.text, 'Start 日本');
      expect(controller.value.composing, TextRange.empty);
      await historyKey(tester);
      tester.testTextInput.updateEditingValue(composing);
      await tester.pump();
      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: 'Start ',
          selection: TextSelection.collapsed(offset: 6),
        ),
      );
      await tester.pump();
      await historyKey(tester, redo: true);
      expect(
        controller.text,
        'Start 日本',
        reason: 'Cancelling a composition must not discard an existing Redo',
      );
      await tester.pumpWidget(const SizedBox.shrink());
      controller.dispose();
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );

  testWidgets(
    'form callbacks, formatters, controller replacement and read-only state stay consistent',
    (tester) async {
      var controller = TextEditingController(text: '12');
      var readOnly = false;
      final changes = <String>[];
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                rebuild = setState;
                return draftTextField(
                  key: const Key('form'),
                  formField: true,
                  controller: controller,
                  readOnly: readOnly,
                  onChanged: changes.add,
                  inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                );
              },
            ),
          ),
        ),
      );
      final field = find.byKey(const Key('form'));
      await tester.tap(field);
      await tester.enterText(field, '34a');
      expect(controller.text, '34');
      await historyKey(tester);
      expect(controller.text, '12');
      await historyKey(tester, redo: true);
      expect(controller.text, '34');
      expect(changes, ['34', '12', '34']);
      rebuild(() => readOnly = true);
      await tester.pump();
      await historyKey(tester);
      expect(controller.text, '34');
      final old = controller;
      rebuild(() {
        readOnly = false;
        controller = TextEditingController(text: '56');
      });
      await tester.pump();
      await tester.tap(field);
      await tester.enterText(field, '78');
      await historyKey(tester);
      expect(controller.text, '56');
      await tester.pumpWidget(const SizedBox.shrink());
      old.dispose();
      controller.dispose();
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
