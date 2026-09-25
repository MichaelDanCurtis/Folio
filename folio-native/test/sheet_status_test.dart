import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_status.dart';

void main() {
  final cells = {
    'A1': 'Label',
    'A2': '10',
    'A3': '=A2*2',
    'A4': 'TRUE',
    'A5': '=1/0',
    'B2': '2.5',
    'B3': '',
    'C9': '100',
  };
  final calculated = {
    'A2': 10,
    'A3': 20,
    'A4': true,
    'A5': '#DIV/0!',
    'B2': 2.5,
  };
  final formatted = {
    'A5': {'text': '#DIV/0!', 'isError': true},
  };
  SelectionStats? stats(String range, {Set<int> hiddenRows = const {}}) =>
      selectionStats(
        range: range,
        cells: cells,
        calculated: calculated,
        formatted: formatted,
        rowHidden: hiddenRows.contains,
        columnHidden: (_) => false,
      );

  test('a single cell has no aggregates', () {
    expect(stats('A2'), isNull);
    expect(stats('A2:A2'), isNull);
  });

  test('numbers, formulas, text and booleans', () {
    final s = stats('A1:B4')!;
    expect(s.count, 5, reason: 'non-empty cells, including text');
    expect(s.numericCount, 3, reason: '10, 20 and 2.5; not TRUE or text');
    expect(s.sum, 32.5);
    expect(s.average, closeTo(10.8333333333, 1e-9));
    expect(s.minimum, 2.5);
    expect(s.maximum, 20);
    expect(s.hasError, isFalse);
    expect(
      statusText(s, defaultAggregates),
      'Average: 10.8333333333    Count: 5    Sum: 32.5',
    );
    expect(
      statusText(s, SelectionAggregate.values.toSet()),
      'Average: 10.8333333333    Count: 5    Numerical Count: 3    '
      'Min: 2.5    Max: 20    Sum: 32.5',
    );
  });

  test('an error in the selection leaves only Count', () {
    final s = stats('A1:A5')!;
    expect(s.hasError, isTrue);
    expect(statusText(s, defaultAggregates), 'Count: 5');
  });

  test('hidden rows are left out', () {
    final s = stats('A2:A3', hiddenRows: {2})!;
    expect(s.sum, 10);
    expect(s.count, 1);
  });

  test('whole-column ranges walk only populated cells', () {
    final s = stats('C1:C1048576')!;
    expect(s.sum, 100);
    expect(s.count, 1);
  });

  test('status numbers group thousands and trim zeros', () {
    expect(formatStatusNumber(1234567.5), '1,234,567.5');
    expect(formatStatusNumber(-1000), '-1,000');
    expect(formatStatusNumber(0.1 + 0.2), '0.3');
    expect(formatStatusNumber(-0.0), '0');
    expect(formatStatusNumber(12), '12');
  });
}
