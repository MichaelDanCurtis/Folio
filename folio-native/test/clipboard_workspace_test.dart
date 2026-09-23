import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_clipboard.dart';

void main() {
  testWidgets(
    'clipboard commands preserve snapshots, formulas, multiline cells, paste options and atomic history',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      Map<String, dynamic>? clipboard;
      Completer<void>? readGate;
      Completer<void>? readStarted;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SheetClipboard.channel,
        (call) async {
          if (call.method == 'write') {
            clipboard = Map<String, dynamic>.from(call.arguments as Map);
            expect(clipboard!.keys.toSet(), {'text', 'token'});
            return null;
          }
          if (call.method == 'read') {
            readStarted?.complete();
            readStarted = null;
            await readGate?.future;
            return clipboard;
          }
          throw MissingPluginException();
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SheetClipboard.channel,
          null,
        ),
      );
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-clipboard-ui-tests',
      );
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        await CoreBridge.initialize();
        final core = CoreBridge();
        final initial = await core.call({'op': 'new', 'blank': true});
        final workspace = initial['workspace'] as Map;
        workspace['sheets'] = [
          {
            'id': 'sheet-1',
            'name': 'Clipboard',
            'cells': {
              'A1': '2',
              'B1': 'One\nTwo\ttab',
              'A2': '=A1*3',
              'B2': "'001",
              'H1': 'old',
              'H4': 'keep',
            },
            'cellStyles': {
              'B1': {'wrapText': true},
              'A2': {'bold': true, 'numFmt': '0.0000'},
              'H1': {'italic': true},
            },
          },
          {'id': 'sheet-2', 'name': 'Second', 'cells': {}},
        ];
        await core.call({'op': 'replace', 'workspace': workspace});
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        model.snapshot = await core.call({'op': 'get'});
      });
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
        for (var i = 0; i < 700 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (error != null) throw error!;
        expect(done, isTrue);
        await tester.pumpAndSettle();
        return value as T;
      }

      Future<void> revision(int wanted) async {
        for (var i = 0; i < 500 && model.revision < wanted; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(model.revision, wanted);
        await tester.pumpAndSettle();
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.switchMode(true));
      final grid = screen.sheetKey.currentState!;
      Future<void> select(String range) async {
        await tester.enterText(find.byKey(ValueKey(grid.selectedRange)), range);
        await tester.testTextInput.receiveAction(TextInputAction.done);
        await drive(grid.flush);
        expect(grid.selectedRange, range);
      }

      Future<void> key(LogicalKeyboardKey k, {bool shift = false}) async {
        grid.gridFocus.requestFocus();
        await tester.pumpAndSettle();
        await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
        if (shift) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyEvent(k);
        if (shift) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
      }

      Future<void> pasteMenu(String label) async {
        await tester.tap(find.byKey(const Key('paste-cells-menu')));
        await tester.pumpAndSettle();
        await tester.tap(find.text(label));
      }

      Map sheet() => model.sheets[0] as Map;
      await select('A1:B2');
      final beforeCopy = model.revision;
      await key(LogicalKeyboardKey.keyC);
      for (var i = 0; i < 300 && clipboard == null; i++) {
        await tester.pump(const Duration(milliseconds: 20));
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 10)),
        );
      }
      expect(clipboard, isNotNull);
      expect(model.revision, beforeCopy);
      expect(clipboard!['text'], contains('"One\nTwo\ttab"'));
      // Changing the original content and format cannot change a copied snapshot.
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 'sheet-1', 'address': 'A1', 'value': '99'},
          ],
        }),
      );
      await select('D4');
      var before = model.revision;
      await key(LogicalKeyboardKey.keyV);
      await revision(before + 1);
      expect(sheet()['cells']['D4'], '2');
      expect(sheet()['cells']['D5'], '=D4*3');
      expect(sheet()['cells']['E4'], 'One\nTwo\ttab');
      expect(sheet()['cells']['E5'], "'001");
      expect(sheet()['cellStyles']['D5']['bold'], true);
      expect(sheet()['rowHeights']['4'], greaterThan(18.75));
      final pasted = jsonEncode(sheet());
      await key(LogicalKeyboardKey.keyZ);
      await revision(before + 2);
      expect(sheet()['cells']['D4'], isNull);
      await key(LogicalKeyboardKey.keyZ, shift: true);
      await revision(before + 3);
      expect(jsonEncode(sheet()), pasted);
      await select('H1');
      before = model.revision;
      await pasteMenu('Values only');
      await revision(before + 1);
      expect(sheet()['cells']['H1'], '2');
      expect(sheet()['cells']['H2'], '6.0');
      expect(sheet()['cellStyles']['H1']['italic'], true);
      await select('A2');
      await drive(grid.copy);
      await select('H4');
      before = model.revision;
      await pasteMenu('Formatting only');
      await revision(before + 1);
      expect(sheet()['cells']['H4'], 'keep');
      expect(sheet()['cellStyles']['H4']['bold'], true);
      await select('A1:B2');
      await drive(grid.copy);
      await select('D10');
      before = model.revision;
      await key(LogicalKeyboardKey.keyV, shift: true);
      await tester.pumpAndSettle();
      expect(find.text('Paste special'), findsOneWidget);
      await tester.tap(find.byType(DropdownButtonFormField<String>));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Values only').last);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Transpose rows and columns'));
      await tester.tap(find.text('Skip blank cells'));
      await tester.tap(find.widgetWithText(FilledButton, 'Paste'));
      await revision(before + 1);
      expect(sheet()['cells']['D10'], '99');
      expect(sheet()['cells']['E10'], '297.0');
      expect(sheet()['cells']['D11'], 'One\nTwo\ttab');
      expect(sheet()['cells']['E11'], "'001");
      clipboard = {'text': '"quoted\nline"\t"tab\there"\r\n"a""b"\tend\r\n'};
      await select('A16');
      before = model.revision;
      await drive(() => grid.paste());
      expect(model.revision, before + 1);
      expect(sheet()['cells']['A16'], 'quoted\nline');
      expect(sheet()['cells']['B16'], 'tab\there');
      expect(sheet()['cells']['A17'], 'a"b');
      // Async clipboard access must keep the invocation's destination.
      clipboard = {'text': 'captured destination'};
      readGate = Completer<void>();
      final firstReadStarted = readStarted = Completer<void>();
      await select('D18');
      final pending = grid.paste();
      await drive(() => firstReadStarted.future);
      await select('F20');
      readGate.complete();
      readGate = null;
      await drive(() => pending);
      expect(sheet()['cells']['D18'], 'captured destination');
      expect(sheet()['cells']['F20'], isNull);
      // A later content edit invalidates an in-flight paste rather than using
      // a newly supplied revision to overwrite the invocation's destination.
      readGate = Completer<void>();
      final nextReadStarted = readStarted = Completer<void>();
      final stalePaste = grid.paste();
      await drive(() => nextReadStarted.future);
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {
              'sheetId': 'sheet-1',
              'address': 'J20',
              'value': 'Concurrent edit',
            },
          ],
        }),
      );
      final afterConcurrent = jsonEncode(model.workspace);
      readGate.complete();
      readGate = null;
      await drive(() => stalePaste);
      expect(jsonEncode(model.workspace), afterConcurrent);
      ScaffoldMessenger.of(
        tester.element(find.byType(WorkspaceScreen)),
      ).clearSnackBars();
      await tester.pumpAndSettle();
      final unchanged = jsonEncode(model.workspace);
      clipboard = {'text': '"broken'};
      await drive(() => grid.paste());
      expect(jsonEncode(model.workspace), unchanged);
      ScaffoldMessenger.of(
        tester.element(find.byType(WorkspaceScreen)),
      ).clearSnackBars();
      await tester.pumpAndSettle();
      clipboard = {'text': 'measurement must fail'};
      model.rowMeasurer = (_) =>
          throw StateError('Expected measurement failure');
      await drive(() => grid.paste());
      expect(jsonEncode(model.workspace), unchanged);
      model.rowMeasurer = null;
      ScaffoldMessenger.of(
        tester.element(find.byType(WorkspaceScreen)),
      ).clearSnackBars();
      await tester.pumpAndSettle();
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/clipboard.xlsx',
        }),
      );
      await drive(model.flush);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      await drive(
        () => model.core.call({'op': 'load', 'path': model.recoveryPath}),
      );
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/reopened.xlsx',
        }),
      );
      expect(
        File('${folder.path}/clipboard.xlsx').readAsBytesSync(),
        File('${folder.path}/reopened.xlsx').readAsBytesSync(),
      );
      model.dispose();
    },
  );
}
