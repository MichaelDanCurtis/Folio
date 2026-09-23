import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_tabs.dart';

void main() {
  Map stop(int position, [String alignment = 'left']) =>
      DocumentTabStop(position, alignment: alignment).toJson();
  test(
    'selected paragraphs clear each inherited stop without altering other metadata or next paragraph',
    () {
      final doc = {
        'type': 'doc',
        'attrs': {'defaultTabStop': 960},
        'content': [
          {
            'type': 'paragraph',
            'attrs': {
              'officeId': 'a',
              'tabs': {
                'inherited': [stop(720)],
                'direct': [stop(1440)],
              },
            },
            'content': [
              {
                'type': 'text',
                'text': 'A\tB',
                'marks': [
                  {'type': 'bold'},
                ],
              },
            ],
          },
          {
            'type': 'paragraph',
            'attrs': {
              'officeId': 'b',
              'tabs': {
                'inherited': [stop(1000, 'center')],
                'direct': [],
              },
            },
            'content': [
              {'type': 'text', 'text': 'C\tD'},
            ],
          },
          {
            'type': 'paragraph',
            'content': [
              {'type': 'text', 'text': 'Outside'},
            ],
          },
        ],
      };
      final before = documentToDelta(doc);
      final change = changeTabStops(
        before,
        const TextSelection(baseOffset: 0, extentOffset: 8),
        const [DocumentTabStop(2880, alignment: 'right')],
      );
      final after = deltaToDocument(
        before.compose(change),
        attributes: doc['attrs'] as Map,
      );
      final nodes = after['content'] as List;
      expect(after['attrs'], doc['attrs']);
      expect(nodes[0]['attrs']['officeId'], 'a');
      expect(nodes[0]['content'][0]['marks'], [
        {'type': 'bold'},
      ]);
      expect(nodes[0]['attrs']['tabs']['direct'], [
        stop(720, 'clear'),
        stop(2880, 'right'),
      ]);
      expect(nodes[1]['attrs']['tabs']['direct'], [
        stop(1000, 'clear'),
        stop(2880, 'right'),
      ]);
      expect(nodes[2], (doc['content'] as List)[2]);
      expect(before.compose(change).compose(change.invert(before)), before);
      expect(jsonEncode(doc['attrs']), '{"defaultTabStop":960}');
      expect(
        changeTabStops(
          before,
          const TextSelection.collapsed(offset: 0),
          effectiveTabStops((doc['content'] as List)[0]['attrs']['tabs']),
        ).isEmpty,
        isTrue,
      );
    },
  );

  test('tab formatting rejects selected embedded tables', () {
    final delta = Delta()
      ..insert({
        'folioNode': jsonEncode({'type': 'table'}),
      })
      ..insert('\n');
    expect(
      () => changeTabStops(
        delta,
        const TextSelection.collapsed(offset: 0),
        const [],
      ),
      throwsFormatException,
    );
  });

  test(
    'custom stops align text, then default tabs resume at interval multiples',
    () {
      for (final (alignment, start, end, text) in [
        ('left', 144.0, 192.0, '123'),
        ('right', 96.0, 144.0, '123'),
        ('center', 120.0, 168.0, '123'),
        ('decimal', 112.0, 192.0, '12.34'),
      ]) {
        final source = TextSpan(
          text: 'A\t$text\tZ',
          style: const TextStyle(fontSize: 16),
        );
        final painter = TextPainter(
          text: layoutTabStops(
            source,
            maxWidth: 500,
            interval: 40,
            stops: [DocumentTabStop(2160, alignment: alignment)],
          ),
          textDirection: TextDirection.ltr,
        )..layout(maxWidth: 500);
        expect(
          painter
              .getOffsetForCaret(const TextPosition(offset: 2), Rect.zero)
              .dx,
          closeTo(start, .05),
          reason: alignment,
        );
        expect(
          painter
              .getOffsetForCaret(
                TextPosition(offset: 2 + text.length),
                Rect.zero,
              )
              .dx,
          closeTo(end, .05),
          reason: alignment,
        );
        final automatic = ((end + .01) / 40).floor() * 40 + 40;
        expect(
          painter
              .getOffsetForCaret(
                TextPosition(offset: 3 + text.length),
                Rect.zero,
              )
              .dx,
          closeTo(automatic, .05),
          reason: alignment,
        );
        expect(painter.text!.toPlainText(), source.toPlainText());
        painter.dispose();
      }
    },
  );

  test(
    'decimal alignment measures styled prefix and uses locale decimal separator',
    () {
      const source = TextSpan(
        style: TextStyle(fontSize: 16),
        children: [
          TextSpan(text: 'A\t'),
          TextSpan(text: '12', style: TextStyle(fontSize: 24)),
          TextSpan(text: ',34'),
        ],
      );
      final painter = TextPainter(
        text: layoutTabStops(
          source,
          maxWidth: 500,
          locale: const Locale('de'),
          stops: const [DocumentTabStop(3000, alignment: 'decimal')],
        ),
        textDirection: TextDirection.ltr,
      )..layout(maxWidth: 500);
      expect(
        painter.getOffsetForCaret(const TextPosition(offset: 4), Rect.zero).dx,
        closeTo(200, .05),
      );
      expect(painter.text!.toPlainText(), 'A\t12,34');
      painter.dispose();
    },
  );
}
