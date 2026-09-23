import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_page_preview.dart';
import 'package:folio_native/src/document_columns.dart';
import 'package:folio_native/src/document_page_layout.dart';

void main() {
  testWidgets(
    'column controls apply one section with pending text, cancel, preview, history and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1200, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final semantics = tester.ensureSemantics();

      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-columns-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': File('test/fixtures/page-preview.docx').absolute.path,
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        await model.initialize();
      });
      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await tester.pumpAndSettle();
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
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
            () => Future<void>.delayed(const Duration(milliseconds: 6)),
          );
        }
        if (error != null) throw error!;
        expect(done, true);
        await tester.pumpAndSettle();
      }

      Future<void> preview() async {
        await tester.tap(find.text('Layout').first);
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key('document-page-preview')));
        for (var i = 0; i < 500; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 6)),
          );
          if (find
              .byKey(const Key('page-preview-count'))
              .evaluate()
              .isNotEmpty) {
            break;
          }
        }
        await tester.pumpAndSettle();
        expect(find.byType(DocumentPagePreview), findsOneWidget);
        expect(tester.takeException(), isNull);
      }

      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 500 && !ready(); i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 6)),
          );
        }
        expect(ready(), true);
        await tester.pumpAndSettle();
      }

      Future<void> openColumns() async {
        await tester.tap(find.text('Layout').first);
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key('document-columns')));
        await until(
          () => find.byType(DocumentColumnsDialog).evaluate().isNotEmpty,
        );
      }

      final original = jsonEncode(model.workspace);
      await openColumns();
      await tester.tap(find.byKey(const Key('columns-preset-three')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('columns-cancel')));
      await tester.pumpAndSettle();
      expect(jsonEncode(model.workspace), original);
      final editor = tester
          .widget<q.QuillEditor>(find.byKey(const Key('document-editor')))
          .controller;
      editor.replaceText(
        0,
        0,
        'Pending column edit. ',
        const TextSelection.collapsed(offset: 21),
      );
      await openColumns();
      final before = jsonEncode(model.workspace['document']);
      final revision = model.revision;
      await tester.tap(find.byKey(const Key('columns-preset-left')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('columns-separator')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('columns-rtl')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('columns-apply')));
      await until(() => model.revision == revision + 1);
      final edited = jsonEncode(model.workspace['document']);
      final sections =
          model.workspace['document']['attrs']['pageLayout']['sections']
              as List;
      expect(sections[0]['columnLayout']['count'], 2);
      expect(sections[0]['columnLayout']['equalWidth'], false);
      expect(sections[0]['columnLayout']['separator'], true);
      expect(sections[0]['columnLayout']['rtl'], true);
      expect(sections[1]['columnLayout']['count'], 1);
      await preview();
      expect(
        find.bySemanticsLabel(RegExp('Pending column edit', dotAll: true)),
        findsOneWidget,
      );
      tester.view.physicalSize = const Size(1150, 720);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.tap(find.byKey(const Key('page-preview-close')));
      await tester.pumpAndSettle();
      expect(jsonEncode(model.workspace['document']), edited);
      tester.view.physicalSize = const Size(1200, 900);
      await tester.pumpAndSettle();
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      expect(jsonEncode(model.workspace['document']), before);
      await drive(() => model.mutate({'op': 'redo'}, refreshDocument: true));
      expect(jsonEncode(model.workspace['document']), edited);
      late Map pageData;
      await drive(() async {
        pageData = await model.core.call({'op': 'documentPagePreview'});
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/columns-ui.docx',
        });
        final fresh = CoreBridge();
        await fresh.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expectSync(
          jsonEncode(
            (await fresh.call({'op': 'get'}))['workspace']['document'],
          ),
          edited,
        );
      });
      final layout = DocumentPageLayout.build(pageData);
      expect(layout.pages.first.columnBands.first.geometry.columns.length, 2);
      expect(layout.pages.first.columnBands.first.geometry.rtl, true);
      expect(layout.converged, true);
      layout.dispose();
      // ignore: avoid_print
      print('COLUMNS_UI_OUTPUT=${folder.path}/columns-ui.docx');
      semantics.dispose();
    },
  );
}
