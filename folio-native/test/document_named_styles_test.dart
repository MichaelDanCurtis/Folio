import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_list_rules.dart';
import 'package:folio_native/src/document_lists.dart';
import 'package:folio_native/src/document_named_styles.dart';
import 'package:folio_native/src/document_style_resolution.dart';
import 'package:folio_native/src/document_text_style.dart';
import 'package:folio_native/src/document_paragraph_format.dart';
import 'package:folio_native/src/document_styles.dart';
import 'package:folio_native/src/document_tabs.dart';
import 'package:folio_native/src/document_table_view.dart';

Map<String, dynamic> catalog() => {
  'defaultStyle': 'Normal',
  'defaults': {
    'paragraph': {'before': 0, 'after': 180, 'line': 336, 'lineRule': 'auto'},
    'tabs': [],
    'run': {
      'sz': 24,
      'rFonts': {'asciiTheme': 'minorHAnsi', 'hAnsiTheme': 'minorHAnsi'},
    },
  },
  'theme': {
    'fonts': {
      'minorFont': {'latin': 'Georgia'},
      'majorFont': {'latin': 'Arial'},
    },
    'colors': {'accent1': '185ABD'},
  },
  'styles': [
    {
      'id': 'Normal',
      'name': 'Normal',
      'type': 'paragraph',
      'default': true,
      'properties': {'paragraph': {}, 'tabs': [], 'run': {}},
    },
    {
      'id': 'Title',
      'name': 'Project title',
      'type': 'paragraph',
      'basedOn': 'Normal',
      'next': 'Normal',
      'properties': {
        'paragraph': {'after': 270},
        'outlineLevel': 0,
        'alignment': 'center',
        'tabs': [],
        'run': {
          'bold': true,
          'italic': true,
          'u': 'single',
          'strike': true,
          'sz': 48,
        },
      },
    },
    {
      'id': 'Child',
      'name': 'Child',
      'type': 'paragraph',
      'basedOn': 'Title',
      'properties': {
        'paragraph': {},
        'tabs': [],
        'run': {
          'bold': true,
          'rFonts': {'ascii': 'Arial'},
          'color': {'themeColor': 'accent1', 'val': '000000'},
        },
      },
    },
  ],
};
Map<String, dynamic> styledDoc() => {
  'type': 'doc',
  'attrs': {'wordStyles': catalog()},
  'content': [
    {
      'type': 'heading',
      'attrs': {
        'level': 1,
        'paragraphFormat': namedParagraphFormat(catalog(), 'Title'),
      },
      'content': [
        {'type': 'text', 'text': 'Styled '},
        {
          'type': 'text',
          'text': 'plain',
          'marks': [
            for (final key in ['bold', 'italic', 'underline', 'strike'])
              {
                'type': key,
                'attrs': {'value': false},
              },
          ],
        },
        {
          'type': 'text',
          'text': ' large',
          'marks': [
            {
              'type': 'textStyle',
              'attrs': {'fontSize': 18, 'fontFamily': 'Arial'},
            },
          ],
        },
      ],
    },
  ],
};
List<({String text, TextStyle style})> runs(
  InlineSpan span, [
  TextStyle parent = const TextStyle(),
]) {
  if (span is! TextSpan) return [];
  final style = parent.merge(span.style);
  return [
    if (span.text?.isNotEmpty == true) (text: span.text!, style: style),
    for (final child in span.children ?? <InlineSpan>[]) ...runs(child, style),
  ];
}

Widget host(Widget child) => MaterialApp(
  localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
  builder: (context, child) => q.QuillParagraphLayout(
    builder: layoutDocumentTabs,
    paddingBuilder: paragraphPadding,
    styleBuilder: paragraphTextStyle,
    inlineStyleBuilder: paragraphInlineStyle,
    alignmentBuilder: paragraphAlignment,
    strutBuilder: paragraphStrut,
    blockSpacingBuilder: paragraphBlockSpacing,
    child: child!,
  ),
  home: Scaffold(body: child),
);
void main() {
  test('false direct marks and style metadata survive Quill codec edits', () {
    final doc = styledDoc(), delta = documentToDelta(styledDoc());
    expect(delta.toList()[1].attributes!['bold'], false);
    expect(deltaToDocument(delta, attributes: doc['attrs']), doc);
    final controller = q.QuillController(
      document: q.Document.fromDelta(delta),
      selection: const TextSelection(baseOffset: 7, extentOffset: 12),
    );
    expect(documentMarkEnabled(controller, 'bold'), false);
    toggleDocumentMark(controller, 'bold');
    expect(documentMarkEnabled(controller, 'bold'), true);
    toggleDocumentMark(controller, 'bold');
    expect(documentMarkEnabled(controller, 'bold'), false);
    expect(
      controller.document.toDelta().slice(7, 12).first.attributes!['bold'],
      false,
    );
    controller.dispose();
  });
  test('style projection resolves toggles and literal/theme priority', () {
    final c = catalog(),
        normal = resolveNamedStyle(c, 'Normal'),
        child = resolveNamedStyle(c, 'Child');
    expect(normal['run']['rFonts']['ascii'], 'Georgia');
    expect(child['run']['rFonts']['ascii'], 'Arial');
    expect(child['run']['rFonts']['asciiTheme'], null);
    expect(child['run']['bold'], false);
    expect(child['run']['color']['val'], '185ABD');
    expect(namedParagraphFormat(c, 'Title')['inheritedAlignment'], 'center');
  });
  test(
    'Enter uses following styles, preserves identity once, and split/paste keep the style',
    () {
      final source = styledDoc();
      source['content'][0]['attrs']['officeId'] = 'original';
      for (final (offset, text, following) in [
        (18, '\n', true),
        (3, '\n', false),
        (18, 'a\nb\n', false),
      ]) {
        final doc = q.Document.fromDelta(documentToDelta(source));
        configureDocumentLists(doc, styleCatalog: catalog);
        final length = doc.length;
        doc.insert(offset, text);
        expect(doc.length, length + text.length);
        final lines = listLines(doc.toDelta());
        expect(
          listMetadata(lines.first.attrs['token'])['officeId'],
          'original',
        );
        expect(
          lines
              .skip(1)
              .every(
                (l) => !listMetadata(l.attrs['token']).containsKey('officeId'),
              ),
          true,
        );
        final last = listMetadata(lines.last.attrs['token'])['paragraphFormat'];
        expect(last['styleId'], following ? 'Normal' : 'Title');
        expect(lines.last.attrs['header'], following ? null : 1);
        expect(doc.toPlainText().replaceAll('\n', ''), contains('Styled'));
      }
    },
  );
  test(
    'main document and cell drafts keep their own following-style rules',
    () {
      final main = q.Document.fromDelta(documentToDelta(styledDoc()));
      final cell = q.Document.fromDelta(documentToDelta(styledDoc()));
      final other = catalog();
      other['styles'][1]['next'] = 'Child';
      configureDocumentLists(main, styleCatalog: catalog);
      configureDocumentLists(cell, styleCatalog: () => other);
      main.insert(main.length - 1, '\n');
      cell.insert(cell.length - 1, '\n');
      String following(q.Document doc) =>
          listMetadata(
                listLines(doc.toDelta()).last.attrs['token'],
              )['paragraphFormat']['styleId']
              as String;
      expect(following(main), 'Normal');
      expect(following(cell), 'Child');
      cell.close();
      main.insert(main.length - 1, 'Main continues\n');
      expect(following(main), 'Normal');
      main.close();
    },
  );
  test(
    'heading commands and list toggles retain named styling and editability',
    () {
      final c = catalog();
      c['styles'].add({
        'id': 'Heading2',
        'name': 'Heading 2',
        'type': 'paragraph',
        'basedOn': 'Normal',
        'next': 'Normal',
        'properties': {
          'paragraph': {'before': 180},
          'tabs': [],
          'outlineLevel': 1,
          'run': {'sz': 36},
        },
      });
      final doc = q.Document.fromDelta(documentToDelta(styledDoc()));
      configureDocumentLists(doc, styleCatalog: () => c);
      doc.format(0, 0, q.Attribute.ol);
      expect(listLines(doc.toDelta()).first.attrs['header'], 1);
      expect(listLines(doc.toDelta()).first.attrs['list'], 'ordered');
      final listed = deltaToDocument(doc.toDelta());
      expect(
        listed['content'][0]['content'][0]['content'][0]['type'],
        'heading',
      );
      expect(documentToDelta(listed), doc.toDelta());
      doc.format(0, 0, q.Attribute.h2);
      var line = listLines(doc.toDelta()).first;
      expect(line.attrs['list'], 'ordered');
      expect(line.attrs['header'], 2);
      var format = listMetadata(line.attrs['token'])['paragraphFormat'];
      expect(format['styleId'], 'Heading2');
      expect(format['inheritedRun']['sz'], 36);
      doc.format(0, 0, q.Attribute.header);
      line = listLines(doc.toDelta()).first;
      format = listMetadata(line.attrs['token'])['paragraphFormat'];
      expect(line.attrs['header'], null);
      expect(line.attrs['list'], 'ordered');
      expect(format['styleId'], 'Normal');
      expect(format['inheritedRun']['sz'], 24);
      expect(documentToDelta(deltaToDocument(doc.toDelta())), doc.toDelta());
      doc.close();
    },
  );
  testWidgets(
    'editor and static cell display inherited text and explicit off overrides',
    (tester) async {
      final doc = styledDoc();
      final controller = q.QuillController(
        document: q.Document.fromDelta(documentToDelta(doc)),
        selection: const TextSelection.collapsed(offset: 1),
      );
      final focus = FocusNode();
      await tester.pumpWidget(
        host(
          SizedBox(
            width: 650,
            child: Column(
              children: [
                DocumentTextToolbar(controller: controller, focus: focus),
                Expanded(
                  child: q.QuillEditor.basic(
                    controller: controller,
                    focusNode: focus,
                    config: q.QuillEditorConfig(
                      customStyles: documentEditorStyles(),
                      customShortcuts: documentMarkShortcuts,
                      customActions: documentMarkActions(controller, focus),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      var rendered = tester
          .widgetList<q.QuillLayoutText>(find.byType(q.QuillLayoutText))
          .where((w) => w.text.toPlainText().contains('Styled'))
          .first;
      var all = runs(rendered.text);
      var styled = all.firstWhere((r) => r.text.contains('Styled')).style;
      var plain = all.firstWhere((r) => r.text.contains('plain')).style;
      expect(styled.fontWeight, FontWeight.bold);
      expect(styled.fontFamily, 'Georgia');
      expect(styled.fontSize, 32);
      expect(rendered.textAlign, TextAlign.center);
      expect(styled.decoration!.contains(TextDecoration.underline), true);
      expect(plain.fontWeight, FontWeight.normal);
      expect(plain.fontStyle, FontStyle.normal);
      expect(plain.decoration, TextDecoration.none);
      expect(
        all.firstWhere((r) => r.text.contains('large')).style.fontSize,
        24,
      );
      controller.updateSelection(
        const TextSelection(baseOffset: 0, extentOffset: 6),
        q.ChangeSource.local,
      );
      await tester.pump();
      await tester.tap(find.byKey(const Key('document-mark-bold')));
      await tester.pumpAndSettle();
      expect(controller.document.toDelta().first.attributes!['bold'], false);
      await tester.pumpWidget(
        host(
          SizedBox(
            width: 650,
            child: DocumentTableView(
              node: {
                'type': 'table',
                'content': [
                  {
                    'type': 'tableRow',
                    'content': [
                      {'type': 'tableCell', 'content': doc['content']},
                    ],
                  },
                ],
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      rendered = tester
          .widgetList<q.QuillLayoutText>(find.byType(q.QuillLayoutText))
          .where((w) => w.text.toPlainText().contains('Styled'))
          .first;
      all = runs(rendered.text);
      expect(
        all.firstWhere((r) => r.text.contains('Styled')).style.fontWeight,
        FontWeight.bold,
      );
      expect(
        all.firstWhere((r) => r.text.contains('Styled')).style.fontSize,
        32,
      );
      expect(
        all.firstWhere((r) => r.text.contains('plain')).style.fontWeight,
        FontWeight.normal,
      );
      expect(
        all.firstWhere((r) => r.text.contains('plain')).style.decoration,
        TextDecoration.none,
      );
      expect(tester.takeException(), null);
      await tester.pumpWidget(const SizedBox.shrink());
      controller.dispose();
      focus.dispose();
    },
  );
  testWidgets(
    'style editor validates values and preserves untouched declarations',
    (tester) async {
      final c = catalog();
      final data = {
        'catalog': c,
        'resolved': {
          for (final s in c['styles']) s['id']: resolveNamedStyle(c, s['id']),
        },
        'defaults': resolveNamedStyle(c, ''),
      };
      Map? result;
      await tester.pumpWidget(
        host(
          Builder(
            builder: (context) => TextButton(
              onPressed: () async {
                result = await showDialog<Map>(
                  context: context,
                  builder: (_) => DocumentStyleEditor(
                    data: data,
                    style: styleCopy(c['styles'][1]),
                    create: false,
                  ),
                );
              },
              child: const Text('Edit'),
            ),
          ),
        ),
      );
      await tester.tap(find.text('Edit'));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('style-editor-size')),
        'Infinity',
      );
      await tester.pump();
      expect(
        tester
            .widget<FilledButton>(find.byKey(const Key('style-editor-save')))
            .onPressed,
        null,
      );
      expect(tester.takeException(), null);
      await tester.enterText(find.byKey(const Key('style-editor-size')), '18');
      await tester.enterText(
        find.byKey(const Key('style-editor-name')),
        'Renamed title',
      );
      await tester.ensureVisible(find.byKey(const Key('style-editor-save')));
      await tester.tap(find.byKey(const Key('style-editor-save')));
      await tester.pumpAndSettle();
      expect(result?['id'], 'Title');
      expect(result?['name'], 'Renamed title');
      expect(result?['properties']['run']['sz'], 36);
      expect(result?['properties']['run']['bold'], true);
      expect(result?['next'], 'Normal');
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
}
