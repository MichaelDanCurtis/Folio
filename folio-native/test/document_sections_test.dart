import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_sections.dart';

Map<String, dynamic> catalog() => {
  'canInsert': true,
  'targetSection': 1,
  'targetKey': 'paragraph',
  'sections': [
    {'key': 'one'},
    {'key': 'two'},
    {'key': 'three'},
  ],
};

void main() {
  testWidgets('section dialog returns chosen break and explains removal', (
    t,
  ) async {
    t.view.physicalSize = const Size(1100, 900);
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
                  builder: (_) => DocumentSectionBreakDialog(data: catalog()),
                );
              },
              child: const Text('Open'),
            ),
          ),
        ),
      ),
    );
    Future<void> open() async {
      await t.tap(find.text('Open'));
      await t.pumpAndSettle();
    }

    await open();
    await t.tap(find.byKey(const Key('section-break-type')));
    await t.pumpAndSettle();
    await t.tap(find.text('Odd page').last);
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('section-break-apply')));
    await t.pumpAndSettle();
    expect(result, {
      'action': 'insert',
      'start': 'oddPage',
      'targetKey': 'paragraph',
    });
    await open();
    await t.tap(find.text('Remove').first);
    await t.pumpAndSettle();
    expect(find.textContaining('Section 2’s page settings'), findsOneWidget);
    await t.tap(find.byKey(const Key('section-break-boundary')));
    await t.pumpAndSettle();
    await t.tap(find.text('Between sections 2 and 3').last);
    await t.pumpAndSettle();
    expect(find.textContaining('Section 3’s page settings'), findsOneWidget);
    await t.tap(find.byKey(const Key('section-break-apply')));
    await t.pumpAndSettle();
    expect(result, {'action': 'remove', 'section': 1, 'sectionKey': 'two'});
    await open();
    await t.tap(find.byKey(const Key('section-break-cancel')));
    await t.pumpAndSettle();
    expect(result, isNull);
    expect(t.takeException(), isNull);
  });

  testWidgets(
    'section dialog prevents protected, recorded and invalid changes',
    (t) async {
      for (final data in [
        {...catalog(), 'protected': true},
        {...catalog(), 'needsAuthor': true},
        {...catalog(), 'recordingEnabled': true},
        {
          ...catalog(),
          'canInsert': false,
          'sections': [
            {'key': 'only'},
          ],
        },
        {
          ...catalog(),
          'sections': [
            {'key': 'one'},
            {'key': 'two', 'hasTrackedSettings': true},
          ],
        },
      ]) {
        await t.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: DocumentSectionBreakDialog(key: UniqueKey(), data: data),
            ),
          ),
        );
        await t.pumpAndSettle();
        expect(
          t
              .widget<FilledButton>(
                find.byKey(const Key('section-break-apply')),
              )
              .onPressed,
          isNull,
        );
        expect(find.byKey(const Key('section-break-reason')), findsOneWidget);
        expect(t.takeException(), isNull);
      }
    },
  );
}
