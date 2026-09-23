import 'draft_text_field.dart';
import 'package:flutter/material.dart';
import 'document_codec.dart';
import 'document_table_conversion.dart';
import 'document_table_view.dart';
import 'document_tables.dart';

class DocumentTableConversionDialog extends StatefulWidget {
  final TextTableSelection? selection;
  final Map? table;
  final int? firstRow, lastRow;
  final bool applyTableEdits;
  const DocumentTableConversionDialog({
    super.key,
    this.selection,
    this.table,
    this.firstRow,
    this.lastRow,
    this.applyTableEdits = false,
  }) : assert((selection == null) != (table == null));
  @override
  State<DocumentTableConversionDialog> createState() =>
      _ConversionDialogState();
}

class _ConversionDialogState extends State<DocumentTableConversionDialog> {
  String separator = '\t';
  final other = TextEditingController(text: ';');
  final columns = TextEditingController();
  final width = TextEditingController();
  final firstRow = TextEditingController(), lastRow = TextEditingController();
  bool nested = true;
  bool get toTable => widget.selection != null;
  String get delimiter => separator == 'other' ? other.text : separator;
  @override
  void initState() {
    super.initState();
    if (toTable) {
      final text = widget.selection!.text;
      separator = text.contains('\t')
          ? '\t'
          : text.contains(',')
          ? ','
          : '\n';
      columns.text = '${widget.selection!.suggestedColumns(separator)}';
    } else {
      final rows = DocumentTable(widget.table!).rows.length;
      firstRow.text = '${(widget.firstRow ?? 0) + 1}';
      lastRow.text = '${(widget.lastRow ?? rows - 1) + 1}';
    }
    columns.addListener(_settingsChanged);
    width.addListener(_settingsChanged);
    other.addListener(_otherChanged);
    firstRow.addListener(_settingsChanged);
    lastRow.addListener(_settingsChanged);
  }

  void _settingsChanged() {
    if (mounted) setState(() {});
  }

  void _otherChanged() {
    if (toTable && delimiter.runes.length == 1) {
      columns.text = '${widget.selection!.suggestedColumns(delimiter)}';
    }
    _settingsChanged();
  }

  @override
  void dispose() {
    other.dispose();
    columns.dispose();
    width.dispose();
    firstRow.dispose();
    lastRow.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    Map<String, dynamic>? preview;
    TableRowsConversion? rowConversion;
    String? error;
    try {
      if (toTable) {
        final count = int.tryParse(columns.text);
        final points = width.text.trim().isEmpty
            ? null
            : double.tryParse(width.text);
        if (count == null) {
          throw const FormatException('Enter a whole number of columns.');
        }
        if (width.text.trim().isNotEmpty && points == null) {
          throw const FormatException(
            'Enter a numeric column width in points.',
          );
        }
        preview = widget.selection!.table(
          separator: delimiter,
          columns: count,
          columnWidth: points,
        );
      } else {
        final first = int.tryParse(firstRow.text);
        final last = int.tryParse(lastRow.text);
        if (first == null || last == null) {
          throw const FormatException('Enter whole row numbers.');
        }
        rowConversion = TableRowsConversion(
          widget.table!,
          firstRow: first - 1,
          lastRow: last - 1,
          separator: delimiter,
          convertNested: nested,
        );
        preview = rowConversion.document;
      }
    } on FormatException catch (e) {
      error = e.message;
    }
    final result = preview;
    final rowCount = toTable && result != null
        ? (result['content'] as List).length
        : 0;
    final tablePreview = toTable && result != null
        ? <String, dynamic>{
            ...result,
            'content': (result['content'] as List).take(10).toList(),
          }
        : null;
    return AlertDialog(
      title: Text(toTable ? 'Convert text to table' : 'Convert table to text'),
      content: SizedBox(
        width: 640,
        height: toTable ? 420 : 490,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: DropdownButtonFormField<String>(
                    key: const Key('conversion-separator'),
                    initialValue: separator,
                    decoration: InputDecoration(
                      labelText: toTable
                          ? 'Separate text at'
                          : 'Separate cells with',
                    ),
                    items: const [
                      DropdownMenuItem(value: '\t', child: Text('Tabs')),
                      DropdownMenuItem(value: ',', child: Text('Commas')),
                      DropdownMenuItem(value: '\n', child: Text('Paragraphs')),
                      DropdownMenuItem(
                        value: 'other',
                        child: Text('Other character'),
                      ),
                    ],
                    onChanged: (v) => setState(() {
                      separator = v!;
                      if (toTable && delimiter.runes.length == 1) {
                        columns.text =
                            '${widget.selection!.suggestedColumns(delimiter)}';
                      }
                    }),
                  ),
                ),
                if (separator == 'other') ...[
                  const SizedBox(width: 16),
                  SizedBox(
                    width: 110,
                    child: draftTextField(
                      key: const Key('conversion-other'),
                      controller: other,
                      decoration: const InputDecoration(labelText: 'Character'),
                    ),
                  ),
                ],
              ],
            ),
            const SizedBox(height: 12),
            if (toTable)
              Row(
                children: [
                  Expanded(
                    child: draftTextField(
                      key: const Key('conversion-columns'),
                      controller: columns,
                      decoration: const InputDecoration(labelText: 'Columns'),
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: draftTextField(
                      key: const Key('conversion-width'),
                      controller: width,
                      decoration: const InputDecoration(
                        labelText: 'Column width (pt)',
                        hintText: 'Fill available width',
                      ),
                    ),
                  ),
                ],
              )
            else ...[
              Row(
                children: [
                  Expanded(
                    child: draftTextField(
                      key: const Key('conversion-first-row'),
                      controller: firstRow,
                      decoration: const InputDecoration(labelText: 'First row'),
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: draftTextField(
                      key: const Key('conversion-last-row'),
                      controller: lastRow,
                      decoration: const InputDecoration(labelText: 'Last row'),
                    ),
                  ),
                ],
              ),
              CheckboxListTile(
                key: const Key('conversion-nested'),
                contentPadding: EdgeInsets.zero,
                title: const Text('Convert nested tables too'),
                value: nested,
                onChanged: (v) => setState(() => nested = v!),
              ),
            ],
            const SizedBox(height: 12),
            Text(
              toTable
                  ? 'Paragraphs start new rows. Short rows are padded; excess cells continue on the next row.'
                  : 'Rows stay separate. Merged positions become empty fields. Joined paragraphs use the first paragraph’s format.',
            ),
            const SizedBox(height: 12),
            if (error != null)
              Text(error, style: const TextStyle(color: Colors.red)),
            if (rowConversion != null) ...[
              Text(
                'Rows ${rowConversion.rows.top + 1}–${rowConversion.rows.bottom + 1} will become text.',
                key: const Key('conversion-row-summary'),
              ),
              if (rowConversion.expanded)
                const Text(
                  'The range includes extra rows to keep merged cells intact.',
                ),
              if (widget.applyTableEdits)
                const Text(
                  'Convert and apply also saves your current table edits.',
                ),
            ],
            if (result != null) ...[
              Text(
                toTable
                    ? '$rowCount rows × ${DocumentTable(tablePreview!).columns} columns${rowCount > 10 ? ' · Preview: first 10 rows' : ''}'
                    : 'Preview (→ marks a tab)',
                style: const TextStyle(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 8),
              Expanded(
                child: SingleChildScrollView(
                  child: toTable
                      ? DocumentTableView(node: tablePreview!)
                      : Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            if (rowConversion!.before != null)
                              _retainedRows(rowConversion.rows.top, 'above'),
                            SelectableText(
                              nodeText(
                                rowConversion.converted,
                              ).replaceAll('\t', ' → '),
                            ),
                            if (rowConversion.after != null)
                              _retainedRows(
                                rowConversion.table.rows.length -
                                    rowConversion.rows.bottom -
                                    1,
                                'below',
                              ),
                          ],
                        ),
                ),
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
          key: const Key('conversion-apply'),
          onPressed: result == null
              ? null
              : () => Navigator.pop(context, result),
          child: Text(widget.applyTableEdits ? 'Convert and apply' : 'Convert'),
        ),
      ],
    );
  }

  Widget _retainedRows(int count, String position) => Container(
    margin: const EdgeInsets.symmetric(vertical: 8),
    padding: const EdgeInsets.all(10),
    decoration: BoxDecoration(
      border: Border.all(color: Colors.blueGrey.shade200),
    ),
    child: Text(
      '$count ${count == 1 ? 'row' : 'rows'} $position ${count == 1 ? 'remains' : 'remain'} in a table.',
    ),
  );
}
