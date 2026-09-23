import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_list_rules.dart';
import 'package:folio_native/src/document_lists.dart';
import 'package:folio_native/src/document_paragraph_format.dart';
import 'package:folio_native/src/document_styles.dart';
import 'package:folio_native/src/search.dart';
import 'document_named_styles_test.dart' as helpers;

void main() {
  test(
    'outline overrides change structure independently of styles and runs',
    () {
      final doc = helpers.styledDoc();
      final source = documentToDelta(doc);
      for (var level = 0; level <= 9; level++) {
        final delta = source.compose(
          changeParagraphFormat(
            source,
            const TextSelection.collapsed(offset: 0),
            {'outlineLevel': level},
          ),
        );
        final next = deltaToDocument(delta, attributes: doc['attrs']);
        final p = next['content'][0];
        expect(p['attrs']['paragraphFormat']['styleId'], 'Title');
        expect(p['content'], doc['content'][0]['content']);
        expect(p['type'], level == 9 ? 'paragraph' : 'heading');
        expect(
          DocumentSearchIndex(next).blocks.first.heading,
          level == 9 ? 0 : level + 1,
        );
        final cleared = deltaToDocument(
          delta.compose(
            changeParagraphFormat(
              delta,
              const TextSelection.collapsed(offset: 0),
              {'outlineLevel': null},
            ),
          ),
          attributes: doc['attrs'],
        );
        expect(cleared, doc);
      }
    },
  );
  test(
    'Enter, multiline text and heading shortcuts preserve a direct override',
    () {
      for (final level in [0, 8, 9]) {
        for (final insertion in ['\n', 'more\ntext\n']) {
          final doc = helpers.styledDoc();
          final source = documentToDelta(doc);
          final d = q.Document.fromDelta(
            source.compose(
              changeParagraphFormat(
                source,
                const TextSelection.collapsed(offset: 0),
                {'outlineLevel': level},
              ),
            ),
          );
          configureDocumentLists(d, styleCatalog: helpers.catalog);
          d.insert(d.length - 1, insertion);
          final lines = listLines(d.toDelta());
          for (final line in lines) {
            expect(line.attrs['header'], level == 9 ? null : level + 1);
            expect(
              listMetadata(
                line.attrs['token'],
              )['paragraphFormat']['direct']['outlineLevel'],
              level,
            );
          }
          final c = q.QuillController(
            document: d,
            selection: const TextSelection.collapsed(offset: 0),
          );
          c.formatSelection(const q.HeaderAttribute(level: 3));
          final first = listLines(d.toDelta()).first;
          expect(first.attrs['header'], level == 9 ? null : level + 1);
          expect(
            listMetadata(first.attrs['token'])['paragraphFormat']['styleId'],
            'Heading3',
          );
          c.dispose();
        }
      }
    },
  );
  testWidgets('all outline levels retain identical rendered typography', (
    tester,
  ) async {
    for (final native in [false, true]) {
      final source = native
          ? documentToDelta({
              'type': 'doc',
              'content': [
                {
                  'type': 'heading',
                  'attrs': {'level': 1},
                  'content': [
                    {'type': 'text', 'text': 'Appearance stays'},
                  ],
                },
              ],
            })
          : documentToDelta({
              'type': 'doc',
              'content': [
                {
                  'type': 'paragraph',
                  'attrs': {
                    'paragraphFormat': {
                      'styleId': 'Body',
                      'inherited': {
                        'before': 0,
                        'after': 0,
                        'line': 240,
                        'lineRule': 'auto',
                      },
                      'direct': {},
                      'inheritedRun': {
                        'sz': 28,
                        'rFonts': {'ascii': 'Arial'},
                      },
                    },
                  },
                  'content': [
                    {'type': 'text', 'text': 'Appearance stays'},
                  ],
                },
              ],
            });
      for (var level = 0; level <= 9; level++) {
        final delta = source.compose(
          changeParagraphFormat(
            source,
            const TextSelection.collapsed(offset: 0),
            {'outlineLevel': level},
          ),
        );
        final controller = q.QuillController(
          document: q.Document.fromDelta(delta),
          selection: const TextSelection.collapsed(offset: 0),
        );
        await tester.pumpWidget(
          helpers.host(
            q.QuillEditor.basic(
              controller: controller,
              config: q.QuillEditorConfig(
                customStyles: documentEditorStyles(),
                scrollable: false,
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final run = tester
            .widgetList<q.QuillLayoutText>(find.byType(q.QuillLayoutText))
            .expand((w) => helpers.runs(w.text))
            .firstWhere((r) => r.text.contains('Appearance stays'));
        expect(
          run.style.fontSize,
          closeTo(native ? 32 : 28 / 2 * 4 / 3, 0.001),
        );
        expect(run.style.fontWeight, FontWeight.normal);
        expect(run.style.fontStyle, anyOf(isNull, FontStyle.normal));
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
        controller.dispose();
      }
    }
  });
  test(
    'mixed selection preserves lists, source identities and explicit body text',
    () {
      final doc = helpers.styledDoc();
      doc['content'] = <dynamic>[
        ...doc['content'],
        jsonDecode(jsonEncode(doc['content'][0])),
      ];
      var delta = documentToDelta(doc);
      delta = delta.compose(
        changeParagraphFormat(
          delta,
          TextSelection(baseOffset: 0, extentOffset: listLines(delta).last.end),
          {'outlineLevel': 9},
        ),
      );
      for (final line in listLines(delta)) {
        expect(line.attrs['header'], isNull);
        expect(
          listMetadata(line.attrs['token'])['paragraphFormat']['styleId'],
          'Title',
        );
      }
      expect(
        DocumentSearchIndex(
          deltaToDocument(delta),
        ).blocks.every((b) => b.heading == 0),
        isTrue,
      );
    },
  );
}
