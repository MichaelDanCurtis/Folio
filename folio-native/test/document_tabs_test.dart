import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_tabs.dart';

void main() {
  TextPainter layout(
    InlineSpan source, {
    double width = 400,
    double interval = 48,
    TextScaler scaler = TextScaler.noScaling,
  }) {
    final result = layoutTabStops(
      source,
      maxWidth: width,
      interval: interval,
      textScaler: scaler,
    );
    expect(
      result.toPlainText(includeSemanticsLabels: false),
      source.toPlainText(includeSemanticsLabels: false),
    );
    final painter = TextPainter(
      text: result,
      textDirection: TextDirection.ltr,
      textScaler: scaler,
    )..layout(maxWidth: width);
    addTearDown(painter.dispose);
    return painter;
  }

  Offset caret(TextPainter painter, int offset) =>
      painter.getOffsetForCaret(TextPosition(offset: offset), Rect.zero);

  test(
    'literal tabs advance to the next stop, including repeated and final tabs',
    () {
      const source = TextSpan(
        text: '\tA\tB\t\t',
        style: TextStyle(fontSize: 16),
      );
      final p = layout(source);
      expect(caret(p, 1).dx, closeTo(48, .05));
      expect(caret(p, 3).dx, closeTo(96, .05));
      expect(caret(p, 5).dx, closeTo(144, .05));
      expect(p.width, closeTo(192, .05));
      // Each tab remains one selectable character with an actual hit-testable box.
      final box = p
          .getBoxesForSelection(
            const TextSelection(baseOffset: 2, extentOffset: 3),
          )
          .single;
      expect(box.left, closeTo(64, .05));
      expect(box.right, closeTo(96, .05));
    },
  );

  test(
    'mixed fonts, Unicode, recognizers and proofing styles retain offsets',
    () {
      final link = TapGestureRecognizer();
      addTearDown(link.dispose);
      final source = TextSpan(
        style: const TextStyle(fontSize: 16),
        children: [
          const TextSpan(text: '🙂e\u0301'),
          TextSpan(
            text: '\tLink',
            recognizer: link,
            style: const TextStyle(
              fontSize: 24,
              fontWeight: FontWeight.bold,
              decoration: TextDecoration.underline,
              decorationStyle: TextDecorationStyle.wavy,
            ),
          ),
          const TextSpan(text: '\tEnd', style: TextStyle(fontSize: 12)),
        ],
      );
      final p = layout(source);
      final text = source.toPlainText();
      for (final i in [text.indexOf('\t'), text.lastIndexOf('\t')]) {
        expect(caret(p, i + 1).dx % 48, closeTo(0, .05));
      }
      final span =
          p.text!.getSpanForPosition(const TextPosition(offset: 6)) as TextSpan;
      expect(span.recognizer, same(link));
      expect(span.style!.decorationStyle, TextDecorationStyle.wavy);
    },
  );

  test('resize and text scaling use shaped visual rows', () {
    const source = TextSpan(
      text: 'one two three four\tX\tZ',
      style: TextStyle(fontSize: 16),
    );
    for (final width in [400.0, 176.0, 96.0]) {
      final p = layout(source, width: width);
      for (final i in [18, 20]) {
        final box = p
            .getBoxesForSelection(
              TextSelection(baseOffset: i, extentOffset: i + 1),
            )
            .single;
        expect(box.right, lessThanOrEqualTo(width + .05));
        expect(
          box.right % 48,
          anyOf(closeTo(0, .05), closeTo(width % 48, .05)),
        );
      }
    }
    final p = layout(
      const TextSpan(text: 'A\tB', style: TextStyle(fontSize: 16)),
      scaler: TextScaler.linear(2),
    );
    expect(caret(p, 2).dx, closeTo(48, .05));
  });

  test('text without tabs retains its exact span tree', () {
    const source = TextSpan(text: 'Unchanged', style: TextStyle(fontSize: 16));
    expect(layoutTabStops(source, maxWidth: 100), same(source));
  });
  testWidgets(
    'spacer layout supports actual rendering and nearest cursor edge',
    (tester) async {
      const source = TextSpan(text: 'A\tB', style: TextStyle(fontSize: 16));
      final span = layoutTabStops(source, maxWidth: 400);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(width: 400, child: q.QuillLayoutText(text: span)),
          ),
        ),
      );
      final p = tester.renderObject<RenderParagraph>(
        find.byType(q.QuillLayoutText),
      );
      expect(
        p.getOffsetForCaret(const TextPosition(offset: 2), Rect.zero).dx,
        closeTo(48, .05),
      );
      expect(p.getPositionForOffset(const Offset(47, 8)).offset, 2);
      expect(p.getPositionForOffset(const Offset(17, 8)).offset, 1);
      expect(p.text.toPlainText(), 'A\tB');
      expect(tester.takeException(), isNull);
    },
  );
  test(
    'long mixed-size paragraphs stay on the grid after repeated wrapping',
    () {
      final source = TextSpan(
        style: const TextStyle(fontSize: 16),
        children: [
          for (var i = 0; i < 120; i++)
            TextSpan(
              text: '${'word ' * (i % 4)}${i % 7}\t',
              style: TextStyle(fontSize: 12.0 + i % 5 * 3),
            ),
        ],
      );
      final text = source.toPlainText();
      for (final width in [97.0, 300.0, 601.0]) {
        final p = layout(source, width: width);
        for (var at = 0; at < text.length; at++) {
          if (text.codeUnitAt(at) != 9) continue;
          final box = p
              .getBoxesForSelection(
                TextSelection(baseOffset: at, extentOffset: at + 1),
              )
              .single;
          expect(box.right, lessThanOrEqualTo(width + .05));
          expect(
            box.right % 48,
            anyOf(closeTo(0, .05), closeTo(width % 48, .05)),
            reason: 'tab at $at, width $width',
          );
        }
      }
    },
  );

  test('right-to-left paragraphs measure stops from the right edge', () {
    for (final example in [
      ('א\tבגד\tה', <int, double>{1: 48, 5: 144}),
      ('\tא\t', <int, double>{0: 48, 2: 96}),
    ]) {
      final source = TextSpan(
        text: example.$1,
        style: const TextStyle(fontSize: 16),
      );
      final span = layoutTabStops(
        source,
        maxWidth: 400,
        textDirection: TextDirection.rtl,
      );
      final p = TextPainter(text: span, textDirection: TextDirection.rtl)
        ..layout(minWidth: 400, maxWidth: 400);
      addTearDown(p.dispose);
      for (final stop in example.$2.entries) {
        final box = p
            .getBoxesForSelection(
              TextSelection(baseOffset: stop.key, extentOffset: stop.key + 1),
            )
            .single;
        expect(400 - box.left, closeTo(stop.value, .05), reason: example.$1);
      }
      expect(span.toPlainText(), source.toPlainText());
      expect(
        p.getOffsetForCaret(const TextPosition(offset: 0), Rect.zero).dx,
        closeTo(400, .05),
      );
    }
  });
}
