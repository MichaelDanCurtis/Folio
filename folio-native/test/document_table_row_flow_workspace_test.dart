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
    'row page-break Apply, Cancel, preview, history, DOCX and recovery agree',
    (tester) async {
      tester.view.physicalSize = const Size(1200, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-row-flow-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        Map paragraphLines(String prefix, int count) {
          final p = f.paragraph('');
          p['content'] = [
            for (var i = 0; i < count; i++) ...[
              if (i > 0) {'type': 'hardBreak'},
              {
                'type': 'text',
                'text': '$prefix${i.toString().padLeft(2, '0')}',
              },
            ],
          ];
          return p;
        }

        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [
              paragraphLines('Before', 30),
              f.table([
                f.row([
                  f.cell([paragraphLines('A', 18)]),
                  f.cell([paragraphLines('B', 18)]),
                ]),
              ], columns: 2),
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

      void verify(Map snapshot, bool allowed) {
        final result = DocumentPageLayout.build(snapshot);
        try {
          expect(result.converged, true);
          expect(result.pages.length, 2);
          expect(result.pages.first.visibleText.contains('A00'), allowed);
          final all = result.pages.map((p) => p.visibleText).join('\n');
          for (final prefix in ['A', 'B']) {
            expect(
              RegExp(
                '$prefix[0-9]{2}',
              ).allMatches(all).map((m) => m[0]).toList(),
              List.generate(
                18,
                (i) => '$prefix${i.toString().padLeft(2, '0')}',
              ),
            );
          }
          expect(all, contains('After table'));
          expect(result.notices, isEmpty);
        } finally {
          result.dispose();
        }
      }

      Future<void> verifyCore(CoreBridge core, bool allowed) async {
        late Map snapshot;
        await drive(() async {
          snapshot = await core.call({'op': 'documentPagePreview'});
        });
        verify(snapshot, allowed);
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await tester.pumpAndSettle();
      final original = jsonEncode(model.workspace['document']),
          sheets = jsonEncode(model.workspace['sheets']);
      final revision = model.revision;
      await verifyCore(model.core, true);
      Future<void> draft() async {
        await tester.ensureVisible(
          find.byKey(const ValueKey('table-cell-0-0')).first,
        );
        await tester.tap(find.byKey(const ValueKey('table-cell-0-0')).first);
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key('table-format')));
        await tester.pumpAndSettle();
        final choice = find.byKey(const Key('table-allow-row-break'));
        await tester.ensureVisible(choice);
        expect(tester.widget<CheckboxListTile>(choice).value, true);
        await tester.tap(choice);
        await tester.pumpAndSettle();
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
      await verifyCore(model.core, false);

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
        false,
      );
      await tester.tap(find.byKey(const Key('page-preview-close')));
      await tester.pumpAndSettle();
      expect(jsonEncode(model.workspace), beforePreview);
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), original);
      await verifyCore(model.core, true);
      await drive(() => screen.history(true));
      expect(jsonEncode(model.workspace['document']), applied);
      await verifyCore(model.core, false);
      await drive(model.flush);
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/kept.docx',
        });
      });
      expect(jsonEncode(model.workspace['sheets']), sheets);
      await tester.runAsync(() async {
        final recovered = CoreBridge();
        await recovered.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        verify(await recovered.call({'op': 'documentPagePreview'}), false);
        await recovered.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/recovered.docx',
        });
        expect(
          await File('${folder.path}/recovered.docx').readAsBytes(),
          await File('${folder.path}/kept.docx').readAsBytes(),
        );
        final imported = CoreBridge();
        await imported.call({
          'op': 'importOffice',
          'path': '${folder.path}/kept.docx',
        });
        verify(await imported.call({'op': 'documentPagePreview'}), false);
      });
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      model.dispose();
      // ignore: avoid_print
      print('Row-flow workspace artifacts: ${folder.path}');
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
