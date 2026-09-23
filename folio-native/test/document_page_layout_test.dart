import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_fields.dart';
import 'package:folio_native/src/document_page_layout.dart';
import 'package:folio_native/src/document_page_preview.dart';
import 'package:folio_native/src/document_page_setup.dart';

Map paragraph(String text, {Map properties = const {}}) => {
  'type': 'paragraph',
  'attrs': {
    'paragraphFormat': {
      'inherited': {},
      'direct': {'after': 0, ...properties},
    },
  },
  'content': [
    {'type': 'text', 'text': text},
  ],
};
Map geometry({
  String start = 'nextPage',
  int width = 4500,
  int height = 4500,
}) => {
  ...defaultPageGeometry,
  'width': width,
  'height': height,
  'left': 300,
  'right': 300,
  'top': 300,
  'bottom': 300,
  'header': 100,
  'footer': 100,
  'start': start,
};
Map section(
  int first,
  int last, {
  Map? page,
  String format = 'decimal',
  int? start,
  Map? footer,
}) => {
  'firstBlock': first,
  'lastBlock': last,
  'geometry': page ?? geometry(),
  'pageNumbering': {'format': format, 'start': start},
  if (footer != null) 'footer': {'default': footer},
};
Map snapshot(List content, List sections) => {
  'document': {'type': 'doc', 'content': content},
  'sections': sections,
};
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'line fragments preserve all text and stay inside physical body bounds',
    () {
      final lines = List.generate(
        70,
        (i) => 'Line ${i.toString().padLeft(2, '0')}',
      );
      final input = snapshot([paragraph(lines.join('\n'))], [section(0, 0)]);
      final before = jsonEncode(input);
      final layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.converged, true);
      expect(layout.pages.length, greaterThan(4));
      final rendered = <String>[];
      for (final page in layout.pages) {
        for (final item in page.content.where((c) => c.area == 'body')) {
          expect(item.offset.dy, greaterThanOrEqualTo(page.top - .01));
          expect(
            item.offset.dy + item.height,
            lessThanOrEqualTo(page.bottom + .01),
          );
          rendered.addAll(item.text.split('\n'));
        }
      }
      expect(rendered, lines);
      expect(jsonEncode(input), before);
    },
  );
  test(
    'section geometry, odd-page blank, continuous flow and restarted footer fields',
    () {
      final footer = {
        'content': [
          {
            'type': 'paragraph',
            'content': [
              {'type': 'text', 'text': 'Page '},
              newPageField('PAGE'),
              {'type': 'text', 'text': ' of '},
              newPageField('NUMPAGES'),
            ],
          },
        ],
      };
      final input = snapshot(
        [paragraph('One'), paragraph('Two'), paragraph('Three')],
        [
          section(0, 0, footer: footer),
          section(
            1,
            1,
            page: geometry(start: 'oddPage', width: 6000),
            format: 'lowerRoman',
            start: 5,
            footer: footer,
          ),
          section(
            2,
            2,
            page: geometry(start: 'continuous', width: 6000),
            footer: footer,
          ),
        ],
      );
      final before = jsonEncode(input),
          layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.pages.length, 3);
      expect(layout.pages[1].blank, true);
      expect(layout.pages[2].size.width, 400);
      expect(layout.pages[2].sections, {1, 2});
      expect(layout.pages[0].visibleText, contains('Page 1 of 3'));
      expect(layout.pages[2].visibleText, contains('Page v of 3'));
      expect(layout.pages[2].visibleText, contains('Three'));
      expect(layout.converged, true);
      expect(jsonEncode(input), before);
    },
  );
  test(
    'fields in a long paragraph resolve at each fragment and keep saved caches',
    () {
      final inline = <Map>[];
      for (var i = 0; i < 30; i++) {
        inline.add({'type': 'text', 'text': '$i: '});
        inline.add(newPageField('PAGE'));
        inline.add({'type': 'hardBreak'});
      }
      final input = snapshot(
        [
          {'type': 'paragraph', 'content': inline},
        ],
        [section(0, 0, start: 7)],
      );
      final before = jsonEncode(input),
          layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.pages.length, greaterThan(2));
      expect(layout.converged, true);
      for (final page in layout.pages) {
        for (final match in RegExp(
          r'(\d+): (\d+)',
        ).allMatches(page.visibleText)) {
          expect(int.parse(match.group(2)!), page.number);
        }
      }
      expect(jsonEncode(input), before);
    },
  );
  test(
    'table row groups cross pages without splitting vertical merges or losing cells',
    () {
      Map cell(String text, {int rows = 1}) => {
        'type': 'tableCell',
        'attrs': {'rowspan': rows, 'colspan': 1},
        'content': [paragraph(text)],
      };
      final rows = [
        {
          'type': 'tableRow',
          'content': [cell('Merged', rows: 2), cell('R0')],
        },
        {
          'type': 'tableRow',
          'content': [cell('R1')],
        },
        for (var i = 2; i < 18; i++)
          {
            'type': 'tableRow',
            'content': [cell('Left$i'), cell('R$i')],
          },
      ];
      final input = snapshot(
        [
          {
            'type': 'table',
            'attrs': {
              'columnWidths': [1800, 1800],
            },
            'content': rows,
          },
        ],
        [section(0, 0)],
      );
      final layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.pages.length, greaterThan(2));
      expect(layout.pages.first.visibleText, contains('Merged'));
      expect(layout.pages.first.visibleText, contains('R1'));
      final text = layout.pages.map((p) => p.visibleText).join('\n');
      for (var i = 0; i < 18; i++) {
        expect(RegExp('R$i(?![0-9])').allMatches(text).length, 1);
      }
    },
  );
  test(
    'page-break-before and keep-lines move paragraphs as units when they fit',
    () {
      final input = snapshot(
        [
          paragraph('A\nB\nC\nD\nE\nF\nG\nH\nI'),
          paragraph(
            'Keep one\nKeep two\nKeep three',
            properties: {'keepLines': true},
          ),
          paragraph('Break before', properties: {'pageBreakBefore': true}),
        ],
        [section(0, 2)],
      );
      final layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.pages.length, 3);
      expect(
        layout.pages[1].visibleText,
        contains('Keep one\nKeep two\nKeep three'),
      );
      expect(layout.pages[2].visibleText, contains('Break before'));
    },
  );
  test(
    'manual page breaks split within a paragraph and preserve ordinary line breaks',
    () {
      final node = {
        'type': 'paragraph',
        'content': [
          {'type': 'text', 'text': 'Before'},
          {
            'type': 'hardBreak',
            'attrs': {'breakType': 'page'},
          },
          {'type': 'text', 'text': 'After'},
          {'type': 'hardBreak'},
          {'type': 'text', 'text': 'Same page'},
        ],
      };
      final input = snapshot([node], [section(0, 0)]);
      final before = jsonEncode(input);
      final layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.pages.length, 2);
      expect(layout.pages.first.visibleText.trim(), 'Before');
      expect(layout.pages.last.visibleText.trim(), 'After\nSame page');
      expect(jsonEncode(input), before);
    },
  );
  test('first and even footer variants repeat and expand body exclusion', () {
    Map story(String text) => {
      'content': [paragraph(text)],
    };
    final row = section(0, 0);
    row['differentFirstPage'] = true;
    row['footer'] = {
      'first': story('First footer'),
      'even': story('Even footer'),
      'default': story('Odd footer'),
    };
    row['header'] = {'first': story('Header one\nHeader two\nHeader three')};
    final input = snapshot(
      [paragraph(List.generate(40, (i) => 'Body $i').join('\n'))],
      [row],
    );
    input['differentOddEven'] = true;
    final layout = DocumentPageLayout.build(input);
    addTearDown(layout.dispose);
    expect(layout.pages.length, greaterThan(3));
    expect(layout.pages[0].visibleText, contains('First footer'));
    expect(layout.pages[0].top, greaterThan(60));
    expect(layout.pages[1].visibleText, contains('Even footer'));
    expect(layout.pages[2].visibleText, contains('Odd footer'));
    expect(layout.pages[3].visibleText, contains('Even footer'));
  });
  test(
    'section counts, field switches and locked cached results resolve independently',
    () {
      Map footer() => {
        'content': [
          {
            'type': 'paragraph',
            'content': [
              {'type': 'text', 'text': 'Section '},
              newPageField('SECTION'),
              {'type': 'text', 'text': ' pages '},
              newPageField('SECTIONPAGES'),
              {'type': 'text', 'text': ' page '},
              {
                ...newPageField('PAGE'),
                'attrs': {
                  ...newPageField('PAGE')['attrs'],
                  'instruction': r'PAGE \* ROMAN',
                },
              },
              {'type': 'text', 'text': ' locked '},
              {
                ...newPageField('NUMPAGES'),
                'text': '42',
                'attrs': {...newPageField('NUMPAGES')['attrs'], 'locked': true},
              },
            ],
          },
        ],
      };
      final input = snapshot(
        [
          paragraph(List.generate(30, (i) => 'One $i').join('\n')),
          paragraph('Two'),
        ],
        [section(0, 0, footer: footer()), section(1, 1, footer: footer())],
      );
      final layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      final firstPages = layout.pages.where((p) => p.section == 0).length;
      expect(firstPages, greaterThan(1));
      expect(
        layout.pages.first.visibleText.replaceAll('\n', ''),
        contains('Section 1 pages $firstPages page I locked 42'),
      );
      expect(
        layout.pages.last.visibleText.replaceAll('\n', ''),
        contains('Section 2 pages 1 page'),
      );
      expect(
        layout.pages.last.visibleText.replaceAll('\n', ''),
        contains('locked 42'),
      );
    },
  );
  testWidgets(
    'page preview paints pages, navigates, zooms and explains actual limitations',
    (tester) async {
      tester.view.physicalSize = const Size(1100, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final input = snapshot(
        [paragraph(List.generate(40, (i) => 'Preview line $i').join('\n'))],
        [section(0, 0)],
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: DocumentPagePreview(snapshot: input)),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(
        find.byKey(const ValueKey('page-preview-paper-0')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key('page-preview-next')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('page-preview-paper-1')),
        findsOneWidget,
      );
      tester.view.physicalSize = const Size(700, 650);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.tap(find.byKey(const Key('page-preview-zoom')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('200%').last);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
}
