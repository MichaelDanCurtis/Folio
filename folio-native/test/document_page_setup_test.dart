import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_page_setup.dart';

Map<String, dynamic> catalog() => {
  'sections': [
    {
      'key': 'one',
      'geometry': {...defaultPageGeometry},
      'endOfficeId': 'word-1',
    },
    {
      'key': 'two',
      'geometry': {
        ...defaultPageGeometry,
        'width': 16838,
        'height': 11906,
        'orientation': 'landscape',
        'left': 1080,
      },
      'endOfficeId': null,
    },
  ],
};

void main() {
  Future<void> dropdown(WidgetTester t, String key, String text) async {
    await t.ensureVisible(find.byKey(Key(key)));
    await t.tap(find.byKey(Key(key)));
    await t.pumpAndSettle();
    await t.tap(find.text(text).last);
    await t.pumpAndSettle();
  }

  testWidgets(
    'page dialog applies only changed settings with units and scope',
    (t) async {
      t.view.physicalSize = const Size(1100, 1000);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      Map? result;
      await t.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () async {
                  result = await showDialog<Map>(
                    context: context,
                    builder: (_) => DocumentPageSetupDialog(
                      data: catalog(),
                      initialSection: 1,
                    ),
                  );
                },
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      );
      await t.tap(find.text('Open'));
      await t.pumpAndSettle();
      expect(
        t
            .widget<FilledButton>(find.byKey(const Key('page-setup-apply')))
            .onPressed,
        isNull,
      );
      await dropdown(t, 'page-unit', 'Centimeters');
      await t.enterText(find.byKey(const Key('page-left')), '2.54');
      await t.pumpAndSettle();
      await dropdown(t, 'page-scope', 'Whole document');
      await t.tap(find.byKey(const Key('page-setup-apply')));
      await t.pumpAndSettle();
      expect(result, {
        'section': 1,
        'sectionKey': 'two',
        'scope': 'document',
        'patch': {'left': 1440},
      });
      expect(t.takeException(), isNull);
    },
  );
  testWidgets(
    'paper presets, section switching, invalid input and cancellation',
    (t) async {
      t.view.physicalSize = const Size(800, 740);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      Map? result = {'pending': true};
      await t.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () async {
                  result = await showDialog<Map>(
                    context: context,
                    builder: (_) => DocumentPageSetupDialog(data: catalog()),
                  );
                },
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      );
      await t.tap(find.text('Open'));
      await t.pumpAndSettle();
      await dropdown(t, 'page-paper-Letter', 'A4');
      await dropdown(t, 'page-orientation-portrait', 'Landscape');
      expect(
        t
            .widget<TextField>(find.byKey(const Key('page-width')))
            .controller!
            .text,
        '11.693',
      );
      await dropdown(t, 'page-section', 'Section 2 of 2');
      await t.ensureVisible(find.byKey(const Key('page-left')));
      await t.enterText(find.byKey(const Key('page-left')), '22');
      await t.pumpAndSettle();
      expect(
        t
            .widget<FilledButton>(find.byKey(const Key('page-setup-apply')))
            .onPressed,
        isNull,
      );
      expect(find.byKey(const Key('page-setup-error')), findsOneWidget);
      await t.enterText(find.byKey(const Key('page-left')), 'abc');
      await t.pumpAndSettle();
      expect(
        t
            .widget<DropdownButtonFormField<String>>(
              find.byKey(const Key('page-unit')),
            )
            .onChanged,
        isNull,
      );
      await t.enterText(find.byKey(const Key('page-left')), '0.5');
      await t.pumpAndSettle();
      expect(
        t
            .widget<FilledButton>(find.byKey(const Key('page-setup-apply')))
            .onPressed,
        isNotNull,
      );
      await t.tap(find.byKey(const Key('page-setup-cancel')));
      await t.pumpAndSettle();
      expect(result, isNull);
      expect(t.takeException(), isNull);
    },
  );
  testWidgets('whole document cannot bypass a pending section change', (
    t,
  ) async {
    final data = catalog();
    data['sections'][1]['hasTrackedSettings'] = true;
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(body: DocumentPageSetupDialog(data: data)),
      ),
    );
    await t.enterText(find.byKey(const Key('page-left')), '0.5');
    await t.pumpAndSettle();
    expect(
      t
          .widget<FilledButton>(find.byKey(const Key('page-setup-apply')))
          .onPressed,
      isNotNull,
    );
    await dropdown(t, 'page-scope', 'Whole document');
    expect(
      t
          .widget<FilledButton>(find.byKey(const Key('page-setup-apply')))
          .onPressed,
      isNull,
    );
    expect(t.takeException(), isNull);
  });
  testWidgets(
    'physical paper width stays exact and scrolls in a narrow viewport',
    (t) async {
      final geometry = DocumentPageGeometry({
        ...defaultPageGeometry,
        'width': 15840,
        'height': 12240,
        'left': 12000,
        'right': 720,
      });
      expect(geometry.margins.left, 800);
      await t.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SizedBox(
              width: 600,
              height: 500,
              child: DocumentPageFrame(
                geometry: geometry,
                child: const SizedBox(key: Key('content'), height: 624),
              ),
            ),
          ),
        ),
      );
      await t.pumpAndSettle();
      expect(t.getSize(find.byKey(const Key('document-paper'))).width, 1056);
      expect(t.getSize(find.byKey(const Key('content'))).width, 208);
      final horizontal = t
          .widgetList<SingleChildScrollView>(find.byType(SingleChildScrollView))
          .firstWhere((s) => s.scrollDirection == Axis.horizontal)
          .controller!;
      horizontal.jumpTo(horizontal.position.maxScrollExtent);
      await t.pumpAndSettle();
      expect(horizontal.offset, 504);
      expect(t.takeException(), isNull);
    },
  );
  test('section selection follows the last split paragraph identity', () {
    final doc = {
      'type': 'doc',
      'attrs': {'pageLayout': catalog()},
      'content': [
        for (final pair in [
          ('word-0', 'Long opening paragraph'),
          ('word-1', 'B'),
          ('word-1', 'C'),
          ('word-2', 'D'),
        ])
          {
            'type': 'paragraph',
            'attrs': {'officeId': pair.$1},
            'content': [
              {'type': 'text', 'text': pair.$2},
            ],
          },
      ],
    };
    expect(
      pageSectionAtSelection(doc, const TextSelection.collapsed(offset: 25)),
      0,
    );
    expect(
      pageSectionAtSelection(doc, const TextSelection.collapsed(offset: 27)),
      1,
    );
  });
}
