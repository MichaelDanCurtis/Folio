import 'recovery_test_helpers.dart';
import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/workspace_file_commands.dart';
import 'package:folio_native/src/working_files.dart';

void main() {
  testWidgets(
    'leaving files honors Save, Don’t Save, Cancel, failures and native close',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory dir;
      final output = Directory(
        '${Directory.systemTemp.path}/folio-file-leave-ui',
      );
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-file-leave-');
        await output.create(recursive: true);
        final core = CoreBridge();
        await core.call({'op': 'new'});
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
      });
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      const selector = MethodChannel('plugins.flutter.io/file_selector');
      String? selected;
      var pickerCalls = 0, failAccess = false;
      messenger.setMockMethodCallHandler(selector, (call) async {
        pickerCalls++;
        return call.method == 'openFile'
            ? (selected == null ? null : [selected])
            : selected;
      });
      messenger.setMockMethodCallHandler(fileAccessChannel, (call) async {
        if (call.method == 'release') return null;
        final args = call.arguments as Map;
        if (call.method == 'capture') return args['path'];
        if (failAccess) {
          throw PlatformException(
            code: 'denied',
            message: 'Test access unavailable',
          );
        }
        return {
          'path': args['bookmark'],
          'bookmark': args['bookmark'],
          'lease': 'test',
        };
      });
      messenger.setMockMethodCallHandler(fileMenuChannel, (_) async => null);
      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await tester.pumpAndSettle();
      var screen = tester.state<WorkspaceScreenState>(
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
        await until(() => done);
        if (error != null) throw error!;
      }

      Future<void> command(WorkspaceFileCommand cmd) =>
          screen.guard(() => screen.fileCommand(cmd));
      Future<void> dialog() => until(
        () => find.text('Save changes to your files?').evaluate().isNotEmpty,
      );
      Future<void> choose(String text) async {
        await tester.tap(find.widgetWithText(TextButton, text).last);
        await tester.pumpAndSettle();
      }

      void type(String text) => screen.editor.replaceText(
        0,
        0,
        text,
        TextSelection.collapsed(offset: text.length),
      );
      Future<void> cell(String value) => model.mutate({
        'op': 'cells',
        'cells': [
          {'sheetId': screen.sheetId, 'address': 'B4', 'value': value},
        ],
      });
      Future<bool> nativeClose() async {
        final response = Completer<bool>();
        messenger.handlePlatformMessage(
          lifecycleChannel.name,
          const StandardMethodCodec().encodeMethodCall(
            const MethodCall('prepareClose'),
          ),
          (bytes) => response.complete(
            const StandardMethodCodec().decodeEnvelope(bytes!) as bool,
          ),
        );
        return response.future;
      }

      final doc = File('${output.path}/document.docx');
      final book = File('${output.path}/book.xlsx');
      selected = doc.path;
      await drive(() => command(WorkspaceFileCommand.save));
      await drive(() => screen.switchMode(true));
      selected = book.path;
      await drive(() => command(WorkspaceFileCommand.save));
      await drive(() => screen.switchMode(false));
      final docBefore = doc.readAsBytesSync(),
          bookBefore = book.readAsBytesSync();
      final initialId = model.workspace['id'];
      type('Saved on leave. ');
      await drive(() => cell('6789'));
      var done = false;
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await dialog();
      expect(find.text(doc.path), findsOneWidget);
      expect(find.text(book.path), findsOneWidget);
      // Repeated quit and File operations cannot bypass the pending decision.
      await drive(() async {
        expectSync(await nativeClose(), isFalse);
      });
      await drive(() => command(WorkspaceFileCommand.newWorkspace));
      expect(model.workspace['id'], initialId);
      await choose('Cancel');
      await until(() => done);
      expect(model.workspace['id'], initialId);
      expect(doc.readAsBytesSync(), docBefore);
      expect(book.readAsBytesSync(), bookBefore);

      done = false;
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await dialog();
      await tester.tap(find.widgetWithText(FilledButton, 'Save'));
      await until(() => done);
      expect(model.workspace['id'], isNot(initialId));
      expect(doc.readAsBytesSync(), isNot(docBefore));
      expect(book.readAsBytesSync(), isNot(bookBefore));
      expect(pickerCalls, 2, reason: 'Both saved files retain their locations');
      final snapshot = jsonDecode(
        latestRecoveryFor(dir, initialId).readAsStringSync(),
      );
      expect(snapshot['sheets'][0]['cells']['B4'], '6789');

      // Opening a workbook retains unrelated unsaved document edits.
      selected = doc.path;
      await drive(() => command(WorkspaceFileCommand.open));
      type('Retained local. ');
      selected = book.path;
      await drive(() => command(WorkspaceFileCommand.open));
      expect(
        screen.editor.document.toPlainText(),
        startsWith('Retained local. Saved on leave.'),
      );
      await drive(() => cell('7777'));
      done = false;
      unawaited(command(WorkspaceFileCommand.open).then((_) => done = true));
      await dialog();
      expect(find.text(book.path), findsOneWidget);
      expect(find.text(doc.path), findsNothing);
      await choose('Cancel');
      await until(() => done);
      expect(model.sheets.first['cells']['B4'], '7777');

      // Failed saving must cancel replacement and retain all content.
      final id = model.workspace['id'];
      failAccess = true;
      done = false;
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await dialog();
      await tester.tap(find.widgetWithText(FilledButton, 'Save'));
      await until(() => done);
      expect(model.workspace['id'], id);
      expect(screen.changedWorkingFiles(), hasLength(2));
      failAccess = false;

      // A conflict, then a cancelled Save As, also cancels New.
      final validDoc = doc.readAsBytesSync();
      doc.writeAsBytesSync([...validDoc, 10, 32]);
      done = false;
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await dialog();
      await tester.tap(find.widgetWithText(FilledButton, 'Save'));
      await until(
        () =>
            find.text('This file changed outside Folio').evaluate().isNotEmpty,
      );
      selected = null;
      await tester.tap(find.widgetWithText(FilledButton, 'Save As…'));
      await until(() => done);
      expect(model.workspace['id'], id);
      expect(
        screen.workingFiles.bound(model.workspace, WorkingFileKind.docx)!.path,
        doc.path,
      );
      doc.writeAsBytesSync(validDoc);

      // Native close cancellation propagates to AppKit, with one prompt.
      bool? closeResult;
      unawaited(nativeClose().then((value) => closeResult = value));
      await dialog();
      await drive(() async {
        expectSync(await nativeClose(), isFalse);
      });
      await choose('Cancel');
      await until(() => closeResult != null);
      expect(closeResult, isFalse);
      expect(screen.fileCommandEnabled(WorkspaceFileCommand.save), isTrue);

      // Don’t Save preserves external files and the outgoing recovery snapshot.
      final unchangedDoc = doc.readAsBytesSync(),
          unchangedBook = book.readAsBytesSync();
      done = false;
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await dialog();
      await choose('Don’t Save');
      await until(() => done);
      expect(model.workspace['id'], isNot(id));
      expect(doc.readAsBytesSync(), unchangedDoc);
      expect(book.readAsBytesSync(), unchangedBook);
      expect(
        jsonDecode(
          latestRecoveryFor(dir, id).readAsStringSync(),
        )['sheets'][0]['cells']['B4'],
        '7777',
      );

      // Save from Close drains the edit, and the final window/quit handoff does
      // not ask again. This uses the actual platform method handler.
      selected = doc.path;
      await drive(() => command(WorkspaceFileCommand.open));
      type('Saved on close. ');
      closeResult = null;
      unawaited(nativeClose().then((value) => closeResult = value));
      await dialog();
      await tester.tap(find.widgetWithText(FilledButton, 'Save'));
      await until(() => closeResult != null);
      expect(closeResult, isTrue);
      expect(screen.changedWorkingFiles(), isEmpty);
      await drive(() async {
        expectSync(await nativeClose(), isTrue);
      });
      expect(doc.readAsBytesSync(), isNot(unchangedDoc));
      // Recreate the screen to verify persisted clean state, not just live flags.
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await tester.pumpAndSettle();
      screen = tester.state<WorkspaceScreenState>(find.byType(WorkspaceScreen));
      await drive(() => screen.filesReady);
      expect(screen.fileSaveStatus, startsWith('Saved ·'));
      expect(tester.takeException(), isNull);
    },
  );
}
