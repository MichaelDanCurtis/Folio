import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/formula_inspector.dart';

void main() {
  testWidgets(
    'real workbook tracing commits drafts, navigates ranges, rejects stale results and survives export/recovery',
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
        final initial = await core.call({'op': 'get'});
        final w = initial['workspace'];
        w['sheets'] = [
          {
            'id': 'forecast',
            'name': 'Forecast',
            'cells': {
              'A1': 'Total',
              'C1': '=SUM(Sources!B2:B4)',
              'D1': '=INDIRECT("C1")',
            },
          },
          {
            'id': 'sources',
            'name': 'Sources',
            'cells': {
              'A1': 'Values',
              'B2': '10',
              'B3': '20',
              'B4': '30',
              'F1': '=Forecast!C1',
            },
          },
        ];
        await core.call({'op': 'replace', 'workspace': w});
        folder = await Directory.systemTemp.createTemp(
          'folio-formula-audit-ui-',
        );
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

      Future<void> until(Finder finder) async {
        await t.pump();
        final loading = find.descendant(
          of: find.byType(FormulaInspector),
          matching: find.byType(LinearProgressIndicator),
        );
        for (
          var i = 0;
          i < 400 &&
              (finder.evaluate().isEmpty || loading.evaluate().isNotEmpty);
          i++
        ) {
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(finder, findsOneWidget);
        expect(loading, findsNothing);
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
      final before = model.workspace;
      Future<void> open() async {
        await t.tap(find.byKey(const Key('cell-actions')));
        await t.pumpAndSettle();
        await t.tap(find.text('Formula inspector…'));
        await until(find.byKey(const Key('audit-formula')));
      }

      await open();
      expect(find.text('Value: 60'), findsOneWidget);
      expect(find.text('Sources!B2:B4'), findsOneWidget);
      expect(find.textContaining('Unresolved formulas: 1.'), findsOneWidget);
      if (capture) {
        await t.runAsync(() async {
          final bytes =
              await (boundary.currentContext!.findRenderObject()!
                      as RenderRepaintBoundary)
                  .toImage(pixelRatio: 1);
          final png = await bytes.toByteData(format: ui.ImageByteFormat.png);
          bytes.dispose();
          await File(
            '${folder.path}/formula-inspector.png',
          ).writeAsBytes(png!.buffer.asUint8List());
        });
      }
      await t.tap(find.byKey(const Key('goto-precedents-sources-B2:B4')));
      await t.pumpAndSettle();
      await drive(screen.flush);
      expect(screen.sheetId, 'sources');
      expect(screen.sheetKey.currentState!.selectedRange, 'B2:B4');
      expect(model.workspace, before);
      await drive(() async {
        await screen.sheetKey.currentState!.select(2, 1, extend: false);
      });
      screen.sheetKey.currentState!.edit();
      await t.pumpAndSettle();
      await t.enterText(find.byKey(const Key('cell-editor')), '25');
      await open();
      expect(model.sheets[1]['cells']['B3'], '25');
      expect(find.text('Value: 25'), findsOneWidget);
      await t.tap(find.text('Dependents (1)'));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('inspect-dependents-forecast-C1')));
      await until(find.text('Forecast!C1'));
      await until(find.text('Value: 65'));
      await t.tap(find.byTooltip('Back to previous cell'));
      await until(find.text('Sources!B3'));
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 'sources', 'address': 'B2', 'value': '12'},
          ],
        }),
      );
      expect(find.byKey(const Key('audit-stale')), findsOneWidget);
      expect(
        t
            .widget<TextButton>(
              find.widgetWithText(TextButton, 'Go to this cell'),
            )
            .onPressed,
        isNull,
      );
      await t.tap(find.widgetWithText(TextButton, 'Close'));
      await t.pumpAndSettle();
      await drive(() => screen.history(false));
      await drive(() => screen.history(false));
      expect(model.sheets[1]['cells']['B3'], '20');
      late Map audit;
      await t.runAsync(() async {
        Future<Map> inspect(String id) => model.core.call({
          'op': 'formulaAudit',
          'workspaceId': model.workspace['id'],
          'baseRevision': model.revision,
          'calculationGeneration': model.snapshot['evaluatedCells'],
          'sheetId': id,
          'address': 'C1',
        });
        audit = await inspect('forecast');
        expect(audit['value'], 60);
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/audit.xlsx',
        });
        final loaded = await model.core.call({
          'op': 'load',
          'path': model.recoveryPath,
        });
        expect(loaded['workspace']['sheets'], model.workspace['sheets']);
        final restored = await model.core.call({
          'op': 'formulaAudit',
          'workspaceId': loaded['workspace']['id'],
          'baseRevision': loaded['workspace']['revision'],
          'calculationGeneration': loaded['evaluatedCells'],
          'sheetId': 'forecast',
          'address': 'C1',
        });
        expect(restored['precedents'], audit['precedents']);
        expect(restored['value'], 60);
        final imported = await model.core.call({
          'op': 'importOffice',
          'path': '${folder.path}/audit.xlsx',
        });
        final importedSheets = imported['workspace']['sheets'] as List;
        final forecast = importedSheets.firstWhere(
          (s) => s['name'] == 'Forecast',
        );
        final traced = await model.core.call({
          'op': 'formulaAudit',
          'workspaceId': imported['workspace']['id'],
          'baseRevision': imported['workspace']['revision'],
          'calculationGeneration': imported['evaluatedCells'],
          'sheetId': forecast['id'],
          'address': 'C1',
        });
        expect(traced['value'], 60);
        expect(traced['precedents'].single['range'], 'B2:B4');
        expect(traced['precedents'].single['sheetName'], 'Sources');
        // Preserve the fixture and optional rendered result for independent review.
        await File(
          '${Directory.current.path}/../../work/formula-audit-artifact-path.txt',
        ).writeAsString(folder.path);
      });
      expect(find.byType(FormulaInspector), findsNothing);
      expect(t.takeException(), isNull);
    },
  );
}
