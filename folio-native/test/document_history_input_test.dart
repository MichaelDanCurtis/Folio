import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_history_input.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_bookmarks.dart';
import 'package:folio_native/src/document_edit_tracking.dart';

void main() {
  test('later explicit formatting wins over history without moving text', () {
    final before = Delta()
      ..insert('Body', {'bold': true})
      ..insert('\n');
    final history = Delta()..retain(4, {'bold': null});
    final local = Delta()..retain(4, {'bold': false, 'italic': true});
    final input = DocumentHistoryInput(before);
    input.commands.add(QueuedDocumentEdit(local, []));
    final optimistic = input.visible;
    final change = input.rebase(before.compose(history));
    expect(input.visible.toJson(), [
      {
        'insert': 'Body',
        'attributes': {'bold': false, 'italic': true},
      },
      {'insert': '\n'},
    ]);
    expect(optimistic.compose(change), input.visible);
  });

  test('queued rich replacements retain exact repeated-text journals', () {
    final before = Delta()..insert('aaaaaa\nAfter\n');
    final input = DocumentHistoryInput(before);
    final replacement = Delta()
      ..retain(2)
      ..insert('🚀', {'italic': true})
      ..delete(2);
    input.commands.add(
      QueuedDocumentEdit(replacement, bookmarkDeltaEdits(before, replacement)),
    );
    final second = Delta()
      ..retain(4)
      ..insert('a');
    input.commands.add(
      QueuedDocumentEdit(
        second,
        bookmarkDeltaEdits(before.compose(replacement), second),
      ),
    );
    final optimistic = input.visible;
    final change = input.rebase((Delta()..insert('aaa\nAfter\n')));
    expect(optimistic.compose(change), input.visible);
    var base = input.base;
    for (final command in input.commands.cast<QueuedDocumentEdit>()) {
      final next = base.compose(command.change);
      expect(
        checkedTextEdits(
          deltaToDocument(base),
          deltaToDocument(next),
          command.textEdits,
        ),
        isNotNull,
      );
      base = next;
    }
    expect(
      input.visible.toJson().any((op) => op['attributes']?['italic'] == true),
      isTrue,
    );
  });

  test(
    'table internal journal moves through preceding history text changes',
    () {
      Map paragraph(String text) => {
        'type': 'paragraph',
        'content': [
          {'type': 'text', 'text': text},
        ],
      };
      Map document(String heading, String cell) => {
        'type': 'doc',
        'content': [
          paragraph(heading),
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
          paragraph('After'),
        ],
      };
      final before = documentToDelta(document('Head old', 'aaaaaa'));
      final next = documentToDelta(document('Head old', 'aaXaaaa'));
      final edit = QueuedDocumentEdit(before.diff(next), [
        {'start': 11, 'end': 11, 'text': 'X'},
      ]);
      final input = DocumentHistoryInput(before)..commands.add(edit);
      final optimistic = input.visible;
      final change = input.rebase(documentToDelta(document('Head', 'aaaaaa')));
      expect(optimistic.compose(change), input.visible);
      expect(edit.textEdits, [
        {'start': 7, 'end': 7, 'text': 'X'},
      ]);
      expect(
        checkedTextEdits(
          deltaToDocument(input.base),
          deltaToDocument(input.visible),
          edit.textEdits,
        ),
        isNotNull,
      );
    },
  );
}
