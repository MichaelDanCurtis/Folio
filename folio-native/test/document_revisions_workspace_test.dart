import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_revisions.dart';

void main() {
  testWidgets(
    'Track Changes applies, rejects, filters, undoes and recovers through the real engine',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final capture = Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1';
      if (capture) {
        await tester.runAsync(() async {
          for (final (family, path) in [
            ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
            ('Aptos', '/System/Library/Fonts/Supplemental/Arial.ttf'),
            (
              'MaterialIcons',
              '/opt/homebrew/share/flutter/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
            ),
          ]) {
            await (FontLoader(family)..addFont(
                  Future.value(
                    ByteData.sublistView(await File(path).readAsBytes()),
                  ),
                ))
                .load();
          }
        });
      }
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-revisions-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': File('test/fixtures/revisions.docx').absolute.path,
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

      final boundary = GlobalKey();
      await tester.pumpWidget(
        RepaintBoundary(
          key: boundary,
          child: FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
        ),
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
        final finder = find.byKey(key);
        await tester.ensureVisible(finder);
        await tester.pumpAndSettle();
        await tester.tap(finder);
        await drive(screen.flush);
      }

      Future<void> waitForRows(int count) async {
        for (var i = 0; i < 200; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
          if (find.byType(DocumentRevisionsDialog).evaluate().isNotEmpty &&
              find.byKey(const Key('revision-row-0')).evaluate().isEmpty &&
              count == 0) {
            break;
          }
          if (find.text('Accept all shown ($count)').evaluate().isNotEmpty &&
              find.byType(LinearProgressIndicator).evaluate().isEmpty) {
            break;
          }
        }
        await tester.pumpAndSettle();
        expect(find.text('Accept all shown ($count)'), findsOneWidget);
      }

      await tester.tap(find.text('Review').first);
      await tester.pumpAndSettle();
      await tap(const Key('document-revisions'));
      await waitForRows(2);
      expect(
        find.text(
          'Recording is off. Use Record changes to track new edits.',
        ),
        findsOneWidget,
      );
      expect(find.byType(RevisionPreview), findsWidgets);
      if (capture) {
        await tester.runAsync(() async {
          final image =
              await (boundary.currentContext!.findRenderObject()
                      as RenderRepaintBoundary)
                  .toImage(pixelRatio: 1.2);
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          await File(
            '../../work/revisions-widget.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
          image.dispose();
        });
      }
      await tap(const Key('revision-accept'));
      await waitForRows(1);
      expect(
        (model.workspace['document']['content'][0]['attrs']['revisionPreview']
                as List)
            .any((s) => s['kind'] == 'delete'),
        isFalse,
      );
      await tester.tap(find.byTooltip('Undo review'));
      await waitForRows(2);
      await tester.tap(find.byTooltip('Redo review'));
      await waitForRows(1);
      await tap(const Key('revision-reject'));
      await waitForRows(0);
      expect(
        model.workspace['document']['content'][0]['content'][0]['text'],
        'Keep ',
      );
      await tester.tap(find.byTooltip('Undo review'));
      await waitForRows(1);
      await tester.tap(find.byTooltip('Undo review'));
      await waitForRows(2);
      await tester.tap(find.byType(DropdownButtonFormField<String>).at(1));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Deleted content').last);
      await tester.pumpAndSettle();
      expect(find.text('Accept all shown (1)'), findsOneWidget);
      await tap(const Key('revision-reject-shown'));
      // The exhausted filter resets so the still-pending insertion stays visible.
      await waitForRows(1);
      await tap(const Key('revision-accept-shown'));
      await waitForRows(0);
      final result = model.workspace['document'].toString();
      expect(result, contains('old😀'));
      expect(result, contains('new'));
      await tap(const Key('revision-close'));
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/reviewed.docx',
        });
        await model.flush();
      });
      final finalDocument = model.workspace['document'];
      await tester.runAsync(() async {
        final recovered = await model.core.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expect(recovered['workspace']['document'], finalDocument);
        final r = await model.core.call({'op': 'documentRevisions'});
        expect(r['revisions'], isEmpty);
      });
      // Reopen a fresh source then prove a stale review cannot change the document.
      await drive(
        () => model.mutate({
          'op': 'importOffice',
          'path': File('test/fixtures/revisions.docx').absolute.path,
        }, refreshDocument: true),
      );
      tester.view.physicalSize = const Size(960, 680);
      await tester.pumpAndSettle();
      await tap(const Key('document-revisions'));
      await waitForRows(2);
      expect(tester.takeException(), isNull);
      await drive(
        () => model.mutate({'op': 'title', 'title': 'Changed while reviewing'}),
      );
      final before = model.workspace['document'];
      await tap(const Key('revision-accept'));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('revision-error')), findsOneWidget);
      expect(model.workspace['document'], before);
      await tap(const Key('revision-close'));
      expect(tester.takeException(), isNull);
      // Emit the file location for independent package validation.
      // ignore: avoid_print
      print('REVISION_UI_OUTPUT=${folder.path}/reviewed.docx');
    },
  );
}
