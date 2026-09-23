import 'dart:async';
import 'dart:io';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_conditional_dialog.dart';

void main() {
  testWidgets(
    'date rule authoring, calculation button/F9, Undo and file recovery use the real engine',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-dates-ui-tests',
      );
      late WorkspaceModel model;
      late String id;
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        await CoreBridge.initialize();
        final core = CoreBridge();
        final response = await core.call({'op': 'new', 'blank': true});
        id = response['workspace']['sheets'][0]['id'] as String;
        await core.call({
          'op': 'cells',
          'cells': [
            for (final e in {
              'A1': '=TODAY()-1',
              'A2': '=TODAY()',
              'A3': '=TODAY()+1',
              'B1': '=NOW()',
            }.entries)
              {'sheetId': id, 'address': e.key, 'value': e.value},
          ],
        });
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

      Future<void> drive(Future<dynamic> Function() action) async {
        var done = false;
        Object? error;
        unawaited(
          Future.sync(action).then(
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
      final revision = model.revision;
      await tester.tap(find.byKey(const Key('conditional-format-button')));
      await until(
        () => find.byType(SheetConditionalDialog).evaluate().isNotEmpty,
      );
      await tester.tap(find.byKey(const Key('conditional-new')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('conditional-range')),
        'A1:A1048576',
      );
      await tester.tap(find.byType(DropdownButtonFormField<String>).first);
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        find.text('A date occurring').last,
        200,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.tap(find.text('A date occurring').last);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('conditional-period')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Tomorrow').last);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('conditional-rule-done')));
      await tester.pumpAndSettle();
      expect(model.revision, revision);
      await tester.tap(find.byKey(const Key('conditional-apply')));
      await until(() => model.revision == revision + 1);
      expect(
        model.sheets[0]['conditionalFormats'][0]['timePeriod'],
        'tomorrow',
      );
      final styles = model.snapshot['formatted'][id];
      expect(styles['A2']['conditionalStyle'], isNull);
      expect(styles['A3']['conditionalStyle'], isNotNull);
      final before = jsonEncode(model.workspace);
      final history = [model.snapshot['canUndo'], model.snapshot['canRedo']];
      final epoch = model.documentEpoch;
      var generation = model.snapshot['evaluatedCells'] as int;
      await tester.tap(find.byKey(const Key('calculate-now')));
      await until(() => (model.snapshot['evaluatedCells'] as int) > generation);
      expect(jsonEncode(model.workspace), before);
      expect(model.documentEpoch, epoch);
      expect([model.snapshot['canUndo'], model.snapshot['canRedo']], history);
      generation = model.snapshot['evaluatedCells'] as int;
      await tester.sendKeyEvent(LogicalKeyboardKey.f9);
      await until(() => (model.snapshot['evaluatedCells'] as int) > generation);
      expect(model.revision, revision + 1);
      await drive(() => model.refreshCalculations());
      expect(model.documentEpoch, epoch);
      await drive(() => screen.history(false));
      expect(model.sheets[0]['conditionalFormats'] ?? [], isEmpty);
      final undone = model.revision;
      await drive(() => screen.recalculate());
      expect(model.revision, undone);
      expect(model.snapshot['canRedo'], true);
      await drive(() => screen.history(true));
      expect(
        model.sheets[0]['conditionalFormats'][0]['timePeriod'],
        'tomorrow',
      );
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/dates.xlsx',
        }),
      );
      await drive(() => screen.flush());
      final rules = jsonEncode(model.sheets[0]['conditionalFormats']);
      late Map<String, dynamic> recovered;
      await drive(() async {
        recovered = await CoreBridge().call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
      });
      expect(
        jsonEncode(recovered['workspace']['sheets'][0]['conditionalFormats']),
        rules,
      );
      expect(recovered['formatted'][id]['A3']['conditionalStyle'], isNotNull);
    },
  );
}
