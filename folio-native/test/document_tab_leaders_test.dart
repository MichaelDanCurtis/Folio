import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_tabs.dart';
import 'package:folio_native/src/document_table_view.dart';
import 'package:folio_native/src/document_tab_leaders.dart';

class _PaintCounterSpan extends TextSpan implements q.QuillInlinePaintSpan {
  final VoidCallback count;
  _PaintCounterSpan(this.count, {Color color = Colors.black})
    : super(
        text: 'A',
        style: TextStyle(fontSize: 16, color: color),
      );
  @override
  bool get hasInlinePaint => true;
  @override
  void paintInline(Canvas canvas, Rect bounds, TextScaler scaler) {
    count();
    canvas.drawLine(
      bounds.topLeft,
      bounds.bottomRight,
      Paint()..color = Colors.red,
    );
  }
}

void main() {
  testWidgets(
    'inline decorations are cached for repaints and invalidated by text, layout and scale',
    (tester) async {
      var paints = 0;
      final span = _PaintCounterSpan(() => paints++);
      Future<void> show(
        InlineSpan source,
        double width,
        TextScaler scale,
      ) async {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Center(
              child: SizedBox(
                width: width,
                child: q.QuillLayoutText(
                  key: const Key('text'),
                  text: source,
                  textScaler: scale,
                ),
              ),
            ),
          ),
        );
      }

      await show(span, 200, TextScaler.noScaling);
      expect(paints, 1);
      final render = tester.renderObject<RenderParagraph>(
        find.byKey(const Key('text')),
      );
      for (var i = 0; i < 3; i++) {
        render.markNeedsPaint();
        await tester.pump();
      }
      expect(paints, 1);
      await show(span, 240, TextScaler.noScaling);
      expect(paints, 2);
      await show(span, 240, TextScaler.linear(2));
      expect(paints, 3);
      await show(
        _PaintCounterSpan(() => paints++, color: Colors.blue),
        240,
        TextScaler.linear(2),
      );
      expect(paints, 4);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'all leaders paint only inside the tab and preserve text, semantics, caret and RTL geometry',
    (tester) async {
      // Native system font remains on the host; the fixture does not redistribute it.
      await tester.runAsync(() async {
        await (FontLoader('LeaderTest')..addFont(
              Future.value(
                ByteData.sublistView(
                  await File(
                    '/System/Library/Fonts/Supplemental/Arial.ttf',
                  ).readAsBytes(),
                ),
              ),
            ))
            .load();
      });
      final semantics = tester.ensureSemantics();
      try {
        final masks = <String, List<int>>{};
        final centroids = <String, double>{};
        List<int>? noLeader;
        for (final leader in [...tabLeaderNames.keys, 'none']) {
          const source = TextSpan(
            text: 'A\tB',
            style: TextStyle(
              fontFamily: 'LeaderTest',
              fontSize: 24,
              color: Colors.black,
            ),
          );
          final text = layoutTabStops(
            source,
            maxWidth: 220,
            stops: [DocumentTabStop(2160, leader: leader)],
          );
          await tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.ltr,
              child: Center(
                child: RepaintBoundary(
                  key: const Key('frame'),
                  child: SizedBox(
                    width: 220,
                    height: 60,
                    child: ColoredBox(
                      color: Colors.white,
                      child: q.QuillLayoutText(
                        key: const Key('text'),
                        text: text,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          );
          final render = tester.renderObject<RenderParagraph>(
            find.byKey(const Key('text')),
          );
          expect(render.text.toPlainText(), 'A\tB');
          expect(
            tester.getSemantics(find.byKey(const Key('text'))).label,
            'A\tB',
          );
          expect(
            render
                .getOffsetForCaret(const TextPosition(offset: 2), Rect.zero)
                .dx,
            closeTo(144, .05),
          );
          final box = render
              .getBoxesForSelection(
                const TextSelection(baseOffset: 1, extentOffset: 2),
              )
              .single
              .toRect();
          expect(
            render
                .getPositionForOffset(Offset(box.right - 1, box.center.dy))
                .offset,
            2,
          );
          late ByteData pixels;
          await tester.runAsync(() async {
            final image = await tester
                .renderObject<RenderRepaintBoundary>(
                  find.byKey(const Key('frame')),
                )
                .toImage();
            pixels = (await image.toByteData(
              format: ui.ImageByteFormat.rawRgba,
            ))!;
            image.dispose();
          });
          final all = pixels.buffer.asUint8List().toList();
          if (leader == 'none') noLeader ??= all;
          final ink = <int>[];
          var ySum = 0.0;
          for (var y = 0; y < 60; y++) {
            for (var x = 0; x < 220; x++) {
              final at = (y * 220 + x) * 4;
              final inTab = box.contains(Offset(x + .5, y + .5));
              if (!inTab) {
                expect(
                  all.sublist(at, at + 4),
                  noLeader!.sublist(at, at + 4),
                  reason: '$leader painted outside its tab',
                );
              } else if (all[at] < 200) {
                ink.add(y * 220 + x);
                ySum += y;
              }
            }
          }
          masks[leader] = ink;
          centroids[leader] = ink.isEmpty ? 0 : ySum / ink.length;
          expect(ink.isEmpty, leader == 'none', reason: leader);
        }
        expect(
          masks['heavy']!.length,
          greaterThan(masks['underscore']!.length * 1.5),
        );
        expect(centroids['middleDot'], lessThan(centroids['dot']!));
        expect(
          masks.values.map((m) => m.join(',')).toSet().length,
          tabLeaderNames.length,
        );

        // No leader remains when changing just the leader property back to None.
        const source = TextSpan(
          text: '\tאב',
          style: TextStyle(
            fontFamily: 'LeaderTest',
            fontSize: 24,
            color: Colors.black,
          ),
        );
        for (final leader in ['dot', 'none']) {
          final text = layoutTabStops(
            source,
            maxWidth: 220,
            textDirection: TextDirection.rtl,
            stops: [DocumentTabStop(2160, leader: leader)],
          );
          await tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.rtl,
              child: Center(
                child: RepaintBoundary(
                  key: const Key('rtl-frame'),
                  child: SizedBox(
                    width: 220,
                    height: 60,
                    child: ColoredBox(
                      color: Colors.white,
                      child: q.QuillLayoutText(
                        key: const Key('rtl'),
                        text: text,
                        textDirection: TextDirection.rtl,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          );
          final render = tester.renderObject<RenderParagraph>(
            find.byKey(const Key('rtl')),
          );
          final box = render
              .getBoxesForSelection(
                const TextSelection(baseOffset: 0, extentOffset: 1),
              )
              .single;
          expect(box.right - box.left, closeTo(144, .05));
          expect(render.text.toPlainText(), '\tאב');
          var ink = 0;
          await tester.runAsync(() async {
            final image = await tester
                .renderObject<RenderRepaintBoundary>(
                  find.byKey(const Key('rtl-frame')),
                )
                .toImage();
            final pixels = (await image.toByteData(
              format: ui.ImageByteFormat.rawRgba,
            ))!.buffer.asUint8List();
            image.dispose();
            for (var y = box.top.ceil(); y < box.bottom.floor(); y++) {
              for (
                var x = box.left.ceil() + 1;
                x < box.right.floor() - 1;
                x++
              ) {
                if (pixels[(y * 220 + x) * 4] < 200) ink++;
              }
            }
          });
          expect(ink > 0, leader == 'dot');
          expect(tester.takeException(), isNull);
        }
        if (Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1') {
          await tester.pumpWidget(
            Directionality(
              textDirection: TextDirection.ltr,
              child: Center(
                child: RepaintBoundary(
                  key: const Key('leader-gallery'),
                  child: Container(
                    width: 640,
                    padding: const EdgeInsets.all(24),
                    color: Colors.white,
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        const Text(
                          'Tab leaders',
                          style: TextStyle(
                            fontFamily: 'LeaderTest',
                            fontSize: 28,
                            fontWeight: FontWeight.bold,
                            color: Color(0xff185abd),
                          ),
                        ),
                        const SizedBox(height: 16),
                        for (final leader in tabLeaderNames.entries) ...[
                          Text(
                            leader.value,
                            style: const TextStyle(
                              fontFamily: 'LeaderTest',
                              fontSize: 13,
                              color: Color(0xff616161),
                            ),
                          ),
                          q.QuillLayoutText(
                            text: layoutTabStops(
                              const TextSpan(
                                text: 'Project revenue\t12,345.00',
                                style: TextStyle(
                                  fontFamily: 'LeaderTest',
                                  fontSize: 20,
                                  color: Color(0xff242424),
                                ),
                              ),
                              maxWidth: 592,
                              stops: [
                                DocumentTabStop(
                                  6750,
                                  alignment: 'right',
                                  leader: leader.key,
                                ),
                              ],
                            ),
                          ),
                          const SizedBox(height: 14),
                        ],
                        const Text(
                          'Each row retains one tab character.',
                          style: TextStyle(
                            fontFamily: 'LeaderTest',
                            fontSize: 12,
                            color: Color(0xff616161),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          );
          await tester.runAsync(() async {
            final image = await tester
                .renderObject<RenderRepaintBoundary>(
                  find.byKey(const Key('leader-gallery')),
                )
                .toImage();
            final bytes = await image.toByteData(
              format: ui.ImageByteFormat.png,
            );
            image.dispose();
            await File(
              '${Directory.current.path}/release/previews/tab-leaders.png',
            ).writeAsBytes(bytes!.buffer.asUint8List());
          });
          expect(tester.takeException(), isNull);
        }
      } finally {
        semantics.dispose();
      }
    },
    skip: !Platform.isMacOS,
  );
  testWidgets('static table cells paint leaders with the tab text color', (
    tester,
  ) async {
    final node = {
      'type': 'table',
      'content': [
        {
          'type': 'tableRow',
          'content': [
            {
              'type': 'tableCell',
              'content': [
                {
                  'type': 'paragraph',
                  'attrs': {
                    'tabs': {
                      'inherited': [],
                      'direct': [
                        const DocumentTabStop(1440, leader: 'heavy').toJson(),
                      ],
                    },
                  },
                  'content': [
                    {
                      'type': 'text',
                      'text': 'A\tB',
                      'marks': [
                        {
                          'type': 'textStyle',
                          'attrs': {'color': '#1166CC'},
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
      ],
    };
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RepaintBoundary(
            key: const Key('table-frame'),
            child: SizedBox(width: 200, child: DocumentTableView(node: node)),
          ),
        ),
      ),
    );
    final render = tester
        .renderObjectList<RenderParagraph>(
          find.byWidgetPredicate((w) => w is RichText),
        )
        .firstWhere((r) => r.text.toPlainText() == 'A\tB');
    final tab = render.text.getSpanForPosition(const TextPosition(offset: 1));
    expect(tab, isA<q.QuillInlinePaintSpan>());
    expect((tab as q.QuillInlinePaintSpan).hasInlinePaint, isTrue);
    final frame = tester.renderObject<RenderRepaintBoundary>(
      find.byKey(const Key('table-frame')),
    );
    final origin =
        render.localToGlobal(Offset.zero) - frame.localToGlobal(Offset.zero);
    final bounds = render
        .getBoxesForSelection(
          const TextSelection(baseOffset: 1, extentOffset: 2),
        )
        .single
        .toRect()
        .shift(origin);
    var blue = 0;
    await tester.runAsync(() async {
      final image = await frame.toImage();
      final width = image.width;
      final pixels = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!.buffer.asUint8List();
      image.dispose();
      for (var y = bounds.top.ceil(); y < bounds.bottom.floor(); y++) {
        for (
          var x = bounds.left.ceil() + 1;
          x < bounds.right.floor() - 1;
          x++
        ) {
          final at = (y * width + x) * 4;
          if (pixels[at + 2] > pixels[at] + 20) blue++;
        }
      }
    });
    expect(blue, greaterThan(30));
    expect(tester.takeException(), isNull);
  });
}
