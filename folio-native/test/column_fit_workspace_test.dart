import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_font.dart';

void main() {
  testWidgets(
    'Column AutoFit, manual character widths and boundary gestures preserve workbook history',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-column-fit-ui-tests',
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
          'path': '${Directory.current.path}/test/fixtures/column-autofit.xlsx',
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
          key: const Key('column-fit-preview'),
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

      await select('A1:D1');
      final before = model.revision;
      final originalRows = jsonEncode(model.sheets[0]['rowHeights']);
      expect(
        model.sheets[0]['styleDefaults']['normalFont']['fontName'],
        'Georgia',
      );
      expect(model.sheets[0]['styleDefaults']['normalFont']['fontSize'], 14);
      await tester.tap(find.byKey(const Key('structure-menu')));
      await settle();
      await tester.tap(find.text('AutoFit Column Width'));
      await revision(before + 1);
      final widths = Map<String, dynamic>.from(model.sheets[0]['columnWidths']);
      expect(widths['4'], 17);
      for (final e in (jsonDecode(originalRows) as Map).entries) {
        expect(model.sheets[0]['rowHeights'][e.key], e.value);
      }
      expect(model.sheets[0]['rowHeightModes']['200'], 'auto');
      expect(model.sheets[0]['rowHeights']['11'], greaterThan(18));
      expect(model.sheets[0]['hiddenColumns'], [4]);
      // The distant C200 value participates even though it has never been drawn.
      final font = cellStyle(model.sheets[0], 'C200');
      final measure = SheetFontRenderBox(
        sheetFontRuns('Offscreen reviewer', font),
        Colors.black,
        TextDirection.ltr,
        TextScaler.noScaling,
        CellTextLayout(font),
      );
      expect(
        grid.columnMetrics.pixels(widths['3']),
        greaterThanOrEqualTo(measure.autoFitWidth() + 22),
      );
      measure.dispose();
      for (final address in ['A1', 'B1', 'C1', 'B2']) {
        final box = tester.renderObject<SheetFontRenderBox>(
          find.byKey(Key('sheet-font-$address')),
        );
        expect(box.textBounds.width, lessThanOrEqualTo(box.size.width + .01));
      }
      await export('fitted');
      await tester.runAsync(
        () => File(
          '${folder.path}/widths.json',
        ).writeAsString(jsonEncode(widths)),
      );
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      await settle();
      await export('undone');
      await drive(() => model.mutate({'op': 'redo'}, refreshDocument: true));
      await settle();
      await export('redone');
      await drive(model.flush);
      await drive(
        () => model.mutate({
          'op': 'load',
          'path': model.recoveryPath,
        }, refreshDocument: true),
      );
      await settle();
      await export('reopened');
      await select('A1');
      final sizing = grid.geometry('columnWidth');
      await settle();
      await tester.enterText(find.byKey(const Key('dimension-size')), '20');
      await tester.tap(find.byKey(const Key('dimension-apply')));
      await drive(() => sizing);
      await settle();
      expect(
        model.sheets[0]['columnWidths']['1'],
        grid.columnMetrics.fromCharacters(20),
      );
      expect(
        grid.columnMetrics.characters(model.sheets[0]['columnWidths']['1']),
        20,
      );
      final shortRevision = model.revision;
      await tester.tap(find.byKey(const Key('column-resize-0')));
      await tester.pump(const Duration(milliseconds: 80));
      await tester.tap(find.byKey(const Key('column-resize-0')));
      await revision(shortRevision + 1);
      expect(model.sheets[0]['columnWidths']['1'], widths['1']);
      // A row context selection must not narrow a multi-column fit.
      await select('A1:C1');
      await tester.tap(
        find.byKey(const Key('column-header-1')),
        buttons: kSecondaryMouseButton,
      );
      await settle();
      expect(grid.selectedRange, 'A1:C1');
      await tester.tap(find.text('AutoFit Column Width'));
      await drive(grid.autoFitColumns);
      await settle();
      expect(model.sheets[0]['columnWidths'], widths);
      if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
        await select('A1:C11');
        await drive(grid.autoFitRows);
        await select('A1');
        await drive(model.flush);
        await settle();
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('column-fit-preview')),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          await File(
            '${Directory.current.path}/release/previews/column-autofit.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
        });
      }
      await select('B4');
      grid.edit();
      await settle();
      final pending = List.filled(12, 'New review notes').join(' ');
      await tester.enterText(find.byKey(const Key('cell-editor')), pending);
      final pendingRevision = model.revision;
      await drive(grid.autoFitColumns);
      await settle();
      expect(model.revision, pendingRevision + 2);
      expect(model.sheets[0]['cells']['B4'], pending);
      expect(model.sheets[0]['columnWidths']['2'], greaterThan(widths['2']));
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      await settle();
      expect(model.sheets[0]['columnWidths']['2'], widths['2']);
      expect(model.sheets[0]['cells']['B4'], pending);
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      await settle();
      expect(
        model.sheets[0]['cells']['B4'],
        'Build together and test the details.',
      );
      await drive(model.flush);
      expect(tester.takeException(), isNull);
    },
  );
}
