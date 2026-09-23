import 'package:flutter/material.dart';
import 'document_table_sort.dart';
import 'document_tables.dart';
import 'draft_text_field.dart';

const tableSortLanguages = {
  'en_US': 'English (United States)',
  'en_GB': 'English (United Kingdom)',
  'de_DE': 'German',
  'fr_FR': 'French',
  'es_ES': 'Spanish',
  'sv_SE': 'Swedish',
  'tr_TR': 'Turkish',
  'hi_IN': 'Hindi (India)',
  'ar': 'Arabic',
  'he': 'Hebrew',
  'ja_JP': 'Japanese',
  'zh_CN': 'Chinese (Simplified)',
};

class DocumentTableSortDialog extends StatefulWidget {
  final DocumentTable table;
  final TableArea selection;
  final TableTextRanker ranker;
  const DocumentTableSortDialog({
    super.key,
    required this.table,
    required this.selection,
    this.ranker = nativeTableTextRanks,
  });
  @override
  State<DocumentTableSortDialog> createState() => _TableSortDialogState();
}

class _TableSortDialogState extends State<DocumentTableSortDialog> {
  final first = TextEditingController(), last = TextEditingController();
  final headings = TextEditingController();
  final fields = List.generate(3, (_) => TextEditingController(text: '1'));
  late List<int> columns;
  final types = List.filled(3, TableSortType.text);
  final descending = List.filled(3, false);
  String locale = 'en_US', separator = '\n';
  bool caseSensitive = false, busy = true;
  int generation = 0;
  TableSortResult? preview;
  String? error;
  @override
  void initState() {
    super.initState();
    columns = [widget.selection.left, -1, -1];
    final selected = widget.selection.bottom > widget.selection.top;
    first.text = '${selected ? widget.selection.top + 1 : 1}';
    last.text =
        '${selected ? widget.selection.bottom + 1 : widget.table.rows.length}';
    headings.text =
        '${int.parse(first.text) == 1 ? widget.table.headerRows : 0}';
    for (final c in [first, last, headings, ...fields]) {
      c.addListener(refresh);
    }
    refresh();
  }

  Future<void> refresh() async {
    final token = ++generation;
    setState(() {
      busy = true;
      preview = null;
      error = null;
    });
    try {
      final firstRow = int.tryParse(first.text),
          lastRow = int.tryParse(last.text);
      final headerRows = int.tryParse(headings.text);
      if (firstRow == null || lastRow == null || headerRows == null) {
        throw const FormatException(
          'Enter whole row numbers and a heading count.',
        );
      }
      final keys = <TableSortKey>[];
      for (var i = 0; i < 3; i++) {
        if (columns[i] < 0) continue;
        final field = int.tryParse(fields[i].text);
        if (field == null || field < 1) {
          throw const FormatException('Field numbers start at 1.');
        }
        keys.add(
          TableSortKey(
            columns[i],
            type: types[i],
            descending: descending[i],
            field: field - 1,
          ),
        );
      }
      final value = await sortDocumentTable(
        widget.table.node,
        keys: keys,
        firstRow: firstRow - 1,
        lastRow: lastRow - 1,
        headerRows: headerRows,
        locale: locale,
        caseSensitive: caseSensitive,
        separator: separator,
        ranker: widget.ranker,
      );
      if (mounted && token == generation) setState(() => preview = value);
    } catch (e) {
      if (mounted && token == generation) {
        setState(
          () => error = e is FormatException
              ? e.message
              : 'Sorting could not finish. Try again.',
        );
      }
    } finally {
      if (mounted && token == generation) setState(() => busy = false);
    }
  }

  void settings(VoidCallback update) {
    setState(update);
    refresh();
  }

  @override
  void dispose() {
    for (final c in [first, last, headings, ...fields]) {
      c.dispose();
    }
    super.dispose();
  }

  String label(int column) {
    if ((int.tryParse(headings.text) ?? 0) > 0 &&
        int.tryParse(first.text) == 1) {
      final text = tableSortCellText(
        widget.table.at(0, column).cell,
      ).replaceAll('\n', ' ').trim();
      if (text.isNotEmpty) return '${column + 1}: $text';
    }
    return 'Column ${column + 1}';
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Sort table'),
    content: SizedBox(
      width: 740,
      height: 560,
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const Text(
              'Complete rows move together. Sorting stays in your table draft until Apply changes.',
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                for (final (key, title, controller) in [
                  ('sort-first-row', 'First row', first),
                  ('sort-last-row', 'Last row', last),
                  ('sort-heading-rows', 'Heading rows to keep', headings),
                ])
                  Expanded(
                    child: Padding(
                      padding: const EdgeInsets.only(right: 12),
                      child: draftTextField(
                        key: Key(key),
                        controller: controller,
                        decoration: InputDecoration(labelText: title),
                      ),
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 16),
            for (var i = 0; i < 3; i++)
              Padding(
                padding: const EdgeInsets.only(bottom: 12),
                child: Row(
                  children: [
                    Expanded(
                      flex: 3,
                      child: DropdownButtonFormField<int>(
                        key: Key('sort-column-$i'),
                        initialValue: columns[i],
                        isExpanded: true,
                        decoration: InputDecoration(
                          labelText: i == 0 ? 'Sort by' : 'Then by',
                        ),
                        items: [
                          if (i > 0)
                            const DropdownMenuItem(
                              value: -1,
                              child: Text('None'),
                            ),
                          for (var c = 0; c < widget.table.columns; c++)
                            DropdownMenuItem(
                              value: c,
                              child: Text(
                                label(c),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                        ],
                        onChanged: (v) => settings(() => columns[i] = v!),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      flex: 2,
                      child: DropdownButtonFormField<TableSortType>(
                        key: Key('sort-type-$i'),
                        initialValue: types[i],
                        decoration: const InputDecoration(labelText: 'Type'),
                        items: [
                          for (final type in TableSortType.values)
                            DropdownMenuItem(
                              value: type,
                              child: Text(switch (type) {
                                TableSortType.text => 'Text',
                                TableSortType.number => 'Number',
                                TableSortType.date => 'Date',
                              }),
                            ),
                        ],
                        onChanged: columns[i] < 0
                            ? null
                            : (v) => settings(() => types[i] = v!),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      flex: 2,
                      child: DropdownButtonFormField<bool>(
                        key: Key('sort-order-$i'),
                        isExpanded: true,
                        initialValue: descending[i],
                        decoration: const InputDecoration(labelText: 'Order'),
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
                        onChanged: columns[i] < 0
                            ? null
                            : (v) => settings(() => descending[i] = v!),
                      ),
                    ),
                    const SizedBox(width: 12),
                    SizedBox(
                      width: 62,
                      child: draftTextField(
                        key: Key('sort-field-$i'),
                        controller: fields[i],
                        enabled: columns[i] >= 0,
                        decoration: const InputDecoration(labelText: 'Field'),
                      ),
                    ),
                  ],
                ),
              ),
            Row(
              children: [
                Expanded(
                  child: DropdownButtonFormField<String>(
                    key: const Key('sort-language'),
                    initialValue: locale,
                    isExpanded: true,
                    decoration: const InputDecoration(
                      labelText: 'Sort language',
                    ),
                    items: [
                      for (final e in tableSortLanguages.entries)
                        DropdownMenuItem(value: e.key, child: Text(e.value)),
                    ],
                    onChanged: (v) => settings(() => locale = v!),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: DropdownButtonFormField<String>(
                    key: const Key('sort-separator'),
                    isExpanded: true,
                    initialValue: separator,
                    decoration: const InputDecoration(
                      labelText: 'Separate fields at',
                    ),
                    items: const [
                      DropdownMenuItem(value: '\n', child: Text('Paragraphs')),
                      DropdownMenuItem(value: '\t', child: Text('Tabs')),
                      DropdownMenuItem(value: ',', child: Text('Commas')),
                      DropdownMenuItem(value: ';', child: Text('Semicolons')),
                    ],
                    onChanged: (v) => settings(() => separator = v!),
                  ),
                ),
              ],
            ),
            CheckboxListTile(
              key: const Key('sort-case-sensitive'),
              contentPadding: EdgeInsets.zero,
              title: const Text('Case sensitive'),
              value: caseSensitive,
              onChanged: (v) => settings(() => caseSensitive = v!),
            ),
            const Text(
              'Empty fields stay last. Equal keys keep their original order. Dates and numbers use the selected language.',
            ),
            const SizedBox(height: 12),
            if (busy) const LinearProgressIndicator(),
            if (error != null)
              Text(
                error!,
                key: const Key('sort-error'),
                style: const TextStyle(color: Colors.red),
              ),
            if (preview != null) ...[
              const Text(
                'Preview · original row numbers',
                style: TextStyle(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 6),
              for (final row in preview!.order.take(8))
                Text(
                  'Row ${row + 1}   ${tableSortCellText(widget.table.at(row, columns.first).cell).replaceAll('\n', ' ↵ ')}',
                  key: ValueKey('sort-preview-row-$row'),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              if (preview!.order.length > 8)
                Text('… ${preview!.order.length - 8} more rows'),
            ],
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
        key: const Key('sort-table-apply'),
        onPressed: preview == null || busy
            ? null
            : () => Navigator.pop(context, preview),
        child: const Text('Sort'),
      ),
    ],
  );
}
