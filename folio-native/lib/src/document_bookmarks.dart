import 'dart:convert';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'document_codec.dart';

class BookmarksIntent extends Intent {
  const BookmarksIntent();
}

const bookmarkShortcuts = <ShortcutActivator, Intent>{
  SingleActivator(LogicalKeyboardKey.keyG, control: true): BookmarksIntent(),
  SingleActivator(LogicalKeyboardKey.f5): BookmarksIntent(),
  SingleActivator(LogicalKeyboardKey.f5, control: true, shift: true):
      BookmarksIntent(),
};

class BookmarkParagraph {
  final List<int> path;
  final int start, end, editorOffset;
  final bool embedded;
  final String text, label;
  const BookmarkParagraph(
    this.path,
    this.start,
    this.end,
    this.editorOffset,
    this.embedded,
    this.text,
    this.label,
  );
}

class _Span {
  final int start, end, editorStart, editorEnd;
  final bool embedded;
  const _Span(
    this.start,
    this.end,
    this.editorStart,
    this.editorEnd,
    this.embedded,
  );
}

/// Matches the Rust body-story index, including table paragraphs. Quill embeds
/// have different lengths; a selection inside one must not guess text offsets.
class DocumentBookmarkIndex {
  final paragraphs = <BookmarkParagraph>[];
  final _spans = <_Span>[];
  final _nodeRanges = <String, TextRange>{};
  late final String text;
  DocumentBookmarkIndex(Map document) {
    final buffer = StringBuffer();
    var logical = 0, editor = 0;
    for (final (top, raw) in (document['content'] as List? ?? []).indexed) {
      final node = raw as Map;
      final delta = documentToDelta({
        'type': 'doc',
        'content': [node],
      });
      final embedded =
          delta.first.data is Map &&
          (delta.first.data as Map).containsKey('folioNode');
      final begin = logical;
      void visit(Map n, List<int> path, String? label) {
        final start = logical;
        if (['paragraph', 'heading'].contains(n['type'])) {
          final value = _nodeText(n);
          paragraphs.add(
            BookmarkParagraph(
              List.unmodifiable(path),
              logical,
              logical + value.length,
              editor + (embedded ? 0 : logical - begin),
              embedded,
              value,
              label ?? 'Paragraph ${paragraphs.length + 1}',
            ),
          );
          buffer.write('$value\n');
          logical += value.length + 1;
        } else if ([
          'doc',
          'table',
          'tableRow',
          'tableCell',
          'tableHeader',
          'bulletList',
          'orderedList',
          'listItem',
          'blockquote',
        ].contains(n['type'])) {
          for (final (i, child) in (n['content'] as List? ?? []).indexed) {
            visit(
              child,
              [...path, i],
              n['type'] == 'table'
                  ? 'Table · row ${i + 1}'
                  : n['type'] == 'tableRow'
                  ? '$label · cell ${i + 1}'
                  : label,
            );
          }
        } else {
          buffer.write('\uFFFC\n');
          logical += 2;
        }
        _nodeRanges[path.join('/')] = TextRange(start: start, end: logical);
      }

      visit(node, [top], null);
      final editorEnd =
          editor + delta.toList().fold<int>(0, (sum, op) => sum + op.length!);
      if (embedded) {
        _spans.add(_Span(begin, logical, editor, editorEnd, true));
      } else {
        var logicalPart = begin, editorPart = editor;
        for (final op in delta.toList()) {
          final insert = op.data;
          final isField = insert is Map && insert['folioPageField'] is String;
          final width = isField
              ? (jsonDecode(insert['folioPageField'])['text'] as String).length
              : op.length!;
          _spans.add(
            _Span(
              logicalPart,
              logicalPart + width,
              editorPart,
              editorPart + op.length!,
              isField,
            ),
          );
          logicalPart += width;
          editorPart += op.length!;
        }
      }
      editor = editorEnd;
    }
    text = buffer.toString();
    for (var i = 0; i < paragraphs.length; i++) {
      final p = paragraphs[i];
      final offset = exactEditorOffset(p.start);
      if (!p.embedded && offset != null) {
        paragraphs[i] = BookmarkParagraph(
          p.path,
          p.start,
          p.end,
          offset,
          false,
          p.text,
          p.label,
        );
      }
    }
  }

  static String _nodeText(Map n) {
    if (n['text'] is String) return n['text'];
    if (n['type'] == 'hardBreak') return '\n';
    return (n['content'] as List? ?? [])
        .map((n) => _nodeText(n as Map))
        .join(
          [
                'doc',
                'bulletList',
                'orderedList',
                'table',
                'tableRow',
              ].contains(n['type'])
              ? '\n'
              : '',
        );
  }

  BookmarkParagraph? paragraph(int position) => paragraphs
      .where((p) => p.start <= position && position <= p.end)
      .firstOrNull;
  BookmarkParagraph? atPath(List<int> path) =>
      paragraphs.where((p) => _samePath(p.path, path)).firstOrNull;
  TextRange? rangeAtPath(List<int> path) => _nodeRanges[path.join('/')];
  static bool _samePath(List<int> a, List<int> b) =>
      a.length == b.length && a.indexed.every((e) => e.$2 == b[e.$1]);
  bool boundary(int p) =>
      p >= 0 &&
      p <= text.length &&
      !(p > 0 &&
          p < text.length &&
          text.codeUnitAt(p) >= 0xdc00 &&
          text.codeUnitAt(p) <= 0xdfff &&
          text.codeUnitAt(p - 1) >= 0xd800 &&
          text.codeUnitAt(p - 1) <= 0xdbff);

  int? logicalOffset(int position) {
    for (final s in _spans) {
      if (position < s.editorStart || position > s.editorEnd) continue;
      if (!s.embedded) return s.start + position - s.editorStart;
      if (position == s.editorStart) return s.start;
      if (position == s.editorEnd) return s.end;
      if (s.editorEnd - s.editorStart > 1 && position == s.editorStart + 1) {
        return s.end - 1;
      }
    }
    return null;
  }

  /// Inverse projection only at genuine editor boundaries. A position inside
  /// table text cannot be represented by the main editor's one-character embed.
  int? exactEditorOffset(int position) {
    if (!boundary(position)) return null;
    for (final span in _spans) {
      if (position < span.start || position > span.end) continue;
      if (!span.embedded) return span.editorStart + position - span.start;
      if (position == span.start) return span.editorStart;
      if (position == span.end) return span.editorEnd;
      if (span.editorEnd - span.editorStart > 1 && position == span.end - 1) {
        return span.editorStart + 1;
      }
    }
    return null;
  }

  TextSelection? capture(TextSelection selection) {
    if (!selection.isValid) return null;
    final start = logicalOffset(selection.start),
        end = logicalOffset(selection.end);
    if (start == null ||
        end == null ||
        !boundary(start) ||
        !boundary(end) ||
        paragraph(start) == null ||
        paragraph(end) == null) {
      return null;
    }
    return TextSelection(baseOffset: start, extentOffset: end);
  }

  TextSelection? editorSelection(int start, int end) {
    final a = paragraph(start), b = paragraph(end);
    if (a == null || b == null) return null;
    return TextSelection(
      baseOffset: a.editorOffset + (a.embedded ? 0 : start - a.start),
      extentOffset: b.editorOffset + (b.embedded ? 1 : end - b.start),
    );
  }

  String excerpt(int start, int end) {
    if (!boundary(start) || !boundary(end) || start > end) return '';
    if (start == end) {
      final p = paragraph(start);
      return p == null || p.text.isEmpty ? 'Insertion point' : p.text;
    }
    final selected = text.substring(start, end);
    return String.fromCharCodes(selected.runes.take(220)) +
        (selected.runes.length > 220 ? '…' : '');
  }
}

List<Map<String, dynamic>> documentBookmarks(Map document) =>
    (document['attrs']?['bookmarks'] as List? ?? [])
        .map((b) => Map<String, dynamic>.from(b))
        .toList();

/// Convert known editor operations; use checkedTextEdits before submitting.
List<Map<String, dynamic>>? bookmarkEditorEdits(Map before, List<Map> edits) {
  final index = DocumentBookmarkIndex(before);
  final output = <Map<String, dynamic>>[];
  for (final e in edits) {
    final start = index.logicalOffset(e['start']),
        end = index.logicalOffset(e['end']);
    if (start == null || end == null) return null;
    output.add({'start': start, 'end': end, 'text': e['text']});
  }
  return output;
}

List<Map<String, dynamic>>? checkedTextEdits(
  Map before,
  Map after,
  List<Map<String, dynamic>>? edits,
) {
  if (edits == null) return null;
  final old = DocumentBookmarkIndex(before),
      next = DocumentBookmarkIndex(after);
  final indexed = edits.indexed.toList()
    ..sort((a, b) {
      final order = (a.$2['start'] as int).compareTo(b.$2['start']);
      return order != 0 ? order : a.$1.compareTo(b.$1);
    });
  final ordered = indexed.map((entry) => entry.$2);
  final result = StringBuffer();
  var cursor = 0;
  final changed = <Map<String, dynamic>>[];
  for (final e in ordered) {
    final start = e['start'] as int,
        end = e['end'] as int,
        value = e['text'] as String;
    if (start < cursor ||
        end < start ||
        !old.boundary(start) ||
        !old.boundary(end)) {
      return null;
    }
    result.write(old.text.substring(cursor, start));
    result.write(value);
    cursor = end;
    if (old.text.substring(start, end) != value) changed.add(e);
  }
  result.write(old.text.substring(cursor));
  return result.toString() == next.text ? changed : null;
}
