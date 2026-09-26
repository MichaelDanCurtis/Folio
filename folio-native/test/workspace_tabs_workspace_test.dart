import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/workspace_file_commands.dart';
import 'package:folio_native/src/workspace_tabs.dart';

void main() {
  testWidgets('files open in tabs with their own engines and come back after '
      'a restart', (tester) async {
    tester.view.physicalSize = const Size(1440, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(fileMenuChannel, (_) async => null);
    addTearDown(
      () => messenger.setMockMethodCallHandler(fileMenuChannel, null),
    );
    late Directory dir;
    late WorkspaceModel first;
    await tester.runAsync(() async {
      await CoreBridge.initialize();
      dir = await Directory.systemTemp.createTemp('folio-tabs-');
      final core = CoreBridge();
      await core.call({'op': 'new', 'blank': true});
      first = WorkspaceModel(core, '${dir.path}/workspace.folio');
      await first.initialize();
    });

    Future<void> until(bool Function() ready) async {
      for (var i = 0; i < 600 && !ready(); i++) {
        await tester.pump(const Duration(milliseconds: 20));
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 5)),
        );
      }
      expect(ready(), isTrue);
      await tester.pumpAndSettle();
    }

    Future<T> drive<T>(Future<T> Function() action) async {
      T? value;
      var done = false;
      Object? error;
      unawaited(
        Future.sync(action).then(
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
      await until(() => done);
      if (error != null) throw error!;
      return value as T;
    }

    String cells(WorkspaceModel model) =>
        jsonEncode((model.sheets.first as Map)['cells']);
    final screens = find.byType(WorkspaceScreen, skipOffstage: false);

    await tester.pumpWidget(
      FolioApp(model: first, agentDirectory: '${dir.path}/agent'),
    );
    await tester.pumpAndSettle();
    var host = tester.state<WorkspaceTabsState>(find.byType(WorkspaceTabs));
    expect(screens, findsOneWidget);
    // One file shows no tab strip.
    expect(find.byKey(const ValueKey('workspace-tab-strip')), findsNothing);

    // New Tab (the File menu's command) opens a blank workspace in front.
    final firstScreen = tester.state<WorkspaceScreenState>(screens.first);
    await drive(() => firstScreen.fileCommand(WorkspaceFileCommand.newTab));
    expect(host.tabs, hasLength(2));
    expect(host.active, 1);
    expect(screens, findsNWidgets(2));
    expect(find.byKey(const ValueKey('workspace-tab-strip')), findsOneWidget);
    final second = host.tabs[1];
    expect(second.model.snapshot['blankWorkspace'], isTrue);
    expect(second.model.core.session, second.id);
    expect(tester.widgetList<WorkspaceScreen>(screens).map((s) => s.active), [
      false,
      true,
    ]);

    // Each tab has its own engine.
    await drive(
      () => second.model.mutate({
        'op': 'cells',
        'cells': [
          {'sheetId': 'sheet-1', 'address': 'B4', 'value': 'tab two'},
        ],
      }),
    );
    expect(cells(second.model), contains('tab two'));
    final engineOne = await drive(() => first.core.call({'op': 'get'}));
    expect(
      jsonEncode(engineOne['workspace']['sheets'][0]['cells']),
      isNot(contains('tab two')),
    );

    // Switching tabs moves the menus and focus to the tab in front.
    await tester.tap(find.byKey(const ValueKey('workspace-tab-chip-main')));
    await tester.pumpAndSettle();
    expect(tester.widgetList<WorkspaceScreen>(screens).map((s) => s.active), [
      true,
      false,
    ]);
    host.select(1);
    await tester.pumpAndSettle();

    // The open tabs are remembered for the next launch.
    final stored = jsonDecode(File('${dir.path}/tabs.json').readAsStringSync());
    expect(stored, {
      'tabs': ['main', second.id],
      'active': 1,
    });
    await drive(second.model.flush);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
    first.dispose();
    second.model.dispose();

    // A restart restores both tabs, the second in front, with its work.
    late WorkspaceModel restartedFirst;
    await tester.runAsync(() async {
      restartedFirst = WorkspaceModel(
        CoreBridge(),
        '${dir.path}/workspace.folio',
      );
      await restartedFirst.initialize();
    });
    await tester.pumpWidget(
      FolioApp(model: restartedFirst, agentDirectory: '${dir.path}/agent'),
    );
    host = tester.state<WorkspaceTabsState>(find.byType(WorkspaceTabs));
    await until(() => host.tabs.length == 2);
    expect(host.active, 1);
    final restored = host.tabs[1];
    expect(restored.id, second.id);
    expect(cells(restored.model), contains('tab two'));

    // Closing a tab with unsaved work asks first; Don’t Save closes it and
    // removes its files.
    bool? closed;
    unawaited(host.closeTab(restored).then((v) => closed = v));
    await until(
      () =>
          find.text('Save changes to your files?').evaluate().isNotEmpty ||
          closed != null,
    );
    if (find.text('Save changes to your files?').evaluate().isNotEmpty) {
      await tester.tap(find.widgetWithText(TextButton, 'Don’t Save').last);
      await tester.pumpAndSettle();
    }
    await until(() => closed != null);
    expect(closed, isTrue);
    expect(host.tabs, hasLength(1));
    expect(screens, findsOneWidget);
    expect(find.byKey(const ValueKey('workspace-tab-strip')), findsNothing);
    await until(() => !Directory('${dir.path}/tabs/${second.id}').existsSync());
    expect(File('${dir.path}/tabs.json').existsSync(), isFalse);
    // The last tab stays open.
    expect(await drive(() => host.closeTab(host.tabs.single)), isFalse);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
    restartedFirst.dispose();
    await tester.runAsync(() => dir.delete(recursive: true));
  });

  test('a damaged tab list falls back to the first workspace', () async {
    final dir = await Directory.systemTemp.createTemp('folio-tabs-list-');
    addTearDown(() => dir.delete(recursive: true));
    expect(readOpenTabs(dir.path).ids, ['main']);
    File('${dir.path}/tabs.json').writeAsStringSync('{not json');
    expect(readOpenTabs(dir.path).ids, ['main']);
    // Tabs whose workspace is gone, and unsafe names, are dropped.
    File(
      '${dir.path}/tabs.json',
    ).writeAsStringSync('{"tabs":["main","gone","../x"],"active":2}');
    final read = readOpenTabs(dir.path);
    expect(read.ids, ['main']);
    expect(read.active, 0);
  });
}
