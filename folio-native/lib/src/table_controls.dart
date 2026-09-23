import 'draft_text_field.dart';
import 'package:flutter/material.dart';
import 'workbook_block.dart' show parseCell;
import 'sheet_view.dart' show cellAddress, columnLabel, displayCell;

class TableArea {
  final int top, left, bottom, right;
  const TableArea(this.top, this.left, this.bottom, this.right);
  factory TableArea.parse(String range) {
    final ends = range.split(':');
    final (r1, c1) = parseCell(ends.first);
    final (r2, c2) = parseCell(ends.last);
    return TableArea(r1, c1, r2, c2);
  }
  bool contains(int r, int c) =>
      r >= top && r <= bottom && c >= left && c <= right;
}

/// Maps the virtual viewport to physical Excel row numbers without allocating
/// a million entries. Hidden rows remain addressable by formulas and Go To.
class VisibleAxis {
  final List<int> hidden;
  final int limit;
  VisibleAxis(Iterable<int> values, this.limit)
    : hidden = (values.where((n) => n >= 0 && n < limit).toSet().toList()
        ..sort());
  int before(int physical) {
    var lo = 0, hi = hidden.length;
    while (lo < hi) {
      final mid = (lo + hi) ~/ 2;
      if (hidden[mid] < physical) {
        lo = mid + 1;
      } else {
        hi = mid;
      }
    }
    return lo;
  }

  bool isHidden(int row) {
    final index = before(row);
    return index < hidden.length && hidden[index] == row;
  }

  int toVisible(int physical) => physical - before(physical);
  int toPhysical(int visible) {
    var lo = visible, hi = (visible + hidden.length).clamp(0, limit - 1);
    while (lo < hi) {
      final mid = (lo + hi) ~/ 2;
      if (mid - before(mid + 1) < visible) {
        lo = mid + 1;
      } else {
        hi = mid;
      }
    }
    return lo;
  }

  int next(int physical, int direction) {
    var r = (physical + direction).clamp(0, limit - 1);
    while (isHidden(r) && r + direction >= 0 && r + direction < limit) {
      r += direction;
    }
    return isHidden(r) ? physical : r;
  }
}

class VisibleRows extends VisibleAxis {
  VisibleRows(Map sheet)
    : super([
        for (final r in (sheet['hiddenRows'] as List? ?? []))
          (r as num).toInt() - 1,
        for (final r in (sheet['autoFilter']?['filteredRows'] as List? ?? []))
          (r as num).toInt() - 1,
        for (final t in (sheet['tables'] as List? ?? []))
          for (final r in (t['filteredRows'] as List? ?? []))
            (r as num).toInt() - 1,
      ], 1048576);
}

class VisibleColumns extends VisibleAxis {
  VisibleColumns(Map sheet)
    : super([
        for (final c in (sheet['hiddenColumns'] as List? ?? []))
          (c as num).toInt() - 1,
      ], 16384);
}

/// Sparse prefix sums keep distant-cell scrolling independent of sheet size.
class SheetAxisLayout {
  final VisibleAxis visible;
  final double defaultExtent;
  final Map<int, double> sizes;
  late final List<int> positions;
  late final List<double> deltas;
  SheetAxisLayout(this.visible, this.defaultExtent, this.sizes) {
    positions = (sizes.keys.where((n) => !visible.isHidden(n)).toList()
      ..sort());
    deltas = [0];
    for (final n in positions) {
      deltas.add(deltas.last + extent(n) - defaultExtent);
    }
  }
  double extent(int physical) =>
      (sizes[physical] ?? defaultExtent).clamp(1.0, double.infinity);
  double offset(int physical) {
    var lo = 0, hi = positions.length;
    while (lo < hi) {
      final mid = (lo + hi) ~/ 2;
      if (positions[mid] < physical) {
        lo = mid + 1;
      } else {
        hi = mid;
      }
    }
    return visible.toVisible(physical) * defaultExtent + deltas[lo];
  }
}

class SheetSizeDialog extends StatefulWidget {
  final bool rows;
  final double size;
  final double? maxSize;
  const SheetSizeDialog({
    super.key,
    required this.rows,
    required this.size,
    this.maxSize,
  });
  @override
  State<SheetSizeDialog> createState() => _SheetSizeDialogState();
}

class _SheetSizeDialogState extends State<SheetSizeDialog> {
  late final value = TextEditingController(
    text: widget.size.toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), ''),
  );
  String? error;
  @override
  void dispose() {
    value.dispose();
    super.dispose();
  }

  void apply() {
    final n = double.tryParse(value.text.trim());
    final max = widget.maxSize ?? (widget.rows ? 409 : 255);
    if (n == null || !n.isFinite || n < 0 || n > max) {
      final label = max == max.roundToDouble()
          ? max.toInt().toString()
          : max.toString();
      setState(() => error = 'Enter a number from 0 to $label.');
      return;
    }
    Navigator.pop(context, n);
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(widget.rows ? 'Row height' : 'Column width'),
    content: SizedBox(
      width: 300,
      child: draftTextField(
        key: const Key('dimension-size'),
        controller: value,
        autofocus: true,
        keyboardType: const TextInputType.numberWithOptions(decimal: true),
        decoration: InputDecoration(
          labelText: widget.rows ? 'Height (points)' : 'Width (characters)',
          helperText: 'A size of 0 hides the selection.',
          errorText: error,
        ),
        onSubmitted: (_) => apply(),
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('dimension-apply'),
        onPressed: apply,
        child: const Text('Apply'),
      ),
    ],
  );
}

Future<Map<String, dynamic>?> tableCreateDialog(
  BuildContext context,
  String range,
) async {
  final rangeText = TextEditingController(text: range);
  final name = TextEditingController();
  final result = await showDialog<Map<String, dynamic>>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Format as table'),
      content: SizedBox(
        width: 350,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            draftTextField(
              key: const Key('table-range'),
              controller: rangeText,
              autofocus: true,
              decoration: const InputDecoration(
                labelText: 'Range',
                helperText:
                    'Include column headings and at least one data row.',
              ),
            ),
            const SizedBox(height: 16),
            draftTextField(
              key: const Key('table-name'),
              controller: name,
              decoration: const InputDecoration(
                labelText: 'Table name (optional)',
                hintText: 'Sales',
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('table-create-apply'),
          onPressed: () => Navigator.pop(context, {
            'action': 'create',
            'range': rangeText.text.trim().toUpperCase(),
            if (name.text.trim().isNotEmpty) 'name': name.text.trim(),
          }),
          child: const Text('Create table'),
        ),
      ],
    ),
  );
  // Dialog exit animations retain their fields until the next frame.
  await Future<void>.delayed(const Duration(milliseconds: 250));
  rangeText.dispose();
  name.dispose();
  return result;
}

class TableResizeDialog extends StatefulWidget {
  final Map table;
  const TableResizeDialog({super.key, required this.table});
  @override
  State<TableResizeDialog> createState() => _TableResizeDialogState();
}

class _TableResizeDialogState extends State<TableResizeDialog> {
  late final range = TextEditingController(text: widget.table['range']);
  String? error;
  @override
  void dispose() {
    range.dispose();
    super.dispose();
  }

  void apply() {
    final input = range.text.trim().replaceAll(r'$', '').toUpperCase();
    final match = RegExp(
      r'^([A-Z]{1,3}[1-9][0-9]*):([A-Z]{1,3}[1-9][0-9]*)$',
    ).firstMatch(input);
    if (match == null) {
      setState(() => error = 'Enter a range such as A1:D12.');
      return;
    }
    late TableArea a;
    try {
      a = TableArea.parse(input);
    } on FormatException {
      setState(() => error = 'The range exceeds the worksheet boundaries.');
      return;
    }
    final old = TableArea.parse(widget.table['range']);
    final reserved =
        (widget.table['headerRows'] as int? ?? 1) +
        (widget.table['totalsRows'] as int? ?? 0);
    if (a.top != old.top ||
        a.right < old.left ||
        a.left > old.right ||
        a.left > a.right ||
        a.bottom - a.top + 1 <= reserved ||
        a.right >= 16384 ||
        a.bottom >= 1048576) {
      setState(
        () => error =
            'Keep the header row, overlap the current table, and include at least one data row.',
      );
      return;
    }
    Navigator.pop(context, {'action': 'resize', 'range': input});
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text('Resize ${widget.table['name']}'),
    content: SizedBox(
      width: 390,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Current range: ${widget.table['range']}'),
          const SizedBox(height: 16),
          draftTextField(
            key: const Key('table-resize-range'),
            controller: range,
            autofocus: true,
            onSubmitted: (_) => apply(),
            decoration: InputDecoration(
              labelText: 'New table range',
              errorText: error,
              errorMaxLines: 3,
            ),
          ),
          const SizedBox(height: 16),
          const Text(
            'Cells outside a smaller table stay on the worksheet. Existing filters are reapplied to the new range.',
          ),
          if (widget.table['totalsRows'] == 1) ...[
            const SizedBox(height: 12),
            const Text(
              'Include the totals row in the range. If it moves, choose an empty row for its new position.',
            ),
          ],
        ],
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('table-resize-apply'),
        onPressed: apply,
        child: const Text('Resize table'),
      ),
    ],
  );
}

class TableSortDialog extends StatefulWidget {
  final Map table;
  final Map? cells;
  const TableSortDialog({super.key, required this.table, this.cells});
  @override
  State<TableSortDialog> createState() => _TableSortDialogState();
}

class _TableSortDialogState extends State<TableSortDialog> {
  late final List<Map<String, dynamic>> keys = [
    for (final k in (widget.table['sort'] as List? ?? []))
      Map<String, dynamic>.from(k),
    if ((widget.table['sort'] as List? ?? []).isEmpty)
      {'column': 0, 'descending': false},
  ];
  late bool caseSensitive = widget.table['sortCaseSensitive'] == true;
  late final rangeText = TextEditingController(text: widget.table['range']);
  late bool headers = widget.table['headers'] as bool? ?? true;
  String? error;
  bool get rangeMode => widget.cells != null;
  TableArea? get range {
    try {
      final text = rangeText.text.trim().replaceAll(r'$', '').toUpperCase();
      if (text.split(':').length > 2) return null;
      final a = TableArea.parse(text);
      return a.top <= a.bottom && a.left <= a.right ? a : null;
    } on FormatException {
      return null;
    }
  }

  List get columns {
    if (!rangeMode) return widget.table['columns'] as List;
    final a = range;
    if (a == null) return widget.table['columns'] as List;
    return [
      for (var c = a.left; c <= a.right; c++)
        {
          'name': headers
              ? '${columnLabel(c)} — ${displayCell(widget.cells![cellAddress(a.top, c)])}'
              : 'Column ${columnLabel(c)}',
        },
    ];
  }

  void updateRange() => setState(() {
    error = null;
    final count = columns.length;
    for (final key in keys) {
      if ((key['column'] as int) >= count) key['column'] = 0;
    }
  });
  @override
  void dispose() {
    rangeText.dispose();
    super.dispose();
  }

  void apply() {
    final a = range;
    if (rangeMode && (a == null || a.bottom - a.top + 1 <= (headers ? 1 : 0))) {
      setState(() => error = 'Enter a valid range with at least one data row.');
      return;
    }
    Navigator.pop(context, {
      'action': 'sort',
      'keys': keys,
      'caseSensitive': caseSensitive,
      if (rangeMode) ...{
        'range':
            '${cellAddress(a!.top, a.left)}:${cellAddress(a.bottom, a.right)}',
        'headers': headers,
      },
    });
  }

  @override
  Widget build(BuildContext context) {
    final columns = this.columns;
    return AlertDialog(
      title: Text(rangeMode ? 'Sort range' : 'Sort ${widget.table['name']}'),
      content: SizedBox(
        width: 480,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                rangeMode
                    ? 'Rows stay together within this range. Hidden rows stay in place.'
                    : 'Rows stay together. Column headings and totals stay in place.',
              ),
              if (rangeMode) ...[
                const SizedBox(height: 16),
                draftTextField(
                  key: const Key('sort-range'),
                  controller: rangeText,
                  onChanged: (_) => updateRange(),
                  decoration: InputDecoration(
                    labelText: 'Range to sort',
                    errorText: error,
                  ),
                ),
                CheckboxListTile(
                  key: const Key('sort-headers'),
                  contentPadding: EdgeInsets.zero,
                  title: const Text('My data has headers'),
                  value: headers,
                  onChanged: (v) => setState(() => headers = v!),
                ),
              ],
              const SizedBox(height: 16),
              for (var i = 0; i < keys.length; i++)
                Padding(
                  padding: const EdgeInsets.only(bottom: 10),
                  child: Row(
                    children: [
                      SizedBox(
                        width: 62,
                        child: Text(i == 0 ? 'Sort by' : 'Then by'),
                      ),
                      Expanded(
                        child: DropdownButtonFormField<int>(
                          key: ValueKey('sort-column-$i-${keys[i]['column']}'),
                          initialValue: keys[i]['column'],
                          isExpanded: true,
                          decoration: const InputDecoration(
                            isDense: true,
                            border: OutlineInputBorder(),
                          ),
                          items: [
                            for (var c = 0; c < columns.length; c++)
                              DropdownMenuItem(
                                value: c,
                                child: Text(
                                  columns[c]['name'],
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                          ],
                          onChanged: (c) =>
                              setState(() => keys[i]['column'] = c),
                        ),
                      ),
                      const SizedBox(width: 8),
                      SizedBox(
                        width: 168,
                        child: DropdownButtonFormField<bool>(
                          key: ValueKey(
                            'sort-direction-$i-${keys[i]['descending']}',
                          ),
                          isExpanded: true,
                          style: const TextStyle(
                            fontSize: 13,
                            color: Colors.black87,
                          ),
                          initialValue: keys[i]['descending'] == true,
                          decoration: const InputDecoration(
                            isDense: true,
                            border: OutlineInputBorder(),
                          ),
                          items: const [
                            DropdownMenuItem(
                              value: false,
                              child: Text('Ascending'),
                            ),
                            DropdownMenuItem(
                              value: true,
                              child: Text('Descending'),
                            ),
                          ],
                          onChanged: (v) =>
                              setState(() => keys[i]['descending'] = v),
                        ),
                      ),
                      IconButton(
                        tooltip: 'Remove sort level',
                        onPressed: keys.length == 1
                            ? null
                            : () => setState(() => keys.removeAt(i)),
                        icon: const Icon(Icons.close, size: 18),
                      ),
                    ],
                  ),
                ),
              TextButton.icon(
                key: const Key('sort-add-level'),
                onPressed: keys.length >= 64
                    ? null
                    : () => setState(
                        () => keys.add({
                          'column': keys.length % columns.length,
                          'descending': false,
                        }),
                      ),
                icon: const Icon(Icons.add),
                label: const Text('Add level'),
              ),
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Case sensitive'),
                value: caseSensitive,
                onChanged: (v) => setState(() => caseSensitive = v!),
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
          key: const Key('table-sort-apply'),
          onPressed: apply,
          child: const Text('Sort'),
        ),
      ],
    );
  }
}

class TableFilterDialog extends StatefulWidget {
  final bool truncated;
  final Set<String> numericValues;
  final Map table;
  final int column;
  final List<(String, String)> values;
  const TableFilterDialog({
    this.truncated = false,
    this.numericValues = const {},
    super.key,
    required this.table,
    required this.column,
    required this.values,
  });
  @override
  State<TableFilterDialog> createState() => _TableFilterDialogState();
}

bool _sameFilterValue(String a, String b, bool numeric) {
  final x = double.tryParse(a), y = double.tryParse(b);
  return numeric && x != null && y != null && x.isFinite && y.isFinite
      ? x == y
      : a.toLowerCase() == b.toLowerCase();
}

class _TableFilterDialogState extends State<TableFilterDialog> {
  late final Map? rule = widget.table['filters']?['${widget.column}'] as Map?;
  late String mode = rule?['kind'] == 'custom' ? 'custom' : 'values';
  late final Set<String> selected = {
    if (rule?['kind'] == 'values') ...[
      for (final v in rule!['values'])
        widget.values
            .firstWhere(
              (entry) => _sameFilterValue(
                v.toString(),
                entry.$1,
                widget.numericValues.contains(entry.$1),
              ),
              orElse: () => (v.toString(), v.toString()),
            )
            .$1,
      if (rule!['blank'] == true) '',
    ] else
      ...widget.values.map((v) => v.$1),
  };
  late String join = rule?['join'] == 'or' ? 'or' : 'and';
  late final List<String> ops = [
    for (var i = 0; i < 2; i++)
      rule?['kind'] == 'custom' && i < (rule!['conditions'] as List).length
          ? rule!['conditions'][i]['op'] as String
          : 'equal',
  ];
  late final List<TextEditingController> text = [
    for (var i = 0; i < 2; i++)
      TextEditingController(
        text:
            rule?['kind'] == 'custom' &&
                i < (rule!['conditions'] as List).length
            ? rule!['conditions'][i]['value'] as String
            : '',
      ),
  ];
  late bool second =
      rule?['kind'] == 'custom' && (rule!['conditions'] as List).length > 1;
  String search = '';
  @override
  void dispose() {
    for (final t in text) {
      t.dispose();
    }
    super.dispose();
  }

  static const operators = {
    'equal': 'Equals',
    'notEqual': 'Does not equal',
    'lessThan': 'Less than',
    'lessThanOrEqual': 'At most',
    'greaterThan': 'Greater than',
    'greaterThanOrEqual': 'At least',
    'contains': 'Contains',
    'beginsWith': 'Begins with',
    'endsWith': 'Ends with',
  };
  @override
  Widget build(BuildContext context) {
    final values = widget.values
        .where((v) => v.$2.toLowerCase().contains(search.toLowerCase()))
        .toList();
    return AlertDialog(
      title: Text('Filter ${widget.table['columns'][widget.column]['name']}'),
      content: SizedBox(
        width: 400,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            SegmentedButton<String>(
              segments: const [
                ButtonSegment(value: 'values', label: Text('Choose values')),
                ButtonSegment(value: 'custom', label: Text('Conditions')),
              ],
              selected: {mode},
              onSelectionChanged: (v) => setState(() => mode = v.first),
            ),
            const SizedBox(height: 12),
            if (widget.truncated)
              const Text(
                'Showing the first 10,000 unique values. Use Conditions for larger lists.',
              ),
            if (rule?['kind'] == 'unsupported')
              const Text('Choose new criteria to replace this Excel filter.'),
            if (mode == 'values') ...[
              draftTextField(
                key: const Key('filter-search'),
                decoration: const InputDecoration(
                  prefixIcon: Icon(Icons.search),
                  hintText: 'Search values',
                  isDense: true,
                ),
                onChanged: (v) => setState(() => search = v),
              ),
              Row(
                children: [
                  TextButton(
                    onPressed: () => setState(
                      () => selected.addAll(values.map((v) => v.$1)),
                    ),
                    child: const Text('Select all'),
                  ),
                  TextButton(
                    onPressed: () => setState(
                      () => selected.removeAll(values.map((v) => v.$1)),
                    ),
                    child: const Text('Clear selection'),
                  ),
                ],
              ),
              SizedBox(
                height: 230,
                child: ListView.builder(
                  itemCount: values.length,
                  itemBuilder: (_, i) {
                    final v = values[i];
                    return CheckboxListTile(
                      key: ValueKey('filter-value-${v.$1}'),
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      title: Text(
                        v.$2,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      value: selected.contains(v.$1),
                      onChanged: (yes) => setState(() {
                        if (yes!) {
                          selected.add(v.$1);
                        } else {
                          selected.remove(v.$1);
                        }
                      }),
                    );
                  },
                ),
              ),
            ] else ...[
              for (var i = 0; i < (second ? 2 : 1); i++)
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: Row(
                    children: [
                      SizedBox(
                        width: 155,
                        child: DropdownButtonFormField<String>(
                          key: ValueKey('filter-operator-$i'),
                          initialValue: operators.containsKey(ops[i])
                              ? ops[i]
                              : 'equal',
                          isExpanded: true,
                          items: [
                            for (final op in operators.entries)
                              DropdownMenuItem(
                                value: op.key,
                                child: Text(op.value),
                              ),
                          ],
                          onChanged: (v) => setState(() => ops[i] = v!),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: draftTextField(
                          key: ValueKey('filter-condition-$i'),
                          controller: text[i],
                          decoration: const InputDecoration(
                            labelText: 'Value',
                            isDense: true,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Second condition'),
                value: second,
                onChanged: (v) => setState(() => second = v),
              ),
              if (second)
                SegmentedButton<String>(
                  segments: const [
                    ButtonSegment(value: 'and', label: Text('Both conditions')),
                    ButtonSegment(value: 'or', label: Text('Either condition')),
                  ],
                  selected: {join},
                  onSelectionChanged: (v) => setState(() => join = v.first),
                ),
              const SizedBox(height: 8),
              const Text(
                'Equals accepts * for any text, ? for one character, and ~ to escape them.',
                style: TextStyle(fontSize: 12, color: Colors.black54),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          key: const Key('table-filter-clear'),
          onPressed: () => Navigator.pop(context, {
            'action': 'filter',
            'column': widget.column,
            'rule': null,
          }),
          child: const Text('Clear filter'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('table-filter-apply'),
          onPressed: () => Navigator.pop(context, {
            'action': 'filter',
            'column': widget.column,
            'rule': mode == 'values'
                ? {
                    'kind': 'values',
                    'values': selected.where((v) => v.isNotEmpty).toList(),
                    'blank': selected.contains(''),
                  }
                : {
                    'kind': 'custom',
                    'join': join,
                    'conditions': [
                      for (var i = 0; i < (second ? 2 : 1); i++)
                        {'op': ops[i], 'value': text[i].text},
                    ],
                  },
          }),
          child: const Text('Apply filter'),
        ),
      ],
    );
  }
}

Future<String?> worksheetFilterDialog(
  BuildContext context,
  String range,
) async {
  final text = TextEditingController(text: range);
  final result = await showDialog<String>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Filter a range'),
      content: SizedBox(
        width: 350,
        child: draftTextField(
          key: const Key('worksheet-filter-range'),
          controller: text,
          autofocus: true,
          decoration: const InputDecoration(
            labelText: 'Range',
            helperText: 'Include column headings and the data to filter.',
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('worksheet-filter-enable'),
          onPressed: () => Navigator.pop(context, text.text.trim()),
          child: const Text('Enable filters'),
        ),
      ],
    ),
  );
  await Future<void>.delayed(const Duration(milliseconds: 300));
  text.dispose();
  return result;
}
