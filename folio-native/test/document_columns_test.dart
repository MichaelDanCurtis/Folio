import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_columns.dart';
import 'package:folio_native/src/document_fields.dart';
import 'package:folio_native/src/document_page_layout.dart';
import 'document_page_layout_test.dart' as fixture;

Map columns({bool custom = false, bool rtl = false}) => {
  'count': 2,
  'equalWidth': !custom,
  'space': 360,
  'separator': true,
  'rtl': rtl,
  'columns': custom
      ? [
          {'width': 1500, 'space': 360},
          {'width': 4140, 'space': 0},
        ]
      : [],
};
Map section(
  int first,
  int last, {
  bool custom = false,
  bool rtl = false,
  String start = 'nextPage',
}) => {
  ...fixture.section(
    first,
    last,
    page: fixture.geometry(width: 6600, height: 4000, start: start),
  ),
  'columnLayout': columns(custom: custom, rtl: rtl),
};
String compact(String text) => text.replaceAll(RegExp(r'\s+'), '');
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'equal columns fill in reading order without text loss across pages',
    () {
      final lines = List.generate(
        80,
        (i) => 'Line ${i.toString().padLeft(2, '0')}',
      );
      final input = fixture.snapshot(
        [fixture.paragraph(lines.join('\n'))],
        [section(0, 0)],
      );
      final before = jsonEncode(input),
          layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.converged, true);
      expect(layout.pages.length, greaterThan(2));
      expect(
        layout.pages.first.content
            .where((c) => c.area == 'body')
            .map((c) => c.offset.dx)
            .toSet()
            .length,
        2,
      );
      final text = <String>[];
      for (final page in layout.pages) {
        final body = page.content.where((c) => c.area == 'body').toList();
        for (final item in body) {
          expect(item.offset.dy, greaterThanOrEqualTo(page.top - .01));
          expect(
            item.offset.dy + item.height,
            lessThanOrEqualTo(page.bottom + .01),
          );
          text.addAll(item.text.split('\n'));
        }
        expect(page.columnBands.single.geometry.separator, true);
      }
      expect(text, lines);
      expect(jsonEncode(input), before);
    },
  );
  test(
    'unequal columns reflow rich Unicode paragraphs at every width change',
    () {
      final words = List.generate(160, (i) => 'W$i 😀 café');
      final node = fixture.paragraph(words.join(' '));
      node['content'] = [
        for (var i = 0; i < words.length; i++)
          {
            'type': 'text',
            'text': '${words[i]}${i + 1 < words.length ? ' ' : ''}',
            'marks': [
              {'type': i.isEven ? 'bold' : 'italic'},
            ],
          },
      ];
      final input = fixture.snapshot([node], [section(0, 0, custom: true)]);
      final before = jsonEncode(input),
          layout = DocumentPageLayout.build(input);
      addTearDown(layout.dispose);
      expect(layout.converged, true);
      expect(layout.pages.length, greaterThan(2));
      final body = layout.pages
          .expand((p) => p.content.where((c) => c.area == 'body'))
          .toList();
      expect(body.map((b) => b.offset.dx).toSet(), {20.0, 144.0});
      expect(compact(body.map((b) => b.text).join()), compact(words.join(' ')));
      expect(jsonEncode(input), before);
    },
  );
  test(
    'column and page breaks differ and fields count physical pages in RTL flow',
    () {
      final node = fixture.paragraph('');
      node['content'] = [
        {'type': 'text', 'text': 'First '},
        newPageField('PAGE'),
        {
          'type': 'hardBreak',
          'attrs': {'breakType': 'column'},
        },
        {'type': 'text', 'text': 'Second '},
        newPageField('PAGE'),
        {
          'type': 'hardBreak',
          'attrs': {'breakType': 'page'},
        },
        {'type': 'text', 'text': 'Third '},
        newPageField('PAGE'),
      ];
      final layout = DocumentPageLayout.build(
        fixture.snapshot([node], [section(0, 0, rtl: true)]),
      );
      addTearDown(layout.dispose);
      expect(layout.pages.length, 2);
      final body = layout.pages.first.content
          .where((c) => c.area == 'body')
          .toList();
      expect(compact(body[0].text), 'First1');
      expect(compact(body[1].text), 'Second1');
      expect(body[0].offset.dx, greaterThan(body[1].offset.dx));
      expect(compact(layout.pages.last.visibleText), 'Third2');
    },
  );
  test(
    'table row groups remeasure in unequal columns and retain each cell once',
    () {
      final rows = List.generate(
        18,
        (i) => {
          'type': 'tableRow',
          'content': [
            {
              'type': 'tableCell',
              'content': [fixture.paragraph('Cell $i with wrapping text')],
            },
          ],
        },
      );
      final table = {
        'type': 'table',
        'attrs': {
          'columnWidths': [3000],
        },
        'content': rows,
      };
      final layout = DocumentPageLayout.build(
        fixture.snapshot([table], [section(0, 0, custom: true)]),
      );
      addTearDown(layout.dispose);
      final body = layout.pages
          .expand((p) => p.content.where((c) => c.area == 'body'))
          .toList();
      expect(body.map((b) => b.offset.dx).toSet().length, 2);
      expect(
        compact(body.map((b) => b.text).join()),
        compact(List.generate(18, (i) => 'Cell $i with wrapping text').join()),
      );
    },
  );
  test(
    'continuous column changes start below preceding content without overlap',
    () {
      final first = fixture.section(
        0,
        0,
        page: fixture.geometry(width: 6600, height: 4000),
      );
      final second = section(1, 1, start: 'continuous');
      final last = fixture.section(
        2,
        2,
        page: fixture.geometry(width: 6600, height: 4000, start: 'continuous'),
      );
      final layout = DocumentPageLayout.build(
        fixture.snapshot(
          [
            fixture.paragraph('Heading'),
            fixture.paragraph('Two column text'),
            fixture.paragraph('Ending'),
          ],
          [first, second, last],
        ),
      );
      addTearDown(layout.dispose);
      expect(layout.pages.length, 1);
      final bands = layout.pages.single.columnBands;
      expect(bands.length, 3);
      expect(bands[1].top, bands[0].usedBottom);
      expect(bands[2].top, bands[1].usedBottom);
      expect(
        compact(layout.pages.single.visibleText),
        'HeadingTwocolumntextEnding',
      );
    },
  );
  test(
    'next-column sections share the physical page and honor column direction',
    () {
      final layout = DocumentPageLayout.build(
        fixture.snapshot(
          [
            fixture.paragraph('First section'),
            fixture.paragraph('Second section'),
          ],
          [section(0, 0), section(1, 1, start: 'nextColumn')],
        ),
      );
      addTearDown(layout.dispose);
      expect(layout.pages.length, 1);
      expect(layout.pages.single.sections, {0, 1});
      final body = layout.pages.single.content
          .where((c) => c.area == 'body')
          .toList();
      expect(body[1].offset.dx, greaterThan(body[0].offset.dx));
      expect(body[1].offset.dy, layout.pages.single.top);
    },
  );
  testWidgets(
    'column dialog presets, custom measurements, validation and cancel are drafts',
    (tester) async {
      tester.view.physicalSize = const Size(1000, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final data = {
        'sections': [
          {'key': 'section-0', ...section(0, 0)},
        ],
      };
      Map? result;
      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () async {
                  result = await showDialog<Map>(
                    context: context,
                    builder: (_) => DocumentColumnsDialog(data: data),
                  );
                },
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text('Open'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('columns-preset-left')));
      await tester.pumpAndSettle();
      await tester.enterText(find.byKey(const Key('columns-width-0')), '20');
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('columns-error')), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(find.byKey(const Key('columns-apply')))
            .onPressed,
        isNull,
      );
      await tester.tap(find.byKey(const Key('columns-preset-three')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('columns-rtl')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('columns-apply')));
      await tester.pumpAndSettle();
      expect(result!['patch']['columnLayout']['count'], 3);
      expect(result!['patch']['columnLayout']['rtl'], true);
      expect(columnSettings((data['sections'] as List).first)['count'], 2);
      result = null;
      await tester.tap(find.text('Open'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('columns-preset-one')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('columns-cancel')));
      await tester.pumpAndSettle();
      expect(result, isNull);
      expect(tester.takeException(), isNull);
    },
  );
}
