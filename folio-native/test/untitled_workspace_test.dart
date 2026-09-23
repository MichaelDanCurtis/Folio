import 'recovery_test_helpers.dart';
import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_view.dart';
import 'package:folio_native/src/workspace_file_commands.dart';
import 'package:folio_native/src/working_files.dart';

class DiscardFailureCore extends CoreBridge {
  bool failDiscard = false;
  Completer<void>? discardReached, discardRelease;
  @override
  Future<T> transaction<T>(Future<T> Function(CoreCall dispatch) action) =>
      super.transaction(
        (dispatch) => action((request) async {
          if (request['op'] == 'saveEmptyWorkspace' && failDiscard) {
            throw StateError('Test recovery write unavailable');
          }
          final result = await dispatch(request);
          if (request['op'] == 'saveEmptyWorkspace') {
            discardReached?.complete();
            await discardRelease?.future;
          }
          return result;
        }),
      );
}

void main() {
  testWidgets(
    'first saves are per part and discarded close restarts blank with recoverable edits',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late DiscardFailureCore core;
      late WorkspaceModel model;
      late Directory dir;
      final output = Directory(
        '${Directory.systemTemp.path}/folio-untitled-ui',
      );
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-untitled-ui-');
        await output.create(recursive: true);
        core = DiscardFailureCore();
        await core.call({'op': 'new'});
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
      });
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      const selector = MethodChannel('plugins.flutter.io/file_selector');
      final choices = <String?>[];
      final suggestions = <String>[];
      String? openPath;
      messenger.setMockMethodCallHandler(selector, (call) async {
        if (call.method == 'openFile') {
          return openPath == null ? null : [openPath];
        }
        suggestions.add((call.arguments as Map)['suggestedName'] as String);
        return choices.isEmpty ? null : choices.removeAt(0);
      });
      messenger.setMockMethodCallHandler(fileAccessChannel, (call) async {
        if (call.method == 'release') return null;
        final args = call.arguments as Map;
        if (call.method == 'capture') return args['path'];
        return {
          'path': args['bookmark'],
          'bookmark': args['bookmark'],
          'lease': 'test',
        };
      });
      messenger.setMockMethodCallHandler(fileMenuChannel, (_) async => null);
      late WorkspaceScreenState screen;
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

      Future<void> mount() async {
        await tester.pumpWidget(
          FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
        );
        await tester.pumpAndSettle();
        screen = tester.state<WorkspaceScreenState>(
          find.byType(WorkspaceScreen),
        );
        await drive(() => screen.filesReady);
      }

      await mount();
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
        messenger.setMockMethodCallHandler(selector, null);
        messenger.setMockMethodCallHandler(fileAccessChannel, null);
        messenger.setMockMethodCallHandler(fileMenuChannel, null);
      });
      List<PendingWorkingFile> pending() => screen.workingFiles.pending(
        model.workspace,
        model.snapshot['fileStates'] as Map,
        model.revision,
      );
      Future<void> command(WorkspaceFileCommand cmd) =>
          screen.guard(() => screen.fileCommand(cmd));
      Future<void> dialog() => until(
        () => find.text('Save changes to your files?').evaluate().isNotEmpty,
      );
      void type(String text) => screen.editor.replaceText(
        0,
        0,
        text,
        TextSelection.collapsed(offset: text.length),
      );
      Future<void> saveDecision() async {
        await tester.tap(find.widgetWithText(FilledButton, 'Save'));
        await tester.pump();
      }

      Future<void> discardDecision() async {
        await tester.tap(find.widgetWithText(TextButton, 'Don’t Save'));
        await tester.pump();
      }

      expect(
        pending(),
        isEmpty,
        reason: 'Untouched starter content is not a new edit',
      );
      type('First document. ');
      await drive(screen.flush);
      expect(screen.fileSaveStatus, contains('Not saved to a file'));
      final originalId = model.workspace['id'];
      tester
          .state<DocumentViewState>(find.byType(DocumentView))
          .reveal(0, 0, requestFocus: true);
      await tester.pumpAndSettle();
      final documentFocus = FocusManager.instance.primaryFocus;
      var done = false;
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await dialog();
      expect(find.text('This document (not saved to a file)'), findsOneWidget);
      expect(find.text('This workbook (not saved to a file)'), findsNothing);
      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await until(() => done);
      expect(model.workspace['id'], originalId);
      expect(FocusManager.instance.primaryFocus, same(documentFocus));

      // Cancel the first Save As: New must remain cancelled and edits retained.
      done = false;
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await dialog();
      await saveDecision();
      await until(() => done);
      expect(suggestions.last, endsWith('.docx'));
      expect(model.workspace['id'], originalId);
      expect(pending().single.kind, WorkingFileKind.docx);
      final first = '${output.path}/first.docx';
      choices.add(first);
      done = false;
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await dialog();
      await saveDecision();
      await until(() => done);
      expect(File(first).existsSync(), true);
      expect(model.workspace['id'], isNot(originalId));
      expect(pending(), isEmpty);
      expect(model.snapshot['blankWorkspace'], true);

      type('Document and workbook saved separately.');
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': screen.sheetId, 'address': 'B4', 'value': '8888'},
          ],
        }),
      );
      final beforeNew = model.workspace['id'];
      final second = '${output.path}/second.docx',
          book = '${output.path}/book.xlsx';
      choices.addAll([second, null]);
      done = false;
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await dialog();
      expect(find.text('This document (not saved to a file)'), findsOneWidget);
      expect(find.text('This workbook (not saved to a file)'), findsOneWidget);
      await saveDecision();
      await until(() => done);
      expect(model.workspace['id'], beforeNew);
      expect(screen.spreadsheet, isFalse);
      expect(suggestions[suggestions.length - 2], endsWith('.docx'));
      expect(
        suggestions.last,
        endsWith('.xlsx'),
        reason: 'First workbook save must not follow the visible Word mode',
      );
      expect(pending().single.kind, WorkingFileKind.xlsx);
      expect(File(second).existsSync(), true);
      choices.add(book);
      done = false;
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await dialog();
      expect(find.text('This document (not saved to a file)'), findsNothing);
      await saveDecision();
      await until(() => done);
      expect(File(book).existsSync(), true);
      expect(model.workspace['id'], isNot(beforeNew));

      openPath = first;
      await drive(() => command(WorkspaceFileCommand.open));
      final external = File(first).readAsBytesSync();
      type('Discarded close edit. ');
      await drive(screen.flush);
      final discardId = model.workspace['id'];
      final documentBeforeClose = jsonEncode(model.workspace['document']);
      core.failDiscard = true;
      Object? closeError;
      bool? closeResult;
      unawaited(
        screen.prepareClose().then(
          (value) => closeResult = value,
          onError: (Object e) => closeError = e,
        ),
      );
      await dialog();
      await discardDecision();
      await until(() => closeError != null);
      expect(closeResult, isNull);
      expect(jsonEncode(model.workspace['document']), documentBeforeClose);
      expect(model.canUndo, isTrue);
      expect(screen.fileCommandEnabled(WorkspaceFileCommand.save), isTrue);
      expect(
        jsonEncode(
          jsonDecode(File(model.recoveryPath).readAsStringSync())['document'],
        ),
        documentBeforeClose,
      );
      core.failDiscard = false;
      core.discardReached = Completer<void>();
      core.discardRelease = Completer<void>();
      unawaited(screen.prepareClose().then((value) => closeResult = value));
      await dialog();
      await discardDecision();
      await until(() => core.discardReached!.isCompleted);
      var lateFlush = false;
      unawaited(model.flush().then((_) => lateFlush = true));
      await tester.pump();
      expect(lateFlush, isFalse);
      core.discardRelease!.complete();
      await until(() => closeResult != null && lateFlush);
      expect(closeResult, isTrue);
      expect(File(first).readAsBytesSync(), external);
      final blankBytes = File(model.recoveryPath).readAsBytesSync();
      final blank = jsonDecode(utf8.decode(blankBytes));
      expect(blank['id'], isNot(discardId));
      expect(blank['sheets'][0]['cells'], isEmpty);
      expect(blank['document']['content'], [
        {'type': 'paragraph'},
      ]);
      await drive(model.flush);
      expect(
        File(model.recoveryPath).readAsBytesSync(),
        blankBytes,
        reason: 'A late autosave must not revive discarded content',
      );
      final recoveryCopy = latestRecoveryFor(dir, discardId).path;
      expect(
        jsonEncode(
          jsonDecode(File(recoveryCopy).readAsStringSync())['document'],
        ),
        documentBeforeClose,
      );
      File(recoveryCopy).copySync('${output.path}/discarded-recovery.folio');
      File(model.recoveryPath).copySync('${output.path}/blank-restart.folio');

      // New process state: the blank recovery suppresses stale file grants and
      // does not ask to save an untouched blank document/workbook.
      await tester.pumpWidget(const SizedBox.shrink());
      final recoveryPath = model.recoveryPath;
      model.dispose();
      await tester.runAsync(() async {
        core = DiscardFailureCore();
        model = WorkspaceModel(core, recoveryPath);
        await model.initialize();
      });
      await mount();
      expect(model.recovered, isTrue);
      expect(model.snapshot['blankWorkspace'], true);
      expect(screen.activeWorkingFile, isNull);
      expect(pending(), isEmpty);
      openPath = recoveryCopy;
      await drive(() => command(WorkspaceFileCommand.open));
      expect(
        jsonEncode(model.workspace['document']),
        documentBeforeClose,
        reason: 'Discarded work remains recoverable via ordinary Open',
      );
      expect(tester.takeException(), isNull);
    },
  );
}
