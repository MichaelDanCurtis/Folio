import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_lists.dart';

void main() {
  testWidgets(
    'Word numbering controls render, persist, export and undo one transaction',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late Directory dir;
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-numbering-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/numbering.docx',
        });
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
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
        for (var i = 0; i < 500 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (error != null) throw error!;
        expect(done, isTrue);
        await tester.pumpAndSettle();
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      final original = jsonEncode(model.workspace['document']);
      expect(find.text('5.a)'), findsOneWidget);
      expect(find.text('6.c)'), findsOneWidget);
      expect(find.text('IV)'), findsOneWidget);
      final revision = model.workspace['revision'] as int;
      screen.editor.updateSelection(
        const TextSelection.collapsed(offset: 0),
        q.ChangeSource.local,
      );
      await tester.tap(find.byKey(const Key('document-numbering-menu')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Set numbering…'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byKey(const Key('numbering-start')), '-1');
      await tester.tap(find.byKey(const Key('numbering-apply')));
      await tester.pumpAndSettle();
      expect(
        find.text('Enter a number from 0 to 2,147,483,647.'),
        findsOneWidget,
      );
      expect(model.workspace['revision'], revision);
      await tester.enterText(find.byKey(const Key('numbering-start')), '7');
      await tester.tap(find.byKey(const Key('numbering-format')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('A, B, C').last);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('numbering-apply')));
      await tester.pumpAndSettle();
      await drive(() => model.flush());
      expect(model.workspace['revision'], revision + 1);
      expect(find.text('G.'), findsOneWidget);
      expect(find.text('G.a)'), findsOneWidget);
      expect(model.workspace['document']['content'][0]['content'][0]['marks'], [
        {'type': 'bold'},
      ]);
      final saved = await tester.runAsync(
        () => File('${dir.path}/workspace.folio').readAsString(),
      );
      expect(jsonDecode(saved!)['document'], model.workspace['document']);
      final exportPath = '${dir.path}/numbering-ui.docx';
      await drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': exportPath,
        });
      });
      expect(await tester.runAsync(() => File(exportPath).exists()), isTrue);
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), original);
      expect(find.text('5.a)'), findsOneWidget);
      await drive(() => screen.history(true));
      expect(find.text('G.'), findsOneWidget);
      // Continue the separate list at the earlier level, retaining prose between.
      final lines = listLines(screen.editor.document.toDelta());
      final separate = screen.editor.document.toPlainText().indexOf(
        'Separate instance',
      );
      screen.editor.updateSelection(
        TextSelection.collapsed(offset: separate),
        q.ChangeSource.local,
      );
      await tester.tap(find.byKey(const Key('document-numbering-menu')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Continue previous list'));
      await tester.pumpAndSettle();
      await drive(() => model.flush());
      expect(
        documentListLabels(screen.editor.document.toDelta())[separate],
        '6.',
      );
      expect(lines.length, 10);
      await drive(() => screen.history(false));
      final numberStart = screen.editor.document.toPlainText().indexOf(
        'Roman item',
      );
      screen.editor.updateSelection(
        TextSelection.collapsed(offset: numberStart),
        q.ChangeSource.local,
      );
      await tester.tap(find.byKey(const Key('document-numbering-menu')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Restart at 1'));
      await tester.pumpAndSettle();
      await drive(() => model.flush());
      expect(
        documentListLabels(screen.editor.document.toDelta())[numberStart],
        'I)',
      );
      await drive(() => screen.history(false));
      expect(
        documentListLabels(screen.editor.document.toDelta())[numberStart],
        'IV)',
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      model.dispose();
      await tester.runAsync(() => dir.delete(recursive: true));
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
