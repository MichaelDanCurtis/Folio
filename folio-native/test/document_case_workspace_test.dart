import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'Change Case and Clear All Formatting save to DOCX and undo in one step',
    (t) async {
      t.view.physicalSize = const Size(1440, 1100);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await t.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-case-ui-');
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
                  {
                    'type': 'text',
                    'text': 'loud ',
                    'marks': [
                      {'type': 'bold'},
                      {
                        'type': 'textStyle',
                        'attrs': {'color': '#C00000'},
                      },
                    ],
                  },
                  {
                    'type': 'text',
                    'text': 'link',
                    'marks': [
                      {'type': 'italic'},
                      {
                        'type': 'link',
                        'attrs': {'href': 'https://example.org/'},
                      },
                    ],
                  },
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
      final screen = t.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      addTearDown(() async {
        await t.pumpWidget(const SizedBox());
        model.dispose();
      });
      Map first() => model.workspace['document']['content'][0] as Map;
      List marks(int run) => [
        for (final m in first()['content'][run]['marks'] ?? []) m['type'],
      ];
      final original = jsonDecode(jsonEncode(model.workspace['document']));
      expect(first()['type'], 'heading');

      t
          .widget<q.QuillEditor>(find.byType(q.QuillEditor))
          .focusNode
          .requestFocus();
      screen.editor.updateSelection(
        const TextSelection(baseOffset: 0, extentOffset: 9),
        q.ChangeSource.local,
      );
      await t.pumpAndSettle();
      await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await t.sendKeyEvent(LogicalKeyboardKey.f3);
      await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await drive(screen.flush);
      expect(first()['content'].map((r) => r['text']).join(), 'Loud Link');
      expect(marks(1), containsAll(['italic', 'link']));
      await drive(() => screen.history(false));
      expect(jsonDecode(jsonEncode(model.workspace['document'])), original);

      screen.editor.updateSelection(
        const TextSelection(baseOffset: 0, extentOffset: 9),
        q.ChangeSource.local,
      );
      await t.tap(find.byKey(const Key('document-clear-formatting')));
      await drive(screen.flush);
      expect(first()['type'], 'paragraph');
      expect(marks(0), isEmpty);
      expect(marks(1), ['link']);
      expect(first()['content'][1]['text'], 'link');

      await t.runAsync(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/cleared.docx',
        });
      });

      await drive(() => screen.history(false));
      expect(jsonDecode(jsonEncode(model.workspace['document'])), original);

      // The reopened bridge shares the engine, so it is checked last.
      await t.runAsync(() async {
        final reopened = CoreBridge();
        final imported = await reopened.call({
          'op': 'importOffice',
          'path': '${folder.path}/cleared.docx',
        });
        final p = imported['workspace']['document']['content'][0] as Map;
        expect(p['type'], 'paragraph');
        expect(p['content'].map((r) => r['text']).join(), 'loud link');
        final runMarks = [
          for (final r in p['content'])
            [for (final m in r['marks'] ?? []) m['type']],
        ];
        expect(runMarks.expand((m) => m).toSet(), {'link'});
      });
      await t.pump(const Duration(seconds: 5));
    },
  );
}
