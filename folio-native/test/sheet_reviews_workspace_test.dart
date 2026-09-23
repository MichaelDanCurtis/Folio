import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_reviews.dart';
import 'package:folio_native/src/sheet_clipboard.dart';

void main() {
  testWidgets(
    'sheet comments and notes: context menu, history, copy, navigation, stale state and Office recovery',
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
        folder = await Directory.systemTemp.createTemp(
          'folio-sheet-reviews-ui-',
        );
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        final initial = await core.call({'op': 'get'});
        final w = initial['workspace'];
        w['sheets'] = [
          {
            'id': 'review',
            'name': 'Forecast',
            'cells': {
              'A1': 'Revenue',
              'B1': '125000',
              'A2': 'Costs',
              'B2': '84000',
            },
          },
          {
            'id': 'other',
            'name': 'Sources',
            'cells': {'A1': 'Source workbook'},
          },
        ];
        await core.call({'op': 'replace', 'workspace': w});
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

      final renderKey = GlobalKey();
      await tester.pumpWidget(
        RepaintBoundary(
          key: renderKey,
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
      Future<void> settle() => drive(screen.flush);
      Future<void> tap(Key key) async {
        final f = find.byKey(key);
        await tester.ensureVisible(f);
        await tester.pumpAndSettle();
        await tester.tap(f);
        await settle();
      }

      Future<void> enter(Key key, String value) async {
        final f = find.byKey(key);
        await tester.ensureVisible(f);
        await tester.pumpAndSettle();
        await tester.enterText(f, value);
        await tester.pump();
      }

      Future<void> close() async {
        await tap(const Key('sheet-reviews-close'));
      }

      Map<String, dynamic>? clipboard;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SheetClipboard.channel,
        (call) async {
          if (call.method == 'write') {
            clipboard = Map<String, dynamic>.from(call.arguments as Map);
            return null;
          }
          if (call.method == 'read') return clipboard;
          throw MissingPluginException();
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SheetClipboard.channel,
          null,
        ),
      );
      Map getSheet(String id) => model.sheets.firstWhere((s) => s['id'] == id);
      Map? review(String cell, [String id = 'review']) =>
          cellReview(getSheet(id), cell);
      await drive(() => screen.switchMode(true));
      await drive(() async {
        await screen.sheetKey.currentState!.select(0, 0, extend: false);
      });
      await tester.tap(find.text('Review').first);
      await tester.pumpAndSettle();
      await tap(const Key('sheet-new-comment'));
      expect(find.byType(SheetReviewsDialog), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(find.byKey(const Key('sheet-review-post')))
            .onPressed,
        isNull,
      );
      await enter(const Key('sheet-review-author'), 'Morgan');
      await enter(
        const Key('sheet-review-draft'),
        'Check the forecast against the source workbook.',
      );
      await tap(const Key('sheet-review-post'));
      expect(review('A1')!['kind'], 'comment');
      expect(review('A1')!['messages'][0]['author'], 'Morgan');
      await enter(const Key('sheet-review-draft'), 'Confirmed the numbers.');
      await tap(const Key('sheet-review-post'));
      expect(review('A1')!['messages'].length, 2);
      await tap(const Key('sheet-review-edit-1'));
      await enter(
        const Key('sheet-review-draft'),
        'Confirmed the numbers and source.',
      );
      await tap(const Key('sheet-review-post'));
      expect(
        review('A1')!['messages'][1]['text'],
        'Confirmed the numbers and source.',
      );
      await tap(const Key('sheet-review-resolve'));
      expect(review('A1')!['resolved'], true);
      expect(find.byKey(const Key('sheet-review-draft')), findsNothing);
      await tap(const Key('sheet-reviews-undo'));
      expect(review('A1')!['resolved'], false);
      await tap(const Key('sheet-reviews-redo'));
      expect(review('A1')!['resolved'], true);
      await tap(const Key('sheet-review-resolve'));
      await tap(const Key('sheet-review-go-to'));
      expect(screen.selectedCell, 'A1');
      expect(find.byType(SheetReviewsDialog), findsNothing);
      expect(
        tester
            .widget<SheetReviewMarker>(
              find.byKey(const Key('review-marker-A1')),
            )
            .review!['kind'],
        'comment',
      );
      final grid = screen.sheetKey.currentState!;
      await drive(() async {
        await grid.select(1, 1, extend: false);
      });
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.f2);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await settle();
      expect(find.byType(SheetReviewsDialog), findsOneWidget);
      await enter(const Key('sheet-review-author'), 'Taylor');
      await enter(const Key('sheet-review-draft'), 'Costs include delivery.');
      await tap(const Key('sheet-review-post'));
      expect(review('B2')!['kind'], 'note');
      expect(find.byKey(const Key('sheet-review-draft')), findsNothing);
      expect(find.byKey(const Key('sheet-review-resolve')), findsNothing);
      await tap(const Key('sheet-review-edit-0'));
      await enter(
        const Key('sheet-review-draft'),
        'Costs include delivery and setup.',
      );
      await tap(const Key('sheet-review-post'));
      expect(
        review('B2')!['messages'][0]['text'],
        'Costs include delivery and setup.',
      );
      if (capture) {
        await tester.runAsync(() async {
          final image =
              await (renderKey.currentContext!.findRenderObject()
                      as RenderRepaintBoundary)
                  .toImage();
          final data = await image.toByteData(format: ui.ImageByteFormat.png);
          await File(
            '../../work/sheet-reviews-widget.png',
          ).writeAsBytes(data!.buffer.asUint8List());
          image.dispose();
        });
      }
      await close();
      await drive(grid.copy);
      await drive(() async {
        await grid.select(2, 2, extend: false);
      });
      final pasteSpecial = grid.pasteSpecial();
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('paste-special-mode')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Comments and notes only').last);
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, 'Paste'));
      await drive(() => pasteSpecial);
      expect(
        review('C3')!['messages'][0]['text'],
        'Costs include delivery and setup.',
      );
      expect(getSheet('review')['cells']['C3'], isNull);
      expect(review('C3')!['id'], isNot(review('B2')!['id']));
      // The actual right-click gesture opens review controls for this cell.
      final mouse = await tester.createGesture(
        kind: ui.PointerDeviceKind.mouse,
        buttons: 2,
      );
      await mouse.down(tester.getCenter(find.byKey(const Key('cell-C3'))));
      await mouse.up();
      await tester.pumpAndSettle();
      await tester.tap(find.text('Edit note'));
      await settle();
      expect(find.byType(SheetReviewsDialog), findsOneWidget);
      // Opening Edit note and closing it unchanged must not claim a draft was lost.
      await close();
      expect(find.text('Discard this draft?'), findsNothing);
      expect(find.byType(SheetReviewsDialog), findsNothing);
      await tap(const Key('sheet-new-note'));
      await enter(
        const Key('sheet-review-draft'),
        'Draft that must not disappear.',
      );
      await close();
      expect(find.text('Discard this draft?'), findsOneWidget);
      await tester.tap(find.text('Keep writing'));
      await tester.pumpAndSettle();
      expect(find.text('Draft that must not disappear.'), findsOneWidget);
      await enter(const Key('sheet-review-draft'), '');
      await tap(const Key('sheet-review-delete'));
      expect(review('C3'), isNull);
      await tap(const Key('sheet-reviews-undo'));
      expect(review('C3'), isNotNull);
      await close();
      await tap(const Key('sheet-reviews'));
      await tap(const Key('sheet-review-edit-0'));
      await enter(const Key('sheet-review-draft'), 'Stale note');
      await drive(
        () => model.mutate({'op': 'title', 'title': 'Concurrent change'}),
      );
      final revision = model.revision;
      await tap(const Key('sheet-review-post'));
      expect(find.textContaining('workbook changed'), findsOneWidget);
      expect(model.revision, revision);
      await enter(const Key('sheet-review-draft'), '');
      await close();
      // Navigate across the workbook from the review list, including a filtered view.
      await tester.tap(find.text('Sources').last);
      await settle();
      await drive(() async {
        await screen.sheetKey.currentState!.select(0, 0, extend: false);
      });
      await tap(const Key('sheet-new-comment'));
      await enter(const Key('sheet-review-author'), 'Alex');
      await enter(const Key('sheet-review-draft'), 'Independent source check.');
      await tap(const Key('sheet-review-post'));
      expect(review('A1', 'other')!['messages'][0]['author'], 'Alex');
      await tester.tap(find.byKey(const Key('sheet-review-sheet-filter')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('All sheets').last);
      await tester.pumpAndSettle();
      await tap(const Key('sheet-review-review-A1'));
      await tap(const Key('sheet-review-go-to'));
      expect(screen.sheetId, 'review');
      expect(screen.selectedCell, 'A1');
      await tap(const Key('sheet-reviews'));
      await tester.tap(find.byKey(const Key('sheet-review-kind-filter')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Notes').last);
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('sheet-review-resolve')), findsNothing);
      expect(find.byKey(const Key('sheet-review-draft')), findsNothing);
      await close();
      tester.view.physicalSize = const Size(960, 680);
      await tester.pumpAndSettle();
      await tap(const Key('sheet-reviews'));
      expect(tester.takeException(), isNull);
      await close();
      final file = '${folder.path}/reviews.xlsx';
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': file,
        });
        await model.flush();
      });
      await tester.runAsync(() async {
        final round = await model.core.call({
          'op': 'importOffice',
          'path': file,
        });
        final r = round['workspace']['sheets'][0]['cellReviews'];
        expect(r['A1']['messages'].length, 2);
        expect(r['B2']['kind'], 'note');
        expect(
          r['C3']['messages'][0]['text'],
          'Costs include delivery and setup.',
        );
        final recovered = await model.core.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expect(
          recovered['workspace']['sheets'][0]['cellReviews']['A1']['messages']
              .length,
          2,
        );
      });
      expect(tester.takeException(), isNull);
    },
  );
}
