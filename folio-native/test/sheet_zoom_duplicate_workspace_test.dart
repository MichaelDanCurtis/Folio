import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets('status-bar zoom and Duplicate sheet use the real engine', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1440, 960);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final folder = Directory(
      '${Directory.systemTemp.path}/folio-zoom-duplicate-ui-tests',
    );
    late WorkspaceModel model;
    await tester.runAsync(() async {
      await folder.create(recursive: true);
      await CoreBridge.initialize();
      final core = CoreBridge();
      await core.call({'op': 'new', 'blank': true});
      await core.call({
        'op': 'importOffice',
        'path': '${Directory.current.path}/test/fixtures/views.xlsx',
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

    Map shown() => model.sheets.firstWhere((s) => s['id'] == screen.sheetId);
    // Zoom in from the status bar: saved on the sheet, one Undo step. The
    // fixture's first sheet is saved at 90%; 100% is stored as no zoom.
    var revision = model.revision;
    expect(shown()['zoom'], 90);
    expect(find.text('90%'), findsOneWidget);
    await tester.tap(find.byKey(const Key('zoom-in')));
    await until(() => shown()['zoom'] == null);
    expect(model.revision, greaterThan(revision));
    await tester.tap(find.byKey(const Key('zoom-in')));
    await until(() => shown()['zoom'] == 125);
    expect(find.text('125%'), findsOneWidget);
    await drive(() => screen.history(false));
    await until(() => shown()['zoom'] == null);
    await drive(() => screen.history(false));
    await until(() => shown()['zoom'] == 90);

    // Duplicate the first sheet from the sheet menu: the copy follows it and
    // is shown.
    final source = model.sheets.indexWhere((s) => s['id'] == screen.sheetId);
    final name = model.sheets[source]['name'] as String;
    final count = model.sheets.length;
    revision = model.revision;
    await tester.tap(find.byKey(const Key('sheet-actions')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('sheet-duplicate')));
    await until(() => model.sheets.length == count + 1);
    expect(model.sheets[source + 1]['name'], '$name (2)');
    expect(
      model.sheets[source + 1]['frozenPane'],
      model.sheets[source]['frozenPane'],
    );
    expect(screen.sheetId, model.sheets[source + 1]['id']);
    await drive(() => screen.history(false));
    await until(() => model.sheets.length == count);
  });
}
