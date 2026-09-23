import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_sections.dart';

void main() {
  testWidgets(
    'section dialog splits selected paragraph, removes and restores exact history',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-sections-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': File('test/fixtures/section-breaks.docx').absolute.path,
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
        await tap('document-section-breaks');
        expect(find.byType(DocumentSectionBreakDialog), findsOneWidget);
      }

      // A selected range must not silently become an insertion at its start.
      screen.editor.updateSelection(
        const TextSelection(baseOffset: 0, extentOffset: 5),
        q.ChangeSource.local,
      );
      final initialRevision = model.revision;
      await open();
      expect(
        tester
            .widget<FilledButton>(find.byKey(const Key('section-break-apply')))
            .onPressed,
        isNull,
      );
      await tap('section-break-cancel');
      expect(model.revision, initialRevision);
      // Flush ordinary typing before using the resulting caret as the split point.
      screen.editor.replaceText(
        0,
        0,
        'New ',
        const TextSelection.collapsed(offset: 4),
      );
      await drive(screen.flush);
      screen.editor.updateSelection(
        const TextSelection.collapsed(offset: 12),
        q.ChangeSource.local,
      );
      final before = model.workspace['document'];
      await open();
      await tester.tap(find.byKey(const Key('section-break-type')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Continuous').last);
      await tester.pumpAndSettle();
      await tap('section-break-apply');
      expect(model.error, isNull);
      expect(
        screen.editor.document.toPlainText(),
        'New Alpha 😀\n beta\nSecond paragraph.\n',
      );
      expect(screen.editor.selection.baseOffset, 13);
      expect(
        model.workspace['document']['attrs']['pageLayout']['sections'].length,
        2,
      );
      final inserted = model.workspace['document'];
      await drive(() => screen.history(false));
      expect(model.workspace['document'], before);
      await drive(() => screen.history(true));
      expect(model.workspace['document'], inserted);
      // The restored caret belongs to the second section in Page setup.
      screen.editor.updateSelection(
        const TextSelection.collapsed(offset: 14),
        q.ChangeSource.local,
      );
      await tap('document-page-setup');
      expect(find.text('Section 2 of 2'), findsOneWidget);
      await tester.enterText(find.byKey(const Key('page-left')), '0.5');
      await tap('page-setup-apply');
      expect(model.error, isNull);
      expect(
        model
            .workspace['document']['attrs']['pageLayout']['sections'][0]['geometry']['left'],
        1440,
      );
      expect(
        model
            .workspace['document']['attrs']['pageLayout']['sections'][1]['geometry']['left'],
        720,
      );
      await open();
      await tester.tap(find.text('Remove').first);
      await tester.pumpAndSettle();
      expect(find.textContaining('Section 2’s page settings'), findsOneWidget);
      await tap('section-break-apply');
      expect(model.error, isNull);
      expect(
        screen.editor.document.toPlainText(),
        'New Alpha 😀 beta\nSecond paragraph.\n',
      );
      expect(
        model.workspace['document']['attrs']['pageLayout']['sections'].length,
        1,
      );
      expect(
        model
            .workspace['document']['attrs']['pageLayout']['sections'][0]['geometry']['left'],
        720,
      );
      await drive(() => screen.history(false));
      expect(
        model.workspace['document']['attrs']['pageLayout']['sections'].length,
        2,
      );
      await drive(() => screen.history(true));
      final finalDoc = model.workspace['document'];
      await drive(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/sections-ui.docx',
        });
      });
      await tester.runAsync(() async {
        final loaded = await model.core.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expect(loaded['workspace']['document'], finalDoc);
      });
      expect(tester.takeException(), isNull);
      // ignore: avoid_print
      print('SECTIONS_UI_OUTPUT=${folder.path}/sections-ui.docx');
    },
  );
}
