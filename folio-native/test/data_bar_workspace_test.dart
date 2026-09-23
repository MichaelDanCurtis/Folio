import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_conditional_dialog.dart';
import 'package:folio_native/src/sheet_data_bar.dart';

void main() {
  testWidgets(
    'data-bar pixels respect axis, direction, gradient, borders and cell bounds',
    (tester) async {
      Future<List<int>> pixel(Map<String, dynamic> bar, int x, int y) async {
        final recorder = ui.PictureRecorder();
        final canvas = Canvas(recorder);
        canvas.drawColor(Colors.white, BlendMode.src);
        DataBarPainter(bar).paint(canvas, const Size(100, 30));
        final picture = recorder.endRecording();
        final image = await tester.runAsync(() => picture.toImage(100, 30));
        final bytes = await tester.runAsync(
          () => image!.toByteData(format: ui.ImageByteFormat.rawRgba),
        );
        final data = bytes!.buffer.asUint8List();
        final i = (y * 100 + x) * 4;
        final result = data.sublist(i, i + 4);
        image!.dispose();
        picture.dispose();
        return result;
      }

      final bar = {
        'start': .5,
        'end': .1,
        'axis': .5,
        'color': '#FF0000',
        'border': true,
        'borderColor': '#0000FF',
        'axisColor': '#000000',
        'gradient': false,
      };
      expect(await pixel(bar, 25, 15), [255, 0, 0, 255]);
      expect(await pixel(bar, 70, 15), [255, 255, 255, 255]);
      expect(await pixel(bar, 25, 0), [255, 255, 255, 255]);
      expect((await pixel(bar, 50, 15))[0], lessThan(255));
      final gradient = {...bar, 'gradient': true, 'border': false};
      expect(
        (await pixel(gradient, 15, 15))[1],
        greaterThan((await pixel(gradient, 45, 15))[1]),
      );
      expect(await pixel({...bar, 'end': .5, 'axis': null}, 25, 15), [
        255,
        255,
        255,
        255,
      ]);
    },
  );

  testWidgets(
    'data-bar editor, live cells, bar-only display, history, Excel export and last row',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-data-bars-ui-tests',
      );
      late WorkspaceModel model;
      late String id;
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        await CoreBridge.initialize();
        final core = CoreBridge();
        final response = await core.call({'op': 'new', 'blank': true});
        id = response['workspace']['sheets'][0]['id'] as String;
        await core.call({
          'op': 'cells',
          'cells': [
            for (final e in {
              'A1': '-20',
              'A2': '0',
              'A3': '40',
              'A4': '80',
              'B1': '=A3*2',
            }.entries)
              {'sheetId': id, 'address': e.key, 'value': e.value},
          ],
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
        expect(
          check(),
          isTrue,
          reason:
              'revision=${model.revision}, cells=${model.sheets[0]['cells']}',
        );
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
      }

      Future<void> drive(Future<dynamic> Function() action) async {
        var done = false;
        Object? error;
        unawaited(
          Future.sync(action).then(
            (_) {
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
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
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
      await tester.tap(find.byKey(const Key('conditional-format-button')));
      await until(
        () => find.byType(SheetConditionalDialog).evaluate().isNotEmpty,
      );
      await tester.tap(find.byKey(const Key('conditional-new')));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('conditional-range')),
        'A1:A1048576',
      );
      await tester.tap(find.byType(DropdownButtonFormField<String>).first);
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        find.text('Data bars').last,
        240,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.tap(find.text('Data bars').last);
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.byTooltip('Positive fill #63BE7B'));
      await tester.tap(find.byTooltip('Positive fill #63BE7B'));
      await tester.pumpAndSettle();
      final colorField = find.descendant(
        of: find.byKey(const Key('bar-color')),
        matching: find.byType(TextField),
      );
      expect(tester.widget<TextField>(colorField).controller!.text, '#63BE7B');
      await tester.ensureVisible(find.byKey(const Key('bar-showValue')));
      await tester.tap(find.byKey(const Key('bar-showValue')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('conditional-rule-done')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('conditional-apply')));
      await until(
        () =>
            ((model.sheets[0]['conditionalFormats'] as List?) ?? []).isNotEmpty,
      );
      SheetDataBar bar(String cell) =>
          tester.widget<SheetDataBar>(find.byKey(Key('data-bar-$cell')));
      expect(bar('A1').bar!['color'], '#FF0000');
      expect(bar('A3').bar!['color'], '#63BE7B');
      expect(bar('A3').bar!['axis'], closeTo(.2, 1e-10));
      expect(bar('A3').bar!['end'], closeTo(.6, 1e-10));
      final hidden = find.descendant(
        of: find.byKey(const Key('data-bar-A3')),
        matching: find.byType(Opacity),
      );
      expect(tester.widget<Opacity>(hidden).opacity, 0);
      expect(tester.widget<Opacity>(hidden).alwaysIncludeSemantics, true);
      expect(grid.formatted('A3'), '40');
      // A continuous entry burst reaches the real worker before it can finish
      // any save. Each cell keeps its own history entry and correct destination.
      await drive(() => grid.select(2, 0));
      final revision = model.revision;
      await tester.sendKeyEvent(LogicalKeyboardKey.digit6, character: '6');
      await tester.sendKeyEvent(LogicalKeyboardKey.digit0, character: '0');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.sendKeyEvent(LogicalKeyboardKey.digit7, character: '7');
      await tester.sendKeyEvent(LogicalKeyboardKey.digit0, character: '0');
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.sendKeyEvent(LogicalKeyboardKey.digit8, character: '8');
      await tester.sendKeyEvent(LogicalKeyboardKey.digit0, character: '0');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await until(
        () => model.revision == revision + 3 && grid.selected == 'C4',
      );
      expect(model.sheets[0]['cells']['A3'], '60');
      expect(model.sheets[0]['cells']['B3'], '70');
      expect(model.sheets[0]['cells']['C3'], '80');
      expect(model.snapshot['calculated'][id]['B1'], 120);
      expect(bar('A3').bar!['end'], closeTo(.8, 1e-10));
      for (var undo = 1; undo <= 3; undo++) {
        await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
        await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
        await until(() => model.revision == revision + 3 + undo);
      }
      expect(model.sheets[0]['cells']['A3'], '40');
      expect(model.sheets[0]['cells']['B3'] ?? '', '');
      expect(model.sheets[0]['cells']['C3'] ?? '', '');
      expect(model.snapshot['calculated'][id]['B1'], 80);
      expect(bar('A3').bar!['end'], closeTo(.6, 1e-10));
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': id, 'address': 'A4', 'value': '160'},
          ],
        }),
      );
      expect(bar('A3').bar!['axis'], closeTo(1 / 9, 1e-10));
      await drive(() => screen.history(false));
      expect(bar('A3').bar!['axis'], closeTo(.2, 1e-10));
      await drive(() => grid.select(1048575, 0));
      await tester.pumpAndSettle();
      final label = find.descendant(
        of: find.byKey(const Key('row-header-1048575')),
        matching: find.text('1048576'),
      );
      expect(label, findsOneWidget);
      final paragraph = tester.renderObject<RenderParagraph>(
        find.descendant(of: label, matching: find.byType(RichText)),
      );
      expect(
        paragraph.getBoxesForSelection(
          const TextSelection(baseOffset: 0, extentOffset: 7),
        ),
        hasLength(1),
      );
      expect(bar('A1048576').bar, isNull);
      expect(tester.takeException(), isNull);
      await drive(() => grid.select(0, 0));
      await drive(
        () => model.core.call({
          'op': 'save',
          'path': '${folder.path}/data-bars.folio',
        }),
      );
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/data-bars.xlsx',
        }),
      );
      await drive(() => screen.flush());
    },
  );
}
