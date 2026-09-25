// Status-bar aggregates for a worksheet selection (D01.06): Average, Count
// and Sum by default, as in Excel, with Numerical Count, Minimum and Maximum
// available from the status bar's menu.
import 'package:flutter/material.dart';

import 'workbook_block.dart' show parseCell;

/// The aggregates Excel's status bar offers.
enum SelectionAggregate {
  average,
  count,
  numericalCount,
  minimum,
  maximum,
  sum,
}

const defaultAggregates = {
  SelectionAggregate.average,
  SelectionAggregate.count,
  SelectionAggregate.sum,
};

class SelectionStats {
  final int count, numericCount;
  final double sum, minimum, maximum;
  final bool hasError;
  const SelectionStats({
    required this.count,
    required this.numericCount,
    required this.sum,
    required this.minimum,
    required this.maximum,
    required this.hasError,
  });
  double? get average => numericCount == 0 ? null : sum / numericCount;
}

/// Aggregates over the populated, visible cells of `range`, using calculated
/// values. Returns null for a single cell, which shows no aggregates.
///
/// Count counts non-empty cells; the numeric aggregates use numbers only
/// (text, booleans and errors are left out). Rows and columns hidden by the
/// user or a filter are left out. Excel's documentation doesn't say which
/// hidden cells its status bar counts; this is an open comparison point
/// (see the M1 package plan).
SelectionStats? selectionStats({
  required String range,
  required Map cells,
  required Map calculated,
  required Map formatted,
  required bool Function(int row) rowHidden,
  required bool Function(int column) columnHidden,
}) {
  final ends = range.split(':');
  if (ends.length != 2 || ends[0] == ends[1]) return null;
  final (top, left) = parseCell(ends[0]);
  final (bottom, right) = parseCell(ends[1]);
  var count = 0, numericCount = 0;
  var sum = 0.0, minimum = double.infinity, maximum = double.negativeInfinity;
  var hasError = false;
  for (final entry in cells.entries) {
    final address = entry.key as String;
    if ((entry.value?.toString() ?? '').isEmpty) continue;
    final (r, c) = parseCell(address);
    if (r < top || r > bottom || c < left || c > right) continue;
    if (rowHidden(r) || columnHidden(c)) continue;
    count++;
    if ((formatted[address] as Map?)?['isError'] == true) {
      hasError = true;
      continue;
    }
    final value = calculated.containsKey(address)
        ? calculated[address]
        : entry.value;
    final number = value is num
        ? value.toDouble()
        : value is String && !value.startsWith('=')
        ? double.tryParse(value)
        : null;
    if (number == null || value is bool) continue;
    numericCount++;
    sum += number;
    if (number < minimum) minimum = number;
    if (number > maximum) maximum = number;
  }
  return SelectionStats(
    count: count,
    numericCount: numericCount,
    sum: sum,
    minimum: minimum,
    maximum: maximum,
    hasError: hasError,
  );
}

/// A number for the status bar: grouped thousands, up to ten significant
/// decimals, no trailing zeros.
String formatStatusNumber(double value) {
  if (value.isNaN || value.isInfinite) return '';
  final fixed = value.toStringAsFixed(10);
  var text = fixed.contains('.')
      ? fixed.replaceFirst(RegExp(r'0+$'), '').replaceFirst(RegExp(r'\.$'), '')
      : fixed;
  if (text == '-0') text = '0';
  final negative = text.startsWith('-');
  final digits = negative ? text.substring(1) : text;
  final parts = digits.split('.');
  final whole = parts[0].replaceAllMapped(
    RegExp(r'\B(?=(\d{3})+(?!\d))'),
    (_) => ',',
  );
  return '${negative ? '-' : ''}$whole${parts.length > 1 ? '.${parts[1]}' : ''}';
}

/// The status bar's aggregate text, in Excel's order. With an error in the
/// selection only Count is shown, since the numeric results are undefined.
String statusText(SelectionStats stats, Set<SelectionAggregate> shown) {
  final parts = <String>[];
  for (final aggregate in SelectionAggregate.values) {
    if (!shown.contains(aggregate)) continue;
    if (stats.hasError && aggregate != SelectionAggregate.count) continue;
    final numeric = stats.numericCount > 0;
    switch (aggregate) {
      case SelectionAggregate.average:
        if (numeric) {
          parts.add('Average: ${formatStatusNumber(stats.average!)}');
        }
      case SelectionAggregate.count:
        if (stats.count > 0) parts.add('Count: ${stats.count}');
      case SelectionAggregate.numericalCount:
        parts.add('Numerical Count: ${stats.numericCount}');
      case SelectionAggregate.minimum:
        if (numeric) parts.add('Min: ${formatStatusNumber(stats.minimum)}');
      case SelectionAggregate.maximum:
        if (numeric) parts.add('Max: ${formatStatusNumber(stats.maximum)}');
      case SelectionAggregate.sum:
        if (numeric) parts.add('Sum: ${formatStatusNumber(stats.sum)}');
    }
  }
  return parts.join('    ');
}

/// The status bar menu that turns aggregates on and off.
Future<Set<SelectionAggregate>?> chooseAggregates(
  BuildContext context,
  Offset position,
  Set<SelectionAggregate> shown,
) async {
  const labels = {
    SelectionAggregate.average: 'Average',
    SelectionAggregate.count: 'Count',
    SelectionAggregate.numericalCount: 'Numerical Count',
    SelectionAggregate.minimum: 'Minimum',
    SelectionAggregate.maximum: 'Maximum',
    SelectionAggregate.sum: 'Sum',
  };
  final chosen = await showMenu<SelectionAggregate>(
    context: context,
    position: RelativeRect.fromLTRB(
      position.dx,
      position.dy,
      position.dx,
      position.dy,
    ),
    items: [
      for (final aggregate in SelectionAggregate.values)
        CheckedPopupMenuItem(
          key: Key('status-aggregate-${aggregate.name}'),
          value: aggregate,
          checked: shown.contains(aggregate),
          child: Text(labels[aggregate]!),
        ),
    ],
  );
  if (chosen == null) return null;
  return shown.contains(chosen)
      ? ({...shown}..remove(chosen))
      : {...shown, chosen};
}
