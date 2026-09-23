import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_lists.dart';
import 'package:folio_native/src/document_list_rules.dart';

void main() {
  Map paragraph(String text, Map numbering) => {
    'type': 'paragraph',
    'attrs': {'numbering': numbering},
    'content': [
      {
        'type': 'text',
        'text': text,
        'marks': [
          {'type': 'bold'},
        ],
      },
    ],
  };
  test(
    'imported list counters honor starts, instances, prose and restart rules',
    () {
      final levels = defaultListLevels(start: 5);
      levels[1] = {...levels[1], 'restart': 0, 'text': '%1.%2)'};
      Map n(int level, {String id = '7', List? defs}) => {
        'numId': id,
        'level': level,
        'levels': defs ?? levels,
      };
      final doc = {
        'type': 'doc',
        'content': [
          paragraph('Parent', n(0)),
          paragraph('Child', n(1)),
          paragraph('Child two', n(1)),
          {
            'type': 'paragraph',
            'content': [
              {'type': 'text', 'text': 'Prose'},
            ],
          },
          paragraph('Next parent', n(0)),
          paragraph('Continued child', n(1)),
          paragraph(
            'New instance',
            n(0, id: '8', defs: defaultListLevels(start: 2)),
          ),
        ],
      };
      final delta = documentToDelta(doc);
      final editor = q.Document.fromDelta(delta);
      expect(deltaToDocument(editor.toDelta()), doc);
      expect(documentListLabels(delta).values.toList(), [
        '5.',
        '5.a)',
        '5.b)',
        '6.',
        '6.c)',
        '2.',
      ]);
      final roman = defaultListLevels(start: 4);
      roman[0]['format'] = 'upperRoman';
      roman[0]['text'] = '%1)';
      expect(
        documentListLabels(
          documentToDelta({
            'type': 'doc',
            'content': [paragraph('Roman', n(0, defs: roman))],
          }),
        ).values.single,
        'IV)',
      );
      expect(listNumber(27, 'upperLetter'), 'AA');
      expect(listNumber(3999, 'lowerRoman'), 'mmmcmxcix');
      final restart = jsonDecode(jsonEncode(levels)) as List;
      restart[1]['restart'] = 1;
      expect(
        documentListLabels(
          documentToDelta({
            'type': 'doc',
            'content': [
              paragraph('A', n(0, defs: restart)),
              paragraph('a', n(1, defs: restart)),
              paragraph('B', n(0, defs: restart)),
              paragraph('b', n(1, defs: restart)),
            ],
          }),
        ).values.toList(),
        ['5.', '5.a)', '6.', '6.a)'],
      );
    },
  );
  test(
    'nested native lists round trip without losing indentation or rich text',
    () {
      final delta = Delta()
        ..insert('First', {'bold': true})
        ..insert('\n', {'list': 'ordered'})
        ..insert('Nested')
        ..insert('\n', {'list': 'bullet', 'indent': 1})
        ..insert('Deeper')
        ..insert('\n', {'list': 'ordered', 'indent': 2})
        ..insert('Second')
        ..insert('\n', {'list': 'ordered'})
        ..insert('Prose\n');
      final canonical = deltaToDocument(delta);
      expect(canonical['content'][0]['content'].length, 2);
      expect(
        canonical['content'][0]['content'][0]['content'][1]['type'],
        'bulletList',
      );
      expect(documentToDelta(canonical), delta);
      final standalone = Delta()
        ..insert('Indented')
        ..insert('\n', {'list': 'ordered', 'indent': 3});
      expect(documentToDelta(deltaToDocument(standalone)), standalone);
    },
  );
  test(
    'removing and restoring imported numbering stays editable and explicit',
    () {
      final source = {
        'type': 'doc',
        'content': [
          paragraph('Text', {
            'numId': '7',
            'level': 0,
            'levels': defaultListLevels(),
          }),
        ],
      };
      final editor = q.QuillController(
        document: q.Document.fromDelta(documentToDelta(source)),
        selection: const TextSelection.collapsed(offset: 0),
      );
      editor.formatText(0, 0, const q.ListAttribute(null));
      final plain = deltaToDocument(editor.document.toDelta());
      expect(plain['content'][0]['attrs'].containsKey('numbering'), isTrue);
      expect(plain['content'][0]['attrs']['numbering'], isNull);
      editor.document = q.Document.fromDelta(documentToDelta(plain));
      editor.formatText(0, 0, q.Attribute.ol);
      final restored = deltaToDocument(editor.document.toDelta());
      expect(restored['content'][0]['type'], 'orderedList');
      expect(
        restored['content'][0]['content'][0]['content'][0]['content'][0]['marks'],
        [
          {'type': 'bold'},
        ],
      );
      editor.dispose();
    },
  );
  test(
    'typing and multiline paste continue the imported numbering instance',
    () {
      final source = {
        'type': 'doc',
        'content': [
          paragraph('First', {
            'numId': '7',
            'level': 0,
            'levels': defaultListLevels(start: 5),
          }),
        ],
      };
      final editor = q.QuillController(
        document: q.Document.fromDelta(documentToDelta(source)),
        selection: const TextSelection.collapsed(offset: 5),
      );
      configureDocumentLists(editor.document);
      editor.replaceText(
        5,
        0,
        '\nSecond\nThird',
        const TextSelection.collapsed(offset: 18),
      );
      expect(documentListLabels(editor.document.toDelta()).values.toList(), [
        '5.',
        '6.',
        '7.',
      ]);
      final roundTrip = deltaToDocument(editor.document.toDelta());
      expect(roundTrip['content'].length, 3);
      expect(
        roundTrip['content']
            .map((n) => n['attrs']['numbering']['numId'])
            .toList(),
        ['7', '7', '7'],
      );
      editor.replaceText(editor.document.length - 1, 0, '\n', null);
      expect(documentListLabels(editor.document.toDelta()).values.last, '8.');
      editor.replaceText(editor.document.length - 1, 0, '\n', null);
      expect(listLines(editor.document.toDelta()).last.attrs['list'], isNull);
      editor.dispose();
    },
  );
  test(
    'restart preserves punctuation and leaves adjacent list instances alone',
    () {
      final roman = defaultListLevels(start: 4);
      roman[0]['format'] = 'upperRoman';
      roman[0]['text'] = 'Article %1)';
      final doc = {
        'type': 'doc',
        'content': [
          paragraph('First', {'numId': '7', 'level': 0, 'levels': roman}),
          paragraph('Next', {
            'numId': '8',
            'level': 0,
            'levels': defaultListLevels(start: 10),
          }),
        ],
      };
      final delta = documentToDelta(doc);
      final changed = delta.compose(
        changeNumbering(delta, 0, format: 'upperRoman', instance: 'restarted'),
      );
      expect(documentListLabels(changed).values.toList(), [
        'Article I)',
        '10.',
      ]);
      expect(
        deltaToDocument(changed)['content'][1],
        (doc['content'] as List)[1],
      );
    },
  );
  test(
    'restart and continuation are single deltas without changing text or marks',
    () {
      final delta = Delta()
        ..insert('One', {'italic': true})
        ..insert('\n', {'list': 'ordered'})
        ..insert('Two\n', {'list': 'ordered'})
        ..insert('Prose\n')
        ..insert('Next\n', {'list': 'ordered'})
        ..insert('Last\n', {'list': 'ordered'});
      final start = delta
          .toList()
          .fold<String>(
            '',
            (s, op) => s + (op.data is String ? op.data as String : ''),
          )
          .indexOf('Next');
      final continued = delta.compose(
        changeNumbering(delta, start, continuePrevious: true, instance: 'test'),
      );
      expect(documentListLabels(continued).values.toList(), [
        '1.',
        '2.',
        '3.',
        '4.',
      ]);
      final restarted = continued.compose(
        changeNumbering(
          continued,
          start,
          start: 7,
          format: 'upperLetter',
          instance: 'restart',
        ),
      );
      expect(documentListLabels(restarted).values.toList(), [
        '1.',
        '2.',
        'G.',
        'H.',
      ]);
      expect(nodeText(deltaToDocument(restarted)), contains('Next'));
      expect(restarted.first.attributes, {'italic': true});
      expect(
        () =>
            changeNumbering(delta, 0, continuePrevious: true, instance: 'test'),
        throwsFormatException,
      );
      expect(
        () => changeNumbering(delta, start, start: -1, instance: 'test'),
        throwsFormatException,
      );
      expect(
        () => changeNumbering(delta, 8, instance: 'test'),
        throwsFormatException,
      );
    },
  );
}
