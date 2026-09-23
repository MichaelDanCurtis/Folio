import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/table_controls.dart';

void main() {
  testWidgets(
    'imported numeric and case-insensitive filter choices stay checked',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: TableFilterDialog(
              table: {
                'columns': [
                  {'name': 'Values'},
                ],
                'filters': {
                  '0': {
                    'kind': 'values',
                    'values': ['1', 'pear', 'nan'],
                    'blank': false,
                  },
                },
              },
              column: 0,
              numericValues: const {'1.0'},
              values: const [
                ('1.0', '1'),
                ('Pear', 'Pear'),
                ('', '(Blanks)'),
                ('01', '01 as text'),
                ('NaN', 'NaN'),
              ],
            ),
          ),
        ),
      );
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey('filter-value-1.0')),
            )
            .value,
        isTrue,
      );
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey('filter-value-Pear')),
            )
            .value,
        isTrue,
      );
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey('filter-value-')),
            )
            .value,
        isFalse,
      );
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey('filter-value-01')),
            )
            .value,
        isFalse,
      );
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey('filter-value-NaN')),
            )
            .value,
        isTrue,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'worksheet filters flush edits, honor manual rows, reapply, clear, remove and undo',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late Directory dir;
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-filter-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/range-sort.xlsx',
        });
        await core.call({
          'op': 'cells',
          'cells': [
            {
              'sheetId': 'office-2',
              'address': 'C1',
              'value': '=SUBTOTAL(9,Records!D2:D7)',
            },
            {
              'sheetId': 'office-2',
              'address': 'D1',
              'value': '=SUBTOTAL(109,Records!D2:D7)',
            },
          ],
        });
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
      });
      Future<T> drive<T>(Future<T> Function() job) async {
        var done = false;
        T? value;
        Object? error;
        unawaited(
          Future.sync(job).then(
            (v) {
              value = v;
              done = true;
            },
            onError: (Object e) {
              error = e;
              done = true;
            },
          ),
        );
        for (var i = 0; i < 500 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (!done) throw StateError('Native operation did not finish');
        if (error != null) throw error!;
        return value as T;
      }

      Future<void> settle() async {
        await tester.pump(const Duration(milliseconds: 300));
        await tester.pumpAndSettle();
      }

      Future<void> changed(int previous) async {
        for (var i = 0; i < 300 && model.revision == previous; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(model.revision, greaterThan(previous));
        await settle();
      }

      Future<void> menuReady(String label) async {
        for (var i = 0; i < 300 && find.text(label).evaluate().isEmpty; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        await settle();
        expect(find.text(label), findsOneWidget);
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await settle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.switchMode(true));
      await settle();
      final grid = screen.sheetKey.currentState!;
      await drive(() => grid.select(1, 1));
      await tester.enterText(find.byKey(const Key('formula')), '5');
      Future<void> menu(String label) async {
        await tester.ensureVisible(find.byKey(const Key('range-filter-menu')));
        await tester.tap(find.byKey(const Key('range-filter-menu')));
        await settle();
        await tester.tap(find.text(label));
      }

      await menu('Filter a range');
      await menuReady('Enable filters');
      expect(model.sheets[0]['cells']['B2'], '5');
      await tester.enterText(
        find.byKey(const Key('worksheet-filter-range')),
        r'$A$1:$D$7',
      );
      var before = model.revision;
      await tester.tap(find.byKey(const Key('worksheet-filter-enable')));
      await changed(before);
      expect(model.sheets[0]['autoFilter']['range'], 'A1:D7');
      expect(find.byKey(const Key('range-filter-0')), findsOneWidget);
      await tester.tap(find.byKey(const Key('range-filter-0')));
      await menuReady('Filter Label');
      await tester.tap(find.text('Clear selection'));
      await settle();
      await tester.tap(find.byKey(const ValueKey('filter-value-Pear')));
      await settle();
      await tester.tap(find.byKey(const ValueKey('filter-value-Hidden')));
      await settle();
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey('filter-value-Pear')),
            )
            .value,
        isTrue,
      );
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey('filter-value-Hidden')),
            )
            .value,
        isTrue,
      );
      before = model.revision;
      await tester.tap(find.byKey(const Key('table-filter-apply')));
      await changed(before);
      expect(model.sheets[0]['tables'], isEmpty);
      expect(model.sheets[0]['autoFilter']['filteredRows'], [3, 6]);
      expect(model.sheets[0]['hiddenRows'], [4]);
      expect(find.byKey(const Key('row-header-2')), findsNothing);
      expect(find.byKey(const Key('row-header-3')), findsNothing);
      expect(find.byKey(const Key('row-header-5')), findsNothing);
      expect(model.snapshot['calculated']['office-2']['C1'], 234);
      expect(model.snapshot['calculated']['office-2']['D1'], 36);
      // Keyboard movement skips both manually hidden and filtered rows.
      await drive(() => grid.select(1, 0));
      grid.gridFocus.requestFocus();
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
      await settle();
      expect(grid.row, 4);
      // Another column's choices reflect the existing label filter.
      await tester.tap(find.byKey(const Key('range-filter-1')));
      await menuReady('Filter Qty');
      expect(find.byKey(const ValueKey('filter-value-7.0')), findsNothing);
      await tester.tap(find.text('Conditions'));
      await settle();
      await tester.tap(find.byKey(const ValueKey('filter-operator-0')));
      await settle();
      await tester.tap(find.text('Greater than').last);
      await settle();
      await tester.enterText(
        find.byKey(const ValueKey('filter-condition-0')),
        '1',
      );
      before = model.revision;
      await tester.tap(find.byKey(const Key('table-filter-apply')));
      await changed(before);
      expect(model.sheets[0]['autoFilter']['filteredRows'], [3, 5, 6, 7]);
      expect(model.snapshot['calculated']['office-2']['C1'], 223);
      expect(model.snapshot['calculated']['office-2']['D1'], 25);
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 'office-1', 'address': 'A3', 'value': 'Pear'},
          ],
        }),
      );
      expect(model.sheets[0]['autoFilter']['filteredRows'], [3, 5, 6, 7]);
      before = model.revision;
      await menu('Reapply worksheet filters');
      await changed(before);
      expect(model.sheets[0]['autoFilter']['filteredRows'], [5, 6, 7]);
      expect(model.snapshot['calculated']['office-2']['D1'], 33);
      await drive(screen.flush);
      final saved = await tester.runAsync(
        () => File(model.recoveryPath).readAsString(),
      );
      expect(jsonDecode(saved!)['sheets'][0]['autoFilter']['filteredRows'], [
        5,
        6,
        7,
      ]);
      before = model.revision;
      await menu('Clear worksheet filters');
      await changed(before);
      expect(model.sheets[0]['autoFilter']['filteredRows'], isEmpty);
      expect(model.sheets[0]['hiddenRows'], [4]);
      expect(find.byKey(const Key('row-header-2')), findsOneWidget);
      expect(find.byKey(const Key('row-header-3')), findsNothing);
      before = model.revision;
      await menu('Remove worksheet filter');
      await changed(before);
      expect(model.sheets[0]['autoFilter'], isNull);
      expect(find.byKey(const Key('range-filter-0')), findsNothing);
      await drive(() => screen.history(false));
      await drive(() => screen.history(false));
      expect(model.sheets[0]['autoFilter']['filteredRows'], [5, 6, 7]);
      expect(model.snapshot['calculated']['office-2']['D1'], 33);
      expect(tester.takeException(), isNull);
      await drive(screen.flush);
      await tester.pumpWidget(const SizedBox());
      model.dispose();
      await tester.runAsync(() => dir.delete(recursive: true));
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );
}
