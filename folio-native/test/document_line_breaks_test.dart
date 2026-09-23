import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_bookmarks.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_edit_tracking.dart';
import 'package:folio_native/src/document_line_breaks.dart';
import 'package:folio_native/src/document_list_rules.dart';
import 'package:folio_native/src/document_styles.dart';
import 'package:folio_native/src/document_text_style.dart';
import 'package:folio_native/src/search.dart';
import 'package:folio_native/src/proofreading.dart';

Map<String, dynamic> lineDoc(List<Map> content, {Map? attrs}) => {
  'type': 'doc',
  'content': [
    {'type': 'paragraph', 'attrs': ?attrs, 'content': content},
  ],
};
Map<String, dynamic> word(String text) => {'type': 'text', 'text': text};
const br = {'type': 'hardBreak'};
q.QuillController controller(Map doc, int offset) => q.QuillController(
  document: q.Document.fromDelta(documentToDelta(doc)),
  selection: TextSelection.collapsed(offset: offset),
);
Map decoded(q.QuillController c) => deltaToDocument(c.document.toDelta());
Future<void> shiftEnter(WidgetTester t) async {
  await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
  await t.sendKeyEvent(LogicalKeyboardKey.enter);
  await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  await t.pumpAndSettle();
}

void main() {
  testWidgets(
    'proofreading underlines and corrects the exact word after a break',
    (t) async {
      final c = controller(
        lineDoc([word('Before'), br, word('We recieved it.')]),
        0,
      );
      final proof = ProofreadingController(
        c,
        check: (r) async => {
          'issues': [
            {
              'start': 10,
              'end': 18,
              'text': 'recieved',
              'category': 'spelling',
              'message': 'Check spelling.',
              'suggestions': ['received'],
            },
          ],
          'truncated': false,
        },
      );
      await proof.checkNow();
      expect(proof.issues.single.start, 10);
      await t.pumpWidget(
        MaterialApp(
          localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
          home: Scaffold(
            body: q.QuillEditor.basic(
              controller: c,
              config: q.QuillEditorConfig(textSpanBuilder: proof.spanBuilder),
            ),
          ),
        ),
      );
      await t.pumpAndSettle();
      Iterable<TextSpan> spans(InlineSpan span) sync* {
        if (span is TextSpan) {
          yield span;
          for (final child in span.children ?? <InlineSpan>[]) {
            yield* spans(child);
          }
        }
      }

      final rendered = t.widget<q.QuillLayoutText>(
        find.byType(q.QuillLayoutText),
      );
      final underlines = spans(
        rendered.text,
      ).where((s) => s.style?.decorationStyle == TextDecorationStyle.wavy);
      expect(underlines.map((s) => s.text).toList(), ['recieved']);
      final revised = applyWritingEdits(c, [
        {
          'start': 10,
          'end': 18,
          'expectedText': 'recieved',
          'replacement': 'received',
        },
      ]);
      expect(nodeText(revised), 'Before\nWe received it.');
      expect(revised['content'][0]['content'][1]['type'], 'hardBreak');
      await t.pumpWidget(const SizedBox());
      proof.dispose();
      c.dispose();
    },
  );
  test(
    'leading, trailing, consecutive, formatted breaks and literal separators roundtrip',
    () {
      final doc = lineDoc(
        [
          br,
          word('😀 A\u2028literal'),
          {
            'type': 'hardBreak',
            'attrs': {'clear': 'none'},
            'marks': [
              {'type': 'bold'},
            ],
          },
          br,
          word('after'),
          br,
        ],
        attrs: {'officeId': 'p1', 'textAlign': 'center'},
      );
      final c = controller(doc, 0);
      expect(decoded(c), doc);
      expect(
        c.document.toPlainText(),
        '\u2028😀 A\u2028literal\u2028\u2028after\u2028\n',
      );
      expect(nodeText(doc), '\n😀 A\u2028literal\n\nafter\n');
      final index = DocumentBookmarkIndex(doc);
      expect(index.paragraphs.single.embedded, false);
      for (var i = 0; i <= c.document.length; i++) {
        expect(index.logicalOffset(i), i);
      }
      // A plain text U+2028 remains text, even next to real Word breaks.
      expect(decoded(c)['content'][0]['content'][1]['type'], 'text');
      c.dispose();
    },
  );

  test(
    'page, column, wrap-clearing and unknown break metadata remain preserved',
    () {
      for (final attrs in [
        {'breakType': 'page'},
        {'breakType': 'column'},
        {'clear': 'all'},
        {'custom': 3},
      ]) {
        final doc = lineDoc([
          word('before'),
          {'type': 'hardBreak', 'attrs': attrs},
          word('after'),
        ]);
        final delta = documentToDelta(doc);
        expect(delta.first.data, contains('folioNode'));
        expect(deltaToDocument(delta), doc);
        final c = controller(doc, 0);
        insertDocumentLineBreak(c);
        expect(decoded(c), doc);
        c.dispose();
      }
    },
  );

  test(
    'break insertion and following text keep formatting, exact journals and rich copy',
    () {
      final doc = lineDoc([
        {
          'type': 'text',
          'text': 'AlphaBeta',
          'marks': [
            {'type': 'bold'},
          ],
        },
      ]);
      final c = controller(doc, 5);
      final capture = DocumentEditCapture.attach(c.document);
      insertDocumentLineBreak(c);
      final after = decoded(c);
      expect(capture.take(doc, after), [
        {'start': 5, 'end': 5, 'text': '\n'},
      ]);
      expect((after['content'] as List).length, 1);
      expect(after['content'][0]['content'][1]['marks'], [
        {'type': 'bold'},
      ]);
      expect(c.selection.extentOffset, 6);
      c.replaceText(6, 0, 'NEW', const TextSelection.collapsed(offset: 9));
      expect(decoded(c)['content'][0]['content'][2]['marks'], [
        {'type': 'bold'},
      ]);
      final copy = c.document.toDelta().slice(3, 10);
      final target = q.QuillController.basic();
      target.replaceText(0, 0, copy, const TextSelection.collapsed(offset: 7));
      expect(nodeText(decoded(target)), 'ha\nNEWB');
      expect(
        (decoded(target)['content'][0]['content'] as List)
            .where((n) => n['type'] == 'hardBreak')
            .length,
        1,
      );
      c.formatText(5, 1, q.Attribute.clone(q.Attribute.bold, false));
      expect(decoded(c)['content'][0]['content'][1]['marks'], [
        {
          'type': 'bold',
          'attrs': {'value': false},
        },
      ]);
      c.dispose();
      target.dispose();
    },
  );

  test(
    'search and bookmarks address text after a leading break without selecting the paragraph',
    () {
      final doc = lineDoc([br, word('first'), br, word('last')]);
      final index = DocumentSearchIndex(doc);
      final hits = index.find('last', const SearchOptions()).hits;
      expect(hits.single.documentStart, 7);
      expect(hits.single.documentEnd, 11);
      final changed = index.replace(doc, hits, 'final');
      expect(nodeText(changed), '\nfirst\nfinal');
      expect(DocumentBookmarkIndex(changed).exactEditorOffset(7), 7);
    },
  );

  testWidgets(
    'Shift+Enter makes a visual line within a paragraph; Enter splits the list item',
    (t) async {
      final doc = {
        'type': 'doc',
        'content': [
          {
            'type': 'bulletList',
            'content': [
              {
                'type': 'listItem',
                'content': [
                  {
                    'type': 'paragraph',
                    'content': [word('AlphaBeta')],
                  },
                ],
              },
            ],
          },
        ],
      };
      final c = controller(doc, 5), focus = FocusNode();
      configureDocumentLists(c.document);
      await t.pumpWidget(
        MaterialApp(
          localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
          home: Scaffold(
            body: Column(
              children: [
                DocumentTextToolbar(controller: c, focus: focus),
                Expanded(
                  child: q.QuillEditor.basic(
                    controller: c,
                    focusNode: focus,
                    config: q.QuillEditorConfig(
                      customStyles: documentEditorStyles(),
                      customShortcuts: documentMarkShortcuts,
                      customActions: documentMarkActions(c, focus),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      );
      focus.requestFocus();
      await t.pumpAndSettle();
      await shiftEnter(t);
      final items = decoded(c)['content'][0]['content'] as List;
      expect(items.length, 1);
      expect(items[0]['content'][0]['content'][1]['type'], 'hardBreak');
      final rendered = t
          .widgetList<q.QuillLayoutText>(find.byType(q.QuillLayoutText))
          .firstWhere((w) => w.text.toPlainText().contains('Alpha'));
      expect(rendered.text.toPlainText(), 'Alpha\u2028Beta');
      final painter = TextPainter(
        text: rendered.text,
        textDirection: TextDirection.ltr,
      )..layout(maxWidth: 600);
      expect(painter.computeLineMetrics().length, 2);
      expect(
        painter.getOffsetForCaret(const TextPosition(offset: 6), Rect.zero).dy,
        greaterThan(
          painter
              .getOffsetForCaret(const TextPosition(offset: 4), Rect.zero)
              .dy,
        ),
      );
      painter.dispose();
      // Plain Enter is delivered by the platform text-input connection.
      t.testTextInput.updateEditingValue(
        c.plainTextEditingValue.replaced(
          TextRange.collapsed(c.selection.start),
          '\n',
        ),
      );
      await t.pumpAndSettle();
      expect((decoded(c)['content'][0]['content'] as List).length, 2);
      expect(t.takeException(), isNull);
      await t.pumpWidget(const SizedBox());
      c.dispose();
      focus.dispose();
    },
  );

  testWidgets(
    'toolbar, repeated breaks, backspace and draft Undo preserve paragraph identity',
    (t) async {
      final c = controller(lineDoc([word('A')]), 1), focus = FocusNode();
      await t.pumpWidget(
        MaterialApp(
          localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
          home: Scaffold(
            body: Column(
              children: [
                DocumentTextToolbar(controller: c, focus: focus),
                Expanded(
                  child: q.QuillEditor.basic(
                    controller: c,
                    focusNode: focus,
                    config: q.QuillEditorConfig(
                      customShortcuts: documentMarkShortcuts,
                      customActions: documentMarkActions(c, focus),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      );
      await t.tap(find.byKey(const Key('document-line-break')));
      await t.pumpAndSettle();
      await shiftEnter(t);
      expect(c.document.toPlainText(), 'A\u2028\u2028\n');
      c.document.history.lastRecorded = 0;
      await t.sendKeyEvent(LogicalKeyboardKey.backspace);
      await t.pumpAndSettle();
      expect(nodeText(decoded(c)), 'A\n');
      final changed = jsonEncode(decoded(c));
      c.undo();
      await t.pumpAndSettle();
      expect(c.document.toPlainText(), 'A\u2028\u2028\n');
      c.redo();
      await t.pumpAndSettle();
      expect(jsonEncode(decoded(c)), changed);
      c.readOnly = true;
      insertDocumentLineBreak(c);
      expect(jsonEncode(decoded(c)), changed);
      expect(t.takeException(), isNull);
      await t.pumpWidget(const SizedBox());
      c.dispose();
      focus.dispose();
    },
  );
}
