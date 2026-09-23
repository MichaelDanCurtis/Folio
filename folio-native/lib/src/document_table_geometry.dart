import 'document_tables.dart';

/// Resolve row heights in logical pixels after measuring each cell's contents,
/// including its padding. Exact rows never grow, including within merged cells.
/// Word's automatic rule ignores a stored height and sizes from the contents.
List<double> documentTableRowHeights(
  List<Map?> rows,
  List<TablePosition> cells,
  List<double> contentHeights,
) {
  assert(cells.length == contentHeights.length);
  final heights = [
    for (final height in rows)
      height != null && ['exact', 'atLeast'].contains(height['rule'])
          ? (height['value'] as num).toDouble() / 15
          : 0.0,
  ];
  bool exact(int row) => rows[row]?['rule'] == 'exact';
  // Resolve ordinary cells before distributing merged-cell overflow. Otherwise
  // later ordinary cells can unnecessarily inflate the merged group's height.
  for (var i = 0; i < cells.length; i++) {
    final cell = cells[i];
    if (cell.rows == 1 &&
        !exact(cell.row) &&
        contentHeights[i] > heights[cell.row]) {
      heights[cell.row] = contentHeights[i];
    }
  }
  for (var i = 0; i < cells.length; i++) {
    final cell = cells[i];
    if (cell.rows == 1) continue;
    final available = heights
        .skip(cell.row)
        .take(cell.rows)
        .fold<double>(0, (a, b) => a + b);
    if (contentHeights[i] <= available) continue;
    final flexible = [
      for (var row = cell.row; row < cell.row + cell.rows; row++)
        if (!exact(row)) row,
    ];
    if (flexible.isEmpty) continue;
    final extra = (contentHeights[i] - available) / flexible.length;
    for (final row in flexible) {
      heights[row] += extra;
    }
  }
  return heights;
}
