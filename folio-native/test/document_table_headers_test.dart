import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_page_layout.dart';
import 'package:folio_native/src/document_fields.dart';
import 'package:folio_native/src/document_tables.dart';
import 'document_page_layout_test.dart' as f;
import 'document_column_balance_test.dart' as balance;

Map cell(String text, {int rows = 1, int cols = 1}) => {
  'type': 'tableCell',
  'attrs': {'rowspan': rows, 'colspan': cols, 'fill': 'D9EAF7'},
  'content': [f.paragraph(text)],
};
Map table({int headers = 2, int bodyRows = 23}) => {
  'type': 'table',
  'attrs': {
    'columnWidths': [1800, 1800],
  },
  'content': [
    for (var i = 0; i < headers + bodyRows; i++)
      {
        'type': 'tableRow',
        'attrs': {'header': i < headers},
        'content': [
          cell(i < headers ? 'H$i' : 'R${i - headers}'),
          cell(i < headers ? 'Title$i' : 'V${i - headers}'),
        ],
      },
  ],
};
DocumentPageLayout layout(Map node, {List? content, List? sections}) {
  final result = DocumentPageLayout.build(
    f.snapshot(content ?? [node], sections ?? [f.section(0, 0)]),
  );
  addTearDown(result.dispose);
  expect(result.converged, true);
  return result;
}

void bodyExactlyOnce(DocumentPageLayout result, int count) {
  final values = result.pages
      .expand((p) => p.content.where((c) => c.area == 'body'))
      .map((p) => p.text)
      .join('\n')
      .replaceAll(RegExp(r'\s+'), '');
  expect(
    RegExp(r'R\d+').allMatches(values).map((m) => m[0]).toList(),
    List.generate(count, (i) => 'R$i'),
  );
  for (final page in result.pages) {
    for (final item in page.content.where(
      (c) => ['body', 'tableHeader'].contains(c.area),
    )) {
      expect(item.offset.dy, greaterThanOrEqualTo(page.top - .01));
      expect(
        item.offset.dy + item.height,
        lessThanOrEqualTo(page.bottom + .01),
      );
    }
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test('consecutive header rows repeat without duplicating source or body', () {
    final node = table(), before = jsonEncode(table());
    final result = layout(node);
    expect(result.pages.length, greaterThan(3));
    bodyExactlyOnce(result, 23);
    expect(result.notices, isEmpty);
    for (final page in result.pages) {
      expect(RegExp(r'\bH0\b').allMatches(page.visibleText).length, 1);
      expect(RegExp(r'\bH1\b').allMatches(page.visibleText).length, 1);
      final first = page.content
          .where((p) => ['body', 'tableHeader'].contains(p.area))
          .first;
      expect(first.offset.dy, page.top);
      if (page.physicalNumber > 1) {
        expect(first.area, 'tableHeader');
        expect(first.text, 'H0\nTitle0\nH1\nTitle1');
      }
    }
    expect(jsonEncode(node), before);
  });
  test('orphan flags after a gap do not create repeating headers', () {
    final node = table(headers: 0);
    node['content'][2]['attrs']['header'] = true;
    final result = layout(node);
    expect(
      result.pages
          .expand((p) => p.content)
          .where((p) => p.area == 'tableHeader'),
      isEmpty,
    );
    bodyExactlyOnce(result, 23);
  });
  test('merged header cells retain complete groups on every page', () {
    final node = table();
    node['content'][0]['content'] = [
      cell('Merged heading', rows: 2),
      cell('Title0'),
    ];
    node['content'][1]['content'] = [cell('Title1')];
    final result = layout(node);
    bodyExactlyOnce(result, 23);
    for (final page in result.pages) {
      expect(
        page.visibleText.replaceAll(RegExp(r'\s+'), ' '),
        contains('Merged heading'),
      );
      expect(page.visibleText, contains('Title1'));
    }
    expect(result.notices, isEmpty);
  });
  test('a header/body merge is diagnosed without corrupting the row grid', () {
    final node = table(headers: 1);
    node['content'][0]['content'] = [
      cell('Merged heading', rows: 2),
      cell('Title0'),
    ];
    node['content'][1]['content'] = [cell('R0')];
    final result = layout(node);
    expect(result.notices.join(), contains('crosses a merged body row'));
    expect(
      result.pages
          .expand((p) => p.content)
          .where((p) => p.area == 'tableHeader'),
      isEmpty,
    );
    bodyExactlyOnce(result, 23);
  });
  test(
    'derived header fields use each page and preserve saved source caches',
    () {
      final node = table(headers: 1);
      node['content'][0]['content'][0]['content'] = [
        {
          ...f.paragraph(''),
          'content': [
            {'type': 'text', 'text': 'P'},
            newPageField('PAGE'),
            {'type': 'text', 'text': '/'},
            newPageField('NUMPAGES'),
          ],
        },
      ];
      final before = jsonEncode(node);
      final result = layout(node, sections: [f.section(0, 0, start: 5)]);
      for (final page in result.pages) {
        expect(
          page.visibleText,
          contains('P${page.number}/${result.pages.length}'),
        );
      }
      bodyExactlyOnce(result, 23);
      expect(jsonEncode(node), before);
    },
  );
  test(
    'the first header travels with its first body row when space runs out',
    () {
      final node = table(headers: 1, bodyRows: 2);
      final result = layout(
        node,
        content: [f.paragraph(List.filled(12, 'Intro').join('\n')), node],
        sections: [f.section(0, 1)],
      );
      final tableStart = result.pages
          .expand((p) => p.content)
          .firstWhere((p) => p.bodyIndex == 1);
      expect(tableStart.text, contains('R0'));
      expect(result.pages.first.visibleText, isNot(contains('H0')));
      bodyExactlyOnce(result, 2);
    },
  );
  test(
    'all-header tables and oversized headers terminate and retain source text',
    () {
      for (final node in [
        table(headers: 20, bodyRows: 0),
        table(headers: 10, bodyRows: 4),
      ]) {
        final result = layout(node);
        expect(result.pages.length, inInclusiveRange(2, 10));
        final sourceText = result.pages
            .expand((p) => p.content.where((p) => p.area == 'body'))
            .map((p) => p.text)
            .join('\n');
        expect(
          RegExp(r'\bH\d+\b').allMatches(sourceText).length,
          DocumentTable(node).headerRows,
        );
        expect(
          result.pages
              .expand((p) => p.content)
              .where((p) => p.area == 'tableHeader'),
          isEmpty,
        );
      }
    },
  );
  test('an oversized body row makes progress without empty repeated pages', () {
    final node = table(headers: 1, bodyRows: 3);
    node['content'][1]['content'][1]['content'] = [
      f.paragraph(List.generate(60, (i) => 'Line$i').join('\n')),
    ];
    final result = layout(node);
    bodyExactlyOnce(result, 3);
    final sourceText = result.pages
        .expand((p) => p.content.where((p) => p.area == 'body'))
        .map((p) => p.text)
        .join('\n');
    expect(
      RegExp(r'\bLine\d+\b').allMatches(sourceText).map((m) => m[0]).toList(),
      List.generate(60, (i) => 'Line$i'),
    );
    expect(result.pages.length, lessThan(15));
  });
  test(
    'column balancing replays derived headers once and preserves every row',
    () {
      for (final custom in [false, true]) {
        final node = table(headers: 1, bodyRows: 38);
        final result = layout(
          node,
          content: [node, f.paragraph('After table')],
          sections: [
            balance.twoColumns(0, 0, height: 5000, custom: custom, rtl: custom),
            balance.oneColumn(1, 1, height: 5000),
          ],
        );
        bodyExactlyOnce(result, 38);
        for (final page in result.pages) {
          final placements = page.content
              .where((p) => p.bodyIndex == 0)
              .toList();
          for (final x in placements.map((p) => p.offset.dx).toSet()) {
            final group = placements.where((p) => p.offset.dx == x).toList();
            expect(
              RegExp(
                r'\bH0\b',
              ).allMatches(group.map((p) => p.text).join('\n')).length,
              1,
            );
            expect(group.first.text, contains('H0'));
            for (var i = 1; i < group.length; i++) {
              expect(
                group[i].offset.dy,
                greaterThanOrEqualTo(
                  group[i - 1].offset.dy + group[i - 1].height - .01,
                ),
              );
            }
          }
        }
        expect(result.pages.last.visibleText, contains('After table'));
      }
    },
  );
  test(
    'changing selected rows preserves merged content and source row identity',
    () {
      final node = table(headers: 1), before = jsonEncode(table(headers: 1));
      node['content'][1]['attrs'] = <String, dynamic>{
        ...node['content'][1]['attrs'],
        'officeId': 'row-b',
      };
      final edited = DocumentTable(
        node,
      ).format(const TableArea(0, 0, 1, 1), header: true);
      expect(DocumentTable(edited).headerRows, 2);
      expect(edited['content'][1]['attrs']['officeId'], 'row-b');
      expect(edited['content'][1]['content'], node['content'][1]['content']);
      final off = DocumentTable(
        edited,
      ).format(const TableArea(0, 0, 0, 1), header: false);
      expect(DocumentTable(off).headerRows, 0);
      expect(off['content'][1]['attrs']['header'], true);
      (node['content'][1]['attrs'] as Map).remove('officeId');
      expect(jsonEncode(node), before);
    },
  );
}
