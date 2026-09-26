import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'spilled results show with their outline, and the Name Manager edits names',
    (t) async {
      t.view.physicalSize = const Size(1440, 1000);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await t.runAsync(() async {
        await CoreBridge.initialize();
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        final w = (await core.call({'op': 'get'}))['workspace'];
        w['sheets'] = [
          {
            'id': 'data',
            'name': 'Data',
            'cells': {'A1': '=SEQUENCE(3)', 'C1': '=Rate*2'},
          },
        ];
        await core.call({'op': 'replace', 'workspace': w});
        folder = await Directory.systemTemp.createTemp('folio-names-ui-');
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        await model.initialize();
      });
      Future<void> drive(Future<void> Function() action) async {
        var done = false;
        Object? error;
        unawaited(
          Future.sync(action).then(
            (_) => done = true,
            onError: (Object e) {
              error = e;
              done = true;
            },
          ),
        );
        for (var i = 0; i < 400 && !done; i++) {
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (error != null) throw error!;
        expect(done, isTrue);
        await t.pumpAndSettle();
      }

      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 400 && !ready(); i++) {
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(ready(), isTrue);
        await t.pumpAndSettle();
      }

      await t.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await t.pumpAndSettle();
      final screen = t.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      addTearDown(() async {
        await t.pumpWidget(const SizedBox());
        model.dispose();
      });
      await drive(() => screen.switchMode(true));
      Object? calculated(String a) =>
          (model.snapshot['calculated'] as Map)['data'][a];

      // The spill reaches the grid.
      expect(calculated('A3'), 3);
      expect(model.snapshot['spills'], {
        'data': {'A1': 'A1:A3'},
      });
      await drive(() async {
        await screen.sheetKey.currentState!.select(1, 0, extend: false);
      });
      final formula = t.widget<TextField>(
        find.descendant(
          of: find.byKey(const Key('formula')),
          matching: find.byType(TextField),
          matchRoot: true,
        ),
      );
      expect(formula.decoration!.hintText, '=SEQUENCE(3)');
      Decoration? outline(String a) => t
          .widget<Container>(find.byKey(Key('cell-frame-$a')))
          .foregroundDecoration;
      expect(outline('A2'), isNotNull);
      expect(outline('A3'), isNotNull);
      expect(outline('B2'), isNull);
      // Outside the spill there is no outline.
      await drive(() async {
        await screen.sheetKey.currentState!.select(0, 2, extend: false);
      });
      expect(outline('A2'), isNull);

      // The Name Manager.
      expect(
        (model.snapshot['formatted'] as Map)['data']['C1']['text'],
        '#NAME?',
      );
      await t.tap(find.text('Formulas'));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('name-manager-button')));
      await until(() => find.text('Name Manager').evaluate().length > 1);
      expect(find.textContaining('No names yet'), findsOneWidget);

      Future<void> fill(String name, String refersTo) async {
        await t.enterText(find.byKey(const Key('name-field')), name);
        await t.enterText(find.byKey(const Key('name-refers-to')), refersTo);
        await t.tap(find.byKey(const Key('name-save')));
      }

      await t.tap(find.byKey(const Key('name-new')));
      await t.pumpAndSettle();
      // An invalid name is refused with the engine's explanation.
      await fill('A1', '=0.5');
      await until(
        () => find.byKey(const Key('name-error')).evaluate().isNotEmpty,
      );
      expect(find.textContaining("isn't a valid name"), findsOneWidget);
      await fill('Rate', '=0.5');
      await until(
        () => find
            .byKey(const Key('name-row-Rate-workbook'))
            .evaluate()
            .isNotEmpty,
      );
      expect(calculated('C1'), 1);
      expect(find.text('0.5'), findsOneWidget);

      // Edit it.
      await t.tap(find.byKey(const Key('name-row-Rate-workbook')));
      // A single tap counts once the double-tap timeout passes.
      await t.pump(const Duration(milliseconds: 400));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('name-edit')));
      await t.pumpAndSettle();
      await fill('Rate', '=2');
      await until(() => calculated('C1') == 4);

      // A named LAMBDA.
      await t.tap(find.byKey(const Key('name-new')));
      await t.pumpAndSettle();
      await fill('Twice', '=LAMBDA(x,x*2)');
      await until(
        () => find
            .byKey(const Key('name-row-Twice-workbook'))
            .evaluate()
            .isNotEmpty,
      );

      // Delete Rate.
      await t.tap(find.byKey(const Key('name-row-Rate-workbook')));
      // A single tap counts once the double-tap timeout passes.
      await t.pump(const Duration(milliseconds: 400));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('name-delete')));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('name-delete-confirm')));
      await until(
        () =>
            find.byKey(const Key('name-row-Rate-workbook')).evaluate().isEmpty,
      );
      expect(
        (model.snapshot['formatted'] as Map)['data']['C1']['text'],
        '#NAME?',
      );
      expect(find.byKey(const Key('name-row-Twice-workbook')), findsOneWidget);
      await t.tap(find.text('Close'));
      await t.pumpAndSettle();
      expect(find.text('Name Manager'), findsOneWidget);
    },
  );
}
