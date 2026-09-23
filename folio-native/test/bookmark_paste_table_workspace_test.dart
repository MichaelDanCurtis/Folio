// ignore_for_file: experimental_member_use
import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_quill/src/editor_toolbar_controller_shared/clipboard/clipboard_service.dart';
import 'package:flutter_quill/src/editor_toolbar_controller_shared/clipboard/clipboard_service_provider.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_bookmarks.dart';
import 'package:folio_native/src/document_table_view.dart';

class _Clipboard extends ClipboardService {
  String? html;
  @override
  Future<String?> getHtmlText() async => html;
  @override
  Future<String?> getHtmlFile() async => null;
  @override
  Future<String?> getMarkdownFile() async => null;
  @override
  Future<Uint8List?> getImageFile() async => null;
  @override
  Future<Uint8List?> getGifFile() async => null;
  @override
  Future<void> copyImage(Uint8List bytes) async {}
}

void main() {
  testWidgets(
    'clipboard rich paste and multi-cell drafts retain exact bookmarks through Apply, Undo and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final clipboard = _Clipboard(),
          previousClipboard = ClipboardServiceProvider.instance;
      ClipboardServiceProvider.setInstance(clipboard);
      addTearDown(
        () => ClipboardServiceProvider.setInstance(previousClipboard),
      );
      String? plain;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            plain = call.arguments['text'];
          }
          if (call.method == 'Clipboard.getData') return {'text': plain};
          if (call.method == 'Clipboard.hasStrings') {
            return {'value': plain?.isNotEmpty == true};
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      Map paragraph(String text) => {
        'type': 'paragraph',
        'content': [
          {
            'type': 'text',
            'text': text,
            'marks': [
              {'type': 'bold'},
            ],
          },
        ],
      };
      final document = {
        'type': 'doc',
        'attrs': {
          'bookmarks': [
            {'name': 'Head', 'start': 2, 'end': 4},
            {'name': 'First', 'start': 9, 'end': 11},
            {'name': 'Second', 'start': 16, 'end': 18},
            {'name': 'Tail', 'start': 21, 'end': 25},
          ],
        },
        'content': [
          paragraph('aaaaaa'),
          {
            'type': 'table',
            'content': [
              {
                'type': 'tableRow',
                'content': [
                  {
                    'type': 'tableCell',
                    'content': [paragraph('aaaaaa')],
                  },
                  {
                    'type': 'tableCell',
                    'content': [paragraph('bbbbbb')],
                  },
                ],
              },
            ],
          },
          paragraph('Tail'),
        ],
      };
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp(
          'folio-bookmark-paste-ui-',
        );
        final core = CoreBridge();
        final state = await core.call({'op': 'new'});
        final workspace = Map<String, dynamic>.from(state['workspace']);
        workspace['document'] = document;
        await core.call({'op': 'replace', 'workspace': workspace});
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
      List<int> range(String name) {
        final b = documentBookmarks(
          model.workspace['document'],
        ).singleWhere((b) => b['name'] == name);
        return [b['start'], b['end']];
      }

      void select(q.QuillController c, int a, int b) => c.updateSelection(
        TextSelection(baseOffset: a, extentOffset: b),
        q.ChangeSource.local,
      );
      final original = jsonEncode(model.workspace['document']);
      final originalWorkspace = jsonEncode(model.workspace);
      final originalSheets = jsonEncode(model.workspace['sheets']);
      // Exercise the actual internal clipboard paste path, including its Delta.
      select(screen.editor, 0, 1);
      expect(screen.editor.clipboardSelection(true), isTrue);
      await tester.pump();
      select(screen.editor, 0, 0);
      bool pasted = false;
      await drive(() async {
        pasted = await screen.editor.clipboardPaste();
        await screen.flush();
      });
      expect(pasted, isTrue);
      expect(range('Head'), [3, 5]);
      expect(range('First'), [10, 12]);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), original);
      // External HTML goes through the HTML converter and formatted paste path.
      clipboard.html = '<p><em>a</em></p>';
      select(screen.editor, 0, 0);
      await drive(() async {
        pasted = await screen.editor.clipboardPaste();
        await screen.flush();
      });
      expect(pasted, isTrue);
      expect(range('Head'), [4, 6]);
      expect(model.workspace['document']['content'][0]['content'][0]['marks'], [
        {'type': 'italic'},
      ]);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), original);
      clipboard.html = null;
      // Compose, unlike replaceText, has no text-replacement callback.
      screen.editor.compose(
        Delta()
          ..insert('a', {'bold': true})
          ..retain(4)
          ..insert('a', {'bold': true}),
        const TextSelection.collapsed(offset: 1),
        q.ChangeSource.local,
      );
      await drive(screen.flush);
      expect(range('Head'), [3, 5]);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), original);
      final revision = model.revision;
      Future<void> openCell(int column) async {
        await tester.tap(find.byKey(ValueKey('table-cell-0-$column')).first);
        await tester.pumpAndSettle();
      }

      q.QuillController cellEditor() => tester
          .widget<q.QuillEditor>(find.byType(q.QuillEditor).last)
          .controller;
      await openCell(0);
      var cell = cellEditor();
      cell.replaceText(0, 0, 'a', const TextSelection.collapsed(offset: 1));
      cell.replaceText(
        0,
        0,
        Delta()..insert('a', {'bold': true}),
        const TextSelection.collapsed(offset: 1),
      );
      expect(model.revision, revision);
      expect(jsonEncode(model.workspace['document']), original);
      await tester.tap(
        find.descendant(
          of: find.byType(DocumentTableDialog),
          matching: find.byKey(const ValueKey('table-cell-0-1')),
        ),
      );
      await tester.pumpAndSettle();
      cell = cellEditor();
      cell.replaceText(0, 1, '', const TextSelection.collapsed(offset: 0));
      // No pump between the final edit and Apply: it must already be in the draft.
      await tester.tap(find.byKey(const Key('table-apply')));
      await drive(screen.flush);
      expect(model.revision, revision + 1);
      expect(range('Head'), [2, 4]);
      expect(range('First'), [11, 13]);
      expect(range('Second'), [17, 19]);
      expect(range('Tail'), [22, 26]);
      expect(jsonEncode(model.workspace['sheets']), originalSheets);
      expect(
        DocumentBookmarkIndex(model.workspace['document']).text,
        'aaaaaa\naaaaaaaa\nbbbbb\nTail\n',
      );
      final applied = jsonEncode(model.workspace['document']);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), original);
      await drive(() => screen.history(true));
      expect(jsonEncode(model.workspace['document']), applied);
      // Whole marked text replacement removes just that bookmark. Cancel and Undo
      // restore the full content/ranges, without changing the workbook.
      await openCell(0);
      cellEditor().replaceText(
        4,
        2,
        Delta()..insert('XY', {'italic': true}),
        const TextSelection.collapsed(offset: 6),
      );
      await tester.tap(find.text('Cancel'));
      await drive(screen.flush);
      expect(jsonEncode(model.workspace['document']), applied);
      await openCell(0);
      cellEditor().replaceText(
        4,
        2,
        Delta()..insert('XY', {'italic': true}),
        const TextSelection.collapsed(offset: 6),
      );
      await tester.tap(find.byKey(const Key('table-apply')));
      await drive(screen.flush);
      expect(
        documentBookmarks(
          model.workspace['document'],
        ).any((b) => b['name'] == 'First'),
        isFalse,
      );
      expect(range('Second'), [17, 19]);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), applied);
      // Identical-looking content in a replacement workspace is not the draft's
      // original target. Do not apply the old table into the newly opened file.
      await openCell(0);
      cellEditor().replaceText(
        0,
        0,
        'Wrong workspace',
        const TextSelection.collapsed(offset: 15),
      );
      final other =
          jsonDecode(jsonEncode(model.workspace)) as Map<String, dynamic>;
      other['id'] = '${other['id']}-other';
      await drive(
        () => model.mutate({
          'op': 'replace',
          'workspace': other,
        }, refreshDocument: true),
      );
      final otherRevision = model.revision;
      await tester.tap(find.byKey(const Key('table-apply')));
      await drive(screen.flush);
      expect(model.revision, otherRevision);
      expect(jsonEncode(model.workspace['document']), applied);
      expect(
        find.text('The table changed. Reopen it and try again.'),
        findsOneWidget,
      );
      final output = Directory(
        '${Directory.systemTemp.path}/folio-bookmark-paste-tests',
      );
      await drive(() async {
        await output.create(recursive: true);
        await screen.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${output.path}/edited.docx',
        });
        await File(model.recoveryPath).copy('${output.path}/workspace.folio');
        await File(
          '${output.path}/original-document.json',
        ).writeAsString(original);
        await File(
          '${output.path}/original-workspace.folio',
        ).writeAsString(originalWorkspace);
        await model.core.call({
          'op': 'load',
          'path': '${output.path}/workspace.folio',
        });
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${output.path}/reopened.docx',
        });
      });
      final recovered = await tester.runAsync(
        () => model.core.call({'op': 'get'}),
      );
      expect(jsonEncode(recovered!['workspace']['document']), applied);
      expect(recovered['workspace']['sheets'], model.workspace['sheets']);
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
