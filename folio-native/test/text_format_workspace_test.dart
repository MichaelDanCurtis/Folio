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
    'TEXT edits, formatted filter/sort/chart labels and persistence',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-text-format-ui-tests',
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
        final fresh = await core.call({'op': 'new', 'blank': true});
        final workspace = fresh['workspace'] as Map;
        workspace['sheets'] = [
          {
            'id': 's',
            'name': 'Formatted labels',
            'cells': {
              'A1': 'Calendar',
              'B1': '0.5',
              'C1': 'Text',
              'D1': 'Format',
              'A2': '60',
              'A3': '61',
              'A4': '62',
              'B2': '0.125',
              'B3': '0.25',
              'B4': '0.5',
              'C2': '=TEXT(B2,D2)',
              'C3': '=TEXT(B3,D3)',
              'C4': '=TEXT(A2,D4)',
              'D2': "'0.0%",
              'D3': "'\$0.00",
              'D4': 'yyyy-mm-dd',
            },
            'cellStyles': {
              'B1': {'numFmt': '0%'},
              for (var r = 2; r <= 4; r++) ...{
                'A$r': {'numFmt': 'yyyy-mm-dd'},
                'B$r': {'numFmt': '0.0%'},
              },
            },
          },
        ];
        await core.call({'op': 'replace', 'workspace': workspace});
        await core.call({
          'op': 'autoFilter',
          'sheetId': 's',
          'action': 'enable',
          'range': 'A1:D4',
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
      String shown(String address) => tester
          .widget<SheetFontText>(find.byKey(Key('sheet-font-$address')))
          .text;
      expect(shown('C2'), '12.5%');
      expect(shown('C3'), '\$0.25');
      expect(shown('C4'), '1900-02-29');
      final before = jsonEncode(model.sheets);
      await drive(() => grid.select(1, 3));
      await tester.enterText(find.byKey(const Key('formula')), "'0.0000");
      final revision = model.revision;
      await drive(grid.flush);
      expect(model.revision, revision + 1);
      expect(shown('C2'), '0.1250');
      expect(model.snapshot['calculated']['s']['B2'], .125);
      final edited = jsonEncode(model.sheets);
      await drive(() => screen.history(false));
      expect(shown('C2'), '12.5%');
      expect(jsonEncode(model.sheets), before);
      await drive(() => screen.history(true));
      expect(shown('C2'), '0.1250');
      expect(jsonEncode(model.sheets), edited);
      await drive(() => grid.select(1, 2));
      await drive(grid.copy);
      expect(clipboard!['text'], '0.1250');

      await tester.tap(find.byTooltip('Chart'));
      await tester.pumpAndSettle();
      expect(find.text('1900-02-29'), findsOneWidget);
      expect(find.text('12.5%'), findsOneWidget);
      expect(find.text('25.0%'), findsOneWidget);
      await tester.tap(find.byTooltip('Chart'));
      await tester.pumpAndSettle();
      await drive(() => grid.select(0, 0));
      await tester.tap(find.byKey(const Key('sort-range-button')));
      await until(() => find.text('Sort range').evaluate().isNotEmpty);
      await tester.tap(find.byKey(const Key('sort-column-0-0')));
      await tester.pumpAndSettle();
      expect(find.text('B — 50%'), findsWidgets);
      await tester.tap(find.text('B — 50%').last);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(jsonEncode(model.sheets), edited);

      await tester.tap(find.byKey(const Key('range-filter-1')));
      await until(() => find.text('Filter 50%').evaluate().isNotEmpty);
      expect(find.text('12.5%'), findsOneWidget);
      expect(find.text('25.0%'), findsOneWidget);
      expect(find.text('50.0%'), findsOneWidget);
      await tester.tap(find.text('Clear selection'));
      await tester.enterText(find.byKey(const Key('filter-search')), '25.0%');
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('filter-value-0.125')), findsNothing);
      await tester.tap(find.byKey(const Key('filter-value-0.25')));
      final beforeFilter = model.revision;
      await tester.tap(find.byKey(const Key('table-filter-apply')));
      await until(() => model.revision == beforeFilter + 1);
      expect(model.sheets.first['autoFilter']['filteredRows'], [2, 4]);
      expect(model.sheets.first['autoFilter']['filters']['1']['values'], [
        '0.25',
      ]);
      expect(find.byKey(const Key('row-header-1')), findsNothing);
      expect(find.byKey(const Key('row-header-2')), findsOneWidget);
      final filtered = jsonEncode(model.sheets);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.sheets), edited);
      await drive(() => screen.history(true));
      expect(jsonEncode(model.sheets), filtered);
      await tester.tap(find.byKey(const Key('range-filter-1')));
      await until(() => find.text('Filter 50%').evaluate().isNotEmpty);
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const Key('filter-value-0.25')),
            )
            .value,
        isTrue,
      );
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const Key('filter-value-0.125')),
            )
            .value,
        isFalse,
      );
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      await drive(screen.flush);
      await tester.runAsync(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/formatted.xlsx',
        });
        await File(
          '${folder.path}/snapshot.json',
        ).writeAsString(jsonEncode(model.snapshot));
        await model.core.call({'op': 'new', 'blank': true});
        final reopened = await model.core.call({
          'op': 'load',
          'path': model.recoveryPath,
        });
        expect(reopened['calculated'], model.snapshot['calculated']);
        expect(reopened['formatted'], model.snapshot['formatted']);
        await model.core.call({
          'op': 'exportOffice',
          'format': 'xlsx',
          'path': '${folder.path}/reopened.xlsx',
        });
        expect(
          await File('${folder.path}/formatted.xlsx').readAsBytes(),
          await File('${folder.path}/reopened.xlsx').readAsBytes(),
        );
      });
    },
  );
}
