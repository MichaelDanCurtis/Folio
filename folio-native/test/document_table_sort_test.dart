import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_table_sort.dart';
import 'package:folio_native/src/document_table_sort_dialog.dart';
import 'package:folio_native/src/document_tables.dart';

Map<String, dynamic> sortTable(List<List<String>> rows) => {
  'type': 'table',
  'attrs': {
    'officeId': 'table',
    'columnWidths': List.filled(rows.first.length, 1800),
  },
  'content': [
    for (var r = 0; r < rows.length; r++)
      {
        'type': 'tableRow',
        'attrs': {
          'officeId': 'r$r',
          'cantSplit': r.isEven,
          'height': {'rule': 'atLeast', 'value': 300 + r},
        },
        'content': [
          for (var c = 0; c < rows[r].length; c++)
            {
              'type': 'tableCell',
              'attrs': {
                'officeId': 'r${r}c$c',
                'rowspan': 1,
                'colspan': 1,
                'fill': r.isEven ? 'FFF2CC' : 'EAF1FF',
              },
              'content': [
                {
                  'type': 'paragraph',
                  'attrs': {'officeId': 'p${r}c$c'},
                  'content': [
                    {
                      'type': 'text',
                      'text': rows[r][c],
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

// Fixture collation for platform-independent Dart tests. Native Foundation
// collation is exercised separately by verify_text_sort.swift.
Future<List<List<int>>> fixtureRanks(
  List<List<String>> values,
  String locale,
  bool sensitive,
) async => [
  for (final column in values)
    (() {
      final normalized = [
        for (final value in column) sensitive ? value : value.toLowerCase(),
      ];
      final distinct = normalized.toSet().toList()..sort();
      return [for (final value in normalized) distinct.indexOf(value)];
    })(),
];

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'three sort levels move full rich rows, keep headings, stable ties and blank-last',
    () async {
      final source = sortTable([
        ['Team', 'Count', 'Date'],
        ['Beta', '10', '2026-01-02'],
        ['alpha', '2', '2026-01-03'],
        ['Alpha', '2', '2026-01-01'],
        ['alpha', '2', '2026-01-01'],
        ['', '99', '2026-01-01'],
      ]);
      final before = jsonEncode(source);
      final result = await sortDocumentTable(
        source,
        headerRows: 1,
        ranker: fixtureRanks,
        keys: [
          const TableSortKey(0),
          const TableSortKey(1, type: TableSortType.number),
          const TableSortKey(2, type: TableSortType.date),
        ],
      );
      expect(result.order, [0, 3, 4, 2, 1, 5]);
      for (var i = 0; i < result.order.length; i++) {
        expect(result.node['content'][i], source['content'][result.order[i]]);
      }
      expect(result.node['attrs'], source['attrs']);
      expect(jsonEncode(source), before);
      result.node['content'][1]['content'][0]['content'][0]['content'][0]['text'] =
          'Changed copy';
      expect(jsonEncode(source), before);
      final descending = await sortDocumentTable(
        source,
        headerRows: 1,
        ranker: fixtureRanks,
        keys: [const TableSortKey(0, descending: true)],
      );
      expect(descending.order, [0, 1, 2, 3, 4, 5]);
    },
  );

  test(
    'localized exact numeric ordering includes large integers, exponents, currency and negatives',
    () async {
      final source = sortTable([
        ['9007199254740993'],
        ['9007199254740992'],
        ['-2.5'],
        ['0'],
        ['(1,234.50)'],
        ['\$1,000.25'],
        ['12%'],
        ['1e-3'],
        [''],
        ['-0'],
      ]);
      final result = await sortDocumentTable(
        source,
        keys: [const TableSortKey(0, type: TableSortType.number)],
      );
      expect(result.order, [4, 2, 3, 9, 7, 6, 5, 1, 0, 8]);
      final german = await sortDocumentTable(
        sortTable([
          ['1.234,50'],
          ['12,5'],
          ['-2,1'],
        ]),
        locale: 'de_DE',
        keys: [const TableSortKey(0, type: TableSortType.number)],
      );
      expect(german.order, [2, 1, 0]);
      final indian = await sortDocumentTable(
        sortTable([
          ['12,34,567'],
          ['99,999'],
        ]),
        locale: 'hi_IN',
        keys: [const TableSortKey(0, type: TableSortType.number)],
      );
      expect(indian.order, [1, 0]);
      for (final invalid in ['12,34', '1.2.3', 'NaN', '1,000.1x', '1e999999']) {
        await expectLater(
          sortDocumentTable(
            sortTable([
              [invalid],
              ['2'],
            ]),
            keys: [const TableSortKey(0, type: TableSortType.number)],
          ),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'date locale, strict invalid dates and selected range leave other rows untouched',
    () async {
      final source = sortTable([
        ['Before'],
        ['Heading'],
        ['02/03/2026'],
        ['03/02/2026'],
        ['After'],
      ]);
      final gb = await sortDocumentTable(
        source,
        firstRow: 1,
        lastRow: 3,
        headerRows: 1,
        locale: 'en_GB',
        keys: [const TableSortKey(0, type: TableSortType.date)],
      );
      expect(gb.order, [0, 1, 3, 2, 4]);
      final us = await sortDocumentTable(
        source,
        firstRow: 1,
        lastRow: 3,
        headerRows: 1,
        locale: 'en_US',
        keys: [const TableSortKey(0, type: TableSortType.date)],
      );
      expect(us.order, [0, 1, 2, 3, 4]);
      await expectLater(
        sortDocumentTable(
          sortTable([
            ['31/02/2026'],
            ['01/01/2026'],
          ]),
          locale: 'en_GB',
          keys: [const TableSortKey(0, type: TableSortType.date)],
        ),
        throwsFormatException,
      );
    },
  );

  test(
    'delimited keys retain empty fields and nested/non-key contents',
    () async {
      final source = sortTable([
        ['\t12', 'A'],
        ['Name\t2', 'B'],
        ['Other\t5', 'C'],
      ]);
      source['content'][0]['content'][1]['content'] = <Map>[
        ...source['content'][0]['content'][1]['content'],
        sortTable([
          ['Nested'],
        ]),
      ];
      final result = await sortDocumentTable(
        source,
        separator: '\t',
        keys: [const TableSortKey(0, type: TableSortType.number, field: 1)],
      );
      expect(result.order, [1, 2, 0]);
      expect(result.node['content'][2], source['content'][0]);
      final paragraphs = sortTable([
        ['unused'],
        ['unused'],
      ]);
      for (var row = 0; row < 2; row++) {
        paragraphs['content'][row]['content'][0]['content'] = <Map>[
          {
            'type': 'paragraph',
            'content': <Map>[
              {'type': 'text', 'text': 'First'},
              {'type': 'hardBreak'},
              {'type': 'text', 'text': row == 0 ? '999' : '1000'},
            ],
          },
          {
            'type': 'paragraph',
            'content': <Map>[
              {'type': 'text', 'text': row == 0 ? '2' : '1'},
            ],
          },
        ];
      }
      final secondParagraph = await sortDocumentTable(
        paragraphs,
        keys: [const TableSortKey(0, type: TableSortType.number, field: 1)],
      );
      expect(
        secondParagraph.order,
        [1, 0],
        reason: 'A manual break stays inside its paragraph field.',
      );
    },
  );

  test(
    'merged keys and vertical merges fail without mutation; excluded merged header is retained',
    () async {
      final source = sortTable([
        ['H', 'H2'],
        ['2', 'B'],
        ['1', 'A'],
      ]);
      final horizontal = DocumentTable(
        source,
      ).change('merge', const TableArea(0, 0, 0, 1));
      final result = await sortDocumentTable(
        horizontal,
        headerRows: 1,
        keys: [const TableSortKey(0, type: TableSortType.number)],
      );
      expect(result.order, [0, 2, 1]);
      await expectLater(
        sortDocumentTable(horizontal, keys: [const TableSortKey(0)]),
        throwsFormatException,
      );
      final vertical = DocumentTable(
        source,
      ).change('merge', const TableArea(1, 1, 2, 1));
      final before = jsonEncode(vertical);
      await expectLater(
        sortDocumentTable(
          vertical,
          headerRows: 1,
          keys: [const TableSortKey(0, type: TableSortType.number)],
        ),
        throwsFormatException,
      );
      expect(jsonEncode(vertical), before);
    },
  );

  test(
    'native text rank bridge validates responses and forwards locale and sensitivity',
    () async {
      const channel = MethodChannel('folio/text-sort');
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(channel, (call) async {
        expect(call.method, 'ranks');
        expect(call.arguments, {
          'values': [
            ['z', 'ä'],
          ],
          'locale': 'sv_SE',
          'caseSensitive': true,
        });
        return [
          [0, 1],
        ];
      });
      addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
      expect(
        await nativeTableTextRanks(
          [
            ['z', 'ä'],
          ],
          'sv_SE',
          true,
        ),
        [
          [0, 1],
        ],
      );
      messenger.setMockMethodCallHandler(
        channel,
        (_) async => [
          [0],
        ],
      );
      await expectLater(
        nativeTableTextRanks(
          [
            ['a', 'b'],
          ],
          'en_US',
          false,
        ),
        throwsFormatException,
      );
      messenger.setMockMethodCallHandler(
        channel,
        (_) async => [
          [0, 99],
        ],
      );
      await expectLater(
        nativeTableTextRanks(
          [
            ['a', 'b'],
          ],
          'en_US',
          false,
        ),
        throwsFormatException,
      );
    },
  );

  testWidgets(
    'late preview result cannot replace newer settings and Cancel never mutates',
    (t) async {
      t.view.physicalSize = const Size(1100, 900);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      final calls = <Completer<List<List<int>>>>[];
      final source = sortTable([
        ['B'],
        ['A'],
      ]);
      final before = jsonEncode(source);
      await t.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => TextButton(
              onPressed: () => showDialog<TableSortResult>(
                context: context,
                builder: (_) => DocumentTableSortDialog(
                  table: DocumentTable(source),
                  selection: const TableArea(0, 0, 0, 0),
                  ranker: (values, locale, sensitive) {
                    final c = Completer<List<List<int>>>();
                    calls.add(c);
                    return c.future;
                  },
                ),
              ),
              child: const Text('Open'),
            ),
          ),
        ),
      );
      await t.tap(find.text('Open'));
      await t.pump();
      await t.pump(const Duration(milliseconds: 300));
      expect(calls.length, 1);
      await t.tap(find.byKey(const Key('sort-case-sensitive')));
      await t.pump();
      expect(calls.length, 2);
      calls[1].complete([
        [0, 1],
      ]);
      await t.pumpAndSettle();
      expect(
        t.getTopLeft(find.byKey(const Key('sort-preview-row-0'))).dy,
        lessThan(t.getTopLeft(find.byKey(const Key('sort-preview-row-1'))).dy),
      );
      calls[0].complete([
        [1, 0],
      ]);
      await t.pumpAndSettle();
      expect(
        t.getTopLeft(find.byKey(const Key('sort-preview-row-0'))).dy,
        lessThan(t.getTopLeft(find.byKey(const Key('sort-preview-row-1'))).dy),
      );
      await t.tap(find.text('Cancel'));
      await t.pumpAndSettle();
      expect(jsonEncode(source), before);
      expect(t.takeException(), isNull);
    },
  );
}
