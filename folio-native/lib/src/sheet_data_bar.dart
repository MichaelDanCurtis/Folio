import 'dart:math' as math;
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

Color? dataBarColor(dynamic hex) =>
    hex is String && RegExp(r'^#[0-9a-fA-F]{6}$').hasMatch(hex)
    ? Color(int.parse(hex.substring(1), radix: 16) | 0xff000000)
    : null;

/// A derived display layer. It never changes the cell text, formula or clipboard.
class SheetDataBar extends StatelessWidget {
  final Map? bar;
  final Widget child;
  const SheetDataBar({super.key, required this.bar, required this.child});
  @override
  Widget build(BuildContext context) => bar == null
      ? child
      : CustomPaint(
          painter: DataBarPainter(Map<String, dynamic>.from(bar!)),
          child: child,
        );
}

class DataBarPainter extends CustomPainter {
  final Map<String, dynamic> bar;
  DataBarPainter(this.bar);
  @override
  void paint(Canvas canvas, Size size) {
    if (size.width <= 6 || size.height <= 4) return;
    final start = (bar['start'] as num?)?.toDouble(),
        end = (bar['end'] as num?)?.toDouble();
    if (start == null || end == null || !start.isFinite || !end.isFinite) {
      return;
    }
    final width = size.width - 6;
    final from = 3 + width * start.clamp(0.0, 1.0),
        to = 3 + width * end.clamp(0.0, 1.0);
    final fill = dataBarColor(bar['color']) ?? const Color(0xff638ec6);
    canvas.save();
    canvas.clipRect(Offset.zero & size);
    if (from != to) {
      final rect = Rect.fromLTRB(
        math.min(from, to),
        2,
        math.max(from, to),
        size.height - 2,
      );
      final paint = Paint()..color = fill;
      if (bar['gradient'] == true) {
        paint.shader = LinearGradient(
          begin: from < to ? Alignment.centerLeft : Alignment.centerRight,
          end: from < to ? Alignment.centerRight : Alignment.centerLeft,
          colors: [fill, Color.lerp(fill, Colors.white, .8)!],
        ).createShader(rect);
      }
      canvas.drawRect(rect, paint);
      if (bar['border'] == true) {
        canvas.drawRect(
          rect.deflate(.5),
          Paint()
            ..color = dataBarColor(bar['borderColor']) ?? fill
            ..style = PaintingStyle.stroke
            ..strokeWidth = 1,
        );
      }
    }
    final axis = (bar['axis'] as num?)?.toDouble();
    if (axis != null && axis.isFinite) {
      final x = 3 + width * axis.clamp(0.0, 1.0);
      canvas.drawLine(
        Offset(x, 1),
        Offset(x, size.height - 1),
        Paint()
          ..color = dataBarColor(bar['axisColor']) ?? Colors.black
          ..strokeWidth = 1,
      );
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant DataBarPainter oldDelegate) =>
      !mapEquals(bar, oldDelegate.bar);
}

Map<String, dynamic> defaultDataBar() => {
  'extended': true,
  'color': '#638EC6',
  'negativeFillColor': '#FF0000',
  'borderColor': '#638EC6',
  'negativeBorderColor': '#FF0000',
  'axisColor': '#000000',
  'thresholds': [
    {'type': 'autoMin', 'value': ''},
    {'type': 'autoMax', 'value': ''},
  ],
  'minLength': 0,
  'maxLength': 100,
  'showValue': true,
  'gradient': true,
  'border': false,
  'negativeBarColorSameAsPositive': false,
  'negativeBarBorderColorSameAsPositive': true,
  'direction': 'context',
  'axisPosition': 'automatic',
};

class DataBarEditor extends StatelessWidget {
  final Map<String, dynamic> value;
  final ValueChanged<Map<String, dynamic>> onChanged;
  const DataBarEditor({
    super.key,
    required this.value,
    required this.onChanged,
  });
  void change(String key, dynamic v) =>
      onChanged({...value, 'extended': true, key: v});
  Widget toggle(String key, String label, {bool fallback = false}) =>
      CheckboxListTile(
        key: Key('bar-$key'),
        contentPadding: EdgeInsets.zero,
        dense: true,
        title: Text(label),
        value: value[key] as bool? ?? fallback,
        onChanged: (v) => change(key, v),
      );
  Widget choose(
    String key,
    String label,
    Map<String, String> choices,
    String fallback,
  ) => DropdownButtonFormField<String>(
    key: Key('bar-$key'),
    initialValue: value[key] as String? ?? fallback,
    decoration: InputDecoration(labelText: label),
    isExpanded: true,
    items: [
      for (final entry in choices.entries)
        DropdownMenuItem(value: entry.key, child: Text(entry.value)),
    ],
    onChanged: (v) => change(key, v),
  );
  Widget color(String key, String label, String fallback) => Padding(
    padding: const EdgeInsets.only(top: 10),
    child: Row(
      children: [
        Container(
          width: 24,
          height: 24,
          margin: const EdgeInsets.only(right: 12),
          color: dataBarColor(value[key] ?? fallback),
        ),
        Expanded(
          child: _BarColorField(
            key: Key('bar-$key'),
            value: value[key] as String? ?? fallback,
            label: label,
            onChanged: (v) => change(key, v),
          ),
        ),
        for (final hex in [
          '#638EC6',
          '#63BE7B',
          '#FF0000',
          '#FFB628',
          '#000000',
        ])
          IconButton(
            tooltip: '$label $hex',
            icon: Icon(Icons.circle, color: dataBarColor(hex), size: 18),
            onPressed: () => change(key, hex),
          ),
      ],
    ),
  );
  @override
  Widget build(BuildContext context) {
    final points = (value['thresholds'] as List)
        .map((p) => Map<String, dynamic>.from(p as Map))
        .toList();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        for (var i = 0; i < 2; i++)
          Padding(
            padding: const EdgeInsets.only(top: 12),
            child: Row(
              children: [
                Expanded(
                  child: DropdownButtonFormField<String>(
                    key: Key('bar-threshold-$i'),
                    initialValue: points[i]['type'],
                    decoration: InputDecoration(
                      labelText: i == 0 ? 'Minimum' : 'Maximum',
                    ),
                    isExpanded: true,
                    items: [
                      for (final t in [
                        i == 0 ? 'autoMin' : 'autoMax',
                        i == 0 ? 'min' : 'max',
                        'num',
                        'percent',
                        'percentile',
                        'formula',
                      ])
                        DropdownMenuItem(
                          value: t,
                          child: Text(
                            {
                              'autoMin': 'Automatic',
                              'autoMax': 'Automatic',
                              'min': 'Lowest value',
                              'max': 'Highest value',
                              'num': 'Number',
                              'percent': 'Percent',
                              'percentile': 'Percentile',
                              'formula': 'Formula',
                            }[t]!,
                          ),
                        ),
                    ],
                    onChanged: (v) {
                      points[i]['type'] = v;
                      change('thresholds', points);
                    },
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: TextFormField(
                    key: Key('bar-threshold-value-$i'),
                    initialValue: points[i]['value'] as String? ?? '',
                    enabled: ![
                      'autoMin',
                      'autoMax',
                      'min',
                      'max',
                    ].contains(points[i]['type']),
                    decoration: const InputDecoration(labelText: 'Value'),
                    onChanged: (v) {
                      points[i]['value'] = v;
                      change('thresholds', points);
                    },
                  ),
                ),
              ],
            ),
          ),
        color('color', 'Positive fill', '#638EC6'),
        toggle('showValue', 'Show cell value', fallback: true),
        toggle('gradient', 'Gradient fill', fallback: true),
        toggle('border', 'Bar border'),
        if (value['border'] == true)
          color(
            'borderColor',
            'Positive border',
            value['color'] as String? ?? '#638EC6',
          ),
        choose('direction', 'Direction', {
          'context': 'Follow worksheet',
          'leftToRight': 'Left to right',
          'rightToLeft': 'Right to left',
        }, 'context'),
        const SizedBox(height: 12),
        choose('axisPosition', 'Axis position', {
          'automatic': 'Automatic',
          'middle': 'Cell midpoint',
          'none': 'No axis',
        }, 'automatic'),
        if (value['axisPosition'] != 'none')
          color('axisColor', 'Axis color', '#000000'),
        const Divider(height: 24),
        const Text('Negative values'),
        toggle('negativeBarColorSameAsPositive', 'Use positive fill color'),
        if (value['negativeBarColorSameAsPositive'] != true)
          color('negativeFillColor', 'Negative fill', '#FF0000'),
        if (value['border'] == true) ...[
          toggle(
            'negativeBarBorderColorSameAsPositive',
            'Use positive border color',
            fallback: true,
          ),
          if (value['negativeBarBorderColorSameAsPositive'] == false)
            color('negativeBorderColor', 'Negative border', '#FF0000'),
        ],
        const SizedBox(height: 12),
        Row(
          children: [
            for (final key in ['minLength', 'maxLength'])
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.only(right: 12),
                  child: TextFormField(
                    key: Key('bar-$key'),
                    initialValue:
                        '${value[key] ?? (key == 'minLength' ? 10 : 90)}',
                    decoration: InputDecoration(
                      labelText: key == 'minLength'
                          ? 'Shortest bar (%)'
                          : 'Longest bar (%)',
                    ),
                    onChanged: (v) => change(key, int.tryParse(v) ?? v),
                  ),
                ),
              ),
          ],
        ),
      ],
    );
  }
}

class _BarColorField extends StatefulWidget {
  final String value, label;
  final ValueChanged<String> onChanged;
  const _BarColorField({
    super.key,
    required this.value,
    required this.label,
    required this.onChanged,
  });
  @override
  State<_BarColorField> createState() => _BarColorFieldState();
}

class _BarColorFieldState extends State<_BarColorField> {
  late final controller = TextEditingController(text: widget.value);
  @override
  void didUpdateWidget(covariant _BarColorField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (controller.text != widget.value) {
      controller.value = TextEditingValue(
        text: widget.value,
        selection: TextSelection.collapsed(offset: widget.value.length),
      );
    }
  }

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => TextField(
    controller: controller,
    decoration: InputDecoration(labelText: widget.label, hintText: '#RRGGBB'),
    onChanged: widget.onChanged,
  );
}
