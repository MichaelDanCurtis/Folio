import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/scenario_manager.dart';

void main() {
  testWidgets(
    'Scenario Manager creates, edits, applies, retries, summarizes and roundtrips through the real engine',
    (t) async {
      t.view.physicalSize = const Size(1440, 1000);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      final capture = Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1';
      await t.runAsync(() async {
        if (capture) {
          for (final (name, path) in [
            ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
            ('Aptos', '/System/Library/Fonts/Supplemental/Arial.ttf'),
            (
              'MaterialIcons',
              '/opt/homebrew/share/flutter/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
            ),
          ]) {
            await (FontLoader(name)..addFont(
                  Future.value(
                    ByteData.sublistView(await File(path).readAsBytes()),
                  ),
                ))
                .load();
          }
        }
        await CoreBridge.initialize();
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        final w = (await core.call({'op': 'get'}))['workspace'];
        w['title'] = 'Scenario sample';
        w['sheets'] = [
          {
            'id': 'inputs',
            'name': 'Inputs',
            'cells': {'A1': '10', 'B1': '20', 'C1': '=A1*B1'},
            'cellWatches': {'C1': true},
          },
          {
            'id': 'result',
            'name': 'Result',
            'cells': {'A1': '=Inputs!C1'},
          },
        ];
        await core.call({'op': 'replace', 'workspace': w});
        folder = await Directory.systemTemp.createTemp('folio-scenarios-ui-');
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
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
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (error != null) throw error!;
        expect(done, isTrue);
        await t.pumpAndSettle();
      }

      Future<void> until(Finder f, {bool exists = true}) async {
        for (var i = 0; i < 500; i++) {
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
          if (f.evaluate().isNotEmpty == exists &&
              find.byType(LinearProgressIndicator).evaluate().isEmpty) {
            break;
          }
        }
        expect(f, exists ? findsOneWidget : findsNothing);
        expect(find.byType(LinearProgressIndicator), findsNothing);
        await t.pumpAndSettle();
      }

      final boundary = GlobalKey();
      await t.pumpWidget(
        RepaintBoundary(
          key: boundary,
          child: FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
        ),
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
      await drive(() async {
        await screen.sheetKey.currentState!.select(0, 0, extend: false);
      });
      await t.tap(find.text('Data'));
      await t.pumpAndSettle();
      Future<void> manager() async {
        await t.tap(find.byKey(const Key('scenario-manager-button')));
        await until(find.byType(ScenarioManager));
      }

      Future<void> add(String name, String a, String b) async {
        await t.tap(find.byKey(const Key('scenario-add')));
        await until(find.byType(ScenarioEditor));
        await t.enterText(find.byKey(const Key('scenario-name')), name);
        await t.enterText(find.byKey(const Key('scenario-range')), 'A1:B1');
        await t.tap(find.byKey(const Key('scenario-load-values')));
        await until(find.byKey(const Key('scenario-value-0')));
        await t.enterText(find.byKey(const Key('scenario-value-0')), a);
        await t.enterText(find.byKey(const Key('scenario-value-1')), b);
        await t.tap(find.byKey(const Key('scenario-save')));
        await until(find.byType(ScenarioEditor), exists: false);
      }

      Future<void> value(num n) async {
        for (
          var i = 0;
          i < 500 && model.snapshot['calculated']['result']['A1'] != n;
          i++
        ) {
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(model.snapshot['calculated']['result']['A1'], n);
        await t.pumpAndSettle();
      }

      await drive(screen.flush);
      final original = model.workspace['sheets'];
      final generation = model.snapshot['evaluatedCells'];
      await manager();
      await add('Base', '10', '20');
      await add('Growth', '15', '30');
      expect(model.workspace['sheets'][0]['cells'], original[0]['cells']);
      expect(model.snapshot['evaluatedCells'], generation);
      await t.tap(find.byKey(const Key('scenario-edit')));
      await until(find.byType(ScenarioEditor));
      await t.enterText(find.byKey(const Key('scenario-name')), 'Upside');
      await t.tap(find.byKey(const Key('scenario-save')));
      await until(find.byType(ScenarioEditor), exists: false);
      expect(
        model.workspace['sheets'][0]['scenarios']['items'][1]['name'],
        'Upside',
      );
      await t.tap(find.byKey(const Key('scenario-edit')));
      await until(find.byType(ScenarioEditor));
      await t.enterText(find.byKey(const Key('scenario-value-0')), '999');
      await t.tap(find.text('Cancel'));
      await until(find.byType(ScenarioEditor), exists: false);
      expect(
        model
            .workspace['sheets'][0]['scenarios']['items'][1]['inputs'][0]['value'],
        '15',
      );
      final beforeShow = model.workspace;
      await t.runAsync(() async {
        await model.core.call({
          'op': 'save',
          'path': '${folder.path}/before-show.folio',
        });
      });
      model.rowMeasurer = (_) async =>
          throw StateError('Simulated scenario measurement failure');
      await t.tap(find.byKey(const Key('scenario-show')));
      await until(
        find.textContaining('Simulated scenario measurement failure'),
      );
      expect(model.workspace, beforeShow);
      model.rowMeasurer = null;
      await t.tap(find.byKey(const Key('scenario-show')));
      await value(450);
      expect(model.revision, (beforeShow['revision'] as int) + 1);
      if (capture) {
        await t.runAsync(() async {
          final image =
              await (boundary.currentContext!.findRenderObject()!
                      as RenderRepaintBoundary)
                  .toImage(pixelRatio: 1);
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          await File(
            '${folder.path}/scenarios.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
        });
      }
      await t.tap(find.text('Close'));
      await until(find.byType(ScenarioManager), exists: false);
      final applied = model.workspace;
      await drive(() => model.mutate({'op': 'undo'}));
      expect(model.workspace['sheets'], beforeShow['sheets']);
      await drive(() => model.mutate({'op': 'redo'}));
      expect(model.workspace['sheets'], applied['sheets']);
      await t.runAsync(() async {
        await model.core.call({
          'op': 'save',
          'path': '${folder.path}/applied.folio',
        });
      });
      await manager();
      await t.tap(find.byKey(const Key('scenario-summary')));
      await until(find.byType(ScenarioSummaryDialog));
      await t.enterText(find.byKey(const Key('scenario-result-cells')), 'C1');
      await t.tap(find.byKey(const Key('scenario-create-summary')));
      await until(find.byType(ScenarioManager), exists: false);
      expect(model.sheets.length, 3);
      expect(screen.sheetId, model.sheets.last['id']);
      final sid = model.sheets.last['id'];
      expect(model.snapshot['calculated'][sid]['B9'], 450);
      expect(model.snapshot['calculated'][sid]['C9'], 200);
      expect(model.snapshot['calculated'][sid]['D9'], 450);
      final finalSheets = model.workspace['sheets'];
      await t.runAsync(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/scenarios.xlsx',
        });
        final reopened = await model.core.call({
          'op': 'load',
          'path': model.recoveryPath,
        });
        expect(reopened['workspace']['sheets'], finalSheets);
        final imported = await model.core.call({
          'op': 'importOffice',
          'path': '${folder.path}/scenarios.xlsx',
        });
        final sheets = imported['workspace']['sheets'] as List;
        expect(sheets[0]['scenarios']['items'].map((s) => s['name']).toList(), [
          'Base',
          'Upside',
        ]);
        expect(sheets[0]['cells']['C1'], '=A1*B1');
        expect(imported['calculated'][sheets[2]['id']]['C9'], 200);
        expect(imported['calculated'][sheets[2]['id']]['D9'], 450);
        expect(sheets[0]['cellWatches'], {'C1': true});
        if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
          await File(
            '../../work/scenarios-artifact-path.txt',
          ).writeAsString(folder.path);
        }
      });
      expect(t.takeException(), isNull);
    },
  );
}
