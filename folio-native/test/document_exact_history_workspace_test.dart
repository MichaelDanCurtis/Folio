import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'history retains original insertion sites in repeated text and paragraphs',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        folder = await Directory.systemTemp.createTemp('folio-exact-history-');
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

      for (final (body, added) in [
        ('aaaa', 'a'),
        ('😀😀', '😀'),
        ('Line\nLine', 'Line\n'),
      ]) {
        await drive(model.newWorkspace);
        await drive(
          () => model.mutate({
            'op': 'document',
            'document': {
              'type': 'doc',
              'content': [
                for (final text in [...body.split('\n'), 'After'])
                  {
                    'type': 'paragraph',
                    'content': [
                      {'type': 'text', 'text': text},
                    ],
                  },
              ],
            },
          }, refreshDocument: true),
        );
        screen.editor.updateSelection(
          const TextSelection.collapsed(offset: 0),
          q.ChangeSource.local,
        );
        insert(added);
        await drive(screen.flush);
        expect(screen.editor.document.toPlainText(), '$added$body\nAfter\n');
        final undo = screen.history(false);
        insert('X');
        await drive(() async {
          await undo;
          await screen.flush();
        });
        expect(
          screen.editor.document.toPlainText(),
          'X$body\nAfter\n',
          reason:
              'Undo must remove the original leading insertion, not matching text at the end',
        );
        expect(screen.editor.selection.extentOffset, 1);
        await drive(() => screen.history(false));
        expect(screen.editor.document.toPlainText(), '$body\nAfter\n');
        expect(screen.editor.selection.extentOffset, 0);
        final redo = screen.history(true);
        insert('Y');
        await drive(() async {
          await redo;
          await screen.flush();
        });
        expect(screen.editor.document.toPlainText(), 'XY$body\nAfter\n');
        expect(screen.editor.selection.extentOffset, 2);
        final artifacts = Directory(
          '${Directory.systemTemp.path}/folio-exact-history-tests',
        );
        final name = body == 'aaaa'
            ? 'ascii'
            : body == '😀😀'
            ? 'unicode'
            : 'paragraphs';
        await tester.runAsync(() => artifacts.create(recursive: true));
        await drive(() async {
          await model.saveCopy('${artifacts.path}/$name.folio');
          await model.core.call({
            'op': 'exportOffice',
            'format': 'docx',
            'path': '${artifacts.path}/$name.docx',
          });
        });
        final document = model.workspace['document'];
        await drive(() => model.open('${artifacts.path}/$name.folio'));
        expect(model.workspace['document'], document);
        expect(screen.editor.document.toPlainText(), 'XY$body\nAfter\n');
      }
      await drive(screen.flush);
      expect(tester.takeException(), isNull);
    },
    timeout: const Timeout(Duration(minutes: 2)),
  );
}
