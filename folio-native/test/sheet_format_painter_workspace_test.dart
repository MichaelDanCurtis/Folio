import 'dart:async';
import 'dart:io';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_font.dart';

void main() {
  testWidgets('Format Painter paints cell formats once, or until Esc', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1440, 960);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final folder = Directory(
      '${Directory.systemTemp.path}/folio-sheet-painter-ui-tests',
    );
    late WorkspaceModel model;
    await tester.runAsync(() async {
      await folder.create(recursive: true);
      await CoreBridge.initialize();
      final core = CoreBridge();
      await core.call({'op': 'new', 'blank': true});
      await core.call({
        'op': 'importOffice',
        'path': '${Directory.current.path}/test/fixtures/clipboard.xlsx',
      });
      final got = await core.call({'op': 'get'});
      final sheets = (got['sheets'] ?? got['workspace']['sheets']) as List;
      await core.call({
        'op': 'formatCells',
        'sheetId': sheets.first['id'],
        'addresses': ['A1', 'A2'],
        'style': {'bold': true, 'fill': '#ffeeaa', 'numFmt': '0.00%'},
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
    }

    Future<void> drive(Future<dynamic> Function() job) async {
      var done = false;
      Object? error;
      unawaited(
        Future.sync(job).then(
          (_) => done = true,
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

    Map sheet() => model.sheets.firstWhere((s) => s['id'] == screen.sheetId);
    Map style(String a) => cellStyle(sheet(), a);
    final values = sheet()['cells'];
    await tester.tap(find.byKey(const Key('cell-A1')));
    await tester.pumpAndSettle();
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.tap(find.byKey(const Key('cell-A2')));
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pumpAndSettle();

    // One paint: from the clicked cell, over the copied range's size.
    await tester.tap(find.byKey(const Key('sheet-format-painter')));
    await tester.pump(kDoubleTapTimeout);
    await until(() => screen.sheetKey.currentState?.painterToken != null);
    await tester.tap(find.byKey(const Key('cell-C3')));
    await until(() => style('C4')['bold'] == true);
    expect((style('C3')['fill'] as String).toLowerCase(), '#ffeeaa');
    expect(style('C3')['numFmt'], '0.00%');
    expect(style('C5')['bold'], isNot(true));
    expect(sheet()['cells'], values);
    await tester.tap(find.byKey(const Key('cell-A6')));
    await tester.pumpAndSettle();
    await drive(screen.flush);
    expect(style('A6')['bold'], isNot(true));

    // A double-click keeps it on until Esc.
    await tester.tap(find.byKey(const Key('cell-A1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('sheet-format-painter')));
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tap(find.byKey(const Key('sheet-format-painter')));
    await tester.pumpAndSettle();
    await until(() => screen.sheetKey.currentState?.painterToken != null);
    await tester.tap(find.byKey(const Key('cell-B7')));
    await until(() => style('B7')['bold'] == true);
    await tester.tap(find.byKey(const Key('cell-C7')));
    await until(() => style('C7')['bold'] == true);
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('cell-A9')));
    await tester.pumpAndSettle();
    await drive(screen.flush);
    expect(style('A9')['bold'], isNot(true));
  });
}
