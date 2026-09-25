// Excel's keyboard navigation over data: Ctrl+arrow to the edge of a data
// region, Ctrl+End to the last used cell, and the current region for Ctrl+A.
// Pure functions over the sheet's populated cells, so they cost one pass over
// the populated cells rather than one lookup per row or column crossed.
import 'workbook_block.dart' show parseCell;

/// Populated positions along one line of the sheet, in ascending order:
/// the rows of [column], or the columns of [row]. Hidden positions are left
/// out. Ctrl+arrow skips them the way Excel does.
List<int> populatedAlong(
  Map cells, {
  int? row,
  int? column,
  required bool Function(int) hidden,
}) {
  assert((row == null) != (column == null));
  final positions = <int>[];
  for (final entry in cells.entries) {
    if ((entry.value?.toString() ?? '').isEmpty) continue;
    final (r, c) = parseCell(entry.key as String);
    if (column != null && c == column && !hidden(r)) positions.add(r);
    if (row != null && r == row && !hidden(c)) positions.add(c);
  }
  return positions..sort();
}

/// Where Ctrl+arrow lands, starting at [from] and moving by [direction]
/// (+1 or -1) along a line with the visible [populated] positions (sorted).
///
/// From a filled cell whose next visible neighbour is filled, it goes to
/// the last filled cell before a gap. Otherwise it goes to the next filled
/// cell; with none ahead, to [edge] (the last visible position that way).
/// [step] gives the next visible position, or the same one at the edge.
int dataEdge({
  required int from,
  required int direction,
  required List<int> populated,
  required int Function(int) step,
  required int edge,
}) {
  final filled = populated.toSet();
  final next = step(from);
  if (next == from) return from;
  if (filled.contains(from) && filled.contains(next)) {
    var at = next;
    while (true) {
      final after = step(at);
      if (after == at || !filled.contains(after)) return at;
      at = after;
    }
  }
  // The next populated position beyond `from`, by binary search.
  var lo = 0, hi = populated.length;
  while (lo < hi) {
    final mid = (lo + hi) >> 1;
    if (populated[mid] <= from) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }
  if (direction > 0) return lo < populated.length ? populated[lo] : edge;
  // Moving back: the last populated position before `from`.
  var i = lo - 1;
  while (i >= 0 && populated[i] >= from) {
    i--;
  }
  return i >= 0 ? populated[i] : edge;
}

/// The last used row and column (Ctrl+End): the bottom-right corner of the
/// populated cells, which need not hold a value itself.
(int, int) lastUsedCell(Map cells) {
  var lastRow = 0, lastColumn = 0;
  for (final entry in cells.entries) {
    if ((entry.value?.toString() ?? '').isEmpty) continue;
    final (r, c) = parseCell(entry.key as String);
    if (r > lastRow) lastRow = r;
    if (c > lastColumn) lastColumn = c;
  }
  return (lastRow, lastColumn);
}

/// The current region around a cell (Ctrl+A in data): the smallest
/// rectangle bounded by blank rows and columns that contains the cell and
/// every populated cell touching it, including diagonally. Returns null
/// when the cell and its neighbours are all empty, so Ctrl+A selects the
/// whole sheet.
(int, int, int, int)? currentRegion(Map cells, int row, int column) {
  bool filled(int r, int c) =>
      r >= 0 && c >= 0 && (cells[_address(r, c)]?.toString() ?? '').isNotEmpty;
  var top = row, left = column, bottom = row, right = column;
  bool anyFilled() {
    for (var r = top - 1; r <= bottom + 1; r++) {
      for (var c = left - 1; c <= right + 1; c++) {
        if (filled(r, c)) return true;
      }
    }
    return false;
  }

  if (!anyFilled()) return null;
  // Grow while the ring around the rectangle has a populated cell. Bounded
  // by the populated cells, since each growth needs one.
  var grew = true;
  while (grew) {
    grew = false;
    for (var c = left - 1; c <= right + 1; c++) {
      if (filled(top - 1, c)) {
        top--;
        grew = true;
        break;
      }
    }
    for (var c = left - 1; c <= right + 1; c++) {
      if (filled(bottom + 1, c)) {
        bottom++;
        grew = true;
        break;
      }
    }
    for (var r = top - 1; r <= bottom + 1; r++) {
      if (filled(r, left - 1)) {
        left--;
        grew = true;
        break;
      }
    }
    for (var r = top - 1; r <= bottom + 1; r++) {
      if (filled(r, right + 1)) {
        right++;
        grew = true;
        break;
      }
    }
  }
  return (top, left, bottom, right);
}

String _address(int row, int column) {
  var label = '';
  for (var c = column + 1; c > 0; c = (c - 1) ~/ 26) {
    label = String.fromCharCode(65 + (c - 1) % 26) + label;
  }
  return '$label${row + 1}';
}
