import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_review_view.dart';
import 'package:folio_native/src/document_table_view.dart';

void main() {
  testWidgets(
    'document review switches source views without changing content and reviews with Undo',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-review-display-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': File('test/fixtures/tracked-recording.docx').absolute.path,
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
        expect(find.byType(LinearProgressIndicator), findsNothing);
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
      Future<void> tap(Key key) async {
        await tester.ensureVisible(find.byKey(key));
        await tester.tap(find.byKey(key));
        await drive(screen.flush);
      }

      await tester.tap(find.text('Review').first);
      await tester.pumpAndSettle();
      await tap(const Key('document-review-view'));
      expect(find.byType(DocumentReviewView), findsOneWidget);
      expect(find.text('1 of 5 changes'), findsOneWidget);
      final revision = model.revision;
      final before = model.workspace['document'].toString();
      Map view() => tester
          .widget<DocumentBlockContent>(find.byType(DocumentBlockContent).first)
          .cell;
      Future<void> mode(String label) async {
        await tester.tap(find.byKey(const Key('review-display-mode')));
        await tester.pumpAndSettle();
        await tester.tap(find.text(label).last);
        await drive(screen.flush);
      }

      await mode('Current');
      expect(nodeText(view()), contains('141000'));
      expect(nodeText(view()), contains('Review due Friday.'));
      expect((view()['content'] as List).length, 4);
      await mode('Original');
      expect(nodeText(view()), contains('125000'));
      expect(nodeText(view()), isNot(contains('Review due Friday.')));
      expect((view()['content'] as List).length, 3);
      expect(
        find.byKey(const ValueKey('review-block-word-2-word-3')),
        findsOneWidget,
      );
      expect(model.revision, revision);
      expect(model.workspace['document'].toString(), before);
      await mode('All markup');
      await tap(const Key('review-page-next'));
      expect(find.text('2 of 5 changes'), findsOneWidget);
      await tap(const Key('review-page-previous'));
      expect(find.text('1 of 5 changes'), findsOneWidget);
      await tap(const Key('review-page-accept'));
      expect(find.text('1 of 4 changes'), findsOneWidget);
      expect(model.revision, revision + 1);
      await drive(() => screen.history(false));
      expect(find.text('1 of 5 changes'), findsOneWidget);
      expect(model.workspace['document'].toString(), before);
      await drive(() => screen.history(true));
      expect(find.text('1 of 4 changes'), findsOneWidget);
      await drive(() => screen.history(false));
      await mode('Original');
      tester.view.physicalSize = const Size(960, 740);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tap(const Key('review-page-close'));
      expect(find.byType(DocumentReviewView), findsNothing);
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/unchanged.docx',
        });
        await model.flush();
      });
      await tester.runAsync(() async {
        expect(
          await File('${folder.path}/unchanged.docx').readAsBytes(),
          await File('test/fixtures/tracked-recording.docx').readAsBytes(),
        );
      });
      // Reopening another workspace closes the transient review surface.
      await tap(const Key('document-review-view'));
      await drive(
        () => model.mutate({'op': 'new', 'blank': true}, refreshDocument: true),
      );
      expect(find.byType(DocumentReviewView), findsNothing);
      await drive(model.flush);
    },
  );
}
