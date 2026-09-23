import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_history_change.dart';
import 'package:folio_native/src/document_history_input.dart';

void main() {
  test(
    'exact repeated-text history maps later typing to the original insertion',
    () {
      for (final (body, added) in [
        ('aaaa', 'a'),
        ('😀😀', '😀'),
        ('Line\nLine', 'Line\n'),
      ]) {
        final before = Delta()..insert('$added$body\n');
        final after = Delta()..insert('$body\n');
        final change = exactDocumentHistoryChange(before, after, [
          {'start': 0, 'end': added.length, 'text': ''},
        ]);
        expect(change, isNotNull);
        expect(before.compose(change!), after);
        expect(change.transformPosition(added.length), 0);
        final input = DocumentHistoryInput(before);
        input.commands.add(
          QueuedDocumentEdit(
            Delta()
              ..retain(added.length)
              ..insert('X'),
            null,
          ),
        );
        final optimistic = input.visible;
        final visibleChange = input.rebase(after, change: change);
        expect(input.visible, Delta()..insert('X$body\n'));
        expect(optimistic.compose(visibleChange), input.visible);
      }
    },
  );
  test('same-position inverse insertions retain order and rich properties', () {
    final before = Delta()..insert('c\n');
    final after = Delta()
      ..insert('a', {'bold': true})
      ..insert('b', {'italic': true})
      ..insert('c\n');
    final change = exactDocumentHistoryChange(before, after, [
      {'start': 0, 'end': 0, 'text': 'a'},
      {'start': 0, 'end': 0, 'text': 'b'},
    ]);
    expect(change, isNotNull);
    expect(before.compose(change!), after);
    expect(change.transformPosition(0), 2);
  });
  test(
    'history uses logical positions after embedded tables without guessing inside them',
    () {
      Map paragraph(String s) => {
        'type': 'paragraph',
        'content': [
          {'type': 'text', 'text': s},
        ],
      };
      Map document(String text, String cell) => {
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
                    'content': [paragraph(cell)],
                  },
                ],
              },
            ],
          },
          paragraph(text),
        ],
      };
      final before = documentToDelta(document('aaaaa', 'Cell text'));
      final after = documentToDelta(document('aaaa', 'Cell text'));
      final change = exactDocumentHistoryChange(before, after, [
        {'start': 10, 'end': 11, 'text': ''},
      ]);
      expect(change, isNotNull);
      expect(before.compose(change!), after);
      expect(change.transformPosition(3), 2);
      // An internal table edit is one object replacement in the main editor.
      expect(
        exactDocumentHistoryChange(
          before,
          documentToDelta(document('aaaaa', 'CXll text')),
          [
            {'start': 1, 'end': 2, 'text': 'X'},
          ],
        ),
        isNull,
      );
    },
  );
  test(
    'unavailable or mismatched history journals cannot relocate selections',
    () {
      final before = Delta()..insert('😀abc\n');
      final after = Delta()..insert('Xabc\n');
      for (final journal in [
        null,
        'invalid',
        [],
        [
          {'start': 1, 'end': 2, 'text': 'X'},
        ],
        [
          {'start': 0, 'end': 99, 'text': 'X'},
        ],
        [
          {'start': 0, 'end': 0, 'text': 'wrong'},
        ],
        [
          {'start': '0', 'end': 2, 'text': 'X'},
        ],
      ]) {
        expect(exactDocumentHistoryChange(before, after, journal), isNull);
      }
      expect(
        before.compose(
          exactDocumentHistoryChange(before, after, [
            {'start': 0, 'end': 2, 'text': 'X'},
          ])!,
        ),
        after,
      );
    },
  );
}
