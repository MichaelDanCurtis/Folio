import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/table_controls.dart';

void main() {
  test('filtered viewport retains physical row numbers and Excel bounds', () {
    final map = VisibleRows({
      'hiddenRows': [1, 5, 1048576],
      'tables': [
        {
          'filteredRows': [3, 5],
        },
      ],
    });
    expect([for (var i = 0; i < 6; i++) map.toPhysical(i)], [1, 3, 5, 6, 7, 8]);
    expect(map.toVisible(5), 2);
    expect(map.next(1, 1), 3);
    expect(map.next(5, -1), 3);
    expect(map.toPhysical(1048576 - map.hidden.length - 1), 1048574);
    expect(VisibleRows({}).toPhysical(1048575), 1048575);
  });
  testWidgets(
    'table menus filter visible rows, sort records, add totals, fill formulas and undo',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late Directory dir;
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-tables-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/tables.xlsx',
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
      var tid = model.sheets[0]['tables'][0]['id'];
      await tester.tap(find.byKey(Key('table-filter-$tid-0')));
      await menuReady('Filter Product');
      await tester.tap(find.byKey(const ValueKey('filter-value-Apple')));
      var before = model.revision;
      await tester.tap(find.byKey(const Key('table-filter-apply')));
      await changed(before);
      expect(model.snapshot['calculated']['office-2']['A2'], 23);
      expect(find.byKey(const Key('row-header-2')), findsNothing);
      expect(find.byKey(const Key('row-header-3')), findsOneWidget);
      // Editing the second visible record must write to physical Excel row 4.
      await tester.tap(find.byKey(const Key('cell-B4')));
      await settle();
      await tester.enterText(find.byKey(const Key('formula')), '9');
      before = model.revision;
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await changed(before);
      expect(model.sheets[0]['cells']['B4'], '9');
      expect(model.sheets[0]['cells']['B3'], '2');
      expect(model.snapshot['calculated']['office-2']['A2'], 87);
      await drive(() => screen.history(false));
      await drive(() => grid.select(3, 1));
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowUp);
      await settle();
      expect(grid.selected, 'B2');
      await tester.tap(find.byKey(const Key('table-menu')));
      await menuReady('Sort table');
      await tester.tap(find.text('Sort table'));
      await settle();
      await tester.tap(find.byKey(const Key('sort-add-level')));
      await settle();
      before = model.revision;
      await tester.tap(find.byKey(const Key('table-sort-apply')));
      await changed(before);
      expect(model.sheets[0]['cells']['A2'], 'Apple');
      expect(model.sheets[0]['cells']['B4'], '3');
      expect(find.byKey(const Key('row-header-1')), findsNothing);
      expect(find.byKey(const Key('row-header-2')), findsOneWidget);
      await drive(() => screen.history(false));
      await tester.tap(find.text('Clear filters'));
      before = model.revision;
      await changed(before);
      expect(find.byKey(const Key('row-header-2')), findsOneWidget);
      // Create a second table through its range dialog and enter a calculated column.
      await drive(() => grid.select(0, 5));
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 'office-1', 'address': 'F1', 'value': 'Units'},
            {'sheetId': 'office-1', 'address': 'G1', 'value': 'Cost'},
            {'sheetId': 'office-1', 'address': 'F2', 'value': '2'},
            {'sheetId': 'office-1', 'address': 'F3', 'value': '4'},
          ],
        }),
      );
      await tester.tap(find.byKey(const Key('table-create')));
      await menuReady('Format as table');
      await tester.enterText(find.byKey(const Key('table-range')), 'F1:G3');
      await tester.enterText(find.byKey(const Key('table-name')), 'Costs');
      before = model.revision;
      await tester.tap(find.byKey(const Key('table-create-apply')));
      await changed(before);
      expect(model.sheets[0]['tables'].length, 2);
      await drive(() => grid.select(1, 6));
      await tester.tap(find.byKey(const Key('table-menu')));
      await menuReady('Calculated column');
      await tester.tap(find.text('Calculated column'));
      await menuReady('Formula for Cost');
      await tester.enterText(
        find.byKey(const Key('table-edit-text')),
        '=[@Units]*10',
      );
      before = model.revision;
      await tester.tap(find.byKey(const Key('table-text-apply')));
      await changed(before);
      expect(model.snapshot['calculated']['office-1']['G2'], 20);
      expect(model.snapshot['calculated']['office-1']['G3'], 40);
      await tester.tap(find.byKey(const Key('table-menu')));
      await menuReady('Show / hide totals row');
      await tester.ensureVisible(find.text('Show / hide totals row'));
      await tester.tap(find.text('Show / hide totals row'));
      before = model.revision;
      await changed(before);
      expect(model.sheets[0]['tables'][1]['range'], 'F1:G4');
      expect(model.snapshot['calculated']['office-1']['G4'], 60);
      await tester.tap(find.byKey(const Key('table-menu')));
      await menuReady('Custom total formula');
      await tester.ensureVisible(find.text('Custom total formula'));
      await tester.tap(find.text('Custom total formula'));
      await menuReady('Total for Cost');
      await tester.enterText(
        find.byKey(const Key('table-edit-text')),
        '=SUM(Costs[Cost])*0.1',
      );
      before = model.revision;
      await tester.tap(find.byKey(const Key('table-text-apply')));
      await changed(before);
      expect(model.snapshot['calculated']['office-1']['G4'], 6);
      expect(
        model.sheets[0]['tables'][1]['columns'][1]['totalsFunction'],
        'custom',
      );
      await tester.tap(find.byKey(const Key('table-menu')));
      await menuReady('Show / hide totals row');
      await tester.ensureVisible(find.text('Show / hide totals row'));
      await tester.tap(find.text('Show / hide totals row'));
      before = model.revision;
      await changed(before);
      expect(model.sheets[0]['tables'][1]['totalsRows'], 0);
      await tester.tap(find.byKey(const Key('table-menu')));
      await menuReady('Show / hide totals row');
      await tester.ensureVisible(find.text('Show / hide totals row'));
      await tester.tap(find.text('Show / hide totals row'));
      before = model.revision;
      await changed(before);
      expect(model.snapshot['calculated']['office-1']['G4'], 6);
      await drive(() => screen.history(false));
      await drive(() => screen.history(false));
      await drive(() => screen.history(false));
      expect(model.snapshot['calculated']['office-1']['G4'], 60);
      await drive(() => screen.history(false));
      expect(model.sheets[0]['tables'][1]['totalsRows'], 0);
      // Resize through the real menu; invalid entries keep the dialog open.
      final originalTable = model.sheets[0]['tables'][1];
      await tester.tap(find.byKey(const Key('table-menu')));
      await menuReady('Resize table');
      await tester.tap(find.text('Resize table'));
      await menuReady('Resize Costs');
      before = model.revision;
      await tester.enterText(
        find.byKey(const Key('table-resize-range')),
        'F1:XFE5',
      );
      await tester.tap(find.byKey(const Key('table-resize-apply')));
      await settle();
      expect(
        find.text('The range exceeds the worksheet boundaries.'),
        findsOneWidget,
      );
      expect(model.revision, before);
      await tester.enterText(
        find.byKey(const Key('table-resize-range')),
        r'$F$1:$H$5',
      );
      await tester.tap(find.byKey(const Key('table-resize-apply')));
      await changed(before);
      expect(model.sheets[0]['tables'][1]['range'], 'F1:H5');
      expect(model.sheets[0]['tables'][1]['columns'][2]['name'], 'Column3');
      expect(model.sheets[0]['cells']['G5'], '=[@Units]*10');
      await drive(() => grid.select(1, 5));
      await tester.tap(find.byKey(const Key('table-menu')));
      await menuReady('Resize table');
      await tester.tap(find.text('Resize table'));
      await menuReady('Resize Costs');
      await tester.enterText(
        find.byKey(const Key('table-resize-range')),
        'F1:F2',
      );
      before = model.revision;
      await tester.tap(find.byKey(const Key('table-resize-apply')));
      await changed(before);
      expect(model.sheets[0]['tables'][1]['range'], 'F1:F2');
      expect(model.sheets[0]['cells']['G3'], '=F3*10');
      expect(model.snapshot['calculated']['office-1']['G3'], 40);
      expect(model.sheets[0]['cells']['F3'], '4');
      await drive(() => screen.history(false));
      expect(model.sheets[0]['tables'][1]['range'], 'F1:H5');
      await drive(() => screen.history(false));
      expect(model.sheets[0]['tables'][1], originalTable);
      await drive(screen.flush);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      model.dispose();
      await tester.runAsync(() => dir.delete(recursive: true));
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );
}
