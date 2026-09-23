import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/scenario_manager.dart';

void main() {
  testWidgets(
    'external scenarios browse, cancel, merge, apply, undo and roundtrip in the real app',
    (t) async {
      t.view.physicalSize = const Size(1440, 1000);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      late String sourcePath, sourceBytes;
      await t.runAsync(() async {
        await CoreBridge.initialize();
        final core = CoreBridge();
        folder = await Directory.systemTemp.createTemp(
          'folio-scenario-merge-ui-',
        );
        sourcePath = '${folder.path}/sales.xlsx';
        await core.call({'op': 'new', 'blank': true});
        var w = (await core.call({'op': 'get'}))['workspace'];
        w['sheets'] = [
          {
            'id': 'inputs',
            'name': 'Inputs',
            'cells': {'A1': '1', 'B1': '2', 'C1': '=A1*B1'},
          },
        ];
        await core.call({'op': 'replace', 'workspace': w});
        w = (await core.call({'op': 'get'}))['workspace'];
        await core.call({
          'op': 'scenarios',
          'action': 'add',
          'sheetId': 'inputs',
          'workspaceId': w['id'],
          'baseRevision': w['revision'],
          'name': 'Sales',
          'inputs': [
            {'address': 'A1', 'value': '12'},
            {'address': 'B1', 'value': '25'},
          ],
        });
        await core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': sourcePath,
        });
        sourceBytes = base64Encode(await File(sourcePath).readAsBytes());
        await core.call({'op': 'new', 'blank': true});
        w = (await core.call({'op': 'get'}))['workspace'];
        w['title'] = 'Cross-workbook scenario check';
        w['sheets'] = [
          {
            'id': 'office-1',
            'name': 'Inputs',
            'cells': {'A1': '1', 'B1': '2', 'C1': '=A1*B1'},
            'cellWatches': {'C1': true},
          },
        ];
        await core.call({'op': 'replace', 'workspace': w});
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        await model.initialize();
      });
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      const selector = MethodChannel('plugins.flutter.io/file_selector');
      messenger.setMockMethodCallHandler(selector, (call) async {
        expect(call.method, 'openFile');
        return [sourcePath];
      });
      addTearDown(() => messenger.setMockMethodCallHandler(selector, null));
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
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (error != null) throw error!;
        expect(done, isTrue);
        await t.pumpAndSettle();
      }

      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 500 && !ready(); i++) {
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(ready(), isTrue);
        await t.pumpAndSettle();
      }

      await t.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await t.pumpAndSettle();
      final screen = t.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      addTearDown(() async {
        await t.pumpWidget(const SizedBox());
        model.dispose();
      });
      await drive(() => screen.switchMode(true));
      await t.tap(find.text('Data'));
      await t.pumpAndSettle();
      Future<void> manager() async {
        await t.tap(find.byKey(const Key('scenario-manager-button')));
        await until(() => find.byType(ScenarioManager).evaluate().isNotEmpty);
      }

      Future<void> browse() async {
        await t.tap(find.byKey(const Key('scenario-merge')));
        await until(
          () => find.byType(ScenarioMergeDialog).evaluate().isNotEmpty,
        );
        await t.tap(find.byKey(const Key('scenario-browse-source')));
        await until(() => find.text('sales.xlsx').evaluate().isNotEmpty);
        await t.tap(
          find.descendant(
            of: find.byType(ScenarioMergeDialog),
            matching: find.byType(DropdownButton<String>),
          ),
        );
        await t.pumpAndSettle();
        await t.tap(find.text('Inputs').last);
        await t.pumpAndSettle();
        await t.tap(find.text('Sales'));
        await t.pumpAndSettle();
      }

      await drive(screen.flush);
      final before = model.workspace;
      await t.runAsync(
        () => File(
          '${folder.path}/before.folio',
        ).writeAsString(jsonEncode(before)),
      );
      await manager();
      await browse();
      expect(model.workspace, before);
      await t.tap(find.text('Cancel'));
      await until(() => find.byType(ScenarioMergeDialog).evaluate().isEmpty);
      expect(model.workspace, before);
      await browse();
      await t.tap(find.text('Copy scenarios'));
      await until(() => find.byType(ScenarioMergeDialog).evaluate().isEmpty);
      expect(model.revision, (before['revision'] as int) + 1);
      expect(model.sheets[0]['cells'], before['sheets'][0]['cells']);
      expect(model.snapshot['calculated']['office-1']['C1'], 2);
      expect(model.sheets[0]['scenarios']['items'][0]['name'], 'Sales');
      expect(screen.sheetId, 'office-1');
      await t.runAsync(
        () => File(
          '${folder.path}/merged.folio',
        ).writeAsString(jsonEncode(model.workspace)),
      );
      await t.tap(find.byKey(const Key('scenario-show')));
      await until(() => model.snapshot['calculated']['office-1']['C1'] == 300);
      await t.tap(find.text('Close'));
      await until(() => find.byType(ScenarioManager).evaluate().isEmpty);
      await drive(() => model.mutate({'op': 'undo'}));
      expect(model.snapshot['calculated']['office-1']['C1'], 2);
      await drive(() => model.mutate({'op': 'redo'}));
      expect(model.snapshot['calculated']['office-1']['C1'], 300);
      final finalSheets = model.sheets;
      await t.runAsync(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/merged.xlsx',
        });
        expect(base64Encode(await File(sourcePath).readAsBytes()), sourceBytes);
        final loaded = await model.core.call({
          'op': 'load',
          'path': model.recoveryPath,
        });
        expect(loaded['workspace']['sheets'], finalSheets);
        final imported = await model.core.call({
          'op': 'importOffice',
          'path': '${folder.path}/merged.xlsx',
        });
        expect(
          imported['workspace']['sheets'][0]['scenarios']['items'][0]['name'],
          'Sales',
        );
        expect(imported['workspace']['sheets'][0]['cells']['C1'], '=A1*B1');
        expect(imported['workspace']['sheets'][0]['cellWatches'], {'C1': true});
        expect(imported['calculated']['office-1']['C1'], 300);
        await File(
          '../../work/scenarios-merge-artifact-path.txt',
        ).writeAsString(folder.path);
      });
      expect(t.takeException(), isNull);
    },
  );
}
