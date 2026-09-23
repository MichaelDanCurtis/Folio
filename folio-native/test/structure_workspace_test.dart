import 'dart:async';
import 'dart:io';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'native row/column menus and sheet management preserve edits, formulas and Undo',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late Directory dir;
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-structure-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/structure.xlsx',
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
      // An uncommitted formula-bar edit must save before a structural operation.
      await drive(() => grid.select(1, 1));
      await tester.enterText(find.byKey(const Key('formula')), '2500');
      var before = model.revision;
      await tester.tap(
        find.byKey(const Key('row-header-1')),
        kind: PointerDeviceKind.mouse,
        buttons: kSecondaryMouseButton,
      );
      await menuReady('Insert row above');
      await tester.tap(
        find.text('Insert row above'),
        kind: PointerDeviceKind.mouse,
      );
      // Wait for both the pending edit and the structural mutation.
      for (
        var i = 0;
        i < 300 && model.workspace['sheets'][0]['cells']['B3'] != '2500';
        i++
      ) {
        await tester.pump(const Duration(milliseconds: 20));
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 10)),
        );
      }
      expect(model.workspace['sheets'][0]['cells']['B3'], '2500');
      expect(model.workspace['sheets'][0]['cells']['B5'], '=SUM(B3:B4)');
      expect(model.snapshot['calculated']['office-1']['B5'], 10100);
      expect(model.revision, before + 2);
      await drive(() => screen.history(false));
      expect(model.workspace['sheets'][0]['cells']['B2'], '2500');
      expect(model.snapshot['calculated']['office-1']['B4'], 10100);

      // Column header menu and range-count toolbar operations.
      before = model.revision;
      await tester.tap(
        find.byKey(const Key('column-header-1')),
        kind: PointerDeviceKind.mouse,
        buttons: kSecondaryMouseButton,
      );
      await menuReady('Insert column left');
      await tester.tap(find.text('Insert column left'));
      await changed(before);
      expect(model.workspace['sheets'][0]['cells']['C2'], '2500');
      await drive(() => screen.history(false));
      await drive(() => grid.select(1, 1));
      grid.rangeRow = 2;
      grid.rangeCol = 2;
      before = model.revision;
      await tester.tap(find.byKey(const Key('structure-menu')));
      await settle();
      await tester.tap(find.text('Insert selected rows above'));
      await changed(before);
      expect(model.workspace['sheets'][0]['cells']['B4'], '2500');
      await drive(() => screen.history(false));

      // Use the actual tab context menu to rename, then reorder and delete via
      // the reachable sheet-options menu. Formula references follow the stable id.
      await tester.tap(
        find.byKey(const ValueKey('sheet-tab-office-1')),
        kind: PointerDeviceKind.mouse,
        buttons: kSecondaryMouseButton,
      );
      await settle();
      await tester.tap(find.text('Rename sheet'));
      await settle();
      await tester.enterText(
        find.byKey(const Key('sheet-name')),
        "O'Brien budget",
      );
      before = model.revision;
      await tester.tap(find.text('Rename'));
      await changed(before);
      expect(model.sheets[0]['name'], "O'Brien budget");
      expect(model.sheets[1]['cells']['B1'], "=SUM('O''Brien budget'!B2:B3)");
      await tester.tap(find.byKey(const Key('sheet-actions')));
      await settle();
      before = model.revision;
      await tester.tap(find.text('Move sheet right'));
      await changed(before);
      expect(model.sheets[1]['id'], 'office-1');
      expect(screen.sheetId, 'office-1');
      await tester.tap(find.byKey(const Key('sheet-actions')));
      await settle();
      await tester.tap(find.text('Delete sheet'));
      await settle();
      before = model.revision;
      await tester.tap(find.text('Delete'));
      await changed(before);
      expect(model.sheets.length, 1);
      expect(screen.sheetId, 'office-2');
      expect(model.sheets[0]['cells']['B1'], '=SUM(#REF!)');
      await drive(() => screen.history(false));
      expect(model.sheets.length, 2);
      expect(model.sheets[1]['cells']['B2'], '2500');
      expect(tester.takeException(), isNull);
      await drive(screen.flush);
      await tester.pumpWidget(const SizedBox());
      model.dispose();
      await tester.runAsync(() => dir.delete(recursive: true));
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );
}
