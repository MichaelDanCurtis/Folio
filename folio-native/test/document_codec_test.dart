import 'dart:convert';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/src/document_codec.dart';

void main() {
  test(
    'word count joins inline runs and separates blocks and merged cells',
    () {
      Map paragraph(List<Map> content) => {
        'type': 'paragraph',
        'content': content,
      };
      Map word(String text) => {'type': 'text', 'text': text};
      final doc = {
        'type': 'doc',
        'content': [
          paragraph([
            word('A'),
            {
              ...word('word'),
              'marks': [
                {'type': 'bold'},
              ],
            },
            {'type': 'hardBreak'},
            word('next'),
          ]),
          {
            'type': 'table',
            'content': [
              {
                'type': 'tableRow',
                'content': [
                  {
                    'type': 'tableCell',
                    'attrs': {
                      'colspan': 2,
                      'mergeContinuations': {
                        'retained': paragraph([word('not counted')]),
                      },
                    },
                    'content': [
                      paragraph([word('Cell')]),
                      paragraph([word('paragraph')]),
                    ],
                  },
                  {
                    'type': 'tableCell',
                    'content': [
                      paragraph([word('Neighbor')]),
                    ],
                  },
                ],
              },
            ],
          },
        ],
      };
      expect(documentWordCount(doc), 5);
      expect(documentWordCount({'type': 'doc', 'content': []}), 0);
    },
  );

  test('legacy document survives native editor round trip', () {
    final source = jsonDecode(
      File('rust/src/seed.json').readAsStringSync(),
    )['document'];
    final editor = q.Document.fromDelta(documentToDelta(source));
    expect(deltaToDocument(editor.toDelta()), source);
  });
  test(
    'tables, nested lists and unknown formatting survive adjacent edits',
    () {
      final blocks = [
        {
          'type': 'table',
          'attrs': {'layout': 'custom'},
          'content': [
            {
              'type': 'tableRow',
              'content': [
                {
                  'type': 'tableCell',
                  'attrs': {'colspan': 2, 'rowspan': 1},
                  'content': [
                    {
                      'type': 'paragraph',
                      'content': [
                        {
                          'type': 'text',
                          'text': 'Kept',
                          'marks': [
                            {'type': 'bold'},
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
        {
          'type': 'paragraph',
          'attrs': {'textAlign': 'center'},
          'content': [
            {'type': 'text', 'text': 'Centered'},
          ],
        },
        {
          'type': 'orderedList',
          'attrs': {'start': 3},
          'content': [
            {
              'type': 'listItem',
              'content': [
                {
                  'type': 'paragraph',
                  'content': [
                    {'type': 'text', 'text': 'Third'},
                  ],
                },
              ],
            },
          ],
        },
        {
          'type': 'blockquote',
          'content': [
            {
              'type': 'paragraph',
              'content': [
                {'type': 'text', 'text': 'Quote'},
              ],
            },
          ],
        },
      ];
      final document = {'type': 'doc', 'content': blocks};
      final delta = documentToDelta(document)..insert('New paragraph\n');
      final editor = q.Document.fromDelta(delta);
      final result = deltaToDocument(editor.toDelta());
      expect((result['content'] as List).take(4).toList(), blocks);
      expect(nodeText(result), contains('New paragraph'));
    },
  );
}
