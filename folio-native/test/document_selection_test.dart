import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_selection.dart';

q.Document text(String value) => q.Document.fromJson([
  {'insert': value},
]);

void main() {
  test('formatting and object property changes retain directed selection', () {
    q.Document document(String image, {bool bold = false}) =>
        q.Document.fromJson([
          {
            'insert': {'image': image},
          },
          {'insert': '\n'},
          {
            'insert': 'Body text',
            'attributes': {'bold': bold},
          },
          {'insert': '\n'},
        ]);
    const selection = TextSelection(
      baseOffset: 9,
      extentOffset: 3,
      affinity: TextAffinity.upstream,
      isDirectional: true,
    );
    expect(
      selectionAfterDocumentRefresh(
        document('before.png'),
        document('after.png', bold: true),
        selection,
      ),
      selection,
    );
  });

  test('text insertions and deletion transform both endpoints', () {
    final before = text('Heading\nBody text\n');
    final after = text('New Heading\nBody text\n');
    const selection = TextSelection(baseOffset: 17, extentOffset: 8);
    final shifted = selectionAfterDocumentRefresh(before, after, selection);
    expect(shifted, const TextSelection(baseOffset: 21, extentOffset: 12));
    expect(selectionAfterDocumentRefresh(after, before, shifted), selection);
    expect(
      selectionAfterDocumentRefresh(
        text('abcdef\n'),
        text('af\n'),
        const TextSelection(baseOffset: 5, extentOffset: 2),
      ),
      const TextSelection.collapsed(offset: 1),
    );
  });

  test('undo and redo of insertion keep the caret beside the edit', () {
    final before = text('First\nBody\n');
    final after = text('First\n✅ Body\n');
    const undoCaret = TextSelection.collapsed(offset: 6);
    const redoCaret = TextSelection.collapsed(offset: 8);
    expect(selectionAfterDocumentRefresh(before, after, undoCaret), redoCaret);
    expect(selectionAfterDocumentRefresh(after, before, redoCaret), undoCaret);
  });

  test('invalid and stale positions stay within valid UTF-16 bounds', () {
    final document = text('A😀B\n');
    expect(
      selectionAfterDocumentRefresh(
        document,
        document,
        const TextSelection.collapsed(offset: -1),
      ),
      const TextSelection.collapsed(offset: 0),
    );
    expect(
      selectionAfterDocumentRefresh(
        document,
        document,
        const TextSelection(baseOffset: 2, extentOffset: 90),
      ),
      const TextSelection(baseOffset: 1, extentOffset: 4),
    );
    expect(
      selectionAfterDocumentRefresh(
        document,
        text('\n'),
        const TextSelection.collapsed(offset: 4),
      ),
      const TextSelection.collapsed(offset: 0),
    );
  });
}
