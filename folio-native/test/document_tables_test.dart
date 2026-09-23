import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_tables.dart';

Map<String, dynamic> sample() => {
  'type': 'table',
  'attrs': {
    'columnWidths': [1400, 2200, 1800],
  },
  'content': [
    for (var r = 0; r < 3; r++)
      {
        'type': 'tableRow',
        'attrs': {'officeId': 'r$r'},
        'content': [
          for (var c = 0; c < 3; c++)
            {
              'type': 'tableCell',
              'attrs': {'officeId': 'r${r}c$c', 'rowspan': 1, 'colspan': 1},
              'content': [
                {
                  'type': 'paragraph',
                  'content': [
                    {
                      'type': 'text',
                      'text': '$r,$c',
                      'marks': [
                        {'type': 'bold'},
                      ],
                    },
                  ],
                },
              ],
            },
        ],
      },
  ],
};
void main() {
  test(
    'formatting a merged selection preserves content and transforms dimensions',
    () {
      final original = sample();
      final merged = DocumentTable(
        original,
      ).change('merge', const TableArea(0, 0, 1, 1));
      final grid = DocumentTable(merged);
      final formatted = grid.format(
        grid.area(0, 0, 1, 1),
        columnWidth: 1800,
        height: {'rule': 'exact', 'value': 480},
        fill: 'FFF2CC',
        verticalAlign: 'bottom',
        alignment: 'right',
      );
      final result = DocumentTable(formatted);
      expect(formatted['attrs']['columnWidths'], [1800, 1800, 1800]);
      expect(formatted['attrs']['layout'], 'fixed');
      expect(result.at(0, 0).cell['content'], grid.at(0, 0).cell['content']);
      expect(result.at(0, 0).cell['attrs']['fill'], 'FFF2CC');
      expect(
        result
            .at(0, 0)
            .cell['attrs']['mergeContinuations']['r1']['attrs']['fill'],
        'FFF2CC',
      );
      expect(formatted['content'][0]['attrs']['height'], {
        'rule': 'exact',
        'value': 480,
      });
      expect(formatted['content'][1]['attrs']['height'], {
        'rule': 'exact',
        'value': 480,
      });
      expect(formatted['content'][2]['attrs']['height'], isNull);
      expect(merged['attrs']['columnWidths'], [1400, 2200, 1800]);
      final inserted = DocumentTable(
        result.change('rowBefore', const TableArea(0, 0, 0, 1)),
      );
      expect(inserted.rows[1]['attrs']['height'], {
        'rule': 'exact',
        'value': 480,
      });
      expect(inserted.rows[0]['attrs']?['height'], isNull);
      final equal = DocumentTable(
        original,
      ).format(const TableArea(0, 0, 2, 2), equalColumns: true);
      expect(equal['attrs']['columnWidths'], [1800, 1800, 1800]);
      expect(
        () => grid.format(const TableArea(0, 0, 1, 1), columnWidth: 0),
        throwsFormatException,
      );
      expect(
        () => grid.format(
          const TableArea(0, 0, 1, 1),
          height: {'rule': 'exact', 'value': 0},
        ),
        throwsFormatException,
      );
    },
  );
  test('merge/split keep every rich paragraph and span selection expands', () {
    final source = sample();
    final original = jsonEncode(source);
    final merged = DocumentTable(
      source,
    ).change('merge', const TableArea(0, 0, 1, 1));
    final grid = DocumentTable(merged);
    expect(grid.cells.length, 6);
    expect(grid.at(1, 1).rows, 2);
    expect(grid.at(1, 1).cols, 2);
    expect(nodeText(grid.at(0, 0).cell), '0,00,11,01,1');
    for (final p in grid.at(0, 0).cell['content']) {
      expect(p['content'][0]['marks'], [
        {'type': 'bold'},
      ]);
    }
    final area = grid.area(1, 1, 2, 2);
    expect([area.top, area.left, area.bottom, area.right], [0, 0, 2, 2]);
    final split = DocumentTable(
      grid.change('split', const TableArea(0, 0, 1, 1)),
    );
    expect(split.cells.length, 9);
    expect(split.at(0, 0).cell['content'].length, 4);
    expect(nodeText(split.at(1, 1).cell), '');
    expect(jsonEncode(source), original);
  });
  test('insert inside spans extends them and grid dimensions stay exact', () {
    final merged = DocumentTable(
      sample(),
    ).change('merge', const TableArea(0, 0, 1, 1));
    var grid = DocumentTable(merged);
    grid = DocumentTable(grid.change('rowBefore', const TableArea(1, 2, 1, 2)));
    expect(grid.rows.length, 4);
    expect(grid.at(2, 0).rows, 3);
    expect(grid.at(1, 2).rows, 1);
    expect(nodeText(grid.at(1, 2).cell), '');
    grid = DocumentTable(
      grid.change('columnBefore', const TableArea(3, 1, 3, 1)),
    );
    expect(grid.columns, 4);
    expect(grid.at(1, 2).cols, 3);
    expect(grid.node['attrs']['columnWidths'], [1400, 2200, 2200, 1800]);
    expect(nodeText(grid.at(3, 3).cell), '2,2');
  });
  test('deleting a span anchor keeps merged content in remaining cells', () {
    var grid = DocumentTable(
      DocumentTable(sample()).change('merge', const TableArea(0, 0, 1, 1)),
    );
    final text = nodeText(grid.at(0, 0).cell);
    grid = DocumentTable(
      grid.change('deleteRows', const TableArea(0, 2, 0, 2)),
    );
    expect(grid.rows.length, 2);
    expect(grid.at(0, 0).rows, 1);
    expect(nodeText(grid.at(0, 0).cell), text);
    grid = DocumentTable(
      grid.change('deleteColumns', const TableArea(1, 0, 1, 0)),
    );
    expect(grid.columns, 2);
    expect(grid.at(0, 0).cols, 1);
    expect(nodeText(grid.at(0, 0).cell), text);
    expect(
      () => grid.change('deleteRows', const TableArea(0, 0, 1, 1)),
      throwsFormatException,
    );
    expect(
      () => grid.change('deleteColumns', const TableArea(0, 0, 1, 1)),
      throwsFormatException,
    );
  });
  test(
    'invalid grids and no-op merge/split requests reject without mutation',
    () {
      final table = sample();
      final copy = jsonEncode(table);
      final grid = DocumentTable(table);
      expect(
        () => grid.change('merge', const TableArea(0, 0, 0, 0)),
        throwsFormatException,
      );
      expect(
        () => grid.change('split', const TableArea(0, 0, 0, 0)),
        throwsFormatException,
      );
      expect(jsonEncode(table), copy);
      table['content'][0]['content'][0]['attrs']['rowspan'] = 4;
      expect(() => DocumentTable(table), throwsFormatException);
    },
  );
}
