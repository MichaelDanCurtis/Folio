import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'worksheet toolbar edits retain imported formats through history and reload',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      final folder = Directory(
        '${Directory.systemTemp.path}/folio-style-ui-tests',
      );
      await tester.runAsync(() async {
        await folder.create(recursive: true);
        await CoreBridge.initialize();
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/styles.xlsx',
        });
        model = WorkspaceModel(core, '${folder.path}/recovery.folio');
        // Test exports remain available to the independent reader. Do not load
        // the recovery file from a previous invocation of this isolated test.
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

      Future<void> revision(int wanted) async {
        for (var i = 0; i < 300 && model.revision < wanted; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(model.revision, wanted);
        await settle();
      }

      Future<void> export(String name) async {
        await drive(
          () => model.core.call({
            'op': 'exportOffice',
            'format': 'xlsx',
            'path': '${folder.path}/$name.xlsx',
          }),
        );
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox());
        model.dispose();
      });
      await settle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.switchMode(true));
      await settle();
      final grid = screen.sheetKey.currentState!;
      await tester.enterText(find.byKey(ValueKey(grid.selectedRange)), 'B2');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await drive(grid.flush);
      await settle();
      final before = model.revision;
      await tester.tap(find.byTooltip('Bold'));
      await revision(before + 1);
      expect(model.sheets[0]['cellStyles']['B2']['bold'], false);
      expect(model.sheets[0]['cellStyles']['B2']['fontName'], 'Cambria');
      await drive(() => screen.history(false));
      await export('undone');
      await drive(() => screen.history(true));
      await tester.enterText(find.byKey(const Key('formula')), '=A2*\$H\$1+1');
      final pending = model.revision;
      await tester.tap(find.byTooltip('Italic'));
      await revision(pending + 2);
      expect(model.sheets[0]['cellStyles']['B2']['italic'], false);
      expect(model.sheets[0]['cells']['B2'], '=A2*\$H\$1+1');
      await export('edited');
      await drive(() => screen.history(false));
      expect(model.sheets[0]['cellStyles']['B2']['italic'], true);
      expect(model.sheets[0]['cells']['B2'], '=A2*\$H\$1+1');
      await drive(() => screen.history(true));
      await export('redone');
      await drive(
        () => model.core.call({
          'op': 'save',
          'path': '${folder.path}/saved.folio',
        }),
      );
      await drive(
        () => model.mutate({
          'op': 'load',
          'path': '${folder.path}/saved.folio',
        }, refreshDocument: true),
      );
      await export('reopened');
      await drive(model.flush);
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant({TargetPlatform.macOS}),
  );
}
