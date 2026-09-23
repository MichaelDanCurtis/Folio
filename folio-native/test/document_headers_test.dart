import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_headers.dart';
import 'package:folio_native/src/history_commands.dart';

Map<String, dynamic> doc(String text) => {
  'type': 'doc',
  'content': [
    {
      'type': 'paragraph',
      'content': [
        {'type': 'text', 'text': text},
      ],
    },
  ],
};
Map<String, dynamic> data(Map target) => {
  ...target,
  'workspaceId': 'workspace',
  'baseRevision': 4,
  'storyKey': 'key',
  'linked': false,
  'differentFirstPage': false,
  'differentOddEven': false,
  'affectedSections': [1],
  'sections': [
    {'key': 'one'},
    {'key': 'two'},
  ],
  'document': doc('Current header'),
  'previousDocument': doc('Previous header'),
};

void main() {
  testWidgets(
    'header draft history, link preview and scope-switch draft choice',
    (t) async {
      t.view.physicalSize = const Size(1200, 1000);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      final writes = <Map<String, dynamic>>[];
      await t.pumpWidget(
        MaterialApp(
          localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
          home: Scaffold(
            body: DocumentHeadersDialog(
              initialSection: 1,
              read: (target) async => data(target),
              apply: (r) async => writes.add(r),
            ),
          ),
        ),
      );
      await t.pumpAndSettle();
      q.QuillEditor editor() =>
          t.widget<q.QuillEditor>(find.byType(q.QuillEditor));
      final rich = editor();
      rich.focusNode.requestFocus();
      await t.pump();
      rich.controller.replaceText(
        0,
        0,
        'Draft ',
        const TextSelection.collapsed(offset: 6),
      );
      await t.pumpAndSettle();
      expect(focusedDraftHistory.value?.canUndo, true);
      focusedDraftHistory.value!.undo();
      await t.pumpAndSettle();
      expect(rich.controller.document.toPlainText(), 'Current header\n');
      focusedDraftHistory.value!.redo();
      await t.pumpAndSettle();
      expect(rich.controller.document.toPlainText(), 'Draft Current header\n');
      await t.tap(find.byKey(const Key('header-link')));
      await t.pumpAndSettle();
      expect(editor().controller.document.toPlainText(), 'Previous header\n');
      expect(
        find.textContaining('Apply will replace this section'),
        findsOneWidget,
      );
      await t.tap(find.byKey(const Key('header-link')));
      await t.pumpAndSettle();
      expect(
        editor().controller.document.toPlainText(),
        'Draft Current header\n',
      );
      await t.tap(find.byKey(const Key('header-kind-header')));
      await t.pumpAndSettle();
      await t.tap(find.text('Footer').last);
      await t.pumpAndSettle();
      expect(find.text('Keep these edits?'), findsOneWidget);
      await t.tap(find.text('Keep editing'));
      await t.pumpAndSettle();
      expect(
        editor().controller.document.toPlainText(),
        'Draft Current header\n',
      );
      expect(writes, isEmpty);
      await t.tap(find.byKey(const Key('header-kind-header')));
      await t.pumpAndSettle();
      await t.tap(find.text('Footer').last);
      await t.pumpAndSettle();
      await t.tap(find.text('Discard draft'));
      await t.pumpAndSettle();
      expect(find.byKey(const Key('header-kind-footer')), findsOneWidget);
      expect(writes, isEmpty);
      expect(t.takeException(), isNull);
    },
  );
  testWidgets(
    'header apply failure keeps draft and a failed read keeps its scope',
    (t) async {
      t.view.physicalSize = const Size(1200, 1000);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      await t.pumpWidget(
        MaterialApp(
          localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
          home: Scaffold(
            body: DocumentHeadersDialog(
              initialSection: 1,
              read: (target) async {
                if (target['kind'] == 'footer') {
                  throw StateError('Cannot read footer');
                }
                return data(target);
              },
              apply: (r) async => throw StateError('Document changed'),
            ),
          ),
        ),
      );
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('header-kind-header')));
      await t.pumpAndSettle();
      await t.tap(find.text('Footer').last);
      await t.pumpAndSettle();
      expect(find.byKey(const Key('header-kind-header')), findsOneWidget);
      final controller = t
          .widget<q.QuillEditor>(find.byType(q.QuillEditor))
          .controller;
      controller.replaceText(
        0,
        0,
        'Keep ',
        const TextSelection.collapsed(offset: 5),
      );
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('header-apply')));
      await t.pumpAndSettle();
      expect(find.textContaining('Document changed'), findsOneWidget);
      expect(controller.document.toPlainText(), 'Keep Current header\n');
      expect(controller.readOnly, false);
      expect(t.takeException(), isNull);
    },
  );
  testWidgets('header controls remain usable in a smaller desktop window', (
    t,
  ) async {
    t.view.physicalSize = const Size(800, 650);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.resetPhysicalSize);
    addTearDown(t.view.resetDevicePixelRatio);
    await t.pumpWidget(
      MaterialApp(
        localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
        home: Scaffold(
          body: DocumentHeadersDialog(
            initialSection: 1,
            read: (target) async => data(target),
            apply: (_) async {},
          ),
        ),
      ),
    );
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('header-first')));
    await t.pumpAndSettle();
    expect(
      t.widget<FilledButton>(find.byKey(const Key('header-apply'))).onPressed,
      isNotNull,
    );
    expect(t.getSize(find.byType(q.QuillEditor)).height, greaterThan(80));
    expect(t.takeException(), isNull);
  });
  testWidgets('protected and recording-requested header drafts are read only', (
    t,
  ) async {
    t.view.physicalSize = const Size(1100, 900);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.resetPhysicalSize);
    addTearDown(t.view.resetDevicePixelRatio);
    for (final flag in ['protected', 'needsAuthor', 'recordingEnabled']) {
      await t.pumpWidget(
        MaterialApp(
          localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
          home: Scaffold(
            body: DocumentHeadersDialog(
              key: ValueKey(flag),
              initialSection: 1,
              read: (target) async => {...data(target), flag: true},
              apply: (_) async => fail('Must not apply'),
            ),
          ),
        ),
      );
      await t.pumpAndSettle();
      expect(
        t.widget<q.QuillEditor>(find.byType(q.QuillEditor)).controller.readOnly,
        true,
      );
      expect(
        t.widget<FilledButton>(find.byKey(const Key('header-apply'))).onPressed,
        isNull,
      );
      expect(find.byKey(const Key('header-error')), findsOneWidget);
      expect(t.takeException(), isNull);
    }
  });
}
