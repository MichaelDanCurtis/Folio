// The fill handle (X01.13): drag the selection's corner to repeat its cells
// down, up, right or left; drag back inside to clear; double-click to fill
// down beside the adjacent data.
import 'table_controls.dart';

/// What releasing the handle does: fill [range] from the selection in
/// [direction], or clear the contents of [range] (dragged back inside).
class FillHandlePlan {
  final TableArea range;
  final String? direction;
  const FillHandlePlan.fill(this.range, String this.direction);
  const FillHandlePlan.clear(this.range) : direction = null;
  bool get clears => direction == null;
  @override
  String toString() =>
      '${clears ? 'clear' : direction} ${range.top},${range.left}:${range.bottom},${range.right}';
}

/// The plan for the handle released over ([row], [column]), as Excel reads
/// it: past the selection's edge, the axis with the larger overshoot wins;
/// inside it, the released rows (or columns) are cleared.
FillHandlePlan? planFillHandle(TableArea s, int row, int column) {
  final down = row - s.bottom, up = s.top - row;
  final right = column - s.right, left = s.left - column;
  final vertical = down > 0 ? down : (up > 0 ? up : 0);
  final horizontal = right > 0 ? right : (left > 0 ? left : 0);
  if (vertical > 0 || horizontal > 0) {
    if (vertical >= horizontal) {
      return down > 0
          ? FillHandlePlan.fill(TableArea(s.top, s.left, row, s.right), 'down')
          : FillHandlePlan.fill(
              TableArea(row, s.left, s.bottom, s.right),
              'up',
            );
    }
    return right > 0
        ? FillHandlePlan.fill(
            TableArea(s.top, s.left, s.bottom, column),
            'right',
          )
        : FillHandlePlan.fill(
            TableArea(s.top, column, s.bottom, s.right),
            'left',
          );
  }
  // Inside the selection: rows below the pointer, or columns right of it.
  final releasedRows = s.bottom - row, releasedColumns = s.right - column;
  if (releasedRows <= 0 && releasedColumns <= 0) return null;
  if (releasedRows >= releasedColumns) {
    return FillHandlePlan.clear(TableArea(row + 1, s.left, s.bottom, s.right));
  }
  return FillHandlePlan.clear(TableArea(s.top, column + 1, s.bottom, s.right));
}

/// Double-click: the last row to fill down to, from the data in the column
/// left of the selection (or right of it, when the left one is empty below
/// the selection). Null when there is nothing to follow.
int? fillDownEnd(Map cells, TableArea s) {
  bool filled(int r, int c) =>
      c >= 0 && (cells[_address(r, c)]?.toString() ?? '').isNotEmpty;
  for (final column in [s.left - 1, s.right + 1]) {
    if (!filled(s.bottom + 1, column)) continue;
    var r = s.bottom + 1;
    while (r < 1048575 && filled(r + 1, column)) {
      r++;
    }
    return r;
  }
  return null;
}

/// The cell index at [position] along an axis, from its layout: the last
/// index whose start is at or before it. Hidden indices are skipped.
int indexAt(SheetAxisLayout layout, double position, int limit) {
  var lo = 0, hi = limit - 1;
  if (position <= 0) return layout.visible.next(-1, 1);
  while (lo < hi) {
    final mid = (lo + hi + 1) ~/ 2;
    if (layout.offset(mid) <= position) {
      lo = mid;
    } else {
      hi = mid - 1;
    }
  }
  while (lo > 0 && layout.visible.isHidden(lo)) {
    lo--;
  }
  return lo;
}

String _address(int row, int column) {
  var label = '';
  for (var c = column + 1; c > 0; c = (c - 1) ~/ 26) {
    label = String.fromCharCode(65 + (c - 1) % 26) + label;
  }
  return '$label${row + 1}';
}
