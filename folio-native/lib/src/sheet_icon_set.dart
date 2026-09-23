import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'sheet_font.dart';

const sheetIconSets = {
  '3Arrows': '3 arrows (colored)',
  '3ArrowsGray': '3 arrows (gray)',
  '3Flags': '3 flags',
  '3TrafficLights1': '3 traffic lights',
  '3TrafficLights2': '3 traffic lights (rimmed)',
  '3Signs': '3 signs',
  '3Symbols': '3 symbols (circled)',
  '3Symbols2': '3 symbols',
  '4Arrows': '4 arrows (colored)',
  '4ArrowsGray': '4 arrows (gray)',
  '4RedToBlack': '4 circles (red to black)',
  '4Rating': '4 ratings',
  '4TrafficLights': '4 traffic lights',
  '5Arrows': '5 arrows (colored)',
  '5ArrowsGray': '5 arrows (gray)',
  '5Rating': '5 ratings',
  '5Quarters': '5 quarters',
  '3Stars': '3 stars',
  '3Triangles': '3 triangles',
  '5Boxes': '5 boxes',
};
int sheetIconCount(String set) => int.parse(set[0]);
String sheetIconLabel(String set, int index) => set == 'NoIcons'
    ? 'No cell icon'
    : '${sheetIconSets[set]}, icon ${index + 1} of ${sheetIconCount(set)}';
Map<String, dynamic> defaultIconSet([String set = '3TrafficLights1']) => {
  'set': set,
  'reverse': false,
  'showValue': true,
  'thresholds': [
    for (var i = 0; i < sheetIconCount(set); i++)
      {
        'type': 'percent',
        'value': '${(i * 100 / sheetIconCount(set)).round()}',
        'gte': true,
      },
  ],
};
bool sheetValueHidden(Map style) =>
    (style['_iconSet'] as Map?)?['showValue'] == false ||
    (style['_dataBar'] as Map?)?['showValue'] == false;
double sheetIconSize(Map style, TextScaler scaler) =>
    scaler.scale(sheetTextStyle(style).fontSize ?? 14);
double sheetIconSpace(Map style, TextScaler scaler) {
  final icon = style['_iconSet'] as Map?;
  return icon == null || icon['set'] == 'NoIcons'
      ? 0
      : sheetIconSize(style, scaler) + 4;
}

/// Shares the cell's available width with its number layout. Narrow cells clip
/// the icon; the complete underlying value remains available to accessibility.
class SheetIconValue extends StatelessWidget {
  final Map style;
  final Widget child;
  final String address;
  const SheetIconValue({
    super.key,
    required this.style,
    required this.child,
    required this.address,
  });
  @override
  Widget build(BuildContext context) {
    final text = Opacity(
      opacity: sheetValueHidden(style) ? 0 : 1,
      alwaysIncludeSemantics: true,
      child: child,
    );
    final icon = style['_iconSet'] as Map?;
    if (icon == null || icon['set'] == 'NoIcons') return text;
    final size = sheetIconSize(style, MediaQuery.textScalerOf(context));
    return LayoutBuilder(
      builder: (context, constraints) {
        final space = math.min(size + 4, constraints.maxWidth);
        return Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SizedBox(
              width: space,
              child: ClipRect(
                child: Center(
                  child: SheetIconGlyph(
                    key: Key('conditional-icon-$address'),
                    set: icon['set'] as String,
                    index: (icon['index'] as num).toInt(),
                    size: math.min(size, space),
                  ),
                ),
              ),
            ),
            Expanded(child: text),
          ],
        );
      },
    );
  }
}

class SheetIconGlyph extends StatelessWidget {
  final String set;
  final int index;
  final double size;
  const SheetIconGlyph({
    super.key,
    required this.set,
    required this.index,
    this.size = 20,
  });
  @override
  Widget build(BuildContext context) => Semantics(
    label: sheetIconLabel(set, index),
    image: true,
    child: SizedBox.square(
      dimension: size,
      child: CustomPaint(painter: SheetIconPainter(set, index)),
    ),
  );
}

/// Resolution-independent glyphs. Icon indices follow the OOXML low-to-high
/// category order, including the distinct four-rating and five-rating sets.
class SheetIconPainter extends CustomPainter {
  final String set;
  final int index;
  const SheetIconPainter(this.set, this.index);
  static const red = Color(0xffdc5353),
      yellow = Color(0xffefbe36),
      green = Color(0xff49a562);
  @override
  void paint(Canvas canvas, Size size) {
    if (set == 'NoIcons' || size.isEmpty) return;
    final count = sheetIconCount(set);
    final i = index.clamp(0, count - 1);
    final color = set.contains('Gray')
        ? const Color(0xff8b9195)
        : i == 0
        ? red
        : i == count - 1
        ? green
        : yellow;
    canvas.save();
    canvas.scale(size.width / 20, size.height / 20);
    final fill = Paint()..color = color;
    final outline = Paint()
      ..color = const Color(0xff465157)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.1
      ..strokeJoin = StrokeJoin.round;
    void shape(Path path, Color color) {
      canvas.drawPath(path, fill..color = color);
      canvas.drawPath(path, outline);
    }

    void circle(Color color, [double radius = 7.7]) {
      canvas.drawCircle(const Offset(10, 10), radius, fill..color = color);
      canvas.drawCircle(const Offset(10, 10), radius, outline);
    }

    Path polygon(List<Offset> points) => Path()..addPolygon(points, true);
    if (set.contains('Arrows')) {
      // The base arrow points right; intermediate four-icon arrows are diagonal.
      final angle = count == 3
          ? [math.pi / 2, 0.0, -math.pi / 2][i]
          : count == 4
          ? [math.pi / 2, math.pi / 4, -math.pi / 4, -math.pi / 2][i]
          : [math.pi / 2, math.pi / 4, 0.0, -math.pi / 4, -math.pi / 2][i];
      canvas.translate(10, 10);
      canvas.rotate(angle);
      canvas.translate(-10, -10);
      shape(
        polygon(const [
          Offset(2, 7),
          Offset(10, 7),
          Offset(10, 2),
          Offset(18, 10),
          Offset(10, 18),
          Offset(10, 13),
          Offset(2, 13),
        ]),
        color,
      );
    } else if (set == '3Flags') {
      shape(
        Path()
          ..moveTo(4, 3)
          ..cubicTo(8, 0, 11, 7, 16, 4)
          ..lineTo(16, 11)
          ..cubicTo(11, 14, 8, 7, 4, 10)
          ..close(),
        color,
      );
      canvas.drawLine(
        const Offset(4, 2),
        const Offset(4, 18),
        outline..strokeWidth = 1.8,
      );
    } else if (set.contains('TrafficLights') || set == '4RedToBlack') {
      var light = color;
      if (set == '4TrafficLights') {
        light = [const Color(0xff353535), red, yellow, green][i];
      }
      if (set == '4RedToBlack') {
        light = [
          const Color(0xff171717),
          const Color(0xffa6a6a6),
          const Color(0xffed9292),
          const Color(0xffdf1616),
        ][i];
      }
      if (set == '3TrafficLights2') {
        canvas.drawRRect(
          RRect.fromRectAndRadius(
            const Rect.fromLTWH(1, 0, 18, 20),
            const Radius.circular(3),
          ),
          fill..color = const Color(0xff252b2b),
        );
      }
      circle(light);
    } else if (set == '3Signs') {
      if (i == 2) {
        circle(green);
      } else if (i == 1) {
        shape(
          polygon(const [Offset(10, 2), Offset(19, 17), Offset(1, 17)]),
          yellow,
        );
      } else {
        shape(
          polygon(const [
            Offset(10, 1),
            Offset(19, 10),
            Offset(10, 19),
            Offset(1, 10),
          ]),
          red,
        );
      }
    } else if (set.startsWith('3Symbols')) {
      final circled = set == '3Symbols';
      if (circled) circle(color);
      final pen = Paint()
        ..color = circled ? Colors.white : color
        ..style = PaintingStyle.stroke
        ..strokeWidth = circled ? 2 : 3
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round;
      if (i == 2) {
        canvas.drawPath(
          Path()
            ..moveTo(5, 10)
            ..lineTo(8, 13)
            ..lineTo(15, 6),
          pen,
        );
      } else if (i == 1) {
        canvas.drawLine(const Offset(10, 5), const Offset(10, 11), pen);
        canvas.drawCircle(
          const Offset(10, 15),
          1.2,
          Paint()..color = pen.color,
        );
      } else {
        canvas.drawLine(const Offset(6, 6), const Offset(14, 14), pen);
        canvas.drawLine(const Offset(6, 14), const Offset(14, 6), pen);
      }
    } else if (set.endsWith('Rating')) {
      final filled = count == 4 ? i + 1 : i;
      for (var b = 0; b < 4; b++) {
        final rect = Rect.fromLTWH(1 + b * 4.7, 15 - b * 3.6, 3.1, 4 + b * 3.6);
        canvas.drawRect(
          rect,
          fill
            ..color = b < filled
                ? const Color(0xff2a6389)
                : const Color(0xffe0e3e5),
        );
        canvas.drawRect(rect, outline..strokeWidth = .55);
      }
    } else if (set == '5Quarters') {
      circle(Colors.white);
      if (i > 0) {
        canvas.drawArc(
          const Rect.fromLTWH(2.3, 2.3, 15.4, 15.4),
          -math.pi / 2,
          math.pi * i / 2,
          true,
          fill..color = const Color(0xff252525),
        );
      }
    } else if (set == '3Stars') {
      final star = polygon([
        for (var j = 0; j < 10; j++)
          Offset(
            10 + (j.isEven ? 9 : 4) * math.cos(-math.pi / 2 + j * math.pi / 5),
            10 + (j.isEven ? 9 : 4) * math.sin(-math.pi / 2 + j * math.pi / 5),
          ),
      ]);
      canvas.drawPath(star, fill..color = const Color(0xffe5e7e8));
      canvas.save();
      canvas.clipPath(star);
      canvas.drawRect(Rect.fromLTWH(0, 0, i * 10, 20), fill..color = yellow);
      canvas.restore();
      canvas.drawPath(star, outline);
    } else if (set == '3Triangles') {
      if (i == 1) {
        shape(
          polygon(const [
            Offset(2, 8),
            Offset(18, 8),
            Offset(18, 12),
            Offset(2, 12),
          ]),
          yellow,
        );
      } else if (i == 2) {
        shape(
          polygon(const [Offset(10, 3), Offset(18, 15), Offset(2, 15)]),
          green,
        );
      } else {
        shape(
          polygon(const [Offset(2, 5), Offset(18, 5), Offset(10, 17)]),
          red,
        );
      }
    } else if (set == '5Boxes') {
      const order = [
        Offset(2, 11),
        Offset(11, 11),
        Offset(2, 2),
        Offset(11, 2),
      ];
      for (var b = 0; b < 4; b++) {
        final rect = RRect.fromRectAndRadius(
          order[b] & const Size(7, 7),
          const Radius.circular(.8),
        );
        canvas.drawRRect(
          rect,
          fill
            ..color = b < i ? const Color(0xff497ca8) : const Color(0xfff0f2f3),
        );
        canvas.drawRRect(
          rect,
          outline
            ..color = b < i ? const Color(0xff284a6f) : const Color(0xffc8cdd0),
        );
      }
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(SheetIconPainter oldDelegate) =>
      set != oldDelegate.set || index != oldDelegate.index;
}
