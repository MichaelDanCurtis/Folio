// The Name Manager (X09.04, X09.05): the workbook's defined names, with
// their values, and dialogs to create, edit and delete them. The engine's
// `nameManager` op validates each change and writes it into the workbook.
import 'package:flutter/material.dart';
import 'core.dart';

Future<void> showNameManager(
  BuildContext context,
  WorkspaceModel model, {
  required String activeSheetId,
}) => showDialog<void>(
  context: context,
  builder: (context) =>
      NameManagerDialog(model: model, activeSheetId: activeSheetId),
);

class NameManagerDialog extends StatefulWidget {
  final WorkspaceModel model;
  final String activeSheetId;
  const NameManagerDialog({
    required this.model,
    required this.activeSheetId,
    super.key,
  });

  @override
  State<NameManagerDialog> createState() => _NameManagerDialogState();
}

class _NameManagerDialogState extends State<NameManagerDialog> {
  List<Map> names = [];
  Map? selected;
  String? error;
  bool loading = true;

  @override
  void initState() {
    super.initState();
    load();
  }

  Future<void> load() async {
    final result = await widget.model.core.call({
      'op': 'nameManager',
      'action': 'list',
    });
    if (!mounted) return;
    setState(() {
      names = (result['names'] as List).cast<Map>()
        ..sort(
          (a, b) => (a['name'] as String).toLowerCase().compareTo(
            (b['name'] as String).toLowerCase(),
          ),
        );
      selected = names.where((n) => same(n, selected)).firstOrNull;
      loading = false;
    });
  }

  static bool same(Map a, Map? b) =>
      b != null && a['name'] == b['name'] && a['sheetId'] == b['sheetId'];

  Future<String?> change(Map<String, dynamic> request) async {
    try {
      await widget.model.mutate({
        'op': 'nameManager',
        'baseRevision': widget.model.revision,
        'activeSheetId': widget.activeSheetId,
        ...request,
      });
      await load();
      return null;
    } catch (e) {
      return e.toString().replaceFirst(RegExp(r'^[A-Za-z]*Exception: '), '');
    }
  }

  Future<void> editName([Map? existing]) async {
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (context) => NameEditDialog(
        sheets: widget.model.sheets.cast<Map>(),
        existing: existing,
        activeSheetId: widget.activeSheetId,
        save: (request) => change({
          if (existing == null)
            'action': 'add'
          else ...{
            'action': 'edit',
            'oldName': existing['name'],
            'oldSheetId': existing['sheetId'],
          },
          ...request,
        }),
      ),
    );
    if (result != null && mounted) {
      setState(
        () => selected = names.where((n) => same(n, result)).firstOrNull,
      );
    }
  }

  Future<void> delete(Map name) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete name?'),
        content: Text(
          'Formulas that use ${name['name']} will show #NAME? until it is defined again.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('name-delete-confirm'),
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    final problem = await change({
      'action': 'delete',
      'oldName': name['name'],
      'oldSheetId': name['sheetId'],
    });
    if (mounted) setState(() => error = problem);
  }

  String valueText(Object? value) => switch (value) {
    null => '',
    num n when n == n.roundToDouble() && n.abs() < 1e15 => n.toInt().toString(),
    final v => v.toString(),
  };

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    Widget cell(String text, double width, {bool bold = false}) => SizedBox(
      width: width,
      child: Text(
        text,
        overflow: TextOverflow.ellipsis,
        style: TextStyle(fontWeight: bold ? FontWeight.w600 : null),
      ),
    );
    return AlertDialog(
      title: const Text('Name Manager'),
      content: SizedBox(
        width: 640,
        height: 360,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                FilledButton.tonal(
                  key: const Key('name-new'),
                  onPressed: () => editName(),
                  child: const Text('New…'),
                ),
                const SizedBox(width: 8),
                OutlinedButton(
                  key: const Key('name-edit'),
                  onPressed: selected == null ? null : () => editName(selected),
                  child: const Text('Edit…'),
                ),
                const SizedBox(width: 8),
                OutlinedButton(
                  key: const Key('name-delete'),
                  onPressed: selected == null ? null : () => delete(selected!),
                  child: const Text('Delete'),
                ),
              ],
            ),
            if (error != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  error!,
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              ),
            const SizedBox(height: 12),
            DefaultTextStyle.merge(
              style: const TextStyle(fontSize: 13),
              child: Row(
                children: [
                  cell('Name', 150, bold: true),
                  cell('Value', 120, bold: true),
                  cell('Refers to', 250, bold: true),
                  cell('Scope', 100, bold: true),
                ],
              ),
            ),
            const Divider(height: 12),
            Expanded(
              child: loading
                  ? const SizedBox.shrink()
                  : names.isEmpty
                  ? const Center(
                      child: Text(
                        'No names yet. Choose New… to name a cell, a range, a value or a LAMBDA.',
                      ),
                    )
                  : ListView(
                      children: [
                        for (final n in names)
                          Material(
                            color: same(n, selected)
                                ? theme.colorScheme.primaryContainer
                                : Colors.transparent,
                            child: InkWell(
                              key: Key(
                                'name-row-${n['name']}-${n['sheetId'] ?? 'workbook'}',
                              ),
                              onTap: () => setState(() => selected = n),
                              onDoubleTap: () => editName(n),
                              child: Padding(
                                padding: const EdgeInsets.symmetric(
                                  vertical: 6,
                                ),
                                child: DefaultTextStyle.merge(
                                  style: const TextStyle(fontSize: 13),
                                  child: Semantics(
                                    selected: same(n, selected),
                                    label:
                                        '${n['name']}, refers to ${n['refersTo']}, scope ${n['scope']}',
                                    child: ExcludeSemantics(
                                      child: Row(
                                        children: [
                                          cell(n['name'] as String, 150),
                                          cell(valueText(n['value']), 120),
                                          cell(n['refersTo'] as String, 250),
                                          cell('${n['scope']}', 100),
                                        ],
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ),
                          ),
                      ],
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

/// Creates or edits one name. `save` returns an error message, or null
/// when the engine accepted the change.
class NameEditDialog extends StatefulWidget {
  final List<Map> sheets;
  final Map? existing;
  final String activeSheetId;
  final Future<String?> Function(Map<String, dynamic>) save;
  const NameEditDialog({
    required this.sheets,
    required this.save,
    required this.activeSheetId,
    this.existing,
    super.key,
  });

  @override
  State<NameEditDialog> createState() => _NameEditDialogState();
}

class _NameEditDialogState extends State<NameEditDialog> {
  late final name = TextEditingController(
    text: widget.existing?['name'] as String? ?? '',
  );
  late final refersTo = TextEditingController(
    text: widget.existing?['refersTo'] as String? ?? '=',
  );
  late String? scope = widget.existing?['sheetId'] as String?;
  String? error;
  bool saving = false;

  @override
  void dispose() {
    name.dispose();
    refersTo.dispose();
    super.dispose();
  }

  Future<void> submit() async {
    setState(() => saving = true);
    final problem = await widget.save({
      'name': name.text.trim(),
      'refersTo': refersTo.text.trim(),
      'sheetId': scope,
    });
    if (!mounted) return;
    if (problem == null) {
      Navigator.pop(context, {'name': name.text.trim(), 'sheetId': scope});
    } else {
      setState(() {
        error = problem;
        saving = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.existing == null ? 'New Name' : 'Edit Name'),
      content: SizedBox(
        width: 420,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            TextField(
              key: const Key('name-field'),
              controller: name,
              autofocus: true,
              decoration: const InputDecoration(labelText: 'Name'),
            ),
            const SizedBox(height: 12),
            DropdownButtonFormField<String?>(
              key: const Key('name-scope'),
              initialValue: scope,
              decoration: const InputDecoration(labelText: 'Scope'),
              items: [
                const DropdownMenuItem(value: null, child: Text('Workbook')),
                for (final s in widget.sheets)
                  DropdownMenuItem(
                    value: s['id'] as String,
                    child: Text(s['name'] as String),
                  ),
              ],
              onChanged: (v) => setState(() => scope = v),
            ),
            const SizedBox(height: 12),
            TextField(
              key: const Key('name-refers-to'),
              controller: refersTo,
              style: const TextStyle(fontFamily: 'Menlo', fontSize: 13),
              decoration: const InputDecoration(
                labelText: 'Refers to',
                helperText:
                    'A reference, a value, a formula or a LAMBDA, such as =LAMBDA(x, x*2)',
              ),
              onSubmitted: (_) => saving ? null : submit(),
            ),
            if (error != null)
              Padding(
                padding: const EdgeInsets.only(top: 12),
                child: Text(
                  error!,
                  key: const Key('name-error'),
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
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
          key: const Key('name-save'),
          onPressed: saving ? null : submit,
          child: const Text('OK'),
        ),
      ],
    );
  }
}
