import 'package:flutter/services.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:intl/intl.dart';
import 'document_codec.dart';
import 'document_tables.dart';

enum TableSortType { text, number, date }

class TableSortKey {
  final int column;
  final TableSortType type;
  final bool descending;
  final int field;
  const TableSortKey(
    this.column, {
    this.type = TableSortType.text,
    this.descending = false,
    this.field = 0,
  });
}

typedef TableTextRanker =
    Future<List<List<int>>> Function(
      List<List<String>> values,
      String locale,
      bool caseSensitive,
    );

Future<List<List<int>>> nativeTableTextRanks(
  List<List<String>> values,
  String locale,
  bool caseSensitive,
) async {
  try {
    final result = await const MethodChannel('folio/text-sort')
        .invokeListMethod<dynamic>('ranks', {
          'values': values,
          'locale': locale,
          'caseSensitive': caseSensitive,
        });
    if (result == null || result.length != values.length) {
      throw const FormatException(
        'Text sorting returned an incomplete result.',
      );
    }
    final ranks = [for (final column in result) List<int>.from(column as List)];
    for (var i = 0; i < ranks.length; i++) {
      if (ranks[i].length != values[i].length ||
          ranks[i].any((n) => n < 0 || n >= values[i].length)) {
        throw const FormatException('Text sorting returned an invalid result.');
      }
    }
    return ranks;
  } on MissingPluginException {
    throw const FormatException(
      'Language-aware text sorting is not available on this platform yet.',
    );
  } on PlatformException catch (e) {
    throw FormatException(e.message ?? 'Text sorting is unavailable.');
  }
}

/// Decimal comparison avoids rounding large integers through a floating value.
class _Decimal implements Comparable<_Decimal> {
  final bool negative;
  final String digits;
  final int scale;
  const _Decimal(this.negative, this.digits, this.scale);
  @override
  int compareTo(_Decimal other) {
    if (digits == '0' && other.digits == '0') return 0;
    if (negative != other.negative) return negative ? -1 : 1;
    if (digits == '0') return -1;
    if (other.digits == '0') return 1;
    var result = (digits.length + scale).compareTo(
      other.digits.length + other.scale,
    );
    if (result == 0) {
      final width = digits.length > other.digits.length
          ? digits.length
          : other.digits.length;
      result = digits
          .padRight(width, '0')
          .compareTo(other.digits.padRight(width, '0'));
    }
    return negative ? -result : result;
  }
}

_Decimal _number(String text, String locale) {
  final symbols = NumberFormat.decimalPattern(locale).symbols;
  var value = text.trim().replaceAll('\u2212', '-');
  final zero = symbols.ZERO_DIGIT.runes.single;
  value = String.fromCharCodes(
    value.runes.map((c) => c >= zero && c <= zero + 9 ? c - zero + 48 : c),
  );
  value = value
      .replaceAll(symbols.MINUS_SIGN, '-')
      .replaceAll(symbols.PLUS_SIGN, '+');
  if (value.startsWith('(') && value.endsWith(')')) {
    value = '-${value.substring(1, value.length - 1)}';
  }
  value = value.replaceFirstMapped(
    RegExp(r'^([+-]?)\s*[\$€£¥]\s*'),
    (m) => m[1]!,
  );
  final percent = value.endsWith('%');
  if (percent) value = value.substring(0, value.length - 1).trim();
  final exponentParts = value.split(RegExp('[eE]'));
  if (exponentParts.length > 2) throw const FormatException('Invalid number');
  final exponent = exponentParts.length == 1
      ? 0
      : int.tryParse(exponentParts[1]);
  if (exponent == null || exponent.abs() > 10000) {
    throw const FormatException('Invalid exponent');
  }
  var mantissa = exponentParts[0];
  final negative = mantissa.startsWith('-');
  if (mantissa.startsWith('-') || mantissa.startsWith('+')) {
    mantissa = mantissa.substring(1);
  }
  final parts = mantissa.split(symbols.DECIMAL_SEP);
  if (parts.length > 2) throw const FormatException('Invalid decimal');
  var whole = parts[0];
  final grouping = symbols.GROUP_SEP;
  if (grouping == '\u202f' || grouping == '\u00a0') {
    whole = whole.replaceAll(' ', grouping).replaceAll('\u00a0', grouping);
  }
  final groups = whole.split(grouping);
  if (groups.length > 1) {
    final indian = locale.endsWith('_IN');
    if (groups.first.isEmpty ||
        groups.first.length > (indian && groups.length > 2 ? 2 : 3) ||
        groups.last.length != 3 ||
        groups
            .skip(1)
            .take(groups.length - 2)
            .any((g) => g.length != (indian ? 2 : 3))) {
      throw const FormatException('Invalid grouping');
    }
  }
  whole = groups.join();
  final fraction = parts.length == 2 ? parts[1] : '';
  final digits = '$whole$fraction';
  if (digits.isEmpty || !RegExp(r'^\d+$').hasMatch(digits)) {
    throw const FormatException('Invalid number');
  }
  final significant = digits.replaceFirst(RegExp(r'^0+'), '');
  return _Decimal(
    negative && significant.isNotEmpty,
    significant.isEmpty ? '0' : significant,
    exponent - fraction.length - (percent ? 2 : 0),
  );
}

int _date(String text, String locale) {
  for (final format in [
    DateFormat('yyyy-MM-dd HH:mm:ss'),
    DateFormat('yyyy-MM-dd'),
    DateFormat.yMd(locale),
    DateFormat.yMMMd(locale),
    DateFormat.yMMMMd(locale),
  ]) {
    try {
      return format.parseStrict(text, true).microsecondsSinceEpoch;
    } on FormatException {
      /* Try the next complete format. */
    }
  }
  throw const FormatException('Invalid date');
}

String tableSortCellText(Map cell) =>
    (cell['content'] as List? ?? []).map((n) => nodeText(n as Map)).join('\n');

class TableSortResult {
  final Map<String, dynamic> node;

  /// New row index -> original row index, including the retained range edges.
  final List<int> order;
  const TableSortResult(this.node, this.order);
}

Future<TableSortResult> sortDocumentTable(
  Map source, {
  required List<TableSortKey> keys,
  int firstRow = 0,
  int? lastRow,
  int headerRows = 0,
  String locale = 'en_US',
  bool caseSensitive = false,
  String separator = '\n',
  TableTextRanker ranker = nativeTableTextRanks,
}) async {
  final table = DocumentTable(source);
  final end = lastRow ?? table.rows.length - 1;
  if (firstRow < 0 ||
      end < firstRow ||
      end >= table.rows.length ||
      headerRows < 0 ||
      headerRows > end - firstRow + 1) {
    throw const FormatException('Choose a valid row range and heading count.');
  }
  if (keys.isEmpty ||
      keys.length > 3 ||
      keys.any(
        (key) => key.column < 0 || key.column >= table.columns || key.field < 0,
      ) ||
      separator.runes.length != 1) {
    throw const FormatException(
      'Choose one to three sort levels and a single field separator.',
    );
  }
  final start = firstRow + headerRows;
  if (end - start < 1) {
    throw const FormatException('Choose at least two data rows to sort.');
  }
  if (table.cells.any((c) => c.rows > 1 && c.bottom >= start && c.row <= end)) {
    throw const FormatException(
      'Split vertically merged cells in the data rows before sorting.',
    );
  }
  await initializeDateFormatting(locale);
  final strings = <List<String>>[];
  final values = <List<Comparable?>>[];
  for (final key in keys) {
    final texts = <String>[];
    final parsed = <Comparable?>[];
    for (var row = start; row <= end; row++) {
      final cell = table.at(row, key.column);
      if (cell.cols != 1) {
        throw FormatException(
          'Column ${key.column + 1} contains a merged cell in row ${row + 1}. Choose an unmerged sort column.',
        );
      }
      final fields = separator == '\n'
          ? [
              for (final block in cell.cell['content'] as List? ?? [])
                nodeText(block as Map),
            ]
          : tableSortCellText(cell.cell).split(separator);
      final text = key.field < fields.length ? fields[key.field].trim() : '';
      texts.add(text);
      try {
        parsed.add(
          text.isEmpty
              ? null
              : switch (key.type) {
                  TableSortType.text => 0,
                  TableSortType.number => _number(text, locale),
                  TableSortType.date => _date(text, locale),
                },
        );
      } on FormatException {
        throw FormatException(
          'Row ${row + 1}, column ${key.column + 1} is not a valid ${key.type.name} for $locale: “$text”.',
        );
      }
    }
    strings.add(texts);
    values.add(parsed);
  }
  final textKeys = [
    for (var i = 0; i < keys.length; i++)
      if (keys[i].type == TableSortType.text) i,
  ];
  if (textKeys.isNotEmpty) {
    final ranks = await ranker(
      [for (final i in textKeys) strings[i]],
      locale,
      caseSensitive,
    );
    if (ranks.length != textKeys.length) {
      throw const FormatException('Incomplete text sort result.');
    }
    for (var k = 0; k < textKeys.length; k++) {
      final i = textKeys[k];
      if (ranks[k].length != strings[i].length) {
        throw const FormatException('Incomplete text sort result.');
      }
      for (var row = 0; row < ranks[k].length; row++) {
        if (strings[i][row].isNotEmpty) values[i][row] = ranks[k][row];
      }
    }
  }
  final sorted = List.generate(end - start + 1, (i) => i);
  sorted.sort((a, b) {
    for (var k = 0; k < keys.length; k++) {
      final x = values[k][a], y = values[k][b];
      // Empty fields stay last in both directions.
      if (x == null || y == null) {
        if (x != y) return x == null ? 1 : -1;
        continue;
      }
      final order = x.compareTo(y);
      if (order != 0) return keys[k].descending ? -order : order;
    }
    return a.compareTo(b);
  });
  final order = [
    for (var i = 0; i < start; i++) i,
    ...sorted.map((i) => i + start),
    for (var i = end + 1; i < table.rows.length; i++) i,
  ];
  final next = copyTableNode(table.node);
  next['content'] = [for (final i in order) copyTableNode(table.rows[i])];
  DocumentTable(next);
  return TableSortResult(next, order);
}
