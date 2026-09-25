import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'the status bar opens Word Count for the document and the selection',
    (t) async {
      t.view.physicalSize = const Size(1440, 1100);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await t.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-word-count-ui-');
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
      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 500 && !ready(); i++) {
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 6)),
          );
        }
        expect(ready(), true);
      }

      await t.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await t.pumpAndSettle();
      final screen = t.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      addTearDown(() async {
        await t.pumpWidget(const SizedBox());
        model.dispose();
      });
      String value(String key) => t.widget<Text>(find.byKey(Key(key))).data!;
      await t.tap(find.byKey(const Key('status-word-count')));
      await until(
        () =>
            find.byKey(const Key('word-count-pages')).evaluate().isNotEmpty &&
            value('word-count-pages') != '…',
      );
      expect(value('word-count-pages'), '1');
      expect(value('word-count-lines'), '2');
      expect(value('word-count-words'), '4');
      expect(value('word-count-characters'), '14');
      expect(value('word-count-characters-spaces'), '16');
      expect(value('word-count-paragraphs'), '2');
      await t.tap(find.byKey(const Key('word-count-close')));
      await t.pumpAndSettle();

      screen.editor.updateSelection(
        const TextSelection(baseOffset: 6, extentOffset: 13),
        q.ChangeSource.local,
      );
      await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await t.sendKeyEvent(LogicalKeyboardKey.keyG);
      await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await until(() => find.text('Selection').evaluate().isNotEmpty);
      expect(value('word-count-words'), '2');
      expect(value('word-count-characters-spaces'), '7');
      await t.tap(find.byKey(const Key('word-count-close')));
      await t.pumpAndSettle();

      // The status bar's zoom control drives the page.
      await t.tap(find.byKey(const Key('zoom-in')));
      await t.pumpAndSettle();
      expect(screen.documentZoom.value, 125);
      await t.tap(find.byKey(const Key('zoom-percent')));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('zoom-page-width')));
      await t.pumpAndSettle();
      expect(screen.documentZoom.value, isNot(125));
      expect(find.text('${screen.documentZoom.value}%'), findsOneWidget);
    },
  );
}
