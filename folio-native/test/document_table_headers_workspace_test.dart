import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_page_layout.dart';
import 'package:folio_native/src/document_page_preview.dart';
import 'package:folio_native/src/document_tables.dart';

void main() {
  testWidgets(
    'repeating headings survive actual table Apply, Cancel, Undo, preview, export and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1200, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp(
          'folio-table-headers-ui-',
        );
        final font = File('/System/Library/Fonts/Supplemental/Arial.ttf');
        if (await font.exists()) {
          await (FontLoader(
            'Aptos',
          )..addFont(font.readAsBytes().then(ByteData.sublistView))).load();
        }
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': File('test/fixtures/table-headers.docx').absolute.path,
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

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await tester.pumpAndSettle();
      final original = jsonEncode(model.workspace['document']),
          revision = model.revision;
      expect(
        DocumentTable(model.workspace['document']['content'][0]).headerRows,
        2,
      );
      Future<void> changeHeader() async {
        await tester.ensureVisible(
          find.byKey(const ValueKey('table-cell-0-0')).first,
        );
        await tester.tap(find.byKey(const ValueKey('table-cell-0-0')).first);
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key('table-format')));
        await tester.pumpAndSettle();
        await tester.ensureVisible(
          find.byKey(const Key('table-repeat-header')),
        );
        expect(
          tester
              .widget<CheckboxListTile>(
                find.byKey(const Key('table-repeat-header')),
              )
              .value,
          true,
        );
        await tester.tap(find.byKey(const Key('table-repeat-header')));
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key('table-format-apply')));
        await tester.pumpAndSettle();
        expect(jsonEncode(model.workspace['document']), original);
      }

      await changeHeader();
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(model.revision, revision);
      expect(jsonEncode(model.workspace['document']), original);
      await changeHeader();
      await tester.tap(find.byKey(const Key('table-apply')));
      await drive(model.flush);
      expect(model.revision, revision + 1);
      expect(
        DocumentTable(model.workspace['document']['content'][0]).headerRows,
        0,
      );
      final off = jsonEncode(model.workspace['document']);
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      for (final step in [false, true, false]) {
        await drive(() => screen.history(step));
        expect(jsonEncode(model.workspace['document']), step ? off : original);
      }
      await drive(model.flush);
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
      final snapshot = tester
          .widget<DocumentPagePreview>(find.byType(DocumentPagePreview))
          .snapshot;
      final layout = DocumentPageLayout.build(snapshot);
      try {
        expect(layout.converged, true);
        expect(layout.pages.length, greaterThan(1));
        expect(layout.notices, isEmpty);
        for (var i = 0; i < layout.pages.length; i++) {
          final page = layout.pages[i];
          expect(page.visibleText, contains('Quarterly accounts'));
          expect(
            page.visibleText,
            contains('Amount - page ${page.number} of ${layout.pages.length}'),
          );
          if (i > 0) {
            expect(
              page.content.where((p) => p.area == 'tableHeader').length,
              1,
            );
            await tester.tap(find.byKey(const Key('page-preview-next')));
            await tester.pumpAndSettle();
          }
          final boundary = tester.renderObject<RenderRepaintBoundary>(
            find
                .ancestor(
                  of: find.byKey(ValueKey('page-preview-paper-$i')),
                  matching: find.byType(RepaintBoundary),
                )
                .first,
          );
          await tester.runAsync(() async {
            final image = await boundary.toImage(pixelRatio: 1.2);
            try {
              final bytes = (await image.toByteData(
                format: ui.ImageByteFormat.png,
              ))!;
              await File(
                '${folder.path}/page-${i + 1}.png',
              ).writeAsBytes(bytes.buffer.asUint8List());
            } finally {
              image.dispose();
            }
          });
        }
      } finally {
        layout.dispose();
      }
      await tester.tap(find.byKey(const Key('page-preview-close')));
      await tester.pumpAndSettle();
      expect(jsonEncode(model.workspace), beforePreview);
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/enabled.docx',
        });
      });
      await drive(() => screen.history(true));
      await drive(model.flush);
      expect(jsonEncode(model.workspace['document']), off);
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/disabled.docx',
        });
      });
      await tester.runAsync(() async {
        final saved = jsonDecode(
          await File('${folder.path}/workspace.folio').readAsString(),
        );
        expect(saved['document'], model.workspace['document']);
        final fresh = CoreBridge();
        await fresh.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        final reopened = await fresh.call({'op': 'documentPagePreview'});
        expect(DocumentTable(reopened['document']['content'][0]).headerRows, 0);
        final offLayout = DocumentPageLayout.build(reopened);
        try {
          expect(
            offLayout.pages
                .expand((p) => p.content)
                .where((p) => p.area == 'tableHeader'),
            isEmpty,
          );
        } finally {
          offLayout.dispose();
        }
        await fresh.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/reopened.docx',
        });
        expect(
          await File('${folder.path}/reopened.docx').readAsBytes(),
          await File('${folder.path}/disabled.docx').readAsBytes(),
        );
      });
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      model.dispose();
      // Retained, disposable evidence; no native preview is launched by this test.
      debugPrint('Table header UI evidence: ${folder.path}');
    },
  );
}
