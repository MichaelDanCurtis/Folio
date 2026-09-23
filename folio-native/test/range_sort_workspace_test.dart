import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'range sort preserves headers and hidden rows, flushes pending edits and supports headerless sorting',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late Directory dir;
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-range-sort-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/range-sort.xlsx',
        });
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
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
        for (var i = 0; i < 500 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (!done) throw StateError('Native operation did not finish');
        if (error != null) throw error!;
        return value as T;
      }

      Future<void> settle() async {
        await tester.pump(const Duration(milliseconds: 300));
        await tester.pumpAndSettle();
      }

      Future<void> changed(int previous) async {
        for (var i = 0; i < 300 && model.revision == previous; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(model.revision, greaterThan(previous));
        await settle();
      }

      Future<void> menuReady(String label) async {
        for (var i = 0; i < 300 && find.text(label).evaluate().isEmpty; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        await settle();
        expect(find.text(label), findsOneWidget);
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await settle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.switchMode(true));
      await settle();
      final grid = screen.sheetKey.currentState!;
      await drive(() => grid.select(1, 1));
      await tester.enterText(find.byKey(const Key('formula')), '5');
      await tester.tap(find.byKey(const Key('sort-range-button')));
      await menuReady('Sort range');
      expect(model.sheets[0]['cells']['B2'], '5');
      await tester.enterText(find.byKey(const Key('sort-range')), r'$A$1:$D$7');
      await tester.tap(find.byKey(const Key('sort-add-level')));
      await settle();
      var before = model.revision;
      await tester.tap(find.byKey(const Key('table-sort-apply')));
      await changed(before);
      expect(model.sheets[0]['tables'], isEmpty);
      expect(model.sheets[0]['cells']['A1'], 'Label');
      expect(model.sheets[0]['cells']['A2'], 'Apple');
      expect(model.sheets[0]['cells']['B6'], '5');
      expect(model.sheets[0]['cells']['D6'], '=B6*C6');
      expect(model.snapshot['calculated']['office-1']['D6'], 25);
      expect(model.sheets[0]['cells']['A4'], 'Hidden');
      expect(find.byKey(const Key('row-header-3')), findsNothing);
      expect(model.sheets[0]['cells']['E6'], 'fixed-6');
      await drive(() => screen.history(false));
      expect(model.sheets[0]['cells']['A2'], 'Pear');
      expect(model.sheets[0]['cells']['B2'], '5');
      await drive(() => screen.history(false));
      expect(model.sheets[0]['cells']['B2'], '3');
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {'sheetId': 'office-1', 'address': 'G1', 'value': 'A'},
            {'sheetId': 'office-1', 'address': 'G2', 'value': 'c'},
            {'sheetId': 'office-1', 'address': 'G3', 'value': 'b'},
          ],
        }),
      );
      await drive(() => grid.select(0, 6));
      await tester.tap(find.byKey(const Key('sort-range-button')));
      await menuReady('Sort range');
      await tester.enterText(find.byKey(const Key('sort-range')), 'G1:G3');
      await tester.tap(find.byKey(const Key('sort-headers')));
      await settle();
      await tester.tap(find.byKey(const ValueKey('sort-direction-0-false')));
      await settle();
      await tester.tap(find.text('Descending').last);
      await settle();
      before = model.revision;
      await tester.tap(find.byKey(const Key('table-sort-apply')));
      await changed(before);
      expect(model.sheets[0]['cells']['G1'], 'c');
      expect(model.sheets[0]['cells']['G2'], 'b');
      expect(model.sheets[0]['cells']['G3'], 'A');
      expect(model.sheets[0]['rangeSort']['headers'], false);
      await tester.tap(find.byKey(const Key('sort-range-button')));
      await menuReady('Sort range');
      expect(
        tester
            .widget<CheckboxListTile>(find.byKey(const Key('sort-headers')))
            .value,
        false,
      );
      expect(
        tester
            .widget<TextField>(find.byKey(const Key('sort-range')))
            .controller!
            .text,
        'G1:G3',
      );
      expect(
        find.byKey(const ValueKey('sort-direction-0-true')),
        findsOneWidget,
      );
      await tester.tap(find.text('Cancel'));
      await settle();
      await drive(screen.flush);
      final saved = await tester.runAsync(
        () => File('${dir.path}/workspace.folio').readAsString(),
      );
      expect(jsonDecode(saved!)['sheets'][0]['cells']['G1'], 'c');
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      model.dispose();
      await tester.runAsync(() => dir.delete(recursive: true));
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );
}
