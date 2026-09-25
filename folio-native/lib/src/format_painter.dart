// Format Painter for documents (D03.10): copy the character and paragraph
// formatting at the selection, and paint it onto the next selection.
import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/widgets.dart';
import 'document_case.dart' show clearedCharacterAttributes;
import 'document_lists.dart';

class CancelFormatPainter extends Intent {
  const CancelFormatPainter();
}

/// Formatting captured by the painter.
class PaintedFormat {
  /// Character formatting; a null value clears that attribute.
  final Map<String, dynamic> inline;

  /// Paragraph formatting: alignment, heading level, and the paragraph's
  /// Word properties and tab stops.
  final Map<String, dynamic> block;
  const PaintedFormat(this.inline, this.block);
}

/// The paragraph line holding [offset].
({int start, int end, Map attrs})? _lineAt(Delta document, int offset) {
  for (final line in listLines(document)) {
    if (line.start <= offset && offset <= line.end) {
      return (start: line.start, end: line.end, attrs: line.attrs);
    }
  }
  return null;
}

PaintedFormat captureFormat(Delta document, TextSelection selection) {
  // The first character selected, or the one before a collapsed caret.
  final at = selection.isCollapsed && selection.start > 0
      ? selection.start - 1
      : selection.start;
  final run = document.slice(at, at + 1).toList().firstOrNull;
  final attrs = run?.data is String && run!.data != '\n'
      ? run.attributes ?? const {}
      : const {};
  final line = _lineAt(document, selection.start);
  final metadata = listMetadata(line?.attrs['token']);
  return PaintedFormat(
    {for (final key in clearedCharacterAttributes) key: attrs[key]},
    {
      'align': line?.attrs['align'],
      'header': line?.attrs['header'],
      'paragraphFormat': metadata['paragraphFormat'],
      'tabs': metadata['tabs'],
    },
  );
}

/// The change that paints [format] onto [target]: its characters get the
/// character formatting and its paragraphs the paragraph formatting.
Delta paintFormat(Delta document, TextSelection target, PaintedFormat format) {
  final change = Delta();
  var offset = 0;
  for (final line in listLines(document)) {
    if (line.end < target.start ||
        (target.isCollapsed
            ? line.start > target.start
            : line.start >= target.end)) {
      continue;
    }
    final from = target.start.clamp(line.start, line.end);
    final to = target.end.clamp(line.start, line.end);
    if (from > offset) change.retain(from - offset);
    for (final op in document.slice(from, to).toList()) {
      change.retain(op.length!, op.data is String ? {...format.inline} : null);
    }
    if (line.end > to) change.retain(line.end - to);
    final metadata = listMetadata(line.attrs['token']);
    for (final key in ['paragraphFormat', 'tabs']) {
      if (format.block[key] == null) {
        metadata.remove(key);
      } else {
        metadata[key] = format.block[key];
      }
    }
    change.retain(1, {
      'align': format.block['align'],
      'header': format.block['header'],
      'token': metadata.isEmpty ? null : jsonEncode(metadata),
    });
    offset = line.end + 1;
  }
  return change..trim();
}
