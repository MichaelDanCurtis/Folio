import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_validation_dialog.dart';

void main() {
  testWidgets(
    'validation entry, retry, cancel, override, list, authoring and history use the real core',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-validation-ui-tests',
      );
      late WorkspaceModel model;
      late String id;
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        await CoreBridge.initialize();
        final core = CoreBridge();
        var response = await core.call({'op': 'new', 'blank': true});
        id = response['workspace']['sheets'][0]['id'] as String;
        for (final entry in [
          {
            'range': 'A1',
            'rule': {
              'type': 'whole',
              'formula1': '1',
              'formula2': '5',
              'showErrorMessage': true,
              'errorStyle': 'stop',
              'showInputMessage': true,
              'promptTitle': 'Quantity',
              'prompt': 'Enter a whole number from 1 to 5.',
            },
          },
          {
            'range': 'B1',
            'rule': {
              'type': 'list',
              'formula1': '"Red,Green,Blue"',
              'showErrorMessage': true,
              'errorStyle': 'warning',
            },
          },
        ]) {
          response = await core.call({
            'op': 'dataValidation',
            'sheetId': id,
            'baseRevision': response['workspace']['revision'],
            'action': 'set',
            ...entry,
          });
        }
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        model.snapshot = await core.call({'op': 'get'});
      });
      Future<void> until(bool Function() check) async {
        for (var i = 0; i < 500 && !check(); i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(check(), isTrue);
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
      }

      Future<void> drive(Future<dynamic> Function() job) async {
        var done = false;
        Object? error;
        unawaited(
          Future.sync(job).then(
            (_) {
              done = true;
            },
            onError: (Object e) {
              error = e;
              done = true;
            },
          ),
        );
        await until(() => done);
        if (error != null) throw error!;
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox());
        model.dispose();
      });
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.switchMode(true));
      final grid = screen.sheetKey.currentState!;
      String raw(String a) => model.sheets[0]['cells'][a]?.toString() ?? '';
      final entry = find.byKey(const Key('cell-editor'));
      Future<void> type(String value) async {
        await tester.sendKeyEvent(LogicalKeyboardKey.f2);
        await tester.pumpAndSettle();
        await tester.enterText(entry, value);
        await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      }

      await drive(() => grid.select(0, 0));
      expect(find.byKey(const Key('validation-input-message')), findsOneWidget);
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('validation-input-message')), findsNothing);
      final rev = model.revision;
      await type('9');
      await until(
        () => find.byKey(const Key('validation-alert')).evaluate().isNotEmpty,
      );
      expect(raw('A1'), '');
      expect(model.revision, rev);
      expect(find.byKey(const Key('validation-override')), findsNothing);
      await tester.tap(find.byKey(const Key('validation-retry')));
      await until(() => entry.evaluate().isNotEmpty);
      expect(tester.widget<TextField>(entry).controller!.text, '9');
      await tester.enterText(entry, '3');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await until(() => raw('A1') == '3');
      await drive(() => grid.select(0, 0));
      await type('20');
      await until(
        () => find.byKey(const Key('validation-alert')).evaluate().isNotEmpty,
      );
      await tester.tap(find.byKey(const Key('validation-discard')));
      await until(() => !grid.editing && grid.selected == 'A2');
      expect(raw('A1'), '3');
      await drive(() => grid.select(0, 1));
      await tester.tap(find.byKey(const Key('validation-dropdown')));
      await until(
        () => find.byType(ValidationChoicesDialog).evaluate().isNotEmpty,
      );
      await tester.enterText(
        find.byKey(const Key('validation-search')),
        'gree',
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Green'));
      await until(() => raw('B1') == "'Green");
      final saved = jsonEncode(model.sheets);
      await drive(() => screen.history(false));
      expect(raw('B1'), '');
      await drive(() => screen.history(true));
      expect(jsonEncode(model.sheets), saved);
      await drive(() => grid.select(0, 1));
      await type('Purple');
      await until(
        () => find.byKey(const Key('validation-alert')).evaluate().isNotEmpty,
      );
      await tester.tap(find.byKey(const Key('validation-override')));
      await until(() => raw('B1') == 'Purple');
      await drive(() => screen.history(false));
      expect(raw('B1'), "'Green");
      // Author a new list through the actual desktop settings dialog.
      await drive(() => grid.select(0, 3));
      await tester.tap(find.byKey(const Key('data-validation-button')));
      await until(
        () => find.byType(SheetValidationDialog).evaluate().isNotEmpty,
      );
      await tester.tap(find.byKey(const ValueKey('validation-type-none')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('List').last);
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('validation-formula1')),
        'Small,Medium,Large',
      );
      await tester.tap(find.text('Input message'));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('validation-promptTitle')),
        'Size',
      );
      await tester.enterText(
        find.byKey(const Key('validation-prompt')),
        'Choose a size.',
      );
      await tester.tap(find.byKey(const Key('validation-apply')));
      await until(() => model.sheets[0]['dataValidations'].length == 3);
      expect(find.byKey(const Key('validation-input-message')), findsOneWidget);
      expect(grid.validationAt('D1')?['showDropDown'], false);
      await tester.tap(find.byKey(const Key('validation-dropdown')));
      await until(
        () => find.byType(ValidationChoicesDialog).evaluate().isNotEmpty,
      );
      await tester.tap(find.text('Medium'));
      await until(() => raw('D1') == "'Medium");
      await drive(() => screen.flush());
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/authored.xlsx',
        }),
      );
      final before = jsonEncode(model.sheets);
      await drive(
        () => model.core.call({
          'op': 'save',
          'path': '${folder.path}/validation.folio',
        }),
      );
      await drive(() async {
        model.snapshot = await model.core.call({
          'op': 'load',
          'path': '${folder.path}/validation.folio',
        });
      });
      expect(jsonEncode(model.sheets), before);
      await tester.tap(find.byKey(const Key('data-validation-button')));
      await until(
        () => find.byType(SheetValidationDialog).evaluate().isNotEmpty,
      );
      await tester.tap(find.byKey(const Key('validation-clear')));
      await until(() => grid.validationAt('D1') == null);
      await drive(() => screen.history(false));
      expect(grid.validationAt('D1')?['type'], 'list');
      // The formula bar shares the same check/commit path and keeps its draft.
      await drive(() => grid.select(0, 0));
      final formula = find.byKey(const Key('formula'));
      await tester.tap(formula);
      await tester.enterText(formula, '99');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await until(
        () => find.byKey(const Key('validation-alert')).evaluate().isNotEmpty,
      );
      expect(raw('A1'), '3');
      await tester.tap(find.byKey(const Key('validation-retry')));
      await until(() => grid.formulaDirty);
      expect(grid.formulaController.text, '99');
      await tester.enterText(formula, '4');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await until(() => raw('A1') == '4');
      await drive(() => screen.history(false));
      expect(raw('A1'), '3');
      // Hidden arrows still enforce the rule but do not offer list selection.
      await drive(() => grid.select(0, 3));
      await drive(
        () => model.mutate({
          'op': 'dataValidation',
          'sheetId': id,
          'range': 'D1',
          'baseRevision': model.revision,
          'action': 'set',
          'rule': {...grid.validationAt('D1')!, 'showDropDown': true},
        }),
      );
      expect(find.byKey(const Key('validation-dropdown')), findsNothing);
      await drive(() => screen.history(false));
      expect(find.byKey(const Key('validation-dropdown')), findsOneWidget);
      await drive(() => screen.flush());
    },
  );
}
