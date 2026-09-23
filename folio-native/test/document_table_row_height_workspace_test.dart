import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_page_layout.dart';
import 'package:folio_native/src/document_page_preview.dart';
import 'document_table_row_height_preview_test.dart' as f;

void main() {
  testWidgets(
    'exact-height Apply, Cancel, preview, history, DOCX and recovery agree',
    (tester) async {
      tester.view.physicalSize = const Size(1200, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-row-height-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        final overflowing = f.paragraph('');
        final lines = [
          'Visible row',
          ...List.generate(7, (i) => 'Hidden row $i'),
        ];
        overflowing['content'] = [
          for (var i = 0; i < lines.length; i++) ...[
            if (i > 0) {'type': 'hardBreak'},
            {'type': 'text', 'text': lines[i]},
          ],
        ];
        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [
              f.table([
                f.row([
                  f.cell([overflowing]),
                ]),
                f.row(
                  [
                    f.cell([f.paragraph('Second row')], fill: 'FFF2CC'),
                  ],
                  rule: 'exact',
                  value: 600,
                ),
              ]),
              f.paragraph('After table'),
            ],
          },
        });
        await core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/source.docx',
        });
        await core.call({
          'op': 'importOffice',
          'path': '${folder.path}/source.docx',
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        await model.initialize();
        await model.flush();
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

      void verify(Map snapshot, double height) {
        final result = DocumentPageLayout.build(snapshot);
        try {
          expect(result.converged, true);
          expect(result.pages.length, 1);
          expect(f.tableHeight(result), closeTo(height, .01));
          expect(result.pages.single.visibleText, contains('Visible row'));
          expect(result.pages.single.visibleText, contains('Second row'));
          expect(result.pages.single.visibleText, contains('After table'));
          if (height == 72) {
            expect(
              result.pages.single.visibleText,
              isNot(contains('Hidden row')),
            );
          }
          expect(result.notices, isEmpty);
        } finally {
          result.dispose();
        }
      }

      Future<void> verifyCore(CoreBridge core, double height) async {
        late Map snapshot;
        await drive(() async {
          snapshot = await core.call({'op': 'documentPagePreview'});
        });
        verify(snapshot, height);
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await tester.pumpAndSettle();
      final original = jsonEncode(model.workspace['document']),
          sheets = jsonEncode(model.workspace['sheets']);
      final revision = model.revision;
      await verifyCore(model.core, 216);
      Future<void> draft() async {
        await tester.ensureVisible(
          find.byKey(const ValueKey('table-cell-0-0')).first,
        );
        await tester.tap(find.byKey(const ValueKey('table-cell-0-0')).first);
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key('table-format')));
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key('table-height-rule')));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Exactly').last);
        await tester.pumpAndSettle();
        await tester.enterText(find.byKey(const Key('table-row-height')), '24');
        await tester.tap(find.byKey(const Key('table-format-apply')));
        await tester.pumpAndSettle();
        expect(model.revision, revision);
        expect(jsonEncode(model.workspace['document']), original);
      }

      await draft();
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(model.revision, revision);
      expect(jsonEncode(model.workspace['document']), original);
      await draft();
      await tester.tap(find.byKey(const Key('table-apply')));
      await drive(model.flush);
      expect(model.revision, revision + 1);
      final applied = jsonEncode(model.workspace['document']);
      expect(
        tester
            .getRect(find.byKey(const ValueKey('table-cell-0-0')).first)
            .height,
        32,
      );
      await verifyCore(model.core, 72);

      final beforePreview = jsonEncode(model.workspace);
      await tester.tap(find.text('Layout').first);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('document-page-preview')));
      for (
        var i = 0;
        i < 500 &&
            find.byKey(const Key('page-preview-count')).evaluate().isEmpty;
        i++
      ) {
        await tester.pump(const Duration(milliseconds: 20));
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 6)),
        );
      }
      await tester.pumpAndSettle();
      verify(
        tester
            .widget<DocumentPagePreview>(find.byType(DocumentPagePreview))
            .snapshot,
        72,
      );
      await tester.tap(find.byKey(const Key('page-preview-close')));
      await tester.pumpAndSettle();
      expect(jsonEncode(model.workspace), beforePreview);
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), original);
      await verifyCore(model.core, 216);
      await drive(() => screen.history(true));
      expect(jsonEncode(model.workspace['document']), applied);
      await verifyCore(model.core, 72);
      await drive(model.flush);
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/exact.docx',
        });
      });
      expect(jsonEncode(model.workspace['sheets']), sheets);
      await tester.runAsync(() async {
        final recovered = CoreBridge();
        await recovered.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        verify(await recovered.call({'op': 'documentPagePreview'}), 72);
        await recovered.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/recovered.docx',
        });
        expect(
          await File('${folder.path}/recovered.docx').readAsBytes(),
          await File('${folder.path}/exact.docx').readAsBytes(),
        );
        final imported = CoreBridge();
        await imported.call({
          'op': 'importOffice',
          'path': '${folder.path}/exact.docx',
        });
        verify(await imported.call({'op': 'documentPagePreview'}), 72);
      });
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      model.dispose();
      // ignore: avoid_print
      print('Row-height workspace artifacts: ${folder.path}');
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
