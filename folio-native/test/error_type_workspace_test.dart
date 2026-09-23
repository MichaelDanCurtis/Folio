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
    'errors and identical text keep types through editing, Paste Values and history',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-error-type-ui-tests',
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
        await CoreBridge.initialize();
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'cells',
          'cells': [
            {
              'sheetId': 'sheet-1',
              'address': 'A1',
              'value': 'Error type check',
            },
            {'sheetId': 'sheet-1', 'address': 'A2', 'value': 'Error'},
            {'sheetId': 'sheet-1', 'address': 'A3', 'value': 'Text'},
            {'sheetId': 'sheet-1', 'address': 'B2', 'value': '=NA()'},
            {'sheetId': 'sheet-1', 'address': 'B3', 'value': '="#N/A"'},
            {'sheetId': 'sheet-1', 'address': 'D2', 'value': '=ISERROR(C2)'},
            {'sheetId': 'sheet-1', 'address': 'D3', 'value': '=ISERROR(C3)'},
          ],
        });
        await core.call({
          'op': 'formatCells',
          'sheetId': 'sheet-1',
          'addresses': ['B2', 'B3', 'C2', 'C3'],
          'style': {'numFmt': '0;0;0;"TEXT: "@'},
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
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
      }

      Future<T> drive<T>(Future<T> Function() job) async {
        var done = false;
        T? value;
        Object? error;
        unawaited(
          Future.sync(job).then(
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
      String shown(String a) =>
          tester.widget<SheetFontText>(find.byKey(Key('sheet-font-$a'))).text;
      expect(shown('B2'), '#N/A');
      expect(shown('B3'), 'TEXT: #N/A');
      await tester.enterText(find.byKey(ValueKey(grid.selectedRange)), 'B2:B3');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await drive(grid.flush);
      await drive(grid.copy);
      expect(clipboard!['text'], '#N/A\r\nTEXT: #N/A');
      final before = jsonEncode(model.sheets);
      await drive(() => grid.select(1, 2));
      await tester.tap(find.byKey(const Key('paste-cells-menu')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Values only'));
      await until(
        () => model.revision > 1 && model.sheets.first['cells']['C2'] == '#N/A',
      );
      expect(model.sheets.first['cells']['C3'], "'#N/A");
      expect(shown('C2'), '#N/A');
      expect(shown('C3'), 'TEXT: #N/A');
      expect(model.snapshot['calculated']['sheet-1']['D2'], true);
      expect(model.snapshot['calculated']['sheet-1']['D3'], false);
      final pasted = jsonEncode(model.sheets);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.sheets), before);
      await drive(() => screen.history(true));
      expect(jsonEncode(model.sheets), pasted);
      await drive(() => grid.select(1, 1));
      await tester.sendKeyEvent(LogicalKeyboardKey.f2);
      await tester.pumpAndSettle();
      await tester.enterText(find.byKey(const Key('cell-editor')), '="#N/A"');
      final revision = model.revision;
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await until(() => model.revision == revision + 1);
      expect(shown('B2'), 'TEXT: #N/A');
      await drive(() => screen.history(false));
      expect(shown('B2'), '#N/A');
      expect(jsonEncode(model.sheets), pasted);
      await drive(screen.flush);
      await tester.runAsync(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/errors.xlsx',
        });
        await File(
          '${folder.path}/snapshot.json',
        ).writeAsString(jsonEncode(model.snapshot));
        await model.core.call({'op': 'new', 'blank': true});
        final reopened = await model.core.call({
          'op': 'load',
          'path': model.recoveryPath,
        });
        expect(reopened['workspace']['sheets'][0]['cells']['C2'], '#N/A');
        expect(reopened['workspace']['sheets'][0]['cells']['C3'], "'#N/A");
        expect(reopened['calculated']['sheet-1']['D2'], true);
        expect(reopened['calculated']['sheet-1']['D3'], false);
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/reopened.xlsx',
        });
      });
    },
  );
}
