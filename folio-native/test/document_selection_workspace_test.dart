import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'committed refresh keeps selection, text history maps it, open and new reset it',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-selection-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [
              for (final value in [
                'First paragraph',
                'Body 😀 text',
                'Final paragraph',
              ])
                {
                  'type': 'paragraph',
                  'content': [
                    {'type': 'text', 'text': value},
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
          action().then(
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
        expect(done, true);
        await tester.pumpAndSettle();
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
        await tester.runAsync(() => folder.delete(recursive: true));
      });
      const selected = TextSelection(
        baseOffset: 28,
        extentOffset: 16,
        isDirectional: true,
      );
      screen.editor.updateSelection(selected, q.ChangeSource.local);
      await drive(
        () => model.mutate({
          'op': 'documentStyle',
          'action': 'apply',
          'styleId': 'Heading2',
          'paths': [
            [1],
          ],
          'workspaceId': model.workspace['id'],
          'baseRevision': model.revision,
        }, refreshDocument: true),
      );
      expect(screen.editor.selection, selected);
      tester
          .widget<q.QuillEditor>(find.byType(q.QuillEditor).first)
          .focusNode
          .requestFocus();
      screen.editor.updateSelection(
        const TextSelection.collapsed(offset: 21),
        q.ChangeSource.local,
      );
      await tester.pumpAndSettle();
      await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
      await tester.pumpAndSettle();
      expect(
        screen.editor.selection.start,
        16,
        reason:
            'Selecting a visual line must not capture the preceding paragraph break',
      );
      expect(screen.editor.selection.end, 28);
      screen.editor.updateSelection(selected, q.ChangeSource.local);
      await drive(() => screen.history(false));
      expect(screen.editor.selection, selected);
      await drive(() => screen.history(true));
      expect(screen.editor.selection, selected);

      // Metadata refreshes must not move the document cursor.
      await drive(
        () => model.mutate({
          'op': 'title',
          'title': 'Selection check',
        }, refreshDocument: true),
      );
      expect(screen.editor.selection, selected);
      await drive(() => screen.history(false));
      expect(screen.editor.selection, selected);

      screen.editor.updateSelection(
        const TextSelection.collapsed(offset: 16),
        q.ChangeSource.local,
      );
      screen.editor.replaceText(
        16,
        0,
        '🚀 ',
        const TextSelection.collapsed(offset: 19),
      );
      await drive(screen.flush);
      expect(screen.editor.selection.baseOffset, 19);
      await drive(() => screen.history(false));
      expect(screen.editor.selection.baseOffset, 16);
      expect(screen.editor.document.toPlainText(), isNot(contains('🚀')));
      await drive(() => screen.history(true));
      expect(screen.editor.selection.baseOffset, 19);
      expect(screen.editor.document.toPlainText(), contains('🚀 Body 😀 text'));

      // Reopening the same workspace ID is still an explicit navigation reset.
      final path = '${folder.path}/saved.folio';
      await drive(() => model.saveCopy(path));
      final identity = model.workspace['id'];
      await drive(() => model.open(path));
      expect(model.workspace['id'], identity);
      expect(screen.editor.selection, const TextSelection.collapsed(offset: 0));
      screen.editor.updateSelection(selected, q.ChangeSource.local);
      Object? openError;
      await drive(() async {
        try {
          await model.open('${folder.path}/missing.folio');
        } catch (error) {
          openError = error;
        }
      });
      expect(openError, isNotNull);
      expect(screen.editor.selection, selected);
      await drive(model.newWorkspace);
      expect(screen.editor.selection, const TextSelection.collapsed(offset: 0));
      expect(screen.editor.document.toPlainText(), '\n');
      final wrapped = List.filled(35, 'Wrapped body text').join(' ');
      final fullText = 'First\n$wrapped\n\nFinal\n';
      screen.editor.replaceText(
        0,
        0,
        fullText,
        const TextSelection.collapsed(offset: 6),
      );
      await drive(screen.flush);
      tester
          .widget<q.QuillEditor>(find.byType(q.QuillEditor).first)
          .focusNode
          .requestFocus();
      await tester.pumpAndSettle();
      final raw = tester.state<q.QuillRawEditorState>(
        find.byType(q.QuillRawEditor).first,
      );
      final line = raw.renderEditor.getLineAtOffset(
        const TextPosition(offset: 100),
      );
      expect(line.start, greaterThan(6));
      expect(line.end, lessThan(6 + wrapped.length));
      final middle = (line.start + line.end) ~/ 2;
      screen.editor.updateSelection(
        TextSelection.collapsed(offset: middle),
        q.ChangeSource.local,
      );
      Future<void> expandLine(bool forward) async {
        await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
        await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyEvent(
          forward
              ? LogicalKeyboardKey.arrowRight
              : LogicalKeyboardKey.arrowLeft,
        );
        await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
        await tester.pumpAndSettle();
      }

      await expandLine(true);
      expect(screen.editor.selection.start, middle);
      expect(screen.editor.selection.end, line.end);
      await expandLine(false);
      expect(screen.editor.selection.start, line.start);
      expect(
        screen.editor.selection.end,
        line.end,
        reason:
            'Expanding the other edge must not shrink an existing selection',
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
      await drive(screen.flush);
      final shortened = fullText.replaceRange(line.start, line.end, '');
      expect(screen.editor.document.toPlainText(), '$shortened\n');
      final emptyOffset = shortened.indexOf('\n\n') + 1;
      screen.editor.updateSelection(
        TextSelection.collapsed(offset: emptyOffset),
        q.ChangeSource.local,
      );
      await expandLine(false);
      await expandLine(true);
      expect(
        screen.editor.selection,
        TextSelection.collapsed(offset: emptyOffset),
      );
      await drive(screen.flush);
      expect(tester.takeException(), null);
    },
    timeout: const Timeout(Duration(minutes: 2)),
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
