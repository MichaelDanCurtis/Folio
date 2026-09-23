import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_page_setup.dart';

void main() {
  testWidgets(
    'page setup preserves editing, section splits, history and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-pages-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': File('test/fixtures/page-sections.docx').absolute.path,
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
      Future<void> tap(String key) async {
        await tester.pumpAndSettle();
        await tester.ensureVisible(find.byKey(Key(key)));
        await tester.tap(find.byKey(Key(key)));
        await drive(screen.flush);
      }

      Future<void> open() async {
        await tester.tap(find.text('Layout').first);
        await tester.pumpAndSettle();
        await tap('document-page-setup');
        expect(find.byType(DocumentPageSetupDialog), findsOneWidget);
      }

      Future<void> choose(String key, String label) async {
        await tester.ensureVisible(find.byKey(Key(key)));
        await tester.tap(find.byKey(Key(key)));
        await tester.pumpAndSettle();
        await tester.tap(find.text(label).last);
        await tester.pumpAndSettle();
      }

      final initialRevision = model.revision;
      await open();
      await tap('page-setup-cancel');
      expect(model.revision, initialRevision);
      // Ordinary typing must be baked into the layout operation, not discarded.
      screen.editor.replaceText(
        0,
        0,
        'New ',
        const TextSelection.collapsed(offset: 4),
      );
      await drive(screen.flush);
      final before = model.workspace['document'].toString();
      await open();
      await choose('page-orientation-portrait', 'Landscape');
      await tester.enterText(find.byKey(const Key('page-left')), '0.5');
      await tap('page-setup-apply');
      expect(
        screen.editor.document.toPlainText().startsWith('New First section.'),
        isTrue,
      );
      expect(
        tester.getSize(find.byKey(const Key('document-paper'))).width,
        1056,
      );
      final layout = model.workspace['document']['attrs']['pageLayout'];
      expect(layout['sections'][0]['geometry']['left'], 720);
      expect(layout['sections'][1]['geometry']['left'], 1080);
      await drive(() => screen.history(false));
      expect(model.workspace['document'].toString(), before);
      expect(
        tester.getSize(find.byKey(const Key('document-paper'))).width,
        816,
      );
      await drive(() => screen.history(true));
      expect(
        tester.getSize(find.byKey(const Key('document-paper'))).width,
        1056,
      );
      // Quill's real Enter representation duplicates the paragraph identity.
      final boundary = screen.editor.document.toPlainText().indexOf(
        'boundary.',
      );
      screen.editor.replaceText(
        boundary,
        0,
        '\n',
        TextSelection.collapsed(offset: boundary + 1),
      );
      await drive(screen.flush);
      await open();
      expect(find.text('Section 1 of 2'), findsOneWidget);
      await tap('page-setup-cancel');
      final end = screen.editor.document.toPlainText().indexOf(
        'Second section.',
      );
      screen.editor.updateSelection(
        TextSelection.collapsed(offset: end),
        q.ChangeSource.local,
      );
      await open();
      expect(find.text('Section 2 of 2'), findsOneWidget);
      await tester.enterText(find.byKey(const Key('page-top')), '0.75');
      await tap('page-setup-apply');
      expect(
        model.workspace['document']['attrs']['pageLayout']['sections'].length,
        2,
      );
      expect(
        model
            .workspace['document']['attrs']['pageLayout']['sections'][1]['geometry']['top'],
        1080,
      );
      final document = model.workspace['document'];
      await drive(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/page-ui.docx',
        });
      });
      await tester.runAsync(() async {
        final loaded = await model.core.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expect(loaded['workspace']['document'], document);
      });
      tester.view.physicalSize = const Size(960, 740);
      await tester.pumpAndSettle();
      expect(
        tester.getSize(find.byKey(const Key('document-paper'))).width,
        1056,
      );
      expect(tester.takeException(), isNull);
      // ignore: avoid_print
      print('PAGE_UI_OUTPUT=${folder.path}/page-ui.docx');
    },
  );
}
