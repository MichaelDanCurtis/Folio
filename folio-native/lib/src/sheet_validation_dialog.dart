import 'package:flutter/material.dart';
import 'draft_text_field.dart';

/// A rule is anchored at the first cell in the selected range. The engine
/// translates existing relative formulas to that anchor before this opens.
class SheetValidationDialog extends StatefulWidget {
  final String range;
  final Map? rule;
  const SheetValidationDialog({super.key, required this.range, this.rule});
  @override
  State<SheetValidationDialog> createState() => _SheetValidationDialogState();
}

class _SheetValidationDialogState extends State<SheetValidationDialog> {
  late final rule = <String, dynamic>{
    'type': 'none',
    'operator': 'between',
    'allowBlank': true,
    'showDropDown': false,
    'showInputMessage': true,
    'showErrorMessage': true,
    'errorStyle': 'stop',
    ...?widget.rule,
  };
  late final fields = <String, TextEditingController>{
    for (final key in [
      'formula1',
      'formula2',
      'promptTitle',
      'prompt',
      'errorTitle',
      'error',
    ])
      key: TextEditingController(text: displayFormula(key)),
  };
  String? error;
  String displayFormula(String key) {
    final text = rule[key]?.toString() ?? '';
    if (key == 'formula1' && rule['type'] == 'list') {
      return text.startsWith('"') && text.endsWith('"')
          ? text.substring(1, text.length - 1).replaceAll('""', '"')
          : text.isEmpty
          ? ''
          : '=$text';
    }
    return text;
  }

  Widget choice(String key, String label, Map<String, String> options) =>
      DropdownButtonFormField<String>(
        key: ValueKey('validation-$key-${rule[key]}'),
        initialValue: options.containsKey(rule[key])
            ? rule[key] as String
            : options.keys.first,
        decoration: InputDecoration(labelText: label),
        isExpanded: true,
        items: [
          for (final e in options.entries)
            DropdownMenuItem(value: e.key, child: Text(e.value)),
        ],
        onChanged: (v) => setState(() {
          rule[key] = v;
          error = null;
        }),
      );
  Widget toggle(String key, String label, {bool inverse = false}) =>
      CheckboxListTile(
        key: Key('validation-$key'),
        dense: true,
        contentPadding: EdgeInsets.zero,
        title: Text(label),
        value: inverse ? rule[key] != true : rule[key] == true,
        onChanged: (v) =>
            setState(() => rule[key] = inverse ? v != true : v == true),
      );
  Widget field(
    String key,
    String label, {
    String? hint,
    int? max,
    int lines = 1,
  }) => Padding(
    padding: const EdgeInsets.only(top: 14),
    child: draftTextField(
      key: Key('validation-$key'),
      controller: fields[key],
      maxLength: max,
      maxLines: lines,
      decoration: InputDecoration(labelText: label, helperText: hint),
    ),
  );
  String formula(String key) {
    var value = fields[key]!.text.trim();
    if (value.startsWith('=')) return value.substring(1);
    if (rule['type'] == 'list' && key == 'formula1') {
      return '"${value.replaceAll('"', '""')}"';
    }
    if (rule['type'] == 'date') {
      final match = RegExp(r'^(\d{4})-(\d{2})-(\d{2})$').firstMatch(value);
      if (match != null) {
        final y = int.parse(match[1]!),
            m = int.parse(match[2]!),
            d = int.parse(match[3]!);
        if (y < 1900 || y > 9999) {
          throw const FormatException('Use a date from 1900 through 9999.');
        }
        final date = DateTime(y, m, d);
        if (!(y == 1900 && m == 2 && d == 29) &&
            (date.year != y || date.month != m || date.day != d)) {
          throw const FormatException(
            'Enter a valid date in YYYY-MM-DD format.',
          );
        }
        return 'DATE($y,$m,$d)';
      }
    }
    if (rule['type'] == 'time') {
      final match = RegExp(
        r'^(\d{1,2}):(\d{2})(?::(\d{2}))?$',
      ).firstMatch(value);
      if (match != null) {
        final h = int.parse(match[1]!),
            m = int.parse(match[2]!),
            s = int.parse(match[3] ?? '0');
        if (h > 23 || m > 59 || s > 59) {
          throw const FormatException('Enter a time from 00:00 to 23:59:59.');
        }
        return 'TIME($h,$m,$s)';
      }
    }
    return value;
  }

  void apply() {
    try {
      final type = rule['type'];
      final result = {...rule};
      for (final key in fields.keys) {
        result[key] = fields[key]!.text;
      }
      if (type != 'none') {
        if (fields['formula1']!.text.trim().isEmpty) {
          throw const FormatException('Enter a value, source, or formula.');
        }
        result['formula1'] = formula('formula1');
        if (!['list', 'custom'].contains(type) &&
            ['between', 'notBetween'].contains(rule['operator'])) {
          if (fields['formula2']!.text.trim().isEmpty) {
            throw const FormatException('Enter both boundaries.');
          }
          result['formula2'] = formula('formula2');
        } else {
          result.remove('formula2');
        }
      } else {
        result.remove('formula1');
        result.remove('formula2');
      }
      Navigator.pop(context, {'action': 'set', 'rule': result});
    } on FormatException catch (e) {
      setState(() => error = e.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    final type = rule['type'];
    final comparison = !['none', 'list', 'custom'].contains(type);
    final between = ['between', 'notBetween'].contains(rule['operator']);
    return DefaultTabController(
      length: 3,
      child: AlertDialog(
        title: Text('Data validation · ${widget.range}'),
        content: SizedBox(
          width: 540,
          height: 410,
          child: Column(
            children: [
              const TabBar(
                tabs: [
                  Tab(text: 'Settings'),
                  Tab(text: 'Input message'),
                  Tab(text: 'Error alert'),
                ],
              ),
              Expanded(
                child: TabBarView(
                  children: [
                    ListView(
                      padding: const EdgeInsets.only(top: 12),
                      children: [
                        choice('type', 'Allow', const {
                          'none': 'Any value',
                          'whole': 'Whole number',
                          'decimal': 'Decimal',
                          'list': 'List',
                          'date': 'Date',
                          'time': 'Time',
                          'textLength': 'Text length',
                          'custom': 'Custom formula',
                        }),
                        if (comparison)
                          Padding(
                            padding: const EdgeInsets.only(top: 12),
                            child: choice('operator', 'Data', const {
                              'between': 'between',
                              'notBetween': 'not between',
                              'equal': 'equal to',
                              'notEqual': 'not equal to',
                              'greaterThan': 'greater than',
                              'lessThan': 'less than',
                              'greaterThanOrEqual': 'greater than or equal to',
                              'lessThanOrEqual': 'less than or equal to',
                            }),
                          ),
                        if (type != 'none')
                          field(
                            'formula1',
                            type == 'list'
                                ? 'Source'
                                : type == 'custom'
                                ? 'Formula'
                                : between
                                ? 'Minimum'
                                : 'Value',
                            hint: type == 'list'
                                ? 'Comma-separated values, or =Sheet1!\$A\$1:\$A\$10'
                                : type == 'date'
                                ? 'YYYY-MM-DD, serial number, or =formula'
                                : type == 'time'
                                ? 'HH:mm, fraction of a day, or =formula'
                                : type == 'custom'
                                ? 'A formula that returns TRUE for valid entries.'
                                : 'Number or =formula',
                          ),
                        if (comparison && between) field('formula2', 'Maximum'),
                        if (type != 'none')
                          toggle('allowBlank', 'Ignore blank entries'),
                        if (type == 'list')
                          toggle(
                            'showDropDown',
                            'Show dropdown in cell',
                            inverse: true,
                          ),
                        const Padding(
                          padding: EdgeInsets.only(top: 10),
                          child: Text(
                            'Applies to typed entries. Existing values are kept.',
                            style: TextStyle(fontSize: 12),
                          ),
                        ),
                      ],
                    ),
                    ListView(
                      padding: const EdgeInsets.only(top: 12),
                      children: [
                        toggle(
                          'showInputMessage',
                          'Show message when cell is selected',
                        ),
                        field('promptTitle', 'Title', max: 32),
                        field('prompt', 'Message', max: 255, lines: 3),
                      ],
                    ),
                    ListView(
                      padding: const EdgeInsets.only(top: 12),
                      children: [
                        toggle(
                          'showErrorMessage',
                          'Show alert for invalid entries',
                        ),
                        choice('errorStyle', 'Style', const {
                          'stop': 'Stop — require a valid entry',
                          'warning': 'Warning — allow override',
                          'information': 'Information — allow override',
                        }),
                        field('errorTitle', 'Title', max: 32),
                        field('error', 'Message', max: 255, lines: 3),
                      ],
                    ),
                  ],
                ),
              ),
              if (error != null)
                Text(error!, style: const TextStyle(color: Colors.red)),
            ],
          ),
        ),
        actions: [
          TextButton(
            key: const Key('validation-clear'),
            onPressed: () => Navigator.pop(context, {'action': 'clear'}),
            child: const Text('Clear validation'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('validation-apply'),
            onPressed: apply,
            child: const Text('Apply'),
          ),
        ],
      ),
    );
  }

  @override
  void dispose() {
    for (final field in fields.values) {
      field.dispose();
    }
    super.dispose();
  }
}

/// A searchable list keeps long range-backed validations usable without
/// constructing thousands of menu widgets at once.
class ValidationChoicesDialog extends StatefulWidget {
  final List<Map> choices;
  const ValidationChoicesDialog({super.key, required this.choices});
  @override
  State<ValidationChoicesDialog> createState() =>
      _ValidationChoicesDialogState();
}

class _ValidationChoicesDialogState extends State<ValidationChoicesDialog> {
  String query = '';
  @override
  Widget build(BuildContext context) {
    final matches = widget.choices
        .where((c) => '${c['label']}'.toLowerCase().contains(query))
        .toList();
    return AlertDialog(
      title: const Text('Choose a value'),
      content: SizedBox(
        width: 380,
        height: 340,
        child: Column(
          children: [
            draftTextField(
              key: const Key('validation-search'),
              autofocus: true,
              decoration: const InputDecoration(labelText: 'Search values'),
              onChanged: (v) => setState(() => query = v.toLowerCase()),
            ),
            Expanded(
              child: matches.isEmpty
                  ? const Center(child: Text('No matching values'))
                  : ListView.builder(
                      itemCount: matches.length,
                      itemBuilder: (context, i) => ListTile(
                        key: ValueKey('validation-choice-$i'),
                        title: Text(
                          '${matches[i]['label']}'.isEmpty
                              ? '(blank)'
                              : '${matches[i]['label']}',
                        ),
                        onTap: () => Navigator.pop(context, matches[i]),
                      ),
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
      ],
    );
  }
}
