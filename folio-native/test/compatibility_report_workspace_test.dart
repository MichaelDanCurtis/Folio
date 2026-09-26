import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'the report lists the file by state and a lossy export asks first',
    (t) async {
      t.view.physicalSize = const Size(1440, 1100);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await t.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-compat-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': File('test/fixtures/signed-small-caps.docx').absolute.path,
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        await model.initialize();
        await model.flush();
      });
      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 500 && !ready(); i++) {
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 6)),
          );
        }
        expect(ready(), true);
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

      // The report: what the file holds, grouped by state.
      final shown = screen.compatibility();
      await until(
        () => find
            .byKey(const ValueKey('compatibility-report-docx'))
            .evaluate()
            .isNotEmpty,
      );
      expect(find.text('Verified · Edited and saved by Folio'), findsOneWidget);
      expect(
        find.text('Unsupported · Not kept or not written'),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('compatibility-feature-any.signatures')),
        findsOneWidget,
      );
      expect(find.textContaining('On export:'), findsNothing);
      await t.tap(find.text('Close'));
      await until(() => find.text('Office compatibility').evaluate().isEmpty);
      await t.runAsync(() => shown);

      // Unchanged: nothing at risk, no preview.
      late bool proceed;
      await t.runAsync(() async {
        proceed = await screen.confirmExportImpacts('docx');
      });
      expect(proceed, true);

      // Editing the small-caps paragraph puts both at risk.
      final doc = Map<String, dynamic>.from(model.workspace['document'] as Map);
      final content = [...doc['content'] as List];
      content[0] = {
        ...content[0] as Map,
        'content': [
          {'type': 'text', 'text': 'Signed heading, edited'},
        ],
      };
      doc['content'] = content;
      await t.runAsync(
        () => model.mutate({
          'op': 'document',
          'document': doc,
        }, refreshDocument: true),
      );
      await t.pump();

      Future<bool> choose(String button) async {
        var result = false;
        var done = false;
        screen.confirmExportImpacts('docx').then((v) {
          result = v;
          done = true;
        });
        await until(
          () => find
              .byKey(const ValueKey('export-impact-any.signatures'))
              .evaluate()
              .isNotEmpty,
        );
        expect(
          find.text('Saving as DOCX will change this file'),
          findsOneWidget,
        );
        expect(
          find.byKey(const ValueKey('export-impact-word.text')),
          findsOneWidget,
        );
        expect(find.textContaining('small caps'), findsOneWidget);
        expect(find.text('Save Folio Copy…'), findsOneWidget);
        // Let the dialog finish opening before choosing.
        await t.pump(const Duration(milliseconds: 400));
        await t.tap(find.text(button));
        await until(() => done);
        await until(
          () => find
              .text('Saving as DOCX will change this file')
              .evaluate()
              .isEmpty,
        );
        return result;
      }

      expect(await choose('Cancel'), false);
      expect(await choose('Export Anyway'), true);

      // The report now says what the export will do.
      final again = screen.compatibility();
      await until(
        () => find.textContaining('On export:').evaluate().isNotEmpty,
      );
      expect(find.textContaining('On export:'), findsNWidgets(2));
      await t.tap(find.text('Close'));
      await until(() => find.text('Office compatibility').evaluate().isEmpty);
      await t.runAsync(() => again);
    },
  );
}
