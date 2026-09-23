import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_bookmarks.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_edit_tracking.dart';

Map paragraph(String text) => {
  'type': 'paragraph',
  'content': [
    {'type': 'text', 'text': text},
  ],
};
Map doc(String text) => {
  'type': 'doc',
  'content': [paragraph(text)],
};
Map table(String text) => {
  'type': 'table',
  'content': [
    {
      'type': 'tableRow',
      'content': [
        {
          'type': 'tableCell',
          'content': [paragraph(text)],
        },
      ],
    },
  ],
};

void main() {
  test(
    'engine key ordering does not invalidate cell edits; stale content still does',
    () {
      final original = {
        'content': [
          {
            'content': [
              {'text': 'aaaaaa', 'type': 'text'},
            ],
            'type': 'paragraph',
          },
        ],
        'type': 'doc',
      };
      final controller = q.QuillController(
        document: q.Document.fromDelta(documentToDelta(original)),
        selection: const TextSelection.collapsed(offset: 0),
      );
      final capture = DocumentEditCapture.attach(controller.document);
      controller.replaceText(
        0,
        0,
        'a',
        const TextSelection.collapsed(offset: 1),
      );
      expect(
        capture.take(original, deltaToDocument(controller.document.toDelta())),
        [
          {'start': 0, 'end': 0, 'text': 'a'},
        ],
      );
      controller.replaceText(
        0,
        0,
        'a',
        const TextSelection.collapsed(offset: 1),
      );
      expect(
        capture.take(
          doc('stale'),
          deltaToDocument(controller.document.toDelta()),
        ),
        isNull,
      );
      controller.dispose();
    },
  );
  test(
    'formatted replacements and multiple compose operations retain exact locations',
    () {
      final controller = q.QuillController(
        document: q.Document.fromDelta(documentToDelta(doc('aaaaaa'))),
        selection: const TextSelection.collapsed(offset: 0),
      );
      final capture = DocumentEditCapture.attach(controller.document);
      LogicalTextEdits? observed;
      Map previous = deltaToDocument(controller.document.toDelta());
      controller.addListener(() {
        final after = deltaToDocument(controller.document.toDelta());
        observed = capture.take(previous, after);
        previous = after;
      });
      controller.replaceText(
        0,
        0,
        Delta()..insert('a', {'bold': true}),
        const TextSelection.collapsed(offset: 1),
      );
      expect(observed, [
        {'start': 0, 'end': 0, 'text': 'a'},
      ]);
      controller.compose(
        Delta()
          ..retain(2)
          ..insert('X', {'italic': true})
          ..delete(2)
          ..retain(2)
          ..insert('Y'),
        const TextSelection.collapsed(offset: 3),
        q.ChangeSource.local,
      );
      expect(observed, [
        {'start': 2, 'end': 4, 'text': 'X'},
        {'start': 6, 'end': 6, 'text': 'Y'},
      ]);
      expect(controller.document.toPlainText(), 'aaXaaYa\n');
      controller.formatText(0, 2, q.Attribute.underline);
      expect(observed, isEmpty);
      // Two edits, then the single controller notification used by batch clients.
      // ignore: experimental_member_use
      controller.replaceText(0, 0, 'a', null, shouldNotifyListeners: false);
      // ignore: experimental_member_use
      controller.replaceText(5, 0, 'b', null, shouldNotifyListeners: false);
      controller.updateSelection(
        const TextSelection.collapsed(offset: 6),
        q.ChangeSource.local,
      );
      expect(observed, [
        {'start': 0, 'end': 0, 'text': 'a'},
        {'start': 4, 'end': 4, 'text': 'b'},
      ]);
      controller.dispose();
    },
  );

  test(
    'rich paste maps UTF-16 offsets after embedded tables and paragraph breaks',
    () {
      final body = {
        'type': 'doc',
        'content': [table('Long cell'), paragraph('😀aaaa')],
      };
      final before = documentToDelta(body);
      final change = Delta()
        ..retain(4)
        ..insert('a', {'bold': true})
        ..retain(2)
        ..insert('\n')
        ..retain(1)
        ..delete(1);
      expect(bookmarkDeltaEdits(before, change), [
        {'start': 12, 'end': 12, 'text': 'a'},
        {'start': 14, 'end': 14, 'text': '\n'},
        {'start': 15, 'end': 16, 'text': ''},
      ]);
      expect(
        DocumentBookmarkIndex(deltaToDocument(before.compose(change))).text,
        'Long cell\n😀aaa\na\n',
      );
      expect(
        bookmarkDeltaEdits(
          before,
          Delta()
            ..retain(3)
            ..delete(1),
        ),
        isNull,
        reason: 'Never submit an operation which splits a surrogate pair',
      );
    },
  );

  test(
    'table drafts compose repeated-text edits across cells and reject missing steps',
    () {
      final original = doc('aaaaaa\nbbbbbb');
      final draft = DocumentEditDraft(original);
      var current = original;
      void edit(int start, int end, String value) {
        final text = DocumentBookmarkIndex(current).text;
        final next = doc(
          text
              .replaceRange(start, end, value)
              .substring(0, text.length - (end - start) + value.length - 1),
        );
        draft.record(current, next, [
          {'start': start, 'end': end, 'text': value},
        ]);
        current = next;
      }

      edit(0, 0, 'a');
      edit(3, 3, 'X');
      edit(2, 3, '');
      edit(10, 10, 'b');
      expect(draft.finish(current), [
        {'start': 0, 'end': 0, 'text': 'a'},
        {'start': 1, 'end': 2, 'text': 'X'},
        {'start': 9, 'end': 9, 'text': 'b'},
      ]);
      expect(draft.finish(doc('stale')), isNull);
      draft.invalidate();
      expect(draft.finish(current), isNull);
    },
  );

  test(
    'embedded table updates use internal operations and fall back for untracked structure',
    () {
      final original = {
        'type': 'doc',
        'content': [paragraph('Intro'), table('aaaaaa')],
      };
      final controller = q.QuillController(
        document: q.Document.fromDelta(documentToDelta(original)),
        selection: const TextSelection.collapsed(offset: 0),
      );
      final capture = DocumentEditCapture.attach(controller.document);
      Map previous = deltaToDocument(controller.document.toDelta());
      LogicalTextEdits? observed;
      controller.addListener(() {
        final after = deltaToDocument(controller.document.toDelta());
        observed = capture.take(previous, after);
        previous = after;
      });
      void replace(String text, LogicalTextEdits? edits) =>
          DocumentEditCapture.apply(
            controller,
            edits,
            () => controller.replaceText(
              6,
              1,
              q.BlockEmbed('folioNode', jsonEncode(table(text))),
              const TextSelection.collapsed(offset: 7),
            ),
          );
      replace('aaaaaaa', [
        {'start': 6, 'end': 6, 'text': 'a'},
      ]);
      expect(observed, [
        {'start': 6, 'end': 6, 'text': 'a'},
      ]);
      replace('structure replacement', null);
      expect(observed, isNull);
      replace('bad override', [
        {'start': 0, 'end': 0, 'text': 'X'},
      ]);
      expect(
        observed,
        isNull,
        reason: 'Must reconstruct the actual resulting text',
      );
      controller.dispose();
    },
  );
}
