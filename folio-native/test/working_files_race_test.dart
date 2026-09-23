import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/workspace_file_commands.dart';
import 'package:folio_native/src/working_files.dart';

class ExternalWriterCore extends CoreBridge {
  bool beforeWrite = false, afterWrite = false;
  String? actuallySavedFingerprint;
  @override
  Future<T> transaction<T>(Future<T> Function(CoreCall dispatch) action) =>
      super.transaction(
        (dispatch) => action((request) async {
          final guarded =
              request['op'] == 'exportOffice' &&
              request['expectedFingerprint'] != null;
          final file = guarded ? File(request['path'] as String) : null;
          if (guarded && beforeWrite) {
            beforeWrite = false;
            await file!.writeAsString('external edit before dispatch');
          }
          final result = await dispatch(request);
          if (guarded && afterWrite) {
            afterWrite = false;
            actuallySavedFingerprint = await fileFingerprint(file!.path);
            await file.writeAsString('external edit after dispatch');
          }
          return result;
        }),
      );
}

void main() {
  testWidgets(
    'external writes during the save cannot become the saved baseline',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late ExternalWriterCore core;
      late WorkspaceModel model;
      late Directory dir;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-save-race-');
        core = ExternalWriterCore();
        await core.call({'op': 'new'});
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
      });
      final path = '${dir.path}/working.docx';
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      const selector = MethodChannel('plugins.flutter.io/file_selector');
      messenger.setMockMethodCallHandler(selector, (_) async => path);
      var released = 0;
      Completer<void>? captureReached, captureRelease;
      messenger.setMockMethodCallHandler(fileAccessChannel, (call) async {
        if (call.method == 'capture') {
          captureReached?.complete();
          await captureRelease?.future;
          return path;
        }
        if (call.method == 'release') {
          released++;
          return null;
        }
        return {'path': path, 'bookmark': path, 'lease': 'active'};
      });
      messenger.setMockMethodCallHandler(fileMenuChannel, (_) async => null);
      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
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
        for (var i = 0; i < 500 && !ready(); i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 5)),
          );
        }
        expect(ready(), true);
        await tester.pumpAndSettle();
      }

      Future<void> command(WorkspaceFileCommand command) async {
        var done = false;
        unawaited(
          screen
              .guard(() => screen.fileCommand(command))
              .then((_) => done = true),
        );
        await until(() => done);
      }

      await command(WorkspaceFileCommand.saveAs);
      final originalFingerprint = screen.activeWorkingFile!.fingerprint;
      screen.editor.replaceText(
        0,
        0,
        'Local ',
        const TextSelection.collapsed(offset: 6),
      );
      core.beforeWrite = true;
      await command(WorkspaceFileCommand.save);
      expect(File(path).readAsStringSync(), 'external edit before dispatch');
      expect(screen.activeWorkingFile!.fingerprint, originalFingerprint);
      expect(screen.fileSaveStatus, startsWith('Changes saved locally'));
      expect(released, 1);
      // An explicit Save As authorizes replacing the chosen test destination.
      await command(WorkspaceFileCommand.saveAs);
      screen.editor.replaceText(
        0,
        0,
        'Latest ',
        const TextSelection.collapsed(offset: 7),
      );
      core.afterWrite = true;
      await command(WorkspaceFileCommand.save);
      expect(File(path).readAsStringSync(), 'external edit after dispatch');
      expect(
        screen.activeWorkingFile!.fingerprint,
        core.actuallySavedFingerprint,
      );
      var done = false;
      unawaited(
        screen.fileCommand(WorkspaceFileCommand.save).then((_) => done = true),
      );
      await until(
        () =>
            find.text('This file changed outside Folio').evaluate().isNotEmpty,
      );
      await tester.tap(find.text('Cancel').last);
      await until(() => done);
      expect(File(path).readAsStringSync(), 'external edit after dispatch');
      expect(released, 3);
      await command(WorkspaceFileCommand.saveAs);
      screen.editor.replaceText(
        0,
        0,
        'Before save ',
        const TextSelection.collapsed(offset: 12),
      );
      captureReached = Completer<void>();
      captureRelease = Completer<void>();
      done = false;
      unawaited(
        screen.fileCommand(WorkspaceFileCommand.save).then((_) => done = true),
      );
      await until(() => captureReached!.isCompleted);
      final savedState = model.snapshot['fileStates']['docx'];
      screen.editor.replaceText(
        0,
        0,
        'After save ',
        const TextSelection.collapsed(offset: 11),
      );
      var flushed = false;
      unawaited(screen.flush().then((_) => flushed = true));
      await until(() => flushed);
      captureRelease.complete();
      await until(() => done);
      captureReached = null;
      captureRelease = null;
      expect(screen.activeWorkingFile!.contentSignature, savedState);
      expect(
        screen.fileSaveStatus,
        startsWith('Changes saved locally'),
        reason: 'Typing after the write is not part of its saved content',
      );
      await command(WorkspaceFileCommand.save);
      expect(screen.fileSaveStatus, startsWith('Saved ·'));
      expect(tester.takeException(), isNull);
    },
  );
}
