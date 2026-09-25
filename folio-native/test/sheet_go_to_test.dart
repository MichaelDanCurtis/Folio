import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_go_to.dart';

void main() {
  String describe(SheetTarget? t) => t?.toString() ?? 'none';

  test('cells, ranges, rows and columns', () {
    expect(describe(parseTarget('B7')), '!6,1:6,1');
    expect(describe(parseTarget(r'$c$3:a1')), '!0,0:2,2');
    expect(describe(parseTarget('C:E')), '!0,2:1048575,4 columns');
    expect(describe(parseTarget('6:4')), '!3,0:5,16383 rows');
    expect(describe(parseTarget('XFD1048576')), '!1048575,16383:1048575,16383');
    expect(describe(parseTarget('=D2')), '!1,3:1,3');
  });

  test('other sheets, quoted names included', () {
    expect(describe(parseTarget('Sheet2!A1')), 'Sheet2!0,0:0,0');
    expect(describe(parseTarget("'Q1 plan''s'!B2:B9")), "Q1 plan's!1,1:8,1");
  });

  test('things that are not references', () {
    for (final text in [
      '',
      'XFE1',
      'A0',
      'A1048577',
      'A',
      '3',
      'A1:',
      'hello',
    ]) {
      expect(parseTarget(text), isNull, reason: text);
    }
  });

  test('defined names: sheet scope first, then workbook', () {
    final names = [
      {'name': 'Rate', 'sheetId': null, 'refersTo': r'Inputs!$B$2'},
      {'name': 'Rate', 'sheetId': 's2', 'refersTo': r'Local!$C$3'},
      {'name': 'Block', 'sheetId': null, 'refersTo': r"'My data'!$A$1:$A$9"},
      {'name': 'TaxRate', 'sheetId': null, 'refersTo': '0.07'},
      {'name': 'Alias', 'sheetId': null, 'refersTo': 'Block'},
    ];
    expect(describe(parseTarget('rate', names: names)), 'Inputs!1,1:1,1');
    expect(
      describe(parseTarget('Rate', names: names, currentSheetId: 's2')),
      'Local!2,2:2,2',
    );
    expect(describe(parseTarget('Block', names: names)), 'My data!0,0:8,0');
    expect(describe(parseTarget('Alias', names: names)), 'My data!0,0:8,0');
    expect(parseTarget('TaxRate', names: names), isNull, reason: 'a value');
  });
}
