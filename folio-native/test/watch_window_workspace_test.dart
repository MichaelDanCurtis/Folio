import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/watch_window.dart';

void main() {
  testWidgets(
    'Watch Window stays live across sheets, history, structural edits and XLSX recovery',
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
            ('monospace', '/System/Library/Fonts/SFNSMono.ttf'),
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
        w['title'] = 'Watched budget';
        w['sheets'] = [
          {
            'id': 'forecast',
            'name': 'Forecast',
            'cells': {'A1': 'Total', 'C1': '=SUM(Sources!B2:B4)'},
          },
          {
            'id': 'sources',
            'name': 'Sources',
            'cells': {'A1': 'Costs', 'B2': '10', 'B3': '20', 'B4': '30'},
          },
        ];
        await core.call({'op': 'replace', 'workspace': w});
        folder = await Directory.systemTemp.createTemp('folio-watch-ui-');
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
        for (var i = 0; i < 400 && !done; i++) {
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
        await t.pump();
        for (
          var i = 0;
          i < 400 &&
              (f.evaluate().isNotEmpty != exists ||
                  find.byType(LinearProgressIndicator).evaluate().isNotEmpty);
          i++
        ) {
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
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
      await t.tap(find.text('Formulas'));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('watch-toggle')));
      await t.pumpAndSettle();
      expect(find.text('Watch Window (0)'), findsOneWidget);
      Future<void> add(String range, {String? sheet}) async {
        await t.tap(find.byKey(const Key('watch-add')));
        await until(find.byKey(const Key('watch-add-dialog')));
        if (sheet != null) {
          await t.tap(find.byKey(const Key('watch-sheet')));
          await t.pumpAndSettle();
          await t.tap(find.text(sheet).last);
          await t.pumpAndSettle();
        }
        await t.enterText(find.byKey(const Key('watch-range')), range);
        await t.tap(find.byKey(const Key('watch-add-confirm')));
        await until(find.byKey(const Key('watch-add-dialog')), exists: false);
      }

      await add('C1');
      await add('B2:B4', sheet: 'Sources');
      expect(find.text('Watch Window (4)'), findsOneWidget);
      final watched = model.workspace;
      final generation = model.snapshot['evaluatedCells'];
      await add('B3', sheet: 'Sources');
      expect(model.workspace, watched);
      expect(model.snapshot['evaluatedCells'], generation);
      await t.tap(find.byKey(const Key('watch-open-sources-B3')));
      await drive(screen.flush);
      expect(screen.sheetId, 'sources');
      expect(screen.sheetKey.currentState!.selected, 'B3');
      screen.sheetKey.currentState!.edit();
      await t.pumpAndSettle();
      await t.enterText(find.byKey(const Key('cell-editor')), '25');
      await drive(() => screen.sheetKey.currentState!.finishEditing(1, 0));
      final total = find.byKey(const Key('watch-row-forecast-C1'));
      expect(
        find.descendant(of: total, matching: find.text('65')),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: find.byKey(const Key('watch-row-sources-B3')),
          matching: find.text('25'),
        ),
        findsOneWidget,
      );
      if (capture) {
        await t.runAsync(() async {
          final image =
              await (boundary.currentContext!.findRenderObject()!
                      as RenderRepaintBoundary)
                  .toImage(pixelRatio: 1);
          final png = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          await File(
            '${folder.path}/watch-window.png',
          ).writeAsBytes(png!.buffer.asUint8List());
        });
      }
      await t.tap(find.byKey(const Key('watch-inspect-forecast-C1')));
      await until(find.byKey(const Key('audit-formula')));
      expect(find.text('Value: 65'), findsOneWidget);
      await t.tap(find.widgetWithText(TextButton, 'Close'));
      await t.pumpAndSettle();
      for (final a in ['B2', 'B4']) {
        await t.tap(find.byKey(Key('watch-check-sources-$a')));
        await t.pumpAndSettle();
      }
      await t.tap(find.byKey(const Key('watch-remove')));
      await until(find.text('Watch Window (2)'));
      await drive(() => screen.history(false));
      expect(find.text('Watch Window (4)'), findsOneWidget);
      await drive(() => screen.history(true));
      expect(find.text('Watch Window (2)'), findsOneWidget);
      await t.tap(find.byKey(const Key('watch-open-sources-B3')));
      await drive(screen.flush);
      await drive(
        () => screen.changeSheetStructure({
          'action': 'insertRows',
          'at': 2,
          'count': 1,
        }),
      );
      expect(find.byKey(const Key('watch-row-sources-B4')), findsOneWidget);
      expect(model.sheets[0]['cells']['C1'], '=SUM(Sources!B3:B5)');
      await drive(
        () => screen.changeSheetStructure({
          'action': 'deleteRows',
          'at': 4,
          'count': 1,
        }),
      );
      expect(find.text('Watch Window (1)'), findsOneWidget);
      await drive(() => screen.history(false));
      expect(find.text('Watch Window (2)'), findsOneWidget);
      await t.runAsync(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/watches.xlsx',
        });
        final saved = model.workspace;
        final loaded = await model.core.call({
          'op': 'load',
          'path': model.recoveryPath,
        });
        expect(loaded['workspace']['sheets'], saved['sheets']);
        final imported = await model.core.call({
          'op': 'importOffice',
          'path': '${folder.path}/watches.xlsx',
        });
        final sheets = imported['workspace']['sheets'] as List;
        expect(
          sheets.firstWhere((s) => s['name'] == 'Forecast')['cellWatches'],
          {'C1': true},
        );
        expect(
          sheets.firstWhere((s) => s['name'] == 'Sources')['cellWatches'],
          {'B4': true},
        );
        final sid = sheets.firstWhere((s) => s['name'] == 'Forecast')['id'];
        expect(imported['calculated'][sid]['C1'], 65);
        await File(
          '../../work/watch-window-artifact-path.txt',
        ).writeAsString(folder.path);
      });
      expect(find.byType(WatchWindow), findsOneWidget);
      expect(t.takeException(), isNull);
    },
  );
}
