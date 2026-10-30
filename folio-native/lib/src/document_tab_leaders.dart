import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'document_pdf.dart' show PdfRecorder;

const tabLeaderNames = {
  'none': 'None',
  'dot': 'Dots',
  'hyphen': 'Dashes',
  'underscore': 'Solid line',
  'heavy': 'Heavy line',
  'middleDot': 'Middle dots',
};

/// Decorations are painted over the original tab's shaped box. They never
/// become document characters, clipboard data, or additional semantic nodes.
@immutable
class TabLeaderDecoration {
  final String leader;
  final TextStyle style;
  final double baseline;
  const TabLeaderDecoration(this.leader, this.style, this.baseline);

  void paint(Canvas canvas, Rect bounds, TextScaler scaler) {
    if (!tabLeaderNames.containsKey(leader) ||
        leader == 'none' ||
        bounds.isEmpty) {
      return;
    }
    final size = scaler.scale(style.fontSize ?? 14);
    if (size <= 0) return;
    final color = style.color ?? const Color(0xff242424);
    final y = bounds.top + baseline;
    canvas.save();
    canvas.clipRect(bounds);
    if (leader == 'underscore' || leader == 'heavy') {
      final thickness = math.max(.5, size / (leader == 'heavy' ? 8 : 20));
      final paint = Paint()
        ..color = color
        ..strokeWidth = thickness;
      canvas.drawLine(
        Offset(bounds.left, y + size / 12),
        Offset(bounds.right, y + size / 12),
        paint,
      );
    } else {
      final glyph = switch (leader) {
        'dot' => '.',
        'hyphen' => '-',
        _ => '·',
      };
      final textStyle = style.copyWith(
        decoration: TextDecoration.none,
        backgroundColor: Colors.transparent,
        letterSpacing: 0,
        wordSpacing: 0,
      );
      final painter = TextPainter(
        text: TextSpan(text: glyph, style: textStyle),
        textDirection: TextDirection.ltr,
        textScaler: scaler,
      )..layout();
      try {
        final advance = painter.width;
        if (advance.isFinite && advance > 0) {
          final count = (bounds.width / advance).floor();
          if (count > 0) {
            // Repeat the already-shaped glyph. This keeps memory bounded even
            // for a very wide table cell without constructing giant strings.
            final top =
                y -
                painter.computeDistanceToActualBaseline(
                  TextBaseline.alphabetic,
                );
            final start = bounds.right - count * advance;
            if (canvas is PdfRecorder) {
              // A PDF keeps the leader as text, one fragment of glyphs.
              canvas.leader(
                glyph * count,
                Offset(start, y),
                count * advance,
                textStyle.copyWith(fontSize: size, color: color),
              );
              canvas.restore();
              return;
            }
            for (var i = 0; i < count; i++) {
              painter.paint(canvas, Offset(start + i * advance, top));
            }
          }
        }
      } finally {
        painter.dispose();
      }
    }
    canvas.restore();
  }

  @override
  bool operator ==(Object other) =>
      other is TabLeaderDecoration &&
      leader == other.leader &&
      style == other.style &&
      baseline == other.baseline;
  @override
  int get hashCode => Object.hash(leader, style, baseline);
}
