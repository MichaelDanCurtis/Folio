import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'sheet_text_overflow_test.dart' show overflowInk;

void main() {
  testWidgets(
    'full workspace overflow responds to entry and Undo without exporting borrowed text',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-overflow-ui-tests',
      );
      const label =
          'A long project heading that extends across empty worksheet cells';
      late WorkspaceModel model;
      late String id;
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        await CoreBridge.initialize();
        final core = CoreBridge();
        final first = await core.call({'op': 'new', 'blank': true});
        id = first['workspace']['sheets'][0]['id'] as String;
        await core.call({
          'op': 'cells',
          'cells': [
            {'sheetId': id, 'address': 'A1', 'value': label},
            {'sheetId': id, 'address': 'A2', 'value': '=1/0'},
            {
              'sheetId': id,
              'address': 'A3',
              'value': '="A formula can also return a long text label"',
            },
            {
              'sheetId': id,
              'address': 'A4',
              'value': "'123456789012345678901234567890",
            },
          ],
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        model.snapshot = await core.call({'op': 'get'});
      });
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
        for (var i = 0; i < 500 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(done, isTrue);
        if (error != null) throw error!;
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
      }

      await tester.pumpWidget(
        RepaintBoundary(
          key: const Key('capture'),
          child: FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
        ),
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
      expect(model.snapshot['formatted'][id]['A2']['isError'], true);
      expect(await overflowInk(tester, 'B1'), greaterThan(30));
      expect(await overflowInk(tester, 'B3'), greaterThan(30));
      expect(await overflowInk(tester, 'B4'), greaterThan(30));
      final grid = screen.sheetKey.currentState!;
      await tester.tap(find.byKey(const Key('cell-B1')));
      await tester.sendKeyEvent(LogicalKeyboardKey.keyX, character: 'x');
      await drive(grid.flush);
      expect(model.sheets.first['cells']['B1'], 'x');
      expect(await overflowInk(tester, 'C1'), 0);
      await drive(() => screen.history(false));
      expect(await overflowInk(tester, 'C1'), greaterThan(30));
      await drive(() => screen.history(true));
      expect(await overflowInk(tester, 'C1'), 0);
      await drive(() => screen.history(false));
      await drive(model.flush);
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/overflow.xlsx',
        }),
      );
      await drive(() => model.open('${folder.path}/workspace.folio'));
      expect(model.sheets.first['cells']['A1'], label);
      expect(model.sheets.first['cells'].containsKey('B1'), false);
      expect(await overflowInk(tester, 'C1'), greaterThan(30));
    },
  );
}
