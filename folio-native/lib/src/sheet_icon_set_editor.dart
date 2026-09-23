import 'package:flutter/material.dart';
import 'sheet_icon_set.dart';

class IconSetEditor extends StatelessWidget {
  final Map<String, dynamic> value;
  final ValueChanged<Map<String, dynamic>> onChanged;
  const IconSetEditor({
    super.key,
    required this.value,
    required this.onChanged,
  });
  List get points => value['thresholds'] as List;
  Map iconAt(int category) {
    final custom = value['icons'] as List?;
    return custom == null
        ? {
            'set': value['set'],
            'index': value['reverse'] == true
                ? points.length - 1 - category
                : category,
          }
        : custom[category] as Map;
  }

  void point(int i, String key, dynamic v) => onChanged({
    ...value,
    'thresholds': [
      for (var j = 0; j < points.length; j++)
        {...points[j] as Map, if (i == j) key: v},
    ],
  });
  Future<void> choose(BuildContext context, int category) async {
    final selected = await showDialog<Map>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Choose an icon'),
        content: SizedBox(
          width: 440,
          height: 440,
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                TextButton.icon(
                  onPressed: () =>
                      Navigator.pop(context, {'set': 'NoIcons', 'index': 0}),
                  icon: const Icon(Icons.block),
                  label: const Text('No cell icon'),
                ),
                for (final entry in sheetIconSets.entries)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    child: Row(
                      children: [
                        Expanded(child: Text(entry.value)),
                        for (var i = sheetIconCount(entry.key) - 1; i >= 0; i--)
                          IconButton(
                            key: Key('choose-icon-${entry.key}-$i'),
                            tooltip: sheetIconLabel(entry.key, i),
                            onPressed: () => Navigator.pop(context, {
                              'set': entry.key,
                              'index': i,
                            }),
                            icon: SheetIconGlyph(set: entry.key, index: i),
                          ),
                      ],
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
        ],
      ),
    );
    if (selected != null) {
      onChanged({
        ...value,
        'reverse': false,
        'icons': [
          for (var i = 0; i < points.length; i++)
            Map<String, dynamic>.from(i == category ? selected : iconAt(i)),
        ],
      });
    }
  }

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      DropdownButtonFormField<String>(
        key: ValueKey('icon-set-${value['set']}'),
        initialValue: value['set'] as String,
        isExpanded: true,
        decoration: const InputDecoration(
          labelText: 'Icon style',
          border: OutlineInputBorder(),
        ),
        items: [
          if (value['set'] == 'NoIcons')
            const DropdownMenuItem(
              value: 'NoIcons',
              child: Text('Custom icons'),
            ),
          for (final e in sheetIconSets.entries)
            DropdownMenuItem(
              value: e.key,
              child: Row(
                children: [
                  Expanded(child: Text(e.value)),
                  for (var i = sheetIconCount(e.key) - 1; i >= 0; i--)
                    Padding(
                      padding: const EdgeInsets.only(left: 4),
                      child: SheetIconGlyph(set: e.key, index: i),
                    ),
                ],
              ),
            ),
        ],
        onChanged: (set) {
          if (set == null || set == 'NoIcons') return;
          final next = defaultIconSet(set)
            ..['showValue'] = value['showValue'] != false;
          if (sheetIconCount(set) == points.length) {
            next['thresholds'] = [
              for (final p in points) Map<String, dynamic>.from(p as Map),
            ];
          }
          onChanged(next);
        },
      ),
      CheckboxListTile(
        key: const Key('icon-only'),
        contentPadding: EdgeInsets.zero,
        title: const Text('Show icon only'),
        value: value['showValue'] == false,
        onChanged: (v) => onChanged({...value, 'showValue': v != true}),
      ),
      TextButton.icon(
        key: const Key('reverse-icons'),
        icon: const Icon(Icons.swap_vert),
        label: const Text('Reverse icon order'),
        onPressed: () => onChanged({
          ...value,
          if (value['icons'] is List)
            'icons': (value['icons'] as List).reversed.toList()
          else
            'reverse': value['reverse'] != true,
        }),
      ),
      const Text(
        'Highest values first. Click an icon to customize its category.',
      ),
      const SizedBox(height: 12),
      for (var i = points.length - 1; i >= 0; i--)
        Padding(
          padding: const EdgeInsets.only(bottom: 12),
          child: Row(
            children: [
              IconButton(
                key: Key('custom-icon-$i'),
                tooltip: 'Change category ${i + 1} icon',
                onPressed: () => choose(context, i),
                icon: iconAt(i)['set'] == 'NoIcons'
                    ? const Icon(Icons.block, size: 20)
                    : SheetIconGlyph(
                        set: iconAt(i)['set'] as String,
                        index: (iconAt(i)['index'] as num).toInt(),
                      ),
              ),
              if (i == 0)
                const Expanded(child: Text('All other numeric values'))
              else ...[
                SizedBox(
                  width: 78,
                  child: DropdownButtonFormField<bool>(
                    key: ValueKey('icon-gte-$i-${value['set']}'),
                    initialValue: points[i]['gte'] != false,
                    decoration: const InputDecoration(labelText: 'When'),
                    items: const [
                      DropdownMenuItem(value: true, child: Text('≥')),
                      DropdownMenuItem(value: false, child: Text('>')),
                    ],
                    onChanged: (v) => point(i, 'gte', v),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: TextFormField(
                    key: ValueKey('icon-value-$i-${value['set']}'),
                    initialValue: points[i]['value'] as String,
                    decoration: const InputDecoration(labelText: 'Value'),
                    onChanged: (v) => point(i, 'value', v),
                  ),
                ),
                const SizedBox(width: 12),
                SizedBox(
                  width: 190,
                  child: DropdownButtonFormField<String>(
                    key: ValueKey('icon-type-$i-${value['set']}'),
                    initialValue: points[i]['type'] as String,
                    isExpanded: true,
                    decoration: const InputDecoration(labelText: 'Type'),
                    items: [
                      for (final e in {
                        'num': 'Number',
                        'percent': 'Percent',
                        'percentile': 'Percentile',
                        'formula': 'Formula',
                      }.entries)
                        DropdownMenuItem(value: e.key, child: Text(e.value)),
                    ],
                    onChanged: (v) => point(i, 'type', v),
                  ),
                ),
              ],
            ],
          ),
        ),
      const Text(
        r'Percent divides the lowest-to-highest value range. Percentile uses the distribution of values. Formulas use absolute references, such as =$B$1. Text, blank cells, and errors have no icon.',
      ),
    ],
  );
}
