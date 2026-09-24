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
import 'package:folio_native/src/sheet_font.dart';
import 'package:folio_native/src/sheet_clipboard.dart';
import 'package:folio_native/src/search.dart';

void main() {
  testWidgets(
    'number formats share grid, preview, copy, search, layout and persisted history',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-number-format-ui-tests',
      );
      late WorkspaceModel model;
      Map? clipboard;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SheetClipboard.channel,
        (call) async {
          if (call.method == 'write') {
            clipboard = call.arguments as Map;
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
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        for (final (family, path) in [
          ('Georgia', '/System/Library/Fonts/Supplemental/Georgia.ttf'),
          ('Aptos', '/System/Library/Fonts/SFNS.ttf'),
          ('Calibri', '/System/Library/Fonts/SFNS.ttf'),
          ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
          (
            'MaterialIcons',
            '${Platform.environment['FLUTTER_ROOT'] ?? '/opt/homebrew/share/flutter'}/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
          ),
        ]) {
          await (FontLoader(family)..addFont(
                Future.value(
                  ByteData.sublistView(await File(path).readAsBytes()),
                ),
              ))
              .load();
        }
        await CoreBridge.initialize();
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/number-formats.xlsx',
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        model.snapshot = await core.call({'op': 'get'});
      });
      Future<void> until(bool Function() check) async {
        for (var i = 0; i < 500 && !check(); i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(check(), isTrue);
        await tester.pump();
        expect(tester.takeException(), isNull);
      }

      Future<T> drive<T>(Future<T> Function() work) async {
        var done = false;
        T? value;
        Object? error;
        unawaited(
          Future.sync(work).then(
            (v) {
              value = v;
              done = true;
            },
            onError: (Object e) {
              error = e;
              done = true;
            },
          ),
        );
        await until(() => done);
        if (error != null) throw error!;
        return value as T;
      }

      await tester.pumpWidget(
        RepaintBoundary(
          key: const Key('number-format-screen'),
          child: FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
        ),
      );
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox());
        model.dispose();
      });
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.switchMode(true));
      final grid = screen.sheetKey.currentState!;
      Map getSheet() => model.sheets.first;
      String shown(String a) =>
          model.snapshot['formatted']['office-1'][a]['text'];
      Future<void> select(String range) async {
        await tester.enterText(find.byKey(ValueKey(grid.selectedRange)), range);
        await tester.testTextInput.receiveAction(TextInputAction.done);
        await drive(grid.flush);
      }

      Future<void> dialog() async {
        await tester.tap(find.text('More formats…'));
        await until(
          () =>
              find.byKey(const Key('number-format-code')).evaluate().isNotEmpty,
        );
        await until(
          () => find
              .byKey(const Key('number-format-preview'))
              .evaluate()
              .isNotEmpty,
        );
      }

      Future<void> code(String text) async {
        await tester.enterText(
          find.byKey(const Key('number-format-code')),
          text,
        );
        await tester.pump(const Duration(milliseconds: 220));
        await until(
          () => find
              .byKey(const Key('number-format-preview'))
              .evaluate()
              .isNotEmpty,
        );
      }

      expect(shown('B2'), '6.0000');
      expect(
        tester
            .widget<SheetFontText>(find.byKey(const Key('sheet-font-B2')))
            .text,
        '6.0000',
      );
      expect(
        tester
            .widget<SheetFontText>(find.byKey(const Key('sheet-font-B5')))
            .runs
            .first
            .font['color'],
        '#FF0000',
      );
      await select('B2');
      final originalValues = jsonEncode(model.snapshot['calculated']);
      final originalSheets = jsonEncode(model.sheets);
      final generation = model.snapshot['evaluatedCells'];
      var revision = model.revision;
      await dialog();
      await code('0.00000000');
      expect(
        tester
            .widget<SheetFontText>(
              find.byKey(const Key('number-format-preview')),
            )
            .text,
        '6.00000000',
      );
      await tester.tap(find.widgetWithText(FilledButton, 'Apply'));
      await until(() => model.revision == revision + 1);
      expect(getSheet()['cellStyles']['B2']['numFmt'], '0.00000000');
      expect(shown('B2'), '6.00000000');
      expect(jsonEncode(model.snapshot['calculated']), originalValues);
      expect(model.snapshot['evaluatedCells'], generation);
      final formattedSheets = jsonEncode(model.sheets);
      await drive(grid.copy);
      expect(clipboard!['text'], '6.00000000');
      expect(
        findInWorkbook(
          model.sheets,
          model.snapshot['calculated'],
          'office-1',
          '6.00000000',
          const SearchOptions(values: true, entireCell: true),
          formatted: model.snapshot['formatted'],
        ).hits.single.address,
        'B2',
      );
      await drive(() => screen.history(false));
      expect(jsonEncode(model.sheets), originalSheets);
      await drive(() => screen.history(true));
      expect(jsonEncode(model.sheets), formattedSheets);
      revision = model.revision;
      await dialog();
      await code('"unfinished');
      expect(
        tester
            .widget<FilledButton>(find.widgetWithText(FilledButton, 'Apply'))
            .onPressed,
        isNull,
      );
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(model.revision, revision);
      // A modal preview does not give permission to format a newer workbook.
      await dialog();
      await code('0.00');
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 'office-1', 'address': 'A30', 'value': 'newer edit'},
          ],
        }),
      );
      await tester.tap(find.widgetWithText(FilledButton, 'Apply'));
      await tester.pumpAndSettle();
      expect(model.revision, revision + 1);
      expect(getSheet()['cellStyles']['B2']['numFmt'], '0.00000000');
      expect(
        find.textContaining('The workbook changed. Open Number format'),
        findsOneWidget,
      );
      ScaffoldMessenger.of(
        tester.element(find.byType(WorkspaceScreen)),
      ).hideCurrentSnackBar();
      await tester.pumpAndSettle();
      await select('B2:B3');
      await dialog();
      expect(
        find.text('The selection contains different number formats.'),
        findsOneWidget,
      );
      await tester.tap(find.widgetWithText(ChoiceChip, 'Accounting'));
      await tester.pump(const Duration(milliseconds: 220));
      await until(
        () => find
            .byKey(const Key('number-format-preview'))
            .evaluate()
            .isNotEmpty,
      );
      final preview = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('number-format-preview')),
      );
      expect(preview.textBounds.width, greaterThan(450));
      expect(preview.textBounds.width, lessThanOrEqualTo(preview.size.width));
      await tester.pumpAndSettle();
      if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
        await tester.runAsync(() async {
          final boundary = tester.renderObject<RenderRepaintBoundary>(
            find.byKey(const Key('number-format-screen')),
          );
          final image = await boundary.toImage(pixelRatio: 1);
          final data = await image.toByteData(format: ui.ImageByteFormat.png);
          await File(
            '${folder.path}/number-format-dialog.png',
          ).writeAsBytes(data!.buffer.asUint8List());
          image.dispose();
        });
      }
      revision = model.revision;
      await tester.tap(find.widgetWithText(FilledButton, 'Apply'));
      await until(() => model.revision == revision + 1);
      expect(
        getSheet()['cellStyles']['B3']['numFmt'],
        getSheet()['cellStyles']['B2']['numFmt'],
      );
      final b3 = tester.renderObject<SheetFontRenderBox>(
        find.byKey(const Key('sheet-font-B3')),
      );
      expect(b3.textBounds.width, greaterThan(b3.size.width - 16));
      await tester.runAsync(() async {
        await model.flush();
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/formatted.xlsx',
        });
        final before = jsonEncode(
          (await model.core.call({'op': 'get'}))['formatted'],
        );
        await model.core.call({'op': 'new', 'blank': true});
        final reopened = await model.core.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
        expect(jsonEncode(reopened['formatted']), before);
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/reopened.xlsx',
        });
        expect(
          await File('${folder.path}/reopened.xlsx').readAsBytes(),
          await File('${folder.path}/formatted.xlsx').readAsBytes(),
        );
      });
    },
  );
}
