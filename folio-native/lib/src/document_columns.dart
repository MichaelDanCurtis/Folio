import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'draft_text_field.dart';

Map<String, dynamic> columnSettings(Map section) {
  final source = section['columnLayout'] as Map? ?? {};
  return {
    'count': source['count'] ?? section['columns'] ?? 1,
    'equalWidth': source['equalWidth'] ?? true,
    'space': source['space'] ?? 720,
    'separator': source['separator'] ?? false,
    'rtl': source['rtl'] ?? false,
    'columns': [
      for (final col in source['columns'] as List? ?? [])
        Map<String, dynamic>.from(col),
    ],
  };
}

/// Physical column positions are left-to-right; reading order is separate.
class DocumentColumnGeometry {
  final List<({double left, double width})> columns;
  final bool rtl, separator;
  final String? notice;
  DocumentColumnGeometry(this.columns, this.rtl, this.separator, this.notice);
  factory DocumentColumnGeometry.resolve(Map settings, double width) {
    final requested = (settings['count'] as num? ?? 1).toInt();
    final count = requested.clamp(1, 45);
    final result = <({double left, double width})>[];
    String? notice;
    if (requested != count) notice = 'The imported column count needs repair.';
    if (settings['equalWidth'] != false) {
      final requestedSpace = (settings['space'] as num? ?? 720) / 15;
      final gap = requestedSpace.clamp(
        0.0,
        math.max(0.0, (width - count) / math.max(1, count - 1)),
      );
      if (gap != requestedSpace) {
        notice = 'Column spacing was reduced to fit the page.';
      }
      final w = math.max(1.0, (width - gap * (count - 1)) / count);
      for (var i = 0; i < count; i++) {
        result.add((left: i * (w + gap), width: w));
      }
    } else {
      final definitions = settings['columns'] as List? ?? [];
      if (definitions.length != count ||
          definitions.any(
            (c) =>
                (c['width'] as num? ?? 0) <= 0 || (c['space'] as num? ?? 0) < 0,
          )) {
        return DocumentColumnGeometry(
          [(left: 0, width: width)],
          settings['rtl'] == true,
          false,
          'The imported column widths need repair.',
        );
      }
      var x = 0.0;
      for (var i = 0; i < count; i++) {
        final w = (definitions[i]['width'] as num).toDouble() / 15;
        result.add((left: x, width: w));
        x +=
            w +
            (i + 1 < count ? (definitions[i]['space'] as num? ?? 0) / 15 : 0);
      }
      if (x > width + .01) {
        notice = 'Some imported columns extend beyond the page text area.';
      }
    }
    return DocumentColumnGeometry(
      result,
      settings['rtl'] == true,
      settings['separator'] == true,
      notice,
    );
  }
  int physicalIndex(int readingIndex) =>
      rtl ? columns.length - 1 - readingIndex : readingIndex;
}

class DocumentColumnsDialog extends StatefulWidget {
  final Map data;
  final int initialSection;
  const DocumentColumnsDialog({
    super.key,
    required this.data,
    this.initialSection = 0,
  });
  @override
  State<DocumentColumnsDialog> createState() => _DocumentColumnsDialogState();
}

class _DocumentColumnsDialogState extends State<DocumentColumnsDialog> {
  late int section = widget.initialSection.clamp(0, sections.length - 1);
  late Map<String, dynamic> settings = columnSettings(sections[section]);
  final fields = <String, TextEditingController>{};
  final invalid = <String>{};
  String unit = 'in', scope = 'section';
  bool changed = false;
  List<Map> get sections => (widget.data['sections'] as List).cast<Map>();
  int get count => (settings['count'] as num).toInt();
  List get definitions => settings['columns'] as List;
  double get scale => {'in': 1440.0, 'cm': 1440 / 2.54, 'pt': 20.0}[unit]!;
  double available(Map row) {
    final g = row['geometry'] as Map;
    return ((g['width'] as num) -
            (g['left'] as num) -
            (g['right'] as num) -
            (widget.data['gutterTop'] == true ? 0 : g['gutter'] as num))
        .toDouble();
  }

  String format(num n) =>
      (n / scale).toStringAsFixed(3).replaceFirst(RegExp(r'\.?0+$'), '');
  void resetFields() {
    for (final f in fields.values) {
      f.dispose();
    }
    fields.clear();
    invalid.clear();
  }

  @override
  void dispose() {
    resetFields();
    super.dispose();
  }

  void preset(String name) {
    final n = switch (name) {
      'One' => 1,
      'Three' => 3,
      _ => 2,
    };
    final gap = math
        .min(
          720.0,
          math.max(
            0.0,
            (available(sections[section]) - n) / math.max(1, n - 1),
          ),
        )
        .floor();
    final text = available(sections[section]).round() - gap * (n - 1);
    final custom = name == 'Left' || name == 'Right';
    final narrow = (text / 3).round();
    settings = {
      ...settings,
      'count': n,
      'space': gap,
      'equalWidth': !custom,
      'columns': custom
          ? [
              {'width': name == 'Left' ? narrow : text - narrow, 'space': gap},
              {'width': name == 'Left' ? text - narrow : narrow, 'space': 0},
            ]
          : [],
    };
    changed = true;
    resetFields();
  }

  void customWidths() {
    final metrics = DocumentColumnGeometry.resolve({
      ...settings,
      'equalWidth': true,
    }, available(sections[section]) / 15);
    settings['columns'] = [
      for (var i = 0; i < count; i++)
        {
          'width': (metrics.columns[i].width * 15).floor(),
          'space': i + 1 < count ? settings['space'] : 0,
        },
    ];
  }

  String? get validation {
    if (invalid.isNotEmpty) return 'Enter valid measurements.';
    if (count < 1 || count > 45) return 'Choose between 1 and 45 columns.';
    final gap = settings['space'] as num;
    if (gap < 0 || gap > 31680) {
      return 'Spacing must be between 0 and 22 inches.';
    }
    final equal = settings['equalWidth'] == true;
    if (!equal &&
        (definitions.length != count ||
            definitions.any(
              (c) =>
                  (c['width'] as num) <= 0 ||
                  (c['width'] as num) > 31680 ||
                  (c['space'] as num) < 0 ||
                  (c['space'] as num) > 31680,
            ))) {
      return 'Enter a positive width and nonnegative spacing for each column.';
    }
    final needed = equal
        ? gap * (count - 1) + count
        : definitions.asMap().entries.fold<num>(
            0,
            (sum, e) =>
                sum +
                (e.value['width'] as num) +
                (e.key + 1 < count ? e.value['space'] as num : 0),
          );
    final targets = scope == 'document' ? sections : [sections[section]];
    if (targets.any((row) => needed > available(row))) {
      return 'Column widths and spacing must fit between the page margins in every selected section.';
    }
    return null;
  }

  Widget measurement(
    String key,
    String label,
    num value,
    void Function(int) update, {
    bool whole = false,
  }) {
    final controller = fields.putIfAbsent(
      key,
      () => TextEditingController(text: whole ? '$value' : format(value)),
    );
    return SizedBox(
      width: 160,
      child: draftTextField(
        key: Key('columns-$key'),
        controller: controller,
        decoration: InputDecoration(
          labelText: label,
          suffixText: whole ? null : unit,
          errorText: invalid.contains(key) ? 'Enter a number' : null,
        ),
        keyboardType: const TextInputType.numberWithOptions(decimal: true),
        onChanged: (text) => setState(() {
          final n = double.tryParse(text);
          if (n == null ||
              !n.isFinite ||
              n.abs() > 100000 ||
              (whole && n != n.roundToDouble())) {
            invalid.add(key);
            return;
          }
          invalid.remove(key);
          changed = true;
          update((n * (whole ? 1 : scale)).round());
        }),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final row = sections[section];
    final blocked =
        widget.data['protected'] == true ||
        (scope == 'document'
            ? sections.any((s) => s['hasTrackedSettings'] == true)
            : row['hasTrackedSettings'] == true);
    final metrics = DocumentColumnGeometry.resolve(
      settings,
      math.max(1, available(row) / 15),
    );
    return AlertDialog(
      title: const Text('Columns'),
      content: SizedBox(
        width: 650,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Wrap(
                spacing: 12,
                runSpacing: 12,
                children: [
                  SizedBox(
                    width: 205,
                    child: DropdownButtonFormField<int>(
                      key: const Key('columns-section'),
                      initialValue: section,
                      isExpanded: true,
                      decoration: const InputDecoration(labelText: 'Section'),
                      items: [
                        for (var i = 0; i < sections.length; i++)
                          DropdownMenuItem(
                            value: i,
                            child: Text(
                              'Section ${i + 1} of ${sections.length}',
                            ),
                          ),
                      ],
                      onChanged: (v) => setState(() {
                        section = v!;
                        settings = columnSettings(sections[section]);
                        changed = false;
                        resetFields();
                      }),
                    ),
                  ),
                  SizedBox(
                    width: 205,
                    child: DropdownButtonFormField<String>(
                      key: const Key('columns-scope'),
                      initialValue: scope,
                      isExpanded: true,
                      decoration: const InputDecoration(labelText: 'Apply to'),
                      items: const [
                        DropdownMenuItem(
                          value: 'section',
                          child: Text('This section'),
                        ),
                        DropdownMenuItem(
                          value: 'document',
                          child: Text('Whole document'),
                        ),
                      ],
                      onChanged: (v) => setState(() => scope = v!),
                    ),
                  ),
                  SizedBox(
                    width: 120,
                    child: DropdownButtonFormField<String>(
                      initialValue: unit,
                      decoration: const InputDecoration(labelText: 'Units'),
                      items: [
                        for (final u in ['in', 'cm', 'pt'])
                          DropdownMenuItem(value: u, child: Text(u)),
                      ],
                      onChanged: (v) => setState(() {
                        unit = v!;
                        resetFields();
                      }),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              Wrap(
                spacing: 8,
                children: [
                  for (final name in ['One', 'Two', 'Three', 'Left', 'Right'])
                    OutlinedButton(
                      key: Key('columns-preset-${name.toLowerCase()}'),
                      onPressed: () => setState(() => preset(name)),
                      child: Text(name),
                    ),
                ],
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 16,
                runSpacing: 12,
                children: [
                  measurement('count', 'Number of columns', count, (v) {
                    settings['count'] = v;
                    if (v >= 1 && v <= 45 && settings['equalWidth'] == false) {
                      customWidths();
                      fields.keys
                          .where(
                            (k) => k.startsWith('width') || k.startsWith('gap'),
                          )
                          .toList()
                          .forEach((k) => fields.remove(k)?.dispose());
                    }
                  }, whole: true),
                  if (settings['equalWidth'] == true)
                    measurement(
                      'space',
                      'Spacing',
                      settings['space'],
                      (v) => settings['space'] = v,
                    ),
                ],
              ),
              CheckboxListTile(
                key: const Key('columns-equal'),
                contentPadding: EdgeInsets.zero,
                title: const Text('Equal column width'),
                value: settings['equalWidth'] == true,
                onChanged: (v) => setState(() {
                  settings['equalWidth'] = v!;
                  if (v) {
                    settings['columns'] = [];
                  } else if (count >= 1 && count <= 45) {
                    customWidths();
                  }
                  changed = true;
                  resetFields();
                }),
              ),
              if (settings['equalWidth'] == false) ...[
                for (var i = 0; i < definitions.length; i++)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 12),
                    child: Wrap(
                      spacing: 16,
                      runSpacing: 8,
                      children: [
                        measurement(
                          'width-$i',
                          'Column ${i + 1} width',
                          definitions[i]['width'],
                          (v) => definitions[i]['width'] = v,
                        ),
                        if (i + 1 < definitions.length)
                          measurement(
                            'gap-$i',
                            'Space after column ${i + 1}',
                            definitions[i]['space'],
                            (v) => definitions[i]['space'] = v,
                          ),
                      ],
                    ),
                  ),
              ],
              CheckboxListTile(
                key: const Key('columns-separator'),
                contentPadding: EdgeInsets.zero,
                title: const Text('Line between columns'),
                value: settings['separator'] == true,
                onChanged: (v) => setState(() {
                  settings['separator'] = v!;
                  changed = true;
                }),
              ),
              CheckboxListTile(
                key: const Key('columns-rtl'),
                contentPadding: EdgeInsets.zero,
                title: const Text('Read columns from right to left'),
                value: settings['rtl'] == true,
                onChanged: (v) => setState(() {
                  settings['rtl'] = v!;
                  changed = true;
                }),
              ),
              SizedBox(
                height: 100,
                width: double.infinity,
                child: CustomPaint(
                  painter: _ColumnsDiagram(
                    metrics,
                    math.max(1, available(row) / 15),
                  ),
                ),
              ),
              const SizedBox(height: 8),
              const Text(
                'Use Page preview to see text flow. To change only part of the document, place that text in its own section.',
                style: TextStyle(fontSize: 12),
              ),
              if (blocked)
                const Text(
                  'Unlock document protection or review the pending section formatting change before applying columns.',
                  style: TextStyle(color: Colors.red),
                ),
              if (validation != null)
                Text(
                  validation!,
                  key: const Key('columns-error'),
                  style: const TextStyle(color: Colors.red),
                ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const Key('columns-cancel'),
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('columns-apply'),
          onPressed: blocked || validation != null || !changed
              ? null
              : () => Navigator.pop(context, {
                  'section': section,
                  'sectionKey': row['key'],
                  'scope': scope,
                  'patch': {'columnLayout': settings},
                }),
          child: const Text('Apply'),
        ),
      ],
    );
  }
}

class _ColumnsDiagram extends CustomPainter {
  final DocumentColumnGeometry geometry;
  final double width;
  _ColumnsDiagram(this.geometry, this.width);
  @override
  void paint(Canvas canvas, Size size) {
    final scale = math.min(1.0, (size.width - 24) / width),
        x = (size.width - width * scale) / 2;
    for (final col in geometry.columns) {
      canvas.drawRect(
        Rect.fromLTWH(x + col.left * scale, 8, col.width * scale, 80),
        Paint()..color = const Color(0xffe7edf7),
      );
      for (var y = 18.0; y < 82; y += 10) {
        canvas.drawLine(
          Offset(x + (col.left + 4) * scale, y),
          Offset(x + (col.left + col.width - 4) * scale, y),
          Paint()
            ..color = const Color(0xff6b7a90)
            ..strokeWidth = 1,
        );
      }
    }
    if (geometry.separator) {
      for (var i = 0; i + 1 < geometry.columns.length; i++) {
        final a = geometry.columns[i], b = geometry.columns[i + 1];
        final at = x + (a.left + a.width + b.left) / 2 * scale;
        canvas.drawLine(
          Offset(at, 8),
          Offset(at, 88),
          Paint()..color = Colors.black,
        );
      }
    }
  }

  @override
  bool shouldRepaint(covariant _ColumnsDiagram oldDelegate) => true;
}
