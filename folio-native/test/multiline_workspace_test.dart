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
    'multiline cell and formula edits retain history, sizing and Office round trips',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-multiline-ui-tests',
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
        T? result;
        Object? error;
        unawaited(
          Future.sync(job).then(
            (value) {
              result = value;
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
        if (error != null) throw error!;
        expect(done, isTrue);
        await tester.pumpAndSettle();
        return result as T;
      }

      Future<void> revision(int expected) async {
        for (var i = 0; i < 300 && model.revision < expected; i++) {
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
          key: const Key('multiline-preview'),
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
      await drive(() => grid.select(2, 1));
      final original = model.sheets[0]['cells']['B3'] as String;
      final before = model.revision;
      await tester.sendKeyEvent(LogicalKeyboardKey.f2);
      await tester.pumpAndSettle();
      grid.cellController.selection = TextSelection.collapsed(
        offset: original.length,
      );
      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.altLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.altLeft);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await tester.pump();
      expect(grid.cellController.text, '$original\n');
      const extra = 'Coordination 🦉';
      final text = '$original\n$extra';
      tester.testTextInput.updateEditingValue(
        TextEditingValue(
          text: text,
          selection: TextSelection.collapsed(offset: text.length),
        ),
      );
      await tester.pump();
      expect(model.revision, before);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await revision(before + 1);
      expect(model.sheets[0]['cells']['B3'], text);
      expect(grid.selected, 'B4');
      await export('edited');
      await history();
      expect(model.sheets[0]['cells']['B3'], original);
      await export('undone');
      await history(redo: true);
      expect(model.sheets[0]['cells']['B3'], text);
      await export('redone');

      await drive(() => grid.select(9, 1));
      await tester.enterText(
        find.byKey(const Key('formula')),
        '=SUM(A10,\nA10)',
      );
      await tester.tap(find.byKey(const Key('expand-formula')));
      await tester.pumpAndSettle();
      expect(grid.formulaFocus.hasFocus, isTrue);
      final formulaRevision = model.revision;
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await revision(formulaRevision + 1);
      expect(model.sheets[0]['cells']['B10'], '=SUM(A10,\nA10)');
      expect(model.snapshot['calculated']['office-1']['B10'], 24691.34);
      await export('formula');
      await history();
      expect(model.sheets[0]['cells']['B10'], '=A10*2');
      expect(model.sheets[0]['cells']['B3'], text);
      await history(redo: true);
      await drive(() => grid.select(2, 1));
      await drive(grid.autoFitRows);
      expect(model.sheets[0]['rowHeights']['3'], greaterThan(50));
      await export('fitted');
      await drive(model.flush);
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
          'path': '${folder.path}/reopened.xlsx',
        }, refreshDocument: true),
      );
      expect(model.sheets[0]['cells']['B3'], text);
      expect(model.sheets[0]['cells']['B10'], '=SUM(A10,\nA10)');
      await export('reimported');
      if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
        await tester.pumpAndSettle();
        final box = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('multiline-preview')),
        );
        await tester.runAsync(() async {
          final rendered = await box.toImage();
          final bytes = await rendered.toByteData(
            format: ui.ImageByteFormat.png,
          );
          await File(
            'release/previews/multiline-editing.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
          rendered.dispose();
        });
      }
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
