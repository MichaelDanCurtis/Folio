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
  testWidgets('1904 date entry, clipboard, filters, history and reopen', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1440, 960);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final folder = Directory(
      '${Directory.systemTemp.path}/folio-date-system-ui-tests',
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
        'path': '${Directory.current.path}/test/fixtures/date-system-1904.xlsx',
      });
      await core.call({
        'op': 'autoFilter',
        'sheetId': 'office-1',
        'action': 'enable',
        'range': 'A1:B7',
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
    expect(model.snapshot['date1904'], isTrue);
    expect(shown('A2'), '1904-01-01');
    expect(model.snapshot['calculated']['office-1']['F5'], 0);
    final before = jsonEncode(model.sheets);
    await drive(() => grid.select(1, 3));
    await tester.sendKeyEvent(LogicalKeyboardKey.f2);
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('cell-editor')), '1904-02-29');
    var revision = model.revision;
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await until(() => model.revision == revision + 1);
    expect(shown('D2'), '1904-02-29');
    expect(model.snapshot['calculated']['office-1']['D2'], 59);
    final edited = jsonEncode(model.sheets);
    await drive(() => screen.history(false));
    expect(jsonEncode(model.sheets), before);
    await drive(() => screen.history(true));
    expect(jsonEncode(model.sheets), edited);
    expect(shown('D2'), '1904-02-29');
    await drive(() => grid.select(1, 3));
    await drive(grid.copy);
    expect(clipboard!['text'], '1904-02-29');
    await tester.tap(find.byKey(const Key('range-filter-0')));
    await until(() => find.text('Filter Date').evaluate().isNotEmpty);
    expect(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.text('1904-01-01'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.text('1904-02-29'),
      ),
      findsOneWidget,
    );
    await tester.tap(find.text('Clear selection'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('filter-value-0.0')));
    revision = model.revision;
    await tester.tap(find.byKey(const Key('table-filter-apply')));
    await until(() => model.revision == revision + 1);
    expect(model.sheets.first['autoFilter']['filteredRows'], [3, 4, 5, 6, 7]);
    expect(shown('A2'), '1904-01-01');
    final filtered = jsonEncode(model.sheets);
    await drive(() => screen.history(false));
    expect(jsonEncode(model.sheets), edited);
    await drive(() => screen.history(true));
    expect(jsonEncode(model.sheets), filtered);
    await drive(screen.flush);
    await tester.runAsync(() async {
      await model.core.call({
        'op': 'exportOffice',
        'format': 'xlsx',
        'path': '${folder.path}/calendar.xlsx',
      });
      await File(
        '${folder.path}/snapshot.json',
      ).writeAsString(jsonEncode(model.snapshot));
      await model.core.call({'op': 'new', 'blank': true});
      final reopened = await model.core.call({
        'op': 'load',
        'path': model.recoveryPath,
      });
      expect(reopened['date1904'], isTrue);
      expect(reopened['calculated']['office-1']['D2'], 59);
      expect(reopened['formatted']['office-1']['D2']['text'], '1904-02-29');
      expect(reopened['calculated']['office-1']['F5'], 0);
      expect(reopened['workspace']['sheets'][0]['autoFilter']['filteredRows'], [
        3,
        4,
        5,
        6,
        7,
      ]);
      await model.core.call({
        'op': 'exportOffice',
        'format': 'xlsx',
        'path': '${folder.path}/reopened.xlsx',
      });
    });
  });
}
