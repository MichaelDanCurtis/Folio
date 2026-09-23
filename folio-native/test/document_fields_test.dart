import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_bookmarks.dart';
import 'package:folio_native/src/document_edit_tracking.dart';
import 'package:folio_native/src/document_fields.dart';
import 'package:folio_native/src/document_page_numbers.dart';

Map<String, dynamic> fixture() => {
  'type': 'doc',
  'content': [
    {
      'type': 'paragraph',
      'content': [
        {'type': 'text', 'text': 'Alpha '},
        {...newPageField('PAGE'), 'text': '123'},
        {'type': 'text', 'text': ' 😀 after'},
      ],
    },
    {
      'type': 'paragraph',
      'content': [
        newPageField('NUMPAGES'),
        {'type': 'text', 'text': ' tail'},
      ],
    },
  ],
};
void main() {
  test(
    'page fields roundtrip as inline atoms with cached-length bookmark mapping',
    () {
      final doc = fixture(), delta = documentToDelta(fixture());
      expect(deltaToDocument(delta), doc);
      final index = DocumentBookmarkIndex(doc);
      expect(index.text, 'Alpha 123 😀 after\n? tail\n');
      expect(index.logicalOffset(6), 6);
      expect(index.logicalOffset(7), 9);
      expect(index.exactEditorOffset(7), isNull);
      expect(index.exactEditorOffset(8), isNull);
      expect(index.exactEditorOffset(9), 7);
      expect(index.paragraphs[1].editorOffset, 17);
      expect(index.paragraphs[1].embedded, false);
    },
  );
  test(
    'field deletion and editing after cached digits preserve exact journals and Undo',
    () {
      final c = q.QuillController(
        document: q.Document.fromDelta(documentToDelta(fixture())),
        selection: const TextSelection.collapsed(offset: 7),
      );
      final capture = DocumentEditCapture.attach(c.document);
      final before = deltaToDocument(c.document.toDelta());
      c.replaceText(7, 0, 'New ', const TextSelection.collapsed(offset: 11));
      final after = deltaToDocument(c.document.toDelta());
      expect(capture.take(before, after), [
        {'start': 9, 'end': 9, 'text': 'New '},
      ]);
      c.undo();
      expect(deltaToDocument(c.document.toDelta()), before);
      c.replaceText(6, 1, '', const TextSelection.collapsed(offset: 6));
      expect(
        DocumentBookmarkIndex(deltaToDocument(c.document.toDelta())).text,
        'Alpha  😀 after\n? tail\n',
      );
      c.undo();
      expect(deltaToDocument(c.document.toDelta()), before);
      c.dispose();
    },
  );
  testWidgets(
    'page field insertion stays inline, renders a label and supports formatting',
    (t) async {
      final c = q.QuillController.basic();
      final f = FocusNode();
      addTearDown(c.dispose);
      addTearDown(f.dispose);
      await t.pumpWidget(
        MaterialApp(
          localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
          home: Scaffold(
            body: q.QuillEditor.basic(
              controller: c,
              focusNode: f,
              config: const q.QuillEditorConfig(
                embedBuilders: [PageFieldEmbedBuilder()],
              ),
            ),
          ),
        ),
      );
      c.replaceText(0, 0, 'Page ', const TextSelection.collapsed(offset: 5));
      insertPageField(c, 'PAGE');
      c.replaceText(6, 0, ' of ', const TextSelection.collapsed(offset: 10));
      insertPageField(c, 'NUMPAGES');
      await t.pumpAndSettle();
      expect(find.text('‹Page number›'), findsOneWidget);
      expect(find.text('‹Total pages›'), findsOneWidget);
      final value = deltaToDocument(c.document.toDelta());
      expect((value['content'] as List).length, 1);
      expect(value['content'][0]['content'][1]['type'], 'pageField');
      c.formatText(5, 1, q.Attribute.bold);
      expect(
        deltaToDocument(
          c.document.toDelta(),
        )['content'][0]['content'][1]['marks'],
        [
          {'type': 'bold'},
        ],
      );
      expect(t.takeException(), isNull);
    },
  );
  testWidgets(
    'number format validates restart, chapter choices and returns a local draft',
    (t) async {
      Map? result;
      await t.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Builder(
              builder: (context) => TextButton(
                onPressed: () async {
                  result = await showDialog<Map>(
                    context: context,
                    builder: (_) => const PageNumberFormatDialog(
                      initial: defaultPageNumbering,
                      section: 1,
                    ),
                  );
                },
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      );
      await t.tap(find.text('Open'));
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('page-number-restart')));
      await t.pumpAndSettle();
      await t.enterText(find.byKey(const Key('page-number-start')), '-1');
      await t.tap(find.byKey(const Key('page-number-done')));
      await t.pumpAndSettle();
      expect(
        find.textContaining('Enter a whole starting number'),
        findsOneWidget,
      );
      expect(result, isNull);
      await t.enterText(find.byKey(const Key('page-number-start')), '0');
      await t.tap(find.byKey(const Key('page-number-chapter')));
      await t.pumpAndSettle();
      await t.ensureVisible(find.byKey(const Key('page-number-heading')));
      await t.tap(find.byKey(const Key('page-number-heading')));
      await t.pumpAndSettle();
      await t.tap(find.text('Heading 2').last);
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('page-number-done')));
      await t.pumpAndSettle();
      expect(result, {
        'format': 'decimal',
        'start': 0,
        'chapterStyle': 2,
        'chapterSeparator': 'hyphen',
      });
      expect(t.takeException(), isNull);
    },
  );
}
