import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_bookmarks.dart';

class _DelayedHistoryCore extends CoreBridge {
  String blockedOperation = 'undo';
  Completer<void>? undoReached;
  Completer<void>? releaseUndo;
  bool rejectNextUndo = false, rejectNextDocument = false;

  @override
  Future<T> transaction<T>(Future<T> Function(CoreCall dispatch) action) =>
      super.transaction(
        (dispatch) => action((request) async {
          if (request['op'] == blockedOperation && releaseUndo != null) {
            undoReached!.complete();
            await releaseUndo!.future;
            releaseUndo = null;
          }
          if (request['op'] == 'undo' && rejectNextUndo) {
            rejectNextUndo = false;
            throw StateError('Injected history failure');
          }
          if (request['op'] == 'document' && rejectNextDocument) {
            rejectNextDocument = false;
            throw StateError('Injected document failure');
          }
          return dispatch(request);
        }),
      );
}

void main() {
  var initialized = false;
  for (final timing in ['immediate', 'saving', 'dispatch']) {
    testWidgets(
      'typing across Undo ($timing) is retained in editor, history and recovery',
      (tester) async {
        tester.view.physicalSize = const Size(1440, 1000);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        late WorkspaceModel model;
        late Directory folder;
        late _DelayedHistoryCore core;
        await tester.runAsync(() async {
          if (!initialized) {
            await CoreBridge.initialize();
            initialized = true;
          }
          core = _DelayedHistoryCore();
          folder = await Directory.systemTemp.createTemp(
            'folio-history-input-',
          );
          await core.call({'op': 'new', 'blank': true});
          await core.call({
            'op': 'document',
            'document': {
              'type': 'doc',
              'content': [
                {
                  'type': 'paragraph',
                  'content': [
                    {'type': 'text', 'text': 'Body'},
                  ],
                },
                {
                  'type': 'paragraph',
                  'content': [
                    {'type': 'text', 'text': 'After'},
                  ],
                },
              ],
            },
          });
          final initial = await core.call({'op': 'get'});
          await core.call({
            'op': 'bookmark',
            'action': 'set',
            'name': 'After',
            'start': 5,
            'end': 10,
            'baseRevision': initial['workspace']['revision'],
          });
          model = WorkspaceModel(core, '${folder.path}/recovery.folio');
          await model.initialize();
        });
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
          for (var i = 0; i < 500 && !done; i++) {
            await tester.pump(const Duration(milliseconds: 20));
            await tester.runAsync(
              () => Future<void>.delayed(const Duration(milliseconds: 5)),
            );
          }
          if (error != null) throw error!;
          expect(done, isTrue);
          await tester.pumpAndSettle();
        }

        await tester.pumpWidget(
          FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
        );
        await tester.pumpAndSettle();
        final screen = tester.state<WorkspaceScreenState>(
          find.byType(WorkspaceScreen),
        );
        addTearDown(() async {
          core.releaseUndo?.complete();
          await tester.pumpWidget(const SizedBox.shrink());
          model.dispose();
          await tester.runAsync(() => folder.delete(recursive: true));
        });
        void insert(String text) {
          final at = screen.editor.selection.extentOffset;
          screen.editor.replaceText(
            at,
            0,
            text,
            TextSelection.collapsed(offset: at + text.length),
          );
        }

        screen.editor.updateSelection(
          const TextSelection.collapsed(offset: 4),
          q.ChangeSource.local,
        );
        insert(' old');
        await drive(screen.flush);
        expect(screen.editor.document.toPlainText(), 'Body old\nAfter\n');
        if (timing != 'immediate') {
          core.blockedOperation = timing == 'saving' ? 'save' : 'undo';
          core.undoReached = Completer<void>();
          core.releaseUndo = Completer<void>();
        }
        final undo = screen.history(false);
        if (timing != 'immediate') await drive(() => core.undoReached!.future);
        insert(' new');
        core.releaseUndo?.complete();
        await drive(() => undo);
        await drive(screen.flush);
        expect(screen.editor.document.toPlainText(), 'Body new\nAfter\n');
        expect(screen.editor.selection.extentOffset, 8);
        final saved = jsonDecode(File(model.recoveryPath).readAsStringSync());
        expect(
          saved['document']['content'][0]['content'][0]['text'],
          'Body new',
        );
        expect(model.workspace['document'], saved['document']);
        void checkAfterBookmark(int start) {
          final mark = documentBookmarks(model.workspace['document']).single;
          expect(
            [mark['name'], mark['start'], mark['end']],
            ['After', start, start + 5],
          );
          expect(
            model.workspace['document']['content'][1]['content'][0]['text'],
            'After',
          );
        }

        checkAfterBookmark(9);
        await drive(() => screen.history(false));
        expect(screen.editor.document.toPlainText(), 'Body\nAfter\n');
        await drive(() => screen.history(true));
        expect(screen.editor.document.toPlainText(), 'Body new\nAfter\n');
        // Interleave multiple history commands and optimistic edits without
        // waiting for any engine response. They must execute in intent order.
        final first = screen.history(false);
        insert('A');
        final second = screen.history(false);
        insert('B');
        await drive(() async {
          await first;
          await second;
          await screen.flush();
        });
        expect(screen.editor.document.toPlainText(), 'BodyB\nAfter\n');
        checkAfterBookmark(6);
        await drive(() => screen.history(false));
        expect(screen.editor.document.toPlainText(), 'Body\nAfter\n');
        final redo = screen.history(true);
        insert('🚀');
        await drive(() async {
          await redo;
          await screen.flush();
        });
        expect(screen.editor.document.toPlainText(), 'BodyB🚀\nAfter\n');
        expect(screen.editor.selection.extentOffset, 7);
        checkAfterBookmark(8);

        // A rejected history request does not discard already visible typing.
        core.rejectNextUndo = true;
        Object? failure;
        final rejected = screen.history(false).catchError((Object e) {
          failure = e;
        });
        insert('!');
        await drive(() async {
          await rejected;
          await screen.flush();
        });
        expect(failure.toString(), contains('Injected history failure'));
        expect(screen.editor.document.toPlainText(), 'BodyB🚀!\nAfter\n');
        checkAfterBookmark(9);
        await drive(() => screen.history(false));
        expect(screen.editor.document.toPlainText(), 'BodyB🚀\nAfter\n');
        await drive(screen.flush);
        // A failed deferred write leaves its draft visible, refuses to report a
        // successful save, and cannot be erased by another history shortcut.
        core.rejectNextDocument = true;
        final beforeRejectedEdit = screen.history(false);
        insert('X');
        await drive(() => beforeRejectedEdit);
        Object? saveFailure;
        await drive(() async {
          try {
            await screen.flush();
          } catch (e) {
            saveFailure = e;
          }
        });
        expect(saveFailure.toString(), contains('Injected document failure'));
        expect(model.saveStatus, 'Document changes not saved');
        final draft = screen.editor.document.toPlainText();
        expect(draft, contains('X'));
        Object? historyFailure;
        await drive(() async {
          try {
            await screen.history(false);
          } catch (e) {
            historyFailure = e;
          }
        });
        expect(historyFailure, isNotNull);
        expect(screen.editor.document.toPlainText(), draft);
        insert('Y');
        await drive(screen.flush);
        expect(screen.editor.document.toPlainText(), 'BodyBXY\nAfter\n');
        checkAfterBookmark(8);
        await drive(() => screen.history(false));
        expect(screen.editor.document.toPlainText(), 'BodyBX\nAfter\n');
        await drive(() => screen.history(false));
        expect(screen.editor.document.toPlainText(), 'BodyB\nAfter\n');
        await drive(() => screen.history(true));
        await drive(() => screen.history(true));
        expect(screen.editor.document.toPlainText(), 'BodyBXY\nAfter\n');
        await drive(screen.flush);
        final artifacts = Directory(
          '${Directory.systemTemp.path}/folio-history-input-tests',
        );
        await tester.runAsync(() => artifacts.create(recursive: true));
        final name = timing;
        await drive(() async {
          await model.saveCopy('${artifacts.path}/$name.folio');
          await model.core.call({
            'op': 'exportOffice',
            'format': 'docx',
            'path': '${artifacts.path}/$name.docx',
          });
        });
        final finalDocument = model.workspace['document'];
        await drive(() => model.open('${artifacts.path}/$name.folio'));
        expect(model.workspace['document'], finalDocument);
        expect(screen.editor.document.toPlainText(), 'BodyBXY\nAfter\n');
        checkAfterBookmark(8);
        expect(tester.takeException(), isNull);
      },
      timeout: const Timeout(Duration(minutes: 2)),
    );
  }
}
