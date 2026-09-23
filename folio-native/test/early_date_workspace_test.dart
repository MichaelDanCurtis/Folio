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
    'early dates agree in the editor, clipboard and formula results',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-early-date-ui-tests',
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
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/early-dates.xlsx',
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

      Future<void> drive(Future<dynamic> Function() job) async {
        var done = false;
        Object? error;
        unawaited(
          Future.sync(job).then(
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
      String shown(String a) =>
          tester.widget<SheetFontText>(find.byKey(Key('sheet-font-$a'))).text;
      // Serial zero is displayed as a worksheet date, not a Gregorian Dec 31.
      expect(shown('A2'), '1900-01-00');
      expect(shown('B2'), '0');
      await drive(() => grid.select(61, 0));
      expect(shown('A62'), '1900-02-29');
      expect(shown('B62'), '29');
      final before = jsonEncode(model.sheets);
      await tester.sendKeyEvent(LogicalKeyboardKey.f2);
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('cell-editor')),
        '1900-02-28',
      );
      final revision = model.revision;
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await until(() => model.revision == revision + 1);
      expect(shown('A62'), '1900-02-28');
      expect(shown('B62'), '28');
      expect(model.snapshot['calculated']['office-1']['A62'], 59);
      final edited = jsonEncode(model.sheets);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.sheets), before);
      expect(shown('A62'), '1900-02-29');
      expect(shown('B62'), '29');
      await drive(() => screen.history(true));
      expect(jsonEncode(model.sheets), edited);
      expect(shown('B62'), '28');
      await drive(() => grid.select(61, 0));
      await drive(grid.copy);
      expect(clipboard!['text'], '1900-02-28');
      await drive(screen.flush);
      await tester.runAsync(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/early.xlsx',
        });
        await File(
          '${folder.path}/snapshot.json',
        ).writeAsString(jsonEncode(model.snapshot));
        await model.core.call({'op': 'new', 'blank': true});
        final reopened = await model.core.call({
          'op': 'load',
          'path': model.recoveryPath,
        });
        expect(reopened['calculated']['office-1']['A62'], 59);
        expect(reopened['calculated']['office-1']['B62'], 28);
        expect(reopened['formatted']['office-1']['A62']['text'], '1900-02-28');
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/reopened.xlsx',
        });
      });
    },
  );
}
