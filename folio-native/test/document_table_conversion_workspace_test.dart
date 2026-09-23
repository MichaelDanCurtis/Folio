import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_table_conversion_dialog.dart';
import 'package:folio_native/src/document_tables.dart';

void main() {
  testWidgets(
    'conversion preview, validation, cancel, Apply, Undo, persistence and DOCX export',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = Directory(
          '${Directory.systemTemp.path}/folio-word-conversion-tests',
        );
        await folder.create(recursive: true);
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path':
              '${Directory.current.path}/test/fixtures/word-conversion.docx',
        });
        model = WorkspaceModel(
          core,
          '${folder.path}/workspace-${DateTime.now().microsecondsSinceEpoch}.folio',
        );
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
      final original = jsonEncode(model.workspace['document']);
      final revision = model.revision;
      await drive(() async {
        await model.core.call({
          'op': 'save',
          'path': '${folder.path}/original.folio',
        });
      });
      q.QuillController editor() => tester
          .widget<q.QuillEditor>(find.byType(q.QuillEditor).first)
          .controller;
      Future<void> showConversion() async {
        final controller = editor();
        final text = controller.document.toPlainText();
        controller.updateSelection(
          TextSelection(
            baseOffset: text.indexOf('Name'),
            extentOffset: text.indexOf('After conversion'),
          ),
          q.ChangeSource.local,
        );
        await tester.tap(find.byKey(const Key('text-to-table')));
        await tester.pumpAndSettle();
      }

      await showConversion();
      expect(find.text('2 rows × 2 columns'), findsOneWidget);
      // Native accessibility edits update the controller without onChanged.
      tester
              .widget<TextField>(find.byKey(const Key('conversion-columns')))
              .controller!
              .text =
          '0';
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<FilledButton>(find.byKey(const Key('conversion-apply')))
            .onPressed,
        isNull,
      );
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(jsonEncode(model.workspace['document']), original);
      expect(model.revision, revision);
      await showConversion();
      await tester.enterText(find.byKey(const Key('conversion-width')), '120');
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('conversion-apply')));
      await tester.pumpAndSettle();
      await drive(() => model.flush());
      expect(model.revision, revision + 1);
      expect(
        editor().selection,
        const TextSelection.collapsed(offset: 20),
        reason: 'Typing after conversion must not replace the selected table.',
      );
      final tableDocument = jsonEncode(model.workspace['document']);
      final grid = DocumentTable(model.workspace['document']['content'][1]);
      expect(grid.columns, 2);
      expect(grid.rows.length, 2);
      expect(nodeText(grid.at(1, 0).cell), 'Research');
      Future<void> export(String name) => drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/$name.docx',
        });
      });
      await export('table');
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      Future<void> historyKey(bool redo) async {
        final previous = model.revision;
        await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
        if (redo) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
        if (redo) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
        for (var i = 0; i < 100 && model.revision == previous; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        await tester.pumpAndSettle();
        expect(
          model.revision,
          previous + 1,
          reason:
              'Command-Z must reach workspace history after the conversion dialog closes. Primary focus: ${FocusManager.instance.primaryFocus}',
        );
      }

      await historyKey(false);
      expect(jsonEncode(model.workspace['document']), original);
      await export('undone');
      await historyKey(true);
      expect(jsonEncode(model.workspace['document']), tableDocument);
      await tester.tap(find.byKey(const Key('table-to-text')));
      await tester.pumpAndSettle();
      expect(find.byType(DocumentTableConversionDialog), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(jsonEncode(model.workspace['document']), tableDocument);
      await tester.tap(find.byKey(const Key('table-to-text')));
      await tester.pumpAndSettle();
      final beforeTextRevision = model.revision;
      await tester.tap(find.byKey(const Key('conversion-apply')));
      await tester.pumpAndSettle();
      await drive(() => model.flush());
      expect(model.revision, beforeTextRevision + 1);
      expect(
        editor().document.toPlainText(),
        'Before conversion\nName\tAmount\nResearch\t2400\nAfter conversion\n',
      );
      final textDocument = jsonEncode(model.workspace['document']);
      expect(editor().selection, const TextSelection.collapsed(offset: 44));
      await export('text');
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), tableDocument);
      await drive(() => screen.history(true));
      expect(jsonEncode(model.workspace['document']), textDocument);
      await drive(() => model.flush());
      final saved = await tester.runAsync(
        () => File(model.recoveryPath).readAsString(),
      );
      expect(jsonEncode(jsonDecode(saved!)['document']), textDocument);
      await drive(() async {
        await model.core.call({'op': 'load', 'path': model.recoveryPath});
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/reopened.docx',
        });
      });
      expect(tester.takeException(), isNull);
      // An asynchronous document change must invalidate the captured selection.
      await showConversion();
      editor().replaceText(
        0,
        0,
        'Changed ',
        const TextSelection.collapsed(offset: 8),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('conversion-apply')));
      await tester.pumpAndSettle();
      expect(
        find.text('The document changed. Select the text and try again.'),
        findsOneWidget,
      );
      expect(
        editor().document.toPlainText(),
        startsWith('Changed Before conversion'),
      );
      expect(find.byKey(const Key('table-to-text')), findsNothing);
      await drive(() => model.flush());
      await tester.pumpWidget(const SizedBox.shrink());
      model.dispose();
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
