import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'document_bookmarks.dart';
import 'document_codec.dart';

typedef LogicalTextEdits = List<Map<String, dynamic>>;

/// Preserve the locations encoded by operations, including formatted paste,
/// instead of diffing repeated text after the edit has already happened.
LogicalTextEdits? bookmarkDeltaEdits(Delta before, Delta change) {
  final old = deltaToDocument(before);
  final next = deltaToDocument(before.compose(change));
  final a = DocumentBookmarkIndex(old), b = DocumentBookmarkIndex(next);
  final edits = _replacements(change, a.logicalOffset, b.logicalOffset, b.text);
  return checkedTextEdits(old, next, edits);
}

/// Preserve exact logical journals (including edits inside table embeds) while
/// earlier workspace history changes the surrounding document.
LogicalTextEdits? rebaseDocumentTextEdits(
  Delta before,
  Delta remote,
  LogicalTextEdits? edits,
) {
  if (edits == null) return null;
  final remoteEdits = bookmarkDeltaEdits(before, remote);
  if (remoteEdits == null) return null;
  Delta operations(LogicalTextEdits values) {
    final result = Delta();
    var position = 0;
    for (final edit in values) {
      result.retain(edit['start'] - position);
      result.insert(edit['text']);
      result.delete(edit['end'] - edit['start']);
      position = edit['end'];
    }
    return result;
  }

  final transformed = operations(
    remoteEdits,
  ).transform(operations(edits), true);
  final text = DocumentBookmarkIndex(
    deltaToDocument(before.compose(remote)),
  ).text;
  final after = (Delta()..insert(text)).compose(transformed);
  final nextText = after.toList().map((op) => op.data as String).join();
  return _replacements(transformed, (i) => i, (i) => i, nextText);
}

LogicalTextEdits? _replacements(
  Delta change,
  int? Function(int) oldOffset,
  int? Function(int) newOffset,
  String text,
) {
  final edits = <Map<String, dynamic>>[];
  var old = 0, next = 0;
  int? oldStart, newStart;
  bool flush() {
    if (oldStart == null) return true;
    final start = oldOffset(oldStart!), end = oldOffset(old);
    final a = newOffset(newStart!), b = newOffset(next);
    if (start == null ||
        end == null ||
        a == null ||
        b == null ||
        a < 0 ||
        b < a ||
        b > text.length) {
      return false;
    }
    edits.add({'start': start, 'end': end, 'text': text.substring(a, b)});
    oldStart = newStart = null;
    return true;
  }

  for (final op in change.toList()) {
    if (op.isRetain) {
      if (!flush()) return null;
      old += op.length!;
      next += op.length!;
    } else {
      oldStart ??= old;
      newStart ??= next;
      if (op.isDelete) {
        old += op.length!;
      } else {
        next += op.length!;
      }
    }
  }
  return flush() ? edits : null;
}

// Rust serializes object keys in a different order from the editor codec.
// Key order is not content identity; array order and values are significant.
bool _sameJson(Object? a, Object? b) {
  if (a is Map && b is Map) {
    return a.length == b.length &&
        a.entries.every(
          (e) => b.containsKey(e.key) && _sameJson(e.value, b[e.key]),
        );
  }
  if (a is List && b is List) {
    return a.length == b.length &&
        a.indexed.every((e) => _sameJson(e.$2, b[e.$1]));
  }
  return a == b;
}

bool sameDocumentContent(Map a, Map b) => _sameJson(a['content'], b['content']);

/// One controller notification can include insertion, deletion and formatting
/// events. Capture them synchronously, compose their operations, then consume
/// once with the authoritative before/after documents at that notification.
class DocumentEditCapture {
  static final _captures = Expando<DocumentEditCapture>();
  Delta? _before, _change, _last;
  bool _valid = true;
  ({Map before, LogicalTextEdits edits})? _override;

  factory DocumentEditCapture.attach(
    q.Document document, {
    bool Function()? enabled,
  }) => _captures[document] ??= DocumentEditCapture._(document, enabled);

  DocumentEditCapture._(q.Document document, bool Function()? enabled) {
    document.synchronousChanges.listen((event) {
      if (enabled != null && !enabled()) {
        _before = _change = _last = null;
        _override = null;
        _valid = true;
        return;
      }
      if (_last != null && _last != event.before) _valid = false;
      _before ??= event.before;
      _change = (_change ?? Delta()).compose(event.change);
      _last = event.before.compose(event.change);
    });
  }

  bool get hasChanges => _before != null;

  LogicalTextEdits? take(Map before, Map after) {
    final first = _before, change = _change, last = _last;
    final valid = _valid, override = _override;
    _before = _change = _last = null;
    _valid = true;
    _override = null;
    if (!valid ||
        first == null ||
        change == null ||
        last == null ||
        !sameDocumentContent(before, deltaToDocument(first)) ||
        !sameDocumentContent(after, deltaToDocument(last))) {
      return null;
    }
    if (override != null) {
      return sameDocumentContent(before, override.before)
          ? checkedTextEdits(before, after, override.edits)
          : null;
    }
    return bookmarkDeltaEdits(first, change);
  }

  /// A table is a single Quill embed. Supply its verified internal operations
  /// so replacing that embed does not delete every bookmark inside the table.
  static void apply(
    q.QuillController controller,
    LogicalTextEdits? edits,
    void Function() mutation,
  ) {
    final capture = _captures[controller.document];
    if (capture != null) {
      if (edits != null && capture._before == null) {
        capture._override = (
          before: deltaToDocument(controller.document.toDelta()),
          edits: edits,
        );
      } else {
        // An internal table change without an exact journal is not equivalent
        // to deleting the whole table and pasting a new one.
        capture._valid = false;
      }
    }
    try {
      mutation();
    } finally {
      capture?._override = null;
    }
  }
}

/// A table draft can change several cells before Apply. Compose the actual
/// logical operations, retaining their original locations even in equal text.
class DocumentEditDraft {
  final Map before;
  late String _text = DocumentBookmarkIndex(before).text;
  Delta _change = Delta();
  bool _valid = true;
  DocumentEditDraft(Map before) : before = jsonDecode(jsonEncode(before));

  void invalidate() => _valid = false;

  void record(Map old, Map next, LogicalTextEdits? edits) {
    if (!_valid) return;
    final checked = checkedTextEdits(old, next, edits);
    if (checked == null || DocumentBookmarkIndex(old).text != _text) {
      invalidate();
      return;
    }
    final step = Delta();
    var position = 0;
    for (final edit in checked) {
      step.retain(edit['start'] - position);
      step.insert(edit['text']);
      step.delete(edit['end'] - edit['start']);
      position = edit['end'];
    }
    _change = _change.compose(step);
    _text = DocumentBookmarkIndex(next).text;
  }

  LogicalTextEdits? finish(Map after) {
    if (!_valid || DocumentBookmarkIndex(after).text != _text) return null;
    return checkedTextEdits(
      before,
      after,
      _replacements(_change, (i) => i, (i) => i, _text),
    );
  }
}
