import 'package:flutter/material.dart';

/// Read-only tracing. The caller pins the workbook revision and calculation
/// generation, and validates again before navigating out of this dialog.
class FormulaInspector extends StatefulWidget {
  final String sheetId, address;
  final Future<Map<String, dynamic>> Function(Map<String, dynamic>) onRead;
  final Listenable changes;
  final bool Function() isCurrent;
  const FormulaInspector({
    super.key,
    required this.sheetId,
    required this.address,
    required this.onRead,
    required this.changes,
    required this.isCurrent,
  });
  @override
  State<FormulaInspector> createState() => _FormulaInspectorState();
}

class _FormulaInspectorState extends State<FormulaInspector> {
  Map<String, dynamic>? data;
  late String sheetId = widget.sheetId, address = widget.address;
  final history = <(String, String)>[];
  bool busy = true, stale = false;
  String? error;
  int offset = 0;

  @override
  void initState() {
    super.initState();
    widget.changes.addListener(changed);
    read();
  }

  @override
  void dispose() {
    widget.changes.removeListener(changed);
    super.dispose();
  }

  void changed() {
    if (!widget.isCurrent() && mounted) setState(() => stale = true);
  }

  Future<void> read({bool more = false}) async {
    if (stale || !widget.isCurrent()) {
      changed();
      return;
    }
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final nextOffset = more ? offset + 100 : 0;
      final next = await widget.onRead({
        'sheetId': sheetId,
        'address': address,
        'offset': nextOffset,
        'limit': 100,
      });
      if (!mounted) return;
      if (!widget.isCurrent()) {
        changed();
        return;
      }
      setState(() {
        if (more && data != null) {
          for (final key in [
            'precedents',
            'dependents',
            'unresolvedFormulas',
          ]) {
            next[key] = [...data![key] as List, ...next[key] as List];
          }
        }
        data = next;
        offset = nextOffset;
      });
    } catch (e) {
      if (mounted) setState(() => error = '$e');
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  bool get enabled => !busy && !stale && error == null;
  void inspect(Map cell) {
    if (!enabled || !widget.isCurrent()) {
      changed();
      return;
    }
    history.add((sheetId, address));
    sheetId = cell['sheetId'];
    address = cell['address'];
    read();
  }

  void go(Map cell) {
    if (!enabled || !widget.isCurrent()) {
      changed();
      return;
    }
    Navigator.pop(context, cell);
  }

  Widget links(String key, String countKey, String empty) {
    final rows = (data?[key] as List?) ?? const [];
    final count = data?[countKey] as int? ?? 0;
    return Column(
      children: [
        Expanded(
          child: rows.isEmpty
              ? Center(
                  child: Padding(
                    padding: const EdgeInsets.all(20),
                    child: Text(empty),
                  ),
                )
              : ListView.separated(
                  key: ValueKey('audit-$key-list'),
                  itemCount: rows.length,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (_, i) {
                    final cell = rows[i] as Map;
                    final label = '${cell['sheetName']}!${cell['range']}';
                    final warnings = (cell['warnings'] as List?)?.join('\n');
                    return ListTile(
                      title: Text(label),
                      subtitle: warnings == null ? null : Text(warnings),
                      trailing: Wrap(
                        spacing: 4,
                        children: [
                          TextButton(
                            key: ValueKey(
                              'inspect-$key-${cell['sheetId']}-${cell['range']}',
                            ),
                            onPressed: enabled ? () => inspect(cell) : null,
                            child: Text(
                              cell['isRange'] == true
                                  ? 'Inspect first cell'
                                  : 'Inspect',
                            ),
                          ),
                          TextButton(
                            key: ValueKey(
                              'goto-$key-${cell['sheetId']}-${cell['range']}',
                            ),
                            onPressed: enabled ? () => go(cell) : null,
                            child: Text(
                              cell['isRange'] == true
                                  ? 'Go to range'
                                  : 'Go to cell',
                            ),
                          ),
                        ],
                      ),
                    );
                  },
                ),
        ),
        if (count > rows.length)
          TextButton(
            key: ValueKey('audit-more-$key'),
            onPressed: enabled ? () => read(more: true) : null,
            child: Text('Show more · ${rows.length} of $count'),
          ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final cell = data?['cell'] as Map?;
    final warnings = (data?['warnings'] as List?) ?? const [];
    final incomplete = (data?['unresolvedFormulaCount'] as int? ?? 0) > 0;
    return AlertDialog(
      key: const Key('formula-inspector'),
      title: const Text('Formula inspector'),
      content: SizedBox(
        width: 850,
        height: 590,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                IconButton(
                  tooltip: 'Back to previous cell',
                  onPressed: enabled && history.isNotEmpty
                      ? () {
                          final previous = history.removeLast();
                          sheetId = previous.$1;
                          address = previous.$2;
                          read();
                        }
                      : null,
                  icon: const Icon(Icons.arrow_back),
                ),
                Expanded(
                  child: Text(
                    cell == null
                        ? address
                        : '${cell['sheetName']}!${cell['address']}',
                    key: const Key('audit-current-cell'),
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                TextButton(
                  onPressed: enabled && cell != null ? () => go(cell) : null,
                  child: const Text('Go to this cell'),
                ),
              ],
            ),
            if (data != null) ...[
              ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 100),
                child: SingleChildScrollView(
                  child: SelectableText(
                    '${data!['input']}'.isEmpty
                        ? '(Empty cell)'
                        : '${data!['input']}',
                    key: const Key('audit-formula'),
                    style: const TextStyle(fontFamily: 'monospace'),
                  ),
                ),
              ),
              const SizedBox(height: 8),
              Text(
                'Value: ${data!['display'] ?? data!['value'] ?? '(empty)'}',
                style: TextStyle(
                  color: data!['isError'] == true
                      ? Theme.of(context).colorScheme.error
                      : null,
                ),
              ),
            ],
            const SizedBox(height: 12),
            const Text(
              'Direct formula references, including all IF branches. Inspect a linked cell to follow the next level.',
            ),
            if (incomplete)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  'Unresolved formulas: ${data!['unresolvedFormulaCount']}. The dependent list may be incomplete; see Unresolved.',
                ),
              ),
            if (warnings.isNotEmpty)
              ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 80),
                child: SingleChildScrollView(child: Text(warnings.join('\n'))),
              ),
            if (stale)
              const Padding(
                padding: EdgeInsets.symmetric(vertical: 10),
                child: Text(
                  'The workbook changed. Close and reopen Formula inspector to refresh its references.',
                  key: Key('audit-stale'),
                ),
              ),
            if (error != null)
              Row(
                children: [
                  Expanded(child: Text(error!, key: const Key('audit-error'))),
                  TextButton(
                    onPressed: busy || stale ? null : () => read(),
                    child: const Text('Retry'),
                  ),
                ],
              ),
            if (busy) const LinearProgressIndicator(),
            const SizedBox(height: 8),
            Expanded(
              child: DefaultTabController(
                length: 3,
                child: Column(
                  children: [
                    TabBar(
                      tabs: [
                        Tab(
                          text: 'Precedents (${data?['precedentCount'] ?? 0})',
                        ),
                        Tab(
                          text: 'Dependents (${data?['dependentCount'] ?? 0})',
                        ),
                        Tab(
                          text:
                              'Unresolved (${data?['unresolvedFormulaCount'] ?? 0})',
                        ),
                      ],
                    ),
                    Expanded(
                      child: TabBarView(
                        children: [
                          links(
                            'precedents',
                            'precedentCount',
                            'No direct cell references found in this cell.',
                          ),
                          links(
                            'dependents',
                            'dependentCount',
                            'No direct dependent formulas found.',
                          ),
                          links(
                            'unresolvedFormulas',
                            'unresolvedFormulaCount',
                            'No unresolved formula references found.',
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Close'),
        ),
      ],
    );
  }
}
