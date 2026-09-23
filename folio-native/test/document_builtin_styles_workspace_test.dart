import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_builtin_styles.dart';
import 'package:folio_native/src/document_named_styles.dart';
import 'package:folio_native/src/document_table_view.dart';
import 'package:folio_native/src/document_view.dart';

void main() {
  testWidgets(
    'missing headings work in the gallery, rapid typing, cells and all nine levels',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1100);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory dir;
      final output = Directory(
        '${Directory.systemTemp.path}/folio-builtin-style-ui',
      );
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-builtin-ui-');
        await output.create(recursive: true);
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path':
              '${Directory.current.path}/test/fixtures/word-missing-headings.docx',
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
      final view = tester.state<DocumentViewState>(find.byType(DocumentView));
      Map doc() => model.workspace['document'] as Map;
      Map catalog() => doc()['attrs']['wordStyles'] as Map;
      bool has(int level) =>
          (catalog()['styles'] as List).any((s) => s['id'] == 'Heading$level');
      Future<void> undo() =>
          drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      Future<void> export(String name) => drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${output.path}/$name.docx',
        });
      });
      Future<void> choose(int level) async {
        await until(
          () => find.byType(DocumentStylesDialog).evaluate().isNotEmpty,
        );
        await tester.enterText(
          find.byKey(const Key('named-style-search')),
          'Heading $level',
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(ValueKey('named-style-Heading$level')));
        await tester.pump();
        expect(
          tester
              .widget<TextButton>(find.byKey(const Key('named-style-delete')))
              .onPressed,
          isNull,
        );
        await tester.tap(find.byKey(const Key('named-style-apply')));
        await until(() => find.byType(DocumentStylesDialog).evaluate().isEmpty);
      }

      final original = jsonEncode(doc());
      late Map data;
      await drive(() async {
        data = await model.core.call({'op': 'documentStyles'});
      });
      expect(
        availableBuiltinStyles(data['catalog'] as Map),
        data['availableStyles'],
      );
      expect(jsonEncode(doc()), original);
      expect(has(9), isFalse);
      unawaited(view.editStyles());
      await choose(9);
      await drive(screen.flush);
      expect(doc()['content'][0]['attrs']['level'], 9);
      expect(has(9), isTrue);
      expect(tester.takeException(), isNull);
      await undo();
      expect(jsonEncode(doc()), original);
      expect(has(9), isFalse);
      // Formatting and Enter occur synchronously before the save completes.
      screen.editor.updateSelection(
        const TextSelection.collapsed(offset: 17),
        q.ChangeSource.local,
      );
      screen.editor.formatSelection(const q.HeaderAttribute(level: 5));
      screen.editor.replaceText(
        17,
        0,
        '\n',
        const TextSelection.collapsed(offset: 18),
      );
      screen.editor.replaceText(
        18,
        0,
        'Following body',
        const TextSelection.collapsed(offset: 32),
      );
      await drive(screen.flush);
      expect(
        doc()['content'][0]['attrs']['paragraphFormat']['styleId'],
        'Heading5',
      );
      expect(
        doc()['content'][1]['attrs']['paragraphFormat']['styleId'],
        'Normal',
      );
      expect(doc()['content'][1]['type'], 'paragraph');
      // Formatting, Enter and typing remain three ordered Undo steps.
      await undo();
      await undo();
      await undo();
      expect(jsonEncode(doc()), original);
      Future<void> cellStyle() async {
        await tester.ensureVisible(find.byType(DocumentTableView).first);
        await tester.tap(find.text('Cell candidate', findRichText: true).first);
        await until(
          () => find.byType(DocumentTableDialog).evaluate().isNotEmpty,
        );
        await tester.tap(find.byKey(const Key('table-styles')));
        // Cell picker intentionally has no global style-management buttons.
        await until(
          () => find.byType(DocumentStylesDialog).evaluate().isNotEmpty,
        );
        await tester.enterText(
          find.byKey(const Key('named-style-search')),
          'Heading 7',
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('named-style-Heading7')));
        await tester.tap(find.byKey(const Key('named-style-apply')));
        await until(() => find.byType(DocumentStylesDialog).evaluate().isEmpty);
        expect(has(7), isFalse);
        expect(jsonEncode(doc()), original);
        expect(tester.takeException(), isNull);
      }

      await cellStyle();
      await tester.tap(
        find
            .descendant(
              of: find.byType(DocumentTableDialog),
              matching: find.text('Cancel'),
            )
            .last,
      );
      await until(() => find.byType(DocumentTableDialog).evaluate().isEmpty);
      expect(jsonEncode(doc()), original);
      await cellStyle();
      await tester.tap(find.byKey(const Key('table-apply')));
      await until(() => find.byType(DocumentTableDialog).evaluate().isEmpty);
      await drive(screen.flush);
      expect(has(7), isTrue);
      await export('cell-heading');
      await undo();
      expect(jsonEncode(doc()), original);
      for (var level = 1; level <= 9; level++) {
        screen.editor.updateSelection(
          const TextSelection.collapsed(offset: 0),
          q.ChangeSource.local,
        );
        screen.editor.formatSelection(q.HeaderAttribute(level: level));
        await drive(screen.flush);
        expect(doc()['content'][0]['attrs']['level'], level);
        expect(
          doc()['content'][0]['attrs']['paragraphFormat']['styleId'],
          'Heading$level',
        );
        expect(has(level), isTrue);
        expect(tester.takeException(), isNull);
      }
      final beforeSave = jsonEncode(doc());
      await export('headings');
      await drive(() async {
        await model.core.call({
          'op': 'save',
          'path': '${output.path}/headings.folio',
        });
        await model.open('${output.path}/headings.folio');
      });
      expect(jsonEncode(doc()), beforeSave);
      await export('reopened');
      expect(tester.takeException(), isNull);
    },
    timeout: const Timeout(Duration(minutes: 4)),
  );
}
