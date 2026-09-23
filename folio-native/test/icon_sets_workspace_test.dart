import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_conditional_dialog.dart';
import 'package:folio_native/src/sheet_icon_set.dart';
import 'package:folio_native/src/sheet_icon_set_editor.dart';
import 'package:folio_native/src/sheet_font.dart';
import 'package:folio_native/src/sheet_autofit.dart';
import 'package:folio_native/src/sheet_column_metrics.dart';
import 'package:folio_native/src/table_controls.dart';

void main() {
  testWidgets(
    'all icon categories render distinctly; scaled/narrow display keeps values accessible',
    (tester) async {
      final font = File('/System/Library/Fonts/Supplemental/Arial.ttf');
      if (font.existsSync()) {
        await tester.runAsync(() async {
          final loader = FontLoader('IconAtlasLabels')
            ..addFont(font.readAsBytes().then((b) => ByteData.sublistView(b)));
          await loader.load();
        });
      }
      final atlas = ui.PictureRecorder();
      final canvas = Canvas(atlas);
      canvas.drawColor(Colors.white, BlendMode.src);
      var row = 0;
      for (final entry in sheetIconSets.entries) {
        final label = TextPainter(
          text: TextSpan(
            text: entry.value,
            style: const TextStyle(
              color: Colors.black,
              fontSize: 14,
              fontFamily: 'IconAtlasLabels',
            ),
          ),
          textDirection: TextDirection.ltr,
        )..layout();
        label.paint(canvas, Offset(8, row * 42 + 8));
        label.dispose();
        final hashes = <String>{};
        for (var i = 0; i < sheetIconCount(entry.key); i++) {
          final recorder = ui.PictureRecorder();
          final c = Canvas(recorder);
          c.drawColor(Colors.white, BlendMode.src);
          SheetIconPainter(entry.key, i).paint(c, const Size(30, 30));
          final picture = recorder.endRecording();
          final image = await tester.runAsync(() => picture.toImage(30, 30));
          final bytes = await tester.runAsync(
            () => image!.toByteData(format: ui.ImageByteFormat.rawRgba),
          );
          expect(
            bytes!.buffer.asUint8List().any((v) => v != 255),
            isTrue,
            reason: '${entry.key} $i must be visible',
          );
          expect(
            hashes.add(bytes.buffer.asUint8List().toString()),
            isTrue,
            reason: 'Every ${entry.key} category needs a distinct glyph',
          );
          canvas.save();
          canvas.translate(
            275 + (sheetIconCount(entry.key) - 1 - i) * 37,
            row * 42.0,
          );
          SheetIconPainter(entry.key, i).paint(canvas, const Size(30, 30));
          canvas.restore();
          image!.dispose();
          picture.dispose();
        }
        row++;
      }
      final picture = atlas.endRecording();
      final image = await tester.runAsync(() => picture.toImage(500, 840));
      final png = await tester.runAsync(
        () => image!.toByteData(format: ui.ImageByteFormat.png),
      );
      await tester.runAsync(() async {
        final d = Directory(
          '${Directory.systemTemp.path}/folio-icons-ui-tests',
        );
        await d.create(recursive: true);
        await File(
          '${d.path}/icon-catalog.png',
        ).writeAsBytes(png!.buffer.asUint8List());
      });
      image!.dispose();
      picture.dispose();
      final semantics = tester.ensureSemantics();

      for (final width in [2.0, 12.0, 60.0]) {
        final style = {
          'fontSize': 36,
          '_iconSet': {'set': '3Stars', 'index': 1, 'showValue': false},
        };
        await tester.pumpWidget(
          MaterialApp(
            home: Center(
              child: SizedBox(
                width: width,
                height: 25,
                child: SheetIconValue(
                  address: 'A1',
                  style: style,
                  child: SheetFontText('123', font: style),
                ),
              ),
            ),
          ),
        );
        expect(tester.takeException(), isNull);
        expect(find.bySemanticsLabel('3 stars, icon 2 of 3'), findsOneWidget);
        expect(
          tester.widget<Opacity>(find.byType(Opacity)).alwaysIncludeSemantics,
          isTrue,
        );
      }
      semantics.dispose();
    },
  );
  testWidgets(
    'imported custom category count and NoIcons base can be reversed and edited',
    (tester) async {
      var value = <String, dynamic>{
        'set': 'NoIcons',
        'reverse': true,
        'showValue': true,
        'thresholds': [
          for (var i = 0; i < 4; i++)
            {'type': 'num', 'value': '${i * 20}', 'gte': true},
        ],
        'icons': [
          {'set': 'NoIcons', 'index': 0},
          {'set': '4Rating', 'index': 1},
          {'set': '3Triangles', 'index': 2},
          {'set': '5Boxes', 'index': 4},
        ],
      };
      tester.view.physicalSize = const Size(1100, 1100);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Center(
              child: SizedBox(
                width: 620,
                child: StatefulBuilder(
                  builder: (context, setState) => IconSetEditor(
                    value: value,
                    onChanged: (next) => setState(() => value = next),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.tap(find.byKey(const Key('reverse-icons')));
      await tester.pumpAndSettle();
      expect((value['icons'] as List).first, {'set': '5Boxes', 'index': 4});
      expect((value['thresholds'] as List).last['value'], '60');
      await tester.enterText(
        find.byKey(const Key('icon-value-3-NoIcons')),
        '80',
      );
      await tester.pumpAndSettle();
      expect((value['thresholds'] as List).last['value'], '80');
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets(
    'icon editor, custom categories, history, AutoFit and Excel recovery use the real engine',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-icons-ui-tests',
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
              'A1': '0',
              'A2': '50',
              'A3': '100',
              'A4': 'text',
              'A5': '=1/0',
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
        expect(check(), isTrue, reason: 'revision=${model.revision}');
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
      final revision = model.revision;
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
        find.text('Icon sets').last,
        240,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.tap(find.text('Icon sets').last);
      await tester.pumpAndSettle();
      expect(find.byType(IconSetEditor), findsOneWidget);
      await tester.ensureVisible(find.byKey(const Key('custom-icon-0')));
      await tester.tap(find.byKey(const Key('custom-icon-0')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('No cell icon'));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.byKey(const Key('custom-icon-1')));
      await tester.tap(find.byKey(const Key('custom-icon-1')));
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        find.byKey(const Key('choose-icon-5Quarters-2')),
        220,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.tap(find.byKey(const Key('choose-icon-5Quarters-2')));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.byKey(const Key('icon-only')));
      await tester.tap(find.byKey(const Key('icon-only')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('conditional-rule-done')));
      await tester.pumpAndSettle();
      expect(model.revision, revision);
      await tester.tap(find.byKey(const Key('conditional-apply')));
      await until(() => model.revision == revision + 1);
      SheetIconGlyph icon(String cell) => tester.widget<SheetIconGlyph>(
        find.byKey(Key('conditional-icon-$cell')),
      );
      expect(find.byKey(const Key('conditional-icon-A1')), findsNothing);
      expect(icon('A2').set, '5Quarters');
      expect(icon('A2').index, 2);
      expect(icon('A3').set, '3TrafficLights1');
      expect(icon('A3').index, 2);
      expect(find.byKey(const Key('conditional-icon-A4')), findsNothing);
      expect(find.byKey(const Key('conditional-icon-A5')), findsNothing);
      expect(grid.formatted('A2'), '50');
      expect(model.sheets[0]['cellStyles']?['A2'], isNull);
      final opacity = find.descendant(
        of: find.byKey(const Key('data-bar-A2')),
        matching: find.byType(Opacity),
      );
      expect(tester.widget<Opacity>(opacity).opacity, 0);
      await drive(() => screen.history(false));
      expect(find.byKey(const Key('conditional-icon-A2')), findsNothing);
      await drive(() => screen.history(true));
      expect(icon('A2').set, '5Quarters');
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': id, 'address': 'A3', 'value': '200'},
          ],
        }),
      );
      expect(
        find.byKey(const Key('conditional-icon-A2')),
        findsNothing,
      ); // 50 lies below 33% now.
      // Undo restores the original range population and icon assignment.
      await drive(() => screen.history(false));
      expect(icon('A2').set, '5Quarters');
      final normal = normalSheetFont(model.sheets[0]);
      final metrics = SheetColumnMetrics.fromFont(normal);
      final columns = SheetAxisLayout(VisibleColumns(model.sheets[0]), 122, {});
      Future<double> fit(Map overlay) async {
        final fitted = await measureSheetColumns(
          sheet: {
            ...model.sheets[0],
            'cells': {'A2': '50'},
          },
          calculated: {'A2': 50},
          formatted: {
            'A2': {'conditionalStyle': overlay},
          },
          first: 0,
          count: 1,
          columns: columns,
          metrics: metrics,
          scaler: TextScaler.noScaling,
          direction: TextDirection.ltr,
          checkpoint: () async {},
        );
        return fitted['1']!;
      }

      final plain = await fit({});
      final withIcon = await fit({
        '_iconSet': {'set': '3Stars', 'index': 2, 'showValue': true},
      });
      final iconOnly = await fit({
        '_iconSet': {'set': '3Stars', 'index': 2, 'showValue': false},
      });
      expect(withIcon, greaterThan(plain));
      expect(iconOnly, lessThan(withIcon));
      await drive(
        () => model.core.call({
          'op': 'save',
          'path': '${folder.path}/icons.folio',
        }),
      );
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/icons.xlsx',
        }),
      );
      final before = model.snapshot['formatted'];
      late Map<String, dynamic> recovered;
      await drive(() async {
        final core = CoreBridge();
        recovered = await core.call({
          'op': 'load',
          'path': '${folder.path}/icons.folio',
        });
      });
      expect(recovered['formatted'], before);
      await drive(() => screen.flush());
    },
  );
}
