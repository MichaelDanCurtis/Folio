import 'dart:io';
import 'dart:async';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/src/document_anchor.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets('desktop layout edits the real Rust workbook with shared undo', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1440, 960);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    late WorkspaceModel model;
    late Directory dir;
    await tester.runAsync(() async {
      await CoreBridge.initialize();
      dir = await Directory.systemTemp.createTemp('folio-ui-test-');
      model = WorkspaceModel(CoreBridge(), '${dir.path}/workspace.folio');
      await model.initialize();
    });
    await tester.pumpWidget(
      FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(
      find.text('Good ideas start here.', findRichText: true),
      findsOneWidget,
    );
    await tester.tap(find.byTooltip('Workbook'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('cell-B4')));
    await tester.pump(const Duration(milliseconds: 350));
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('cell-editor')), '3000');
    await tester.runAsync(() async {
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await Future<void>.delayed(const Duration(milliseconds: 100));
    });
    await tester.pumpAndSettle();
    expect(model.snapshot['calculated']['budget']['B9'], 20600);
    await tester.tap(find.byKey(const Key('undo')));
    for (var i = 0; i < 10 && model.revision < 2; i++) {
      await tester.pump();
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 30)),
      );
    }
    await tester.pumpAndSettle();
    expect(model.snapshot['calculated']['budget']['B9'], 20000);
    expect(tester.takeException(), isNull);
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
          onError: (Object e, StackTrace st) {
            error = e;
            done = true;
          },
        ),
      );
      for (var i = 0; i < 250 && !done; i++) {
        await tester.pump(const Duration(milliseconds: 20));
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 10)),
        );
      }
      if (!done) {
        throw StateError('Native operation did not complete during UI test');
      }
      if (error != null) throw error!;
      return value as T;
    }

    final screen = tester.state<WorkspaceScreenState>(
      find.byType(WorkspaceScreen),
    );
    await drive(() => screen.switchMode(false));
    await tester.pumpAndSettle();
    final beforeDocument = model.workspace['document'];
    final plain = screen.editor.document.toPlainText();
    final start = plain.indexOf('A place');
    screen.editor.updateSelection(
      TextSelection.collapsed(offset: start < 0 ? 25 : start),
      q.ChangeSource.local,
    );
    screen.anchor = DocumentAnchor.capture(
      screen.editor,
      model.revision,
      model.workspace['id'],
    );
    final selected = screen.anchor!.text;
    await drive(() async {
      final context = await screen.agent.onTool('read_workspace', {});
      expectSync(context['selection']['text'], selected);
      await screen.agent.onTool('propose_changes', {
        'baseRevision': model.revision,
        'summary': 'Source-backed paragraph',
        'replaceSelection': {
          'expectedText': selected,
          'text':
              'The budget is 20,000, with 14,750 spent (Project budget!B9:C9).',
        },
      });
    });
    await tester.pumpAndSettle();
    expect(model.workspace['document'], beforeDocument);
    await drive(() => screen.apply(screen.proposals.last));
    await tester.pumpAndSettle();
    expect(nodeText(model.workspace['document']), contains('14,750 spent'));
    await drive(() => screen.history(false));
    await tester.pumpAndSettle();
    expect(model.workspace['document'], beforeDocument);
    await drive(
      () => screen.agent.onTool('propose_changes', {
        'baseRevision': model.revision,
        'summary': 'Live data block',
        'linkedBlock': {
          'sheetId': 'budget',
          'range': 'A3:D9',
          'title': 'Budget summary',
          'mode': 'table',
        },
      }),
    );
    await drive(() => screen.apply(screen.proposals.last));
    await tester.pumpAndSettle();
    expect(
      model.workspace['document']['content'].last['type'],
      'workbookBlock',
    );
    await drive(() => screen.history(false));
    await tester.pumpAndSettle();
    expect(model.workspace['document'], beforeDocument);
    expect(tester.takeException(), isNull);

    final editorFocus = tester
        .widget<q.QuillEditor>(find.byType(q.QuillEditor))
        .focusNode;
    editorFocus.requestFocus();
    await tester.pump();
    const typed = '😀 We recieved an letter.\nKeep this paragraph.\n';
    tester.testTextInput.updateEditingValue(
      const TextEditingValue(
        text: typed,
        selection: TextSelection.collapsed(offset: typed.length - 1),
      ),
    );
    await drive(screen.flush);
    await tester.pump();
    expect(screen.editor.document.toPlainText(), typed);
    expect(
      tester.widget<q.QuillEditor>(find.byType(q.QuillEditor)).focusNode,
      same(editorFocus),
    );
    expect(editorFocus.hasFocus, isTrue);
    screen.editor.formatText(6, 8, q.Attribute.bold);
    await drive(screen.flush);
    final beforeProofread = model.workspace['document'];
    await drive(screen.proofreading.checkNow);
    for (var i = 0; i < 500 && screen.proofreading.busy; i++) {
      await tester.pump(const Duration(milliseconds: 20));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 10)),
      );
    }
    expect(screen.proofreading.error, isNull);
    final spelling = screen.proofreading.issues.firstWhere(
      (i) => i.text == 'recieved',
    );
    await tester.tap(find.byKey(const Key('writing-editor')));
    await tester.pumpAndSettle();
    expect(find.text('Writing Editor'), findsOneWidget);
    expect(find.text('English (US)'), findsOneWidget);
    expect(find.byKey(const Key('ai-grammar-selection')), findsOneWidget);
    expect(tester.takeException(), isNull);
    await drive(() => screen.fixWriting(spelling, 'received'));
    expect(nodeText(model.workspace['document']), contains('received'));
    await drive(() => screen.history(false));
    expect(model.workspace['document'], beforeProofread);
    screen.anchor = null;
    await drive(() async {
      final context = await screen.agent.onTool('read_workspace', {});
      final block = context['proofreading']['blocks'].first;
      await screen.agent.onTool('propose_changes', {
        'baseRevision': model.revision,
        'summary': 'Correct spelling and grammar.',
        'proofreadEdits': [
          {
            'blockId': block['id'],
            'expectedText': 'recieved',
            'replacement': 'received',
          },
          {
            'blockId': block['id'],
            'expectedText': 'an letter',
            'replacement': 'a letter',
          },
        ],
      });
    });
    expect(model.workspace['document'], beforeProofread);
    await drive(() => screen.apply(screen.proposals.last));
    expect(
      nodeText(model.workspace['document']),
      contains('We received a letter.'),
    );
    expect(
      screen.editor.document.toDelta().toJson().any(
        (op) => op['insert'] == 'received' && op['attributes']?['bold'] == true,
      ),
      isTrue,
    );
    await drive(() => screen.history(false));
    expect(model.workspace['document'], beforeProofread);
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);

    for (var i = 0; i < 20 && model.saveStatus != 'Saved on this Mac'; i++) {
      await tester.pump(const Duration(milliseconds: 400));
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 30)),
      );
    }
    expect(model.saveStatus, 'Saved on this Mac');
    await tester.pumpWidget(const SizedBox());
    model.dispose();
    await tester.runAsync(() => dir.delete(recursive: true));
  });
}
