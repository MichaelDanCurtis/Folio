import 'package:flutter/material.dart';
import 'draft_text_field.dart';
import 'sheet_font.dart';

class SheetAlignmentDialog extends StatefulWidget {
  final List<Map<String, dynamic>> styles;
  final Map<String, dynamic> previewStyle, normalFont;
  final String sample;
  final dynamic value;
  const SheetAlignmentDialog({
    required this.styles,
    required this.previewStyle,
    required this.sample,
    this.value,
    this.normalFont = const {},
    super.key,
  });
  @override
  State<SheetAlignmentDialog> createState() => _SheetAlignmentDialogState();
}

class _SheetAlignmentDialogState extends State<SheetAlignmentDialog> {
  late final common = commonSheetAlignment(widget.styles);
  final changes = <String, dynamic>{};
  late final indent = TextEditingController(
    text: common['indent']?.toString() ?? '',
  );
  late final degrees = TextEditingController(
    text: common['textRotation'] == 255
        ? '0'
        : common['textRotation']?.toString() ?? '',
  );
  final errors = <String, String>{};
  String? get error => errors.values.firstOrNull;
  dynamic current(String key) =>
      changes.containsKey(key) ? changes[key] : common[key];
  void change(String key, dynamic value) => setState(() {
    changes[key] = value;
    errors.remove(key);
  });
  Widget choice(String key, String label, Map<String, String> options) {
    final value = current(key)?.toString();
    final values = {
      ...options,
      if (value != null && !options.containsKey(value))
        value: 'Keep imported setting ($value)',
    };
    return DropdownButtonFormField<String>(
      key: ValueKey('alignment-$key-$value'),
      initialValue: value,
      isExpanded: true,
      decoration: InputDecoration(
        labelText: label,
        hintText: 'Mixed — unchanged',
      ),
      items: [
        for (final item in values.entries)
          DropdownMenuItem(value: item.key, child: Text(item.value)),
      ],
      onChanged: (v) {
        if (v == null) return;
        change(key, key == 'readingOrder' ? int.parse(v) : v);
        if (key == 'horizontalAlignment' && v == 'justify') {
          change('wrapText', true);
        }
      },
    );
  }

  Widget toggle(String key, String label) => CheckboxListTile(
    key: Key('alignment-$key'),
    dense: true,
    contentPadding: EdgeInsets.zero,
    title: Text(label),
    tristate: true,
    value: current(key) as bool?,
    onChanged: (_) {
      final value = current(key) != true;
      change(key, value);
      if (value && key == 'wrapText') change('shrinkToFit', false);
      if (value && key == 'shrinkToFit') change('wrapText', false);
    },
  );
  void number(String key, String value) {
    final number = int.tryParse(value);
    if (number == null ||
        (key == 'indent'
            ? number < 0 || number > 250
            : number < -90 || number > 90)) {
      setState(
        () => errors[key] = key == 'indent'
            ? 'Enter an indent from 0 to 250.'
            : 'Enter whole degrees from −90 to 90.',
      );
      return;
    }
    change(key, number);
    if (key == 'indent' &&
        number > 0 &&
        ![
          'left',
          'right',
          'distributed',
        ].contains(current('horizontalAlignment'))) {
      change('horizontalAlignment', 'left');
    }
  }

  void reset() => setState(() {
    changes.clear();
    errors.clear();
    indent.text = common['indent']?.toString() ?? '';
    degrees.text = common['textRotation'] == 255
        ? '0'
        : common['textRotation']?.toString() ?? '';
  });
  @override
  Widget build(BuildContext context) {
    final preview = {...widget.previewStyle, ...changes};
    final stacked = current('textRotation') == 255;
    return AlertDialog(
      title: const Text('Alignment'),
      content: SizedBox(
        width: 570,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                '${widget.styles.length} cell${widget.styles.length == 1 ? '' : 's'} selected${common.values.any((v) => v == null) ? ' · Mixed settings stay unchanged' : ''}',
                key: const Key('alignment-summary'),
              ),
              const SizedBox(height: 16),
              Row(
                children: [
                  Expanded(
                    child: choice('horizontalAlignment', 'Horizontal', const {
                      'general': 'General',
                      'left': 'Left',
                      'center': 'Center',
                      'right': 'Right',
                      'justify': 'Justify',
                    }),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: choice('verticalAlignment', 'Vertical', const {
                      'top': 'Top',
                      'center': 'Middle',
                      'bottom': 'Bottom',
                    }),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  Expanded(
                    child: draftTextField(
                      key: const Key('alignment-indent'),
                      controller: indent,
                      keyboardType: TextInputType.number,
                      decoration: const InputDecoration(
                        labelText: 'Indent',
                        hintText: 'Mixed',
                      ),
                      onChanged: (v) => number('indent', v),
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: choice('readingOrder', 'Reading order', const {
                      '0': 'Context',
                      '1': 'Left to right',
                      '2': 'Right to left',
                    }),
                  ),
                ],
              ),
              Row(
                children: [
                  Expanded(child: toggle('wrapText', 'Wrap text')),
                  Expanded(child: toggle('shrinkToFit', 'Shrink to fit')),
                ],
              ),
              Row(
                children: [
                  Expanded(
                    child: draftTextField(
                      key: const Key('alignment-degrees'),
                      controller: degrees,
                      enabled: !stacked,
                      keyboardType: const TextInputType.numberWithOptions(
                        signed: true,
                      ),
                      decoration: const InputDecoration(
                        labelText: 'Rotation (degrees)',
                        helperText: 'Positive tilts upward',
                        hintText: 'Mixed',
                      ),
                      onChanged: (v) => number('textRotation', v),
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: CheckboxListTile(
                      key: const Key('alignment-stacked'),
                      title: const Text('Stack characters'),
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      value: stacked,
                      onChanged: (v) {
                        if (v == true) {
                          change('textRotation', 255);
                        } else {
                          if (degrees.text.isEmpty) degrees.text = '0';
                          number('textRotation', degrees.text);
                        }
                      },
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 14),
              const Text('Preview · first selected cell'),
              const SizedBox(height: 6),
              Container(
                height: 120,
                padding: const EdgeInsets.all(6),
                decoration: BoxDecoration(
                  color: Colors.white,
                  border: Border.all(color: const Color(0xffc7ccd0)),
                ),
                child: SheetFontText(
                  widget.sample.isEmpty ? 'Sample text' : widget.sample,
                  key: const Key('alignment-preview'),
                  font: preview,
                  layout: CellTextLayout(
                    preview,
                    value: widget.value,
                    normalFont: widget.normalFont,
                  ),
                ),
              ),
              const SizedBox(height: 6),
              const Text(
                'Rows with a fixed height may clip wrapped text. Adjust the row height to reveal more.',
                style: TextStyle(fontSize: 11, color: Colors.black54),
              ),
              if (error != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    error!,
                    style: const TextStyle(color: Colors.red),
                  ),
                ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(onPressed: reset, child: const Text('Reset changes')),
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('apply-cell-alignment'),
          onPressed: error != null || changes.isEmpty
              ? null
              : () =>
                    Navigator.pop(context, Map<String, dynamic>.from(changes)),
          child: const Text('Apply'),
        ),
      ],
    );
  }

  @override
  void dispose() {
    indent.dispose();
    degrees.dispose();
    super.dispose();
  }
}
