import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_table_resize.dart';
import 'package:folio_native/src/document_table_view.dart';

void main() {
  test('borders are found near interior columns and row bottoms', () {
    final g = TableGridGeometry()
      ..xs = [0, 100, 250]
      ..ys = [0, 30, 60, 90]
      ..cells = [
        // A heading merged across both columns, then two plain rows, the
        // second column's last two cells merged down.
        (row: 0, col: 0, rows: 1, cols: 2),
        (row: 1, col: 0, rows: 1, cols: 1),
        (row: 1, col: 1, rows: 2, cols: 1),
        (row: 2, col: 0, rows: 1, cols: 1),
      ];
    // No column border inside the merged heading.
    expect(tableBorderAt(g, const Offset(100, 15)), isNull);
    expect(tableBorderAt(g, const Offset(100, 45)), (column: true, index: 0));
    // No row border inside the cell merged down.
    expect(tableBorderAt(g, const Offset(200, 60)), isNull);
    expect(tableBorderAt(g, const Offset(50, 60)), (column: false, index: 1));
    expect(tableBorderAt(g, const Offset(102, 40)), (column: true, index: 0));
    expect(tableBorderAt(g, const Offset(50, 28)), (column: false, index: 0));
    expect(tableBorderAt(g, const Offset(50, 89)), (column: false, index: 2));
    expect(tableBorderAt(g, const Offset(50, 15)), isNull);
    // The outer edges don't move a column pair.
    expect(tableBorderAt(g, const Offset(249, 15)), isNull);
    expect(tableBorderAt(g, const Offset(1, 15)), isNull);
  });

  test('columns keep their total and a minimum width', () {
    final node = {
      'type': 'table',
      'attrs': {
        'columnWidths': [2000, 2000],
      },
      'content': [],
    };
    expect(
      resizeTableColumns(
        node,
        [2000, 2000],
        0,
        40,
        400,
      )['attrs']['columnWidths'],
      [2400, 1600],
    );
    expect(
      resizeTableColumns(
        node,
        [2000, 2000],
        0,
        900,
        400,
      )['attrs']['columnWidths'],
      [4000 - minColumnTwips, minColumnTwips],
    );
    expect(node['attrs'], {
      'columnWidths': [2000, 2000],
    });
  });

  test('rows get an at-least height, and exact stays exact', () {
    final node = {
      'type': 'table',
      'content': [
        {'type': 'tableRow', 'content': []},
        {
          'type': 'tableRow',
          'attrs': {
            'height': {'rule': 'exact', 'value': 600},
          },
          'content': [],
        },
      ],
    };
    expect(resizeTableRow(node, 0, 40)['content'][0]['attrs']['height'], {
      'rule': 'atLeast',
      'value': 600,
    });
    expect(resizeTableRow(node, 1, 1)['content'][1]['attrs']['height'], {
      'rule': 'exact',
      'value': minRowTwips,
    });
  });

  testWidgets('cells keep their clicks; only borders resize', (t) async {
    Map cell(String s) => {
      'type': 'tableCell',
      'attrs': {'colspan': 1, 'rowspan': 1},
      'content': [
        {
          'type': 'paragraph',
          'content': [
            {'type': 'text', 'text': s},
          ],
        },
      ],
    };
    final node = {
      'type': 'table',
      'attrs': {
        'columnWidths': [4000, 4000],
      },
      'content': [
        {
          'type': 'tableRow',
          'content': [cell('A'), cell('B')],
        },
      ],
    };
    final events = <Object>[];
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: DocumentTableView(
            node: node,
            onCell: (r, c) => events.add('$r,$c'),
            onResize: events.add,
          ),
        ),
      ),
    );
    await t.tap(find.byKey(const ValueKey('table-cell-0-1')));
    await t.pumpAndSettle();
    expect(events, ['0,1']);
    final a = t.getRect(find.byKey(const ValueKey('table-cell-0-0')));
    await t.dragFrom(Offset(a.right, a.center.dy), const Offset(-30, 0));
    await t.pumpAndSettle();
    expect(events, hasLength(2));
    final widths = (events.last as Map)['attrs']['columnWidths'] as List;
    expect(widths[0], lessThan(4000));
    expect(widths[0] + widths[1], 8000);
  });
}
