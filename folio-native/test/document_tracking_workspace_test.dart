import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_tracking.dart';

void main() {
  testWidgets(
    'tracked typing survives editor queues, history, review and recovery',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp(
          'folio-tracking-widget-',
        );
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [
              for (final text in ['aaaa😀', 'Tail'])
                {
                  'type': 'paragraph',
                  'content': [
                    {'type': 'text', 'text': text},
                  ],
                },
            ],
          },
        });
        model = WorkspaceModel(core, '${folder.path}/recovery.folio');
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
        for (var i = 0; i < 600 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 5)),
          );
        }
        if (error != null) throw error!;
        expect(done, isTrue);
        await tester.pumpAndSettle();
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 400 && !ready(); i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 5)),
          );
        }
        expect(ready(), isTrue);
        await tester.pumpAndSettle();
      }

      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
      });
      final initialRevision = model.revision;
      await tester.tap(find.text('Review').first);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('document-tracking-toggle')));
      await until(
        () => find.byType(DocumentTrackingDialog).evaluate().isNotEmpty,
      );
      await tester.tap(find.byKey(const Key('tracking-apply')));
      await tester.pumpAndSettle();
      expect(
        find.text('Enter the name to show beside your changes.'),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('tracking-cancel')));
      await tester.pumpAndSettle();
      expect(model.revision, initialRevision);
      await tester.tap(find.byKey(const Key('document-tracking-toggle')));
      await until(
        () => find.byType(DocumentTrackingDialog).evaluate().isNotEmpty,
      );
      await tester.enterText(find.byKey(const Key('tracking-author')), 'Alex');
      await tester.tap(find.byKey(const Key('tracking-apply')));
      await until(() => find.byType(DocumentTrackingDialog).evaluate().isEmpty);
      expect(find.text('Recording changes as Alex'), findsOneWidget);
      void replace(int start, int length, String text) =>
          screen.editor.replaceText(
            start,
            length,
            text,
            TextSelection.collapsed(offset: start + text.length),
          );
      // Queue these without waiting for a native reply, as during fast typing.
      replace(2, 2, 'XY');
      replace(4, 0, '\n');
      replace(7, 0, '!');
      await drive(screen.flush);
      expect(screen.editor.document.toPlainText(), 'aaXY\n😀!\nTail\n');
      screen.editor.formatText(0, 2, q.Attribute.bold);
      await drive(screen.flush);
      replace(4, 1, '');
      await drive(screen.flush);
      expect(screen.editor.document.toPlainText(), 'aaXY😀!\nTail\n');
      await drive(() => screen.history(false));
      expect(screen.editor.document.toPlainText(), 'aaXY\n😀!\nTail\n');
      await drive(() => screen.history(true));
      expect(screen.editor.document.toPlainText(), 'aaXY😀!\nTail\n');
      final document = model.workspace['document'];
      final artifacts = Directory(
        '${Directory.systemTemp.path}/folio-track-widget-artifacts',
      );
      await tester.runAsync(() => artifacts.create(recursive: true));
      await drive(() async {
        await model.saveCopy('${artifacts.path}/tracked.folio');
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${artifacts.path}/tracked.docx',
        });
        final changes = await model.core.call({'op': 'documentRevisions'});
        expectSync(
          (changes['revisions'] as List).length,
          greaterThanOrEqualTo(3),
        );
        await model.mutate({
          'op': 'documentRevision',
          'workspaceId': model.workspace['id'],
          'baseRevision': model.revision,
          'action': 'reject',
          'keys': [for (final change in changes['revisions']) change['key']],
        }, refreshDocument: true);
      });
      expect(screen.editor.document.toPlainText(), 'aaaa😀\nTail\n');
      expect(
        screen.editor.document.toDelta().toJson().any(
          (op) => op['attributes']?['bold'] == true,
        ),
        isFalse,
      );
      await drive(() => model.open('${artifacts.path}/tracked.folio'));
      expect(model.workspace['document'], document);
      expect(screen.editor.document.toPlainText(), 'aaXY😀!\nTail\n');
      replace(7, 0, '?');
      await drive(screen.flush);
      expect(screen.editor.document.toPlainText(), 'aaXY😀!?\nTail\n');
      // A rejected edit must remain visible and prevent saving an older state.
      final committed = model.workspace['document'];
      screen.editor.formatText(0, 2, q.LinkAttribute('https://example.test'));
      await until(
        () =>
            find.byKey(const Key('document-edit-error')).evaluate().isNotEmpty,
      );
      expect(model.workspace['document'], committed);
      Object? saveError;
      await drive(() async {
        try {
          await screen.flush();
        } catch (e) {
          saveError = e;
        }
      });
      expect(saveError, isNotNull);
      expect(
        screen.editor.document.toDelta().toJson().any(
          (op) => op['attributes']?['link'] != null,
        ),
        isTrue,
      );
      await tester.tap(find.byKey(const Key('discard-unrecorded-edits')));
      await until(
        () => find.byKey(const Key('document-edit-error')).evaluate().isEmpty,
      );
      await drive(screen.flush);
      expect(
        screen.editor.document.toDelta().toJson().any(
          (op) => op['attributes']?['link'] != null,
        ),
        isFalse,
      );
      replace(8, 0, '!');
      await drive(screen.flush);
      expect(screen.editor.document.toPlainText(), 'aaXY😀!?!\nTail\n');
      // The non-conflicting Control shortcut leaves Command+Shift+E for Export.
      screen.documentKey.currentState!.reveal(8, 8, requestFocus: true);
      await tester.pumpAndSettle();
      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyE);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await until(
        () => model.workspace['officeDocument']['tracking']['enabled'] == false,
      );
      expect(
        find.text(
          'Recording is off. Earlier changes remain available for review.',
        ),
        findsOneWidget,
      );
      await drive(
        () => model.mutate({
          'op': 'importOffice',
          'path': File('test/fixtures/tracked-recording.docx').absolute.path,
        }, refreshDocument: true),
      );
      expect(screen.editor.readOnly, isTrue);
      expect(find.text('Choose author'), findsOneWidget);
      await tester.tap(find.text('Choose author'));
      await until(
        () => find.byType(DocumentTrackingDialog).evaluate().isNotEmpty,
      );
      await tester.enterText(
        find.byKey(const Key('tracking-author')),
        'Morgan',
      );
      await tester.tap(find.byKey(const Key('tracking-apply')));
      await until(() => find.byType(DocumentTrackingDialog).evaluate().isEmpty);
      expect(screen.editor.readOnly, isFalse);
      expect(find.text('Recording changes as Morgan'), findsOneWidget);
      await drive(screen.flush);
      screen.editor.formatText(0, 2, q.LinkAttribute('https://example.test'));
      await until(
        () =>
            find.byKey(const Key('document-edit-error')).evaluate().isNotEmpty,
      );
      await tester.tap(find.byKey(const Key('discard-unrecorded-edits')));
      await until(
        () => find.byKey(const Key('document-edit-error')).evaluate().isEmpty,
      );
      await drive(screen.flush);
      final end = screen.editor.document.length - 1;
      screen.documentKey.currentState!.reveal(end, end, requestFocus: true);
      await tester.pumpAndSettle();
      final raw = tester.state<q.QuillRawEditorState>(
        find.byType(q.QuillRawEditor).first,
      );
      final value = raw.textEditingValue;
      raw.updateEditingValue(
        value.copyWith(
          text: '${value.text}\n',
          selection: TextSelection.collapsed(offset: value.text.length + 1),
        ),
      );
      await drive(screen.flush);
      expect(screen.editor.document.toPlainText(), '${value.text}\n');
      expect(screen.editor.selection.extentOffset, end + 1);
      final nextValue = raw.textEditingValue;
      raw.updateEditingValue(
        nextValue.copyWith(
          text: nextValue.text.replaceRange(end + 1, end + 1, 'Confirmed.'),
          selection: TextSelection.collapsed(offset: end + 11),
        ),
      );
      await drive(screen.flush);
      expect(screen.editor.document.toPlainText(), '${value.text}Confirmed.\n');
      expect(tester.takeException(), isNull);
    },
    timeout: const Timeout(Duration(minutes: 2)),
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
