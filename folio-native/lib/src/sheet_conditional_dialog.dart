import 'sheet_data_bar.dart';
import 'sheet_icon_set.dart';
import 'sheet_icon_set_editor.dart';
import 'package:flutter/material.dart';
import 'table_controls.dart';

const conditionalTypes = {
  'cellIs': 'Cell value',
  'expression': 'Formula is true',
  'containsText': 'Text contains',
  'notContainsText': 'Text does not contain',
  'beginsWith': 'Text begins with',
  'endsWith': 'Text ends with',
  'containsBlanks': 'Blanks',
  'notContainsBlanks': 'Nonblank cells',
  'containsErrors': 'Errors',
  'notContainsErrors': 'No errors',
  'duplicateValues': 'Duplicate values',
  'uniqueValues': 'Unique values',
  'top10': 'Top / bottom values',
  'aboveAverage': 'Above / below average',
  'colorScale': 'Color scale',
  'dataBar': 'Data bars',
  'iconSet': 'Icon sets',
  'timePeriod': 'A date occurring',
};
const conditionalPeriods = {
  'yesterday': 'Yesterday',
  'today': 'Today',
  'tomorrow': 'Tomorrow',
  'last7Days': 'In the last 7 days',
  'lastWeek': 'Last week',
  'thisWeek': 'This week',
  'nextWeek': 'Next week',
  'lastMonth': 'Last month',
  'thisMonth': 'This month',
  'nextMonth': 'Next month',
};
const conditionalOperators = {
  'greaterThan': 'greater than',
  'greaterThanOrEqual': 'greater than or equal to',
  'lessThan': 'less than',
  'lessThanOrEqual': 'less than or equal to',
  'equal': 'equal to',
  'notEqual': 'not equal to',
  'between': 'between',
  'notBetween': 'not between',
};
Color? conditionalColor(dynamic hex) =>
    hex is String && RegExp(r'^#[0-9a-fA-F]{6}$').hasMatch(hex)
    ? Color(int.parse(hex.substring(1), radix: 16) | 0xff000000)
    : null;
String conditionalSummary(Map r) {
  final kind = r['type'];
  final formulas = (r['formulas'] as List? ?? []).join(' and ');
  return switch (kind) {
    'cellIs' =>
      'Cell value ${conditionalOperators[r['operator']] ?? r['operator']} $formulas',
    'expression' => 'Formula: $formulas',
    'timePeriod' =>
      'Date: ${conditionalPeriods[r['timePeriod']] ?? r['timePeriod']}',
    'iconSet' =>
      'Icons: ${r['iconSet']?['icons'] is List ? 'Custom' : sheetIconSets[r['iconSet']?['set']] ?? r['iconSet']?['set']}',
    'top10' =>
      '${r['bottom'] == true ? 'Bottom' : 'Top'} ${r['rank']}${r['percent'] == true ? '%' : ' values'}',
    'aboveAverage' =>
      '${r['aboveAverage'] == false ? 'Below' : 'Above'} average',
    'containsText' ||
    'notContainsText' ||
    'beginsWith' ||
    'endsWith' => '${conditionalTypes[kind]} “${r['text']}”',
    _ => conditionalTypes[kind] ?? 'Excel rule: $kind',
  };
}

class SheetConditionalDialog extends StatefulWidget {
  final List rules;
  final String range;
  final bool extended;
  final List warnings;
  final Future<void> Function(Map<String, dynamic>)? onValidate;
  const SheetConditionalDialog({
    super.key,
    required this.rules,
    required this.range,
    this.extended = false,
    this.warnings = const [],
    this.onValidate,
  });
  @override
  State<SheetConditionalDialog> createState() => _SheetConditionalDialogState();
}

class _SheetConditionalDialogState extends State<SheetConditionalDialog> {
  late final List<Map<String, dynamic>> rules = widget.rules
      .map((r) => Map<String, dynamic>.from(r as Map))
      .toList();
  late final List<int?> origins = List<int?>.generate(
    widget.rules.length,
    (i) => i,
  );
  bool changed = false;
  int? selected;
  String? error;
  bool applying = false;
  Future<void> apply() async {
    final result = <String, dynamic>{
      'action': 'replace',
      'rules': [
        for (var i = 0; i < rules.length; i++)
          {'origin': origins[i], 'rule': rules[i]},
      ],
    };
    setState(() {
      applying = true;
      error = null;
    });
    try {
      await widget.onValidate?.call(result);
      if (mounted) Navigator.pop(context, result);
    } catch (e) {
      if (mounted) {
        setState(() {
          applying = false;
          error = e.toString();
        });
      }
    }
  }

  Future<void> edit([int? index]) async {
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => ConditionalRuleEditor(
        range: widget.range,
        rule: index == null ? null : rules[index],
      ),
    );
    if (result == null || !mounted) return;
    setState(() {
      if (index == null) {
        rules.insert(0, result);
        selected = 0;
        origins.insert(0, null);
        changed = true;
      } else {
        rules[index] = result;
        changed = true;
      }
    });
  }

  void move(int direction) {
    final i = selected!, to = i + direction;
    setState(() {
      rules.insert(to, rules.removeAt(i));
      selected = to;
      origins.insert(to, origins.removeAt(i));
      changed = true;
    });
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Conditional formatting'),
    content: SizedBox(
      width: 720,
      height: 440,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text('Rules for this worksheet · highest priority first'),
          const SizedBox(height: 8),
          if (widget.warnings.isNotEmpty)
            Text(
              widget.warnings.join('\n'),
              maxLines: 3,
              style: const TextStyle(color: Colors.deepOrange),
            ),
          if (widget.extended)
            const Text(
              'Extended Excel rules are preserved. Rule editing is not available for this worksheet yet.',
              style: TextStyle(color: Colors.deepOrange),
            ),
          Wrap(
            spacing: 8,
            children: [
              TextButton.icon(
                key: const Key('conditional-new'),
                onPressed: widget.extended || applying ? null : () => edit(),
                icon: const Icon(Icons.add),
                label: const Text('New rule'),
              ),
              TextButton(
                onPressed:
                    selected == null ||
                        applying ||
                        widget.extended ||
                        rules[selected!]['unsupported'] == true
                    ? null
                    : () => edit(selected),
                child: const Text('Edit rule'),
              ),
              TextButton(
                onPressed: selected == null || applying || widget.extended
                    ? null
                    : () => setState(() {
                        final i = selected!;
                        rules.removeAt(i);
                        origins.removeAt(i);
                        changed = true;
                        selected = null;
                      }),
                child: const Text('Delete rule'),
              ),
              IconButton(
                tooltip: 'Move rule up',
                onPressed: selected == null || selected == 0 || widget.extended
                    ? null
                    : () => move(-1),
                icon: const Icon(Icons.arrow_upward),
              ),
              IconButton(
                tooltip: 'Move rule down',
                onPressed:
                    selected == null ||
                        selected == rules.length - 1 ||
                        applying ||
                        widget.extended
                    ? null
                    : () => move(1),
                icon: const Icon(Icons.arrow_downward),
              ),
            ],
          ),
          const Divider(),
          Expanded(
            child: rules.isEmpty
                ? const Center(
                    child: Text(
                      'No rules yet. Select New rule to highlight values or use a formula.',
                    ),
                  )
                : ListView.builder(
                    itemCount: rules.length,
                    itemBuilder: (context, i) {
                      final r = rules[i], style = r['style'] as Map? ?? {};
                      return ListTile(
                        key: Key('conditional-rule-$i'),
                        selected: selected == i,
                        selectedTileColor: const Color(0x14107c41),
                        onTap: applying
                            ? null
                            : () => setState(() => selected = i),
                        leading: Container(
                          width: 46,
                          height: 32,
                          alignment: Alignment.center,
                          decoration: BoxDecoration(
                            color:
                                conditionalColor(style['fill']) ?? Colors.white,
                            border: Border.all(color: Colors.black12),
                          ),
                          child: Text(
                            'Aa',
                            style: TextStyle(
                              color: conditionalColor(style['color']),
                              fontWeight: style['bold'] == true
                                  ? FontWeight.bold
                                  : null,
                              fontStyle: style['italic'] == true
                                  ? FontStyle.italic
                                  : null,
                            ),
                          ),
                        ),
                        title: Text(conditionalSummary(r)),
                        subtitle: Text(
                          'Applies to ${r['sqref']}${r['stopIfTrue'] == true ? ' · Stop if true' : ''}${r['unsupported'] == true ? '\nPreserved Excel rule · not displayed yet' : ''}',
                        ),
                        trailing: Text('${i + 1}'),
                      );
                    },
                  ),
          ),
          const Divider(),
          const Text(
            'Higher rules win when formats conflict. Stop if true prevents lower rules from applying.',
            style: TextStyle(fontSize: 12),
          ),
        ],
      ),
    ),
    actions: [
      if (error != null)
        SizedBox(
          width: 480,
          child: Text(
            error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        ),
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('conditional-apply'),
        onPressed: !changed || applying ? null : apply,
        child: const Text('Apply'),
      ),
    ],
  );
}

class ConditionalRuleEditor extends StatefulWidget {
  final String range;
  final Map? rule;
  const ConditionalRuleEditor({super.key, required this.range, this.rule});
  @override
  State<ConditionalRuleEditor> createState() => _ConditionalRuleEditorState();
}

class _ConditionalRuleEditorState extends State<ConditionalRuleEditor> {
  late final range = TextEditingController(
    text: widget.rule?['sqref'] as String? ?? widget.range,
  );
  late final first = TextEditingController(
    text: (widget.rule?['formulas'] as List?)?.firstOrNull?.toString() ?? '0',
  );
  late final second = TextEditingController(
    text:
        (widget.rule?['formulas'] as List?)?.elementAtOrNull(1)?.toString() ??
        '100',
  );
  late final text = TextEditingController(
    text: widget.rule?['text'] as String? ?? '',
  );
  late final rank = TextEditingController(
    text: '${widget.rule?['rank'] ?? 10}',
  );
  late final fill = TextEditingController(
    text:
        widget.rule?['style']?['fill'] as String? ??
        (widget.rule == null ? '#FFC7CE' : ''),
  );
  late final color = TextEditingController(
    text:
        widget.rule?['style']?['color'] as String? ??
        (widget.rule == null ? '#9C0006' : ''),
  );
  late final number = TextEditingController(
    text: widget.rule?['style']?['numFmt'] as String? ?? '',
  );
  late String type = widget.rule?['type'] as String? ?? 'cellIs',
      operator = widget.rule?['operator'] as String? ?? 'greaterThan';
  late bool stop = widget.rule?['stopIfTrue'] == true,
      percent = widget.rule?['percent'] == true,
      bottom = widget.rule?['bottom'] == true,
      above = widget.rule?['aboveAverage'] != false,
      equal = widget.rule?['equalAverage'] == true;
  late String period = widget.rule?['timePeriod'] as String? ?? 'today';
  late int stdDev = (widget.rule?['stdDev'] as num?)?.toInt() ?? 0;
  late bool? bold = widget.rule?['style']?['bold'] as bool?,
      italic = widget.rule?['style']?['italic'] as bool?;
  late final List<Map<String, dynamic>> scale =
      (widget.rule?['scale'] as List? ??
              [
                {'type': 'min', 'value': '', 'color': '#F8696B'},
                {'type': 'max', 'value': '', 'color': '#63BE7B'},
              ])
          .map((p) => Map<String, dynamic>.from(p as Map))
          .toList();
  late Map<String, dynamic> bar = Map<String, dynamic>.from(
    widget.rule?['dataBar'] as Map? ?? defaultDataBar(),
  );
  late Map<String, dynamic> icons = Map<String, dynamic>.from(
    widget.rule?['iconSet'] as Map? ?? defaultIconSet(),
  );
  String? error;
  Widget input(
    TextEditingController controller,
    String label, {
    String? hint,
    Key? key,
  }) => Padding(
    padding: const EdgeInsets.only(bottom: 12),
    child: TextField(
      key: key,
      controller: controller,
      decoration: InputDecoration(
        labelText: label,
        hintText: hint,
        border: const OutlineInputBorder(),
      ),
      onChanged: (_) => setState(() {}),
    ),
  );
  void apply() {
    try {
      final refs = range.text.trim().split(RegExp(r'\s+'));
      for (final ref in refs) {
        TableArea.parse(ref.replaceAll(r'$', '').toUpperCase());
      }
      if (refs.isEmpty || refs.any((s) => s.isEmpty)) {
        throw const FormatException('Enter a cell range.');
      }
      if (type == 'expression' && first.text.trim().isEmpty) {
        throw const FormatException('Enter a formula.');
      }
      final style = <String, dynamic>{
        ...Map<String, dynamic>.from(widget.rule?['style'] as Map? ?? {}),
      };
      for (final entry in {
        'fill': fill.text.trim(),
        'color': color.text.trim(),
        'numFmt': number.text.trim(),
      }.entries) {
        if (entry.value.isEmpty) {
          style.remove(entry.key);
        } else {
          style[entry.key] = entry.value;
        }
      }
      for (final entry in {'bold': bold, 'italic': italic}.entries) {
        if (entry.value == null) {
          style.remove(entry.key);
        } else {
          style[entry.key] = entry.value;
        }
      }
      for (final key in ['fill', 'color']) {
        if (style[key] != null &&
            style[key] != 'automatic' &&
            conditionalColor(style[key]) == null) {
          throw const FormatException(
            'Enter colors as #RRGGBB, or leave them blank.',
          );
        }
      }
      final result = <String, dynamic>{
        'type': type,
        'sqref': refs.join(' '),
        'style': ['colorScale', 'dataBar', 'iconSet'].contains(type)
            ? <String, dynamic>{}
            : style,
        'stopIfTrue':
            !['colorScale', 'dataBar', 'iconSet'].contains(type) && stop,
      };
      if (type == 'cellIs') {
        result['operator'] = operator;
        result['formulas'] = [
          first.text.trim(),
          if (operator == 'between' || operator == 'notBetween')
            second.text.trim(),
        ];
      }
      if (type == 'expression') result['formulas'] = [first.text.trim()];
      if (type == 'timePeriod') result['timePeriod'] = period;
      if ([
        'containsText',
        'notContainsText',
        'beginsWith',
        'endsWith',
      ].contains(type)) {
        result['text'] = text.text;
      }
      if (type == 'top10') {
        final n = int.tryParse(rank.text);
        if (n == null || n < 1 || n > (percent ? 100 : 1000)) {
          throw const FormatException('Enter a valid rank or percentage.');
        }
        result.addAll({'rank': n, 'percent': percent, 'bottom': bottom});
      }
      if (type == 'aboveAverage') {
        result.addAll({
          'aboveAverage': above,
          'equalAverage': equal,
          'stdDev': stdDev,
        });
      }
      if (type == 'colorScale') {
        for (final p in scale) {
          if (conditionalColor(p['color']) == null) {
            throw const FormatException('Enter each scale color as #RRGGBB.');
          }
          final t = p['type'];
          if (['num', 'percent', 'percentile'].contains(t)) {
            final n = double.tryParse(p['value'] ?? '');
            if (n == null || !n.isFinite || t != 'num' && (n < 0 || n > 100)) {
              throw const FormatException('Enter valid scale thresholds.');
            }
          }
        }
        result['scale'] = scale
            .map((p) => Map<String, dynamic>.from(p))
            .toList();
      }
      if (type == 'dataBar') result['dataBar'] = bar;
      if (type == 'iconSet') result['iconSet'] = icons;
      Navigator.pop(context, result);
    } catch (e) {
      setState(
        () => error = e.toString().replaceFirst('FormatException: ', ''),
      );
    }
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(
      widget.rule == null ? 'New formatting rule' : 'Edit formatting rule',
    ),
    content: SizedBox(
      width: 600,
      height: 530,
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            input(
              range,
              'Applies to',
              hint: 'A1:A20 C1:C20',
              key: const Key('conditional-range'),
            ),
            DropdownButtonFormField<String>(
              initialValue: type,
              decoration: const InputDecoration(
                labelText: 'Rule type',
                border: OutlineInputBorder(),
              ),
              items: [
                for (final e in conditionalTypes.entries)
                  DropdownMenuItem(value: e.key, child: Text(e.value)),
              ],
              onChanged: (v) => setState(() => type = v!),
            ),
            const SizedBox(height: 16),
            if (type == 'cellIs') ...[
              DropdownButtonFormField<String>(
                initialValue: operator,
                decoration: const InputDecoration(
                  labelText: 'Cell value is',
                  border: OutlineInputBorder(),
                ),
                items: [
                  for (final e in conditionalOperators.entries)
                    DropdownMenuItem(value: e.key, child: Text(e.value)),
                ],
                onChanged: (v) => setState(() => operator = v!),
              ),
              const SizedBox(height: 12),
              input(
                first,
                'Value or formula',
                key: const Key('conditional-first'),
              ),
              if (operator == 'between' || operator == 'notBetween')
                input(second, 'Second value or formula'),
            ],
            if (type == 'expression') ...[
              input(
                first,
                'Formula',
                hint: r'=$B2>100',
                key: const Key('conditional-first'),
              ),
              const Padding(
                padding: EdgeInsets.only(bottom: 12),
                child: Text(
                  'Write the formula for the first cell in Applies to. Relative references adjust for every cell.',
                ),
              ),
            ],
            if ([
              'containsText',
              'notContainsText',
              'beginsWith',
              'endsWith',
            ].contains(type))
              input(text, 'Text'),
            if (type == 'timePeriod') ...[
              DropdownButtonFormField<String>(
                key: const Key('conditional-period'),
                initialValue: period,
                isExpanded: true,
                decoration: const InputDecoration(
                  labelText: 'Date occurs',
                  border: OutlineInputBorder(),
                ),
                items: [
                  for (final e in conditionalPeriods.entries)
                    DropdownMenuItem(value: e.key, child: Text(e.value)),
                ],
                onChanged: (v) => setState(() => period = v!),
              ),
              const Padding(
                padding: EdgeInsets.symmetric(vertical: 12),
                child: Text(
                  'Uses your local date. Weeks run Sunday–Saturday; the last 7 days includes today.',
                ),
              ),
            ],
            if (type == 'top10') ...[
              input(rank, percent ? 'Percentage' : 'Number of values'),
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Bottom values'),
                value: bottom,
                onChanged: (v) => setState(() => bottom = v!),
              ),
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Percentage of values'),
                value: percent,
                onChanged: (v) => setState(() => percent = v!),
              ),
            ],
            if (type == 'aboveAverage') ...[
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Above average'),
                value: above,
                onChanged: (v) => setState(() => above = v!),
              ),
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Include values equal to the threshold'),
                value: equal,
                onChanged: (v) => setState(() => equal = v!),
              ),
              DropdownButtonFormField<int>(
                initialValue: stdDev,
                decoration: const InputDecoration(
                  labelText: 'Standard deviations',
                ),
                items: [
                  for (var i = 0; i <= 3; i++)
                    DropdownMenuItem(value: i, child: Text('$i')),
                ],
                onChanged: (v) => setState(() => stdDev = v!),
              ),
            ],
            if (type == 'colorScale') ...[
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Three-color scale'),
                value: scale.length == 3,
                onChanged: (v) => setState(() {
                  if (v == true) {
                    scale.insert(1, {
                      'type': 'percentile',
                      'value': '50',
                      'color': '#FFEB84',
                    });
                  } else {
                    scale.removeAt(1);
                  }
                }),
              ),
              for (var i = 0; i < scale.length; i++)
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: Row(
                    children: [
                      Expanded(
                        child: DropdownButtonFormField<String>(
                          key: ValueKey('scale-type-$i-${scale.length}'),
                          initialValue: scale[i]['type'],
                          decoration: InputDecoration(
                            labelText: i == 0
                                ? 'Minimum'
                                : i == scale.length - 1
                                ? 'Maximum'
                                : 'Midpoint',
                          ),
                          items: [
                            for (final t in [
                              if (i == 0) 'min',
                              if (i == scale.length - 1) 'max',
                              'num',
                              'percent',
                              'percentile',
                              'formula',
                            ])
                              DropdownMenuItem(
                                value: t,
                                child: Text(
                                  {
                                    'min': 'Lowest',
                                    'max': 'Highest',
                                    'num': 'Number',
                                    'percent': 'Percent',
                                    'percentile': 'Percentile',
                                    'formula': 'Formula',
                                  }[t]!,
                                ),
                              ),
                          ],
                          onChanged: (v) =>
                              setState(() => scale[i]['type'] = v),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextFormField(
                          key: ValueKey('scale-value-$i-${scale.length}'),
                          initialValue: scale[i]['value'],
                          enabled: !['min', 'max'].contains(scale[i]['type']),
                          decoration: const InputDecoration(labelText: 'Value'),
                          onChanged: (v) => scale[i]['value'] = v,
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextFormField(
                          key: ValueKey('scale-color-$i-${scale.length}'),
                          initialValue: scale[i]['color'],
                          decoration: const InputDecoration(labelText: 'Color'),
                          onChanged: (v) => scale[i]['color'] = v,
                        ),
                      ),
                    ],
                  ),
                ),
            ] else if (type == 'dataBar') ...[
              DataBarEditor(
                value: bar,
                onChanged: (v) => setState(() => bar = v),
              ),
            ] else if (type == 'iconSet') ...[
              IconSetEditor(
                value: icons,
                onChanged: (v) => setState(() => icons = v),
              ),
            ] else ...[
              const Divider(),
              const Text(
                'Format · leave a field blank to keep the existing format',
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  Expanded(
                    child: input(
                      fill,
                      'Fill color',
                      key: const Key('conditional-fill'),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(child: input(color, 'Text color')),
                ],
              ),
              Row(
                children: [
                  Expanded(
                    child: CheckboxListTile(
                      contentPadding: EdgeInsets.zero,
                      tristate: true,
                      title: const Text('Bold'),
                      value: bold,
                      onChanged: (v) => setState(() => bold = v),
                    ),
                  ),
                  Expanded(
                    child: CheckboxListTile(
                      contentPadding: EdgeInsets.zero,
                      tristate: true,
                      title: const Text('Italic'),
                      value: italic,
                      onChanged: (v) => setState(() => italic = v),
                    ),
                  ),
                ],
              ),
              input(number, 'Number format', hint: '0.00%'),
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Stop if true'),
                subtitle: const Text(
                  'Do not apply lower-priority rules when this rule matches.',
                ),
                value: stop,
                onChanged: (v) => setState(() => stop = v!),
              ),
            ],
            if (error != null)
              Text(
                error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
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
        key: const Key('conditional-rule-done'),
        onPressed: apply,
        child: const Text('Done'),
      ),
    ],
  );
  @override
  void dispose() {
    for (final c in [range, first, second, text, rank, fill, color, number]) {
      c.dispose();
    }
    super.dispose();
  }
}
