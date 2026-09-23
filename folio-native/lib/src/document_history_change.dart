import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'document_bookmarks.dart';
import 'document_codec.dart';

/// Reconstruct a history change using authoritative logical edit locations.
/// Slice inserted rich content from the committed document, then reconcile only
/// the remaining properties. Repeated text therefore cannot relocate an edit.
Delta? exactDocumentHistoryChange(Delta before, Delta after, Object? journal) {
  if (journal is! List) return null;
  final edits = <Map<String, dynamic>>[];
  for (final value in journal) {
    if (value is! Map ||
        value['start'] is! int ||
        value['end'] is! int ||
        value['text'] is! String) {
      return null;
    }
    edits.add(Map<String, dynamic>.from(value));
  }
  final oldDocument = deltaToDocument(before),
      newDocument = deltaToDocument(after);
  final checked = checkedTextEdits(oldDocument, newDocument, edits);
  if (checked == null) return null;
  final old = DocumentBookmarkIndex(oldDocument),
      next = DocumentBookmarkIndex(newDocument);
  final change = Delta();
  var cursor = 0, shift = 0;
  for (final edit in checked) {
    final a = old.exactEditorOffset(edit['start']),
        b = old.exactEditorOffset(edit['end']);
    final count = (edit['text'] as String).length;
    final x = next.exactEditorOffset(edit['start'] + shift);
    final y = next.exactEditorOffset(edit['start'] + shift + count);
    // Text inside an opaque table/embed needs its own editing context; a main
    // editor caret must not invent a position inside that object.
    if (a == null ||
        b == null ||
        x == null ||
        y == null ||
        a < cursor ||
        b < a ||
        y < x) {
      return null;
    }
    change.retain(a - cursor);
    for (final operation in after.slice(x, y).toList()) {
      change.push(operation);
    }
    change.delete(b - a);
    cursor = b;
    shift += count - ((edit['end'] as int) - (edit['start'] as int));
  }
  final intermediate = before.compose(change);
  String text(Delta delta) =>
      delta.toList().map((op) => op.data is String ? op.data : '\uFFFC').join();
  if (text(intermediate) != text(after)) return null;
  return change.compose(intermediate.diff(after));
}
