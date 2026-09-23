import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_clipboard.dart';
import 'package:folio_native/src/sheet_font.dart';

void main() {
  testWidgets(
    'narrow numeric displays preserve formula, copy, sizing history and exported values',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-number-width-ui-tests',
      );
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
      late WorkspaceModel model;
      late String id;
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        await CoreBridge.initialize();
        final core = CoreBridge();
        final start = await core.call({'op': 'new', 'blank': true});
        id = start['workspace']['sheets'][0]['id'];
        await core.call({
          'op': 'cells',
          'cells': [
            for (final entry in {
              'A1': '12345.6789',
              'B1': 'Fixed decimal',
              'A2': '=A1/7',
              'B2': 'General formula',
              'A3': '45000',
              'B3': 'Date',
              'A4': "'123456789.012345",
              'B4': 'Numeric text',
              'A5': '=1/0',
              'B5': 'Formula error',
              'A6': '-1',
              'B6': 'Invalid date',
              'A7': '=TEXT(A1,"0.0000")',
              'B7': 'Formula returning text',
              'A8': '12345.6789',
              'B8': 'Shrink to fit',
            }.entries)
              {'sheetId': id, 'address': entry.key, 'value': entry.value},
          ],
        });
        for (final group in [
          {
            'addresses': ['A1', 'A8'],
            'style': {'numFmt': '0.0000'},
          },
          {
            'addresses': ['A3', 'A6'],
            'style': {'numFmt': 'yyyy-mm-dd'},
          },
          {
            'addresses': ['A8'],
            'style': {'shrinkToFit': true},
          },
        ]) {
          await core.call({'op': 'formatCells', 'sheetId': id, ...group});
        }
        await core.call({
          'op': 'sheetGeometry',
          'sheetId': id,
          'axis': 'columns',
          'action': 'resize',
          'at': 1,
          'count': 1,
          'size': 7.0,
        });
        await core.call({
          'op': 'sheetGeometry',
          'sheetId': id,
          'axis': 'columns',
          'action': 'resize',
          'at': 2,
          'count': 1,
          'size': 28.0,
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        model.snapshot = await core.call({'op': 'get'});
      });
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
        for (var i = 0; i < 500 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(done, true);
        if (error != null) throw error!;
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
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
      SheetFontRenderBox box(String a) => tester
          .renderObject<SheetFontRenderBox>(find.byKey(Key('sheet-font-$a')));
      final values = jsonEncode(model.snapshot['calculated']);
      final cells = jsonEncode(model.sheets.first['cells']);
      expect(box('A1').paintedText, matches(r'^#+$'));
      expect(box('A3').paintedText, matches(r'^#+$'));
      expect(box('A6').paintedText, matches(r'^#+$'));
      expect(
        box('A2').paintedText,
        isNot(model.snapshot['formatted'][id]['A2']['text']),
      );
      expect(box('A2').paintedText, isNot(matches(r'^#+$')));
      expect(box('A4').paintedText, '123456789.012345');
      expect(box('A5').paintedText, '#DIV/0!');
      expect(box('A7').paintedText, '12345.6789');
      expect(box('A8').paintedText, '12345.6789');
      expect(box('A8').paintScale, lessThan(1));
      await drive(() => grid.select(0, 0));
      expect(grid.formulaController.text, '12345.6789');
      await drive(grid.copy);
      expect(clipboard?['text'], '12345.6789');
      await drive(model.flush);
      await tester.runAsync(
        () => File(model.recoveryPath).copy('${folder.path}/narrow.folio'),
      );
      final revision = model.revision;
      await drive(() => grid.geometry('columnWidth', size: 30));
      expect(model.revision, revision + 1);
      expect(box('A1').paintedText, '12345.6789');
      expect(box('A3').paintedText, '2023-03-15');
      expect(box('A6').paintedText, matches(r'^#+$'));
      expect(box('A6').paintedText.length, greaterThan(8));
      expect(jsonEncode(model.snapshot['calculated']), values);
      expect(jsonEncode(model.sheets.first['cells']), cells);
      await drive(() => screen.history(false));
      expect(box('A1').paintedText, matches(r'^#+$'));
      await drive(() => screen.history(true));
      expect(box('A1').paintedText, '12345.6789');
      await drive(() => screen.history(false));
      await drive(() => grid.autoFitColumns());
      expect(box('A1').paintedText, '12345.6789');
      expect(box('A3').paintedText, '2023-03-15');
      await drive(() => screen.history(false));
      expect(box('A1').paintedText, matches(r'^#+$'));
      await drive(model.flush);
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/number-width.xlsx',
        }),
      );
      await drive(() => model.open(model.recoveryPath));
      expect(box('A1').paintedText, matches(r'^#+$'));
      expect(jsonEncode(model.snapshot['calculated']), values);
      expect(jsonEncode(model.sheets.first['cells']), cells);
    },
  );
}
