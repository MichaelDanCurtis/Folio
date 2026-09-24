import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'Goal Seek preview, cancel, measured Apply retry, history and XLSX recovery use the real engine',
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
        w['title'] = 'Goal Seek sample';
        w['sheets'] = [
          {
            'id': 'summary',
            'name': 'Summary',
            'cells': {'A1': 'Total', 'C1': '=Inputs!B2*Inputs!B3'},
            'cellWatches': {'C1': true},
          },
          {
            'id': 'inputs',
            'name': 'Inputs',
            'cells': {
              'A2': 'Units',
              'B2': '10',
              'A3': 'Unit price',
              'B3': '20',
            },
            'cellWatches': {'B3': true},
          },
        ];
        await core.call({'op': 'replace', 'workspace': w});
        folder = await Directory.systemTemp.createTemp('folio-goal-seek-ui-');
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
        await screen.sheetKey.currentState!.select(0, 2, extend: false);
      });
      await t.tap(find.text('Data'));
      await t.pumpAndSettle();
      Future<void> seek() async {
        await t.tap(find.byKey(const Key('goal-seek-button')));
        await until(find.byKey(const Key('goal-seek-dialog')));
        await t.enterText(find.byKey(const Key('goal-target-value')), '500');
        await t.tap(find.byKey(const Key('goal-changing-sheet')));
        await t.pumpAndSettle();
        await t.tap(find.text('Inputs').last);
        await t.pumpAndSettle();
        await t.enterText(find.byKey(const Key('goal-changing-cell')), 'B3');
        await t.tap(find.byKey(const Key('goal-run')));
        await until(find.byKey(const Key('goal-apply')));
        expect(find.text('A matching input value was found.'), findsOneWidget);
      }

      await drive(screen.flush);
      final before = model.workspace;
      final generation = model.snapshot['evaluatedCells'];
      await t.runAsync(() async {
        await model.core.call({
          'op': 'save',
          'path': '${folder.path}/before.folio',
        });
      });
      await seek();
      expect(model.workspace, before);
      expect(model.snapshot['evaluatedCells'], generation);
      expect(model.snapshot['calculated']['summary']['C1'], 200);
      await t.tap(find.byKey(const Key('goal-cancel')));
      await until(find.byKey(const Key('goal-seek-dialog')), exists: false);
      expect(model.workspace, before);
      await seek();
      if (capture) {
        await t.runAsync(() async {
          final image =
              await (boundary.currentContext!.findRenderObject()!
                      as RenderRepaintBoundary)
                  .toImage(pixelRatio: 1);
          final png = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          await File(
            '${folder.path}/goal-seek.png',
          ).writeAsBytes(png!.buffer.asUint8List());
        });
      }
      model.rowMeasurer = (_) async =>
          throw StateError('Simulated measurement failure');
      await t.tap(find.byKey(const Key('goal-apply')));
      await until(find.byKey(const Key('goal-error')));
      expect(model.workspace, before);
      expect(model.snapshot['evaluatedCells'], generation);
      model.rowMeasurer = null;
      await t.tap(find.byKey(const Key('goal-apply')));
      await until(find.byKey(const Key('goal-seek-dialog')), exists: false);
      expect(
        (model.workspace['sheets'][1]['cells']['B3'] as String),
        isNotEmpty,
      );
      expect(
        double.parse(model.workspace['sheets'][1]['cells']['B3']),
        closeTo(50, 1e-7),
      );
      expect(model.snapshot['calculated']['summary']['C1'], closeTo(500, 1e-6));
      final applied = model.workspace;
      await drive(() => model.mutate({'op': 'undo'}));
      expect(model.workspace['sheets'], before['sheets']);
      await drive(() => model.mutate({'op': 'redo'}));
      expect(model.workspace['sheets'], applied['sheets']);
      await t.runAsync(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/goal-seek.xlsx',
        });
        final loaded = await model.core.call({
          'op': 'load',
          'path': model.recoveryPath,
        });
        expect(loaded['workspace']['sheets'], applied['sheets']);
        final imported = await model.core.call({
          'op': 'importOffice',
          'path': '${folder.path}/goal-seek.xlsx',
        });
        final sheets = imported['workspace']['sheets'] as List;
        final summary = sheets.firstWhere((s) => s['name'] == 'Summary'),
            inputs = sheets.firstWhere((s) => s['name'] == 'Inputs');
        expect(summary['cells']['C1'], '=Inputs!B2*Inputs!B3');
        expect(inputs['cells']['B2'], '10');
        expect(double.parse(inputs['cells']['B3']), closeTo(50, 1e-7));
        expect(summary['cellWatches'], {'C1': true});
        expect(inputs['cellWatches'], {'B3': true});
        expect(imported['calculated'][summary['id']]['C1'], closeTo(500, 1e-6));
        if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
          await File(
            '../../work/goal-seek-artifact-path.txt',
          ).writeAsString(folder.path);
        }
      });
      expect(t.takeException(), isNull);
    },
  );
}
