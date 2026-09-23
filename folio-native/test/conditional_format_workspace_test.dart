import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_conditional_dialog.dart';
import 'package:folio_native/src/sheet_font.dart';
import 'package:folio_native/src/sheet_number_format.dart';

void main() {
  test(
    'conditional font properties override rich runs and number-format colors',
    () {
      final style = numberDisplayStyle(
        {
          '_richText': [
            {
              'text': 'Hello',
              'style': {'bold': false, 'color': '#111111'},
            },
          ],
        },
        {
          'color': '#FF0000',
          'conditionalStyle': {'bold': true, 'color': '#00FF00'},
        },
      );
      final runs = sheetFontRuns('Hello', style);
      expect(runs.single.font['bold'], true);
      expect(runs.single.font['color'], '#00FF00');
    },
  );
  testWidgets(
    'rule manager, populated and blank cells, history, cancel and persistence use the real engine',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-conditional-ui-tests',
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
              'A1': '5',
              'A2': '20',
              'B1': '2',
              'B2': '=A2*2',
            }.entries)
              {'sheetId': id, 'address': e.key, 'value': e.value},
          ],
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        model.snapshot = await core.call({'op': 'get'});
        await core.call({'op': 'save', 'path': '${folder.path}/seed.folio'});
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
      final grid = screen.sheetKey.currentState!;
      Future<void> manager() async {
        await tester.tap(find.byKey(const Key('conditional-format-button')));
        await until(
          () => find.byType(SheetConditionalDialog).evaluate().isNotEmpty,
        );
      }

      await manager();
      await tester.tap(find.byKey(const Key('conditional-new')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('conditional-range')),
        'A1:A3',
      );
      await tester.enterText(
        find.byKey(const Key('conditional-first')),
        'SUM(',
      );
      await tester.tap(find.byKey(const Key('conditional-rule-done')));
      await tester.pumpAndSettle();
      expect(find.textContaining('greater than SUM('), findsOneWidget);
      expect((model.sheets[0]['conditionalFormats'] as List?) ?? [], isEmpty);
      await tester.tap(find.byKey(const Key('conditional-apply')));
      await until(
        () => find.textContaining('cannot be copied').evaluate().isNotEmpty,
      );
      expect(find.byType(SheetConditionalDialog), findsOneWidget);
      expect((model.sheets[0]['conditionalFormats'] as List?) ?? [], isEmpty);
      await tester.tap(find.text('Edit rule'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byKey(const Key('conditional-first')), '10');
      await tester.tap(find.byKey(const Key('conditional-rule-done')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('conditional-apply')));
      await until(
        () =>
            ((model.sheets[0]['conditionalFormats'] as List?) ?? []).length ==
            1,
      );
      expect(grid.displayAt('A2')?['conditionalStyle']['fill'], '#FFC7CE');
      expect(grid.displayAt('A1')?['conditionalStyle'], isNull);
      final colored = tester
          .widgetList<Container>(
            find.descendant(
              of: find.byKey(const Key('cell-A2')),
              matching: find.byType(Container),
            ),
          )
          .where(
            (c) =>
                (c.decoration as BoxDecoration?)?.color ==
                const Color(0xffffc7ce),
          );
      expect(colored, isNotEmpty);
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': id, 'address': 'A2', 'value': '2'},
          ],
        }),
      );
      expect(grid.displayAt('A2')?['conditionalStyle'], isNull);
      await drive(() => screen.history(false));
      expect(grid.displayAt('A2')?['conditionalStyle']['fill'], '#FFC7CE');
      await manager();
      await tester.tap(find.byKey(const Key('conditional-rule-0')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete rule'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect((model.sheets[0]['conditionalFormats'] as List).length, 1);
      await drive(
        () => model.mutate({
          'op': 'conditionalFormat',
          'sheetId': id,
          'baseRevision': model.revision,
          'action': 'add',
          'rule': {
            'sqref': 'D1:D1048576',
            'type': 'containsBlanks',
            'style': {'fill': '#EEEEEE'},
          },
        }),
      );
      await until(
        () => grid.displayAt('D2')?['conditionalStyle']?['fill'] == '#EEEEEE',
      );
      final revision = model.revision;
      await manager();
      await tester.tap(find.byKey(const Key('conditional-rule-1')));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Move rule up'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('conditional-apply')));
      await until(() => model.revision > revision);
      expect(model.sheets[0]['conditionalFormats'][0]['type'], 'cellIs');
      await drive(() => screen.history(false));
      expect(
        model.sheets[0]['conditionalFormats'][0]['type'],
        'containsBlanks',
      );
      await drive(
        () => model.core.call({
          'op': 'save',
          'path': '${folder.path}/conditional.folio',
        }),
      );
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/conditional.xlsx',
        }),
      );
      await drive(() => screen.flush());
    },
  );
}
