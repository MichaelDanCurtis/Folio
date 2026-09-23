import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_headers.dart';

void main() {
  testWidgets(
    'header editing unlinks a section, preserves body, supports history and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-headers-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': File('test/fixtures/headers-sections.docx').absolute.path,
        });
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
        for (var i = 0; i < 500; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 6)),
          );
          if (done && find.byType(LinearProgressIndicator).evaluate().isEmpty) {
            break;
          }
        }
        if (error != null) throw error!;
        expect(done, true);
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
      Future<void> tap(String key) async {
        await tester.pumpAndSettle();
        await tester.ensureVisible(find.byKey(Key(key)));
        await tester.tap(find.byKey(Key(key)));
        await drive(screen.flush);
      }

      Future<void> choose(String key, String value) async {
        await tap(key);
        await tester.tap(find.text(value).last);
        await drive(screen.flush);
      }

      Future<void> open() async {
        await tester.tap(find.text('Layout').first);
        await tester.pumpAndSettle();
        await tap('document-headers');
        expect(find.byType(DocumentHeadersDialog), findsOneWidget);
      }

      q.QuillController controller() => tester
          .widget<q.QuillEditor>(
            find.descendant(
              of: find.byType(DocumentHeadersDialog),
              matching: find.byType(q.QuillEditor),
            ),
          )
          .controller;
      final baseline = model.workspace['document'];
      final body = screen.editor.document.toPlainText();
      await open();
      await tap('header-cancel');
      expect(model.workspace['document'], baseline);
      await open();
      await choose('header-section-0', 'Section 2');
      expect(find.textContaining('Sections 1, 2'), findsOneWidget);
      await tap('header-link');
      expect(find.textContaining('will start its own header'), findsOneWidget);
      final editor = controller();
      editor.replaceText(
        0,
        0,
        'Chapter ',
        const TextSelection.collapsed(offset: 8),
      );
      await tap('header-first');
      await tap('header-even');
      await tap('header-apply');
      expect(model.error, isNull);
      expect(screen.editor.document.toPlainText(), body);
      late Map one, two, three;
      await tester.runAsync(() async {
        Future<Map> read(int section) => model.core.call({
          'op': 'documentHeaders',
          'section': section,
          'kind': 'header',
          'variant': 'default',
        });
        one = await read(0);
        two = await read(1);
        three = await read(2);
      });
      expect(one['document']['content'][0]['content'][0]['text'], 'Shared 😀');
      expect(
        two['document']['content'][0]['content'][0]['text'],
        'Chapter Shared 😀',
      );
      expect(
        three['document']['content'][0]['content'][0]['text'],
        'Independent',
      );
      expect(two['linked'], false);
      expect(two['differentFirstPage'], true);
      expect(one['differentOddEven'], true);
      final edited = model.workspace['document'];
      await drive(() => screen.history(false));
      expect(model.workspace['document'], baseline);
      await drive(() => screen.history(true));
      expect(model.workspace['document'], edited);
      await open();
      await choose('header-section-0', 'Section 2');
      await choose('header-kind-header', 'Footer');
      await choose('header-variant-default', 'First page');
      await tap('header-link');
      controller().replaceText(
        0,
        0,
        'Chapter footer',
        const TextSelection.collapsed(offset: 14),
      );
      await tap('header-apply');
      expect(model.error, isNull);
      expect(screen.editor.document.toPlainText(), body);
      await drive(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/headers-ui.docx',
        });
      });
      final finalDoc = model.workspace['document'];
      await tester.runAsync(() async {
        final reopened = await model.core.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expect(reopened['workspace']['document'], finalDoc);
        final footer = await model.core.call({
          'op': 'documentHeaders',
          'section': 1,
          'kind': 'footer',
          'variant': 'first',
        });
        expect(
          footer['document']['content'][0]['content'][0]['text'],
          'Chapter footer',
        );
        expect(footer['linked'], false);
      });
      expect(tester.takeException(), isNull);
      // ignore: avoid_print
      print('HEADERS_UI_OUTPUT=${folder.path}/headers-ui.docx');
    },
  );
}
