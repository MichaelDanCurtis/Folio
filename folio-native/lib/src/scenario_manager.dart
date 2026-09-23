import 'package:flutter/material.dart';
import 'core.dart' show CoreCall;
import 'draft_text_field.dart';

class ScenarioManager extends StatefulWidget {
  final List Function() sheets;
  final String sheetId, selection;
  final Listenable changes;
  final bool Function() isCurrent;
  final CoreCall onRead;
  final Future<Map<String, dynamic>?> Function()? onBrowseSource;
  final Future<void> Function(Map<String, dynamic>) onCommand;
  const ScenarioManager({
    super.key,
    required this.sheets,
    required this.sheetId,
    required this.selection,
    required this.changes,
    required this.isCurrent,
    required this.onRead,
    this.onBrowseSource,
    required this.onCommand,
  });
  @override
  State<ScenarioManager> createState() => _ScenarioManagerState();
}

class _ScenarioManagerState extends State<ScenarioManager> {
  int? selected;
  bool busy = false, stale = false, replaceFormulas = false;
  String? error;
  late Map savedSheet;
  Map get sheet {
    final candidates = widget.sheets().where((s) => s['id'] == widget.sheetId);
    if (widget.isCurrent() && candidates.isNotEmpty) {
      savedSheet = candidates.first as Map;
    }
    return savedSheet;
  }

  List get scenarios => (sheet['scenarios']?['items'] as List?) ?? [];
  Map? get current => selected != null && selected! < scenarios.length
      ? scenarios[selected!] as Map
      : null;
  bool get enabled => !busy && !stale;
  bool get replacesFormulas => (current?['inputs'] as List? ?? []).any(
    (i) =>
        i['deleted'] != true &&
        (sheet['cells'][i['address']] as String? ?? '').startsWith('='),
  );
  @override
  void initState() {
    super.initState();
    savedSheet =
        widget.sheets().firstWhere((s) => s['id'] == widget.sheetId) as Map;
    if (scenarios.isNotEmpty) selected = 0;
    widget.changes.addListener(changed);
  }

  @override
  void dispose() {
    widget.changes.removeListener(changed);
    super.dispose();
  }

  void changed() {
    if (!mounted || busy || widget.isCurrent()) return;
    setState(() {
      stale = true;
      error = 'The workbook changed. Close and reopen Scenario Manager.';
    });
  }

  Future<void> command(Map<String, dynamic> request) async {
    if (!widget.isCurrent()) {
      changed();
      throw StateError('The workbook changed. Reopen Scenario Manager.');
    }
    setState(() {
      busy = true;
      error = null;
    });
    try {
      await widget.onCommand({'sheetId': widget.sheetId, ...request});
      if (!mounted) return;
      setState(() {
        replaceFormulas = false;
        if (scenarios.isEmpty) {
          selected = null;
        } else if (selected == null || selected! >= scenarios.length) {
          selected = scenarios.length - 1;
        }
      });
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  Future<void> run(Map<String, dynamic> request) async {
    try {
      await command(request);
    } catch (e) {
      if (mounted) setState(() => error = e.toString());
    }
  }

  Future<void> edit(bool existing) async {
    final index = existing ? selected : null;
    final saved = await showDialog<bool>(
      context: context,
      builder: (_) => ScenarioEditor(
        initial: existing ? Map<String, dynamic>.from(current!) : null,
        selection: widget.selection,
        isCurrent: widget.isCurrent,
        onCapture: (range) => widget.onRead({
          'op': 'scenarioCapture',
          'sheetId': widget.sheetId,
          'range': range,
        }),
        onSave: (value) => command({
          'op': 'scenarios',
          'action': existing ? 'edit' : 'add',
          if (existing) 'index': index,
          ...value,
        }),
      ),
    );
    if (mounted && saved == true && !existing) {
      setState(() => selected = scenarios.length - 1);
    }
    if (mounted) changed();
  }

  Future<void> merge() async {
    int? sourceToken;
    try {
      await showDialog<bool>(
        context: context,
        builder: (_) => ScenarioMergeDialog(
          sheets: widget.sheets(),
          targetId: widget.sheetId,
          isCurrent: widget.isCurrent,
          onBrowse: widget.onBrowseSource == null
              ? null
              : () async {
                  final source = await widget.onBrowseSource!();
                  if (source != null) sourceToken = source['token'] as int;
                  return source;
                },
          onExternalMerge: (token, id, indices) => command({
            'op': 'scenarios',
            'action': 'mergeExternal',
            'sourceToken': token,
            'sourceSheetId': id,
            'indices': indices,
          }),
          onMerge: (id, indices) => command({
            'op': 'scenarios',
            'action': 'merge',
            'sourceSheetId': id,
            'indices': indices,
          }),
        ),
      );
    } finally {
      if (sourceToken != null) {
        try {
          await widget.onRead({
            'op': 'closeScenarioSource',
            'sourceToken': sourceToken,
          });
        } catch (_) {
          // A changed workbook already invalidates the bounded source snapshot.
        }
      }
    }
    if (mounted) changed();
  }

  Future<void> summary() async {
    final result = await showDialog<bool>(
      context: context,
      builder: (_) => ScenarioSummaryDialog(
        initial:
            sheet['scenarios']?['resultCells'] as String? ?? widget.selection,
        onCreate: (range) =>
            command({'op': 'scenarioSummary', 'resultCells': range}),
      ),
    );
    if (mounted && result == true) Navigator.pop(context, 'summary');
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !busy,
    child: AlertDialog(
      title: const Text('Scenario Manager'),
      content: SizedBox(
        width: 640,
        height: 460,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Save and compare input sets for ${sheet['name']}. Showing a scenario replaces its changing cells; Undo restores them.',
            ),
            const SizedBox(height: 12),
            Expanded(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: Container(
                      decoration: BoxDecoration(
                        border: Border.all(
                          color: Theme.of(context).dividerColor,
                        ),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: scenarios.isEmpty
                          ? const Center(
                              child: Text(
                                'No scenarios yet. Add a set of changing cells.',
                              ),
                            )
                          : ListView.builder(
                              itemCount: scenarios.length,
                              itemBuilder: (_, i) {
                                final item = scenarios[i];
                                return ListTile(
                                  key: Key('scenario-$i'),
                                  dense: true,
                                  selected: i == selected,
                                  title: Text(
                                    item['name'] as String? ??
                                        'Unsupported scenario',
                                  ),
                                  subtitle: Text(
                                    item['unsupported'] == true
                                        ? 'Preserved; additional support needed'
                                        : '${(item['inputs'] as List).length} changing cells',
                                  ),
                                  onTap: enabled
                                      ? () => setState(() {
                                          selected = i;
                                          replaceFormulas = false;
                                        })
                                      : null,
                                );
                              },
                            ),
                    ),
                  ),
                  const SizedBox(width: 12),
                  SizedBox(
                    width: 140,
                    child: ListView(
                      children: [
                        OutlinedButton(
                          key: const Key('scenario-add'),
                          onPressed: enabled ? () => edit(false) : null,
                          child: const Text('Add…'),
                        ),
                        OutlinedButton(
                          key: const Key('scenario-edit'),
                          onPressed:
                              enabled &&
                                  current != null &&
                                  current?['unsupported'] != true
                              ? () => edit(true)
                              : null,
                          child: const Text('Edit…'),
                        ),
                        OutlinedButton(
                          key: const Key('scenario-delete'),
                          onPressed:
                              enabled &&
                                  current != null &&
                                  current?['unsupported'] != true
                              ? () => run({
                                  'op': 'scenarios',
                                  'action': 'delete',
                                  'index': selected,
                                })
                              : null,
                          child: const Text('Delete'),
                        ),
                        OutlinedButton(
                          key: const Key('scenario-merge'),
                          onPressed: enabled ? merge : null,
                          child: const Text('Merge…'),
                        ),
                        OutlinedButton(
                          key: const Key('scenario-summary'),
                          onPressed: enabled && scenarios.isNotEmpty
                              ? summary
                              : null,
                          child: const Text('Summary…'),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 12),
            if (current != null) ...[
              Text(
                'Changing cells: ${(current!['inputs'] as List? ?? []).where((i) => i['deleted'] != true).map((i) => i['address']).join(', ')}',
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 4),
              Text(
                current!['comment'] as String? ?? '',
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
              ),
              if ((current!['inputs'] as List? ?? []).any(
                (i) => i['deleted'] == true,
              ))
                const Text(
                  'Deleted input cells are retained in the file and skipped when shown.',
                ),
              if (replacesFormulas)
                CheckboxListTile(
                  key: const Key('scenario-replace-formulas'),
                  dense: true,
                  contentPadding: EdgeInsets.zero,
                  title: const Text(
                    'Allow this scenario to replace formulas in changing cells',
                  ),
                  value: replaceFormulas,
                  onChanged: enabled
                      ? (v) => setState(() => replaceFormulas = v!)
                      : null,
                ),
            ],
            if (error != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: busy ? null : () => Navigator.pop(context),
          child: const Text('Close'),
        ),
        FilledButton(
          key: const Key('scenario-show'),
          onPressed:
              enabled &&
                  current != null &&
                  current?['unsupported'] != true &&
                  (!replacesFormulas || replaceFormulas)
              ? () => run({
                  'op': 'scenarioShow',
                  'index': selected,
                  'replaceFormulas': replaceFormulas,
                })
              : null,
          child: Text(busy ? 'Working…' : 'Show'),
        ),
      ],
    ),
  );
}

class ScenarioEditor extends StatefulWidget {
  final Map<String, dynamic>? initial;
  final String selection;
  final bool Function() isCurrent;
  final Future<Map<String, dynamic>> Function(String) onCapture;
  final Future<void> Function(Map<String, dynamic>) onSave;
  const ScenarioEditor({
    super.key,
    this.initial,
    required this.selection,
    required this.isCurrent,
    required this.onCapture,
    required this.onSave,
  });
  @override
  State<ScenarioEditor> createState() => _ScenarioEditorState();
}

class _ScenarioEditorState extends State<ScenarioEditor> {
  late final name = TextEditingController(
    text: widget.initial?['name'] as String? ?? '',
  );
  late final comment = TextEditingController(
    text: widget.initial?['comment'] as String? ?? '',
  );
  late final range = TextEditingController(
    text: widget.initial == null
        ? widget.selection
        : (widget.initial!['inputs'] as List)
              .map((i) => i['address'])
              .join(', '),
  );
  final List<(TextEditingController, TextEditingController)> inputs = [];
  late bool locked = widget.initial?['locked'] as bool? ?? true,
      hidden = widget.initial?['hidden'] as bool? ?? false;
  bool busy = false;
  String? error;
  @override
  void initState() {
    super.initState();
    if (widget.initial != null) {
      load(widget.initial!['inputs'] as List);
    }
  }

  void load(List rows) {
    for (final (a, v) in inputs) {
      a.dispose();
      v.dispose();
    }
    inputs.clear();
    for (final row in rows) {
      inputs.add((
        TextEditingController(text: row['address'] as String),
        TextEditingController(text: row['value'] as String),
      ));
    }
  }

  @override
  void dispose() {
    for (final c in [
      name,
      comment,
      range,
      ...inputs.expand((i) => [i.$1, i.$2]),
    ]) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> capture() async {
    setState(() {
      busy = true;
      error = null;
    });
    try {
      if (!widget.isCurrent()) {
        throw StateError('The workbook changed. Reopen Scenario Manager.');
      }
      final result = await widget.onCapture(range.text);
      if (!mounted) return;
      if (!widget.isCurrent()) {
        throw StateError('The workbook changed. Reopen Scenario Manager.');
      }
      setState(() => load(result['inputs'] as List));
    } catch (e) {
      if (mounted) setState(() => error = e.toString());
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  Future<void> save() async {
    if (name.text.trim().isEmpty || inputs.isEmpty || inputs.length > 32) {
      setState(() => error = 'Enter a name and load 1–32 changing cells.');
      return;
    }
    setState(() {
      busy = true;
      error = null;
    });
    try {
      if (!widget.isCurrent()) {
        throw StateError('The workbook changed. Reopen Scenario Manager.');
      }
      await widget.onSave({
        'name': name.text.trim(),
        'comment': comment.text,
        'locked': locked,
        'hidden': hidden,
        'inputs': inputs
            .map((i) => {'address': i.$1.text, 'value': i.$2.text})
            .toList(),
      });
      if (mounted) Navigator.pop(context, true);
    } catch (e) {
      if (mounted) setState(() => error = e.toString());
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !busy,
    child: AlertDialog(
      title: Text(widget.initial == null ? 'Add scenario' : 'Edit scenario'),
      content: SizedBox(
        width: 570,
        height: 510,
        child: Column(
          children: [
            draftTextField(
              key: const Key('scenario-name'),
              controller: name,
              enabled: !busy,
              maxLength: 255,
              decoration: const InputDecoration(
                labelText: 'Scenario name',
                counterText: '',
              ),
            ),
            draftTextField(
              key: const Key('scenario-comment'),
              controller: comment,
              enabled: !busy,
              maxLength: 255,
              decoration: const InputDecoration(
                labelText: 'Comment',
                counterText: '',
              ),
            ),
            Row(
              children: [
                Expanded(
                  child: draftTextField(
                    key: const Key('scenario-range'),
                    controller: range,
                    enabled: !busy,
                    decoration: const InputDecoration(
                      labelText: 'Changing cells',
                      hintText: 'B2:B4, D5',
                    ),
                  ),
                ),
                TextButton(
                  key: const Key('scenario-load-values'),
                  onPressed: busy ? null : capture,
                  child: const Text('Load current values'),
                ),
              ],
            ),
            const SizedBox(height: 8),
            const Text(
              'Edit the saved values below. Saving the scenario does not change the worksheet.',
            ),
            const SizedBox(height: 8),
            Expanded(
              child: inputs.isEmpty
                  ? const Center(
                      child: Text(
                        'Load the changing cells to set their values.',
                      ),
                    )
                  : ListView.builder(
                      itemCount: inputs.length,
                      itemBuilder: (_, i) => Row(
                        children: [
                          SizedBox(
                            width: 110,
                            child: draftTextField(
                              key: Key('scenario-address-$i'),
                              controller: inputs[i].$1,
                              enabled: !busy,
                              decoration: const InputDecoration(
                                labelText: 'Cell',
                              ),
                            ),
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: draftTextField(
                              key: Key('scenario-value-$i'),
                              controller: inputs[i].$2,
                              enabled: !busy,
                              maxLength: 255,
                              decoration: const InputDecoration(
                                labelText: 'Value',
                                counterText: '',
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
            ),
            Row(
              children: [
                Checkbox(
                  value: locked,
                  onChanged: busy ? null : (v) => setState(() => locked = v!),
                ),
                const Expanded(child: Text('Prevent changes when protected')),
                Checkbox(
                  value: hidden,
                  onChanged: busy ? null : (v) => setState(() => hidden = v!),
                ),
                const Expanded(child: Text('Hide when protected')),
              ],
            ),
            if (error != null)
              Text(
                error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: busy ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('scenario-save'),
          onPressed: busy ? null : save,
          child: Text(busy ? 'Saving…' : 'Save scenario'),
        ),
      ],
    ),
  );
}

class ScenarioMergeDialog extends StatefulWidget {
  final List sheets;
  final String targetId;
  final Future<void> Function(String, List<int>) onMerge;
  final Future<Map<String, dynamic>?> Function()? onBrowse;
  final Future<void> Function(int, String, List<int>)? onExternalMerge;
  final bool Function()? isCurrent;
  const ScenarioMergeDialog({
    super.key,
    required this.sheets,
    required this.targetId,
    required this.onMerge,
    this.onBrowse,
    this.onExternalMerge,
    this.isCurrent,
  });
  @override
  State<ScenarioMergeDialog> createState() => _ScenarioMergeDialogState();
}

class _ScenarioMergeDialogState extends State<ScenarioMergeDialog> {
  String? source;
  Map<String, dynamic>? external;
  final selected = <int>{};
  bool busy = false;
  String? error;
  List get choices => (external?['sheets'] as List? ?? widget.sheets)
      .where(
        (s) =>
            (external != null || s['id'] != widget.targetId) &&
            (s['scenarios']?['items'] as List? ?? []).isNotEmpty,
      )
      .toList();
  List get scenarios => source == null
      ? []
      : choices.firstWhere((s) => s['id'] == source)['scenarios']['items']
            as List;
  bool get current => widget.isCurrent?.call() ?? true;
  Future<void> browse() async {
    if (!current) {
      setState(
        () =>
            error = 'The workbook changed. Close and reopen Scenario Manager.',
      );
      return;
    }
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final next = await widget.onBrowse!();
      if (!mounted) return;
      if (!current) {
        throw StateError('The workbook changed. Reopen Scenario Manager.');
      }
      if (next != null) {
        setState(() {
          external = next;
          source = null;
          selected.clear();
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          external = null;
          source = null;
          selected.clear();
          error = e.toString();
        });
      }
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  Future<void> merge() async {
    setState(() {
      busy = true;
      error = null;
    });
    try {
      if (!current) {
        throw StateError('The workbook changed. Reopen Scenario Manager.');
      }
      final indices = selected.toList()..sort();
      if (external == null) {
        await widget.onMerge(source!, indices);
      } else {
        await widget.onExternalMerge!(
          external!['token'] as int,
          source!,
          indices,
        );
      }
      if (mounted) Navigator.pop(context, true);
    } catch (e) {
      if (mounted) setState(() => error = e.toString());
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !busy,
    child: AlertDialog(
      title: const Text('Merge scenarios'),
      content: SizedBox(
        width: 500,
        height: 410,
        child: Column(
          children: [
            const Text(
              'Copy scenarios from a worksheet or another workbook. Changing cell addresses stay the same, so source and destination should use the same layout. Duplicate names get a numbered suffix.',
            ),
            if (widget.onBrowse != null)
              Row(
                children: [
                  Expanded(
                    child: Text(
                      external?['name'] as String? ?? 'This workbook',
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  if (external != null)
                    TextButton(
                      onPressed: busy
                          ? null
                          : () => setState(() {
                              external = null;
                              source = null;
                              selected.clear();
                              error = null;
                            }),
                      child: const Text('This workbook'),
                    ),
                  TextButton(
                    key: const Key('scenario-browse-source'),
                    onPressed: busy ? null : browse,
                    child: Text(busy ? 'Working…' : 'Choose workbook…'),
                  ),
                ],
              ),
            DropdownButton<String>(
              isExpanded: true,
              value: source,
              hint: const Text('Source worksheet'),
              items: choices
                  .map(
                    (s) => DropdownMenuItem(
                      value: s['id'] as String,
                      enabled: s['unavailableReason'] == null,
                      child: Text(
                        '${s['name']}${s['unavailableReason'] == null ? '' : ' (unavailable)'}',
                      ),
                    ),
                  )
                  .toList(),
              onChanged: busy
                  ? null
                  : (v) => setState(() {
                      source = v;
                      selected.clear();
                    }),
            ),
            Expanded(
              child: choices.isEmpty
                  ? const Center(
                      child: Text('No available worksheets contain scenarios.'),
                    )
                  : ListView.builder(
                      itemCount: scenarios.length,
                      itemBuilder: (_, i) => CheckboxListTile(
                        title: Text(scenarios[i]['name'] as String),
                        subtitle: scenarios[i]['unsupported'] == true
                            ? const Text(
                                'This scenario needs additional file support.',
                              )
                            : null,
                        value: selected.contains(i),
                        onChanged: busy || scenarios[i]['unsupported'] == true
                            ? null
                            : (v) => setState(() {
                                if (v!) {
                                  selected.add(i);
                                } else {
                                  selected.remove(i);
                                }
                              }),
                      ),
                    ),
            ),
            if (error != null)
              Text(
                error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: busy ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: busy || selected.isEmpty ? null : merge,
          child: const Text('Copy scenarios'),
        ),
      ],
    ),
  );
}

class ScenarioSummaryDialog extends StatefulWidget {
  final String initial;
  final Future<void> Function(String) onCreate;
  const ScenarioSummaryDialog({
    super.key,
    required this.initial,
    required this.onCreate,
  });
  @override
  State<ScenarioSummaryDialog> createState() => _ScenarioSummaryDialogState();
}

class _ScenarioSummaryDialogState extends State<ScenarioSummaryDialog> {
  late final result = TextEditingController(text: widget.initial);
  bool busy = false;
  String? error;
  @override
  void dispose() {
    result.dispose();
    super.dispose();
  }

  Future<void> create() async {
    setState(() {
      busy = true;
      error = null;
    });
    try {
      await widget.onCreate(result.text);
      if (mounted) Navigator.pop(context, true);
    } catch (e) {
      if (mounted) setState(() => error = e.toString());
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !busy,
    child: AlertDialog(
      title: const Text('Scenario Summary'),
      content: SizedBox(
        width: 480,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text(
              'Create a new worksheet comparing the current values and all scenarios. The report is a static snapshot; create another to reflect later edits.',
            ),
            draftTextField(
              key: const Key('scenario-result-cells'),
              controller: result,
              enabled: !busy,
              decoration: const InputDecoration(
                labelText: 'Result cells',
                hintText: 'C1, E4:E6',
              ),
            ),
            if (error != null)
              Text(
                error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: busy ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('scenario-create-summary'),
          onPressed: busy ? null : create,
          child: Text(busy ? 'Calculating…' : 'Create summary'),
        ),
      ],
    ),
  );
}
