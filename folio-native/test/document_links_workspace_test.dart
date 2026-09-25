import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_bookmarks.dart';
import 'package:folio_native/src/document_view.dart';

void main() {
  testWidgets('a link to a heading bookmarks it, follows and saves to DOCX', (
    t,
  ) async {
    t.view.physicalSize = const Size(1440, 1100);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.resetPhysicalSize);
    addTearDown(t.view.resetDevicePixelRatio);
    late WorkspaceModel model;
    late Directory folder;
    await t.runAsync(() async {
      await CoreBridge.initialize();
      folder = await Directory.systemTemp.createTemp('folio-links-ui-');
      final core = CoreBridge();
      await core.call({'op': 'new', 'blank': true});
      await core.call({
        'op': 'document',
        'document': {
          'type': 'doc',
          'content': [
            {
              'type': 'heading',
              'attrs': {'level': 1},
              'content': [
                {'type': 'text', 'text': 'Intro'},
              ],
            },
            {
              'type': 'paragraph',
              'content': [
                {'type': 'text', 'text': 'Back to top'},
              ],
            },
          ],
        },
      });
      await core.call({
        'op': 'exportOffice',
        'format': 'docx',
        'path': '${folder.path}/source.docx',
      });
      await core.call({
        'op': 'importOffice',
        'path': '${folder.path}/source.docx',
      });
      model = WorkspaceModel(core, '${folder.path}/workspace.folio');
      await model.initialize();
      await model.flush();
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
      for (var i = 0; i < 500; i++) {
        await t.pump(const Duration(milliseconds: 20));
        await t.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 6)),
        );
        if (done && find.byType(LinearProgressIndicator).evaluate().isEmpty) {
          break;
        }
      }
      if (error != null) throw error!;
      expect(done, true);
      await t.pumpAndSettle();
    }

    await t.pumpWidget(
      FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
    );
    await t.pumpAndSettle();
    final screen = t.state<WorkspaceScreenState>(find.byType(WorkspaceScreen));
    addTearDown(() async {
      await t.pumpWidget(const SizedBox());
      model.dispose();
    });
    t
        .widget<q.QuillEditor>(find.byType(q.QuillEditor))
        .focusNode
        .requestFocus();
    screen.editor.updateSelection(
      const TextSelection(baseOffset: 6, extentOffset: 17),
      q.ChangeSource.local,
    );
    await t.pumpAndSettle();
    await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await t.sendKeyEvent(LogicalKeyboardKey.keyK);
    await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await drive(() async {});
    expect(find.text('Insert hyperlink'), findsOneWidget);
    await t.tap(find.text('Place in this document'));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('link-place-0')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('link-apply')));
    await drive(screen.flush);

    final document = model.workspace['document'] as Map;
    expect(documentBookmarks(document).single, {
      'name': '_Intro',
      'start': 0,
      'end': 5,
    });
    final run = document['content'][1]['content'][0] as Map;
    expect(run['text'], 'Back to top');
    expect(run['marks'], [
      {
        'type': 'link',
        'attrs': {'href': '#_Intro'},
      },
    ]);

    await drive(
      () => t
          .state<DocumentViewState>(find.byType(DocumentView))
          .followLink('#_Intro'),
    );
    expect(
      screen.editor.selection,
      const TextSelection(baseOffset: 0, extentOffset: 5),
    );

    await drive(screen.flush);
    await t.runAsync(() async {
      await model.core.call({
        'op': 'exportOffice',
        'format': 'docx',
        'path': '${folder.path}/linked.docx',
      });
      final reopened = CoreBridge();
      final imported = await reopened.call({
        'op': 'importOffice',
        'path': '${folder.path}/linked.docx',
      });
      final doc = imported['workspace']['document'] as Map;
      expect(
        documentBookmarks(doc).map((b) => [b['name'], b['start'], b['end']]),
        [
          ['_Intro', 0, 5],
        ],
      );
      expect(doc['content'][1]['content'][0]['marks'], [
        {
          'type': 'link',
          'attrs': {'href': '#_Intro'},
        },
      ]);
    });
    await t.pump(const Duration(seconds: 5));
  });
}
