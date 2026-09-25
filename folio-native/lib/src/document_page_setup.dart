import 'dart:math' as math;
import 'scaled_box.dart';
import 'package:flutter/material.dart';
import 'draft_text_field.dart';
import 'document_codec.dart';

const defaultPageGeometry = <String, dynamic>{
  'width': 12240,
  'height': 15840,
  'orientation': 'portrait',
  'top': 1440,
  'bottom': 1440,
  'left': 1440,
  'right': 1440,
  'header': 720,
  'footer': 720,
  'gutter': 0,
  'start': 'nextPage',
};

int pageSectionAtSelection(Map document, TextSelection selection) {
  final blocks = document['content'] as List? ?? [];
  var offset = 0, block = 0;
  for (var i = 0; i < blocks.length; i++) {
    final length = documentToDelta({
      'type': 'doc',
      'content': [blocks[i]],
    }).operations.fold<int>(0, (length, op) => length + (op.length ?? 0));
    block = i;
    if (selection.start < offset + length) {
      break;
    }
    offset += length;
  }
  final sections = document['attrs']?['pageLayout']?['sections'] as List? ?? [];
  for (var i = 0; i < sections.length; i++) {
    final end = sections[i]['endOfficeId'];
    if (end == null) {
      return i;
    }
    final endIndex = blocks.lastIndexWhere(
      (n) => n['attrs']?['officeId'] == end,
    );
    if (endIndex >= block) {
      return i;
    }
  }
  return 0;
}

class DocumentPageGeometry {
  final Map geometry;
  final bool gutterTop, rtlGutter;
  DocumentPageGeometry(
    this.geometry, {
    this.gutterTop = false,
    this.rtlGutter = false,
  });
  factory DocumentPageGeometry.fromDocument(Map? document) {
    final layout = document?['attrs']?['pageLayout'];
    final sections = layout?['sections'] as List? ?? [];
    final section = sections.firstOrNull as Map?;
    return DocumentPageGeometry(
      section?['geometry'] as Map? ?? defaultPageGeometry,
      gutterTop: layout?['gutterTop'] == true,
      rtlGutter: section?['rtlGutter'] == true,
    );
  }
  double twips(String name) =>
      (geometry[name] as num? ?? defaultPageGeometry[name] as num).toDouble();
  double get width => twips('width').clamp(1440, 31680) / 15;
  double get height => twips('height').clamp(1440, 31680) / 15;
  EdgeInsets get margins {
    final gutter = twips('gutter').clamp(0, 31680) / 15;
    var left = math.max(
      0.0,
      twips('left') / 15 + (!gutterTop && !rtlGutter ? gutter : 0),
    );
    var right = math.max(
      0.0,
      twips('right') / 15 + (!gutterTop && rtlGutter ? gutter : 0),
    );
    var top = twips('top').abs() / 15 + (gutterTop ? gutter : 0);
    var bottom = twips('bottom').abs() / 15;
    // Keep valid asymmetric margins exact. Only malformed imported geometry
    // needs a proportional fallback to leave a usable editing surface.
    if (left + right >= width) {
      final factor = (width - 24) / (left + right);
      left *= factor;
      right *= factor;
    }
    if (top + bottom >= height) {
      final factor = (height - 24) / (top + bottom);
      top *= factor;
      bottom *= factor;
    }
    return EdgeInsets.fromLTRB(left, top, right, bottom);
  }

  double get contentHeight => math.max(24, height - margins.vertical);
}

/// Continuous editing at the section's physical paper width. Pagination is a
/// separate layout concern; this frame never pretends overflow is another page.
class DocumentPageFrame extends StatefulWidget {
  final DocumentPageGeometry geometry;
  final Widget child;

  /// The zoom (1 is 100%), and the viewport size for fitting zooms.
  final double zoom;
  final ValueChanged<Size>? onViewport;
  const DocumentPageFrame({
    super.key,
    required this.geometry,
    required this.child,
    this.zoom = 1,
    this.onViewport,
  });
  @override
  State<DocumentPageFrame> createState() => _DocumentPageFrameState();
}

class _DocumentPageFrameState extends State<DocumentPageFrame> {
  final horizontal = ScrollController();
  @override
  void dispose() {
    horizontal.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => ColoredBox(
    color: const Color(0xffe8ebef),
    child: LayoutBuilder(
      builder: (context, viewport) {
        widget.onViewport?.call(viewport.biggest);
        final width = widget.geometry.width * widget.zoom;
        return Scrollbar(
          controller: horizontal,
          thumbVisibility: width + 48 > viewport.maxWidth,
          notificationPredicate: (n) => n.metrics.axis == Axis.horizontal,
          child: SingleChildScrollView(
            controller: horizontal,
            scrollDirection: Axis.horizontal,
            child: SizedBox(
              width: math.max(viewport.maxWidth, width + 48),
              height: viewport.maxHeight,
              child: SingleChildScrollView(
                padding: const EdgeInsets.fromLTRB(24, 24, 24, 40),
                child: Center(
                  child: ScaledBox(
                    key: const Key('document-zoom'),
                    scale: widget.zoom,
                    child: Container(
                      key: const Key('document-paper'),
                      width: widget.geometry.width,
                      constraints: BoxConstraints(
                        minHeight: widget.geometry.height,
                      ),
                      padding: widget.geometry.margins,
                      foregroundDecoration: BoxDecoration(
                        border: Border.all(color: const Color(0xffd1d5da)),
                      ),
                      decoration: const BoxDecoration(
                        color: Colors.white,
                        boxShadow: [
                          BoxShadow(
                            color: Color(0x14000000),
                            blurRadius: 8,
                            offset: Offset(0, 2),
                          ),
                        ],
                      ),
                      child: widget.child,
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
      },
    ),
  );
}

class DocumentPageSetupDialog extends StatefulWidget {
  final Map data;
  final int initialSection;
  const DocumentPageSetupDialog({
    super.key,
    required this.data,
    this.initialSection = 0,
  });
  @override
  State<DocumentPageSetupDialog> createState() =>
      _DocumentPageSetupDialogState();
}

class _DocumentPageSetupDialogState extends State<DocumentPageSetupDialog> {
  late int section = widget.initialSection.clamp(0, sections.length - 1);
  String scope = 'section', unit = 'in';
  final patch = <String, dynamic>{};
  final invalid = <String>{};
  final fields = <String, TextEditingController>{};
  List<Map> get sections => (widget.data['sections'] as List).cast<Map>();
  Map get row => sections[section];
  Map<String, dynamic> get geometry => {
    ...Map<String, dynamic>.from(row['geometry']),
    ...patch,
  };
  double get scale => {'in': 1440.0, 'cm': 1440 / 2.54, 'pt': 20.0}[unit]!;
  static const papers = {
    'Letter': [12240, 15840],
    'Legal': [12240, 20160],
    'A4': [11906, 16838],
    'A5': [8391, 11906],
    'Tabloid': [15840, 24480],
  };
  static const labels = {
    'width': 'Width',
    'height': 'Height',
    'top': 'Top',
    'bottom': 'Bottom',
    'left': 'Left',
    'right': 'Right',
    'gutter': 'Gutter',
    'header': 'Header from edge',
    'footer': 'Footer from edge',
  };
  String format(num n) =>
      (n / scale).toStringAsFixed(3).replaceFirst(RegExp(r'\.?0+$'), '');
  @override
  void initState() {
    super.initState();
    for (final key in labels.keys) {
      fields[key] = TextEditingController(text: format(geometry[key]));
    }
  }

  void refreshFields() {
    for (final key in labels.keys) {
      fields[key]!.text = format(geometry[key]);
    }
    invalid.clear();
  }

  @override
  void dispose() {
    for (final field in fields.values) {
      field.dispose();
    }
    super.dispose();
  }

  String get paper {
    final g = geometry;
    final dims = [g['width'] as int, g['height'] as int]..sort();
    return papers.entries
            .where((p) => p.value[0] == dims[0] && p.value[1] == dims[1])
            .firstOrNull
            ?.key ??
        'Custom';
  }

  String? get validation {
    if (invalid.isNotEmpty) {
      return 'Enter valid measurements.';
    }
    final g = geometry;
    int n(String k) => g[k] as int;
    if (n('width') < 1440 ||
        n('height') < 1440 ||
        n('width') > 31680 ||
        n('height') > 31680) {
      return 'Paper dimensions must be between 1 and 22 inches.';
    }
    for (final key in labels.keys) {
      if (n(key).abs() > 31680 ||
          (!['top', 'bottom'].contains(key) && n(key) < 0)) {
        return 'Check the ${labels[key]!.toLowerCase()} measurement.';
      }
    }
    final topGutter = widget.data['gutterTop'] == true;
    if (n('left') + n('right') + (topGutter ? 0 : n('gutter')) >= n('width') ||
        n('top').abs() + n('bottom').abs() + (topGutter ? n('gutter') : 0) >=
            n('height')) {
      return 'The margins and gutter must leave space for document text.';
    }
    if (n('header') >= n('height') || n('footer') >= n('height')) {
      return 'Header and footer distances must fit inside the paper.';
    }
    return null;
  }

  Widget field(String key) => SizedBox(
    width: 160,
    child: draftTextField(
      key: Key('page-$key'),
      controller: fields[key],
      decoration: InputDecoration(
        labelText: labels[key],
        suffixText: unit,
        errorText: invalid.contains(key) ? 'Enter a number' : null,
      ),
      keyboardType: const TextInputType.numberWithOptions(
        decimal: true,
        signed: true,
      ),
      onChanged: (text) => setState(() {
        final value = double.tryParse(text);
        if (value == null || !value.isFinite || value.abs() > 100000) {
          invalid.add(key);
          return;
        }
        invalid.remove(key);
        patch[key] = (value * scale).round();
        if (key == 'width' || key == 'height') {
          patch['orientation'] = geometry['width'] > geometry['height']
              ? 'landscape'
              : 'portrait';
        }
      }),
    ),
  );
  @override
  Widget build(BuildContext context) {
    final g = geometry;
    final blocked =
        widget.data['protected'] == true ||
        (scope == 'document'
            ? sections.any((s) => s['hasTrackedSettings'] == true)
            : row['hasTrackedSettings'] == true);
    return AlertDialog(
      title: const Text('Page setup'),
      content: SizedBox(
        width: 720,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Wrap(
                spacing: 16,
                runSpacing: 12,
                children: [
                  SizedBox(
                    width: 215,
                    child: DropdownButtonFormField<int>(
                      isExpanded: true,
                      key: const Key('page-section'),
                      initialValue: section,
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
                      onChanged: (value) => setState(() {
                        section = value!;
                        refreshFields();
                      }),
                    ),
                  ),
                  SizedBox(
                    width: 215,
                    child: DropdownButtonFormField<String>(
                      isExpanded: true,
                      key: const Key('page-scope'),
                      initialValue: scope,
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
                      onChanged: (value) => setState(() => scope = value!),
                    ),
                  ),
                  SizedBox(
                    width: 150,
                    child: DropdownButtonFormField<String>(
                      isExpanded: true,
                      key: const Key('page-unit'),
                      initialValue: unit,
                      decoration: const InputDecoration(labelText: 'Units'),
                      items: const [
                        DropdownMenuItem(value: 'in', child: Text('Inches')),
                        DropdownMenuItem(
                          value: 'cm',
                          child: Text('Centimeters'),
                        ),
                        DropdownMenuItem(value: 'pt', child: Text('Points')),
                      ],
                      onChanged: invalid.isNotEmpty
                          ? null
                          : (value) => setState(() {
                              unit = value!;
                              refreshFields();
                            }),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              const Text(
                'Only changed settings apply. Other section settings stay as they are.',
              ),
              if (blocked)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    widget.data['protected'] == true
                        ? 'This document is protected. Unlock it in Word to change page setup.'
                        : 'Review this section’s pending formatting change before changing its page setup.',
                  ),
                ),
              const SizedBox(height: 16),
              Wrap(
                spacing: 16,
                runSpacing: 12,
                crossAxisAlignment: WrapCrossAlignment.start,
                children: [
                  SizedBox(
                    width: math.min(
                      500,
                      MediaQuery.sizeOf(context).width - 128,
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Wrap(
                          spacing: 16,
                          runSpacing: 12,
                          children: [
                            SizedBox(
                              width: 215,
                              child: DropdownButtonFormField<String>(
                                isExpanded: true,
                                key: ValueKey('page-paper-$paper'),
                                initialValue: paper,
                                decoration: const InputDecoration(
                                  labelText: 'Paper size',
                                ),
                                items: [
                                  for (final name in ['Custom', ...papers.keys])
                                    DropdownMenuItem(
                                      value: name,
                                      child: Text(name),
                                    ),
                                ],
                                onChanged: (name) => setState(() {
                                  if (name == 'Custom') {
                                    return;
                                  }
                                  final dims = papers[name]!;
                                  final landscape =
                                      geometry['orientation'] == 'landscape';
                                  patch['width'] = dims[landscape ? 1 : 0];
                                  patch['height'] = dims[landscape ? 0 : 1];
                                  refreshFields();
                                }),
                              ),
                            ),
                            SizedBox(
                              width: 215,
                              child: DropdownButtonFormField<String>(
                                isExpanded: true,
                                key: ValueKey(
                                  'page-orientation-${g['orientation']}',
                                ),
                                initialValue: g['orientation'],
                                decoration: const InputDecoration(
                                  labelText: 'Orientation',
                                ),
                                items: const [
                                  DropdownMenuItem(
                                    value: 'portrait',
                                    child: Text('Portrait'),
                                  ),
                                  DropdownMenuItem(
                                    value: 'landscape',
                                    child: Text('Landscape'),
                                  ),
                                ],
                                onChanged: (value) => setState(() {
                                  final dims = [
                                    geometry['width'] as int,
                                    geometry['height'] as int,
                                  ]..sort();
                                  patch['orientation'] = value;
                                  patch['width'] =
                                      dims[value == 'landscape' ? 1 : 0];
                                  patch['height'] =
                                      dims[value == 'landscape' ? 0 : 1];
                                  refreshFields();
                                }),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 12),
                        Wrap(
                          spacing: 16,
                          runSpacing: 12,
                          children: [field('width'), field('height')],
                        ),
                        const SizedBox(height: 16),
                        Row(
                          children: [
                            const Text(
                              'Margins',
                              style: TextStyle(fontWeight: FontWeight.w600),
                            ),
                            const SizedBox(width: 12),
                            PopupMenuButton<String>(
                              key: const Key('page-margin-presets'),
                              tooltip: 'Margin presets',
                              onSelected: (name) => setState(() {
                                final side = {
                                  'Normal': 1440,
                                  'Narrow': 720,
                                  'Moderate': 1080,
                                  'Wide': 2880,
                                }[name]!;
                                patch.addAll({
                                  'left': side,
                                  'right': side,
                                  'top': name == 'Narrow' ? 720 : 1440,
                                  'bottom': name == 'Narrow' ? 720 : 1440,
                                });
                                refreshFields();
                              }),
                              itemBuilder: (_) => [
                                for (final name in [
                                  'Normal',
                                  'Narrow',
                                  'Moderate',
                                  'Wide',
                                ])
                                  PopupMenuItem(value: name, child: Text(name)),
                              ],
                              child: const Text('Presets ▾'),
                            ),
                          ],
                        ),
                        Wrap(
                          spacing: 16,
                          runSpacing: 12,
                          children: [
                            field('top'),
                            field('bottom'),
                            field('left'),
                            field('right'),
                            field('gutter'),
                          ],
                        ),
                        const SizedBox(height: 16),
                        const Text(
                          'Headers and footers',
                          style: TextStyle(fontWeight: FontWeight.w600),
                        ),
                        Wrap(
                          spacing: 16,
                          runSpacing: 12,
                          children: [field('header'), field('footer')],
                        ),
                        const SizedBox(height: 16),
                        SizedBox(
                          width: 310,
                          child: DropdownButtonFormField<String>(
                            isExpanded: true,
                            key: ValueKey('page-start-${g['start']}'),
                            initialValue: g['start'],
                            decoration: const InputDecoration(
                              labelText: 'Section start',
                            ),
                            items: [
                              for (final entry in {
                                'nextPage': 'New page',
                                'continuous': 'Continuous',
                                'evenPage': 'Even page',
                                'oddPage': 'Odd page',
                                'nextColumn': 'New column',
                              }.entries)
                                DropdownMenuItem(
                                  value: entry.key,
                                  child: Text(entry.value),
                                ),
                            ],
                            onChanged: (value) =>
                                setState(() => patch['start'] = value),
                          ),
                        ),
                      ],
                    ),
                  ),
                  SizedBox(
                    width: 175,
                    child: Column(
                      children: [
                        const Text('Preview'),
                        const SizedBox(height: 12),
                        SizedBox(
                          width: 175,
                          height: 225,
                          child: CustomPaint(
                            key: const Key('page-setup-preview'),
                            painter: _PagePreview(
                              DocumentPageGeometry(
                                g,
                                gutterTop: widget.data['gutterTop'] == true,
                                rtlGutter: row['rtlGutter'] == true,
                              ),
                            ),
                          ),
                        ),
                        Text(
                          '${format(g['width'])} × ${format(g['height'])} $unit',
                          style: const TextStyle(fontSize: 12),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              if (row['inheritedPageGeometry'] == true)
                const Text(
                  'This continuous section inherits unspecified page settings from the following section.',
                ),
              if (widget.data['mirrorMargins'] == true)
                const Text(
                  'Mirrored margins are retained; Left and Right represent inside and outside margins.',
                ),
              if (widget.data['gutterTop'] == true)
                const Text('The document’s gutter is at the top.'),
              const Text(
                'The editor uses a continuous view with Section 1’s page size. Pagination, columns, and headers or footers are not fully displayed.',
                style: TextStyle(fontSize: 12, color: Color(0xff616161)),
              ),
              if (validation != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    validation!,
                    key: const Key('page-setup-error'),
                    style: const TextStyle(color: Colors.red),
                  ),
                ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const Key('page-setup-cancel'),
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('page-setup-apply'),
          onPressed: blocked || validation != null || patch.isEmpty
              ? null
              : () => Navigator.pop(context, {
                  'section': section,
                  'sectionKey': row['key'],
                  'scope': scope,
                  'patch': patch,
                }),
          child: const Text('Apply'),
        ),
      ],
    );
  }
}

class _PagePreview extends CustomPainter {
  final DocumentPageGeometry geometry;
  _PagePreview(this.geometry);
  @override
  void paint(Canvas canvas, Size size) {
    final scale = math.min(
      (size.width - 8) / geometry.width,
      (size.height - 8) / geometry.height,
    );
    final paper = Rect.fromLTWH(
      (size.width - geometry.width * scale) / 2,
      4,
      geometry.width * scale,
      geometry.height * scale,
    );
    canvas.drawRect(paper, Paint()..color = Colors.white);
    canvas.drawRect(
      paper,
      Paint()
        ..color = const Color(0xffadb6c3)
        ..style = PaintingStyle.stroke,
    );
    final m = geometry.margins;
    final content = Rect.fromLTRB(
      paper.left + m.left * scale,
      paper.top + m.top * scale,
      paper.right - m.right * scale,
      paper.bottom - m.bottom * scale,
    );
    canvas.drawRect(content, Paint()..color = const Color(0xffeaf0fc));
    for (var y = content.top + 6; y < content.bottom - 4; y += 8) {
      canvas.drawLine(
        Offset(content.left + 4, y),
        Offset(content.right - 4, y),
        Paint()
          ..color = const Color(0xffbdcce5)
          ..strokeWidth = 1.5,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _PagePreview oldDelegate) => true;
}
