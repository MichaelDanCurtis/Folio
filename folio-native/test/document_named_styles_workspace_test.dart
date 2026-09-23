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
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_named_styles.dart';
import 'package:folio_native/src/document_style_resolution.dart';
import 'package:folio_native/src/document_table_view.dart';
import 'package:folio_native/src/document_view.dart';

void main() {
  testWidgets(
    'named styles apply, modify, type, draft, history and recover through the real app',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1100);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory dir;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-style-app-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path':
              '${Directory.current.path}/test/fixtures/word-named-styles.docx',
        });
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
      });
      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 500 && !ready(); i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(ready(), true);
        await tester.pumpAndSettle();
      }

      Future<void> drive(Future<void> Function() action) async {
        var done = false;
        Object? error;
        unawaited(
          action().then(
            (_) {
              done = true;
            },
            onError: (Object e) {
              error = e;
              done = true;
            },
          ),
        );
        await until(() => done);
        if (error != null) throw error!;
      }

      Future<void> settle() => drive(
        tester.state<WorkspaceScreenState>(find.byType(WorkspaceScreen)).flush,
      );
      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await tester.pumpAndSettle();
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
      });
      final view = tester.state<DocumentViewState>(find.byType(DocumentView));
      q.QuillController editor() => tester
          .widget<q.QuillEditor>(find.byType(q.QuillEditor).first)
          .controller;
      Map doc() => model.workspace['document'] as Map;
      Map paragraph(String prefix) => (doc()['content'] as List)
          .where((n) => n['type'] == 'paragraph' || n['type'] == 'heading')
          .cast<Map>()
          .firstWhere(
            (n) => (n['content'] as List? ?? [])
                .map((r) => r['text'] ?? '')
                .join()
                .startsWith(prefix),
          );
      void select(String prefix, {bool whole = true}) {
        final p = DocumentBookmarkIndex(
          doc(),
        ).paragraphs.firstWhere((p) => p.text.startsWith(prefix));
        editor().updateSelection(
          TextSelection(
            baseOffset: p.editorOffset,
            extentOffset: p.editorOffset + (whole ? p.text.length : 0),
          ),
          q.ChangeSource.local,
        );
      }

      Future<void> styles({String? find}) async {
        unawaited(view.editStyles());
        await until(() => findByStyles().evaluate().isNotEmpty);
        if (find != null) {
          await tester.enterText(findByStylesSearch(), find);
          await tester.pumpAndSettle();
        }
      }

      Future<void> searchAndPick(String id, String query) async {
        await tester.enterText(findByStylesSearch(), query);
        await tester.pumpAndSettle();
        await tester.ensureVisible(find.byKey(ValueKey('named-style-$id')));
        await tester.tap(find.byKey(ValueKey('named-style-$id')));
        await tester.pump();
      }

      Future<void> history(bool redo) async {
        debugPrint('Style check: keyboard ${redo ? 'redo' : 'undo'}');
        final before = model.revision;
        await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
        if (redo) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
        if (redo) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
        await until(() => model.revision != before);
        await settle();
      }

      final original = jsonEncode(doc());
      late Map data;
      await drive(() async {
        data = await model.core.call({'op': 'documentStyles'});
      });
      debugPrint('Style check: resolver equivalence');
      for (final s in data['catalog']['styles'] as List) {
        expect(
          resolveNamedStyle(data['catalog'] as Map, s['id'] as String),
          data['resolved'][s['id']],
          reason: 'Dart/engine style ${s['id']}',
        );
      }
      final one = DocumentBookmarkIndex(
        doc(),
      ).paragraphs.firstWhere((p) => p.text.startsWith('Revenue'));
      final two = DocumentBookmarkIndex(
        doc(),
      ).paragraphs.firstWhere((p) => p.text.startsWith('Second'));
      editor().updateSelection(
        TextSelection(
          baseOffset: one.editorOffset,
          extentOffset: two.editorOffset + two.text.length,
        ),
        q.ChangeSource.local,
      );
      await styles();
      await searchAndPick('ReportBase', 'Report base');
      await tester.tap(find.byKey(const Key('named-style-modify')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('style-editor-name')),
        'Report base renamed',
      );
      await tester.enterText(find.byKey(const Key('style-editor-size')), '18');
      await tester.tap(find.byKey(const Key('style-editor-save')));
      await until(() => find.byType(DocumentStylesDialog).evaluate().isEmpty);
      await settle();
      debugPrint('Style check: base style saved');
      final changed = jsonEncode(doc());
      final styledSelection = editor().selection;
      expect(styledSelection.start, greaterThan(0));
      expect(changed, isNot(original));
      expect(
        paragraph('Revenue')['attrs']['paragraphFormat']['inheritedRun']['sz'],
        36,
      );
      expect(
        paragraph('Second')['attrs']['paragraphFormat']['inheritedRun']['sz'],
        36,
      );
      expect(
        paragraph(
          'Unrelated',
        )['attrs']['paragraphFormat']['inheritedRun']['sz'],
        24,
      );
      await history(false);
      expect(jsonEncode(doc()), original);
      expect(editor().selection, styledSelection);
      await history(true);
      expect(jsonEncode(doc()), changed);
      expect(editor().selection, styledSelection);
      debugPrint('Style check: base style history passed');
      // The toolbar can turn inherited bold off, including keyboard routing after
      // dialogs; an explicit off value must not disappear from the codec.
      final revenue = DocumentBookmarkIndex(
        doc(),
      ).paragraphs.firstWhere((p) => p.text.startsWith('Revenue'));
      editor().updateSelection(
        TextSelection(
          baseOffset: revenue.editorOffset,
          extentOffset: revenue.editorOffset + 7,
        ),
        q.ChangeSource.local,
      );
      await tester.pump();
      await tester.tap(find.byKey(const Key('document-mark-bold')));
      await settle();
      expect(
        paragraph('Revenue')['content'][0]['marks'].any(
          (m) => m['type'] == 'bold' && m['attrs']?['value'] == false,
        ),
        true,
      );
      await history(false);
      expect(jsonEncode(doc()), changed);
      select('Revenue');
      await styles();
      final revision = model.revision;
      await tester.tap(find.widgetWithText(TextButton, 'Cancel').last);
      await until(() => find.byType(DocumentStylesDialog).evaluate().isEmpty);
      await settle();
      expect(model.revision, revision);
      debugPrint('Style check: direct bold and Cancel passed');
      // A table style is only a draft until the outer Apply button is pressed.
      final tableFinder = find.byType(DocumentTableView).first;
      await tester.ensureVisible(tableFinder);
      await tester.tap(
        find.text('Table report text', findRichText: true).first,
      );
      await until(() => find.byType(DocumentTableDialog).evaluate().isNotEmpty);
      var cellEditor = tester
          .widget<q.QuillEditor>(
            find
                .descendant(
                  of: find.byType(DocumentTableDialog),
                  matching: find.byType(q.QuillEditor),
                )
                .first,
          )
          .controller;
      final cellEnd = cellEditor.document.length - 1;
      cellEditor.replaceText(
        cellEnd,
        0,
        ' added',
        TextSelection.collapsed(offset: cellEnd + 6),
      );
      cellEditor.updateSelection(
        const TextSelection.collapsed(offset: 0),
        q.ChangeSource.local,
      );
      await tester.pumpAndSettle();
      final beforeDraft = jsonEncode(doc());
      final beforeRevision = model.revision;
      await tester.tap(find.byKey(const Key('table-styles')));
      await until(
        () => find.byType(DocumentStylesDialog).evaluate().isNotEmpty,
      );
      await searchAndPick('ReportHeading', 'Report heading');
      await tester.tap(find.byKey(const Key('named-style-apply')));
      await until(() => find.byType(DocumentStylesDialog).evaluate().isEmpty);
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 60)),
      );
      await tester.pumpAndSettle();
      expect(jsonEncode(doc()), beforeDraft);
      expect(model.revision, beforeRevision);
      await tester.tap(find.byKey(const Key('table-apply')));
      await until(() => find.byType(DocumentTableDialog).evaluate().isEmpty);
      await settle();
      Map cell() =>
          (doc()['content'] as List).firstWhere(
                (n) => n['type'] == 'table',
              )['content'][0]['content'][0]['content'][0]
              as Map;
      expect(cell()['attrs']['paragraphFormat']['styleId'], 'ReportHeading');
      expect(
        (cell()['content'] as List).map((r) => r['text']).join(),
        'Table report text added',
      );
      final withTable = jsonEncode(doc());
      await history(false);
      expect(jsonEncode(doc()), beforeDraft);
      await history(true);
      expect(jsonEncode(doc()), withTable);
      debugPrint('Style check: table draft and history passed');
      // A style applied to numbered text stays editable and keeps its number.
      select('Revenue');
      editor().formatSelection(q.Attribute.ol);
      await settle();
      await styles();
      await searchAndPick('ReportHeading', 'Report heading');
      await tester.tap(find.byKey(const Key('named-style-apply')));
      await until(() => find.byType(DocumentStylesDialog).evaluate().isEmpty);
      await settle();
      final listed = DocumentBookmarkIndex(
        doc(),
      ).paragraphs.firstWhere((p) => p.text.startsWith('Revenue'));
      final heading = nodeAtStylePath(doc(), listed.path);
      expect(heading['type'], 'heading');
      expect(heading['attrs']['paragraphFormat']['styleId'], 'ReportHeading');
      expect(
        documentToDelta(doc()).toList().where((op) => op.data is Map).length,
        1,
        reason: 'Only the existing table is embedded; the styled list is text',
      );
      await drive(() async {
        final folder = Directory(
          '${Directory.systemTemp.path}/folio-named-style-ui',
        );
        await folder.create(recursive: true);
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/list-styled.docx',
        });
      });
      await history(false);
      await history(false);
      expect(jsonEncode(doc()), withTable);
      debugPrint('Style check: styled list and history passed');
      // Enter selects the configured following style; new typing keeps it.
      select('Executive summary');
      final title = DocumentBookmarkIndex(
        doc(),
      ).paragraphs.firstWhere((p) => p.text == 'Executive summary');
      final end = title.editorOffset + title.text.length;
      editor().replaceText(
        end,
        0,
        '\n',
        TextSelection.collapsed(offset: end + 1),
      );
      editor().updateSelection(
        TextSelection.collapsed(offset: end + 1),
        q.ChangeSource.local,
      );
      editor().replaceText(
        end + 1,
        0,
        'New section detail',
        TextSelection.collapsed(offset: end + 19),
      );
      await settle();
      expect(paragraph('New section detail')['type'], 'paragraph');
      expect(
        paragraph('New section detail')['attrs']['paragraphFormat']['styleId'],
        'ReportBody',
      );
      debugPrint('Style check: following style passed');
      // A source change while Styles is open makes its captured selection stale.
      select('Revenue');
      await styles();
      await searchAndPick('Normal', 'Normal');
      await drive(
        () => model.mutate({'op': 'title', 'title': 'Style verification'}),
      );
      final stale = jsonEncode(doc());
      await tester.tap(find.byKey(const Key('named-style-apply')));
      await until(() => find.byType(DocumentStylesDialog).evaluate().isEmpty);
      await settle();
      expect(jsonEncode(doc()), stale);
      expect(
        find.textContaining('The document changed. Reopen Styles'),
        findsWidgets,
      );
      final beforeCreate = jsonEncode(doc()['content']);
      await styles();
      await tester.tap(find.byKey(const Key('named-style-new')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('style-editor-name')),
        'Unused proof style',
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('style-editor-save')));
      await until(() => find.byType(DocumentStylesDialog).evaluate().isEmpty);
      await settle();
      final created = (doc()['attrs']['wordStyles']['styles'] as List)
          .firstWhere((s) => s['name'] == 'Unused proof style');
      expect(jsonEncode(doc()['content']), beforeCreate);
      await styles();
      await searchAndPick(created['id'] as String, 'Unused proof style');
      await tester.tap(find.byKey(const Key('named-style-delete')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('named-style-delete-confirm')));
      await until(() => find.byType(DocumentStylesDialog).evaluate().isEmpty);
      await settle();
      expect(
        (doc()['attrs']['wordStyles']['styles'] as List).any(
          (s) => s['id'] == created['id'],
        ),
        false,
      );
      expect(jsonEncode(doc()['content']), beforeCreate);
      debugPrint('Style check: create and delete passed');
      final output = Directory(
        '${Directory.systemTemp.path}/folio-named-style-ui',
      );
      debugPrint('Style check: stale dialog passed');
      await drive(() async {
        await output.create(recursive: true);
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${output.path}/styled.docx',
        });
        await model.core.call({
          'op': 'save',
          'path': '${output.path}/styled.folio',
        });
      });
      final saved = jsonEncode(doc());
      late Map reopened;
      await drive(() async {
        reopened = await model.core.call({
          'op': 'load',
          'path': '${output.path}/styled.folio',
        });
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${output.path}/reopened.docx',
        });
      });
      expect(jsonEncode(reopened['workspace']['document']), saved);
      debugPrint('Style check: export and recovery passed');
      expect(tester.takeException(), null);
    },
    timeout: const Timeout(Duration(minutes: 4)),
  );
}

Finder findByStyles() => find.byType(DocumentStylesDialog);
Finder findByStylesSearch() => find.byKey(const Key('named-style-search'));
