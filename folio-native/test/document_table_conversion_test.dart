import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_table_conversion.dart';
import 'package:folio_native/src/document_tables.dart';

String plain(Delta d) =>
    d.toList().map((op) => op.data is String ? op.data : 'TABLE').join();
void main() {
  Map<String, dynamic> cell(String text, {int rows = 1}) => {
    'type': 'tableCell',
    'attrs': {'rowspan': rows, 'colspan': 1},
    'content': [
      {
        'type': 'paragraph',
        'content': [
          {'type': 'text', 'text': text},
        ],
      },
    ],
  };
  Map<String, dynamic> row(List<Map<String, dynamic>> cells) => {
    'type': 'tableRow',
    'content': cells,
  };

  test(
    'typing after a partial conversion at document end keeps both tables',
    () {
      final table = {
        'type': 'table',
        'content': [
          for (var r = 0; r < 3; r++) row([cell('Row $r')]),
        ],
      };
      final controller = q.QuillController(
        document: q.Document.fromDelta(
          documentToDelta({
            'type': 'doc',
            'content': [table],
          }),
        ),
        selection: const TextSelection.collapsed(offset: 0),
      );
      final result = TableRowsConversion(table, firstRow: 1, lastRow: 1);
      final edit = TableConversionEdit(0, 2, documentToDelta(result.document));
      controller.compose(
        edit.change,
        const TextSelection.collapsed(offset: 0),
        q.ChangeSource.local,
      );
      final caret = edit.caret.clamp(0, controller.document.length - 1);
      controller.updateSelection(
        TextSelection.collapsed(offset: caret),
        q.ChangeSource.local,
      );
      controller.replaceText(
        caret,
        0,
        'X',
        TextSelection.collapsed(offset: caret + 1),
      );
      final document = deltaToDocument(controller.document.toDelta());
      final remaining = (document['content'] as List)
          .where((n) => n['type'] == 'table')
          .toList();
      expect(remaining.length, 2);
      expect(nodeText(remaining.first), 'Row 0');
      expect(nodeText(remaining.last), 'Row 2');
      expect(nodeText(document), contains('X'));
      controller.dispose();
    },
  );

  test('selected rows become text between intact table fragments', () {
    final Map<String, dynamic> source = {
      'type': 'table',
      'attrs': {
        'officeId': 'word-1',
        'columnWidths': [1440, 2160],
        'layout': 'fixed',
        'alignment': 'right',
      },
      'content': [
        for (var r = 0; r < 5; r++)
          {
            ...row([cell('R${r + 1}A'), cell('R${r + 1}B')]),
            'attrs': {
              'officeId': 'word-1-$r',
              'height': {'rule': 'exact', 'value': 400},
            },
          },
      ],
    };
    final original = jsonEncode(source);
    final result = TableRowsConversion(source, firstRow: 1, lastRow: 2);
    expect(nodeText(result.converted), 'R2A\tR2B\nR3A\tR3B');
    expect(result.document['content'].map((n) => n['type']), [
      'table',
      'paragraph',
      'paragraph',
      'table',
    ]);
    expect(result.before!['content'], [source['content']![0]]);
    expect(result.after!['content'], source['content']!.sublist(3));
    expect(result.before!['attrs'], source['attrs']);
    expect(result.after!['attrs'], source['attrs']);
    expect(DocumentTable(result.after!).rows.length, 2);
    expect(result.expanded, isFalse);
    expect(jsonEncode(source), original);
  });

  test(
    'row conversion expands across chained vertical merges without losing content',
    () {
      final source = {
        'type': 'table',
        'content': [
          row([cell('A0'), cell('B0'), cell('C0', rows: 2)]),
          row([cell('A1', rows: 2), cell('B1')]),
          row([cell('B2'), cell('C2', rows: 2)]),
          row([cell('A3'), cell('B3')]),
        ],
      };
      final result = TableRowsConversion(source, firstRow: 1, lastRow: 1);
      expect(result.rows.top, 0);
      expect(result.rows.bottom, 3);
      expect(result.before, isNull);
      expect(result.after, isNull);
      expect(result.expanded, isTrue);
      expect(
        nodeText(result.converted),
        'A0\tB0\tC0\nA1\tB1\t\n\tB2\tC2\nA3\tB3\t',
      );
    },
  );

  test(
    'first, last and all-row conversion handle edges and reject invalid ranges',
    () {
      final source = {
        'type': 'table',
        'content': [
          row([cell('one')]),
          row([cell('two')]),
          row([cell('three')]),
        ],
      };
      final first = TableRowsConversion(source, firstRow: 0, lastRow: 0);
      expect(first.before, isNull);
      expect(DocumentTable(first.after!).rows.length, 2);
      final last = TableRowsConversion(source, firstRow: 2, lastRow: 2);
      expect(last.after, isNull);
      expect(DocumentTable(last.before!).rows.length, 2);
      final all = TableRowsConversion(source, firstRow: 0, lastRow: 2);
      expect(all.document, tableToText(source));
      for (final (a, b) in [(-1, 1), (2, 1), (0, 3)]) {
        expect(
          () => TableRowsConversion(source, firstRow: a, lastRow: b),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'conversion retains rich runs, partial paragraph boundaries and source metadata',
    () {
      final source = Delta()
        ..insert('Before\nPrefix ')
        ..insert('Name', {'bold': true})
        ..insert('\tAmount')
        ..insert('\n', {'align': 'right', 'token': '{"officeId":"word-1"}'})
        ..insert('Research', {'italic': true, 'link': 'https://example.org'})
        ..insert('\t2400 suffix\nAfter\n');
      final start = plain(source).indexOf('Name');
      final end = plain(source).indexOf(' suffix');
      final selected = TextTableSelection(source, start, end);
      final table = selected.table(columnWidth: 72);
      final grid = DocumentTable(table);
      expect(grid.columns, 2);
      expect(grid.rows.length, 2);
      expect(grid.node['attrs']['columnWidths'], [1440.0, 1440.0]);
      final first = grid.at(0, 0).cell['content'][0];
      expect(first['attrs'], {'officeId': 'word-1', 'textAlign': 'right'});
      expect(first['content'][0]['marks'], [
        {'type': 'bold'},
      ]);
      expect(
        grid.at(1, 0).cell['content'][0]['content'][0]['marks'],
        contains(equals({'type': 'italic'})),
      );
      final edit = selected.replaceWith(table);
      final doc = q.Document.fromDelta(source);
      doc.compose(edit.change, q.ChangeSource.local);
      expect(plain(doc.toDelta()), 'Before\nPrefix \nTABLE\n suffix\nAfter\n');
      final unconverted = tableToText(table);
      expect(nodeText(unconverted), 'Name\tAmount\nResearch\t2400');
      expect(
        unconverted['content'][1]['content'][0]['marks'],
        contains(
          equals({
            'type': 'link',
            'attrs': {'href': 'https://example.org'},
          }),
        ),
      );
      doc.close();
    },
  );

  test(
    'all selection endings keep a legal document without an extra paragraph',
    () {
      for (final sample in ['A\tB\n', 'Before\nA\tB\nAfter\n']) {
        final source = Delta()..insert(sample);
        final start = sample.indexOf('A\tB');
        for (final includeMark in [false, true]) {
          final selected = TextTableSelection(
            source,
            start,
            start + 3 + (includeMark ? 1 : 0),
          );
          final edit = selected.replaceWith(selected.table());
          final doc = q.Document.fromDelta(source);
          doc.compose(edit.change, q.ChangeSource.local);
          expect(
            plain(doc.toDelta()),
            sample.replaceFirst('A\tB\n', 'TABLE\n'),
          );
          expect(
            deltaToDocument(
              doc.toDelta(),
            )['content'].where((n) => n['type'] == 'table').length,
            1,
          );
          doc.close();
        }
      }
    },
  );

  test(
    'custom separators across marks, empty fields, ragged rows and column wrapping',
    () {
      final d = Delta()
        ..insert('a;')
        ..insert(';b', {'bold': true})
        ..insert(';\nc\n');
      final selected = TextTableSelection(d, 0, plain(d).length);
      final grid = DocumentTable(selected.table(separator: ';'));
      expect(grid.columns, 4);
      expect(
        [for (var c = 0; c < 4; c++) nodeText(grid.at(0, c).cell)],
        ['a', '', 'b', ''],
      );
      expect(nodeText(grid.at(1, 3).cell), '');
      final wrapped = DocumentTable(selected.table(separator: ';', columns: 2));
      expect(wrapped.rows.length, 3);
      expect(nodeText(wrapped.at(1, 0).cell), 'b');
      final paras = TextTableSelection(
        Delta()..insert('one\ntwo\nthree\n'),
        0,
        14,
      );
      final grouped = DocumentTable(paras.table(separator: '\n', columns: 2));
      expect(grouped.rows.length, 2);
      expect(nodeText(grouped.at(0, 1).cell), 'two');
      expect(nodeText(grouped.at(1, 1).cell), '');
      final emoji = TextTableSelection(Delta()..insert('a🟢b\n'), 0, 5);
      expect(DocumentTable(emoji.table(separator: '🟢')).columns, 2);
    },
  );

  test(
    'merged cells are emitted once, multi-paragraph content and nested objects survive',
    () {
      Map p(String text) => {
        'type': 'paragraph',
        'content': [
          {'type': 'text', 'text': text},
        ],
      };
      Map cell(List content, [Map attrs = const {}]) => {
        'type': 'tableCell',
        'attrs': attrs,
        'content': content,
      };
      final nested = {
        'type': 'table',
        'content': [
          {
            'type': 'tableRow',
            'content': [
              cell([p('Nested')]),
            ],
          },
        ],
      };
      final opaque = {
        'type': 'officeBlock',
        'attrs': {'officeId': 'opaque'},
      };
      final table = {
        'type': 'table',
        'content': [
          {
            'type': 'tableRow',
            'content': [
              cell([p('A'), p('B')], {'rowspan': 2}),
              cell([nested, opaque, p('C')]),
            ],
          },
          {
            'type': 'tableRow',
            'content': [
              cell([p('D')]),
            ],
          },
        ],
      };
      final before = jsonEncode(table);
      final text = tableToText(table);
      expect(nodeText(text), 'A\nB\tNested\n\nC\n\tD');
      expect(text['content'], contains(equals(opaque)));
      final kept = tableToText(table, convertNested: false);
      expect(kept['content'], contains(equals(nested)));
      final separate = tableToText(table, separator: '\n');
      expect(separate['content'].length, 7);
      expect(jsonEncode(table), before);
    },
  );

  test('invalid selections and settings cannot discard nontext objects', () {
    final d = Delta()
      ..insert('A')
      ..insert({'folioNode': '{}'})
      ..insert('\n');
    expect(() => TextTableSelection(d, 0, 2), throwsFormatException);
    expect(() => TextTableSelection(d, 0, 0), throwsFormatException);
    final selection = TextTableSelection(Delta()..insert('A\n'), 0, 1);
    for (final s in ['', '::', '\r']) {
      expect(() => selection.table(separator: s), throwsFormatException);
    }
    expect(() => selection.table(columns: 0), throwsFormatException);
    expect(
      () => selection.table(columnWidth: double.nan),
      throwsFormatException,
    );
  });
}
