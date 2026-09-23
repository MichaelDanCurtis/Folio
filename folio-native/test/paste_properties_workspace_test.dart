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
    'Paste Special properties use the real worker, measured geometry, history and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
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
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-paste-properties-ui-tests',
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
            'tables': [],
            'hiddenRows': [],
            'rowHeightModes': {},
            'autoFilter': null,
            'frozenPane': null,
            'id': 's',
            'name': 'Paste Special',
            'cells': {
              'A1': '=A2*2',
              'A2': '0.25',
              'B1': '7',
              'D1': '99',
              'D2': '5',
              'E2': '3',
            },
            'columnWidths': {'1': 10.5, '4': 35},
            'cellStyles': {
              'A1': {'bold': true, 'numFmt': '0.0000'},
              'D1': {'italic': true, 'fill': '#0000FF'},
            },
          },
        ];
        await core.call({'op': 'replace', 'workspace': workspace});
        final state = await core.call({'op': 'get'});
        await core.call({
          'op': 'dataValidation',
          'sheetId': 's',
          'baseRevision': state['workspace']['revision'],
          'action': 'set',
          'range': 'A1:A2',
          'rule': {
            'type': 'decimal',
            'formula1': '0',
            'formula2': '1',
            'allowBlank': true,
            'showErrorMessage': true,
            'errorStyle': 'stop',
          },
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        model.snapshot = await core.call({'op': 'get'});
      });
      Future<void> until(bool Function() check) async {
        for (var i = 0; i < 700 && !check(); i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(check(), true);
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
      Future<void> select(String range) async {
        await tester.enterText(find.byKey(ValueKey(grid.selectedRange)), range);
        await tester.testTextInput.receiveAction(TextInputAction.done);
        await drive(grid.flush);
        expect(grid.selectedRange, range);
      }

      Future<void> dialogMode(String text, {bool cancel = false}) async {
        unawaited(grid.pasteSpecial());
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key('paste-special-mode')));
        await tester.pumpAndSettle();
        await tester.ensureVisible(find.text(text).last);
        await tester.tap(find.text(text).last);
        await tester.pumpAndSettle();
        if (text.contains('widths')) {
          final tile = tester.widget<CheckboxListTile>(
            find.widgetWithText(CheckboxListTile, 'Transpose rows and columns'),
          );
          expect(tile.value, false);
          expect(tile.onChanged, isNull);
        }
        await tester.tap(
          cancel
              ? find.widgetWithText(TextButton, 'Cancel')
              : find.widgetWithText(FilledButton, 'Paste'),
        );
      }

      Map sheet() => model.sheets[0];
      await select('A1');
      await drive(grid.copy);
      expect(clipboard!.keys.toSet(), {'text', 'token'});
      await select('D1');
      var revision = model.revision;
      await dialogMode('Values and number formats');
      await until(() => model.revision == revision + 1);
      expect(sheet()['cells']['D1'], '0.5');
      expect(sheet()['cellStyles']['D1']['numFmt'], '0.0000');
      expect(sheet()['cellStyles']['D1']['italic'], true);
      expect(sheet()['cellStyles']['D1']['bold'], false);
      expect(sheet()['cellStyles']['D1']['fill'], '#0000FF');
      await drive(() => screen.history(false));
      expect(sheet()['cells']['D1'], '99');
      await drive(() => screen.history(true));
      expect(sheet()['cells']['D1'], '0.5');
      await select('E1');
      revision = model.revision;
      await dialogMode('Formulas and number formats');
      await until(() => model.revision == revision + 1);
      expect(sheet()['cells']['E1'], '=E2*2');
      expect(model.snapshot['calculated']['s']['E1'], 6.0);
      await select('D1');
      revision = model.revision;
      final cells = jsonEncode(sheet()['cells']);
      await dialogMode('Validation rules only');
      await until(() => model.revision == revision + 1);
      expect(jsonEncode(sheet()['cells']), cells);
      final rules = sheet()['dataValidations'] as List;
      expect(
        rules.any((r) => r['sqref'] == 'D1:D1' && r['type'] == 'decimal'),
        true,
      );
      await select('A1:B2');
      await drive(grid.copy);
      final defaultPixels = grid.columnExtent(1);
      await select('D1');
      revision = model.revision;
      await dialogMode('Column widths only');
      await until(() => model.revision == revision + 1);
      expect(jsonEncode(sheet()['cells']), cells);
      expect(sheet()['columnWidths']['4'], 10.5);
      expect(grid.columnExtent(4), defaultPixels);
      await drive(() => screen.history(false));
      expect(sheet()['columnWidths']['4'], 35);
      await drive(() => screen.history(true));
      expect(grid.columnExtent(4), defaultPixels);
      await select('D5');
      revision = model.revision;
      await dialogMode('Cells with source column widths');
      await until(() => model.revision == revision + 1);
      expect(sheet()['cells']['D5'], '=D6*2');
      expect(model.snapshot['calculated']['s']['D5'], 0.5);
      expect(sheet()['cellStyles']['D5']['bold'], true);
      final unchanged = jsonEncode(model.workspace);
      await dialogMode('Column widths only', cancel: true);
      await tester.pumpAndSettle();
      expect(jsonEncode(model.workspace), unchanged);
      await select('J1');
      model.rowMeasurer = (_) =>
          throw StateError('Expected measurement failure');
      await drive(() => grid.paste(mode: 'allColumnWidths'));
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
          'path': '${folder.path}/paste-properties.xlsx',
        }),
      );
      await drive(
        () => model.core.call({
          'op': 'save',
          'path': '${folder.path}/workspace.folio',
        }),
      );
      final loaded = CoreBridge();
      late Map<String, dynamic> result;
      await drive(() async {
        result = await loaded.call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
      });
      expect(result['workspace']['sheets'], model.workspace['sheets']);
      expect(result['calculated']['s']['D5'], 0.5);
    },
  );
}
