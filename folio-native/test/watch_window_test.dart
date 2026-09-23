import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/watch_window.dart';

Map workbook(String id, {bool empty = false}) => {
  'id': id,
  'title': 'Budget',
  'sheets': [
    {
      'id': 'a',
      'name': 'Forecast',
      'cells': {'C1': '=SUM(B1:B3)'},
      'cellWatches': empty ? <String, bool>{} : {'C1': true},
    },
    {
      'id': 'b',
      'name': 'Sources',
      'cells': {'A2': '25', 'A10': '=1/0'},
      'cellWatches': empty ? <String, bool>{} : {'A10': true, 'A2': true},
    },
  ],
};

void main() {
  testWidgets(
    'watch values update, columns resize, navigation and removal retry preserve selection',
    (t) async {
      t.view.physicalSize = const Size(1200, 800);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      final state = ValueNotifier(workbook('first'));
      addTearDown(state.dispose);
      final opened = <(String, String)>[], inspected = <(String, String)>[];
      var fail = true;
      List<Map>? removed;
      await t.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ValueListenableBuilder(
              valueListenable: state,
              builder: (_, w, _) => WatchWindow(
                workbook: w,
                documentEpoch: 0,
                calculated: const {
                  'a': {'C1': 65},
                  'b': {'A2': 25, 'A10': '#DIV/0!'},
                },
                formatted: const {
                  'a': {
                    'C1': {'text': '\$65.00'},
                  },
                  'b': {
                    'A10': {'text': '#DIV/0!', 'isError': true},
                  },
                },
                onAdd: () {},
                onClose: () {},
                onOpen: (id, a) => opened.add((id, a)),
                onInspect: (id, a) => inspected.add((id, a)),
                onRemove: (cells) async {
                  if (fail) throw StateError('Keep selected watches');
                  removed = cells;
                  state.value = workbook('first', empty: true);
                },
              ),
            ),
          ),
        ),
      );
      await t.pumpAndSettle();
      expect(find.text('\$65.00'), findsOneWidget);
      expect(find.text('#DIV/0!'), findsOneWidget);
      expect(
        t.getTopLeft(find.byKey(const Key('watch-row-b-A2'))).dy,
        lessThan(t.getTopLeft(find.byKey(const Key('watch-row-b-A10'))).dy),
      );
      await t.tap(find.byKey(const Key('watch-open-b-A2')));
      await t.pumpAndSettle();
      expect(opened, [('b', 'A2')]);
      await t.tap(find.byKey(const Key('watch-inspect-a-C1')));
      await t.pumpAndSettle();
      expect(inspected, [('a', 'C1')]);
      final before = t.getTopLeft(find.byKey(const Key('watch-column-1'))).dx;
      await t.drag(
        find.byKey(const Key('watch-column-0')),
        const Offset(45, 0),
      );
      await t.pumpAndSettle();
      expect(
        t.getTopLeft(find.byKey(const Key('watch-column-1'))).dx,
        greaterThan(before),
      );
      final height = t.getSize(find.byKey(const Key('watch-window'))).height;
      await t.drag(find.byKey(const Key('watch-resize')), const Offset(0, -50));
      await t.pumpAndSettle();
      expect(
        t.getSize(find.byKey(const Key('watch-window'))).height,
        greaterThan(height),
      );
      await t.tap(find.byKey(const Key('watch-select-all')));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('watch-remove')));
      await t.pumpAndSettle();
      expect(find.textContaining('Keep selected watches'), findsOneWidget);
      expect(find.text('Delete watch (3)'), findsOneWidget);
      fail = false;
      await t.tap(find.byKey(const Key('watch-remove')));
      await t.pumpAndSettle();
      expect(removed, hasLength(3));
      expect(find.text('Watch Window (0)'), findsOneWidget);
      expect(t.takeException(), isNull);
    },
  );

  testWidgets(
    'a late remove failure does not attach itself to another workbook',
    (t) async {
      final state = ValueNotifier(workbook('first'));
      addTearDown(state.dispose);
      final pending = Completer<void>();
      await t.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ValueListenableBuilder(
              valueListenable: state,
              builder: (_, w, _) => WatchWindow(
                workbook: w,
                documentEpoch: 0,
                calculated: const {},
                formatted: const {},
                onAdd: () {},
                onClose: () {},
              onOpen: (_, _) {},
              onInspect: (_, _) {},
                onRemove: (_) => pending.future,
              ),
            ),
          ),
        ),
      );
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('watch-check-a-C1')));
      await t.pump();
      await t.tap(find.byKey(const Key('watch-remove')));
      await t.pump();
      state.value = workbook('second');
      await t.pump();
      pending.completeError(StateError('Old workbook failure'));
      await t.pumpAndSettle();
      expect(find.byKey(const Key('watch-error')), findsNothing);
      expect(
        t.widget<Checkbox>(find.byKey(const Key('watch-check-a-C1'))).value,
        false,
      );
      expect(t.takeException(), isNull);
    },
  );

  testWidgets(
    'Add watch keeps invalid or stale requests reviewable until retry or cancel',
    (t) async {
      var fail = true;
      final added = <(String, String)>[];
      await t.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () => showDialog<void>(
                  context: context,
                  builder: (_) => AddWatchDialog(
                    sheets: workbook('first')['sheets'],
                    sheetId: 'a',
                    range: 'C1',
                    onAdd: (id, a) async {
                      if (fail) throw StateError('Invalid or stale range');
                      added.add((id, a));
                    },
                  ),
                ),
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      );
      await t.tap(find.text('Open'));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('watch-add-confirm')));
      await t.pumpAndSettle();
      expect(find.byKey(const Key('watch-add-error')), findsOneWidget);
      expect(added, isEmpty);
      await t.enterText(find.byKey(const Key('watch-range')), 'A1:B2');
      fail = false;
      await t.tap(find.byKey(const Key('watch-add-confirm')));
      await t.pumpAndSettle();
      expect(added, [('a', 'A1:B2')]);
      expect(find.byKey(const Key('watch-add-dialog')), findsNothing);
    },
  );
}
