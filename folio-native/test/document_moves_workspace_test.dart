import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_table_view.dart';

void main() {
  testWidgets(
    'linked moves navigate, resolve through filters and recover with Undo',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-moves-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': File('test/fixtures/moved-revisions.docx').absolute.path,
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
      Future<void> tap(String key) async {
        final finder = find.byKey(Key(key));
        await tester.ensureVisible(finder);
        await tester.tap(finder);
        await drive(screen.flush);
      }

      await tester.tap(find.text('Review').first);
      await tester.pumpAndSettle();
      await tap('document-review-view');
      expect(find.text('1 of 3 changes'), findsOneWidget);
      final original = model.workspace['document'].toString();
      final revision = model.revision;
      await tap('review-page-linked-location');
      expect(find.text('2 of 3 changes'), findsOneWidget);
      await tap('review-page-linked-location');
      expect(find.text('1 of 3 changes'), findsOneWidget);
      Future<List<String>> mode(String label) async {
        await tester.tap(find.byKey(const Key('review-display-mode')));
        await tester.pumpAndSettle();
        await tester.tap(find.text(label).last);
        await drive(screen.flush);
        final document = tester
            .widget<DocumentBlockContent>(
              find.byType(DocumentBlockContent).first,
            )
            .cell;
        return [for (final n in document['content']) nodeText(n)];
      }

      expect(await mode('Current'), [
        'Planning notes',
        'Before:  stays.',
        'After: Budget 😀 here.',
        'Unrelated note.',
      ]);
      expect(await mode('Original'), [
        'Planning notes',
        'Before: Budget 😀 stays.',
        'After:  here.',
        '',
      ]);
      expect(model.revision, revision);
      expect(model.workspace['document'].toString(), original);
      await mode('All markup');
      await tap('review-page-reject');
      expect(find.text('1 of 1 changes'), findsOneWidget);
      await drive(() => screen.history(false));
      expect(find.text('1 of 3 changes'), findsOneWidget);
      await tap('review-page-close');
      await tap('document-revisions');
      Future<void> filterMovedFrom() async {
        await tester.tap(find.byType(DropdownButtonFormField<String>).at(1));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Moved from').last);
        await tester.pumpAndSettle();
        expect(find.text('Accept all shown (1)'), findsOneWidget);
      }

      await filterMovedFrom();
      await tap('revision-linked-location');
      expect(find.text('Accept all shown (3)'), findsOneWidget);
      await filterMovedFrom();
      await tap('revision-accept-shown');
      expect(find.text('Accept all shown (1)'), findsOneWidget);
      expect(find.text('Moved to'), findsNothing);
      expect(find.text('Morgan · Document'), findsWidgets);
      await tester.tap(find.byTooltip('Undo review'));
      await drive(screen.flush);
      expect(find.text('Accept all shown (3)'), findsOneWidget);
      tester.view.physicalSize = const Size(960, 740);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tap('revision-reject');
      expect(find.text('Accept all shown (1)'), findsOneWidget);
      await tap('revision-close');
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/moves-reviewed.docx',
        });
        await model.flush();
      });
      final finalDocument = model.workspace['document'];
      await tester.runAsync(() async {
        final loaded = await model.core.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expect(loaded['workspace']['document'], finalDocument);
        final pending = await model.core.call({'op': 'documentRevisions'});
        expect((pending['revisions'] as List).length, 1);
        expect(pending['revisions'][0]['author'], 'Morgan');
      });
      expect(tester.takeException(), isNull);
      // ignore: avoid_print
      print('MOVES_UI_OUTPUT=${folder.path}/moves-reviewed.docx');
    },
  );
}
