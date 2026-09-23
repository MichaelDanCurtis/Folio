import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' show Tristate;
import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/table_controls.dart';

void main() {
  test(
    'sparse dimensions scroll to distant cells and respect hidden edges',
    () {
      final visible = VisibleAxis([0, 2, 4, 1048575], 1048576);
      final layout = SheetAxisLayout(visible, 20, {
        1: 40,
        2: 900,
        3: 10,
        1048574: 30,
      });
      expect(visible.next(1, -1), 1);
      expect(visible.next(1048574, 1), 1048574);
      expect(layout.offset(1), 0);
      expect(layout.offset(3), 40);
      expect(layout.offset(5), 50);
      expect(layout.offset(1048574), (1048574 - 3) * 20 + 10);
      expect(layout.extent(1048574), 30);
      final cols = VisibleColumns({
        'hiddenColumns': [1, 3, 16384],
      });
      expect(cols.toPhysical(0), 1);
      expect(cols.next(1, 1), 3);
      expect(cols.next(16382, 1), 16382);
    },
  );
  testWidgets(
    'dimension menus commit edits, resize, hide, navigate, drag, save and undo',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final semantics = tester.ensureSemantics();
      late Directory dir;
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-geometry-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/geometry.xlsx',
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
      expect(screen.selectedCell, grid.selected);
      expect(grid.selected, 'E5');
      expect(find.byKey(const Key('column-header-1')), findsNothing);
      expect(find.byKey(const Key('column-header-4')), findsOneWidget);
      await tester.tap(find.byKey(const Key('cell-A2')));
      await settle();
      expect(grid.gridFocus.hasPrimaryFocus, isTrue);
      expect(
        tester
            .getSemantics(find.byKey(const Key('sheet-grid-focus')))
            .flagsCollection
            .isFocused,
        Tristate.isTrue,
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await settle();
      expect(grid.selected, 'E2');
      expect(grid.gridFocus.hasPrimaryFocus, isTrue);
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await settle();
      expect(grid.selected, 'F2');
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
      await settle();
      expect(grid.selected, 'E2');
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await settle();
      expect(grid.selected, 'A2');
      expect(grid.selectedRange, 'A2');
      expect(grid.gridFocus.hasPrimaryFocus, isTrue);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await settle();
      await tester.sendKeyEvent(LogicalKeyboardKey.f2);
      await settle();
      expect(grid.editing, isTrue);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await settle();
      expect(grid.editing, isFalse);
      expect(grid.selectedRange, 'A2');
      await drive(() => grid.select(1, 0));
      await tester.enterText(find.byKey(const Key('formula')), 'Edited');
      Future<void> menu(String label) async {
        await tester.tap(find.byKey(const Key('structure-menu')));
        await menuReady(label);
        await tester.ensureVisible(find.text(label));
        await tester.tap(find.text(label));
        if (label == 'Row height…' || label == 'Column width…') {
          // The dialog opens only after the pending native edit and row layout
          // transaction finishes; pumpAndSettle alone does not await FFI work.
          for (
            var i = 0;
            i < 300 &&
                find.byKey(const Key('dimension-size')).evaluate().isEmpty;
            i++
          ) {
            await tester.pump(const Duration(milliseconds: 20));
            await tester.runAsync(
              () => Future<void>.delayed(const Duration(milliseconds: 10)),
            );
          }
          expect(find.byKey(const Key('dimension-size')), findsOneWidget);
        }
        await settle();
      }

      await menu('Row height…');
      expect(model.sheets[0]['cells']['A2'], 'Edited');
      await tester.enterText(find.byKey(const Key('dimension-size')), '410');
      await tester.tap(find.byKey(const Key('dimension-apply')));
      await settle();
      expect(find.text('Enter a number from 0 to 409.'), findsOneWidget);
      await tester.enterText(find.byKey(const Key('dimension-size')), '36');
      var before = model.revision;
      await tester.tap(find.byKey(const Key('dimension-apply')));
      await changed(before);
      expect(model.sheets[0]['rowHeights']['2'], 36);
      expect(tester.getSize(find.byKey(const Key('cell-A2'))).height, 48);
      await menu('Column width…');
      await tester.enterText(find.byKey(const Key('dimension-size')), '28.5');
      before = model.revision;
      await tester.tap(find.byKey(const Key('dimension-apply')));
      await changed(before);
      expect(
        tester.getSize(find.byKey(const Key('cell-A2'))).width,
        closeTo(28.5 * grid.columnMetrics.digitWidth + 5, 1),
      );
      before = model.revision;
      await menu('Hide selected rows');
      await changed(before);
      expect(model.sheets[0]['hiddenRows'], [2, 4]);
      expect(find.byKey(const Key('row-header-1')), findsNothing);
      expect(grid.selected, 'A3');
      before = model.revision;
      await tester.tap(
        find.byKey(const Key('row-header-2')),
        buttons: kSecondaryMouseButton,
      );
      await menuReady('Unhide adjacent rows');
      await tester.tap(find.text('Unhide adjacent rows'));
      await changed(before);
      expect(model.sheets[0]['hiddenRows'], isEmpty);
      expect(find.byKey(const Key('row-header-1')), findsOneWidget);
      // Hidden columns remain directly addressable so the first column can be unhidden.
      await drive(() => grid.select(1, 1));
      expect(grid.selected, 'B2');
      before = model.revision;
      await menu('Unhide selected columns');
      await changed(before);
      expect(model.sheets[0]['hiddenColumns'], [3, 4]);
      expect(find.byKey(const Key('cell-B2')), findsOneWidget);
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await settle();
      expect(grid.selected, 'E2');
      final previous = (model.sheets[0]['columnWidths']['2'] as num).toDouble();
      final previousPixels = grid.columnLayout.extent(1);
      before = model.revision;
      await tester.drag(
        find.byKey(const Key('column-resize-1')),
        const Offset(35, 0),
      );
      await changed(before);
      expect(grid.columnLayout.extent(1), closeTo(previousPixels + 35, 1));
      expect(model.sheets[0]['columnWidths']['2'], greaterThan(previous));
      before = model.revision;
      await tester.drag(
        find.byKey(const Key('row-resize-1')),
        const Offset(0, 24),
      );
      await changed(before);
      expect(model.sheets[0]['rowHeights']['2'], 54);
      await drive(() => screen.history(false));
      expect(model.sheets[0]['rowHeights']['2'], 36);
      await drive(() => screen.history(false));
      expect(model.sheets[0]['columnWidths']['2'], previous);
      before = model.revision;
      await menu('Unhide all columns');
      await changed(before);
      expect(model.sheets[0]['hiddenColumns'], isEmpty);
      // A narrow cell must clip gracefully without overflowing its row.
      await drive(
        () => screen.changeSheetGeometry({
          'axis': 'columns',
          'action': 'resize',
          'at': 2,
          'count': 1,
          'size': 1,
        }),
      );
      await settle();
      expect(tester.takeException(), isNull);
      await drive(screen.flush);
      final saved = await tester.runAsync(
        () => File('${dir.path}/workspace.folio').readAsString(),
      );
      final state = jsonDecode(saved!);
      expect(state['sheets'][0]['rowHeights']['2'], 36);
      expect(state['sheets'][0]['columnWidths']['2'], 1);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      semantics.dispose();
      model.dispose();
      await tester.runAsync(() => dir.delete(recursive: true));
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );
}
