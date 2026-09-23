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
    'page fields and section numbering use real dialog drafts, history and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp(
          'folio-page-numbers-ui-',
        );
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

      final before = model.workspace['officeDocument'];
      final body = screen.editor.document.toPlainText();
      await open();
      await choose('header-section-0', 'Section 2');
      await choose('header-kind-header', 'Footer');
      await tap('header-link');
      final c = controller();
      final end = c.document.length - 1;
      c.replaceText(end, 0, ' Page ', TextSelection.collapsed(offset: end + 6));
      await tap('insert-page-field');
      await tester.tap(find.text('Page number').last);
      await drive(screen.flush);
      c.replaceText(
        c.selection.start,
        0,
        ' of ',
        TextSelection.collapsed(offset: c.selection.start + 4),
      );
      await tap('insert-page-field');
      await tester.tap(find.text('Total pages').last);
      await drive(screen.flush);
      await tap('header-page-numbers');
      await choose('page-number-format', 'i, ii, iii');
      await tap('page-number-restart');
      await tester.enterText(find.byKey(const Key('page-number-start')), '5');
      await tap('page-number-chapter');
      await choose('page-number-heading', 'Heading 2');
      await choose('page-number-separator', ': (colon)');
      await tap('page-number-done');
      expect(model.workspace['officeDocument'], before);
      await tap('header-apply');
      expect(model.error, isNull);
      expect(screen.editor.document.toPlainText(), body);
      final applied = model.workspace['officeDocument'];
      late Map result;
      await tester.runAsync(() async {
        result = await model.core.call({
          'op': 'documentHeaders',
          'section': 1,
          'kind': 'footer',
          'variant': 'default',
        });
      });
      expect(result['pageNumbering'], {
        'format': 'lowerRoman',
        'start': 5,
        'chapterStyle': 2,
        'chapterSeparator': 'colon',
      });
      expect(result['linked'], false);
      final children = result['document']['content'][0]['content'] as List;
      expect(
        children
            .where((n) => n['type'] == 'pageField')
            .map((n) => n['attrs']['kind'])
            .toList(),
        ['PAGE', 'NUMPAGES'],
      );
      await drive(() => screen.history(false));
      expect(model.workspace['officeDocument'], before);
      await drive(() => screen.history(true));
      expect(model.workspace['officeDocument'], applied);
      await open();
      await choose('header-section-0', 'Section 2');
      await choose('header-kind-header', 'Footer');
      expect(find.text('‹Page number›'), findsOneWidget);
      expect(find.text('‹Total pages›'), findsOneWidget);
      await tap('header-page-numbers');
      await tester.enterText(find.byKey(const Key('page-number-start')), '9');
      await tap('page-number-done');
      await tap('header-cancel');
      expect(model.workspace['officeDocument'], applied);
      await drive(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/page-numbers-ui.docx',
        });
      });
      await tester.runAsync(() async {
        await model.core.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        final restored = await model.core.call({
          'op': 'documentHeaders',
          'section': 1,
          'kind': 'footer',
          'variant': 'default',
        });
        expect(restored['document'], result['document']);
        expect(restored['pageNumbering'], result['pageNumbering']);
      });
      expect(tester.takeException(), isNull);
      // ignore: avoid_print
      print('PAGE_NUMBERS_UI_OUTPUT=${folder.path}/page-numbers-ui.docx');
    },
  );
}
