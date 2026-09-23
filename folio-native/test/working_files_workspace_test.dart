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

void main() {
  testWidgets('Save and Save As retain separate working files, history and recovery', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1440, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    late WorkspaceModel model;
    late Directory dir;
    final output = Directory(
      '${Directory.systemTemp.path}/folio-working-files-ui',
    );
    await tester.runAsync(() async {
      await CoreBridge.initialize();
      dir = await Directory.systemTemp.createTemp('folio-working-files-');
      await output.create(recursive: true);
      final core = CoreBridge();
      await core.call({'op': 'new'});
      model = WorkspaceModel(core, '${dir.path}/workspace.folio');
      await model.initialize();
    });
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    const selector = MethodChannel('plugins.flutter.io/file_selector');
    var panelCount = 0, leases = 0, releases = 0;
    String? chosenPath;
    final movedPaths = <String, String>{};
    messenger.setMockMethodCallHandler(selector, (call) async {
      panelCount++;
      return call.method == 'openFile'
          ? (chosenPath == null ? null : [chosenPath])
          : chosenPath;
    });
    messenger.setMockMethodCallHandler(fileAccessChannel, (call) async {
      final args = call.arguments as Map;
      if (call.method == 'capture') return args['path'];
      if (call.method == 'release') {
        releases++;
        return null;
      }
      leases++;
      final original = args['bookmark'] as String;
      final path = movedPaths[original] ?? original;
      return {'path': path, 'bookmark': path, 'lease': '$leases'};
    });
    messenger.setMockMethodCallHandler(fileMenuChannel, (_) async => null);
    Future<void> until(bool Function() ready) async {
      for (var i = 0; i < 600 && !ready(); i++) {
        await tester.pump(const Duration(milliseconds: 20));
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 5)),
        );
      }
      expect(ready(), true);
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

    Future<WorkspaceScreenState> mount() async {
      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await tester.pumpAndSettle();
      final value = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => value.filesReady);
      return value;
    }

    var screen = await mount();
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox.shrink());
      model.dispose();
      messenger.setMockMethodCallHandler(selector, null);
      messenger.setMockMethodCallHandler(fileAccessChannel, null);
      messenger.setMockMethodCallHandler(fileMenuChannel, null);
    });
    Future<void> command(WorkspaceFileCommand command) =>
        drive(() => screen.guard(() => screen.fileCommand(command)));
    void type(String text) => screen.editor.replaceText(
      0,
      0,
      text,
      TextSelection.collapsed(offset: text.length),
    );
    Future<void> saveKey({bool as = false}) async {
      tester
          .state<DocumentViewState>(find.byType(DocumentView))
          .reveal(0, 0, requestFocus: true);
      await tester.pump();
      await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
      if (as) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyS);
      if (as) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
      await until(() => screen.fileCommandEnabled(WorkspaceFileCommand.save));
    }

    final first = '${output.path}/first.docx';
    final second = '${output.path}/second.docx';
    type('First ');
    await saveKey(); // Cancel the initial Save As.
    expect(screen.activeWorkingFile, isNull);
    chosenPath = first;
    await saveKey();
    expect(screen.activeWorkingFile!.path, first);
    expect(screen.activeWorkingFile!.revision, model.revision);
    expect(screen.activeWorkingFile!.contentSignature, isNotNull);
    type('Temporary ');
    await drive(screen.flush);
    await drive(screen.flush);
    expect(screen.fileSaveStatus, startsWith('Changes saved locally'));
    await drive(() => screen.history(false));
    expect(screen.activeWorkingFile!.revision, isNot(model.revision));
    await drive(screen.flush);
    expect(
      screen.fileSaveStatus,
      startsWith('Saved ·'),
      reason: 'Undo restored the saved contents despite a later revision',
    );
    final firstSaved = File(first).readAsBytesSync();
    type('Second ');
    await saveKey();
    expect(panelCount, 2, reason: 'Normal Save reuses the chosen file');
    expect(File(first).readAsBytesSync(), isNot(firstSaved));
    final beforeAs = File(first).readAsBytesSync();
    chosenPath = second;
    await saveKey(as: true);
    expect(screen.activeWorkingFile!.path, second);
    expect(File(first).readAsBytesSync(), beforeAs);
    type('Third ');
    await saveKey();
    expect(File(first).readAsBytesSync(), beforeAs);
    expect(screen.activeWorkingFile!.revision, model.revision);

    chosenPath = '${output.path}/copy.folio';
    await command(WorkspaceFileCommand.saveCopy);
    expect(
      screen.activeWorkingFile!.path,
      second,
      reason: 'Copy must not retarget Save',
    );
    final wordBytes = File(second).readAsBytesSync();
    await drive(() => screen.switchMode(true));
    expect(screen.activeWorkingFile, isNull);
    await tester.tap(find.byKey(const Key('cell-B4')));
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('cell-editor')), '4567');
    chosenPath = '${output.path}/book.xlsx';
    await command(WorkspaceFileCommand.save);
    expect(screen.activeWorkingFile!.kind, WorkingFileKind.xlsx);
    expect(File(second).readAsBytesSync(), wordBytes);
    await drive(
      () => model.mutate({
        'op': 'cells',
        'cells': [
          {'sheetId': 'budget', 'address': 'B4', 'value': '5678'},
        ],
      }),
    );
    await drive(screen.flush);
    expect(screen.fileSaveStatus, startsWith('Changes saved locally'));
    await drive(() => screen.switchMode(false));
    await drive(screen.flush);
    expect(
      screen.fileSaveStatus,
      startsWith('Saved ·'),
      reason: 'An unrelated workbook edit must not dirty the document',
    );
    await drive(() => screen.switchMode(true));
    final panelsBefore = panelCount;
    await command(WorkspaceFileCommand.save);
    expect(panelCount, panelsBefore);
    expect(screen.activeWorkingFile!.revision, model.revision);
    expect(File(second).readAsBytesSync(), wordBytes);

    // Fresh screen/session restores the local bindings without embedding grants in .folio.
    await tester.pumpWidget(const SizedBox.shrink());
    screen = await mount();
    expect(screen.activeWorkingFile!.path, second);
    expect(
      File('${output.path}/copy.folio').readAsStringSync(),
      isNot(contains('bookmark')),
    );
    await command(WorkspaceFileCommand.save);
    expect(panelCount, panelsBefore);
    await drive(() => screen.switchMode(true));
    expect(screen.activeWorkingFile!.path, '${output.path}/book.xlsx');
    await drive(() => screen.switchMode(false));

    // A renamed file is resolved by the OS grant, then the stored path is refreshed.
    final moved = '${output.path}/renamed.docx';
    File(second).renameSync(moved);
    movedPaths[second] = moved;
    type('Moved ');
    await command(WorkspaceFileCommand.save);
    expect(screen.activeWorkingFile!.path, moved);
    expect(File(second).existsSync(), false);
    final goodMoved = File(moved).readAsBytesSync();

    // Outside changes are never silently replaced. Cancel retains both versions.
    File(moved).writeAsBytesSync([...goodMoved, 10, 32], flush: true);
    final external = File(moved).readAsBytesSync();
    type('Local ');
    var saveDone = false;
    unawaited(
      screen
          .fileCommand(WorkspaceFileCommand.save)
          .then((_) => saveDone = true),
    );
    await until(
      () => find.text('This file changed outside Folio').evaluate().isNotEmpty,
    );
    await tester.tap(find.text('Cancel').last);
    await until(() => saveDone);
    expect(File(moved).readAsBytesSync(), external);
    await drive(screen.flush);
    expect(screen.fileSaveStatus, startsWith('Changes saved locally'));
    expect(leases, releases);

    chosenPath = '${output.path}/both-versions.docx';
    saveDone = false;
    unawaited(
      screen
          .fileCommand(WorkspaceFileCommand.save)
          .then((_) => saveDone = true),
    );
    await until(
      () => find.text('This file changed outside Folio').evaluate().isNotEmpty,
    );
    await tester.tap(find.text('Save As…').last);
    await until(() => saveDone);
    expect(File(moved).readAsBytesSync(), external);
    expect(screen.activeWorkingFile!.path, chosenPath);

    // A failed write must neither retarget Save nor mark a pending edit saved.
    final beforeFailure = screen.activeWorkingFile!;
    type('Failure retained ');
    chosenPath = '${dir.path}/occupied.docx';
    Directory(chosenPath).createSync();
    await command(WorkspaceFileCommand.saveAs);
    expect(screen.activeWorkingFile, same(beforeFailure));
    await drive(screen.flush);
    expect(screen.fileSaveStatus, startsWith('Changes saved locally'));
    expect(Directory(chosenPath).existsSync(), true);
    await command(WorkspaceFileCommand.save);
    expect(screen.activeWorkingFile!.revision, model.revision);

    // Structural edits replace preserved Office packages without changing the
    // working file identity. Save must continue to target the original workbook.
    await drive(() => screen.switchMode(true));
    final workbookPath = screen.activeWorkingFile!.path;
    final source = screen.activeWorkingFile!.sourceKey;
    await drive(
      () => model.mutate({
        'op': 'structure',
        'sheetId': screen.sheetId,
        'action': 'insertRows',
        'at': 2,
        'count': 1,
      }),
    );
    expect(screen.activeWorkingFile!.path, workbookPath);
    expect(
      WorkingFiles.sourceKey(model.workspace, WorkingFileKind.xlsx),
      source,
    );
    await drive(screen.flush);
    expect(screen.fileSaveStatus, startsWith('Changes saved locally'));
    await drive(() => screen.history(false));
    await drive(screen.flush);
    expect(screen.fileSaveStatus, startsWith('Saved ·'));
    await drive(() => screen.switchMode(false));

    // Opening a native workspace binds both views to that workspace file.
    chosenPath = '${output.path}/copy.folio';
    await command(WorkspaceFileCommand.open);
    expect(screen.activeWorkingFile!.kind, WorkingFileKind.folio);
    type('Workspace ');
    await command(WorkspaceFileCommand.save);
    final copy = jsonDecode(File(chosenPath).readAsStringSync());
    expect(copy['document'], model.workspace['document']);
    expect(copy['sheets'], model.workspace['sheets']);
    await drive(() => screen.switchMode(true));
    expect(screen.activeWorkingFile!.kind, WorkingFileKind.folio);
    await command(WorkspaceFileCommand.newWorkspace);
    expect(screen.activeWorkingFile, isNull);
    expect(leases, releases);
    expect(tester.takeException(), isNull);
  });
}
