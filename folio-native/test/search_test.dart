import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/search.dart';

Map<String, dynamic> paragraph(String text) => {
  'type': 'paragraph',
  'content': [
    {'type': 'text', 'text': text},
  ],
};
Map<String, dynamic> doc(List nodes) => {'type': 'doc', 'content': nodes};

void main() {
  test('literal matching keeps UTF-16 offsets and Unicode word boundaries', () {
    const text = '😀 Cat scatter cat café cafeteria cafe\u0301 Cat';
    final matches = textMatches(
      text,
      'cat',
      const SearchOptions(wholeWords: true),
    ).toList();
    expect(matches.map((m) => text.substring(m.start, m.end)), [
      'Cat',
      'cat',
      'Cat',
    ]);
    expect(matches.first.start, 3);
    expect(
      textMatches(
        text,
        'cat',
        const SearchOptions(matchCase: true, wholeWords: true),
      ).length,
      1,
    );
    expect(
      textMatches(text, 'cafe', const SearchOptions(wholeWords: true)),
      isEmpty,
    );
    expect(textMatches('a.b a?b a*b', 'a.b', const SearchOptions()).length, 1);
    expect(
      textMatches('école', 'é', const SearchOptions(wholeWords: true)),
      isEmpty,
    );
  });

  test(
    'document replacement crosses styled runs, preserves outside styles and block metadata',
    () {
      final original = doc([
        {
          'type': 'heading',
          'attrs': {'level': 2, 'officeId': 'p1', 'textAlign': 'center'},
          'content': [
            {
              'type': 'text',
              'text': '😀 Red ',
              'marks': [
                {'type': 'bold'},
              ],
            },
            {
              'type': 'text',
              'text': 'fox and fox.',
              'marks': [
                {'type': 'italic'},
              ],
            },
          ],
        },
        paragraph('Red fox'),
      ]);
      final index = DocumentSearchIndex(original);
      final hits = index.find('Red fox', const SearchOptions()).hits;
      expect(hits.length, 2);
      expect(hits.first.documentStart, 3);
      expect(hits.last.documentStart, '😀 Red fox and fox.\n'.length);
      final changed = index.replace(original, hits, 'Blue bird');
      expect(nodeText(changed), '😀 Blue bird and fox.\nBlue bird');
      expect(changed['content'][0]['attrs'], original['content'][0]['attrs']);
      expect(changed['content'][0]['content'][1]['marks'], [
        {'type': 'bold'},
      ]);
      expect(changed['content'][0]['content'].last['marks'], [
        {'type': 'italic'},
      ]);
      expect(nodeText(original), '😀 Red fox and fox.\nRed fox');
      expect(index.replace(original, hits, 'Red fox'), original);
      expect(() => index.replace(changed, hits, 'new'), throwsStateError);
      expect(
        () => index.replace(original, [hits.first, hits.first], 'new'),
        throwsStateError,
      );
    },
  );

  test(
    'lists and tables are searchable without flattening embeds or linked data',
    () {
      final original = doc([
        paragraph('first'),
        {
          'type': 'bulletList',
          'content': [
            {
              'type': 'listItem',
              'content': [paragraph('fox')],
            },
            {
              'type': 'listItem',
              'content': [paragraph('fox twice fox')],
            },
          ],
        },
        {
          'type': 'table',
          'attrs': {'officeId': 't1'},
          'content': [
            {
              'type': 'tableRow',
              'content': [
                {
                  'type': 'tableCell',
                  'attrs': {'colspan': 1},
                  'content': [paragraph('fox')],
                },
              ],
            },
          ],
        },
        {
          'type': 'workbookBlock',
          'attrs': {'title': 'fox', 'sheetId': 's1', 'range': 'A1:B2'},
        },
        paragraph('fox last'),
      ]);
      final index = DocumentSearchIndex(original);
      final hits = index.find('fox', const SearchOptions()).hits;
      expect(hits.length, 5);
      expect(hits.map((h) => h.documentStart), [6, 10, 20, 24, 28]);
      expect(hits[3].block!.embedded, isTrue);
      expect(hits[3].documentEnd, 25);
      final replaced = index.replace(original, hits, 'wolf');
      expect(replaced['content'][2]['content'][0]['content'][0]['attrs'], {
        'colspan': 1,
      });
      expect(nodeText(replaced['content'][2]), 'wolf');
      expect(replaced['content'][3], original['content'][3]);
      final roundTrip = deltaToDocument(documentToDelta(replaced));
      expect(nodeText(roundTrip), nodeText(replaced));
      expect(roundTrip['content'][2], replaced['content'][2]);
      expect(roundTrip['content'][3], replaced['content'][3]);
      expect(
        documentToDelta(roundTrip).toJson(),
        documentToDelta(replaced).toJson(),
      );
    },
  );

  test(
    'paragraph boundaries and hard breaks cannot be accidentally removed',
    () {
      final original = doc([
        paragraph('a'),
        paragraph('b'),
        {
          'type': 'paragraph',
          'content': [
            {'type': 'text', 'text': 'fox'},
            {'type': 'hardBreak'},
            {'type': 'text', 'text': 'dog'},
          ],
        },
      ]);
      final index = DocumentSearchIndex(original);
      expect(index.find('a\nb', const SearchOptions()).hits, isEmpty);
      expect(index.find('fox\ndog', const SearchOptions()).hits, isEmpty);
      final changed = index.replace(
        original,
        index.find('fox', const SearchOptions()).hits,
        'cat',
      );
      expect(changed['content'][2]['content'][1]['type'], 'hardBreak');
      expect(
        () => index.replace(
          original,
          index.find('fox', const SearchOptions()).hits,
          'cat\ndog',
        ),
        throwsStateError,
      );
    },
  );

  final sheets = [
    {
      'id': 's1',
      'name': 'Budget',
      'cells': {'A1': 'north north', 'B1': '=A2*2', 'A2': '25', 'B2': "'0025"},
      'cellStyles': {
        'B1': {'numFmt': r'$#,##0.00'},
      },
    },
    {
      'id': 's2',
      'name': 'Next year',
      'cells': {'C5000': 'North', 'A1': '=Budget!A2*2'},
    },
  ];
  final calculated = {
    's1': {'B1': 50, 'A2': 25, 'B2': '0025'},
    's2': {'A1': 50},
  };
  test(
    'workbook scopes, row/column ordering and displayed formula values differ',
    () {
      expect(
        findInWorkbook(
          sheets,
          calculated,
          's1',
          'north',
          const SearchOptions(),
        ).hits.length,
        1,
      );
      expect(
        findInWorkbook(
          sheets,
          calculated,
          's1',
          'north',
          const SearchOptions(allSheets: true),
        ).hits.length,
        2,
      );
      expect(
        findInWorkbook(
          sheets,
          calculated,
          's1',
          '25',
          const SearchOptions(entireCell: true),
        ).hits.single.address,
        'A2',
      );
      expect(
        findInWorkbook(
          sheets,
          calculated,
          's1',
          r'$50.00',
          const SearchOptions(values: true),
          formatted: const {
            's1': {
              'B1': {'text': r'$50.00'},
            },
          },
        ).hits.single.address,
        'B1',
      );
      expect(
        findInWorkbook(
          sheets,
          calculated,
          's1',
          'A2',
          const SearchOptions(),
        ).hits.single.address,
        'B1',
      );
      expect(
        findInWorkbook(
          sheets,
          calculated,
          's1',
          'A2',
          const SearchOptions(values: true),
        ).hits,
        isEmpty,
      );
      expect(
        findInWorkbook(
          sheets,
          calculated,
          's1',
          '2',
          const SearchOptions(),
        ).hits.map((h) => h.address),
        ['B1', 'A2', 'B2'],
      );
      expect(
        findInWorkbook(
          sheets,
          calculated,
          's1',
          '2',
          const SearchOptions(byColumns: true),
        ).hits.map((h) => h.address),
        ['A2', 'B1', 'B2'],
      );
    },
  );

  test(
    'replacement changes every occurrence per cell and retains forced text typing',
    () {
      const options = SearchOptions(allSheets: true);
      final hits = findInWorkbook(
        sheets,
        calculated,
        's1',
        'north',
        options,
      ).hits;
      final edits = workbookReplacements(
        sheets,
        hits,
        'north',
        'South',
        options,
      );
      expect(edits.map((e) => e['value']), ['South South', 'South']);
      final zeros = findInWorkbook(
        sheets,
        calculated,
        's1',
        '25',
        options,
      ).hits;
      expect(
        workbookReplacements(sheets, zeros, '25', '36', options).last['value'],
        "'0036",
      );
      final changed = jsonDecode(jsonEncode(sheets));
      changed[0]['cells']['A1'] = 'newer content';
      expect(
        () => workbookReplacements(changed, hits, 'north', 'South', options),
        throwsStateError,
      );
      expect(
        () => workbookReplacements(
          sheets,
          hits,
          'north',
          'South',
          const SearchOptions(values: true),
        ),
        throwsStateError,
      );
    },
  );

  test('oversized result sets explicitly report truncation', () {
    final index = DocumentSearchIndex(
      doc([paragraph(List.filled(10001, 'x').join(' '))]),
    );
    final found = index.find('x', const SearchOptions());
    expect(found.hits.length, 10000);
    expect(found.truncated, isTrue);
  });
}
