import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/search_panel.dart';

void main() {
  testWidgets(
    'desktop find, heading reveal, atomic replacements, formula search and Undo',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory dir;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-search-ui-');
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
                  {'type': 'text', 'text': 'First heading'},
                ],
              },
              {
                'type': 'paragraph',
                'content': [
                  {
                    'type': 'text',
                    'text': '😀 north',
                    'marks': [
                      {'type': 'bold'},
                    ],
                  },
                  {'type': 'text', 'text': ' and north.'},
                ],
              },
              for (var i = 0; i < 35; i++)
                {
                  'type': 'paragraph',
                  'content': [
                    {'type': 'text', 'text': 'Paragraph $i for scrolling.'},
                  ],
                },
              {
                'type': 'heading',
                'attrs': {'level': 2},
                'content': [
                  {'type': 'text', 'text': 'Final heading'},
                ],
              },
            ],
          },
        });
        await core.call({'op': 'addSheet'});
        final sheets = (await core.call({'op': 'get'}))['workspace']['sheets'];
        await core.call({
          'op': 'cells',
          'cells': [
            {'sheetId': sheets[0]['id'], 'address': 'A1', 'value': '25'},
            {'sheetId': sheets[0]['id'], 'address': 'A2', 'value': '=A1*2'},
            {
              'sheetId': sheets[0]['id'],
              'address': 'B1',
              'value': 'north north',
            },
            {'sheetId': sheets[1]['id'], 'address': 'C5000', 'value': 'north'},
          ],
        });
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
      });
      Future<T> drive<T>(Future<T> Function() job) async {
        var done = false;
        T? value;
        Object? error;
        unawaited(
          Future.sync(job).then(
            (v) {
              value = v;
              done = true;
            },
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
        if (!done) throw StateError('Native operation did not finish');
        if (error != null) throw error!;
        return value as T;
      }

      Future<void> settle() async {
        await tester.pump(const Duration(milliseconds: 250));
        // Search replacement includes asynchronous engine preparation and glyph
        // layout. Wait for the actual operation, not its progress animation.
        for (var i = 0; i < 500; i++) {
          final panels = find.byType(SearchPanel);
          if (panels.evaluate().isEmpty) break;
          final state = tester.state<SearchPanelState>(panels);
          if (!state.busy && !state.pending) break;
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        await tester.pumpAndSettle();
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await settle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      // Check the real shortcut while Quill owns keyboard focus.
      final raw = tester.state<q.QuillRawEditorState>(
        find.byType(q.QuillRawEditor),
      );
      await tester.tapAt(
        raw.renderEditor.localToGlobal(
          raw.renderEditor
              .getLocalRectForCaret(const TextPosition(offset: 2))
              .center,
        ),
      );
      await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyF);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
      await drive(screen.flush);
      await settle();
      expect(find.byType(SearchPanel), findsOneWidget);
      final panel = tester.state<SearchPanelState>(find.byType(SearchPanel));
      await tester.tap(find.text('Headings'));
      await settle();
      expect(panel.results.hits.length, 2);
      await tester.tap(find.byKey(const ValueKey('search-hit-1')));
      await settle();
      final caret = raw.renderEditor.localToGlobal(
        raw.renderEditor
            .getLocalRectForCaret(
              TextPosition(offset: screen.editor.selection.start),
            )
            .center,
      );
      expect(caret.dy, inInclusiveRange(140, 738));
      expect(
        screen.editor.document.toPlainText().substring(
          screen.editor.selection.start,
          screen.editor.selection.end,
        ),
        'Final heading',
      );
      await tester.tap(find.text('Replace').first);
      await settle();
      await tester.enterText(find.byKey(const Key('find-query')), 'north');
      await tester.enterText(find.byKey(const Key('replace-text')), 'south');
      await settle();
      expect(panel.results.hits.length, 2);
      final before = model.workspace['document'];
      final revision = model.revision;
      await tester.tap(find.byKey(const Key('replace-all')));
      await drive(screen.flush);
      await settle();
      expect(
        nodeText(model.workspace['document']),
        contains('😀 south and south.'),
      );
      expect(model.revision, revision + 1);
      final bold = screen.editor.document
          .toDelta()
          .toJson()
          .where((op) => op['attributes']?['bold'] == true)
          .map((op) => op['insert'])
          .join();
      expect(bold, contains('😀 south'));
      await drive(() => screen.history(false));
      expect(model.workspace['document'], before);
      await settle();
      // A background/user edit invalidates results before another replacement.
      screen.editor.replaceText(
        0,
        0,
        'New ',
        const TextSelection.collapsed(offset: 4),
      );
      expect(panel.pending, isTrue);
      await drive(() => panel.replaceMatches(true));
      expect(
        nodeText(model.workspace['document']),
        contains('north and north'),
      );
      await drive(screen.flush);
      await drive(() => screen.switchMode(true));
      await settle();
      await tester.tap(find.text('Find').first);
      await settle();
      await tester.enterText(find.byKey(const Key('find-query')), 'north');
      await settle();
      expect(panel.results.hits.length, 1);
      await tester.tap(find.text('Current sheet'));
      await settle();
      await tester.tap(find.text('Workbook').last);
      await settle();
      expect(panel.results.hits.length, 2);
      await tester.tap(find.byKey(const ValueKey('search-hit-1')));
      await settle();
      expect(screen.sheetId, model.sheets[1]['id']);
      expect(screen.selectedCell, 'C5000');
      expect(
        screen.sheetKey.currentState!.vertical.offset,
        greaterThan(100000),
      );
      await tester.tap(find.text('Replace').first);
      await settle();
      await tester.enterText(find.byKey(const Key('find-query')), 'north');
      await tester.enterText(find.byKey(const Key('replace-text')), 'south');
      await settle();
      final beforeSheets = model.workspace['sheets'];
      final sheetRevision = model.revision;
      await tester.tap(find.byKey(const Key('replace-all')));
      await drive(screen.flush);
      await settle();
      expect(model.sheets[0]['cells']['B1'], 'south south');
      expect(model.sheets[1]['cells']['C5000'], 'south');
      expect(model.revision, sheetRevision + 1);
      await drive(() => screen.history(false));
      expect(model.workspace['sheets'], beforeSheets);
      await tester.enterText(find.byKey(const Key('find-query')), '*2');
      await tester.enterText(find.byKey(const Key('replace-text')), '*3');
      await settle();
      await tester.tap(find.byKey(const Key('replace-one')));
      await drive(screen.flush);
      await settle();
      expect(model.snapshot['calculated'][model.sheets[0]['id']]['A2'], 75);
      await drive(() => screen.history(false));
      expect(model.snapshot['calculated'][model.sheets[0]['id']]['A2'], 50);
      await drive(() => screen.contextAction('ask'));
      await settle();
      expect(find.byType(SearchPanel), findsNothing);
      expect(screen.searchPanel, isFalse);
      expect(find.text('Rewrite for clarity'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await drive(screen.flush);
      await tester.pumpWidget(const SizedBox());
      model.dispose();
      await tester.runAsync(() => dir.delete(recursive: true));
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );
}
