import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_font.dart';

void main() {
  testWidgets(
    'AutoFit menu and row-boundary gestures commit drafts, preserve history and exported geometry',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-autofit-ui-tests',
      );
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        for (final (family, path) in [
          ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
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
        await CoreBridge.initialize();
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/autofit.xlsx',
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
        for (var i = 0; i < 500 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (!done) throw StateError('Native operation timed out');
        if (error != null) throw error!;
        return value as T;
      }

      Future<void> settle() async {
        await tester.pump(const Duration(milliseconds: 300));
        await tester.pumpAndSettle();
      }

      Future<void> revision(int expected) async {
        for (var i = 0; i < 300 && model.revision < expected; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(model.revision, expected);
        await settle();
      }

      Future<void> export(String name) async {
        await drive(
          () => model.core.call({
            'op': 'exportOffice',
            'format': 'xlsx',
            'path': '${folder.path}/$name.xlsx',
          }),
        );
      }

      await tester.pumpWidget(
        RepaintBoundary(
          key: const Key('autofit-preview'),
          child: FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
        ),
      );
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox());
        model.dispose();
      });
      await settle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.switchMode(true));
      await settle();
      final grid = screen.sheetKey.currentState!;
      Future<void> select(String range) async {
        await tester.enterText(find.byKey(ValueKey(grid.selectedRange)), range);
        await tester.testTextInput.receiveAction(TextInputAction.done);
        await drive(grid.flush);
        await settle();
      }

      await select('A1:C6');
      final before = model.revision;
      await tester.tap(find.byKey(const Key('structure-menu')));
      await settle();
      await tester.tap(find.text('AutoFit Row Height'));
      await revision(before + 1);
      final heights = Map<String, dynamic>.from(model.sheets[0]['rowHeights']);
      expect(heights['6'], 75);
      expect(
        heights['2'],
        greaterThan(45),
      ); // Z2 is offscreen but has four lines.
      expect(heights['3'], greaterThan(35));
      for (final address in ['B2', 'B3', 'B4']) {
        final box = tester.renderObject<SheetFontRenderBox>(
          find.byKey(Key('sheet-font-$address')),
        );
        expect(box.textBounds.top, greaterThanOrEqualTo(-.01));
        expect(box.textBounds.bottom, lessThanOrEqualTo(box.size.height + .01));
      }
      await export('fitted');
      await tester.runAsync(
        () => File(
          '${folder.path}/heights.json',
        ).writeAsString(jsonEncode(heights)),
      );
      if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
        await select('A2');
        await settle();
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('autofit-preview')),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          await File(
            '${Directory.current.path}/release/previews/row-autofit.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
        });
      }
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      await settle();
      await export('undone');
      expect(model.sheets[0]['rowHeights']['2'], 18);
      await drive(() => model.mutate({'op': 'redo'}, refreshDocument: true));
      await settle();
      await export('redone');
      await drive(model.flush);
      final recovery = CoreBridge();
      await drive(
        () => recovery.call({'op': 'load', 'path': model.recoveryPath}),
      );
      await drive(
        () => recovery.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/reopened.xlsx',
        }),
      );
      // Restore the live model after the shared bridge load and verify the real gesture.
      await drive(() => model.mutate({'op': 'get'}, refreshDocument: true));
      await select('A3');
      await drive(() => grid.geometry('rowHeight', size: 18));
      await settle();
      final shortRevision = model.revision;
      await tester.tap(find.byKey(const Key('row-resize-2')));
      await tester.pump(const Duration(milliseconds: 80));
      await tester.tap(find.byKey(const Key('row-resize-2')));
      await revision(shortRevision + 1);
      expect(model.sheets[0]['rowHeights']['3'], heights['3']);
      await select('A4');
      grid.edit();
      await settle();
      final pending = List.filled(30, 'Draft text').join(' ');
      await tester.enterText(find.byKey(const Key('cell-editor')), pending);
      final pendingRevision = model.revision;
      await drive(grid.autoFitRows);
      await settle();
      // This row is already automatic: committing the draft fits it in the
      // same edit, so the subsequent identical AutoFit adds no empty history.
      expect(model.revision, pendingRevision + 1);
      expect(model.sheets[0]['cells']['A4'], pending);
      expect(model.sheets[0]['rowHeights']['4'], greaterThan(heights['4']));
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      await settle();
      expect(model.sheets[0]['rowHeights']['4'], heights['4']);
      expect(model.sheets[0]['cells']['A4'], 'Prototype');
      await drive(model.flush);
      expect(tester.takeException(), isNull);
    },
  );
}
