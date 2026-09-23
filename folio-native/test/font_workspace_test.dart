import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter/rendering.dart';
import 'dart:ui' as ui;
import 'package:folio_native/src/sheet_font.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'font dialog previews mixed selections, applies atomically, persists and rejects stale edits',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-font-ui-tests',
      );
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
          for (final (family, path) in [
            ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
            ('Aptos', '/System/Library/Fonts/Supplemental/Arial.ttf'),
            ('Calibri', '/System/Library/Fonts/Supplemental/Arial.ttf'),
            ('Arial', '/System/Library/Fonts/Supplemental/Arial.ttf'),
            (
              'Times New Roman',
              '/System/Library/Fonts/Supplemental/Times New Roman.ttf',
            ),
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
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/styles.xlsx',
        });
        model = WorkspaceModel(core, '${folder.path}/recovery.folio');
        // Test exports remain available to the independent reader. Do not load
        // the recovery file from a previous invocation of this isolated test.
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
          key: const Key('font-preview-frame'),
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
      await tester.enterText(find.byKey(ValueKey(grid.selectedRange)), 'B2');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await drive(grid.flush);
      await settle();

      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        const MethodChannel('folio/fonts'),
        (call) async => ['Arial', 'Georgia', 'Menlo'],
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          const MethodChannel('folio/fonts'),
          null,
        ),
      );
      final original = Map<String, dynamic>.from(
        model.sheets[0]['cellStyles']['B2'],
      );
      final rendered = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('sheet-font-B2')),
      );
      expect(rendered.textStyle.fontFamily, 'Cambria');
      expect(rendered.textStyle.fontSize, 24);
      expect(rendered.baselineShift, lessThan(0));
      Future<void> openFont() async {
        await tester.tap(find.byKey(const Key('cell-font-button')));
        await settle();
        expect(find.text('Font'), findsOneWidget);
      }

      Future<void> select(String range) async {
        await tester.enterText(find.byKey(ValueKey(grid.selectedRange)), range);
        await tester.testTextInput.receiveAction(TextInputAction.done);
        await drive(grid.flush);
        await settle();
      }

      Future<void> shortcut(LogicalKeyboardKey key) async {
        grid.gridFocus.requestFocus();
        await settle();
        await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
        await tester.sendKeyEvent(key);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
        await settle();
      }

      // One cell can itself be mixed. Its actual runs, not just its XF,
      // determine the dialog state and remain visible in the preview.
      await select('S2');
      await openFont();
      expect(find.text('1 cell · Mixed formatting'), findsOneWidget);
      expect(
        tester
            .widget<TextField>(find.byKey(const Key('font-family')))
            .controller!
            .text,
        isEmpty,
      );
      expect(
        tester
            .widget<TextField>(find.byKey(const Key('font-size')))
            .controller!
            .text,
        isEmpty,
      );
      expect(
        tester.widget<Checkbox>(find.byKey(const Key('font-bold'))).value,
        isNull,
      );
      var richPreview = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('font-preview')),
      );
      expect(richPreview.runs.map((r) => r.text), ['Shared', ' title']);
      expect(richPreview.runs[0].font['fontName'], 'Times New Roman');
      expect(richPreview.runs[0].font['color'], '#AA0000');
      expect(richPreview.runs[1].font['fontSize'], 22);
      expect(richPreview.runs[1].font['italic'], true);
      if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('font-preview-frame')),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          await File(
            '${Directory.current.path}/release/previews/rich-cell-fonts.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
        });
      }
      await tester.tap(find.byKey(const Key('font-bold')));
      await settle();
      richPreview = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('font-preview')),
      );
      expect(richPreview.runs.every((r) => r.font['bold'] == false), true);
      final richBefore = model.revision;
      await tester.tap(find.byKey(const Key('apply-cell-font')));
      await revision(richBefore + 1);
      await export('rich-render');
      expect(grid.fontAt('S2')['bold'], false);
      expect(grid.fontAt('T2')['bold'], isNull);
      await drive(() => screen.history(false));
      await export('rich-render-undone');
      expect(grid.fontAt('S2')['bold'], isNull);
      await openFont();
      await tester.tap(find.byKey(const Key('font-character-mode')));
      await settle();
      final selector = find.byKey(const Key('font-character-selection'));
      await tester.tap(selector);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      for (var i = 0; i < 7; i++) {
        await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      }
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await settle();
      expect(
        tester.widget<TextField>(selector).controller!.selection,
        const TextSelection(baseOffset: 2, extentOffset: 9),
      );
      expect(find.textContaining('7 characters selected'), findsOneWidget);
      await tester.enterText(find.byKey(const Key('font-family')), 'Georgia');
      await tester.ensureVisible(find.byKey(const Key('font-bold')));
      await tester.tap(find.byKey(const Key('font-bold')));
      await tester.ensureVisible(find.byKey(const Key('font-preview')));
      await settle();
      richPreview = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('font-preview')),
      );
      expect(richPreview.runs.map((r) => r.text), ['Sh', 'ared', ' ti', 'tle']);
      expect(richPreview.runs[0].font['fontName'], 'Times New Roman');
      expect(richPreview.runs[1].font['fontName'], 'Georgia');
      expect(richPreview.runs[2].font['fontName'], 'Georgia');
      expect(richPreview.runs[3].font['fontSize'], 22);
      expect(richPreview.runs[1].font['bold'], false);
      if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
        tester.view.physicalSize = const Size(1152, 960);
        await settle();
        await tester.ensureVisible(find.byKey(const Key('font-summary')));
        await settle();
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('font-preview-frame')),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          await File(
            '${Directory.current.path}/release/previews/character-fonts.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
        });
        tester.view.physicalSize = const Size(1152, 768);
        await settle();
      }
      final characterBefore = model.revision;
      await tester.tap(find.byKey(const Key('apply-cell-font')));
      await revision(characterBefore + 1);
      await export('characters-ui');
      final editedRuns =
          model.sheets[0]['cellStyles']['S2']['_richText'] as List;
      expect(editedRuns.map((r) => r['text']), ['Sh', 'ared', ' ti', 'tle']);
      expect(editedRuns[1]['style']['fontName'], 'Georgia');
      await drive(() => screen.history(false));
      await export('characters-ui-undone');
      await drive(() => screen.history(true));
      await export('characters-ui-redone');
      await drive(() => screen.history(false));
      await select('B2');

      var before = model.revision;
      await openFont();
      expect(find.byKey(const Key('font-character-mode')), findsNothing);
      await tester.enterText(find.byKey(const Key('font-family')), 'Georgia');
      await tester.tap(find.text('Cancel'));
      await settle();
      expect(model.revision, before);
      expect(model.sheets[0]['cellStyles']['B2'], original);
      await select('B2:E2');
      await openFont();
      expect(find.textContaining('Mixed formatting'), findsOneWidget);
      expect(
        tester
            .widget<TextField>(find.byKey(const Key('font-family')))
            .controller!
            .text,
        isEmpty,
      );
      await tester.enterText(find.byKey(const Key('font-family')), 'Georgia');
      await tester.enterText(find.byKey(const Key('font-size')), '16.2');
      await tester.tap(find.byKey(const Key('apply-cell-font')));
      await settle();
      expect(find.textContaining('half-point steps'), findsOneWidget);
      expect(model.revision, before);
      await tester.enterText(find.byKey(const Key('font-size')), '16.5');
      await tester.tap(find.byKey(const ValueKey('font-underline-null')));
      await settle();
      await tester.tap(find.text('Double accounting').last);
      await settle();
      await tester.tap(find.byKey(const ValueKey('font-verticalAlign-null')));
      await settle();
      await tester.tap(find.text('Subscript').last);
      await settle();
      await tester.tap(find.byKey(const Key('font-strike')));
      await settle();
      await tester.enterText(find.byKey(const Key('font-color')), '#345678');
      await settle();
      final preview = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('font-preview')),
      );
      expect(preview.textStyle.fontFamily, 'Georgia');
      expect(preview.textStyle.fontSize, 22);
      expect(preview.baselineShift, greaterThan(0));
      expect(preview.underline, 'doubleAccounting');
      if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('font-preview-frame')),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          await File(
            '${Directory.current.path}/release/previews/font-dialog.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
        });
      }
      await tester.tap(find.byKey(const Key('apply-cell-font')));
      await revision(before + 1);
      for (final address in ['B2', 'C2', 'D2', 'E2']) {
        final style = model.sheets[0]['cellStyles'][address];
        expect(style['fontName'], 'Georgia');
        expect(style['fontSize'], 16.5);
        expect(style['underline'], 'doubleAccounting');
        expect(style['verticalAlign'], 'subscript');
        expect(style['strike'], false);
        expect(style['color'], '#345678');
      }
      expect(model.sheets[0]['cellStyles']['B2']['bold'], true);
      expect(model.sheets[0]['cellStyles']['C2']['bold'], isNull);
      expect(grid.gridFocus.hasPrimaryFocus, true);
      await export('edited');
      await drive(() => screen.history(false));
      await export('undone');
      expect(model.sheets[0]['cellStyles']['B2'], original);
      await drive(() => screen.history(true));
      await export('redone');
      await drive(
        () => model.core.call({
          'op': 'save',
          'path': '${folder.path}/saved.folio',
        }),
      );
      await drive(
        () => model.mutate({
          'op': 'load',
          'path': '${folder.path}/saved.folio',
        }, refreshDocument: true),
      );
      await export('reopened');
      await select('B2');
      before = model.revision;
      await openFont();
      await tester.enterText(find.byKey(const Key('font-size')), '14');
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 'office-1', 'address': 'A2', 'value': '3'},
          ],
        }),
      );
      await settle();
      await tester.tap(find.byKey(const Key('apply-cell-font')));
      await settle();
      expect(model.revision, before + 1);
      expect(model.sheets[0]['cellStyles']['B2']['fontSize'], 16.5);
      expect(find.textContaining('The workbook changed.'), findsOneWidget);
      before = model.revision;
      await shortcut(LogicalKeyboardKey.keyU);
      await revision(before + 1);
      expect(model.sheets[0]['cellStyles']['B2']['underline'], 'none');
      before = model.revision;
      await shortcut(LogicalKeyboardKey.keyB);
      await revision(before + 1);
      expect(model.sheets[0]['cellStyles']['B2']['bold'], false);
      before = model.revision;
      await shortcut(LogicalKeyboardKey.keyI);
      await revision(before + 1);
      expect(model.sheets[0]['cellStyles']['B2']['italic'], false);
      await shortcut(LogicalKeyboardKey.digit1);
      expect(find.text('Font'), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await settle();
      await tester.enterText(find.byKey(const Key('formula')), 'x' * 10001);
      before = model.revision;
      await tester.tap(find.byKey(const Key('cell-font-button')));
      await drive(grid.editFont);
      await settle();
      expect(find.text('Font'), findsNothing);
      expect(model.revision, before);
      expect(grid.formulaDirty, true);
      await tester.enterText(find.byKey(const Key('formula')), '=A2*\$H\$1');
      await drive(grid.commitFormula);
      await drive(
        () => model.mutate({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/alignment.xlsx',
        }, refreshDocument: true),
      );
      await select('A30:B30');
      Future<void> openAlignment() async {
        await tester.ensureVisible(
          find.byKey(const Key('cell-alignment-button')),
        );
        await tester.tap(find.byKey(const Key('cell-alignment-button')));
        await settle();
        expect(find.text('Alignment'), findsOneWidget);
      }

      before = model.revision;
      await openAlignment();
      await tester.tap(
        find.byKey(const ValueKey('alignment-horizontalAlignment-null')),
      );
      await settle();
      await tester.tap(find.text('Center').last);
      await settle();
      await tester.tap(
        find.byKey(const ValueKey('alignment-verticalAlignment-null')),
      );
      await settle();
      await tester.tap(find.text('Top').last);
      await settle();
      await tester.enterText(find.byKey(const Key('alignment-degrees')), '-30');
      await settle();
      final rotated = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('alignment-preview')),
      );
      expect(rotated.paintRotation, greaterThan(0));
      await tester.enterText(find.byKey(const Key('alignment-degrees')), '0');
      await tester.enterText(find.byKey(const Key('alignment-indent')), '0');
      await tester.tap(
        find.byKey(const ValueKey('alignment-readingOrder-null')),
      );
      await settle();
      await tester.tap(find.text('Context').last);
      await settle();
      await tester.tap(find.byKey(const Key('alignment-wrapText')));
      await settle();
      expect(rotated.lineCount, greaterThan(1));
      if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
        tester.view.physicalSize = const Size(1152, 960);
        await settle();
        await tester.ensureVisible(find.byKey(const Key('alignment-summary')));
        await settle();
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('font-preview-frame')),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          await File(
            '${Directory.current.path}/release/previews/cell-alignment.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
        });
        tester.view.physicalSize = const Size(1152, 768);
        await settle();
      }
      await tester.tap(find.byKey(const Key('apply-cell-alignment')));
      await revision(before + 1);
      for (final a in ['A30', 'B30']) {
        final style = model.sheets[0]['cellStyles'][a];
        expect(style['horizontalAlignment'], 'center');
        expect(style['verticalAlignment'], 'top');
        expect(style['wrapText'], true);
        expect(style['textRotation'], 0);
      }
      expect(
        tester
            .renderObject<SheetFontRenderBox>(
              find.byKey(const Key('sheet-font-A30')),
            )
            .lineCount,
        greaterThan(1),
      );
      await export('alignment-ui');
      await drive(() => screen.history(false));
      await export('alignment-ui-undone');
      await drive(() => screen.history(true));
      await export('alignment-ui-redone');
      await drive(
        () => model.core.call({
          'op': 'save',
          'path': '${folder.path}/alignment.folio',
        }),
      );
      await drive(
        () => model.mutate({
          'op': 'load',
          'path': '${folder.path}/alignment.folio',
        }, refreshDocument: true),
      );
      await export('alignment-ui-reopened');
      await select('A30');
      await openAlignment();
      await tester.enterText(find.byKey(const Key('alignment-indent')), '1');
      before = model.revision;
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 'office-1', 'address': 'A1', 'value': 'Changed'},
          ],
        }),
      );
      await settle();
      await tester.tap(find.byKey(const Key('apply-cell-alignment')));
      await settle();
      expect(model.revision, before + 1);
      expect(model.sheets[0]['cellStyles']['A30']['indent'], 0);
      expect(find.textContaining('Open Alignment again'), findsOneWidget);
      await drive(screen.flush);
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );
}
