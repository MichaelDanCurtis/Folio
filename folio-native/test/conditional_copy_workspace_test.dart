import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/sheet_clipboard.dart';
import 'package:folio_native/src/sheet_icon_set.dart';

void main() {
  testWidgets(
    'conditional copy, merge paste, fill, measured failure, history and Excel recovery use the real worker',
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
        '${Directory.systemTemp.path}/folio-conditional-copy-ui-tests',
      );
      late WorkspaceModel model;
      late String id;
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        await CoreBridge.initialize();
        final core = CoreBridge();
        final initial = await core.call({'op': 'new', 'blank': true});
        id = initial['workspace']['sheets'][0]['id'] as String;
        await core.call({
          'op': 'cells',
          'cells': [
            for (final e in {
              'A1': '0',
              'A2': '25',
              'A3': '75',
              'A4': '100',
              'D1': '5',
              'D2': '9',
              'D3': '99',
            }.entries)
              {'sheetId': id, 'address': e.key, 'value': e.value},
          ],
        });
        for (final rule in [
          {
            'type': 'expression',
            'sqref': 'D1:D4',
            'formulas': ['TRUE'],
            'style': {'fill': '#0000FF', 'italic': true},
          },
          {
            'type': 'expression',
            'sqref': 'A1:A4',
            'formulas': ['A1>=0'],
            'style': {'fill': '#FF0000', 'bold': true},
          },
          {
            'type': 'iconSet',
            'sqref': 'A1:A4',
            'style': {},
            'iconSet': defaultIconSet('3Stars'),
          },
        ]) {
          final before = await core.call({'op': 'get'});
          await core.call({
            'op': 'conditionalFormat',
            'sheetId': id,
            'baseRevision': before['workspace']['revision'],
            'action': 'add',
            'rule': rule,
          });
        }
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

      Future<void> key(LogicalKeyboardKey key, {bool shift = false}) async {
        grid.gridFocus.requestFocus();
        await tester.pumpAndSettle();
        await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
        if (shift) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyEvent(key);
        if (shift) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
      }

      Map style(String cell) =>
          model.snapshot['formatted'][id][cell]['conditionalStyle'] as Map? ??
          {};
      SheetIconGlyph icon(String cell) => tester.widget<SheetIconGlyph>(
        find.byKey(Key('conditional-icon-$cell')),
      );
      await select('A2:A3');
      await key(LogicalKeyboardKey.keyC);
      await until(() => clipboard != null);
      expect(clipboard!.keys.toSet(), {'text', 'token'});
      await select('D1');
      var revision = model.revision;
      await key(LogicalKeyboardKey.keyV);
      await until(() => model.revision == revision + 1);
      expect(style('D1')['fill'], '#FF0000');
      expect(style('D1')['italic'], isNull);
      expect(icon('D1').index, 0);
      expect(icon('D2').index, 2);
      await drive(() => screen.history(false));
      expect(style('D1')['fill'], '#0000FF');
      expect(find.byKey(const Key('conditional-icon-D1')), findsNothing);
      await drive(() => screen.history(true));
      expect(icon('D1').index, 0);
      await drive(() => screen.history(false));
      await select('D1');
      revision = model.revision;
      await key(LogicalKeyboardKey.keyV, shift: true);
      await tester.pumpAndSettle();
      expect(find.text('Paste special'), findsOneWidget);
      await tester.tap(find.byType(DropdownButtonFormField<String>));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Cells and merged conditional formats').last);
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, 'Paste'));
      await until(() => model.revision == revision + 1);
      expect(style('D1')['fill'], '#FF0000');
      expect(style('D1')['italic'], true);
      expect(style('D1')['bold'], true);
      expect(icon('D1').index, 0);
      expect(model.sheets[0]['cells']['D1'], '25');
      await drive(
        () => model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/conditional-paste.xlsx',
        }),
      );
      await select('D1:D4');
      revision = model.revision;
      await drive(() => grid.fillSelection('down'));
      expect(model.revision, revision + 1);
      expect(model.sheets[0]['cells']['D4'], '25');
      expect(icon('D4').set, '3Stars');
      await drive(() => screen.history(false));
      expect(model.sheets[0]['cells']['D3'], '99');
      await drive(() => screen.history(true));
      expect(model.sheets[0]['cells']['D4'], '25');
      await select('G1');
      final unchanged = jsonEncode(model.workspace);
      model.rowMeasurer = (_) =>
          throw StateError('Expected measurement rejection');
      await drive(() => grid.paste());
      expect(jsonEncode(model.workspace), unchanged);
      model.rowMeasurer = null;
      ScaffoldMessenger.of(
        tester.element(find.byType(WorkspaceScreen)),
      ).clearSnackBars();
      await tester.pumpAndSettle();
      await drive(() => screen.flush());
      final rules = jsonEncode(model.sheets[0]['conditionalFormats']);
      late Map<String, dynamic> recovered;
      await drive(() async {
        recovered = await CoreBridge().call({
          'op': 'load',
          'path': '${folder.path}/workspace.folio',
        });
      });
      expect(
        jsonEncode(recovered['workspace']['sheets'][0]['conditionalFormats']),
        rules,
      );
      expect(
        recovered['formatted'][id]['D4']['conditionalStyle']['_iconSet']['set'],
        '3Stars',
      );
    },
  );
}
