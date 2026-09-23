import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_autofit.dart';

void main() {
  testWidgets(
    'automatic rows grow and shrink atomically with edits, width, fonts and formulas',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-row-reflow-ui-tests',
      );
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        for (final (family, path) in [
          ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
          // Match native system-font fallback instead of the test-only Ahem font.
          ('Calibri', '/System/Library/Fonts/SFNS.ttf'),
          ('Aptos', '/System/Library/Fonts/SFNS.ttf'),
          ('Menlo', '/System/Library/Fonts/Menlo.ttc'),
          ('Georgia', '/System/Library/Fonts/Supplemental/Georgia.ttf'),
          (
            'MaterialIcons',
            '/opt/homebrew/share/flutter/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
          ),
        ]) {
          await (FontLoader(family)..addFont(
                Future.value(
                  ByteData.sublistView(await File(path).readAsBytes()),
                ),
              ))
              .load();
        }
        await CoreBridge.initialize();
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/row-reflow.xlsx',
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        model.snapshot = await core.call({'op': 'get'});
        await model.flush();
      });
      Future<T> drive<T>(Future<T> Function() job) async {
        var done = false;
        T? value;
        Object? error;
        unawaited(
          Future.sync(job).then(
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
        for (var i = 0; i < 600 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (error != null) throw error!;
        expect(done, isTrue);
        await tester.pumpAndSettle();
        return value as T;
      }

      Future<void> revision(int expected) async {
        for (var i = 0; i < 400 && model.revision < expected; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(model.revision, expected);
        await tester.pumpAndSettle();
      }

      Future<void> export(String name) => drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/$name.xlsx',
        });
      });
      Future<void> history({bool redo = false}) async {
        final before = model.revision;
        await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
        if (redo) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
        if (redo) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
        await revision(before + 1);
      }

      await tester.pumpWidget(
        RepaintBoundary(
          key: const Key('row-reflow-preview'),
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
      final grid = screen.sheetKey.currentState!;
      Future<void> edit(int row, String text) async {
        await drive(() => grid.select(row - 1, 1));
        await tester.sendKeyEvent(LogicalKeyboardKey.f2);
        await tester.pumpAndSettle();
        await tester.enterText(find.byKey(const Key('cell-editor')), text);
        final rev = model.revision;
        await tester.sendKeyEvent(LogicalKeyboardKey.enter);
        await revision(rev + 1);
      }

      num height(int row) => model.sheets[0]['rowHeights']?['$row'] ?? 15;
      Future<void> cells(String sheet, String address, String value) => drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': sheet, 'address': address, 'value': value},
          ],
        }),
      );
      const lines = 'First line\nSecond line\nThird line';
      final before = jsonEncode(model.sheets);
      await edit(2, lines);
      expect(height(2), greaterThan(40));
      expect(model.sheets[0]['rowHeightModes']['2'], 'auto');
      final grown = height(2);
      await export('edited');
      await history();
      expect(jsonEncode(model.sheets), before);
      await export('undone');
      await history(redo: true);
      expect(height(2), grown);
      await export('redone');

      await edit(3, lines);
      expect(height(3), 18);
      await drive(() => grid.select(2, 1));
      await drive(grid.autoFitRows);
      expect(height(3), greaterThan(40));
      expect(model.sheets[0]['rowHeightModes']['3'], 'auto');
      await edit(3, 'Short');
      expect(height(3), lessThan(grown));
      await drive(
        () => model.mutate({
          'op': 'sheetGeometry',
          'axis': 'rows',
          'action': 'resize',
          'sheetId': 'office-1',
          'at': 3,
          'count': 1,
          'size': 18,
        }),
      );
      await edit(3, lines);
      expect(height(3), 18);
      expect(model.sheets[0]['rowHeightModes']['3'], 'manual');

      const wrapped =
          'A longer sentence that changes height when its column becomes narrower.';
      await edit(2, wrapped);
      final wide = height(2);
      final beforeWidth = jsonEncode(model.sheets);
      await drive(
        () => model.mutate({
          'op': 'sheetGeometry',
          'axis': 'columns',
          'action': 'resize',
          'sheetId': 'office-1',
          'at': 2,
          'count': 1,
          'size': 12,
        }),
      );
      expect(height(2), greaterThan(wide));
      expect(height(200), greaterThan(15));
      expect(height(1048576), greaterThan(15));
      expect(height(5), 22);
      expect(height(8), 20);
      expect(height(3), 18);
      expect(height(10), lessThan(35));
      await export('narrow');
      await history();
      expect(jsonEncode(model.sheets), beforeWidth);
      await history(redo: true);
      final beforeFont = jsonEncode(model.sheets);
      final small = height(2);
      await drive(
        () => model.mutate({
          'op': 'formatCells',
          'sheetId': 'office-1',
          'addresses': ['B2'],
          'style': {'fontSize': 22},
        }),
      );
      expect(height(2), greaterThan(small));
      await history();
      expect(jsonEncode(model.sheets), beforeFont);

      final beforeFormula = jsonEncode(model.sheets);
      await cells(
        'office-2',
        'A1',
        'Formula first\nFormula second\nFormula third',
      );
      expect(
        model.snapshot['calculated']['office-1']['B7'],
        'Formula first\nFormula second\nFormula third',
      );
      expect(height(7), greaterThan(40));
      expect(model.sheets[1]['rowHeights']['1'], greaterThan(30));
      await export('formula');
      await history();
      expect(jsonEncode(model.sheets), beforeFormula);
      await history(redo: true);

      // The normal app-level Apply path must include both document and workbook
      // edits in the same history record, with row fitting derived from the result.
      final beforeAi = jsonEncode(model.workspace);
      await drive(
        () => model.mutate({
          'op': 'apply',
          'proposal': {
            'summary': 'Append summary and edit source',
            'baseRevision': model.revision,
            'appendParagraphs': ['Reviewed source.'],
            'cells': [
              {
                'sheetId': 'office-1',
                'address': 'B2',
                'value': 'AI one\nAI two\nAI three\nAI four',
              },
            ],
          },
        }, refreshDocument: true),
      );
      expect(height(2), greaterThan(40));
      await history();
      final undone = Map<String, dynamic>.from(model.workspace)
        ..['revision'] = jsonDecode(beforeAi)['revision'];
      expect(jsonEncode(undone), beforeAi);

      // Force measurement failure: text, calculated values, geometry and history
      // must all stay at the previous committed snapshot.
      final beforeFailure = jsonEncode(model.snapshot);
      model.rowMeasurer = (_) async =>
          throw StateError('Injected layout failure');
      await drive(() async {
        await expectLater(
          model.mutate({
            'op': 'cells',
            'cells': [
              {'sheetId': 'office-1', 'address': 'B2', 'value': 'Uncommitted'},
            ],
          }),
          throwsStateError,
        );
      });
      expect(jsonEncode(model.snapshot), beforeFailure);
      final stillCurrent = await drive(() => model.core.call({'op': 'get'}));
      expect(jsonEncode(stillCurrent), beforeFailure);
      model.rowMeasurer = null;
      await drive(model.flush);
      await export('final');
      final saved = jsonEncode(model.workspace);
      await drive(
        () => model.mutate({
          'op': 'load',
          'path': model.recoveryPath,
        }, refreshDocument: true),
      );
      expect(jsonEncode(model.workspace), saved);
      await export('reopened');
      await drive(
        () => model.mutate({
          'op': 'importOffice',
          'path': '${folder.path}/final.xlsx',
        }, refreshDocument: true),
      );
      expect(model.sheets[0]['rowHeightModes']['2'], 'auto');
      expect(model.sheets[0]['rowHeightModes']['3'], 'manual');
      await export('reimported');
      await edit(2, 'Small again');
      expect(height(2), lessThan(small));

      // A save and subsequent edit queued while glyph layout yields cannot observe
      // the prepared workbook or overwrite it with stale model state.
      final gate = Completer<void>(), started = Completer<void>();
      model.rowMeasurer = (p) async {
        started.complete();
        await gate.future;
        return measureAutomaticRows(p);
      };
      final revisionBeforeQueue = model.revision;
      int? revisionDuringLayout;
      await drive(() async {
        final change = model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 'office-1', 'address': 'B2', 'value': lines},
          ],
        });
        await started.future;
        revisionDuringLayout = model.revision;
        final save = model.flush();
        final title = model.mutate({
          'op': 'title',
          'title': 'Queued edit preserved',
        });
        gate.complete();
        await Future.wait([change, save, title]);
      });
      model.rowMeasurer = null;
      expect(revisionDuringLayout, revisionBeforeQueue);
      expect(model.revision, revisionBeforeQueue + 2);
      expect(model.workspace['title'], 'Queued edit preserved');
      expect(model.sheets[0]['cells']['B2'], lines);
      expect(height(2), greaterThan(40));
      await drive(model.flush);
      await export('queued');

      if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
        await drive(() => grid.select(1, 1));
        await tester.runAsync(() async {
          final boundary = tester.renderObject<RenderRepaintBoundary>(
            find.byKey(const Key('row-reflow-preview')),
          );
          final image = await boundary.toImage(pixelRatio: 1);
          final bytes = (await image.toByteData(
            format: ui.ImageByteFormat.png,
          ))!;
          await File(
            '${Directory.current.path}/release/previews/automatic-rows.png',
          ).writeAsBytes(bytes.buffer.asUint8List());
          image.dispose();
        });
      }
      expect(tester.takeException(), isNull);
    },
  );
}
