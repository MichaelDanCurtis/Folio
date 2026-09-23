import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/formula_inspector.dart';

Map<String, dynamic> cell(String sheet, String range) => {
  'sheetId': sheet,
  'sheetName': sheet == 'a' ? 'Forecast' : 'Sources',
  'address': range.split(':').first,
  'range': range,
  'isRange': range.contains(':'),
};
Map<String, dynamic> result(String sheet, String address, {int offset = 0}) => {
  'cell': cell(sheet, address),
  'input': address == 'C1' ? '=SUM(Sources!B2:B4)' : '25',
  'value': 25,
  'display': '\$25.00',
  'isError': false,
  'precedents': address == 'C1' && offset == 0 ? [cell('b', 'B2:B4')] : [],
  'precedentCount': address == 'C1' ? 1 : 0,
  'dependents': [
    for (var i = offset; i < offset + 100 && i < 205; i++)
      cell('a', 'D${i + 1}'),
  ],
  'dependentCount': 205,
  'warnings': [],
  'unresolvedFormulaCount': 1,
  'unresolvedFormulas': offset == 0
      ? [
          {
            ...cell('b', 'F5'),
            'warnings': ['INDIRECT target not traced.'],
          },
        ]
      : [],
};

void main() {
  testWidgets(
    'inspect cross-sheet links, back, page and return an exact range',
    (t) async {
      t.view.physicalSize = const Size(1200, 900);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      final changes = ValueNotifier(0);
      addTearDown(changes.dispose);
      final reads = <Map<String, dynamic>>[];
      Map? destination;
      await t.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () async {
                  destination = await showDialog<Map>(
                    context: context,
                    builder: (_) => FormulaInspector(
                      sheetId: 'a',
                      address: 'C1',
                      changes: changes,
                      isCurrent: () => true,
                      onRead: (request) async {
                        reads.add(request);
                        return result(
                          request['sheetId'],
                          request['address'],
                          offset: request['offset'],
                        );
                      },
                    ),
                  );
                },
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      );
      await t.tap(find.text('Open'));
      await t.pumpAndSettle();
      expect(find.text('Forecast!C1'), findsOneWidget);
      expect(find.text('Value: \$25.00'), findsOneWidget);
      expect(
        find.textContaining('dependent list may be incomplete'),
        findsOneWidget,
      );
      await t.tap(find.byKey(const Key('inspect-precedents-b-B2:B4')));
      await t.pumpAndSettle();
      expect(find.text('Sources!B2'), findsOneWidget);
      await t.tap(find.byTooltip('Back to previous cell'));
      await t.pumpAndSettle();
      expect(find.text('Forecast!C1'), findsOneWidget);
      await t.tap(find.text('Dependents (205)'));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('audit-more-dependents')));
      await t.pumpAndSettle();
      expect(reads.last['offset'], 100);
      expect(find.text('Show more · 200 of 205'), findsOneWidget);
      await t.tap(find.byKey(const Key('audit-more-dependents')));
      await t.pumpAndSettle();
      expect(reads.last['offset'], 200);
      expect(find.byKey(const Key('audit-more-dependents')), findsNothing);
      await t.tap(find.text('Unresolved (1)'));
      await t.pumpAndSettle();
      expect(find.text('INDIRECT target not traced.'), findsOneWidget);
      await t.tap(find.text('Precedents (1)'));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('goto-precedents-b-B2:B4')));
      await t.pumpAndSettle();
      expect(destination?['sheetId'], 'b');
      expect(destination?['range'], 'B2:B4');
      expect(t.takeException(), isNull);
    },
  );

  testWidgets('a stale response and changed workbook disable navigation', (
    t,
  ) async {
    t.view.physicalSize = const Size(1200, 900);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.resetPhysicalSize);
    addTearDown(t.view.resetDevicePixelRatio);
    final changes = ValueNotifier(0);
    addTearDown(changes.dispose);
    final pending = Completer<Map<String, dynamic>>();
    await t.pumpWidget(
      MaterialApp(
        home: FormulaInspector(
          sheetId: 'a',
          address: 'C1',
          changes: changes,
          isCurrent: () => changes.value == 0,
          onRead: (_) => pending.future,
        ),
      ),
    );
    changes.value = 1;
    await t.pump();
    pending.complete(result('a', 'C1'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('audit-stale')), findsOneWidget);
    expect(find.byKey(const Key('audit-formula')), findsNothing);
    expect(
      t
          .widget<TextButton>(
            find.widgetWithText(TextButton, 'Go to this cell'),
          )
          .onPressed,
      isNull,
    );
    expect(t.takeException(), isNull);
  });

  testWidgets('a read failure is visible and retry restores tracing', (
    t,
  ) async {
    final changes = ValueNotifier(0);
    addTearDown(changes.dispose);
    var failed = true;
    await t.pumpWidget(
      MaterialApp(
        home: FormulaInspector(
          sheetId: 'a',
          address: 'C1',
          changes: changes,
          isCurrent: () => true,
          onRead: (_) async {
            if (failed) throw StateError('Read failed');
            return result('a', 'C1');
          },
        ),
      ),
    );
    await t.pumpAndSettle();
    expect(find.textContaining('Read failed'), findsOneWidget);
    failed = false;
    await t.tap(find.text('Retry'));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('audit-error')), findsNothing);
    expect(find.byKey(const Key('audit-formula')), findsOneWidget);
    expect(t.takeException(), isNull);
  });
}
