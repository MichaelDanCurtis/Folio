import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_view.dart';

void main() {
  testWidgets(
    'frozen panes scroll, reveal cells, commit edits, save and undo',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late Directory dir;
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-views-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/views.xlsx',
        });
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
      });
      Future<void> drive(Future<void> Function() action) async {
        var done = false;
        Object? error;
        unawaited(
          Future.sync(action).then(
            (_) => done = true,
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
        if (error != null) throw error!;
        expect(done, isTrue);
        await tester.pumpAndSettle();
      }

      Future<void> menu(String label) async {
        final grid = tester.state<SheetViewState>(find.byType(SheetView));
        final rows = label == 'Freeze top row'
            ? 1
            : label == 'Freeze first column'
            ? 0
            : grid.row;
        final columns = label == 'Freeze first column'
            ? 1
            : label == 'Freeze top row'
            ? 0
            : grid.col;
        bool applied() {
          final pane = model.sheets[0]['frozenPane'];
          return label == 'Unfreeze panes'
              ? pane == null
              : pane?['rows'] == rows && pane?['columns'] == columns;
        }

        await tester.ensureVisible(find.byKey(const Key('freeze-panes-menu')));
        await tester.tap(find.byKey(const Key('freeze-panes-menu')));
        await tester.pumpAndSettle();
        await tester.tap(find.text(label));
        await tester.pumpAndSettle();
        await drive(() async {
          for (var i = 0; i < 300 && !applied(); i++) {
            await Future<void>.delayed(const Duration(milliseconds: 10));
          }
        });
        expect(applied(), isTrue);
        await drive(() => model.flush());
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.switchMode(true));
      final grid = screen.sheetKey.currentState!;
      expect(grid.pinnedRows, 3); // row heading + visible frozen rows 1 and 3
      expect(grid.pinnedColumns, 2); // column heading + A, with B hidden
      final anchor = tester.getTopLeft(find.byKey(const Key('cell-A1')));
      final origin = tester.getTopLeft(find.byKey(const Key('cell-G20')));
      final beforeScroll = grid.vertical.offset;
      await tester.sendEventToBinding(
        PointerScrollEvent(
          position: origin + const Offset(20, 10),
          scrollDelta: const Offset(0, 160),
        ),
      );
      await tester.pumpAndSettle();
      expect(grid.vertical.offset, greaterThan(beforeScroll));
      expect(tester.getTopLeft(find.byKey(const Key('cell-A1'))), anchor);
      final beforeHorizontal = grid.horizontal.offset;
      await tester.sendEventToBinding(
        PointerScrollEvent(
          position: tester.getRect(find.byKey(const Key('sheet-grid'))).center,
          scrollDelta: const Offset(170, 0),
        ),
      );
      await tester.pumpAndSettle();
      expect(grid.horizontal.offset, greaterThan(beforeHorizontal));
      expect(tester.getTopLeft(find.byKey(const Key('cell-A1'))), anchor);
      final scrolled = grid.vertical.offset;
      await drive(() => grid.select(0, 0));
      expect(
        grid.vertical.offset,
        scrolled,
      ); // selecting pinned cells doesn't scroll
      await drive(() => grid.select(34, 11));
      final visible = tester.getRect(find.byKey(const Key('cell-L35')));
      final viewport = tester.getRect(find.byKey(const Key('sheet-grid')));
      expect(viewport.contains(visible.center), isTrue);
      expect(
        visible.top,
        greaterThanOrEqualTo(
          tester.getRect(find.byKey(const Key('cell-A3'))).bottom,
        ),
      );
      expect(
        visible.left,
        greaterThanOrEqualTo(
          tester.getRect(find.byKey(const Key('cell-A1'))).right,
        ),
      );
      await drive(() => grid.select(5, 3));
      await tester.enterText(find.byKey(const Key('formula')), 'Pending edit');
      final revision = model.revision;
      await menu('Freeze panes at selection');
      expect(model.sheets[0]['cells']['D6'], 'Pending edit');
      expect(model.revision, revision + 2);
      expect(model.sheets[0]['frozenPane']['rows'], 5);
      expect(model.sheets[0]['frozenPane']['columns'], 3);
      final saved = jsonDecode(
        await tester.runAsync(() => File(model.recoveryPath).readAsString())
            as String,
      );
      expect(saved['sheets'][0]['frozenPane']['topLeftCell'], 'D6');
      await drive(() => screen.history(false));
      expect(model.sheets[0]['frozenPane']['rows'], 3);
      expect(model.sheets[0]['cells']['D6'], 'Pending edit');
      await drive(() => screen.history(false));
      expect(model.sheets[0]['cells']['D6'], '=B6+C6');
      await menu('Unfreeze panes');
      expect(grid.pinnedRows, 1);
      expect(grid.pinnedColumns, 1);
      await menu('Freeze top row');
      expect(grid.pinnedRows, 2);
      expect(grid.pinnedColumns, 1);
      await menu('Freeze first column');
      expect(grid.pinnedRows, 1);
      expect(grid.pinnedColumns, 2);
      await drive(() => grid.select(5, 0));
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pumpAndSettle();
      expect(grid.selected, 'C6'); // hidden B is still skipped
      await drive(() => screen.flush());
      await tester.pumpWidget(const SizedBox());
      model.dispose();
      await tester.runAsync(() => dir.delete(recursive: true));
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );

  testWidgets(
    'oversized saved panes remain editable without changing stored boundary',
    (tester) async {
      final sheet = <String, dynamic>{
        'id': 's',
        'name': 'Huge freeze',
        'cells': <String, dynamic>{},
        'frozenPane': {
          'state': 'frozen',
          'rows': 1000000,
          'columns': 16000,
          'topLeftCell': 'WQK1000001',
        },
      };
      // Set a valid column origin corresponding to the frozen count.
      sheet['frozenPane']['topLeftCell'] = 'WQK1000001';
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SheetView(
              sheet: sheet,
              calculated: const {},
              onSelection: (_) {},
              onChanges: (_) async {},
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final grid = tester.state<SheetViewState>(find.byType(SheetView));
      expect(find.byKey(const Key('frozen-pane-size-notice')), findsOneWidget);
      expect(grid.pinnedRows, 1);
      expect(grid.pinnedColumns, 1);
      expect(grid.frozenRows, 1000000);
      await grid.select(0, 0);
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('cell-A1')), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    },
  );
}
