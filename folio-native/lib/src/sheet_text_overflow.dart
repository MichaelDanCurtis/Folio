import 'package:flutter/rendering.dart';
import 'table_controls.dart';
import 'workbook_block.dart' show parseCell;

/// Sparse occupied-cell lookup. A formula displaying an empty string remains
/// an obstruction; hidden columns occupy no display space.
class SheetOverflowIndex {
  final Map<int, List<int>> _rows = {};
  final List<TableArea> barriers;
  final VisibleColumns columns;
  SheetOverflowIndex(Map cells, this.columns, this.barriers) {
    for (final entry in cells.entries) {
      if (entry.value == null || entry.value.toString().isEmpty) continue;
      final (row, column) = parseCell(entry.key as String);
      if (!columns.isHidden(column)) {
        (_rows[row] ??= []).add(column);
      }
    }
    for (final row in _rows.values) {
      row.sort();
    }
  }

  List<int> candidates(int row, int column, int frozenColumns) {
    if (barriers.any((a) => a.contains(row, column))) return const [];
    final occupied = _rows[row] ?? const <int>[];
    var low = 0, high = occupied.length;
    while (low < high) {
      final mid = (low + high) ~/ 2;
      if (occupied[mid] < column) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }
    if (low < occupied.length && occupied[low] == column) return [column];
    return [
      for (final source in [
        if (low > 0) occupied[low - 1],
        if (low < occupied.length) occupied[low],
      ])
        if ((source < frozenColumns) == (column < frozenColumns) &&
            !barriers.any(
              (a) =>
                  row >= a.top &&
                  row <= a.bottom &&
                  a.left <= (source > column ? source : column) &&
                  a.right >= (source < column ? source : column),
            ))
          source,
    ];
  }
}

/// Fragments carry geometry only. Their source remains the sole editable and
/// accessible cell; a borrowed fragment never changes the cell beneath it.
class SheetOverflowFragment {
  final int source;
  final Rect ink;
  const SheetOverflowFragment(this.source, this.ink);
}

/// Competing labels share only their overlapping blank space. Keep the split
/// deterministic and independent of viewport/scroll position.
List<SheetOverflowFragment> clipOverflowFragments(
  List<SheetOverflowFragment> fragments,
) {
  if (fragments.length < 2) return fragments;
  final left = fragments[0], right = fragments[1];
  if (!left.ink.overlaps(right.ink)) return fragments;
  final split = (left.ink.right + right.ink.left) / 2;
  return [
    SheetOverflowFragment(
      left.source,
      Rect.fromLTRB(left.ink.left, left.ink.top, split, left.ink.bottom),
    ),
    SheetOverflowFragment(
      right.source,
      Rect.fromLTRB(split, right.ink.top, right.ink.right, right.ink.bottom),
    ),
  ];
}
