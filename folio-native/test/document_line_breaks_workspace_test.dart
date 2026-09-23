import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_headers.dart';
import 'package:folio_native/src/document_table_view.dart';
import 'document_line_breaks_test.dart' as f;

void main() {
  testWidgets(
    'manual breaks in body, cell and header survive Apply, Cancel, history and Office recovery',
    (t) async {
      t.view.physicalSize = const Size(1440, 1100);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await t.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-line-breaks-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [
              f.lineDoc([f.word('Alpha'), f.br, f.word('Beta')])['content'][0],
              {
                'type': 'table',
                'attrs': {
                  'columnWidths': [4000],
                },
                'content': [
                  {
                    'type': 'tableRow',
                    'content': [
                      {
                        'type': 'tableCell',
                        'attrs': {'colspan': 1, 'rowspan': 1},
                        'content': f.lineDoc([
                          f.word('Left'),
                          f.br,
                          f.word('Right'),
                        ])['content'],
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
      final sheets = jsonEncode(model.workspace['sheets']);
      final original = jsonEncode(model.workspace['document']);
      final body = t.widget<q.QuillEditor>(find.byType(q.QuillEditor));
      body.focusNode.requestFocus();
      screen.editor.updateSelection(
        const TextSelection.collapsed(offset: 10),
        q.ChangeSource.local,
      );
      await t.pumpAndSettle();
      await f.shiftEnter(t);
      await drive(screen.flush);
      screen.editor.replaceText(
        11,
        0,
        'Tail',
        const TextSelection.collapsed(offset: 15),
      );
      await drive(screen.flush);
      expect(
        nodeText(model.workspace['document']['content'][0]),
        'Alpha\nBeta\nTail',
      );
      expect((model.workspace['document']['content'] as List).length, 2);
      final bodyEdited = jsonEncode(model.workspace['document']);
      await drive(() => screen.history(false));
      expect(
        nodeText(model.workspace['document']['content'][0]),
        'Alpha\nBeta\n',
      );
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), original);
      await drive(() => screen.history(true));
      await drive(() => screen.history(true));
      expect(jsonEncode(model.workspace['document']), bodyEdited);

      Future<void> editCell() async {
        final cell = find.byKey(const ValueKey('table-cell-0-0')).first;
        await t.ensureVisible(cell);
        await t.tap(cell);
        await t.pumpAndSettle();
        final editor = t.widget<q.QuillEditor>(
          find.descendant(
            of: find.byType(DocumentTableDialog),
            matching: find.byType(q.QuillEditor),
          ),
        );
        expect(editor.controller.document.toPlainText(), 'Left\u2028Right\n');
        editor.controller.updateSelection(
          const TextSelection.collapsed(offset: 10),
          q.ChangeSource.local,
        );
        editor.focusNode.requestFocus();
        await t.pumpAndSettle();
        await f.shiftEnter(t);
        editor.controller.replaceText(
          11,
          0,
          'Bottom',
          const TextSelection.collapsed(offset: 17),
        );
        await t.pumpAndSettle();
        expect((f.decoded(editor.controller)['content'] as List).length, 1);
      }

      await editCell();
      await t.tap(find.text('Cancel').last);
      await t.pumpAndSettle();
      expect(jsonEncode(model.workspace['document']), bodyEdited);
      await editCell();
      await t.tap(find.byKey(const Key('table-apply')));
      await drive(screen.flush);
      final withTable = jsonEncode(model.workspace['document']);
      expect(
        nodeText(
          model.workspace['document']['content'][1]['content'][0]['content'][0],
        ),
        'Left\nRight\nBottom',
      );
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), bodyEdited);
      await drive(() => screen.history(true));
      expect(jsonEncode(model.workspace['document']), withTable);

      await t.tap(find.text('Layout').first);
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('document-headers')));
      await drive(screen.flush);
      final header = t.widget<q.QuillEditor>(
        find.descendant(
          of: find.byType(DocumentHeadersDialog),
          matching: find.byType(q.QuillEditor),
        ),
      );
      header.controller.replaceText(
        0,
        0,
        'Header',
        const TextSelection.collapsed(offset: 6),
      );
      header.focusNode.requestFocus();
      await t.pumpAndSettle();
      await f.shiftEnter(t);
      header.controller.replaceText(
        7,
        0,
        'Second',
        const TextSelection.collapsed(offset: 13),
      );
      await t.tap(find.byKey(const Key('header-apply')));
      await drive(screen.flush);
      expect(model.error, isNull);
      final finalDoc = jsonDecode(jsonEncode(model.workspace['document']));
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), withTable);
      await drive(() => screen.history(true));
      expect(model.workspace['document'], finalDoc);
      expect(jsonEncode(model.workspace['sheets']), sheets);
      await drive(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/edited.docx',
        });
      });
      await t.runAsync(() async {
        Future<void> checkHeader(CoreBridge core) async {
          final h = await core.call({
            'op': 'documentHeaders',
            'section': 0,
            'kind': 'header',
            'variant': 'default',
          });
          expect(nodeText(h['document']), 'Header\nSecond');
          expect((h['document']['content'] as List).length, 1);
        }

        final recovered = CoreBridge();
        final saved = await recovered.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expect(saved['workspace']['document'], finalDoc);
        await checkHeader(recovered);
        await recovered.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/recovered.docx',
        });
        final reopened = CoreBridge();
        final imported = await reopened.call({
          'op': 'importOffice',
          'path': '${folder.path}/edited.docx',
        });
        final document = imported['workspace']['document'];
        expect(nodeText(document['content'][0]), 'Alpha\nBeta\nTail');
        expect(
          nodeText(document['content'][1]['content'][0]['content'][0]),
          'Left\nRight\nBottom',
        );
        expect((document['content'] as List).length, 2);
        await checkHeader(reopened);
      });
      expect(t.takeException(), isNull);
      // ignore: avoid_print
      print('LINE_BREAKS_UI_OUTPUT=${folder.path}');
    },
  );
}
