import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_bookmarks.dart';
import 'package:folio_native/src/document_bookmarks_dialog.dart';
import 'package:folio_native/src/document_table_view.dart';

void main() {
  testWidgets(
    'bookmark add, exact rapid typing, Go To, sorting, deletion, history, stale state, export and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-bookmark-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [
              {
                'type': 'paragraph',
                'content': [
                  {
                    'type': 'text',
                    'text': 'aaaaaa',
                    'marks': [
                      {'type': 'bold'},
                    ],
                  },
                ],
              },
              {
                'type': 'paragraph',
                'content': [
                  {
                    'type': 'text',
                    'text': 'A second paragraph for navigation.',
                  },
                ],
              },
            ],
          },
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
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
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
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
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
      });
      Future<void> settle() => drive(screen.flush);
      Future<void> open() async {
        await tester.tap(find.byKey(const Key('document-bookmarks')));
        await settle();
        expect(find.byType(DocumentBookmarksDialog), findsOneWidget);
      }

      void select(int a, int b) => screen.editor.updateSelection(
        TextSelection(baseOffset: a, extentOffset: b),
        q.ChangeSource.local,
      );
      Map mark() => documentBookmarks(
        model.workspace['document'],
      ).singleWhere((b) => b['name'] == 'Middle');
      select(2, 4);
      final originalText = screen.editor.document.toPlainText();
      final initialRevision = model.revision;
      await open();
      expect(
        tester
            .widget<DocumentBookmarksDialog>(
              find.byType(DocumentBookmarksDialog),
            )
            .selection,
        const TextSelection(baseOffset: 2, extentOffset: 4),
      );
      await tester.enterText(find.byKey(const Key('bookmark-name')), '1 bad');
      await tester.pump();
      await tester.tap(find.byKey(const Key('bookmark-add')));
      await settle();
      expect(find.byKey(const Key('bookmark-error')), findsOneWidget);
      expect(model.revision, initialRevision);
      await tester.enterText(find.byKey(const Key('bookmark-name')), 'Middle');
      await tester.pump();
      await tester.tap(find.byKey(const Key('bookmark-add')));
      await settle();
      expect(mark()['start'], 2);
      expect(mark()['end'], 4);
      final addedRevision = model.revision;
      await tester.tap(find.byKey(const Key('bookmark-go-to')));
      await settle();
      expect(
        screen.editor.selection,
        const TextSelection(baseOffset: 2, extentOffset: 4),
      );
      expect(
        model.revision,
        addedRevision,
        reason: 'Navigation must not add a document Undo step',
      );
      expect(screen.editor.document.toPlainText(), originalText);
      // Two edits before the engine responds: matching letters make inferred diffs ambiguous.
      screen.editor.replaceText(
        0,
        0,
        'a',
        const TextSelection.collapsed(offset: 1),
      );
      screen.editor.replaceText(
        0,
        0,
        'a',
        const TextSelection.collapsed(offset: 1),
      );
      await settle();
      expect(mark()['start'], 4);
      expect(mark()['end'], 6);
      screen.editor.replaceText(
        4,
        0,
        'a',
        const TextSelection.collapsed(offset: 5),
      );
      await settle();
      expect(mark()['start'], 4);
      expect(mark()['end'], 7);
      screen.editor.replaceText(
        7,
        0,
        'a',
        const TextSelection.collapsed(offset: 8),
      );
      await settle();
      expect(
        mark()['end'],
        7,
        reason: 'Insertion at the ending is outside the bookmark',
      );
      final edited = jsonEncode(model.workspace['document']);
      await drive(() => screen.history(false));
      await drive(() => screen.history(true));
      expect(jsonEncode(model.workspace['document']), edited);
      // A revision change while the dialog is open invalidates its actions.
      await open();
      await tester.enterText(find.byKey(const Key('bookmark-name')), 'Stale');
      await tester.pump();
      await drive(
        () => model.mutate({'op': 'title', 'title': 'Changed while open'}),
      );
      final concurrentRevision = model.revision;
      await tester.tap(find.byKey(const Key('bookmark-add')));
      await settle();
      expect(find.textContaining('document changed'), findsOneWidget);
      expect(model.revision, concurrentRevision);
      await tester.tap(find.text('Close'));
      await tester.pumpAndSettle();
      // Hidden names import through workspace state; ordinary authoring cannot create them.
      await drive(() async {
        final state =
            jsonDecode(jsonEncode(model.workspace)) as Map<String, dynamic>;
        state['document']['attrs']['bookmarks'].add({
          'name': '_Hidden',
          'start': 0,
          'end': 0,
        });
        await model.mutate({
          'op': 'replace',
          'workspace': state,
        }, refreshDocument: true);
      });
      await open();
      expect(find.byKey(const Key('bookmark-_Hidden')), findsNothing);
      await tester.tap(find.byKey(const Key('bookmark-show-hidden')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('bookmark-_Hidden')), findsOneWidget);
      await tester.tap(find.text('Location'));
      await tester.pumpAndSettle();
      final hiddenY = tester
          .getTopLeft(find.byKey(const Key('bookmark-_Hidden')))
          .dy;
      expect(
        hiddenY,
        lessThan(
          tester.getTopLeft(find.byKey(const Key('bookmark-Middle'))).dy,
        ),
      );
      await tester.tap(find.byKey(const Key('bookmark-Middle')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('bookmark-delete')));
      await settle();
      expect(documentBookmarks(model.workspace['document']).length, 1);
      await tester.tap(find.text('Close'));
      await tester.pumpAndSettle();
      await drive(() => screen.history(false));
      expect(mark()['end'], 7);
      // Ctrl+G opens bookmarks without altering Command+G's existing Find Next binding.
      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyG);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await settle();
      expect(find.byType(DocumentBookmarksDialog), findsOneWidget);
      await tester.tap(find.text('Close'));
      await tester.pumpAndSettle();
      final saved = jsonEncode(model.workspace);
      late Map reopenedWorkspace, round;
      await drive(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/bookmarks.docx',
        });
        final reopened = await model.core.call({
          'op': 'load',
          'path': model.recoveryPath,
        });
        reopenedWorkspace = reopened['workspace'];
        await model.core.call({
          'op': 'importOffice',
          'path': '${folder.path}/bookmarks.docx',
        });
        round = (await model.core.call({'op': 'get'}))['workspace']['document'];
      });
      expect(jsonEncode(reopenedWorkspace), saved);
      final imported = documentBookmarks(
        round,
      ).singleWhere((b) => b['name'] == 'Middle');
      expect(imported['start'], 4);
      expect(imported['end'], 7);
      expect(
        DocumentBookmarkIndex(round).text,
        DocumentBookmarkIndex(model.workspace['document']).text,
      );
      expect(
        screen.editor.document.toDelta().toJson().first['attributes']['bold'],
        true,
      );
      Map paragraph(String text) => {
        'type': 'paragraph',
        'content': [
          {'type': 'text', 'text': text},
        ],
      };
      Future<void> replaceBody(Map document) async {
        final state =
            jsonDecode(jsonEncode(model.workspace)) as Map<String, dynamic>;
        state['document'] = document;
        state.remove('officeDocument');
        await model.mutate({
          'op': 'replace',
          'workspace': state,
        }, refreshDocument: true);
      }

      await drive(
        () => replaceBody({
          'type': 'doc',
          'attrs': {
            'bookmarks': [
              {'name': 'InCell', 'start': 11, 'end': 17},
            ],
          },
          'content': [
            paragraph('Intro'),
            {
              'type': 'table',
              'content': [
                {
                  'type': 'tableRow',
                  'content': [
                    {
                      'type': 'tableCell',
                      'content': [paragraph('Cell target tail')],
                    },
                  ],
                },
              ],
            },
            paragraph('End'),
          ],
        }),
      );
      final beforeTableGoTo = model.revision;
      await open();
      await tester.tap(find.byKey(const Key('bookmark-InCell')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('bookmark-go-to')));
      await settle();
      expect(find.byType(DocumentTableDialog), findsOneWidget);
      final tableEditor = tester
          .widget<q.QuillEditor>(
            find.byKey(const ValueKey('table-rich-editor-0')),
          )
          .controller;
      expect(
        tableEditor.selection,
        const TextSelection(baseOffset: 5, extentOffset: 11),
      );
      expect(tableEditor.document.toPlainText().substring(5, 11), 'target');
      await tester.tap(find.text('Cancel'));
      await settle();
      expect(model.revision, beforeTableGoTo);
      // A real spelling correction before a bookmark, followed by search replacing
      // the entire bookmarked occurrence, exercises both exact-edit clients.
      await drive(
        () => replaceBody({
          'type': 'doc',
          'attrs': {
            'bookmarks': [
              {'name': 'Second', 'start': 19, 'end': 27},
            ],
          },
          'content': [paragraph('We recieved it. We recieved it.')],
        }),
      );
      await drive(screen.proofreading.checkNow);
      for (var i = 0; i < 50 && !screen.proofreading.isCurrent(); i++) {
        await tester.pump(const Duration(milliseconds: 100));
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 20)),
        );
      }
      expect(screen.proofreading.error, isNull);
      expect(screen.proofreading.isCurrent(), isTrue);
      expect(
        screen.proofreading.issues.any(
          (i) =>
              i.start == 3 &&
              i.suggestions.any((s) => s.toLowerCase() == 'received'),
        ),
        isTrue,
        reason:
            '${screen.proofreading.checkedText}: ${screen.proofreading.issues.map((i) => '${i.start}:${i.end} ${i.text} ${i.suggestions}').join('; ')}',
      );
      final issue = screen.proofreading.issues.firstWhere(
        (i) =>
            i.start == 3 &&
            i.suggestions.any((s) => s.toLowerCase() == 'received'),
      );
      final correction = issue.suggestions.firstWhere(
        (s) => s.toLowerCase() == 'received',
      );
      await drive(() => screen.fixWriting(issue, correction));
      expect(
        screen.editor.document.toPlainText(),
        'We $correction it. We recieved it.\n',
      );
      expect(
        documentBookmarks(model.workspace['document']).single['start'],
        19,
      );
      await drive(() => screen.showSearch(true));
      await tester.enterText(find.byKey(const Key('find-query')), 'recieved');
      await tester.enterText(
        find.byKey(const Key('replace-text')),
        'confirmed',
      );
      await tester.pump(const Duration(milliseconds: 350));
      await settle();
      await drive(() => screen.searchKey.currentState!.replaceMatches(true));
      expect(
        screen.editor.document.toPlainText(),
        'We $correction it. We confirmed it.\n',
      );
      expect(documentBookmarks(model.workspace['document']), isEmpty);
      await drive(() => screen.history(false));
      expect(documentBookmarks(model.workspace['document']).single['end'], 27);
      expect(tester.takeException(), isNull);
    },
  );
}
