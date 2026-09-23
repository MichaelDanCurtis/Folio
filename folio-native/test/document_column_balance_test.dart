import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_page_layout.dart';
import 'package:folio_native/src/document_fields.dart';
import 'document_page_layout_test.dart' as fixture;
import 'document_columns_test.dart' as column_fixture;

Map twoColumns(
  int first,
  int last, {
  bool custom = false,
  bool rtl = false,
  String start = 'nextPage',
  int height = 6000,
}) => {
  ...column_fixture.section(
    first,
    last,
    custom: custom,
    rtl: rtl,
    start: start,
  ),
  'geometry': fixture.geometry(width: 6600, height: height, start: start),
};
Map oneColumn(
  int first,
  int last, {
  String start = 'continuous',
  int height = 6000,
}) => fixture.section(
  first,
  last,
  page: fixture.geometry(width: 6600, height: height, start: start),
);
List<PagePlacement> body(LaidOutDocumentPage page, {int? index}) => page.content
    .where((p) => p.area == 'body' && (index == null || p.bodyIndex == index))
    .toList();
String text(DocumentPageLayout layout) => column_fixture.compact(
  layout.pages.expand((p) => body(p)).map((p) => p.text).join(),
);
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'physical page fields converge after balancing frees room for the next section',
    () {
      final paragraphs = List.generate(
        20,
        (i) => {
          ...fixture.paragraph(''),
          'content': [
            {'type': 'text', 'text': 'Line $i: '},
            newPageField('PAGE'),
            {'type': 'text', 'text': '/'},
            newPageField('NUMPAGES'),
          ],
        },
      );
      final input = fixture.snapshot(
        [...paragraphs, fixture.paragraph('After')],
        [twoColumns(0, 19, height: 4000), oneColumn(20, 20, height: 4000)],
      );
      final before = jsonEncode(input);
      final layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.converged, true);
      for (final page in layout.pages) {
        for (final p in body(page).where((p) => p.bodyIndex! < 20)) {
          expect(
            column_fixture.compact(p.text),
            'Line${p.bodyIndex}:${page.number}/${layout.pages.length}',
          );
        }
      }
      expect(jsonEncode(input), before);
    },
  );
  test(
    'a new-column section preserves the earlier section in its shared band',
    () {
      final three = {...column_fixture.columns(), 'count': 3};
      final first = {...twoColumns(0, 0), 'columnLayout': three};
      final second = {
        ...twoColumns(1, 1, start: 'nextColumn'),
        'columnLayout': three,
      };
      final input = fixture.snapshot(
        [
          fixture.paragraph('Earlier\nStays\nHere'),
          fixture.paragraph('A\nB\nC\nD\nE\nF'),
          fixture.paragraph('After'),
        ],
        [first, second, oneColumn(2, 2)],
      );
      final layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      final page = layout.pages.single;
      final earlier = body(page, index: 0).single;
      final middle = body(page, index: 1);
      expect(earlier.offset.dx, page.margins.left);
      expect(earlier.offset.dy, page.top);
      expect(earlier.text, 'Earlier\nStays\nHere');
      expect(middle.map((p) => p.offset.dx).toSet().length, 2);
      expect(middle.first.offset.dx, greaterThan(earlier.offset.dx));
      expect(
        body(page, index: 2).single.offset.dy,
        greaterThanOrEqualTo(earlier.offset.dy + earlier.height),
      );
      expect(text(layout), 'EarlierStaysHereABCDEFAfter');
    },
  );
  test(
    'unequal-column constraints can retain a short tail in the wide column',
    () {
      final words = List.generate(70, (i) => 'W$i 😀 café');
      final input = fixture.snapshot(
        [fixture.paragraph(words.join(' ')), fixture.paragraph('After')],
        [twoColumns(0, 0, custom: true, rtl: true), oneColumn(1, 1)],
      );
      final layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.converged, true);
      expect(text(layout), column_fixture.compact('${words.join(' ')}After'));
      final last = layout.pages.last;
      expect(
        body(last, index: 1).single.offset.dy,
        last.columnBands.first.usedBottom,
      );
      for (final p in body(last, index: 0)) {
        expect(
          p.offset.dy + p.height,
          lessThanOrEqualTo(last.columnBands.first.bottom + .01),
        );
      }
    },
  );
  test(
    'vertically merged table cells cannot be divided by a balanced column break',
    () {
      Map cell(String value, {int rows = 1}) => {
        'type': 'tableCell',
        'attrs': {'rowspan': rows},
        'content': [fixture.paragraph(value)],
      };
      final table = {
        'type': 'table',
        'attrs': {
          'columnWidths': [1200, 1200],
        },
        'content': [
          {
            'type': 'tableRow',
            'content': [cell('Merged', rows: 2), cell('R0')],
          },
          {
            'type': 'tableRow',
            'content': [cell('R1')],
          },
          for (var i = 2; i < 7; i++)
            {
              'type': 'tableRow',
              'content': [cell('L$i'), cell('R$i')],
            },
        ],
      };
      final layout = DocumentPageLayout.build(
        fixture.snapshot(
          [table, fixture.paragraph('After')],
          [twoColumns(0, 0), oneColumn(1, 1)],
        ),
      );
      addTearDown(layout.dispose);
      final pieces = layout.pages.expand((p) => body(p, index: 0)).toList();
      expect(column_fixture.compact(pieces.first.text), contains('Merged'));
      expect(pieces.first.text, contains('R1'));
      for (var i = 0; i < 7; i++) {
        expect(
          RegExp(
            'R$i(?![0-9])',
          ).allMatches(pieces.map((p) => p.text).join()).length,
          1,
        );
      }
    },
  );
  test(
    'continuous boundary balances lines and starts the next section below all columns',
    () {
      final lines = List.generate(10, (i) => fixture.paragraph('Line $i'));
      final input = fixture.snapshot(
        [...lines, fixture.paragraph('After')],
        [twoColumns(0, 9), oneColumn(10, 10)],
      );
      final before = jsonEncode(input),
          layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.converged, true);
      expect(layout.pages.length, 1);
      final page = layout.pages.single, band = page.columnBands.first;
      expect(band.balanced, true);
      final ends = <double, double>{};
      for (final p in body(page).where((p) => p.bodyIndex! < 10)) {
        ends[p.offset.dx] = p.offset.dy + p.height;
      }
      expect(ends.length, 2);
      expect(
        (ends.values.first - ends.values.last).abs(),
        lessThanOrEqualTo(24),
      );
      expect(page.columnBands[1].top, band.usedBottom);
      expect(body(page, index: 10).single.offset.dy, band.usedBottom);
      expect(
        text(layout),
        column_fixture.compact(
          '${List.generate(10, (i) => 'Line $i').join()}After',
        ),
      );
      expect(jsonEncode(input), before);
    },
  );
  test(
    'unequal RTL columns reshape the final-page tail without losing rich Unicode',
    () {
      final words = List.generate(80, (i) => 'W$i 😀 café');
      final paragraph = fixture.paragraph(words.join(' '));
      paragraph['content'] = [
        for (var i = 0; i < words.length; i++)
          {
            'type': 'text',
            'text': '${words[i]}${i + 1 < words.length ? ' ' : ''}',
            'marks': [
              {'type': i.isEven ? 'bold' : 'italic'},
            ],
          },
      ];
      final input = fixture.snapshot(
        [paragraph, fixture.paragraph('After')],
        [twoColumns(0, 0, custom: true, rtl: true), oneColumn(1, 1)],
      );
      final before = jsonEncode(input),
          layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.converged, true);
      expect(text(layout), column_fixture.compact('${words.join(' ')}After'));
      final finalPage = layout.pages.lastWhere((p) => p.sections.contains(0));
      expect(finalPage.columnBands.first.balanced, true);
      final tail = body(finalPage, index: 0);
      expect(tail.first.offset.dx, greaterThan(tail.last.offset.dx));
      expect(jsonEncode(input), before);
    },
  );
  test('balancing leaves all earlier physical pages unchanged', () {
    final lines = List.generate(
      100,
      (i) => 'Line ${i.toString().padLeft(3, '0')}',
    );
    final input = fixture.snapshot(
      [fixture.paragraph(lines.join('\n')), fixture.paragraph('After')],
      [twoColumns(0, 0, height: 4000), oneColumn(1, 1, height: 4000)],
    );
    final unbalanced = DocumentPageLayout.build({
      ...input,
      'noColumnBalance': true,
    });
    final balanced = DocumentPageLayout.build(input);
    addTearDown(unbalanced.dispose);
    addTearDown(balanced.dispose);
    final previous = unbalanced.pages
        .where((p) => p.sections.contains(0))
        .toList();
    expect(previous.length, greaterThan(2));
    for (var i = 0; i + 1 < previous.length; i++) {
      String signature(LaidOutDocumentPage page) =>
          body(page, index: 0).map((p) => '${p.offset}:${p.text}').join('|');
      expect(signature(balanced.pages[i]), signature(previous[i]));
    }
    expect(text(balanced), column_fixture.compact('${lines.join()}After'));
    expect(
      balanced.pages
          .lastWhere((p) => p.sections.contains(0))
          .columnBands
          .first
          .balanced,
      true,
    );
  });
  test(
    'balancing keeps whole paragraphs and keep-with-next pairs together',
    () {
      final input = fixture.snapshot(
        [
          fixture.paragraph(
            'Keep A\nKeep B\nKeep C\nKeep D',
            properties: {'keepLines': true},
          ),
          fixture.paragraph('Pair one', properties: {'keepNext': true}),
          fixture.paragraph('Pair two'),
          fixture.paragraph('After'),
        ],
        [twoColumns(0, 2), oneColumn(3, 3)],
      );
      final layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      final page = layout.pages.single;
      expect(body(page, index: 0).length, 1);
      expect(
        body(page, index: 1).single.offset.dx,
        body(page, index: 2).single.offset.dx,
      );
      expect(text(layout), 'KeepAKeepBKeepCKeepDPaironePairtwoAfter');
    },
  );
  test('explicit column breaks retain their uneven distribution', () {
    final paragraph = fixture.paragraph('');
    paragraph['content'] = [
      {'type': 'text', 'text': List.generate(8, (i) => 'Left $i').join('\n')},
      {
        'type': 'hardBreak',
        'attrs': {'breakType': 'column'},
      },
      {'type': 'text', 'text': 'Right A\nRight B'},
    ];
    final layout = DocumentPageLayout.build(
      fixture.snapshot(
        [paragraph, fixture.paragraph('After')],
        [twoColumns(0, 0), oneColumn(1, 1)],
      ),
    );
    addTearDown(layout.dispose);
    final placements = body(layout.pages.first, index: 0);
    expect(placements.length, 2);
    expect(placements.first.text, contains('Left 7'));
    expect(placements.last.text, contains('Right A'));
    expect(placements.first.height, greaterThan(placements.last.height * 2));
    expect(layout.pages.length, 1);
  });
  test('compatibility opt-out and ordinary section endings do not balance', () {
    for (final start in ['continuous', 'nextPage']) {
      final input = fixture.snapshot(
        [
          fixture.paragraph('One\nTwo\nThree\nFour'),
          fixture.paragraph('After'),
        ],
        [twoColumns(0, 0), oneColumn(1, 1, start: start)],
      );
      if (start == 'continuous') input['noColumnBalance'] = true;
      final layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.pages.first.columnBands.first.balanced, false);
      expect(
        body(
          layout.pages.first,
          index: 0,
        ).map((p) => p.offset.dx).toSet().length,
        1,
      );
    }
    final layout = DocumentPageLayout.build(
      fixture.snapshot(
        [fixture.paragraph('One\nTwo\nThree\nFour')],
        [twoColumns(0, 0)],
      ),
    );
    addTearDown(layout.dispose);
    expect(layout.pages.single.columnBands.single.balanced, false);
    expect(body(layout.pages.single).length, 1);
  });
  test(
    'identical columns still end one balanced area at a continuous boundary',
    () {
      final layout = DocumentPageLayout.build(
        fixture.snapshot(
          [
            fixture.paragraph('A\nB\nC\nD\nE\nF'),
            fixture.paragraph('Following'),
          ],
          [twoColumns(0, 0), twoColumns(1, 1, start: 'continuous')],
        ),
      );
      addTearDown(layout.dispose);
      final page = layout.pages.single;
      expect(page.columnBands.length, 2);
      expect(page.columnBands.first.balanced, true);
      expect(page.columnBands.last.top, page.columnBands.first.usedBottom);
      expect(body(page, index: 1).single.offset.dx, page.margins.left);
    },
  );
  test('table rows balance as groups with every cell retained once', () {
    final table = {
      'type': 'table',
      'attrs': {
        'columnWidths': [3000],
      },
      'content': List.generate(
        7,
        (i) => {
          'type': 'tableRow',
          'content': [
            {
              'type': 'tableCell',
              'content': [fixture.paragraph('Cell $i')],
            },
          ],
        },
      ),
    };
    final layout = DocumentPageLayout.build(
      fixture.snapshot(
        [table, fixture.paragraph('After')],
        [twoColumns(0, 0), oneColumn(1, 1)],
      ),
    );
    addTearDown(layout.dispose);
    final tableParts = layout.pages.expand((p) => body(p, index: 0)).toList();
    expect(tableParts.map((p) => p.offset.dx).toSet().length, 2);
    expect(text(layout), 'Cell0Cell1Cell2Cell3Cell4Cell5Cell6After');
    expect(layout.pages.single.columnBands.first.balanced, true);
  });
}
