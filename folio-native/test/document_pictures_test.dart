import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_bookmarks.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_view.dart';

final doc = {
  'type': 'doc',
  'content': [
    {
      'type': 'paragraph',
      'content': [
        {'type': 'text', 'text': 'See '},
        {
          'type': 'image',
          'attrs': {
            'src': 'word/media/image1.png',
            'width': 40,
            'height': 20,
            'alt': 'Logo',
            'officeXml': '<w:drawing/>',
          },
        },
        {'type': 'text', 'text': ' here'},
      ],
    },
  ],
};

void main() {
  test('pictures round-trip through the editor as one character', () {
    final delta = documentToDelta(doc);
    expect(delta.toList()[1].data, isA<Map>());
    expect(delta.toList()[1].length, 1);
    expect(deltaToDocument(delta), doc);
  });

  test('a picture has no text for bookmarks and comments', () {
    final index = DocumentBookmarkIndex(doc);
    expect(index.text, 'See  here\n');
    // Editor offset 5 is after the picture; its text offset is 4.
    expect(index.logicalOffset(5), 4);
    expect(index.logicalOffset(4), 4);
    expect(index.editorSelection(4, 5), isNotNull);
  });

  testWidgets('a picture shows at its size and deletes as one character', (
    t,
  ) async {
    final controller = q.QuillController(
      document: q.Document.fromDelta(documentToDelta(doc)),
      selection: const TextSelection.collapsed(offset: 0),
    );
    addTearDown(controller.dispose);
    await t.pumpWidget(
      MaterialApp(
        localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
        home: Scaffold(
          body: DocumentView(controller: controller, undo: () {}, redo: () {}),
        ),
      ),
    );
    await t.pumpAndSettle();
    final picture = find.byKey(
      const ValueKey('document-picture-word/media/image1.png'),
    );
    expect(t.getSize(picture), const Size(40, 20));
    expect(t.widget<Semantics>(picture).properties.label, 'Logo');
    // Without a workspace there is nowhere to store a new picture.
    expect(
      t.widget<IconButton>(find.byKey(const Key('insert-picture'))).onPressed,
      isNull,
    );
    controller.replaceText(4, 1, '', const TextSelection.collapsed(offset: 4));
    await t.pump();
    expect(picture, findsNothing);
    expect(controller.document.toPlainText(), 'See  here\n');
  });
}
