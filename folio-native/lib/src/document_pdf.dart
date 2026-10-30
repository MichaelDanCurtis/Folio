// PDF export (D05.04, D05.05). The page layout paints each page onto a
// recording canvas, which keeps the text as positioned fragments rather
// than pixels; the engine's `exportPdf` op writes them with embedded fonts,
// pictures, links, an outline and tags.
import 'dart:convert';
import 'dart:math' as math;
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'core.dart';
import 'document_page_layout.dart';

String _hex(Color c) =>
    '#${(c.toARGB32() & 0xffffff).toRadixString(16).padLeft(6, '0')}';

/// A span that carries a link target, so the PDF can make it clickable.
class LinkTextSpan extends TextSpan {
  final String href;
  const LinkTextSpan({
    required this.href,
    super.text,
    super.style,
    super.children,
  });
}

/// Records a page's drawing as PDF operations, in page pixels.
class PdfRecorder implements Canvas {
  final List<Map<String, dynamic>> ops = [];
  final Set<String> pictures = {};
  Offset _origin = Offset.zero;
  final _stack = <Offset>[];

  List<double> _r(Rect r) {
    final s = r.shift(_origin);
    return [s.left, s.top, s.width, s.height];
  }

  @override
  void save() {
    _stack.add(_origin);
    ops.add({'op': 'save'});
  }

  @override
  void restore() {
    if (_stack.isNotEmpty) _origin = _stack.removeLast();
    ops.add({'op': 'restore'});
  }

  @override
  void translate(double dx, double dy) => _origin += Offset(dx, dy);

  @override
  void clipRect(
    Rect rect, {
    ui.ClipOp clipOp = ui.ClipOp.intersect,
    bool doAntiAlias = true,
  }) => ops.add({'op': 'clip', 'rect': _r(rect)});

  @override
  void drawRect(Rect rect, Paint paint) {
    final stroke = paint.style == PaintingStyle.stroke;
    ops.add({
      'op': 'rect',
      'rect': _r(rect),
      if (stroke) 'stroke': _hex(paint.color) else 'fill': _hex(paint.color),
      'width': paint.strokeWidth,
    });
  }

  @override
  void drawRRect(RRect rrect, Paint paint) => drawRect(rrect.outerRect, paint);

  @override
  void drawLine(Offset p1, Offset p2, Paint paint) => ops.add({
    'op': 'line',
    'from': [p1.dx + _origin.dx, p1.dy + _origin.dy],
    'to': [p2.dx + _origin.dx, p2.dy + _origin.dy],
    'color': _hex(paint.color),
    'width': paint.strokeWidth == 0 ? 1.0 : paint.strokeWidth,
  });

  /// A picture from the document, by part name.
  void picture(String src, Rect box) {
    pictures.add(src);
    ops.add({'op': 'image', 'image': src, 'rect': _r(box)});
  }

  void begin(String tag, {String? alt}) =>
      ops.add({'op': 'begin', 'tag': tag, 'alt': ?alt});

  void end() => ops.add({'op': 'end'});

  /// Repeated tab-leader glyphs as one text fragment on [baseline].
  void leader(String glyphs, Offset baseline, double width, TextStyle style) {
    final at = baseline + _origin;
    ops.add({
      'op': 'text',
      'x': at.dx,
      'y': at.dy,
      'width': width,
      'text': glyphs,
      'size': style.fontSize ?? 14,
      'family': style.fontFamily ?? '',
      'bold': (style.fontWeight?.value ?? 400) >= 600,
      'italic': style.fontStyle == FontStyle.italic,
      'color': _hex(style.color ?? const Color(0xff000000)),
    });
  }

  /// A laid-out paragraph's text: one fragment per style, line and tab stop,
  /// with the position and width the layout gave it.
  void text(TextPainter painter, Offset offset) {
    final plain = painter.plainText;
    final runs = <({int start, int end, TextStyle style, String? link})>[];
    var at = 0;
    void walk(InlineSpan span, TextStyle inherited, String? link) {
      final style = inherited.merge(span.style);
      final href = span is LinkTextSpan ? span.href : link;
      if (span is TextSpan) {
        final length = span.text?.length ?? 0;
        if (length > 0) {
          runs.add((start: at, end: at + length, style: style, link: href));
        }
        at += length;
        for (final child in span.children ?? const <InlineSpan>[]) {
          walk(child, style, href);
        }
      } else {
        at += 1;
      }
    }

    walk(painter.text!, const TextStyle(), null);
    final lines = painter.computeLineMetrics();
    final base = offset + _origin;
    for (final run in runs) {
      var start = run.start;
      while (start < run.end) {
        final line = painter.getLineBoundary(TextPosition(offset: start));
        var end = run.end;
        if (line.end > start && line.end < end) end = line.end;
        // A tab or line break starts a new fragment.
        for (var i = start; i < end; i++) {
          if (plain[i] == '\t' || plain[i] == '\n') {
            end = i == start ? i + 1 : i;
            break;
          }
        }
        final piece = plain.substring(start, end);
        if (piece.trim().isNotEmpty) {
          final boxes = painter.getBoxesForSelection(
            TextSelection(baseOffset: start, extentOffset: end),
          );
          if (boxes.isNotEmpty) {
            final left = boxes.map((b) => b.left).reduce(math.min);
            final right = boxes.map((b) => b.right).reduce(math.max);
            final box = boxes.first;
            final center = (box.top + box.bottom) / 2;
            final metrics = lines.firstWhere(
              (l) =>
                  center >= l.baseline - l.ascent - 1 &&
                  center <= l.baseline + l.descent + 1,
              orElse: () => lines.isEmpty
                  ? ui.LineMetrics(
                      hardBreak: true,
                      ascent: box.bottom - box.top,
                      descent: 0,
                      unscaledAscent: 0,
                      height: 0,
                      width: 0,
                      left: 0,
                      baseline: box.bottom,
                      lineNumber: 0,
                    )
                  : lines.first,
            );
            final style = run.style;
            final size = style.fontSize ?? 14;
            final color = style.color ?? const Color(0xff000000);
            final x = base.dx + left, y = base.dy + metrics.baseline;
            final width = right - left;
            if (style.backgroundColor != null) {
              ops.add({
                'op': 'rect',
                'rect': [x, base.dy + box.top, width, box.bottom - box.top],
                'fill': _hex(style.backgroundColor!),
              });
            }
            ops.add({
              'op': 'text',
              'x': x,
              'y': y,
              'width': width,
              'text': piece,
              'size': size,
              'family': style.fontFamily ?? '',
              'bold': (style.fontWeight?.value ?? 400) >= 600,
              'italic': style.fontStyle == FontStyle.italic,
              'color': _hex(color),
              'link': ?run.link,
            });
            final decoration = style.decoration;
            void rule(double dy) => ops.add({
              'op': 'line',
              'from': [x, y + dy],
              'to': [x + width, y + dy],
              'color': _hex(style.decorationColor ?? color),
              'width': math.max(0.6, size / 16),
            });
            if (decoration != null) {
              if (decoration.contains(TextDecoration.underline)) {
                rule(size * 0.12);
              }
              if (decoration.contains(TextDecoration.lineThrough)) {
                rule(-size * 0.3);
              }
            }
          }
        }
        start = end;
      }
    }
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => null;
}

/// The pages of a laid-out document as the `exportPdf` op's request.
Future<Map<String, dynamic>> documentPdfRequest(
  Map snapshot,
  DocumentPageLayout layout, {
  required String title,
}) async {
  final blocks = snapshot['document']?['content'] as List? ?? const [];
  final pages = <Map<String, dynamic>>[];
  final outline = <Map<String, dynamic>>[];
  final pictures = <String>{};
  final outlined = <int>{};
  for (final (index, page) in layout.pages.indexed) {
    final recorder = PdfRecorder();
    for (final placement in page.content) {
      final i = placement.bodyIndex;
      final block = i != null && i < blocks.length ? blocks[i] as Map : null;
      String tag = 'P';
      String? alt;
      if (placement.area != 'body') {
        tag = 'Artifact';
      } else if (block?['type'] == 'heading') {
        final level = ((block!['attrs']?['level'] as num?)?.toInt() ?? 1).clamp(
          1,
          6,
        );
        tag = 'H$level';
        if (outlined.add(i!)) {
          outline.add({
            'title': placement.text.split('\n').first.trim(),
            'level': level,
            'page': index,
            'y': placement.offset.dy,
          });
        }
      } else if (block?['type'] == 'paragraph') {
        final content = block!['content'] as List? ?? const [];
        if (content.length == 1 && content.first['type'] == 'image') {
          tag = 'Figure';
          final text = '${content.first['attrs']?['alt'] ?? ''}'.trim();
          alt = text.isEmpty ? 'Picture' : text;
        }
      }
      recorder.begin(tag, alt: alt);
      placement.paint(recorder);
      recorder.end();
    }
    for (final band in page.columnBands) {
      recorder.begin('Artifact');
      band.paint(recorder);
      recorder.end();
    }
    pictures.addAll(recorder.pictures);
    pages.add({
      'width': page.size.width,
      'height': page.size.height,
      'ops': recorder.ops,
    });
  }
  final images = <String, Map<String, dynamic>>{};
  for (final src in pictures) {
    final image = previewPictures[src];
    if (image == null) continue;
    final bytes = await image.toByteData(format: ui.ImageByteFormat.rawRgba);
    if (bytes == null) continue;
    images[src] = {
      'width': image.width,
      'height': image.height,
      'rgba': base64Encode(Uint8List.sublistView(bytes)),
    };
  }
  return {
    'title': title,
    'lang': 'en',
    'pages': pages,
    'images': images,
    'outline': outline,
  };
}

/// Lays the document out as the preview does and writes it as a PDF.
Future<void> exportDocumentPdf(
  WorkspaceModel model,
  String path, {
  Locale? locale,
  Future<void> Function(Map snapshot)? preload,
}) async {
  await model.flush();
  final snapshot = await model.core.call({'op': 'documentPagePreview'});
  await preload?.call(snapshot);
  final layout = DocumentPageLayout.build(snapshot, locale: locale);
  try {
    final request = await documentPdfRequest(
      snapshot,
      layout,
      title: '${model.workspace['title'] ?? 'Document'}',
    );
    await model.core.call({'op': 'exportPdf', 'path': path, ...request});
  } finally {
    layout.dispose();
  }
}
