import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/services.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_paragraph_dialog.dart';
import 'package:folio_native/src/document_table_view.dart';

void main() {
  testWidgets(
    'paragraph draft, input validation, Apply/Undo/Redo, stale protection, table drafts and recovery use the real core',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1100);
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
      late Directory dir;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-paragraph-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path':
              '${Directory.current.path}/test/fixtures/word-paragraphs.docx',
        });
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
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

      Future<void> settle() => drive(model.flush);
      await tester.pumpWidget(
        RepaintBoundary(
          key: const Key('paragraph-preview-frame'),
          child: FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
        ),
      );
      await tester.pumpAndSettle();
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
      });
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      final original = jsonEncode(model.workspace['document']);
      final revision = model.revision;
      final originalText = screen.editor.document.toPlainText();
      Future<void> open() async {
        final table = find.byKey(const Key('table-paragraph'));
        await tester.tap(
          table.evaluate().isNotEmpty
              ? table
              : find.byKey(const Key('document-paragraph')),
        );
        await settle();
        expect(find.byType(DocumentParagraphDialog), findsOneWidget);
      }

      Future<void> draft() async {
        await tester.enterText(find.byKey(const Key('paragraph-left')), '54');
        await tester.enterText(find.byKey(const Key('paragraph-after')), '15');
        await tester.tap(
          find.byWidgetPredicate(
            (w) =>
                w is DropdownButtonFormField<String> &&
                (w.key as ValueKey).value.toString().startsWith(
                  'paragraph-line-mode-',
                ),
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(find.text('Exactly').last);
        await tester.pumpAndSettle();
        await tester.enterText(find.byKey(const Key('paragraph-line')), '24');
        await tester.pumpAndSettle();
      }

      await open();
      expect(
        tester
            .widget<TextField>(find.byKey(const Key('paragraph-left')))
            .controller!
            .text,
        '36',
      );
      await tester.enterText(find.byKey(const Key('paragraph-left')), '-1');
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<FilledButton>(find.byKey(const Key('apply-paragraph')))
            .onPressed,
        isNull,
      );
      await draft();
      expect(jsonEncode(model.workspace['document']), original);
      if (capture) {
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('paragraph-preview-frame')),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          await File(
            '${Directory.current.path}/release/previews/paragraph-format.png',
          ).writeAsBytes(bytes!.buffer.asUint8List());
        });
      }
      await tester.tap(find.text('Cancel'));
      await settle();
      expect(model.revision, revision);
      await open();
      await draft();
      await tester.tap(find.byKey(const Key('apply-paragraph')));
      await settle();
      expect(model.revision, revision + 1);
      expect(screen.editor.document.toPlainText(), originalText);
      expect(screen.editor.selection, const TextSelection.collapsed(offset: 0));
      Map direct() =>
          model.workspace['document']['content'][0]['attrs']['paragraphFormat']['direct']
              as Map;
      expect(direct(), {
        'keepNext': true,
        'left': 1080,
        'after': 300,
        'afterAutospacing': false,
        'line': 480,
        'lineRule': 'exact',
      });
      final applied = jsonEncode(model.workspace['document']);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), original);
      await drive(() => screen.history(true));
      expect(jsonEncode(model.workspace['document']), applied);
      await open();
      await tester.enterText(find.byKey(const Key('paragraph-left')), '72');
      await drive(
        () => model.mutate({
          'op': 'title',
          'title': 'Changed while paragraph dialog is open',
        }),
      );
      final beforeStale = model.revision;
      await tester.tap(find.byKey(const Key('apply-paragraph')));
      await settle();
      expect(model.revision, beforeStale);
      expect(jsonEncode(model.workspace['document']), applied);
      expect(
        find.text('The document changed. Reopen Paragraph and try again.'),
        findsOneWidget,
      );
      // Table paragraph Apply remains a draft until the outer table applies.
      await tester.ensureVisible(
        find.byKey(const ValueKey('table-cell-0-0')).first,
      );
      await tester.tap(find.byKey(const ValueKey('table-cell-0-0')).first);
      await tester.pumpAndSettle();
      expect(find.byType(DocumentTableDialog), findsOneWidget);
      await open();
      await tester.enterText(find.byKey(const Key('paragraph-before')), '18');
      await tester.tap(find.byKey(const Key('apply-paragraph')));
      await settle();
      expect(jsonEncode(model.workspace['document']), applied);
      await tester.tap(find.text('Cancel'));
      await settle();
      expect(jsonEncode(model.workspace['document']), applied);
      await tester.tap(find.byKey(const ValueKey('table-cell-0-0')).first);
      await tester.pumpAndSettle();
      await open();
      await tester.enterText(find.byKey(const Key('paragraph-before')), '18');
      await tester.tap(find.byKey(const Key('apply-paragraph')));
      await settle();
      await tester.tap(find.byKey(const Key('table-apply')));
      await settle();
      expect(model.revision, beforeStale + 1);
      final cell = model
          .workspace['document']['content'][3]['content'][0]['content'][0]['content'][0];
      expect(cell['attrs']['paragraphFormat']['direct']['before'], 360);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
      for (var i = 0; i < 200 && model.revision == beforeStale + 1; i++) {
        await tester.pump(const Duration(milliseconds: 20));
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 10)),
        );
      }
      expect(model.revision, beforeStale + 2);
      await settle();
      expect(jsonEncode(model.workspace['document']), applied);
      await drive(() => screen.history(true));
      final finalDoc = jsonEncode(model.workspace['document']);
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${dir.path}/paragraphs.docx',
        });
        await model.flush();
        final out = Directory(
          '${Directory.systemTemp.path}/folio-paragraph-ui-tests',
        );
        await out.create(recursive: true);
        await File(
          '${dir.path}/paragraphs.docx',
        ).copy('${out.path}/ui-edited.docx');
        await File(model.recoveryPath).copy('${out.path}/workspace.folio');
        await model.mutate({
          'op': 'load',
          'path': model.recoveryPath,
        }, refreshDocument: true);
      });
      expect(jsonEncode(model.workspace['document']), finalDoc);
      await drive(
        () => model.mutate({
          'op': 'importOffice',
          'path': '${dir.path}/paragraphs.docx',
        }, refreshDocument: true),
      );
      expect(direct(), {
        'keepNext': true,
        'left': 1080,
        'after': 300,
        'afterAutospacing': false,
        'line': 480,
        'lineRule': 'exact',
      });
      expect(screen.editor.document.toPlainText(), originalText);
      await settle();
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
