import 'dart:convert';
import 'dart:math' as math;

Map<String, dynamic> newDocumentTableRow(List content) => {
  'type': 'tableRow',
  'attrs': {
    'officeId':
        'folio-row-${base64Url.encode(List.generate(16, (_) => math.Random.secure().nextInt(256))).replaceAll('=', '')}',
  },
  'content': content,
};

Map<String, dynamic> copyTableNode(Map node) =>
    Map<String, dynamic>.from(jsonDecode(jsonEncode(node)));
bool tableRowCanSplit(Map row) =>
    (row['attrs']?['cantSplit'] ?? row['attrs']?['inheritedCantSplit']) != true;
Map<String, dynamic> emptyTableCell() => {
  'type': 'tableCell',
  'attrs': {'colspan': 1, 'rowspan': 1},
  'content': [
    {'type': 'paragraph'},
  ],
};

class TablePosition {
  final int row, col, rows, cols, index;
  final Map<String, dynamic> cell;
  const TablePosition(
    this.row,
    this.col,
    this.rows,
    this.cols,
    this.index,
    this.cell,
  );
  int get bottom => row + rows - 1;
  int get right => col + cols - 1;
  bool contains(int r, int c) =>
      r >= row && r <= bottom && c >= col && c <= right;
}

class TableArea {
  final int top, left, bottom, right;
  const TableArea(this.top, this.left, this.bottom, this.right);
  bool includes(TablePosition p) =>
      p.row >= top && p.bottom <= bottom && p.col >= left && p.right <= right;
  bool intersects(TablePosition p) =>
      p.row <= bottom && p.bottom >= top && p.col <= right && p.right >= left;
}

class DocumentTable {
  final Map<String, dynamic> node;
  final List<TablePosition> cells = [];
  late final List rows;
  int columns = 0;

  /// Only the consecutive prefix has repeating-header semantics in Word.
  int get headerRows =>
      rows.takeWhile((r) => r['attrs']?['header'] == true).length;
  DocumentTable(Map source) : node = copyTableNode(source) {
    rows = node['content'] as List;
    if (rows.isEmpty || rows.length > 1000) {
      throw const FormatException('Tables need 1–1,000 rows.');
    }
    final occupied = <(int, int)>{};
    for (var r = 0; r < rows.length; r++) {
      var c = 0, i = 0;
      for (final raw in rows[r]['content'] as List) {
        while (occupied.contains((r, c))) {
          c++;
        }
        final cell = Map<String, dynamic>.from(raw);
        final rs = cell['attrs']?['rowspan'] as int? ?? 1;
        final cs = cell['attrs']?['colspan'] as int? ?? 1;
        if (rs < 1 || cs < 1 || r + rs > rows.length || c + cs > 1000) {
          throw const FormatException('Invalid merged cell.');
        }
        for (var y = r; y < r + rs; y++) {
          for (var x = c; x < c + cs; x++) {
            if (!occupied.add((y, x))) {
              throw const FormatException('Overlapping table cells.');
            }
          }
        }
        cells.add(TablePosition(r, c, rs, cs, i++, cell));
        c += cs;
        if (c > columns) columns = c;
      }
    }
    if (columns == 0 || occupied.length != rows.length * columns) {
      throw const FormatException('The table has uncovered grid positions.');
    }
    final widths = node['attrs']?['columnWidths'] as List?;
    if (widths != null &&
        (widths.length != columns ||
            widths.any(
              (n) => n is! num || !n.isFinite || n <= 0 || n > 31680,
            ))) {
      throw const FormatException('Invalid table column widths.');
    }
  }

  Map<String, dynamic> format(
    TableArea area, {
    int? columnWidth,
    bool equalColumns = false,
    Map<String, dynamic>? height,
    String? fill,
    String? verticalAlign,
    String? alignment,
    bool? header,
    bool? cantSplit,
  }) {
    if (columnWidth != null && (columnWidth < 1 || columnWidth > 31680)) {
      throw const FormatException(
        'Column width must be greater than 0 and at most 1,584 pt.',
      );
    }
    if (height != null &&
        (!['auto', 'atLeast', 'exact'].contains(height['rule']) ||
            height['value'] is! int ||
            height['value'] < 0 ||
            height['value'] > 31680 ||
            (height['rule'] != 'auto' && height['value'] == 0))) {
      throw const FormatException(
        'Row height must be greater than 0 and at most 1,584 pt.',
      );
    }
    if (fill != null &&
        fill != 'auto' &&
        !RegExp(r'^[0-9a-fA-F]{6}$').hasMatch(fill)) {
      throw const FormatException('Enter a six-digit fill color.');
    }
    if (verticalAlign != null &&
        !['top', 'center', 'bottom'].contains(verticalAlign)) {
      throw const FormatException('Invalid vertical alignment.');
    }
    if (alignment != null && !['left', 'center', 'right'].contains(alignment)) {
      throw const FormatException('Invalid table alignment.');
    }
    final next = copyTableNode(node);
    final attrs = next['attrs'] = {...?(next['attrs'] as Map?)};
    if (columnWidth != null || equalColumns) {
      final widths = List<num>.from(
        attrs['columnWidths'] ?? List.filled(columns, 2400),
      );
      final width =
          columnWidth ??
          widths
                  .sublist(area.left, area.right + 1)
                  .fold<num>(0, (a, b) => a + b) /
              (area.right - area.left + 1);
      for (var c = area.left; c <= area.right; c++) {
        widths[c] = width;
      }
      attrs['columnWidths'] = widths;
      attrs['layout'] = 'fixed';
    }
    if (alignment != null) attrs['alignment'] = alignment;
    if (height != null) {
      for (var r = area.top; r <= area.bottom; r++) {
        final row = next['content'][r];
        row['attrs'] = {...?(row['attrs'] as Map?), 'height': Map.of(height)};
      }
    }
    if (header != null) {
      for (var r = area.top; r <= area.bottom; r++) {
        final row = next['content'][r];
        row['attrs'] = {...?(row['attrs'] as Map?), 'header': header};
      }
    }
    if (cantSplit != null) {
      for (var r = area.top; r <= area.bottom; r++) {
        final row = next['content'][r];
        row['attrs'] = {...?(row['attrs'] as Map?), 'cantSplit': cantSplit};
      }
    }
    for (final p in cells.where(area.includes)) {
      final cell = next['content'][p.row]['content'][p.index];
      cell['attrs'] = {
        ...?(cell['attrs'] as Map?),
        'fill': ?fill,
        'verticalAlign': ?verticalAlign,
      };
      // Keep split/remerge behavior consistent with the visible merged cell.
      for (final continuation
          in (cell['attrs']['mergeContinuations'] as Map? ?? {}).values) {
        continuation['attrs'] = {
          ...?(continuation['attrs'] as Map?),
          'fill': ?fill,
          'verticalAlign': ?verticalAlign,
        };
      }
    }
    return next;
  }

  TablePosition at(int row, int col) =>
      cells.firstWhere((c) => c.contains(row, col));
  TableArea area(int r1, int c1, int r2, int c2) {
    var a = TableArea(
      r1 < r2 ? r1 : r2,
      c1 < c2 ? c1 : c2,
      r1 > r2 ? r1 : r2,
      c1 > c2 ? c1 : c2,
    );
    for (;;) {
      var top = a.top, left = a.left, bottom = a.bottom, right = a.right;
      for (final p in cells.where(a.intersects)) {
        if (p.row < top) top = p.row;
        if (p.col < left) left = p.col;
        if (p.bottom > bottom) bottom = p.bottom;
        if (p.right > right) right = p.right;
      }
      if (top == a.top &&
          left == a.left &&
          bottom == a.bottom &&
          right == a.right) {
        return a;
      }
      a = TableArea(top, left, bottom, right);
    }
  }

  Map<String, dynamic> change(String action, TableArea selection) {
    final entries = [
      for (final p in cells)
        _Placed(p.row, p.col, p.rows, p.cols, copyTableNode(p.cell)),
    ];
    final rowNodes = [for (final row in rows) copyTableNode(row)];
    final widths = List<num>.from(
      node['attrs']?['columnWidths'] ?? List.filled(columns, 2400),
    );
    var count = columns;
    switch (action) {
      case 'rowBefore':
      case 'rowAfter':
        final at = action == 'rowBefore' ? selection.top : selection.bottom + 1;
        if (rowNodes.length >= 1000) {
          throw const FormatException('This table has reached the row limit.');
        }
        for (final p in entries) {
          if (p.row >= at) {
            p.row++;
          } else if (p.row + p.rows > at) {
            p.rows++;
          }
        }
        rowNodes.insert(at, newDocumentTableRow([]));
        for (var c = 0; c < count; c++) {
          if (!entries.any((p) => p.contains(at, c))) {
            entries.add(_Placed(at, c, 1, 1, emptyTableCell()));
          }
        }
      case 'columnBefore':
      case 'columnAfter':
        final at = action == 'columnBefore'
            ? selection.left
            : selection.right + 1;
        if (count >= 1000) {
          throw const FormatException(
            'This table has reached the column limit.',
          );
        }
        for (final p in entries) {
          if (p.col >= at) {
            p.col++;
          } else if (p.col + p.cols > at) {
            p.cols++;
          }
        }
        widths.insert(at, widths[(at == count ? at - 1 : at)]);
        count++;
        for (var r = 0; r < rowNodes.length; r++) {
          if (!entries.any((p) => p.contains(r, at))) {
            entries.add(_Placed(r, at, 1, 1, emptyTableCell()));
          }
        }
      case 'deleteRows':
        final n = selection.bottom - selection.top + 1;
        if (n == rowNodes.length) {
          throw const FormatException('Keep at least one row.');
        }
        for (var r = selection.bottom; r >= selection.top; r--) {
          entries.removeWhere((p) => p.row == r && p.rows == 1);
          for (final p in entries) {
            if (p.row > r) {
              p.row--;
            } else if (p.row + p.rows > r) {
              p.rows--;
            }
          }
          rowNodes.removeAt(r);
        }
      case 'deleteColumns':
        final n = selection.right - selection.left + 1;
        if (n == count) {
          throw const FormatException('Keep at least one column.');
        }
        for (var c = selection.right; c >= selection.left; c--) {
          entries.removeWhere((p) => p.col == c && p.cols == 1);
          for (final p in entries) {
            if (p.col > c) {
              p.col--;
            } else if (p.col + p.cols > c) {
              p.cols--;
            }
          }
          widths.removeAt(c);
          count--;
        }
      case 'merge':
        final selected = cells.where(selection.includes).toList();
        if (selected.length < 2) {
          throw const FormatException('Select two or more cells to merge.');
        }
        final first = entries.firstWhere(
          (p) => p.row == selection.top && p.col == selection.left,
        );
        final content = <dynamic>[];
        for (final p in selected) {
          content.addAll(p.cell['content'] as List? ?? []);
        }
        first.cell['content'] = content.isEmpty
            ? [
                {'type': 'paragraph'},
              ]
            : jsonDecode(jsonEncode(content));
        final continuations = <String, dynamic>{};
        for (var r = selection.top + 1; r <= selection.bottom; r++) {
          final id = rowNodes[r]['attrs']?['officeId'];
          if (id != null) {
            final original = at(r, selection.left);
            final source = original.row == r
                ? original.cell
                : original.cell['attrs']?['mergeContinuations']?[id];
            if (source is Map) {
              final copy = copyTableNode(source);
              copy['content'] = [
                {'type': 'paragraph'},
              ];
              (copy['attrs'] as Map?)?.remove('mergeContinuations');
              continuations[id] = copy;
            }
          }
        }
        first.cell['attrs'] = {
          ...?(first.cell['attrs'] as Map?),
          if (continuations.isNotEmpty) 'mergeContinuations': continuations,
        };
        if (continuations.isEmpty) {
          (first.cell['attrs'] as Map).remove('mergeContinuations');
        }
        entries.removeWhere(
          (p) =>
              p != first &&
              p.row >= selection.top &&
              p.row <= selection.bottom &&
              p.col >= selection.left &&
              p.col <= selection.right,
        );
        first.rows = selection.bottom - selection.top + 1;
        first.cols = selection.right - selection.left + 1;
      case 'split':
        final chosen = at(selection.top, selection.left);
        if (chosen.rows == 1 && chosen.cols == 1) {
          throw const FormatException('Select a merged cell to split.');
        }
        final first = entries.firstWhere(
          (p) => p.row == chosen.row && p.col == chosen.col,
        );
        first.rows = 1;
        first.cols = 1;
        for (var r = chosen.row; r <= chosen.bottom; r++) {
          for (var c = chosen.col; c <= chosen.right; c++) {
            if (r == chosen.row && c == chosen.col) continue;
            var cell = emptyTableCell();
            final id = rowNodes[r]['attrs']?['officeId'];
            final source = chosen.cell['attrs']?['mergeContinuations']?[id];
            if (c == chosen.col && source is Map) {
              cell = copyTableNode(source);
            }
            cell['content'] = [
              {'type': 'paragraph'},
            ];
            (cell['attrs'] as Map?)?.remove('mergeContinuations');
            entries.add(_Placed(r, c, 1, 1, cell));
          }
        }
        (first.cell['attrs'] as Map?)?.remove('mergeContinuations');
      default:
        throw const FormatException('Unknown table command.');
    }
    entries.sort(
      (a, b) =>
          a.row != b.row ? a.row.compareTo(b.row) : a.col.compareTo(b.col),
    );
    for (final row in rowNodes) {
      row['content'] = <dynamic>[];
    }
    for (final p in entries) {
      p.cell['attrs'] = {
        ...?(p.cell['attrs'] as Map?),
        'rowspan': p.rows,
        'colspan': p.cols,
      };
      (rowNodes[p.row]['content'] as List).add(p.cell);
    }
    final result = copyTableNode(node);
    result['content'] = rowNodes;
    result['attrs'] = {...?(result['attrs'] as Map?), 'columnWidths': widths};
    DocumentTable(result);
    return result;
  }
}

class _Placed {
  int row, col, rows, cols;
  final Map<String, dynamic> cell;
  _Placed(this.row, this.col, this.rows, this.cols, this.cell);
  bool contains(int r, int c) =>
      r >= row && r < row + rows && c >= col && c < col + cols;
}
