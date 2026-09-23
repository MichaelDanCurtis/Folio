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
import 'package:folio_native/src/document_tabs.dart';
import 'package:folio_native/src/document_tabs_dialog.dart';

void main() {
  testWidgets(
    'tab dialog previews, cancels, applies atomically, rejects stale state, preserves typing settings and exports',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final capture = Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1';
      if (capture) {
        await tester.runAsync(() async {
          for (final (family, path) in [
            ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
            ('Aptos', '/System/Library/Fonts/Supplemental/Arial.ttf'),
            ('Georgia', '/System/Library/Fonts/Supplemental/Georgia.ttf'),
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
        folder = await Directory.systemTemp.createTemp('folio-tab-workspace-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/word-tabs.docx',
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

      Future<void> settle() => drive(model.flush);
      await tester.pumpWidget(
        RepaintBoundary(
          key: const Key('tabs-preview-frame'),
          child: FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
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
      final originalDelta = screen.editor.document.toDelta();
      final originalText = screen.editor.document.toPlainText();
      final revision = model.revision;
      Future<void> capturePreview(String name) async {
        if (!capture) return;
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('tabs-preview-frame')),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          final file = File(
            '${Directory.current.path}/release/previews/$name.png',
          );
          await file.parent.create(recursive: true);
          await file.writeAsBytes(bytes!.buffer.asUint8List());
        });
      }

      Future<void> open() async {
        await tester.tap(find.byKey(const Key('document-tab-stops')));
        await settle();
        expect(find.byType(DocumentTabsDialog), findsOneWidget);
      }

      Future<void> draft() async {
        await tester.tap(find.text('Clear all stops'));
        await tester.enterText(find.byKey(const Key('tab-position')), '144');
        await tester.tap(
          find.byWidgetPredicate(
            (w) =>
                w is DropdownButtonFormField<String> &&
                (w.key as ValueKey).value.toString().startsWith(
                  'tab-alignment-',
                ),
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(find.text('Right').last);
        await tester.pumpAndSettle();
        await tester.tap(
          find.byWidgetPredicate(
            (w) =>
                w is DropdownButtonFormField<String> &&
                (w.key as ValueKey).value.toString().startsWith('tab-leader-'),
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(find.text('Dots').last);
        await tester.pumpAndSettle();
        await tester.tap(find.text('Add stop'));
        await tester.enterText(
          find.byKey(const Key('default-tab-spacing')),
          '48',
        );
        await tester.pumpAndSettle();
      }

      await open();
      expect(
        tester
            .widget<TextField>(find.byKey(const Key('default-tab-spacing')))
            .controller!
            .text,
        '30',
      );
      await draft();
      await capturePreview('tab-stops');
      final preview = tester
          .renderObjectList<RenderParagraph>(
            find.descendant(
              of: find.byKey(const Key('tab-stops-preview')),
              matching: find.byWidgetPredicate((w) => w is RichText),
            ),
          )
          .firstWhere((r) => r.text.toPlainText() == 'Project\t12.50\tReady');
      expect(preview.text.toPlainText(), 'Project\t12.50\tReady');
      expect(jsonEncode(model.workspace['document']), original);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(model.revision, revision);
      await open();
      await draft();
      await tester.tap(find.byKey(const Key('apply-tab-stops')));
      await settle();
      expect(find.byType(DocumentTabsDialog), findsNothing);
      expect(model.revision, revision + 1);
      expect(model.workspace['document']['attrs']['defaultTabStop'], 960);
      final first = model.workspace['document']['content'][0];
      expect(
        effectiveTabStops(
          first['attrs']['tabs'],
        ).map((s) => s.toJson()).toList(),
        [
          const DocumentTabStop(
            2880,
            alignment: 'right',
            leader: 'dot',
          ).toJson(),
        ],
      );
      expect(first['content'][0]['marks'], [
        {'type': 'bold'},
      ]);
      expect(screen.editor.selection, const TextSelection.collapsed(offset: 0));
      final paragraphs = tester
          .renderObjectList<RenderParagraph>(
            find.byWidgetPredicate((w) => w is RichText),
          )
          .where((r) => r.text.toPlainText() == 'Project\t12.50\tReady');
      expect(paragraphs, isNotEmpty);
      final actual = paragraphs.first;
      expect(
        actual.getOffsetForCaret(const TextPosition(offset: 13), Rect.zero).dx,
        closeTo(192, .1),
      );
      await capturePreview('tab-aligned-document');
      final applied = jsonEncode(model.workspace['document']);
      final tabs = jsonEncode(first['attrs']['tabs']);
      screen.editor.replaceText(
        0,
        0,
        'X',
        const TextSelection.collapsed(offset: 1),
      );
      await settle();
      expect(model.workspace['document']['attrs']['defaultTabStop'], 960);
      expect(
        jsonEncode(model.workspace['document']['content'][0]['attrs']['tabs']),
        tabs,
      );
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      expect(jsonEncode(model.workspace['document']), applied);
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      expect(jsonEncode(model.workspace['document']), original);
      expect(screen.editor.document.toDelta(), originalDelta);
      await drive(() => model.mutate({'op': 'redo'}, refreshDocument: true));
      expect(jsonEncode(model.workspace['document']), applied);
      // Edit only the leader, preserving position, alignment and text.
      await open();
      await tester.tap(find.byKey(const ValueKey('tab-stop-2880')));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('tab-leader-dot')), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('tab-leader-dot')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Dashes').last);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Update stop'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('apply-tab-stops')));
      await settle();
      expect(
        effectiveTabStops(
          model.workspace['document']['content'][0]['attrs']['tabs'],
        ).single.leader,
        'hyphen',
      );
      expect(
        screen.editor.document.toPlainText(),
        originalText,
        reason: 'Leader-only editing must retain source characters',
      );
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      expect(jsonEncode(model.workspace['document']), applied);
      // A new revision while the dialog is open invalidates its snapshot.
      await open();
      await draft();
      await drive(
        () => model.mutate({
          'op': 'title',
          'title': 'Changed while tabs are open',
        }),
      );
      final beforeStale = model.revision;
      await tester.tap(find.byKey(const Key('apply-tab-stops')));
      await settle();
      expect(model.revision, beforeStale);
      expect(
        find.text('The document changed. Reopen Tab stops and try again.'),
        findsOneWidget,
      );
      expect(jsonEncode(model.workspace['document']), applied);
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/tabs.docx',
        });
        await model.flush();
        await model.mutate({
          'op': 'load',
          'path': model.recoveryPath,
        }, refreshDocument: true);
      });
      expect(jsonEncode(model.workspace['document']), applied);
      await drive(
        () => model.mutate({
          'op': 'importOffice',
          'path': '${folder.path}/tabs.docx',
        }, refreshDocument: true),
      );
      expect(model.workspace['document']['attrs']['defaultTabStop'], 960);
      expect(
        jsonEncode(model.workspace['document']['content'][0]['attrs']['tabs']),
        tabs,
      );
      await settle();
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
