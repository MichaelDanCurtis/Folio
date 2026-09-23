import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/scenario_manager.dart';

Future<void> open(WidgetTester t, Widget child) async {
  t.view.physicalSize = const Size(1100, 900);
  t.view.devicePixelRatio = 1;
  addTearDown(t.view.resetPhysicalSize);
  addTearDown(t.view.resetDevicePixelRatio);
  await t.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: TextButton(
            onPressed: () =>
                showDialog<void>(context: context, builder: (_) => child),
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await t.tap(find.text('Open'));
  await t.pumpAndSettle();
}

void main() {
  testWidgets(
    'scenario editor captures cells, keeps failed drafts and retries without changing values on cancel',
    (t) async {
      final saves = <Map>[];
      var fail = true;
      await open(
        t,
        ScenarioEditor(
          selection: 'A1:B1',
          isCurrent: () => true,
          onCapture: (range) async {
            expect(range, 'A1:B1');
            return {
              'inputs': [
                {'address': 'A1', 'value': '10'},
                {'address': 'B1', 'value': '20'},
              ],
            };
          },
          onSave: (v) async {
            if (fail) throw StateError('Write failed');
            saves.add(v);
          },
        ),
      );
      await t.tap(find.byKey(const Key('scenario-save')));
      await t.pumpAndSettle();
      expect(
        find.text('Enter a name and load 1–32 changing cells.'),
        findsOneWidget,
      );
      await t.enterText(find.byKey(const Key('scenario-name')), 'Growth');
      await t.tap(find.byKey(const Key('scenario-load-values')));
      await t.pumpAndSettle();
      await t.enterText(find.byKey(const Key('scenario-value-1')), '30');
      await t.tap(find.byKey(const Key('scenario-save')));
      await t.pumpAndSettle();
      expect(find.textContaining('Write failed'), findsOneWidget);
      expect(saves, isEmpty);
      fail = false;
      await t.tap(find.byKey(const Key('scenario-save')));
      await t.pumpAndSettle();
      expect(saves.single['inputs'], [
        {'address': 'A1', 'value': '10'},
        {'address': 'B1', 'value': '30'},
      ]);
      expect(find.byType(ScenarioEditor), findsNothing);
      expect(t.takeException(), isNull);
    },
  );
  testWidgets(
    'late capture rejects changed workbook and Cancel drops unsaved drafts',
    (t) async {
      var current = true;
      var writes = 0;
      final pending = Completer<Map<String, dynamic>>();
      await open(
        t,
        ScenarioEditor(
          selection: 'A1',
          isCurrent: () => current,
          onCapture: (_) => pending.future,
          onSave: (_) async {
            writes++;
          },
        ),
      );
      await t.enterText(find.byKey(const Key('scenario-name')), 'Unsaved');
      await t.tap(find.byKey(const Key('scenario-load-values')));
      await t.pump();
      current = false;
      pending.complete({
        'inputs': [
          {'address': 'A1', 'value': '8'},
        ],
      });
      await t.pumpAndSettle();
      expect(find.textContaining('The workbook changed'), findsOneWidget);
      expect(find.byKey(const Key('scenario-value-0')), findsNothing);
      await t.tap(find.text('Cancel'));
      await t.pumpAndSettle();
      expect(writes, 0);
    },
  );
  testWidgets(
    'manager requires formula replacement acknowledgement and survives source replacement',
    (t) async {
      final changed = ChangeNotifier();
      addTearDown(changed.dispose);
      var valid = true;
      List sheets = [
        {
          'id': 's',
          'name': 'Plan',
          'cells': {'A1': '=1+2'},
          'scenarios': {
            'items': [
              {
                'name': 'New',
                'inputs': [
                  {'address': 'A1', 'value': '5'},
                ],
              },
            ],
          },
        },
      ];
      final calls = <Map>[];
      await open(
        t,
        ScenarioManager(
          sheets: () => sheets,
          sheetId: 's',
          selection: 'A1',
          changes: changed,
          isCurrent: () => valid,
          onRead: (_) async => {},
          onCommand: (r) async {
            calls.add(r);
          },
        ),
      );
      expect(
        t
            .widget<FilledButton>(find.byKey(const Key('scenario-show')))
            .onPressed,
        isNull,
      );
      await t.tap(find.byKey(const Key('scenario-replace-formulas')));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('scenario-show')));
      await t.pumpAndSettle();
      expect(calls.single['replaceFormulas'], true);
      valid = false;
      sheets = [];
      changed.notifyListeners();
      await t.pumpAndSettle();
      expect(find.textContaining('The workbook changed'), findsOneWidget);
      expect(
        t
            .widget<FilledButton>(find.byKey(const Key('scenario-show')))
            .onPressed,
        isNull,
      );
      expect(t.takeException(), isNull);
    },
  );
  testWidgets(
    'merge chooses source and indices; summary preserves failed input for retry',
    (t) async {
      String? source;
      List<int>? indices;
      await open(
        t,
        ScenarioMergeDialog(
          sheets: [
            {
              'id': 'a',
              'name': 'Budget',
              'scenarios': {
                'items': [
                  {'name': 'Low'},
                  {'name': 'High'},
                ],
              },
            },
          ],
          targetId: 'b',
          onMerge: (id, rows) async {
            source = id;
            indices = rows;
          },
        ),
      );
      await t.tap(find.byType(DropdownButton<String>));
      await t.pumpAndSettle();
      await t.tap(find.text('Budget').last);
      await t.pumpAndSettle();
      await t.tap(find.text('High'));
      await t.pumpAndSettle();
      await t.tap(find.text('Copy scenarios'));
      await t.pumpAndSettle();
      expect(source, 'a');
      expect(indices, [1]);
      var fail = true;
      String? range;
      await open(
        t,
        ScenarioSummaryDialog(
          initial: 'C1',
          onCreate: (r) async {
            if (fail) throw StateError('Calculation unavailable');
            range = r;
          },
        ),
      );
      await t.enterText(
        find.byKey(const Key('scenario-result-cells')),
        'C1,D3',
      );
      await t.tap(find.byKey(const Key('scenario-create-summary')));
      await t.pumpAndSettle();
      expect(find.textContaining('Calculation unavailable'), findsOneWidget);
      fail = false;
      await t.tap(find.byKey(const Key('scenario-create-summary')));
      await t.pumpAndSettle();
      expect(range, 'C1,D3');
      expect(find.byType(ScenarioSummaryDialog), findsNothing);
    },
  );
  testWidgets(
    'external merge keeps workbook identities separate and retries a failed copy',
    (t) async {
      final calls = <Object>[];
      var fail = true;
      await open(
        t,
        ScenarioMergeDialog(
          sheets: const [],
          targetId: 'office-1',
          onMerge: (_, _) async => throw StateError('Local merge must not run'),
          onBrowse: () async => {
            'name': 'Sales.xlsx',
            'token': 7,
            'sheets': [
              {
                'id': 'office-1',
                'name': 'Inputs',
                'scenarios': {
                  'items': [
                    {'name': 'Sales'},
                  ],
                },
              },
            ],
          },
          onExternalMerge: (token, id, indices) async {
            calls.add([token, id, indices]);
            if (fail) throw StateError('Copy failed');
          },
        ),
      );
      await t.tap(find.byKey(const Key('scenario-browse-source')));
      await t.pumpAndSettle();
      expect(find.text('Sales.xlsx'), findsOneWidget);
      await t.tap(find.byType(DropdownButton<String>));
      await t.pumpAndSettle();
      await t.tap(find.text('Inputs').last);
      await t.pumpAndSettle();
      await t.tap(find.text('Sales'));
      await t.pumpAndSettle();
      await t.tap(find.text('Copy scenarios'));
      await t.pumpAndSettle();
      expect(find.textContaining('Copy failed'), findsOneWidget);
      expect(calls.single, [
        7,
        'office-1',
        [0],
      ]);
      fail = false;
      await t.tap(find.text('Copy scenarios'));
      await t.pumpAndSettle();
      expect(calls.length, 2);
      expect(find.byType(ScenarioMergeDialog), findsNothing);
      expect(t.takeException(), isNull);
    },
  );
  testWidgets('late source preview is rejected after workbook changes', (
    t,
  ) async {
    final pending = Completer<Map<String, dynamic>?>();
    var valid = true, writes = 0;
    await open(
      t,
      ScenarioMergeDialog(
        sheets: const [],
        targetId: 'target',
        isCurrent: () => valid,
        onMerge: (_, _) async {
          writes++;
        },
        onBrowse: () => pending.future,
        onExternalMerge: (_, _, _) async {
          writes++;
        },
      ),
    );
    await t.tap(find.byKey(const Key('scenario-browse-source')));
    await t.pump();
    expect(
      t.widget<TextButton>(find.widgetWithText(TextButton, 'Cancel')).onPressed,
      isNull,
    );
    valid = false;
    pending.complete({'name': 'Late.xlsx', 'token': 8, 'sheets': []});
    await t.pumpAndSettle();
    expect(find.text('Late.xlsx'), findsNothing);
    expect(find.textContaining('The workbook changed'), findsOneWidget);
    await t.tap(find.text('Cancel'));
    await t.pumpAndSettle();
    expect(writes, 0);
    expect(t.takeException(), isNull);
  });
  testWidgets(
    'cancelled source picker retains selected source and can return to this workbook',
    (t) async {
      var picks = 0, writes = 0;
      await open(
        t,
        ScenarioMergeDialog(
          sheets: const [],
          targetId: 'target',
          onMerge: (_, _) async {
            writes++;
          },
          onBrowse: () async => picks++ == 0
              ? {'name': 'Source.xlsx', 'token': 9, 'sheets': []}
              : null,
          onExternalMerge: (_, _, _) async {
            writes++;
          },
        ),
      );
      await t.tap(find.byKey(const Key('scenario-browse-source')));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('scenario-browse-source')));
      await t.pumpAndSettle();
      expect(find.text('Source.xlsx'), findsOneWidget);
      await t.tap(find.widgetWithText(TextButton, 'This workbook'));
      await t.pumpAndSettle();
      expect(find.text('Source.xlsx'), findsNothing);
      expect(find.text('This workbook'), findsOneWidget);
      await t.tap(find.text('Cancel'));
      await t.pumpAndSettle();
      expect(writes, 0);
      expect(t.takeException(), isNull);
    },
  );
}
