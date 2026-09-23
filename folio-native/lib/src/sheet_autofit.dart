import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'sheet_font.dart';
import 'sheet_icon_set.dart';
import 'sheet_column_metrics.dart';
import 'sheet_number_format.dart';
import 'table_controls.dart';
import 'workbook_block.dart' show parseCell;

/// Measures actual populated/styled cells, not only the viewport or the
/// selected columns. Row measurements use points; rendering uses logical px.
Future<Map<String, double>> measureSheetRows({
  required Map sheet,
  required Map calculated,
  Map formatted = const {},
  required int first,
  required int count,
  required SheetAxisLayout columns,
  required TextScaler scaler,
  required TextDirection direction,
  required Future<void> Function() checkpoint,
  Set<int>? targetRows,
}) async {
  if (first < 0 || count <= 0 || count > 100000 || first + count > 1048576) {
    throw StateError('Choose up to 100,000 rows within the worksheet.');
  }
  final rows = VisibleRows(sheet),
      defaults = sheet['styleDefaults'] as Map? ?? {};
  final normal = normalSheetFont(sheet);
  final sizes = <String, double>{};
  // Blank rows use the worksheet's default height. Explicit row font styles
  // still count, even when the row has no populated cells.
  final defaultHeight = (sheet['defaultRowHeight'] as num?)?.toDouble() ?? 24;
  double measure(String text, Map font, CellTextLayout layout, double width) {
    final box = SheetFontRenderBox(
      sheetFontRuns(text, font),
      const Color(0xff242424),
      direction,
      scaler,
      layout,
    );
    try {
      return box.autoFitHeight(width);
    } finally {
      box.dispose();
    }
  }

  final defaultFontHeight = measure(
    '',
    normal,
    CellTextLayout(normal, normalFont: normal),
    100,
  );
  final selected =
      targetRows?.toList() ?? List<int>.generate(count, (i) => first + i);
  if (selected.any((r) => r < 0 || r >= 1048576)) {
    throw StateError('Invalid automatic row.');
  }
  var visited = 0;
  for (final r in selected) {
    if (!rows.isHidden(r) && sheet['rowHeights']?['${r + 1}'] != 0) {
      final font = defaults['rows']?['${r + 1}'] as Map?;
      final height = font == null
          ? defaultFontHeight
          : measure('', font, CellTextLayout(font, normalFont: normal), 100);
      sizes['${r + 1}'] = math
          .max(defaultHeight, (height + 6).ceilToDouble() * .75)
          .clamp(.75, 409);
    }
    if (++visited % 128 == 0) await checkpoint();
  }
  final addresses = <String>{
    ...(sheet['cells'] as Map? ?? {}).keys.cast<String>(),
    ...calculated.keys.cast<String>(),
    ...(sheet['cellStyles'] as Map? ?? {}).keys.cast<String>(),
  };
  final tables = (sheet['tables'] as List? ?? []).cast<Map>();
  final areas = [for (final t in tables) TableArea.parse(t['range'])];
  final filter = sheet['autoFilter']?['range'] as String?;
  final filterArea = filter == null ? null : TableArea.parse(filter);
  var processed = 0;
  for (final address in addresses) {
    if (++processed % 128 == 0) await checkpoint();
    final (r, c) = parseCell(address);
    final row = '${r + 1}';
    if (!sizes.containsKey(row) || columns.visible.isHidden(c)) continue;
    final style = numberDisplayStyle(
      cellStyle(sheet, address),
      formatted[address] as Map?,
    );
    final value = calculated[address] ?? sheet['cells']?[address] ?? '';
    final text = formatCellValue(
      value,
      style,
      display: formatted[address] as Map?,
    );
    var bold = false,
        filterButton =
            filterArea != null &&
            r == filterArea.top &&
            filterArea.contains(r, c);
    for (var i = 0; i < tables.length; i++) {
      if (!areas[i].contains(r, c)) continue;
      final header = tables[i]['headerRows'] != 0 && r == areas[i].top;
      bold = header || (tables[i]['totalsRows'] == 1 && r == areas[i].bottom);
      filterButton = filterButton || header;
      break;
    }
    final width = columns.extent(c);
    // Match the grid padding plus its worst-case selected border. Filter
    // controls occupy a further 24px in table/worksheet header cells.
    final available = math.max(
      1.0,
      width -
          (width < 42 ? 0 : 18) -
          3.2 -
          (width >= 42 && filterButton ? 24 : 0) -
          sheetIconSpace(style, scaler),
    );
    final height = math.max(
      sheetIconSpace(style, scaler) > 0 ? sheetIconSize(style, scaler) : 0.0,
      measure(
        sheetValueHidden(style) ? '' : text,
        {'bold': bold, ...style},
        CellTextLayout(style, value: value, normalFont: normal),
        available,
      ),
    );
    sizes[row] = math
        .max(sizes[row]!, (height + 7.2).ceilToDouble() * .75)
        .clamp(.75, 409);
  }
  await checkpoint();
  return sizes;
}

/// Rows are chosen by the engine from a staged, recalculated edit. No draft
/// reaches the live workspace until all sheets have been measured successfully.
Future<Map<String, dynamic>> measureAutomaticRows(
  Map<String, dynamic> prepared, {
  TextScaler scaler = TextScaler.noScaling,
  TextDirection direction = TextDirection.ltr,
}) async {
  final result = <String, dynamic>{};
  final targets = prepared['layoutRows'] as Map;
  for (final sheet in prepared['workspace']['sheets'] as List) {
    final id = sheet['id'] as String;
    final rows = targets[id] as List?;
    if (rows == null || rows.isEmpty) continue;
    final metrics = SheetColumnMetrics.fromFont(normalSheetFont(sheet));
    final columns = SheetAxisLayout(
      VisibleColumns(sheet),
      sheet['defaultColumnWidth'] is num
          ? metrics.pixels(sheet['defaultColumnWidth'] as num)
          : 122,
      {
        for (final e in (sheet['columnWidths'] as Map? ?? {}).entries)
          int.parse(e.key) - 1: metrics.pixels(e.value as num),
      },
    );
    result[id] = await measureSheetRows(
      sheet: sheet,
      calculated: prepared['calculated']?[id] as Map? ?? {},
      formatted: prepared['formatted']?[id] as Map? ?? {},
      first: 0,
      count: 1,
      targetRows: rows.map((r) => (r as num).toInt() - 1).toSet(),
      columns: columns,
      scaler: scaler,
      direction: direction,
      checkpoint: () => Future<void>.delayed(Duration.zero),
    );
  }
  return result;
}

/// AutoFit is an explicit fit to natural text lines. Wrap/Shrink settings stay
/// authored, but must not limit the width being measured to the old width.
Future<Map<String, double>> measureSheetColumns({
  required Map sheet,
  required Map calculated,
  Map formatted = const {},
  required int first,
  required int count,
  required SheetAxisLayout columns,
  required SheetColumnMetrics metrics,
  required TextScaler scaler,
  required TextDirection direction,
  required Future<void> Function() checkpoint,
}) async {
  if (first < 0 || count <= 0 || first + count > 16384) {
    throw StateError('Choose columns within the worksheet.');
  }
  final normal = normalSheetFont(sheet), rows = VisibleRows(sheet);
  final sizes = <String, double>{}, measured = <String>{};
  final defaultWidth =
      (sheet['defaultColumnWidth'] as num?)?.toDouble() ??
      metrics.fitPixels(columns.defaultExtent);
  for (var c = first; c < first + count; c++) {
    if (!columns.visible.isHidden(c) &&
        sheet['columnWidths']?['${c + 1}'] != 0) {
      sizes['${c + 1}'] = defaultWidth;
    }
    if ((c - first + 1) % 128 == 0) await checkpoint();
  }
  final addresses = <String>{
    ...(sheet['cells'] as Map? ?? {}).keys.cast<String>(),
    ...calculated.keys.cast<String>(),
    ...(sheet['cellStyles'] as Map? ?? {}).keys.cast<String>(),
  };
  final tables = (sheet['tables'] as List? ?? []).cast<Map>();
  final areas = [for (final table in tables) TableArea.parse(table['range'])];
  final filter = sheet['autoFilter']?['range'] as String?;
  final filterArea = filter == null ? null : TableArea.parse(filter);
  var processed = 0;
  for (final address in addresses) {
    if (++processed % 128 == 0) await checkpoint();
    final (r, c) = parseCell(address);
    final key = '${c + 1}';
    if (!sizes.containsKey(key) ||
        rows.isHidden(r) ||
        sheet['rowHeights']?['${r + 1}'] == 0) {
      continue;
    }
    final style = numberDisplayStyle(
      cellStyle(sheet, address),
      formatted[address] as Map?,
    );
    final value = calculated[address] ?? sheet['cells']?[address] ?? '';
    final text = formatCellValue(
      value,
      style,
      display: formatted[address] as Map?,
    );
    if (text.isEmpty) continue;
    var bold = false,
        filterButton =
            filterArea != null &&
            r == filterArea.top &&
            filterArea.contains(r, c);
    for (var i = 0; i < tables.length; i++) {
      if (!areas[i].contains(r, c)) continue;
      final header = tables[i]['headerRows'] != 0 && r == areas[i].top;
      bold = header || (tables[i]['totalsRows'] == 1 && r == areas[i].bottom);
      filterButton = filterButton || header;
      break;
    }
    final box = SheetFontRenderBox(
      sheetFontRuns(sheetValueHidden(style) ? '' : text, {
        'bold': bold,
        ...style,
      }),
      const Color(0xff242424),
      direction,
      scaler,
      CellTextLayout(style, value: value, normalFont: normal),
    );
    late double width;
    try {
      width = metrics.fitPixels(
        box.autoFitWidth() +
            sheetIconSpace(style, scaler) +
            22 +
            (filterButton ? 24 : 0),
      );
    } finally {
      box.dispose();
    }
    if (measured.add(key)) {
      sizes[key] = width;
    } else {
      sizes[key] = math.max(sizes[key]!, width);
    }
  }
  await checkpoint();
  return sizes;
}
