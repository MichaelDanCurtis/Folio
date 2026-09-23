import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'core.dart';
import 'sheet_view.dart' show cellAddress, displayCell;

(int, int) parseCell(String text) {
  final m = RegExp(
    r'^([A-Z]{1,3})([1-9][0-9]*)$',
  ).firstMatch(text.toUpperCase());
  if (m == null) throw const FormatException('Use an Excel cell address');
  var col = 0;
  for (final c in m[1]!.codeUnits) {
    col = col * 26 + c - 64;
  }
  final row = int.parse(m[2]!);
  if (row > 1048576 || col > 16384) {
    throw const FormatException('Address exceeds worksheet bounds');
  }
  return (row - 1, col - 1);
}

List<List<dynamic>> rangeValues(Map calculated, String range) {
  final parts = range.toUpperCase().split(':');
  if (parts.length > 2) throw const FormatException('Invalid range');
  final (r1, c1) = parseCell(parts.first);
  final (r2, c2) = parseCell(parts.last);
  if (r2 < r1 || c2 < c1 || (r2 - r1 + 1) * (c2 - c1 + 1) > 2000) {
    throw const FormatException('Select up to 2,000 cells');
  }
  return [
    for (var r = r1; r <= r2; r++)
      [for (var c = c1; c <= c2; c++) calculated[cellAddress(r, c)]],
  ];
}

class WorkbookBlock extends StatelessWidget {
  final WorkspaceModel model;
  final Map attributes;
  final void Function(String, String)? onOpen;
  const WorkbookBlock({
    super.key,
    required this.model,
    required this.attributes,
    this.onOpen,
  });
  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: model,
    builder: (context, _) {
      final matches = model.sheets.where(
        (s) => s['id'] == attributes['sheetId'],
      );
      if (matches.isEmpty) {
        return const Text(
          'Source worksheet is unavailable. Restore it or relink this block.',
        );
      }
      final sheet = matches.first;
      final range = attributes['range']?.toString() ?? 'A1:B9';
      if (range == '#REF!') {
        return const Text(
          'The source cells were deleted. Undo the deletion or relink this block.',
        );
      }
      List<List<dynamic>> rows;
      try {
        rows = rangeValues(
          model.snapshot['calculated']?[sheet['id']] ?? {},
          range,
        );
      } catch (e) {
        return Text(e.toString());
      }
      final displays = rangeValues({
        for (final e
            in (model.snapshot['formatted']?[sheet['id']] as Map? ?? {})
                .entries)
          e.key: e.value['text'],
      }, range);
      String shown(List row, int col) =>
          displays[rows.indexOf(row)][col] as String? ?? displayCell(row[col]);
      final chart = attributes['mode'] == 'chart';
      final metrics = rows
          .where((r) => r.length > 1 && r[1] is num)
          .take(30)
          .toList();
      final maxValue = metrics.fold<double>(
        1,
        (m, r) => math.max(m, (r[1] as num).abs().toDouble()),
      );
      return Container(
        margin: const EdgeInsets.symmetric(vertical: 8),
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: Colors.white,
          border: Border.all(color: const Color(0xffcbded3)),
          borderRadius: BorderRadius.circular(6),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                const Icon(Icons.link, size: 15, color: Color(0xff107c41)),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    attributes['title']?.toString() ?? 'Linked workbook data',
                    style: const TextStyle(
                      fontWeight: FontWeight.w600,
                      fontSize: 12,
                    ),
                  ),
                ),
                if (onOpen != null)
                  IconButton(
                    tooltip: 'Open source range',
                    onPressed: () => onOpen!(sheet['id'], range),
                    icon: const Icon(Icons.open_in_new, size: 15),
                  ),
              ],
            ),
            Text(
              '${sheet['name']}!$range · Live · revision ${model.revision}',
              style: const TextStyle(fontSize: 10, color: Color(0xff107c41)),
            ),
            const SizedBox(height: 10),
            if (chart && metrics.isNotEmpty)
              for (final row in metrics)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Row(
                    children: [
                      SizedBox(
                        width: 95,
                        child: Text(
                          shown(row, 0),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(fontSize: 11),
                        ),
                      ),
                      Expanded(
                        child: Align(
                          alignment: Alignment.centerLeft,
                          child: FractionallySizedBox(
                            widthFactor: ((row[1] as num).abs() / maxValue)
                                .clamp(.01, 1),
                            child: Container(
                              height: 14,
                              color: (row[1] as num) < 0
                                  ? Colors.orange
                                  : const Color(0xff185abd),
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Text(shown(row, 1), style: const TextStyle(fontSize: 11)),
                    ],
                  ),
                ),
            if (!chart || metrics.isEmpty)
              SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: DataTable(
                  headingRowHeight: 30,
                  dataRowMinHeight: 28,
                  dataRowMaxHeight: 32,
                  columnSpacing: 18,
                  horizontalMargin: 5,
                  columns: [
                    for (var i = 0; i < rows.first.length; i++)
                      DataColumn(
                        label: Text(
                          shown(rows.first, i),
                          style: const TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                  ],
                  rows: [
                    for (final row in rows.skip(1).take(30))
                      DataRow(
                        cells: [
                          for (var i = 0; i < row.length; i++)
                            DataCell(
                              Text(
                                shown(row, i),
                                style: const TextStyle(fontSize: 11),
                              ),
                            ),
                        ],
                      ),
                  ],
                ),
              ),
            if (rows.length > 31)
              Text(
                'Showing 30 of ${rows.length - 1} data rows. Open the source to see all.',
                style: const TextStyle(fontSize: 10),
              ),
          ],
        ),
      );
    },
  );
}
