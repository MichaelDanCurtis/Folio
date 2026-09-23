import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_page_preview.dart';

void main() {
  testWidgets(
    'page preview uses saved and pending edits without changing history or field caches',
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
        folder = await Directory.systemTemp.createTemp('folio-pagination-ui-');
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
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
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

      final before = jsonEncode(model.workspace);
      await preview();
      final count = tester
          .widget<Text>(find.byKey(const Key('page-preview-count')))
          .data!;
      expect(count, startsWith('Page 1 of '));
      expect(int.parse(count.split(' ').last), greaterThan(2));
      expect(
        find.bySemanticsLabel(RegExp(r'Page 1\. .*Page 1 of ', dotAll: true)),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('page-preview-next')));
      await tester.pumpAndSettle();
      expect(
        find.bySemanticsLabel(RegExp(r'Page 2\. .*Page 2 of ', dotAll: true)),
        findsOneWidget,
      );
      // MainFlutterWindow enforces this native minimum; the standalone preview
      // separately covers smaller panels.
      tester.view.physicalSize = const Size(1150, 720);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.tap(find.byKey(const Key('page-preview-close')));
      await tester.pumpAndSettle();
      expect(jsonEncode(model.workspace), before);
      tester.view.physicalSize = const Size(1200, 900);
      await tester.pumpAndSettle();
      final editor = tester
          .widget<q.QuillEditor>(find.byKey(const Key('document-editor')))
          .controller;
      editor.replaceText(
        0,
        0,
        'Pending preview edit. ',
        const TextSelection.collapsed(offset: 22),
      );
      await preview();
      expect(
        find.bySemanticsLabel(RegExp('Pending preview edit', dotAll: true)),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('page-preview-close')));
      await tester.pumpAndSettle();
      await drive(screen.flush);
      final edited = jsonEncode(model.workspace['document']);
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      expect(jsonEncode(model.workspace['document']), isNot(edited));
      await drive(() => model.mutate({'op': 'redo'}, refreshDocument: true));
      expect(jsonEncode(model.workspace['document']), edited);
      await drive(() async {
        final data = await model.core.call({
          'op': 'documentHeaders',
          'section': 0,
          'kind': 'footer',
          'variant': 'default',
        });
        final fields = (data['document']['content'][0]['content'] as List)
            .where((n) => n['type'] == 'pageField');
        expectSync(fields.map((n) => n['text']), ['99', '99']);
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/pagination-ui.docx',
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
      // ignore: avoid_print
      print('PAGINATION_UI_OUTPUT=${folder.path}/pagination-ui.docx');
      semantics.dispose();
    },
  );
}
