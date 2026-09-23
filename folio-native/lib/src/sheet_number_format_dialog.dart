import 'dart:async';
import 'package:flutter/material.dart';
import 'draft_text_field.dart';
import 'sheet_font.dart';
import 'sheet_number_format.dart';

class SheetNumberFormatDialog extends StatefulWidget {
  final String? format;
  final dynamic value;
  final Map font;
  final Future<Map<String, dynamic>> Function(Map<String, dynamic>) preview;
  const SheetNumberFormatDialog({
    super.key,
    required this.format,
    required this.value,
    required this.font,
    required this.preview,
  });
  @override
  State<SheetNumberFormatDialog> createState() =>
      _SheetNumberFormatDialogState();
}

class _SheetNumberFormatDialogState extends State<SheetNumberFormatDialog> {
  static const formats = <String, String>{
    'General': 'General',
    'Number': '#,##0.00',
    'Currency': r'$#,##0.00;[Red]($#,##0.00)',
    'Accounting': r'_($* #,##0.00_);_($* (#,##0.00);_($* "-"??_);_(@_)',
    'Short date': 'm/d/yyyy',
    'Long date': 'dddd, mmmm d, yyyy',
    'Time': 'h:mm:ss AM/PM',
    'Elapsed time': '[h]:mm:ss',
    'Percentage': '0.00%',
    'Fraction': '# ??/??',
    'Scientific': '0.00E+00',
    'Text': '@',
  };
  late final code = TextEditingController(text: widget.format ?? 'General');
  Timer? timer;
  int generation = 0;
  Map<String, dynamic>? result;
  String? error;
  bool pending = true;
  @override
  void initState() {
    super.initState();
    code.addListener(changed);
    unawaited(refresh());
  }

  void changed() {
    timer?.cancel();
    generation++;
    setState(() {
      pending = true;
      error = null;
    });
    timer = Timer(const Duration(milliseconds: 180), refresh);
  }

  Future<void> refresh() async {
    final ticket = ++generation;
    try {
      final next = await widget.preview({
        'format': code.text,
        'value': widget.value,
      });
      if (!mounted || ticket != generation) return;
      setState(() {
        result = next;
        error = null;
        pending = false;
      });
    } catch (e) {
      if (!mounted || ticket != generation) return;
      setState(() {
        error = e.toString().replaceFirst('Bad state: ', '');
        pending = false;
      });
    }
  }

  @override
  void dispose() {
    timer?.cancel();
    code.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Number format'),
    content: SizedBox(
      width: 540,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (widget.format == null)
              const Padding(
                padding: EdgeInsets.only(bottom: 12),
                child: Text('The selection contains different number formats.'),
              ),
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: [
                for (final entry in formats.entries)
                  ChoiceChip(
                    label: Text(entry.key),
                    selected: code.text == entry.value,
                    onSelected: (_) => code.text = entry.value,
                  ),
              ],
            ),
            const SizedBox(height: 18),
            draftTextField(
              controller: code,
              autofocus: true,
              maxLength: 255,
              key: const Key('number-format-code'),
              decoration: const InputDecoration(
                labelText: 'Format code',
                border: OutlineInputBorder(),
                hintText: '0.0000',
              ),
            ),
            const SizedBox(height: 8),
            const Text(
              'Preview',
              style: TextStyle(fontWeight: FontWeight.w600),
            ),
            Container(
              height: 64,
              margin: const EdgeInsets.only(top: 8),
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                border: Border.all(color: Colors.grey.shade400),
                borderRadius: BorderRadius.circular(4),
              ),
              child: pending
                  ? const Center(
                      child: SizedBox(
                        width: 20,
                        height: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                    )
                  : SheetFontText(
                      result?['text'] as String? ?? '',
                      key: const Key('number-format-preview'),
                      font: numberDisplayStyle(widget.font, result),
                      layout: CellTextLayout(widget.font, value: widget.value),
                    ),
            ),
            if (error != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
            if (error == null && result?['warning'] != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(result!['warning']),
              ),
            const SizedBox(height: 12),
            const Text(
              'Use 0 for a required digit and # for an optional digit. Separate positive, negative, zero, and text formats with semicolons. Formatting changes the display; the value stays the same.',
              style: TextStyle(fontSize: 12),
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
        onPressed: pending || error != null
            ? null
            : () => Navigator.pop(context, code.text),
        child: const Text('Apply'),
      ),
    ],
  );
}
