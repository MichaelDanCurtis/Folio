import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_paragraph_dialog.dart';
import 'package:folio_native/src/document_table_view.dart';
import 'package:folio_native/src/search.dart';

void main() {
  testWidgets(
    'outline dialog preserves appearance, drafts, history and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1100);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory dir;
      final output = Directory('${Directory.systemTemp.path}/folio-outline-ui');
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-outline-ui-');
        await output.create(recursive: true);
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path':
              '${Directory.current.path}/test/fixtures/word-outline-levels.docx',
        });
        model = WorkspaceModel(core, '${dir.path}/recovery.folio');
        await model.initialize();
      });
      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 500 && !ready(); i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 5)),
          );
        }
        expect(ready(), isTrue);
        await tester.pumpAndSettle();
      }

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
        await until(() => done);
        if (error != null) throw error!;
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await tester.pumpAndSettle();
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
      });
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      Map doc() => model.workspace['document'] as Map;
      Map first() => doc()['content'][0] as Map;
      final original = jsonEncode(doc());
      Future<void> undo() =>
          drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      Future<void> open({bool cell = false}) async {
        await tester.tap(
          find.byKey(Key(cell ? 'table-paragraph' : 'document-paragraph')),
        );
        await until(
          () => find.byType(DocumentParagraphDialog).evaluate().isNotEmpty,
        );
      }

      Future<void> choose(String name) async {
        await tester.tap(
          find.byWidgetPredicate(
            (w) =>
                w is DropdownButtonFormField<String> &&
                '${w.key}'.contains('paragraph-outline-'),
          ),
        );
        await tester.pumpAndSettle();
        await tester.ensureVisible(find.text(name).last);
        await tester.tap(find.text(name).last);
        await tester.pumpAndSettle();
      }

      Future<void> apply() async {
        await tester.ensureVisible(find.byKey(const Key('apply-paragraph')));
        await tester.tap(find.byKey(const Key('apply-paragraph')));
        await until(
          () => find.byType(DocumentParagraphDialog).evaluate().isEmpty,
        );
        await drive(screen.flush);
      }

      await open();
      await choose('Body text');
      expect(jsonEncode(doc()), original);
      await tester.tap(find.text('Cancel').last);
      await until(
        () => find.byType(DocumentParagraphDialog).evaluate().isEmpty,
      );
      expect(jsonEncode(doc()), original);
      await open();
      await choose('Body text');
      await apply();
      expect(first()['type'], 'paragraph');
      expect(first()['attrs']['paragraphFormat']['styleId'], 'Outlinebody');
      expect(DocumentSearchIndex(doc()).blocks.first.heading, 0);
      final body = jsonEncode(doc());
      await undo();
      expect(jsonEncode(doc()), original);
      await drive(() => model.mutate({'op': 'redo'}, refreshDocument: true));
      expect(jsonEncode(doc()), body);
      await open();
      await choose('Use style');
      await apply();
      expect(first()['type'], 'paragraph');
      expect(
        (first()['attrs']['paragraphFormat']['direct'] as Map).containsKey(
          'outlineLevel',
        ),
        isFalse,
      );
      // Multi-paragraph selection can include different direct outline values.
      screen.editor.updateSelection(
        const TextSelection(baseOffset: 0, extentOffset: 56),
        q.ChangeSource.local,
      );
      await open();
      expect(find.text('Mixed — keep existing'), findsWidgets);
      await choose('Level 3');
      await apply();
      expect(doc()['content'][0]['attrs']['level'], 3);
      expect(doc()['content'][1]['attrs']['level'], 3);
      expect(
        doc()['content'][1]['attrs']['paragraphFormat']['styleId'],
        'Heading1',
      );
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${output.path}/body.docx',
        });
      });
      await undo();
      final beforeCell = jsonEncode(doc());
      Future<void> cellDraft() async {
        await tester.ensureVisible(find.byType(DocumentTableView).first);
        await tester.tap(find.text('Cell structure', findRichText: true).first);
        await until(
          () => find.byType(DocumentTableDialog).evaluate().isNotEmpty,
        );
        await open(cell: true);
        await choose('Level 5');
        await apply();
        expect(jsonEncode(doc()), beforeCell);
      }

      await cellDraft();
      await tester.tap(
        find
            .descendant(
              of: find.byType(DocumentTableDialog),
              matching: find.text('Cancel'),
            )
            .last,
      );
      await until(() => find.byType(DocumentTableDialog).evaluate().isEmpty);
      expect(jsonEncode(doc()), beforeCell);
      await cellDraft();
      await tester.tap(find.byKey(const Key('table-apply')));
      await until(() => find.byType(DocumentTableDialog).evaluate().isEmpty);
      await drive(screen.flush);
      final afterCell = jsonEncode(doc());
      final cell =
          doc()['content'][5]['content'][0]['content'][0]['content'][0];
      expect(cell['attrs']['level'], 5);
      expect(cell['attrs']['paragraphFormat']['styleId'], 'Outlinebody');
      await undo();
      expect(jsonEncode(doc()), beforeCell);
      await drive(() => model.mutate({'op': 'redo'}, refreshDocument: true));
      expect(jsonEncode(doc()), afterCell);
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${output.path}/cell.docx',
        });
        await model.core.call({
          'op': 'save',
          'path': '${output.path}/saved.folio',
        });
        await model.open('${output.path}/saved.folio');
      });
      expect(jsonEncode(doc()), afterCell);
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${output.path}/reopened.docx',
        });
      });
      expect(tester.takeException(), isNull);
    },
    timeout: const Timeout(Duration(minutes: 4)),
  );
}
