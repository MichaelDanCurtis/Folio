import 'package:flutter/material.dart';
import 'draft_text_field.dart';

const pageNumberFormats = {
  'decimal': '1, 2, 3',
  'upperRoman': 'I, II, III',
  'lowerRoman': 'i, ii, iii',
  'upperLetter': 'A, B, C',
  'lowerLetter': 'a, b, c',
  'decimalZero': '01, 02, 03',
  'numberInDash': '– 1 –, – 2 –',
  'ordinal': '1st, 2nd, 3rd',
  'cardinalText': 'One, Two, Three',
  'ordinalText': 'First, Second, Third',
  'hex': '1, 2, … A, B',
  'chicago': '*, †, ‡',
  'none': 'No number',
};
const chapterSeparators = {
  'hyphen': '- (hyphen)',
  'period': '. (period)',
  'colon': ': (colon)',
  'emDash': '— (em dash)',
  'enDash': '– (en dash)',
};
const defaultPageNumbering = {
  'format': 'decimal',
  'start': null,
  'chapterStyle': null,
  'chapterSeparator': 'hyphen',
};

class PageNumberFormatDialog extends StatefulWidget {
  final Map initial;
  final int section;
  const PageNumberFormatDialog({
    super.key,
    required this.initial,
    required this.section,
  });
  @override
  State<PageNumberFormatDialog> createState() => _PageNumberFormatState();
}

class _PageNumberFormatState extends State<PageNumberFormatDialog> {
  late String format, separator;
  late bool restart, chapter;
  late int heading;
  late TextEditingController start;
  String? error;
  @override
  void initState() {
    super.initState();
    format = widget.initial['format'] ?? 'decimal';
    separator = widget.initial['chapterSeparator'] ?? 'hyphen';
    restart = widget.initial['start'] != null;
    chapter = widget.initial['chapterStyle'] != null;
    heading = (widget.initial['chapterStyle'] as int? ?? 1).clamp(1, 9);
    start = TextEditingController(text: '${widget.initial['start'] ?? 1}');
  }

  @override
  void dispose() {
    start.dispose();
    super.dispose();
  }

  void apply() {
    final value = int.tryParse(start.text.trim());
    if (restart && (value == null || value < 0 || value > 2147483647)) {
      setState(
        () => error = 'Enter a whole starting number from 0 to 2147483647.',
      );
      return;
    }
    Navigator.pop(context, {
      'format': format,
      'start': restart ? value : null,
      'chapterStyle': chapter ? heading : null,
      'chapterSeparator': separator,
    });
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text('Page numbers · Section ${widget.section + 1}'),
    content: SizedBox(
      width: 450,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            DropdownButtonFormField<String>(
              key: const Key('page-number-format'),
              initialValue: format,
              isExpanded: true,
              decoration: const InputDecoration(labelText: 'Number format'),
              items: [
                for (final e in {
                  ...pageNumberFormats,
                  if (!pageNumberFormats.containsKey(format))
                    format: 'Imported: $format',
                }.entries)
                  DropdownMenuItem(value: e.key, child: Text(e.value)),
              ],
              onChanged: (v) => setState(() => format = v!),
            ),
            const SizedBox(height: 12),
            CheckboxListTile(
              key: const Key('page-number-restart'),
              contentPadding: EdgeInsets.zero,
              title: const Text('Start at a specific number'),
              value: restart,
              subtitle: const Text(
                'Otherwise continue from the previous section',
              ),
              onChanged: (v) => setState(() => restart = v!),
            ),
            draftTextField(
              key: const Key('page-number-start'),
              controller: start,
              enabled: restart,
              decoration: const InputDecoration(labelText: 'Start at'),
              keyboardType: TextInputType.number,
            ),
            CheckboxListTile(
              key: const Key('page-number-chapter'),
              contentPadding: EdgeInsets.zero,
              title: const Text('Include chapter number'),
              value: chapter,
              onChanged: (v) => setState(() => chapter = v!),
            ),
            if (chapter) ...[
              DropdownButtonFormField<int>(
                key: const Key('page-number-heading'),
                initialValue: heading,
                decoration: const InputDecoration(
                  labelText: 'Chapter starts with style',
                ),
                items: [
                  for (var i = 1; i <= 9; i++)
                    DropdownMenuItem(value: i, child: Text('Heading $i')),
                ],
                onChanged: (v) => setState(() => heading = v!),
              ),
              DropdownButtonFormField<String>(
                key: const Key('page-number-separator'),
                initialValue: separator,
                decoration: const InputDecoration(labelText: 'Use separator'),
                items: [
                  for (final e in {
                    ...chapterSeparators,
                    if (!chapterSeparators.containsKey(separator))
                      separator: 'Imported: $separator',
                  }.entries)
                    DropdownMenuItem(value: e.key, child: Text(e.value)),
                ],
                onChanged: (v) => setState(() => separator = v!),
              ),
              const Padding(
                padding: EdgeInsets.only(top: 8),
                child: Text(
                  'Chapter headings must use a numbered heading style.',
                ),
              ),
            ],
            const SizedBox(height: 12),
            const Text(
              'Numbering belongs to this section even when its header or footer is linked. Apply in Headers & footers saves these settings. Page values await pagination.',
            ),
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
        key: const Key('page-number-done'),
        onPressed: apply,
        child: const Text('Done'),
      ),
    ],
  );
}
