// Formula point mode (X01.06): while a formula is typed, arrow keys and
// clicks put cell references into it, as in Excel's Point mode.
import 'package:flutter/services.dart';

String _address(int row, int column) {
  var label = '';
  for (var c = column + 1; c > 0; c = (c - 1) ~/ 26) {
    label = String.fromCharCode(65 + (c - 1) % 26) + label;
  }
  return '$label${row + 1}';
}

/// How the cell editor treats arrow keys, shown on the status bar: Enter
/// (typing started the entry; arrows point in formulas), Edit (F2 or a
/// double-click; arrows move the caret) and Point (a reference is being
/// chosen).
enum EditMode { enter, edit, point }

extension EditModeLabel on EditMode {
  String get label => switch (this) {
    EditMode.enter => 'Enter',
    EditMode.edit => 'Edit',
    EditMode.point => 'Point',
  };
}

/// Characters after which a reference can start.
const _referenceStarts = '=+-*/^&(,:<>;{%';

/// Whether a reference can be inserted at [caret] in [text]: the text is a
/// formula, the caret is outside a string, and the nearest character before
/// it (ignoring spaces) starts an operand.
bool canPoint(String text, int caret) {
  if (!text.startsWith('=') || caret < 1 || caret > text.length) return false;
  if ('"'.allMatches(text.substring(0, caret)).length.isOdd) return false;
  var i = caret - 1;
  while (i > 0 && text[i] == ' ') {
    i--;
  }
  return _referenceStarts.contains(text[i]);
}

/// A reference being chosen: where it sits in the text and the cells it
/// names.
class FormulaPoint {
  final int start, end;
  final int anchorRow, anchorColumn, row, column;
  const FormulaPoint({
    required this.start,
    required this.end,
    required this.anchorRow,
    required this.anchorColumn,
    required this.row,
    required this.column,
  });

  String get reference {
    final a = _address(anchorRow, anchorColumn);
    final b = _address(row, column);
    if (a == b) return a;
    final (top, bottom) = anchorRow <= row
        ? (anchorRow, row)
        : (row, anchorRow);
    final (left, right) = anchorColumn <= column
        ? (anchorColumn, column)
        : (column, anchorColumn);
    return '${_address(top, left)}:${_address(bottom, right)}';
  }
}

/// The editor value with [point]'s reference written into it, the caret
/// after it, and the reference's new extent.
(TextEditingValue, FormulaPoint) applyPoint(
  TextEditingValue value,
  FormulaPoint point,
) {
  final reference = point.reference;
  final text = value.text.replaceRange(point.start, point.end, reference);
  final end = point.start + reference.length;
  return (
    TextEditingValue(
      text: text,
      selection: TextSelection.collapsed(offset: end),
    ),
    FormulaPoint(
      start: point.start,
      end: end,
      anchorRow: point.anchorRow,
      anchorColumn: point.anchorColumn,
      row: point.row,
      column: point.column,
    ),
  );
}

/// The arrow key's step as (rows, columns), or null for other keys.
(int, int)? arrowStep(LogicalKeyboardKey key) => switch (key) {
  LogicalKeyboardKey.arrowUp => (-1, 0),
  LogicalKeyboardKey.arrowDown => (1, 0),
  LogicalKeyboardKey.arrowLeft => (0, -1),
  LogicalKeyboardKey.arrowRight => (0, 1),
  _ => null,
};
