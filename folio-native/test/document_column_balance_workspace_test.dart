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

void main() {
  testWidgets(
    'imported column balancing and its compatibility opt-out survive preview, export and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1200, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final semantics = tester.ensureSemantics();
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
        expect(
          done,
          true,
          reason: 'The queued workspace operation must complete.',
        );
        await tester.pumpAndSettle();
      }

      late Directory folder;
      var realFont = false;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp(
          'folio-column-balance-ui-',
        );
        final font = File('/System/Library/Fonts/Supplemental/Arial.ttf');
        if (await font.exists()) {
          final loader = FontLoader('Aptos')
            ..addFont(font.readAsBytes().then(ByteData.sublistView));
          await loader.load();
          realFont = true;
        }
      });
      final followingY = <double>[];
      for (final disabled in [false, true]) {
        final name = disabled ? 'disabled' : 'source';
        final source = File('test/fixtures/column-balance-$name.docx').absolute;
        late WorkspaceModel model;
        await tester.runAsync(() async {
          final core = CoreBridge();
          await core.call({'op': 'new', 'blank': true});
          await core.call({'op': 'importOffice', 'path': source.path});
          model = WorkspaceModel(core, '${folder.path}/$name.folio');
          await model.initialize();
          await model.flush();
        });
        try {
          await tester.pumpWidget(
            FolioApp(
              model: model,
              agentDirectory: '${folder.path}/agent-$name',
            ),
          );
          await tester.pumpAndSettle();
          final before = jsonEncode(model.workspace);
          late List<int> recovery;
          await tester.runAsync(() async {
            recovery = await File('${folder.path}/$name.folio').readAsBytes();
          });
          await tester.tap(find.text('Layout').first);
          await tester.pumpAndSettle();
          await tester.tap(find.byKey(const Key('document-page-preview')));
          for (var i = 0; i < 500; i++) {
            await tester.pump(const Duration(milliseconds: 20));
            await tester.runAsync(
              () => Future<void>.delayed(const Duration(milliseconds: 6)),
            );
            if (find
                .byKey(const Key('page-preview-count'))
                .evaluate()
                .isNotEmpty) {
              break;
            }
          }
          await tester.pumpAndSettle();
          expect(tester.takeException(), isNull);
          final widget = tester.widget<DocumentPagePreview>(
            find.byType(DocumentPagePreview),
          );
          expect(widget.snapshot['noColumnBalance'], disabled);
          final layout = DocumentPageLayout.build(widget.snapshot);
          try {
            expect(layout.converged, true);
            expect(layout.pages.length, 1);
            final page = layout.pages.single;
            final columnText = page.content
                .where((p) => p.area == 'body' && p.bodyIndex! <= 20)
                .toList();
            expect(
              columnText.map((p) => p.offset.dx).toSet().length,
              disabled ? 1 : 2,
            );
            expect(page.columnBands.first.balanced, !disabled);
            expect(
              columnText.map((p) => p.text).join(' '),
              contains('Line 19.'),
            );
            followingY.add(
              page.content
                  .singleWhere((p) => p.area == 'body' && p.bodyIndex == 21)
                  .offset
                  .dy,
            );
            expect(
              find.byWidgetPredicate(
                (w) =>
                    w is Semantics &&
                    (w.properties.label?.startsWith('Page 1.') ?? false) &&
                    (w.properties.label?.contains('Full-width text follows') ??
                        false),
              ),
              findsOneWidget,
            );
          } finally {
            layout.dispose();
          }
          final boundary = tester.renderObject<RenderRepaintBoundary>(
            find
                .ancestor(
                  of: find.byKey(const ValueKey('page-preview-paper-0')),
                  matching: find.byType(RepaintBoundary),
                )
                .first,
          );
          await tester.runAsync(() async {
            final image = await boundary.toImage(pixelRatio: 1.5);
            try {
              final bytes = (await image.toByteData(
                format: ui.ImageByteFormat.png,
              ))!;
              await File(
                '${folder.path}/$name.png',
              ).writeAsBytes(bytes.buffer.asUint8List());
            } finally {
              image.dispose();
            }
          });
          await tester.tap(find.byKey(const Key('page-preview-close')));
          await tester.pumpAndSettle();
          expect(jsonEncode(model.workspace), before);
          await drive(() async {
            expectSync(
              await File('${folder.path}/$name.folio').readAsBytes(),
              recovery,
            );
            final output = '${folder.path}/balance-$name.docx';
            await model.core.call({
              'op': 'exportOffice',
              'format': 'docx',
              'path': output,
            });
            expectSync(
              await File(output).readAsBytes(),
              await source.readAsBytes(),
            );
            final fresh = CoreBridge();
            await fresh.call({
              'op': 'load',
              'path': '${folder.path}/$name.folio',
            });
            final reloaded = await fresh.call({'op': 'documentPagePreview'});
            expectSync(jsonEncode(reloaded), jsonEncode(widget.snapshot));
          });
        } finally {
          await tester.pumpWidget(const SizedBox.shrink());
          model.dispose();
        }
      }
      expect(followingY[0], lessThan(followingY[1]));
      semantics.dispose();
      // ignore: avoid_print
      print('COLUMN_BALANCE_UI_OUTPUT=${folder.path};realFont=$realFont');
    },
  );
}
