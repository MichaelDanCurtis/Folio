import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/src/document_anchor.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/assistant_ui.dart';
import 'package:folio_native/src/core.dart';

void main() {
  test('empty paragraph remains a valid insertion anchor',(){final editor=q.QuillController.basic();final anchor=DocumentAnchor.capture(editor,0,'w');expect(anchor.text,'');expect(anchor.start,0);expect(anchor.end,0);editor.dispose();});
  test(
    'paragraph selection preserves surrounding styles and rejects stale revisions',
    () {
      final editor = q.QuillController(
        document: q.Document.fromJson([
          {
            'insert': 'Keep bold',
            'attributes': {'bold': true},
          },
          {'insert': '\nTarget 👋 passage\nKeep after\n'},
        ]),
        selection: const TextSelection.collapsed(offset: 15),
      );
      final anchor = DocumentAnchor.capture(editor, 7, 'w');
      expect(anchor.text, 'Target 👋 passage');
      final result = anchor.replace(
        editor,
        currentRevision: 7,
        currentWorkspaceId: 'w',
        expectedText: anchor.text,
        replacement: 'Revenue is 1200.',
      );
      expect(result['content'][0]['content'][0]['marks'], [
        {'type': 'bold'},
      ]);
      expect(nodeText(result), contains('Revenue is 1200.\nKeep after'));
      expect(
        () => anchor.replace(
          editor,
          currentRevision: 8,
          currentWorkspaceId: 'w',
          expectedText: anchor.text,
          replacement: 'Wrong',
        ),
        throwsStateError,
      );
      editor.dispose();
    },
  );
  test('Office paragraph identity and formatting survive Quill', () {
    final source = {
      'type': 'doc',
      'content': [
        {
          'type': 'paragraph',
          'attrs': {'officeId': 'word-1', 'textAlign': 'center'},
          'content': [
            {
              'type': 'text',
              'text': 'Formatted',
              'marks': [
                {'type': 'bold'},
                {
                  'type': 'textStyle',
                  'attrs': {
                    'fontFamily': 'Arial',
                    'fontSize': 14.0,
                    'color': '#185ABD',
                  },
                },
              ],
            },
          ],
        },
      ],
    };
    final doc = q.Document.fromDelta(documentToDelta(source));
    expect(deltaToDocument(doc.toDelta()), source);
  });
  testWidgets(
    'actual A2UI renderer displays choices and returns user actions',
    (tester) async {
      final model = WorkspaceModel(CoreBridge(), '/unused');
      final actions = <Map>[];
      final ui = AssistantUi(
        model: model,
        onAction: actions.add,
        onOpen: (_, _) {},
      );
      ui.showChoices(
        id: 'test',
        title: 'Use selected data',
        choices: [('Write a summary', 'summarize')],
      );
      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: ui.surface('test'))),
      );
      await tester.pumpAndSettle();
      expect(find.text('Use selected data'), findsOneWidget);
      await tester.tap(find.text('Write a summary'));
      await tester.pumpAndSettle();
      expect(actions.single['name'], 'summarize');
      expect(
        () => ui.present([
          {
            'version': 'v0.9',
            'updateComponents': {
              'surfaceId': 'test',
              'components': [
                {
                  'id': 'root',
                  'component': 'Column',
                  'children': ['root'],
                },
              ],
            },
          },
        ]),
        throwsFormatException,
      );
      expect(
        () => ui.present([
          {
            'version': 'v0.9',
            'updateComponents': {
              'surfaceId': 'test',
              'components': [
                {
                  'id': 'root',
                  'component': 'WebView',
                  'url': 'https://example.com',
                },
              ],
            },
          },
        ]),
        throwsFormatException,
      );
      await tester.pumpWidget(const SizedBox());
      ui.dispose();
      model.dispose();
    },
  );
}
