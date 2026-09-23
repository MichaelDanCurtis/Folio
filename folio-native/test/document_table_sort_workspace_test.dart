import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_bookmarks.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_table_view.dart';
import 'document_table_sort_test.dart' as f;

void main() {
  testWidgets(
    'table sorting applies one draft with range relocation, exact history, DOCX and recovery',
    (t) async {
      t.view.physicalSize = const Size(1440, 1100);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      const channel = MethodChannel('folio/text-sort');
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(
        channel,
        (call) async => f.fixtureRanks(
          [
            for (final column in call.arguments['values'])
              List<String>.from(column),
          ],
          call.arguments['locale'],
          call.arguments['caseSensitive'],
        ),
      );
      addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
      late Directory folder;
      late WorkspaceModel model;
      await t.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-table-sort-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        final table = f.sortTable([
          ['Name', 'Amount', 'Date'],
          ['Zulu', '10', '2026-02-01'],
          ['Alpha', '2', '2026-01-01'],
          ['Beta', '2', '2026-03-01'],
          ['Alpha', '2', '2026-02-01'],
        ]);
        table['content'][0]['attrs']['header'] = true;
        Map p(String text) => {
          'type': 'paragraph',
          'content': [
            {'type': 'text', 'text': text},
          ],
        };
        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [p('Before table'), table, p('After table')],
          },
        });
        await core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/source.docx',
        });
        var response = await core.call({
          'op': 'importOffice',
          'path': '${folder.path}/source.docx',
        });
        // Core responses include the complete workspace through snapshot.
        var workspace = response['workspace'] as Map;
        final start = DocumentBookmarkIndex(
          workspace['document'],
        ).rangeAtPath([1, 1, 0, 0])!.start;
        response = await core.call({
          'op': 'bookmark',
          'workspaceId': workspace['id'],
          'baseRevision': workspace['revision'],
          'action': 'set',
          'name': 'ZuluMark',
          'start': start,
          'end': start + 4,
        });
        workspace = response['workspace'];
        await core.call({
          'op': 'documentComment',
          'workspaceId': workspace['id'],
          'baseRevision': workspace['revision'],
          'action': 'add',
          'author': 'Test reviewer',
          'text': 'Keep this row',
          'start': start,
          'end': start + 4,
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

      await t.pumpWidget(FolioApp(model: model, agentDirectory: folder.path));
      await t.pumpAndSettle();
      final screen = t.state(find.byType(WorkspaceScreen)) as dynamic;
      addTearDown(() async {
        await t.pumpWidget(const SizedBox());
        model.dispose();
      });
      final original = jsonEncode(model.workspace);
      final originalDoc = jsonEncode(model.workspace['document']);
      q.QuillEditor editor() => t.widget<q.QuillEditor>(
        find.descendant(
          of: find.byType(DocumentTableDialog),
          matching: find.byType(q.QuillEditor),
        ),
      );
      Future<void> openAndEdit() async {
        final cell = find.byKey(const ValueKey('table-cell-1-0')).first;
        await t.ensureVisible(cell);
        await t.tap(cell);
        await t.pumpAndSettle();
        final e = editor();
        expect(e.controller.document.toPlainText(), 'Zulu\n');
        e.controller.replaceText(
          4,
          0,
          '!',
          const TextSelection.collapsed(offset: 5),
        );
        await t.pumpAndSettle();
        await t.tap(find.byKey(const Key('table-sort')));
        await t.pumpAndSettle();
      }

      Future<void> choose(String key, String label) async {
        await t.tap(find.byKey(Key(key)));
        await t.pumpAndSettle();
        await t.tap(find.text(label).last);
        await t.pumpAndSettle();
      }

      await openAndEdit();
      await t.tap(find.text('Cancel').last);
      await t.pumpAndSettle();
      expect(editor().controller.document.toPlainText(), 'Zulu!\n');
      await t.tap(find.text('Cancel').last);
      await t.pumpAndSettle();
      expect(jsonEncode(model.workspace), original);
      await openAndEdit();
      await choose('sort-column-0', '2: Amount');
      await choose('sort-type-0', 'Number');
      await choose('sort-column-1', '1: Name');
      await choose('sort-column-2', '3: Date');
      await choose('sort-type-2', 'Date');
      await t.tap(find.byKey(const Key('sort-table-apply')));
      await t.pumpAndSettle();
      expect(jsonEncode(model.workspace), original);
      expect(editor().controller.document.toPlainText(), 'Zulu!\n');
      editor().controller.replaceText(
        5,
        0,
        '?',
        const TextSelection.collapsed(offset: 6),
      );
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('table-apply')));
      await drive(screen.flush);
      final sorted = jsonDecode(jsonEncode(model.workspace)) as Map;
      final sortedDoc = jsonEncode(sorted['document']);
      expect(sorted['revision'], jsonDecode(original)['revision'] + 1);
      final rows = sorted['document']['content'][1]['content'] as List;
      expect(
        [for (final row in rows) nodeText(row['content'][0])],
        ['Name', 'Alpha', 'Alpha', 'Beta', 'Zulu!?'],
      );
      final oldRows = jsonDecode(originalDoc)['content'][1]['content'];
      for (var i = 0; i < 4; i++) {
        expect(rows[i], oldRows[[0, 2, 4, 3][i]]);
      }
      for (final key in (jsonDecode(original) as Map).keys.where(
        (key) => key != 'document' && key != 'revision',
      )) {
        expect(
          sorted[key],
          jsonDecode(original)[key],
          reason: '$key unchanged',
        );
      }
      void ranges(Map document) {
        final index = DocumentBookmarkIndex(document);
        for (final key in ['bookmarks', 'comments']) {
          final entry = (document['attrs'][key] as List).single;
          expect(index.text.substring(entry['start'], entry['end']), 'Zulu');
          expect(entry['start'], index.rangeAtPath([1, 4, 0, 0])!.start);
        }
      }

      ranges(sorted['document']);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), originalDoc);
      await drive(() => screen.history(true));
      expect(jsonEncode(model.workspace['document']), sortedDoc);
      await drive(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/sorted.docx',
        });
      });
      await t.runAsync(() async {
        await File('${folder.path}/before.json').writeAsString(original);
        await File(
          '${folder.path}/sorted.json',
        ).writeAsString(jsonEncode(sorted));
        final core = CoreBridge();
        final loaded = await core.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expect(jsonEncode(loaded['workspace']['document']), sortedDoc);
        await core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/recovered.docx',
        });
        expect(
          await File('${folder.path}/recovered.docx').readAsBytes(),
          await File('${folder.path}/sorted.docx').readAsBytes(),
        );
        final imported = await core.call({
          'op': 'importOffice',
          'path': '${folder.path}/sorted.docx',
        });
        ranges(imported['workspace']['document']);
        expect(
          nodeText(
            imported['workspace']['document']['content'][1]['content'][4]['content'][0],
          ),
          'Zulu!?',
        );
        expect(imported['workspace']['sheets'], model.workspace['sheets']);
      });
      expect(t.takeException(), isNull);
      debugPrint('Table sort UI evidence: ${folder.path}');
    },
  );
}
