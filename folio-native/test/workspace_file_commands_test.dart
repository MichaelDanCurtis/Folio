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

class HeldHistoryCore extends CoreBridge {
  Completer<void>? reached, release;
  @override
  Future<T> transaction<T>(Future<T> Function(CoreCall dispatch) action) =>
      super.transaction(
        (dispatch) => action((request) async {
          if (request['op'] == 'undo' && release != null) {
            reached!.complete();
            await release!.future;
            release = null;
          }
          return dispatch(request);
        }),
      );
}

void main() {
  testWidgets('file keys and native commands share safe file transactions', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1440, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    late WorkspaceModel model;
    late HeldHistoryCore core;
    late Directory dir;
    final output = Directory(
      '${Directory.systemTemp.path}/folio-file-commands-ui',
    );
    await tester.runAsync(() async {
      await CoreBridge.initialize();
      core = HeldHistoryCore();
      dir = await Directory.systemTemp.createTemp('folio-file-commands-');
      await output.create(recursive: true);
      await core.call({'op': 'new'});
      model = WorkspaceModel(core, '${dir.path}/workspace.folio');
      await model.initialize();
    });
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    const selector = MethodChannel('plugins.flutter.io/file_selector');
    messenger.setMockMethodCallHandler(fileAccessChannel, (call) async {
      if (call.method == 'capture') return (call.arguments as Map)['path'];
      if (call.method == 'release') return null;
      final path = (call.arguments as Map)['bookmark'];
      return {'path': path, 'bookmark': path, 'lease': path};
    });
    final panels = <MethodCall>[];
    final menuStates = <Map>[];
    Completer<Object?>? panel;
    messenger.setMockMethodCallHandler(selector, (call) async {
      panels.add(call);
      panel = Completer<Object?>();
      return panel!.future;
    });
    messenger.setMockMethodCallHandler(fileMenuChannel, (call) async {
      menuStates.add(call.arguments as Map);
      return null;
    });
    await tester.pumpWidget(
      FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
    );
    await tester.pumpAndSettle();
    final screen = tester.state<WorkspaceScreenState>(
      find.byType(WorkspaceScreen),
    );
    addTearDown(() async {
      if (panel != null && !panel!.isCompleted) panel!.complete(null);
      if (core.release != null && !core.release!.isCompleted) {
        core.release!.complete();
      }
      await tester.pumpWidget(const SizedBox.shrink());
      model.dispose();
      messenger.setMockMethodCallHandler(selector, null);
      messenger.setMockMethodCallHandler(fileMenuChannel, null);
      messenger.setMockMethodCallHandler(fileAccessChannel, null);
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

    Future<void> drive(Future<void> Function() job) async {
      var done = false;
      Object? error;
      unawaited(
        Future.sync(job).then(
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

    Future<void> key(
      LogicalKeyboardKey key, {
      bool shift = false,
      bool control = false,
    }) async {
      final modifier = control
          ? LogicalKeyboardKey.controlLeft
          : LogicalKeyboardKey.metaLeft;
      await tester.sendKeyDownEvent(modifier);
      if (shift) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(key);
      if (shift) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyUpEvent(modifier);
      await tester.pump();
    }

    Future<void> native(String command) async {
      final done = Completer<void>();
      messenger.handlePlatformMessage(
        fileMenuChannel.name,
        const StandardMethodCodec().encodeMethodCall(
          MethodCall('command', command),
        ),
        (_) => done.complete(),
      );
      await done.future;
    }

    Future<void> idle() =>
        until(() => screen.fileCommandEnabled(WorkspaceFileCommand.open));
    void insert(String text) {
      screen.editor.replaceText(
        0,
        0,
        text,
        TextSelection.collapsed(offset: text.length),
      );
    }

    void focusDocument() => tester
        .state<DocumentViewState>(find.byType(DocumentView))
        .reveal(0, 0, requestFocus: true);

    // A just-started Undo with new queued typing must finish before Save Copy.
    insert('Old ');
    await drive(screen.flush);
    core.reached = Completer<void>();
    core.release = Completer<void>();
    final undo = screen.history(false);
    await until(() => core.reached!.isCompleted);
    insert('Fresh ');
    focusDocument();
    unawaited(native('saveCopy'));
    expect(panels, isEmpty);
    core.release!.complete();
    await drive(() => undo);
    await until(() => panels.length == 1);
    expect(panels.last.method, 'getSavePath');
    final id = model.workspace['id'];
    await key(LogicalKeyboardKey.keyO);
    await key(LogicalKeyboardKey.keyN);
    await drive(() => native('export'));
    expect(panels.length, 1);
    expect(model.workspace['id'], id);
    panel!.complete('${output.path}/saved.folio');
    await idle();
    final saved = jsonDecode(
      File('${output.path}/saved.folio').readAsStringSync(),
    );
    expect(saved['document'], model.workspace['document']);
    expect(
      screen.editor.document.toPlainText(),
      startsWith('Fresh Good ideas'),
    );
    expect(saved['revision'], model.revision);

    // Cancel Open leaves all content/history intact; failure also releases the gate.
    final before = jsonEncode(model.workspace);
    await key(LogicalKeyboardKey.keyO);
    await until(() => panels.length == 2);
    expect(panels.last.method, 'openFile');
    panel!.complete(null);
    await idle();
    expect(jsonEncode(model.workspace), before);
    await key(LogicalKeyboardKey.keyO, control: true);
    await until(() => panels.length == 3);
    panel!.complete(['${dir.path}/missing.folio']);
    await idle();
    expect(jsonEncode(model.workspace), before);

    // A Flutter dialog owns its draft: neither native commands nor shortcuts replace it.
    final dialog = screen.rename();
    await tester.pumpAndSettle();
    expect(menuStates.last['available'], false);
    await drive(() => native('newWorkspace'));
    await key(LogicalKeyboardKey.keyO);
    expect(model.workspace['id'], id);
    expect(panels.length, 3);
    await tester.tap(find.text('Cancel').last);
    await drive(() => dialog);
    await idle();
    expect(menuStates.last['available'], true);

    // Real DOCX import through the native menu bridge creates a recoverable snapshot.
    unawaited(native('open'));
    await until(() => panels.length == 4);
    panel!.complete([
      '${Directory.current.path}/test/fixtures/word-outline-levels.docx',
    ]);
    await until(
      () => find.text('Save changes to your files?').evaluate().isNotEmpty,
    );
    expect(find.text('This document (not saved to a file)'), findsOneWidget);
    await tester.tap(find.text('Don’t Save').last);
    await idle();
    expect(screen.editor.document.toPlainText(), contains('Direct level one'));
    expect(
      jsonDecode(latestRecoveryFor(dir, id).readAsStringSync())['document'],
      saved['document'],
    );
    focusDocument();
    await key(LogicalKeyboardKey.keyE, shift: true);
    await until(() => panels.length == 5);
    expect((panels.last.arguments as Map)['suggestedName'], endsWith('.docx'));
    panel!.complete('${output.path}/document.docx');
    await idle();
    expect(File('${output.path}/document.docx').existsSync(), true);

    // In-cell typing is committed before an XLSX export, and the native title follows the view.
    await drive(() => screen.switchMode(true));
    expect(menuStates.last['spreadsheet'], true);
    await tester.tap(find.byKey(const Key('cell-B4')));
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('cell-editor')), '4567');
    await key(LogicalKeyboardKey.keyE, shift: true);
    await until(() => panels.length == 6);
    expect(model.sheets.first['cells']['B4'], '4567');
    expect((panels.last.arguments as Map)['suggestedName'], endsWith('.xlsx'));
    panel!.complete('${output.path}/workbook.xlsx');
    await idle();
    expect(File('${output.path}/workbook.xlsx').existsSync(), true);

    // New saves the prior workspace; Open can recover the exact copy.
    final outgoing = jsonDecode(jsonEncode(model.workspace));
    unawaited(native('newWorkspace'));
    await until(
      () => find.text('Save changes to your files?').evaluate().isNotEmpty,
    );
    expect(find.text('This workbook (not saved to a file)'), findsOneWidget);
    await tester.tap(find.text('Don’t Save').last);
    await idle();
    expect(model.workspace['id'], isNot(outgoing['id']));
    expect(
      jsonDecode(
        latestRecoveryFor(dir, outgoing['id']).readAsStringSync(),
      )['sheets'],
      outgoing['sheets'],
    );
    unawaited(native('open'));
    await until(() => panels.length == 7);
    panel!.complete(['${output.path}/saved.folio']);
    await idle();
    expect(model.workspace['document'], saved['document']);
    expect(model.workspace['sheets'], saved['sheets']);
    expect(tester.takeException(), isNull);
  });
}
