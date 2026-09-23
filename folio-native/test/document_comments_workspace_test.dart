import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_comments.dart';

void main() {
  testWidgets(
    'comments post, reply, edit, resolve, history, anchor navigation, stale state and Office recovery',
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
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-comments-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [
              {
                'type': 'paragraph',
                'content': [
                  {'type': 'text', 'text': 'A😀 target and a second passage.'},
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

      final renderKey = GlobalKey();
      await tester.pumpWidget(
        RepaintBoundary(
          key: renderKey,
          child: FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
        ),
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
      Future<void> tap(Key key) async {
        final f = find.byKey(key);
        await tester.ensureVisible(f);
        await tester.pumpAndSettle();
        await tester.tap(f);
        await settle();
      }

      Future<void> enter(Key key, String value) async {
        final f = find.byKey(key);
        await tester.ensureVisible(f);
        await tester.pumpAndSettle();
        await tester.enterText(f, value);
        await tester.pump();
      }

      Future<void> close() async {
        await tester.tap(find.text('Close'));
        await tester.pumpAndSettle();
      }

      List<Map> threads() => documentComments(model.workspace['document']);
      screen.editor.updateSelection(
        const TextSelection(baseOffset: 4, extentOffset: 10),
        q.ChangeSource.local,
      );
      await tester.tap(find.text('Review').first);
      await tester.pumpAndSettle();
      await tap(const Key('document-new-comment'));
      expect(find.byType(DocumentCommentsDialog), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(find.byKey(const Key('comment-post')))
            .onPressed,
        isNull,
      );
      await enter(const Key('comment-author'), 'Morgan');
      await enter(const Key('comment-draft'), 'Please check the forecast.');
      await tap(const Key('comment-post'));
      expect(threads().single['start'], 4);
      expect(threads().single['end'], 10);
      expect(threads().single['messages'][0]['author'], 'Morgan');
      await enter(
        const Key('comment-draft'),
        'Confirmed against the workbook.',
      );
      await tap(const Key('comment-post'));
      expect(threads().single['messages'].length, 2);
      await tap(const Key('comment-edit-1'));
      await enter(
        const Key('comment-draft'),
        'Confirmed against the attached workbook.',
      );
      await tap(const Key('comment-post'));
      expect(
        threads().single['messages'][1]['text'],
        'Confirmed against the attached workbook.',
      );
      await tap(const Key('comments-resolve'));
      expect(threads().single['resolved'], true);
      expect(find.byKey(const Key('comment-post')), findsNothing);
      await tester.tap(find.byType(DropdownButton<String>));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Active').last);
      await tester.pumpAndSettle();
      expect(
        find.text(
          'No matching comments. Select text or place the cursor in a paragraph, then choose New comment.',
        ),
        findsOneWidget,
      );
      expect(find.byKey(const Key('comments-resolve')), findsNothing);
      await tester.tap(find.byType(DropdownButton<String>));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Resolved').last);
      await tester.pumpAndSettle();
      expect(find.text('Reopen'), findsOneWidget);
      await tester.tap(find.byType(DropdownButton<String>));
      await tester.pumpAndSettle();
      await tester.tap(find.text('All').last);
      await tester.pumpAndSettle();

      await tap(const Key('comments-undo'));
      expect(threads().single['resolved'], false);
      await tap(const Key('comments-redo'));
      expect(threads().single['resolved'], true);
      await tap(const Key('comments-resolve'));
      expect(threads().single['resolved'], false);
      if (capture) {
        await tester.runAsync(() async {
          final image =
              await (renderKey.currentContext!.findRenderObject()
                      as RenderRepaintBoundary)
                  .toImage();
          final data = await image.toByteData(format: ui.ImageByteFormat.png);
          await File(
            '../../work/comments-word-widget.png',
          ).writeAsBytes(data!.buffer.asUint8List());
          image.dispose();
        });
      }
      await tap(const Key('comments-go-to'));
      expect(find.byType(DocumentCommentsDialog), findsNothing);
      expect(
        screen.editor.selection,
        const TextSelection(baseOffset: 4, extentOffset: 10),
      );
      screen.editor.replaceText(
        0,
        0,
        'New ',
        const TextSelection.collapsed(offset: 4),
      );
      await settle();
      expect(threads().single['start'], 8);
      await tap(const Key('document-comments'));
      await enter(const Key('comment-author'), 'R');
      await enter(const Key('comment-draft'), 'Unposted draft');
      await close();
      expect(find.text('Discard this draft?'), findsOneWidget);
      await tester.tap(find.text('Keep writing'));
      await tester.pumpAndSettle();
      expect(find.text('Unposted draft'), findsOneWidget);
      await enter(const Key('comment-draft'), '');
      await tap(const Key('comments-delete'));
      expect(threads(), isEmpty);
      await tap(const Key('comments-undo'));
      expect(threads(), hasLength(1));
      await close();
      // Draft anchor commands must reject a concurrent workspace mutation.
      await tap(const Key('document-comments'));
      await enter(const Key('comment-author'), 'R');
      await enter(const Key('comment-draft'), 'Stale reply');
      await drive(
        () => model.mutate({'op': 'title', 'title': 'Concurrent change'}),
      );
      final revision = model.revision;
      await tap(const Key('comment-post'));
      expect(find.textContaining('document changed'), findsOneWidget);
      expect(model.revision, revision);
      await enter(const Key('comment-draft'), '');
      await close();
      final file = '${folder.path}/review.docx';
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': file,
        });
        await model.flush();
      });
      await tester.runAsync(() async {
        final reopened = CoreBridge();
        await reopened.call({'op': 'new', 'blank': true});
        final round = await reopened.call({'op': 'importOffice', 'path': file});
        final t = documentComments(round['workspace']['document']).single;
        expect(t['start'], 8);
        expect(t['end'], 14);
        expect(t['messages'].length, 2);
        expect(t['messages'][0]['author'], 'Morgan');
        final recovered = CoreBridge();
        final recovery = await recovered.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expect(
          documentComments(
            recovery['workspace']['document'],
          ).single['messages'].length,
          2,
        );
      });
      expect(tester.takeException(), isNull);
    },
  );
}
