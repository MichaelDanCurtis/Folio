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

void main() {
  testWidgets(
    'fill menu and shortcuts keep selection, flush drafts, recalculate, undo and persist',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final capture = Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1';
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        if (capture) {
          for (final (family, path) in [
            ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
            ('Aptos', '/System/Library/Fonts/Supplemental/Arial.ttf'),
            ('Georgia', '/System/Library/Fonts/Supplemental/Georgia.ttf'),
            ('Menlo', '/System/Library/Fonts/Menlo.ttc'),
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
        }
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-fill-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'cells',
          'cells': [
            for (final (a, v) in [
              ('A1', 'Units'),
              ('B1', 'Price'),
              ('C1', 'Total'),
              ('A2', '2'),
              ('A3', '3'),
              ('A4', '4'),
              ('B2', '5'),
              ('B3', '6'),
              ('B4', '7'),
              ('C2', '=A2*B2'),
            ])
              {'sheetId': 'sheet-1', 'address': a, 'value': v},
          ],
        });
        await core.call({
          'op': 'formatCells',
          'sheetId': 'sheet-1',
          'addresses': ['C2'],
          'style': {'bold': true, 'numFmt': r'$#,##0.00'},
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        await model.initialize();
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
        for (var i = 0; i < 500 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (!done) throw StateError('Native operation did not finish');
        if (error != null) throw error!;
        return value as T;
      }

      Future<void> settle() async {
        await tester.pump(const Duration(milliseconds: 300));
        await tester.pumpAndSettle();
      }

      Future<void> revision(int wanted) async {
        for (var i = 0; i < 300 && model.revision < wanted; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(model.revision, wanted);
        await settle();
      }

      await tester.pumpWidget(
        RepaintBoundary(
          key: const Key('fill-preview-frame'),
          child: FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
        ),
      );
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox());
        model.dispose();
        await tester.runAsync(() => folder.delete(recursive: true));
      });
      await settle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.switchMode(true));
      await settle();
      final grid = screen.sheetKey.currentState!;
      Future<void> selectRange(String range) async {
        await tester.enterText(find.byKey(ValueKey(grid.selectedRange)), range);
        await tester.testTextInput.receiveAction(TextInputAction.done);
        await drive(grid.flush);
        await settle();
        expect(grid.selectedRange, range);
      }

      Future<void> shortcut(
        LogicalKeyboardKey key, {
        bool command = false,
      }) async {
        grid.gridFocus.requestFocus();
        await settle();
        await tester.sendKeyDownEvent(
          command
              ? LogicalKeyboardKey.metaLeft
              : LogicalKeyboardKey.controlLeft,
        );
        await tester.sendKeyEvent(key);
        await tester.sendKeyUpEvent(
          command
              ? LogicalKeyboardKey.metaLeft
              : LogicalKeyboardKey.controlLeft,
        );
      }

      Future<void> menu(String label) async {
        await tester.tap(find.byKey(const Key('fill-cells-menu')));
        await settle();
        expect(find.text('Fill up'), findsOneWidget);
        await tester.tap(find.text(label));
      }

      await selectRange('C2:C4');
      await tester.enterText(find.byKey(const Key('formula')), '=A2*B2+1');
      var before = model.revision;
      await tester.tap(find.byKey(const Key('fill-cells-menu')));
      await settle();
      if (capture) {
        await drive(grid.commitFormula);
        await drive(screen.flush);
        await settle();
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('fill-preview-frame')),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          final file = File(
            '${Directory.current.path}/release/previews/fill-cells.png',
          );
          await file.parent.create(recursive: true);
          await file.writeAsBytes(bytes!.buffer.asUint8List());
        });
      }
      await tester.tap(find.text('Fill down  ·  ⌘D'));
      await revision(before + 2);
      expect(grid.selectedRange, 'C2:C4');
      expect(model.sheets[0]['cells']['C4'], '=A4*B4+1');
      expect(model.snapshot['calculated']['sheet-1']['C4'], 29);
      expect(model.sheets[0]['cellStyles']['C4']['bold'], true);
      expect(grid.gridFocus.hasPrimaryFocus, isTrue);
      await drive(() => screen.history(false));
      expect(model.sheets[0]['cells']['C4'], isNull);
      expect(model.sheets[0]['cells']['C2'], '=A2*B2+1');
      await drive(() => screen.history(false));
      expect(model.sheets[0]['cells']['C2'], '=A2*B2');
      await drive(() => screen.history(true));
      await drive(() => screen.history(true));
      before = model.revision;
      await shortcut(LogicalKeyboardKey.keyD);
      await revision(before + 1);
      before = model.revision;
      await shortcut(LogicalKeyboardKey.keyD, command: true);
      await revision(before + 1);
      expect(model.sheets[0]['cells']['C4'], '=A4*B4+1');
      await selectRange('A6:C6');
      await tester.enterText(find.byKey(const Key('formula')), '9');
      await drive(grid.commitFormula);
      before = model.revision;
      await shortcut(LogicalKeyboardKey.keyR);
      await revision(before + 1);
      expect(model.sheets[0]['cells']['B6'], '9');
      expect(model.sheets[0]['cells']['C6'], '9');
      before = model.revision;
      await shortcut(LogicalKeyboardKey.keyR, command: true);
      await revision(before + 1);
      expect(model.sheets[0]['cells']['C6'], '9');
      await selectRange('A5:A6');
      before = model.revision;
      await menu('Fill up');
      await revision(before + 1);
      expect(model.sheets[0]['cells']['A5'], '9');
      await selectRange('A1:C1');
      before = model.revision;
      await menu('Fill left');
      await revision(before + 1);
      expect(model.sheets[0]['cells']['A1'], 'Total');
      await drive(() => screen.history(false));
      // Focused fields keep their own control shortcuts; they never fill cells.
      await tester.enterText(find.byKey(const Key('formula')), 'draft');
      before = model.revision;
      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyD);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await settle();
      expect(model.revision, before);
      await tester.enterText(find.byKey(const Key('formula')), 'Units');
      await drive(grid.commitFormula);
      // A rejected pending value must not fill the old value across a selection.
      await selectRange('A2:A4');
      await tester.enterText(find.byKey(const Key('formula')), 'x' * 10001);
      before = model.revision;
      await menu('Fill down  ·  ⌘D');
      await drive(() => grid.fillSelection('down'));
      await settle();
      expect(model.revision, before);
      expect(model.sheets[0]['cells']['A3'], '3');
      expect(grid.formulaDirty, isTrue);
      await tester.enterText(find.byKey(const Key('formula')), '2');
      await drive(grid.commitFormula);
      await drive(screen.flush);
      final saved = await tester.runAsync(
        () => File('${folder.path}/workspace.folio').readAsString(),
      );
      expect(jsonDecode(saved!)['sheets'][0]['cells']['C4'], '=A4*B4+1');
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/filled.xlsx',
        }),
      );
      expect(
        await tester.runAsync(
          () => File('${folder.path}/filled.xlsx').exists(),
        ),
        isTrue,
      );
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );
}
