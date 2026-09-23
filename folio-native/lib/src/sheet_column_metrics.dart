import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'sheet_font.dart';

/// SpreadsheetML widths already include the five-pixel margin/grid allowance.
/// Measure Normal's widest digit at nominal 96dpi, independently of UI scaling.
class SheetColumnMetrics {
  final double digitWidth;
  const SheetColumnMetrics(this.digitWidth) : assert(digitWidth > 0);
  factory SheetColumnMetrics.fromFont(Map normal) {
    final painter = TextPainter(textDirection: TextDirection.ltr);
    var maximum = 0.0;
    for (var digit = 0; digit < 10; digit++) {
      painter.text = TextSpan(text: '$digit', style: sheetTextStyle(normal));
      painter.layout();
      maximum = math.max(maximum, painter.width);
    }
    painter.dispose();
    return SheetColumnMetrics(math.max(1, maximum.roundToDouble()));
  }
  double pixels(num width) => width <= 0
      ? 0
      : (((256 * width + (128 / digitWidth).floor()) / 256) * digitWidth)
            .floorToDouble();

  /// Round upward to the first 1/256-character width that fits the pixels.
  double fitPixels(double desired) {
    if (desired <= 0) return 0;
    final target = desired.ceilToDouble();
    var units = ((target * 256 / digitWidth) - (128 / digitWidth).floor())
        .ceil()
        .clamp(1, 255 * 256);
    if (pixels(units / 256) < target && units < 255 * 256) units++;
    return units / 256;
  }

  double characters(num width) => math.max(
    0,
    (((pixels(width) - 5) / digitWidth * 100 + .5).floor()) / 100,
  );
  double fromCharacters(double characters) => characters <= 0
      ? 0
      : (((characters * digitWidth + 5) / digitWidth * 256).floor() / 256)
            .clamp(0, 255);
  double get maxCharacters =>
      ((pixels(255) - 5) / digitWidth * 100).floor() / 100;
}
