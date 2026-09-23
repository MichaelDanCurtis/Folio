import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_review_view.dart';

void main() {
  testWidgets(
    'review navigation reveals distant paragraphs and failed views retain the displayed mode',
    (tester) async {
      tester.view.physicalSize = const Size(1100, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      var actions = 0;
      final data = {
        'workspaceId': 'test',
        'baseRevision': 3,
        'view': {
          'document': {
            'type': 'doc',
            'content': [
              for (var i = 0; i < 90; i++)
                {
                  'type': 'paragraph',
                  'attrs': {
                    'reviewBlocks': ['word-$i'],
                  },
                  'content': [
                    {'type': 'text', 'text': 'Paragraph $i'},
                  ],
                },
            ],
          },
        },
        'revisions': [
          for (final i in [0, 89])
            {
              'key': '$i',
              'officeId': 'word-$i',
              'author': i == 0 ? 'Alex' : 'Morgan',
              'kind': 'Inserted content',
              'story': 'Document',
              'date': '',
              'preview': [
                {'text': 'Paragraph $i', 'kind': 'insert'},
              ],
              'canAccept': false,
              'canReject': false,
              'acceptReason': 'Protected',
              'rejectReason': 'Protected',
            },
        ],
      };
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: DocumentReviewView(
              revision: 3,
              read: (mode) async {
                if (mode == 'original') {
                  throw StateError('Unsupported original view');
                }
                return data;
              },
              review: (_) async {
                actions++;
              },
              close: () {},
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('review-page-next')));
      await tester.pumpAndSettle();
      expect(find.text('2 of 2 changes'), findsOneWidget);
      final last = tester.getRect(
        find.byKey(const ValueKey('review-block-word-89')),
      );
      expect(last.top, greaterThanOrEqualTo(0));
      expect(last.bottom, lessThan(800));
      expect(
        tester
            .widget<FilledButton>(find.byKey(const Key('review-page-accept')))
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<OutlinedButton>(find.byKey(const Key('review-page-reject')))
            .onPressed,
        isNull,
      );
      expect(actions, 0);
      await tester.tap(find.byKey(const Key('review-display-mode')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Original').last);
      await tester.pumpAndSettle();
      expect(find.textContaining('Unsupported original view'), findsOneWidget);
      expect(
        tester
            .widget<DropdownButton<String>>(
              find.byKey(const Key('review-display-mode')),
            )
            .value,
        'markup',
      );
      expect(
        find.byKey(const ValueKey('review-block-word-89')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );
}
