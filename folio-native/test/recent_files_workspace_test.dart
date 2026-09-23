import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/file_browser.dart';
import 'package:folio_native/src/recent_files.dart';
import 'package:folio_native/src/working_files.dart';
import 'package:folio_native/src/workspace_file_commands.dart';

void main() {
  testWidgets(
    'browser restores immutable copies and reopens pinned moved files with balanced grants',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final capture = Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1';
      if (capture) {
        await tester.runAsync(() async {
          for (final (family, path) in [
            ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
            ('Aptos', '/System/Library/Fonts/Supplemental/Arial.ttf'),
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
        });
      }
      late Directory dir;
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-recent-workspace-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
      });
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      const selector = MethodChannel('plugins.flutter.io/file_selector');
      final choices = <String?>[];
      final suggestions = <String>[];
      String? located, moved;
      var failAccess = false, acquired = 0, released = 0;
      messenger.setMockMethodCallHandler(selector, (call) async {
        if (call.method == 'openFile') {
          return located == null ? null : [located];
        }
        suggestions.add((call.arguments as Map)['suggestedName'] as String);
        return choices.isEmpty ? null : choices.removeAt(0);
      });
      messenger.setMockMethodCallHandler(fileAccessChannel, (call) async {
        final args = call.arguments as Map;
        if (call.method == 'release') {
          released++;
          return null;
        }
        if (call.method == 'capture') return args['path'];
        if (failAccess) {
          throw PlatformException(
            code: 'access',
            message: 'Bookmark unavailable',
          );
        }
        acquired++;
        return {
          'path': moved ?? args['bookmark'],
          'bookmark': moved ?? args['bookmark'],
          'lease': '$acquired',
        };
      });
      messenger.setMockMethodCallHandler(fileMenuChannel, (_) async => null);
      final renderKey = GlobalKey();
      await tester.pumpWidget(
        RepaintBoundary(
          key: renderKey,
          child: FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
        ),
      );
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
        messenger.setMockMethodCallHandler(selector, null);
        messenger.setMockMethodCallHandler(fileAccessChannel, null);
        messenger.setMockMethodCallHandler(fileMenuChannel, null);
      });
      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 600 && !ready(); i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 5)),
          );
        }
        expect(ready(), isTrue);
        await tester.pumpAndSettle();
      }

      Future<void> drive(Future<void> Function() action) async {
        var done = false;
        Object? failure;
        unawaited(
          Future.sync(action).then(
            (_) => done = true,
            onError: (Object e) {
              failure = e;
              done = true;
            },
          ),
        );
        await until(() => done);
        if (failure != null) throw failure!;
      }

      Future<void> command(WorkspaceFileCommand command) =>
          screen.fileCommand(command);
      void type(String text) => screen.editor.replaceText(
        0,
        0,
        text,
        TextSelection.collapsed(offset: text.length),
      );
      await drive(() => screen.filesReady);
      type('Keep this recovery copy. ');
      await drive(() => screen.preserveCurrent());
      late RecoveryCopy copy;
      await tester.runAsync(() async {
        copy = (await screen.recoveryCopies.list()).single;
      });
      final bytes = File(copy.path).readAsBytesSync();
      final originalId = model.workspace['id'];
      type('A later version. ');
      final doc = '${dir.path}/report.docx';
      choices.add(doc);
      await drive(() => command(WorkspaceFileCommand.saveAs));
      await drive(() => screen.preserveCurrent());
      expect(File(copy.path).readAsBytesSync(), bytes);

      var done = false;
      unawaited(command(WorkspaceFileCommand.recent).then((_) => done = true));
      await until(() => find.byType(FileBrowser).evaluate().isNotEmpty);
      expect(screen.fileCommandEnabled(WorkspaceFileCommand.open), isFalse);
      await tester.tap(find.text('Recovery copies'));
      await until(() => find.byKey(ValueKey(copy.path)).evaluate().isNotEmpty);
      await tester.tap(find.byKey(ValueKey(copy.path)));
      await until(
        () =>
            tester
                .widget<FilledButton>(
                  find.widgetWithText(FilledButton, 'Restore copy'),
                )
                .onPressed !=
            null,
      );
      if (capture) {
        await tester.runAsync(() async {
          final boundary =
              renderKey.currentContext!.findRenderObject()
                  as RenderRepaintBoundary;
          final image = await boundary.toImage();
          final data = await image.toByteData(format: ui.ImageByteFormat.png);
          await File(
            '../../work/recovery-browser-widget.png',
          ).writeAsBytes(data!.buffer.asUint8List());
          image.dispose();
        });
      }
      await tester.tap(find.text('Restore copy'));
      await until(() => done);
      expect(model.workspace['id'], isNot(originalId));
      expect(
        screen.editor.document.toPlainText(),
        contains('Keep this recovery copy.'),
      );
      expect(
        screen.editor.document.toPlainText(),
        isNot(contains('A later version.')),
      );
      expect(screen.activeWorkingFile, isNull);
      expect(screen.workingFiles.workspaceDraft, isTrue);
      expect(
        screen.recentFiles.entries.any((f) => f.path == copy.path),
        isFalse,
      );
      expect(File(copy.path).readAsBytesSync(), bytes);

      choices.add(null);
      await drive(() => command(WorkspaceFileCommand.save));
      expect(screen.workingFiles.workspaceDraft, isTrue);
      final recovered = '${dir.path}/recovered.folio';
      choices.add(recovered);
      await drive(() => command(WorkspaceFileCommand.save));
      expect(suggestions.last, endsWith('.folio'));
      expect(screen.activeWorkingFile!.path, recovered);
      expect(File(copy.path).readAsBytesSync(), bytes);
      await drive(() => command(WorkspaceFileCommand.newWorkspace));
      expect(screen.recentFiles.entries, hasLength(2));

      done = false;
      unawaited(command(WorkspaceFileCommand.recent).then((_) => done = true));
      await until(() => find.byType(FileBrowser).evaluate().isNotEmpty);
      await tester.tap(find.byTooltip('Pin report.docx'));
      await until(() => screen.recentFiles.entries.first.pinned);
      await tester.tap(find.text('report.docx'));
      await tester.pump();
      await tester.tap(find.widgetWithText(FilledButton, 'Open'));
      await until(() => done);
      expect(
        screen.editor.document.toPlainText(),
        contains('A later version.'),
      );
      expect(acquired, released);
      final old = screen.recentFiles.entries.firstWhere((e) => e.path == doc);
      moved = '${dir.path}/moved.docx';
      File(doc).renameSync(moved);
      await drive(() => screen.openRecentFile(old));
      expect(screen.activeWorkingFile!.path, moved);
      expect(screen.recentFiles.entries.first.path, moved);
      expect(screen.recentFiles.entries.first.pinned, isTrue);
      expect(screen.recentFiles.entries.any((e) => e.path == doc), isFalse);
      expect(acquired, released);

      // Unavailable bookmark: Cancel preserves state; explicit Locate refreshes
      // the entry and its pin without creating another history item.
      failAccess = true;
      final currentId = model.workspace['id'];
      done = false;
      unawaited(
        screen
            .openRecentFile(screen.recentFiles.entries.first)
            .then((_) => done = true),
      );
      await until(() => find.text('Locate file…').evaluate().isNotEmpty);
      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await until(() => done);
      expect(model.workspace['id'], currentId);
      done = false;
      located = moved;
      unawaited(
        screen
            .openRecentFile(screen.recentFiles.entries.first)
            .then((_) => done = true),
      );
      await until(() => find.text('Locate file…').evaluate().isNotEmpty);
      await tester.tap(find.text('Locate file…'));
      await until(() => done);
      expect(screen.recentFiles.entries, hasLength(2));
      expect(screen.recentFiles.entries.first.pinned, isTrue);
      final reload = RecentFiles(screen.recentFiles.storePath);
      await tester.runAsync(reload.load);
      expect(reload.entries.first.path, moved);
      expect(reload.entries.first.pinned, isTrue);
      failAccess = false;
      moved = null;
      type('Unsaved local edit. ');
      final lastUsed = screen.recentFiles.entries
          .firstWhere((e) => e.path == recovered)
          .lastUsed;
      done = false;
      unawaited(command(WorkspaceFileCommand.recent).then((_) => done = true));
      await until(() => find.byType(FileBrowser).evaluate().isNotEmpty);
      await tester.tap(find.text('recovered.folio'));
      await tester.pump();
      await tester.tap(find.widgetWithText(FilledButton, 'Open'));
      await until(
        () => find.text('Save changes to your files?').evaluate().isNotEmpty,
      );
      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await until(() => done);
      expect(
        screen.editor.document.toPlainText(),
        startsWith('Unsaved local edit.'),
      );
      expect(
        screen.recentFiles.entries
            .firstWhere((e) => e.path == recovered)
            .lastUsed,
        lastUsed,
      );
      expect(acquired, released);
      expect(tester.takeException(), isNull);
    },
  );
}
