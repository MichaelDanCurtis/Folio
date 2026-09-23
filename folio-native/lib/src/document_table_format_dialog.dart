import 'draft_text_field.dart';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'document_tables.dart';

class DocumentTableFormatDialog extends StatefulWidget {
  final DocumentTable table;
  final TableArea area;
  const DocumentTableFormatDialog({
    super.key,
    required this.table,
    required this.area,
  });
  @override
  State<DocumentTableFormatDialog> createState() =>
      _DocumentTableFormatDialogState();
}

class _DocumentTableFormatDialogState extends State<DocumentTableFormatDialog> {
  final width = TextEditingController(), height = TextEditingController();
  final patch = <String, dynamic>{};
  String rule = 'auto';
  String? error;
  bool heightChanged = false;
  bool? header;
  bool? allowBreak;
  @override
  void initState() {
    super.initState();
    final t = widget.table, a = widget.area;
    final widths = List<num>.from(
      t.node['attrs']?['columnWidths'] ?? List.filled(t.columns, 2400),
    );
    final selected = widths.sublist(a.left, a.right + 1);
    if (selected.every((n) => n == selected.first)) {
      width.text = _points(selected.first);
    }
    final h = t.rows[a.top]['attrs']?['height'];
    final heights = [
      for (var r = a.top; r <= a.bottom; r++)
        jsonEncode(t.rows[r]['attrs']?['height']),
    ];
    rule = heights.every((v) => v == heights.first)
        ? (h?['rule'] ?? 'auto')
        : 'unchanged';
    if (h != null && h['value'] > 0) height.text = _points(h['value']);
    final headers = [
      for (var r = a.top; r <= a.bottom; r++)
        t.rows[r]['attrs']?['header'] == true,
    ];
    header = headers.every((v) => v == headers.first) ? headers.first : null;
    final breaks = [
      for (var r = a.top; r <= a.bottom; r++) tableRowCanSplit(t.rows[r]),
    ];
    allowBreak = breaks.every((v) => v == breaks.first) ? breaks.first : null;
  }

  String _points(num n) =>
      (n / 20).toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), '');
  int? dimension(String value) {
    final n = double.tryParse(value);
    return n != null && n.isFinite && n > 0 && n <= 1584
        ? (n * 20).round().clamp(1, 31680)
        : null;
  }

  void apply() {
    if (patch.containsKey('columnWidth')) {
      final n = dimension(width.text);
      if (n == null) {
        setState(
          () => error =
              'Enter a column width greater than 0 and at most 1,584 pt.',
        );
        return;
      }
      patch['columnWidth'] = n;
    }
    if (heightChanged && rule != 'unchanged') {
      final n = rule == 'auto' ? 0 : dimension(height.text);
      if (n == null) {
        setState(
          () =>
              error = 'Enter a row height greater than 0 and at most 1,584 pt.',
        );
        return;
      }
      patch['height'] = {'rule': rule, 'value': n};
    }
    Navigator.pop(context, patch);
  }

  @override
  void dispose() {
    width.dispose();
    height.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final a = widget.area;
    return AlertDialog(
      title: const Text('Table properties'),
      content: SizedBox(
        width: 460,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                'Rows ${a.top + 1}–${a.bottom + 1} · Columns ${a.left + 1}–${a.right + 1}',
              ),
              const Text('Only changed properties are applied.'),
              const SizedBox(height: 16),
              draftTextField(
                key: const Key('table-column-width'),
                controller: width,
                keyboardType: const TextInputType.numberWithOptions(
                  decimal: true,
                ),
                decoration: const InputDecoration(
                  labelText: 'Column width (pt)',
                  hintText: 'Mixed widths',
                  helperText:
                      'Applies to each selected column. Sets fixed table widths.',
                ),
                onChanged: (_) {
                  setState(() {
                    patch['columnWidth'] = true;
                    patch.remove('equalColumns');
                  });
                },
              ),
              Align(
                alignment: Alignment.centerLeft,
                child: TextButton(
                  key: const Key('table-equal-widths'),
                  onPressed: () {
                    setState(() {
                      patch['equalColumns'] = true;
                      patch.remove('columnWidth');
                    });
                  },
                  child: Text(
                    patch['equalColumns'] == true
                        ? 'Equal widths selected'
                        : 'Distribute selected columns evenly',
                  ),
                ),
              ),
              const SizedBox(height: 8),
              DropdownButtonFormField<String>(
                key: const Key('table-height-rule'),
                initialValue: rule,
                decoration: const InputDecoration(labelText: 'Row height'),
                items: [
                  for (final e in {
                    'unchanged': 'Keep existing heights',
                    'auto': 'Automatic',
                    'atLeast': 'At least',
                    'exact': 'Exactly',
                  }.entries)
                    DropdownMenuItem(value: e.key, child: Text(e.value)),
                ],
                onChanged: (v) {
                  setState(() {
                    rule = v!;
                    heightChanged = true;
                    if (!['auto', 'unchanged'].contains(rule) &&
                        height.text.isEmpty) {
                      height.text = '24';
                    }
                  });
                },
              ),
              const SizedBox(height: 8),
              draftTextField(
                key: const Key('table-row-height'),
                controller: height,
                enabled: !['auto', 'unchanged'].contains(rule),
                keyboardType: const TextInputType.numberWithOptions(
                  decimal: true,
                ),
                decoration: InputDecoration(
                  labelText: 'Height (pt)',
                  helperText: rule == 'exact'
                      ? 'Text beyond an exact height is clipped, but retained.'
                      : 'Applies to each selected row.',
                ),
                onChanged: (_) => heightChanged = true,
              ),
              const Divider(height: 28),
              CheckboxListTile(
                key: const Key('table-allow-row-break'),
                contentPadding: EdgeInsets.zero,
                controlAffinity: ListTileControlAffinity.leading,
                title: const Text('Allow row to break across pages'),
                subtitle: const Text(
                  'Applies to each selected row. Rows taller than a page still continue.',
                ),
                tristate: true,
                value: allowBreak,
                onChanged: (_) => setState(() {
                  allowBreak = allowBreak != true;
                  patch['cantSplit'] = !allowBreak!;
                }),
              ),
              CheckboxListTile(
                key: const Key('table-repeat-header'),
                contentPadding: EdgeInsets.zero,
                controlAffinity: ListTileControlAffinity.leading,
                title: const Text('Repeat as header rows on each page'),
                subtitle: Text(
                  widget.table.headerRows < a.top
                      ? 'Only consecutive header rows starting at the first row repeat. Mark the rows above this selection too.'
                      : 'Repeats in Page preview. Applies to each selected row.',
                ),
                tristate: true,
                value: header,
                onChanged: (_) => setState(() {
                  header = header != true;
                  patch['header'] = header;
                }),
              ),
              const Divider(height: 28),
              DropdownButtonFormField<String>(
                key: const Key('table-cell-fill'),
                decoration: const InputDecoration(labelText: 'Cell fill'),
                hint: const Text('Keep existing fill'),
                items: [
                  for (final e in {
                    'auto': 'No fill',
                    'FFFFFF': 'White',
                    'D9EAF7': 'Light blue',
                    'E2F0D9': 'Light green',
                    'FFF2CC': 'Light yellow',
                    'FCE4D6': 'Light orange',
                    'F2F2F2': 'Light gray',
                  }.entries)
                    DropdownMenuItem(
                      value: e.key,
                      child: Row(
                        children: [
                          Container(
                            width: 16,
                            height: 16,
                            decoration: BoxDecoration(
                              color: e.key == 'auto'
                                  ? Colors.transparent
                                  : Color(
                                      0xff000000 | int.parse(e.key, radix: 16),
                                    ),
                              border: Border.all(color: Colors.grey),
                            ),
                          ),
                          const SizedBox(width: 10),
                          Text(e.value),
                        ],
                      ),
                    ),
                ],
                onChanged: (v) => patch['fill'] = v,
              ),
              const SizedBox(height: 12),
              DropdownButtonFormField<String>(
                key: const Key('table-cell-valign'),
                decoration: const InputDecoration(
                  labelText: 'Cell vertical alignment',
                ),
                hint: const Text('Keep existing alignment'),
                items: [
                  for (final e in {
                    'top': 'Top',
                    'center': 'Center',
                    'bottom': 'Bottom',
                  }.entries)
                    DropdownMenuItem(value: e.key, child: Text(e.value)),
                ],
                onChanged: (v) => patch['verticalAlign'] = v,
              ),
              const SizedBox(height: 12),
              DropdownButtonFormField<String>(
                key: const Key('table-alignment'),
                decoration: const InputDecoration(
                  labelText: 'Whole table alignment',
                ),
                hint: const Text('Keep existing alignment'),
                items: [
                  for (final e in {
                    'left': 'Left',
                    'center': 'Center',
                    'right': 'Right',
                  }.entries)
                    DropdownMenuItem(value: e.key, child: Text(e.value)),
                ],
                onChanged: (v) => patch['alignment'] = v,
              ),
              if (error != null)
                Padding(
                  padding: const EdgeInsets.only(top: 12),
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
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('table-format-apply'),
          onPressed: apply,
          child: const Text('Apply to draft'),
        ),
      ],
    );
  }
}
