import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_bookmarks.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_edit_tracking.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;

Map<String, dynamic> paragraph(String text) => {
  'type': 'paragraph',
  'content': [
    {'type': 'text', 'text': text},
  ],
};
Map<String, dynamic> doc(String text) => {
  'type': 'doc',
  'content': [paragraph(text)],
};

void main() {
  test(
    'logical bookmark positions cover Unicode, lists, table cells and opaque blocks',
    () {
      final d = {
        'type': 'doc',
        'content': [
          paragraph('A😀'),
          {
            'type': 'table',
            'content': [
              {
                'type': 'tableRow',
                'content': [
                  {
                    'type': 'tableCell',
                    'content': [paragraph('Cell'), paragraph('Second')],
                  },
                  {
                    'type': 'tableCell',
                    'content': [paragraph('Value')],
                  },
                ],
              },
            ],
          },
          {
            'type': 'officeBlock',
            'content': [paragraph('Preserved')],
          },
          {
            'type': 'bulletList',
            'content': [
              {
                'type': 'listItem',
                'content': [paragraph('Item')],
              },
            ],
          },
          paragraph('End'),
        ],
      };
      final index = DocumentBookmarkIndex(d);
      expect(index.text, 'A😀\nCell\nSecond\nValue\n\uFFFC\nItem\nEnd\n');
      expect(index.paragraphs.map((p) => p.start), [0, 4, 9, 16, 24, 29]);
      expect(
        index.capture(const TextSelection(baseOffset: 1, extentOffset: 2)),
        isNull,
      );
      expect(
        index.capture(const TextSelection(baseOffset: 1, extentOffset: 3))!.end,
        3,
      );
      expect(
        index.capture(const TextSelection(baseOffset: 4, extentOffset: 5))!.end,
        21,
      );
      expect(index.logicalOffset(8), 24);
      expect(
        index.editorSelection(9, 15),
        const TextSelection(baseOffset: 4, extentOffset: 5),
      );
      expect(
        index.editorSelection(29, 32),
        const TextSelection(baseOffset: 13, extentOffset: 16),
      );
      expect(index.atPath([1, 0, 0, 1])!.text, 'Second');
    },
  );
  test(
    'exact edits distinguish repeated text and translate positions after table embeds',
    () {
      final before = {
        'type': 'doc',
        'content': [
          {
            'type': 'table',
            'content': [
              {
                'type': 'tableRow',
                'content': [
                  {
                    'type': 'tableCell',
                    'content': [paragraph('Long table content')],
                  },
                ],
              },
            ],
          },
          paragraph('aaaaaa'),
        ],
      };
      final editor = q.QuillController(
        document: q.Document.fromDelta(documentToDelta(before)),
        selection: const TextSelection.collapsed(offset: 0),
      );
      final capture = DocumentEditCapture.attach(editor.document);
      editor.replaceText(4, 0, 'a', const TextSelection.collapsed(offset: 5));
      final edits = capture.take(
        before,
        deltaToDocument(editor.document.toDelta()),
      );
      expect(edits, [
        {'start': 21, 'end': 21, 'text': 'a'},
      ]);
      expect(checkedTextEdits(doc('Other'), doc('Othera'), edits), isNull);
      editor.dispose();
    },
  );
  test(
    'exact operations preserve separate replacements and reject overlapping or stale edits',
    () {
      final before = doc('aa bb aa');
      final edits = [
        {'start': 0, 'end': 2, 'text': 'X'},
        {'start': 6, 'end': 8, 'text': 'Y'},
      ];
      expect(checkedTextEdits(before, doc('X bb Y'), edits), edits);
      expect(checkedTextEdits(before, doc('X bb Z'), edits), isNull);
      expect(
        checkedTextEdits(before, doc('X bb Y'), [...edits, edits.first]),
        isNull,
      );
      expect(
        checkedTextEdits(doc('😀'), doc('X'), [
          {'start': 1, 'end': 2, 'text': 'X'},
        ]),
        isNull,
      );
      expect(
        checkedTextEdits(before, before, [
          {'start': 0, 'end': 2, 'text': 'aa'},
        ]),
        isEmpty,
      );
    },
  );
  test(
    'paragraph breaks, whole embed deletion and point excerpts use the same text model',
    () {
      final before = {
        'type': 'doc',
        'content': [paragraph('hello world'), paragraph('End')],
      };
      final after = {
        'type': 'doc',
        'content': [paragraph('hello'), paragraph('world'), paragraph('End')],
      };
      expect(
        checkedTextEdits(
          before,
          after,
          bookmarkEditorEdits(before, [
            {'start': 5, 'end': 6, 'text': '\n'},
          ]),
        ),
        [
          {'start': 5, 'end': 6, 'text': '\n'},
        ],
      );
      final withTable = {
        'type': 'doc',
        'content': [
          {
            'type': 'table',
            'content': [
              {
                'type': 'tableRow',
                'content': [
                  {
                    'type': 'tableCell',
                    'content': [paragraph('Cell')],
                  },
                ],
              },
            ],
          },
          paragraph('End'),
        ],
      };
      expect(
        checkedTextEdits(
          withTable,
          doc('End'),
          bookmarkEditorEdits(withTable, [
            {'start': 0, 'end': 2, 'text': ''},
          ]),
        ),
        [
          {'start': 0, 'end': 5, 'text': ''},
        ],
      );
      final index = DocumentBookmarkIndex(before);
      expect(index.excerpt(0, 0), 'hello world');
      expect(index.excerpt(6, 11), 'world');
    },
  );
}
