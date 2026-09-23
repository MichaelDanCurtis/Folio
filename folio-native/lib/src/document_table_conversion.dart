import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'document_codec.dart';
import 'document_tables.dart';

void validateTableSeparator(String separator) {
  if (separator.runes.length != 1 || separator == '\r') {
    throw const FormatException('Choose one separator character.');
  }
}

class TableConversionEdit {
  final int start, length;
  final Delta replacement;
  const TableConversionEdit(this.start, this.length, this.replacement);
  Delta get change =>
      (Delta()..retain(start)).concat(replacement)..delete(length);
  int get caret =>
      start + replacement.toList().fold<int>(0, (n, op) => n + op.length!);
}

/// A captured text selection. Read line metadata from the original document,
/// including when the selection stops before its terminating paragraph mark.
class TextTableSelection {
  final Delta source;
  final int start, end;
  late final String text;
  final List<({Delta text, Map<String, dynamic>? attributes})> lines = [];

  TextTableSelection(this.source, this.start, this.end) {
    final size = source.toList().fold<int>(0, (n, op) => n + op.length!);
    if (start < 0 || end <= start || end > size) {
      throw const FormatException('Select the text to convert first.');
    }
    final selection = source.slice(start, end);
    if (selection.toList().any((op) => op.data is! String)) {
      throw const FormatException(
        'Select text without tables or other blocks.',
      );
    }
    text = selection.toList().map((op) => op.data as String).join();
    var offset = start;
    final endWithoutMark = text.endsWith('\n') ? end - 1 : end;
    while (offset <= endWithoutMark) {
      final tail = _plain(source.slice(offset));
      final newline = tail.indexOf('\n');
      final lineEnd = newline < 0 ? size : offset + newline;
      final stop = lineEnd < endWithoutMark ? lineEnd : endWithoutMark;
      lines.add((
        text: source.slice(offset, stop),
        attributes: _lineAttrs(offset),
      ));
      if (stop == endWithoutMark) break;
      offset = stop + 1;
    }
  }

  Map<String, dynamic>? _lineAttrs(int offset) {
    for (final op in source.slice(offset).toList()) {
      if (op.data is String && (op.data as String).contains('\n')) {
        return op.attributes == null ? null : Map.of(op.attributes!);
      }
    }
    return null;
  }

  int suggestedColumns(String separator) {
    validateTableSeparator(separator);
    if (separator == '\n') return 1;
    return lines.fold(1, (n, line) {
      final count = _plain(line.text).split(separator).length;
      return count > n ? count : n;
    });
  }

  Map<String, dynamic> table({
    String separator = '\t',
    int? columns,
    double? columnWidth,
  }) {
    validateTableSeparator(separator);
    final width = columns ?? suggestedColumns(separator);
    if (width < 1 || width > 1000) {
      throw const FormatException('Enter between 1 and 1,000 columns.');
    }
    if (columnWidth != null &&
        (!columnWidth.isFinite || columnWidth <= 0 || columnWidth > 1584)) {
      throw const FormatException(
        'Enter a column width above 0 and up to 1,584 points.',
      );
    }
    final rows = <Map<String, dynamic>>[];
    var cells = <Map<String, dynamic>>[];
    void finishRow() {
      if (cells.isEmpty) return;
      while (cells.length < width) {
        cells.add(emptyTableCell());
      }
      rows.add(newDocumentTableRow(cells));
      cells = [];
      if (rows.length > 1000 || rows.length * width > 100000) {
        throw const FormatException(
          'Conversion supports up to 1,000 rows and 100,000 cells.',
        );
      }
    }

    for (final line in lines) {
      final plain = _plain(line.text);
      var pos = 0;
      while (true) {
        final next = separator == '\n' ? -1 : plain.indexOf(separator, pos);
        final stop = next < 0 ? plain.length : next;
        final field = line.text.slice(pos, stop)..insert('\n', line.attributes);
        cells.add({
          'type': 'tableCell',
          'attrs': {'colspan': 1, 'rowspan': 1},
          'content': deltaToDocument(field)['content'],
        });
        if (cells.length == width) finishRow();
        if (next < 0) break;
        pos = next + separator.length;
      }
      if (separator != '\n') finishRow();
    }
    finishRow();
    return {
      'type': 'table',
      if (columnWidth != null)
        'attrs': {
          'layout': 'fixed',
          'columnWidths': List.filled(width, columnWidth * 20),
        },
      'content': rows,
    };
  }

  TableConversionEdit replaceWith(Map table) {
    final replacement = Delta();
    // Split a partial paragraph at the table boundary, preserving its metadata.
    if (start > 0 && _plain(source.slice(start - 1, start)) != '\n') {
      replacement.insert('\n', _lineAttrs(start));
    }
    replacement.insert({'folioNode': jsonEncode(table)});
    replacement.insert('\n');
    var stop = end;
    if (!text.endsWith('\n') && _plain(source.slice(end, end + 1)) == '\n') {
      stop++;
    }
    return TableConversionEdit(start, stop - start, replacement);
  }
}

String _plain(Delta delta) =>
    delta.toList().map((op) => op.data is String ? op.data : '\uFFFC').join();

/// Convert whole logical rows, leaving source-preserving table fragments above
/// and below. Expand row boundaries to include any intersecting vertical merge.
class TableRowsConversion {
  final DocumentTable table;
  final int firstRow, lastRow;
  late final TableArea rows;
  late final Map<String, dynamic> converted;
  late final Map<String, dynamic>? before, after;

  TableRowsConversion(
    Map source, {
    required this.firstRow,
    required this.lastRow,
    String separator = '\t',
    bool convertNested = true,
  }) : table = DocumentTable(source) {
    if (firstRow < 0 || lastRow < firstRow || lastRow >= table.rows.length) {
      throw FormatException(
        'Choose rows from 1 to ${table.rows.length}, in order.',
      );
    }
    rows = table.area(firstRow, 0, lastRow, table.columns - 1);
    Map<String, dynamic> fragment(int start, int end) {
      // officeId refers to source formatting, not the new fragment's identity.
      // Row/cell source references remain intact and no merge crosses a split.
      return copyTableNode({
        ...table.node,
        'content': table.rows.sublist(start, end),
      });
    }

    before = rows.top == 0 ? null : fragment(0, rows.top);
    after = rows.bottom == table.rows.length - 1
        ? null
        : fragment(rows.bottom + 1, table.rows.length);
    converted = tableToText(
      fragment(rows.top, rows.bottom + 1),
      separator: separator,
      convertNested: convertNested,
    );
  }

  bool get expanded => rows.top != firstRow || rows.bottom != lastRow;
  Map<String, dynamic> get document => {
    'type': 'doc',
    'content': [
      if (before != null) before,
      ...converted['content'] as List,
      if (after != null) after,
    ],
  };
}

/// Retain blocks and rich runs, merging only compatible paragraph boundaries.
/// Covered positions in a merged cell remain empty; its content occurs once.
Map<String, dynamic> tableToText(
  Map source, {
  String separator = '\t',
  bool convertNested = true,
}) {
  validateTableSeparator(separator);
  final table = DocumentTable(source);
  final result = <Map<String, dynamic>>[];
  bool paragraph(Map n) => n['type'] == 'paragraph' || n['type'] == 'heading';
  List<Map<String, dynamic>> contents(Map cell) {
    final out = <Map<String, dynamic>>[];
    for (final raw in cell['content'] as List? ?? []) {
      final block = copyTableNode(raw);
      if (block['type'] == 'table' && convertNested) {
        out.addAll(
          (tableToText(block, separator: separator)['content'] as List)
              .cast<Map<String, dynamic>>(),
        );
      } else {
        out.add(block);
      }
    }
    return out.isEmpty
        ? [
            {'type': 'paragraph'},
          ]
        : out;
  }

  for (var r = 0; r < table.rows.length; r++) {
    final row = <Map<String, dynamic>>[];
    for (var c = 0; c < table.columns; c++) {
      final position = table.at(r, c);
      final blocks = position.row == r && position.col == c
          ? contents(position.cell)
          : <Map<String, dynamic>>[
              {'type': 'paragraph'},
            ];
      if (c > 0 && separator != '\n') {
        if (row.isEmpty || !paragraph(row.last)) {
          row.add({'type': 'paragraph'});
        }
        final tail = row.last.putIfAbsent('content', () => <dynamic>[]) as List;
        tail.add({'type': 'text', 'text': separator});
        // Lists/opaque blocks keep their structure and their own paragraph.
        if (paragraph(blocks.first) &&
            blocks.first['attrs']?['numbering'] == null) {
          tail.addAll(blocks.removeAt(0)['content'] as List? ?? []);
        }
      }
      row.addAll(blocks);
    }
    result.addAll(row);
  }
  return {'type': 'doc', 'content': result};
}
