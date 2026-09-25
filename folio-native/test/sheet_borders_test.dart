import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_borders.dart';

void main() {
  const grid = BorderSide(color: Color(0xffe1e4e8));

  test('a cell without border lines keeps the gridlines', () {
    expect(cellBorder(null, grid), isNull);
    expect(cellBorder({'fill': '#FFFF00'}, grid), isNull);
    expect(
      cellBorder({
        'border': {
          'top': {'style': 'none'},
        },
      }, grid),
      isNull,
    );
  });

  test('line styles map to widths and colours', () {
    final border = cellBorder({
      'border': {
        'top': {'style': 'thin', 'color': '#FF0000'},
        'left': {'style': 'thick'},
        'bottom': {'style': 'hair', 'color': 'not a colour'},
      },
    }, grid)!;
    expect(border.top.color, const Color(0xffff0000));
    expect(border.top.width, 1);
    expect(border.left.width, 3);
    expect(border.left.color, const Color(0xff000000), reason: 'automatic');
    expect(border.bottom.width, 0.5);
    expect(border.right, grid, reason: 'no line: the gridline shows');
    expect(borderWidth('medium'), 2);
    expect(borderWidth('double'), 3);
    expect(borderWidth('dashDot'), 1);
  });
}
