import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'draft_text_field.dart';
import 'document_paragraph_format.dart';
import 'document_styles.dart';

class DocumentParagraphDialog extends StatefulWidget {
  final List<Map<String, dynamic>> formats;
  final Delta sample;
  final q.DefaultStyles? styles;
  const DocumentParagraphDialog({
    required this.formats,
    required this.sample,
    this.styles,
    super.key,
  });
  @override
  State<DocumentParagraphDialog> createState() =>
      _DocumentParagraphDialogState();
}

class _DocumentParagraphDialogState extends State<DocumentParagraphDialog> {
  final patch = <String, dynamic>{};
  final errors = <String>{};
  final controllers = <String, TextEditingController>{};
  late final first = paragraphProperties(widget.formats.first);
  late String lineMode = _initialMode();
  late String outlineMode = () {
    final choices = widget.formats
        .map((f) => f['direct']?['outlineLevel']?.toString() ?? 'style')
        .toSet();
    return choices.length == 1 ? choices.single : 'mixed';
  }();
  dynamic setting(Map f, String key) => key == 'left'
      ? f['start'] ?? f['left']
      : key == 'right'
      ? f['end'] ?? f['right']
      : f[key];
  dynamic common(String key) {
    final values = widget.formats
        .map(paragraphProperties)
        .map((f) => setting(f, key))
        .toList();
    return values.every((v) => v == values.first) ? values.first : null;
  }

  bool mixed(String key) =>
      widget.formats
          .map(paragraphProperties)
          .map((f) => setting(f, key))
          .toSet()
          .length >
      1;
  String _initialMode() => mixed('line') || mixed('lineRule')
      ? 'mixed'
      : (first['lineRule'] == 'exact'
            ? 'exact'
            : first['lineRule'] == 'atLeast'
            ? 'atLeast'
            : {240: 'single', 360: 'oneHalf', 480: 'double'}[first['line']] ??
                  'multiple');
  String number(dynamic value) {
    final n = (value as num).toDouble();
    return n == n.roundToDouble() ? n.toInt().toString() : n.toString();
  }

  @override
  void initState() {
    super.initState();
    for (final key in ['left', 'right', 'before', 'after']) {
      controllers[key] = TextEditingController(
        text: mixed(key) ? '' : number((common(key) as num? ?? 0) / 20),
      );
    }
    controllers['line'] = TextEditingController(
      text: lineMode == 'mixed'
          ? ''
          : number(
              (common('line') as num? ?? 336) /
                  (lineMode == 'multiple' ? 240 : 20),
            ),
    );
  }

  @override
  void dispose() {
    for (final c in controllers.values) {
      c.dispose();
    }
    super.dispose();
  }

  bool blocked(String key) => widget.formats
      .map(paragraphProperties)
      .any(
        (p) =>
            (key == 'left' &&
                [
                  'leftChars',
                  'startChars',
                ].any((k) => (p[k] as num? ?? 0) != 0)) ||
            (key == 'right' &&
                [
                  'rightChars',
                  'endChars',
                ].any((k) => (p[k] as num? ?? 0) != 0)) ||
            (key == 'before' && (p['beforeLines'] as num? ?? 0) != 0) ||
            (key == 'after' && (p['afterLines'] as num? ?? 0) != 0),
      );
  Widget field(String key, String label, {bool multiple = false}) => SizedBox(
    width: 190,
    child: draftTextField(
      key: Key('paragraph-$key'),
      controller: controllers[key],
      enabled: !blocked(key),
      decoration: InputDecoration(
        labelText: label,
        hintText: mixed(key) ? 'Mixed' : '',
        errorText: errors.contains(key)
            ? 'Enter ${multiple
                  ? '0.05–100'
                  : key == 'line'
                  ? '0.05–1,584'
                  : '0–1,584'}.'
            : null,
      ),
      keyboardType: const TextInputType.numberWithOptions(decimal: true),
      onChanged: (text) => setState(() {
        final n = double.tryParse(text);
        final valid =
            n != null &&
            n.isFinite &&
            n >= (multiple || key == 'line' ? 0.05 : 0) &&
            n <= (multiple ? 100 : 1584) &&
            (key != 'line' || n > 0);
        if (!valid) {
          errors.add(key);
          return;
        }
        errors.remove(key);
        patch[key] = (n * (multiple ? 240 : 20)).round();
        if (key == 'line') {
          patch['lineRule'] = lineMode == 'multiple' ? 'auto' : lineMode;
        }
        if (key == 'before' || key == 'after') {
          patch['${key}Autospacing'] = false;
        }
      }),
    ),
  );
  @override
  Widget build(BuildContext context) {
    final limitations = widget.formats
        .map(paragraphProperties)
        .expand(paragraphPreviewLimits)
        .toSet();
    return AlertDialog(
      title: const Text('Paragraph'),
      content: SizedBox(
        width: 520,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Applies to ${widget.formats.length} selected ${widget.formats.length == 1 ? 'paragraph' : 'paragraphs'}. Only changed settings will apply.',
              ),
              if (limitations.isNotEmpty)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 8),
                  child: Text(
                    '${limitations.join(', ')} are preserved, but are not fully shown in this preview.',
                  ),
                ),
              const SizedBox(height: 12),
              DropdownButtonFormField<String>(
                key: ValueKey('paragraph-outline-$outlineMode'),
                initialValue: outlineMode,
                decoration: const InputDecoration(labelText: 'Outline level'),
                items: [
                  const DropdownMenuItem(
                    value: 'mixed',
                    child: Text('Mixed — keep existing'),
                  ),
                  const DropdownMenuItem(
                    value: 'style',
                    child: Text('Use style'),
                  ),
                  const DropdownMenuItem(value: '9', child: Text('Body text')),
                  for (var level = 0; level < 9; level++)
                    DropdownMenuItem(
                      value: '$level',
                      child: Text('Level ${level + 1}'),
                    ),
                ],
                onChanged: (value) => setState(() {
                  outlineMode = value!;
                  if (value == 'mixed') {
                    patch.remove('outlineLevel');
                  } else {
                    patch['outlineLevel'] = value == 'style'
                        ? null
                        : int.parse(value);
                  }
                }),
              ),
              const SizedBox(height: 16),
              const Text('Indents (points)'),
              Wrap(
                spacing: 16,
                runSpacing: 12,
                children: [field('left', 'Left'), field('right', 'Right')],
              ),
              const SizedBox(height: 16),
              const Text('Paragraph spacing (points)'),
              Wrap(
                spacing: 16,
                runSpacing: 12,
                children: [field('before', 'Before'), field('after', 'After')],
              ),
              const SizedBox(height: 16),
              DropdownButtonFormField<String>(
                key: ValueKey('paragraph-line-mode-$lineMode'),
                initialValue: lineMode,
                decoration: const InputDecoration(labelText: 'Line spacing'),
                items: const [
                  DropdownMenuItem(
                    value: 'mixed',
                    child: Text('Mixed — keep existing'),
                  ),
                  DropdownMenuItem(value: 'single', child: Text('Single')),
                  DropdownMenuItem(value: 'oneHalf', child: Text('1.5 lines')),
                  DropdownMenuItem(value: 'double', child: Text('Double')),
                  DropdownMenuItem(value: 'multiple', child: Text('Multiple')),
                  DropdownMenuItem(value: 'exact', child: Text('Exactly')),
                  DropdownMenuItem(value: 'atLeast', child: Text('At least')),
                ],
                onChanged: (value) => setState(() {
                  lineMode = value!;
                  errors.remove('line');
                  if (value == 'mixed') {
                    patch.remove('line');
                    patch.remove('lineRule');
                    return;
                  }
                  final fixed = {
                    'single': 240,
                    'oneHalf': 360,
                    'double': 480,
                  }[value];
                  patch['lineRule'] = ['exact', 'atLeast'].contains(value)
                      ? value
                      : 'auto';
                  patch['line'] = fixed ?? 240;
                  controllers['line']!.text = number(
                    (patch['line'] as num) / (value == 'multiple' ? 240 : 20),
                  );
                }),
              ),
              if (['multiple', 'exact', 'atLeast'].contains(lineMode))
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: field(
                    'line',
                    lineMode == 'multiple'
                        ? 'Line multiple'
                        : 'Line height (points)',
                    multiple: lineMode == 'multiple',
                  ),
                ),
              const SizedBox(height: 16),
              const Text('Preview'),
              const SizedBox(height: 6),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: Colors.white,
                  border: Border.all(color: Colors.black12),
                ),
                child: _ParagraphPreview(
                  key: ValueKey(jsonEncode(patch)),
                  sample: widget.sample,
                  patch: patch,
                  styles: widget.styles,
                ),
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('apply-paragraph'),
          onPressed: errors.isEmpty
              ? () => Navigator.pop(context, Map<String, dynamic>.from(patch))
              : null,
          child: const Text('Apply'),
        ),
      ],
    );
  }
}

class _ParagraphPreview extends StatefulWidget {
  final Delta sample;
  final Map<String, dynamic> patch;
  final q.DefaultStyles? styles;
  const _ParagraphPreview({
    super.key,
    required this.sample,
    required this.patch,
    this.styles,
  });
  @override
  State<_ParagraphPreview> createState() => _ParagraphPreviewState();
}

class _ParagraphPreviewState extends State<_ParagraphPreview> {
  late final controller = q.QuillController(
    document: q.Document.fromDelta(
      widget.sample.compose(
        changeParagraphFormat(
          widget.sample,
          TextSelection(baseOffset: 0, extentOffset: widget.sample.length - 1),
          widget.patch,
          defaults: widget.styles,
        ),
      ),
    ),
    selection: const TextSelection.collapsed(offset: 0),
    readOnly: true,
  );
  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => IgnorePointer(
    child: q.QuillEditor.basic(
      controller: controller,
      config: q.QuillEditorConfig(
        scrollable: false,
        showCursor: false,
        enableInteractiveSelection: false,
        enableSelectionToolbar: false,
        customStyles: widget.styles ?? documentEditorStyles(),
      ),
    ),
  );
}
