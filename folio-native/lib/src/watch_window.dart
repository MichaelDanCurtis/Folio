import 'package:flutter/material.dart';
import 'workbook_block.dart' show parseCell;

class AddWatchDialog extends StatefulWidget {
  final List sheets;
  final String sheetId, range;
  final Future<void> Function(String sheetId, String range) onAdd;
  const AddWatchDialog({
    super.key,
    required this.sheets,
    required this.sheetId,
    required this.range,
    required this.onAdd,
  });
  @override
  State<AddWatchDialog> createState() => _AddWatchDialogState();
}

class _AddWatchDialogState extends State<AddWatchDialog> {
  late String sheetId = widget.sheetId;
  late final range = TextEditingController(text: widget.range);
  bool busy = false;
  String? error;
  @override
  void dispose() {
    range.dispose();
    super.dispose();
  }

  Future<void> add() async {
    if (busy) return;
    setState(() {
      busy = true;
      error = null;
    });
    try {
      await widget.onAdd(sheetId, range.text.trim());
      if (mounted) Navigator.pop(context, true);
    } catch (e) {
      if (mounted) setState(() => error = '$e');
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !busy,
    child: AlertDialog(
      key: const Key('watch-add-dialog'),
      title: const Text('Add watch'),
      content: SizedBox(
        width: 420,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            DropdownButtonFormField<String>(
              key: const Key('watch-sheet'),
              initialValue: sheetId,
              decoration: const InputDecoration(labelText: 'Worksheet'),
              items: [
                for (final s in widget.sheets)
                  DropdownMenuItem(
                    value: s['id'] as String,
                    child: Text('${s['name']}'),
                  ),
              ],
              onChanged: busy ? null : (v) => setState(() => sheetId = v!),
            ),
            const SizedBox(height: 12),
            TextField(
              key: const Key('watch-range'),
              controller: range,
              enabled: !busy,
              autofocus: true,
              decoration: const InputDecoration(
                labelText: 'Cells to watch',
                hintText: 'A1 or A1:B5',
              ),
              onSubmitted: (_) => add(),
            ),
            const SizedBox(height: 12),
            const Text('One watch per cell. Existing watches are kept.'),
            if (error != null) Text(error!, key: const Key('watch-add-error')),
            if (busy) const LinearProgressIndicator(),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: busy ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('watch-add-confirm'),
          onPressed: busy ? null : add,
          child: const Text('Add'),
        ),
      ],
    ),
  );
}

class WatchWindow extends StatefulWidget {
  final Map workbook, calculated, formatted;
  final int documentEpoch;
  final VoidCallback onAdd, onClose;
  final Future<void> Function(List<Map<String, dynamic>>) onRemove;
  final void Function(String sheetId, String address) onOpen, onInspect;
  const WatchWindow({
    super.key,
    required this.workbook,
    required this.calculated,
    required this.formatted,
    required this.documentEpoch,
    required this.onAdd,
    required this.onClose,
    required this.onRemove,
    required this.onOpen,
    required this.onInspect,
  });
  @override
  State<WatchWindow> createState() => _WatchWindowState();
}

class _WatchWindowState extends State<WatchWindow> {
  final selected = <(String, String)>{};
  final widths = <double>[160, 130, 90, 130, 290];
  List<(Map, String)> watches = [];
  double height = 240;
  bool busy = false;
  String? error;
  @override
  void initState() {
    super.initState();
    rebuild();
  }

  @override
  void didUpdateWidget(covariant WatchWindow old) {
    super.didUpdateWidget(old);
    if (old.workbook['id'] != widget.workbook['id'] ||
        old.documentEpoch != widget.documentEpoch) {
      selected.clear();
      error = null;
    }
    if (old.workbook['sheets'] != widget.workbook['sheets']) rebuild();
  }

  void rebuild() {
    watches = [
      for (final s in widget.workbook['sheets'] as List)
        for (final a in (s['cellWatches'] as Map? ?? {}).keys)
          (s as Map, a as String),
    ];
    final order = {
      for (final (i, s) in (widget.workbook['sheets'] as List).indexed)
        s['id']: i,
    };
    watches.sort((a, b) {
      var n = order[a.$1['id']]!.compareTo(order[b.$1['id']]!);
      if (n != 0) return n;
      final ac = parseCell(a.$2), bc = parseCell(b.$2);
      n = ac.$1.compareTo(bc.$1);
      return n == 0 ? ac.$2.compareTo(bc.$2) : n;
    });
    final existing = {for (final (s, a) in watches) (s['id'] as String, a)};
    selected.removeWhere((cell) => !existing.contains(cell));
  }

  Future<void> remove() async {
    if (busy || selected.isEmpty) return;
    final scope = (widget.workbook['id'], widget.documentEpoch);
    final cells = [
      for (final (id, a) in selected) {'sheetId': id, 'address': a},
    ];
    setState(() {
      busy = true;
      error = null;
    });
    try {
      await widget.onRemove(cells);
      if (mounted && scope == (widget.workbook['id'], widget.documentEpoch)) {
        setState(() => selected.clear());
      }
    } catch (e) {
      if (mounted && scope == (widget.workbook['id'], widget.documentEpoch)) {
        setState(() => error = '$e');
      }
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  Widget textCell(String text, int column, {Color? color}) => SizedBox(
    width: widths[column],
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      child: Tooltip(
        message: text,
        child: Text(
          text,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(fontSize: 12, color: color),
        ),
      ),
    ),
  );
  @override
  Widget build(BuildContext context) {
    final unresolved = (widget.workbook['sheets'] as List)
        .where((s) => s['unsupportedCellWatches'] == true)
        .length;
    final count = selected.length;
    return Container(
      key: const Key('watch-window'),
      height: height.clamp(130, MediaQuery.sizeOf(context).height / 2),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        border: const Border(top: BorderSide(color: Color(0xffb8c1c8))),
      ),
      child: Column(
        children: [
          MouseRegion(
            cursor: SystemMouseCursors.resizeUpDown,
            child: GestureDetector(
              key: const Key('watch-resize'),
              behavior: HitTestBehavior.opaque,
              onVerticalDragUpdate: (d) => setState(
                () => height = (height - d.delta.dy).clamp(130, 450),
              ),
              child: const SizedBox(
                height: 8,
                width: double.infinity,
                child: Center(child: Icon(Icons.drag_handle, size: 10)),
              ),
            ),
          ),
          SizedBox(
            height: 36,
            child: Row(
              children: [
                const SizedBox(width: 12),
                Text(
                  'Watch Window (${watches.length})',
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                const SizedBox(width: 12),
                TextButton.icon(
                  key: const Key('watch-add'),
                  onPressed: busy ? null : widget.onAdd,
                  icon: const Icon(Icons.add, size: 16),
                  label: const Text('Add watch'),
                ),
                TextButton.icon(
                  key: const Key('watch-remove'),
                  onPressed: !busy && count > 0 ? remove : null,
                  icon: const Icon(Icons.remove, size: 16),
                  label: Text(
                    count > 0 ? 'Delete watch ($count)' : 'Delete watch',
                  ),
                ),
                const Spacer(),
                IconButton(
                  tooltip: 'Close Watch Window',
                  onPressed: widget.onClose,
                  icon: const Icon(Icons.close, size: 17),
                ),
              ],
            ),
          ),
          if (error != null) Text(error!, key: const Key('watch-error')),
          if (unresolved > 0)
            const Text(
              'Some saved watches could not be displayed. Their source metadata is preserved.',
              style: TextStyle(fontSize: 11),
            ),
          if (busy) const LinearProgressIndicator(),
          Expanded(
            child: watches.isEmpty
                ? const Center(
                    child: Text(
                      'Select cells in the worksheet, then choose Add watch.',
                    ),
                  )
                : SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: SizedBox(
                      width: 42 + widths.fold(0.0, (a, b) => a + b) + 112,
                      child: Column(
                        children: [
                          SizedBox(
                            height: 28,
                            child: Row(
                              children: [
                                SizedBox(
                                  width: 42,
                                  child: Checkbox(
                                    key: const Key('watch-select-all'),
                                    tristate: true,
                                    value: count == 0
                                        ? false
                                        : count == watches.length
                                        ? true
                                        : null,
                                    onChanged: busy
                                        ? null
                                        : (_) => setState(() {
                                            if (count == watches.length) {
                                              selected.clear();
                                            } else {
                                              selected.addAll(
                                                watches.map(
                                                  (v) => (
                                                    v.$1['id'] as String,
                                                    v.$2,
                                                  ),
                                                ),
                                              );
                                            }
                                          }),
                                  ),
                                ),
                                for (final (i, name) in [
                                  'Workbook',
                                  'Sheet',
                                  'Cell',
                                  'Value',
                                  'Formula',
                                ].indexed)
                                  SizedBox(
                                    width: widths[i],
                                    child: Row(
                                      children: [
                                        Expanded(
                                          child: Padding(
                                            padding: const EdgeInsets.only(
                                              left: 8,
                                            ),
                                            child: Text(
                                              name,
                                              style: const TextStyle(
                                                fontSize: 11,
                                                fontWeight: FontWeight.w600,
                                              ),
                                            ),
                                          ),
                                        ),
                                        MouseRegion(
                                          cursor: SystemMouseCursors
                                              .resizeLeftRight,
                                          child: GestureDetector(
                                            key: ValueKey('watch-column-$i'),
                                            behavior: HitTestBehavior.opaque,
                                            onHorizontalDragUpdate: (d) =>
                                                setState(
                                                  () => widths[i] =
                                                      (widths[i] + d.delta.dx)
                                                          .clamp(65, 700),
                                                ),
                                            child: const SizedBox(
                                              width: 8,
                                              height: 28,
                                              child: VerticalDivider(width: 1),
                                            ),
                                          ),
                                        ),
                                      ],
                                    ),
                                  ),
                                const SizedBox(width: 112),
                              ],
                            ),
                          ),
                          const Divider(height: 1),
                          Expanded(
                            child: ListView.builder(
                              key: const Key('watch-list'),
                              itemExtent: 34,
                              itemCount: watches.length,
                              itemBuilder: (_, i) {
                                final (sheet, a) = watches[i];
                                final id = sheet['id'] as String;
                                final raw = '${sheet['cells'][a] ?? ''}';
                                final display =
                                    widget.formatted[id]?[a] as Map?;
                                final value =
                                    '${display?['text'] ?? widget.calculated[id]?[a] ?? ''}';
                                final checked = selected.contains((id, a));
                                return Container(
                                  key: ValueKey('watch-row-$id-$a'),
                                  color: checked
                                      ? Theme.of(context)
                                            .colorScheme
                                            .primaryContainer
                                            .withValues(alpha: 0.5)
                                      : null,
                                  child: Row(
                                    children: [
                                      SizedBox(
                                        width: 42,
                                        child: Checkbox(
                                          key: ValueKey('watch-check-$id-$a'),
                                          value: checked,
                                          onChanged: busy
                                              ? null
                                              : (v) => setState(() {
                                                  if (v == true) {
                                                    selected.add((id, a));
                                                  } else {
                                                    selected.remove((id, a));
                                                  }
                                                }),
                                        ),
                                      ),
                                      GestureDetector(
                                        key: ValueKey('watch-details-$id-$a'),
                                        onDoubleTap: () => widget.onOpen(id, a),
                                        child: Row(
                                          children: [
                                            textCell(
                                              '${widget.workbook['title']}',
                                              0,
                                            ),
                                            textCell('${sheet['name']}', 1),
                                            textCell(a, 2),
                                            textCell(
                                              value.isEmpty ? '(blank)' : value,
                                              3,
                                              color: display?['isError'] == true
                                                  ? Theme.of(
                                                      context,
                                                    ).colorScheme.error
                                                  : null,
                                            ),
                                            textCell(
                                              raw.startsWith('=') ? raw : '',
                                              4,
                                            ),
                                          ],
                                        ),
                                      ),
                                      IconButton(
                                        key: ValueKey('watch-open-$id-$a'),
                                        tooltip: 'Go to $a in ${sheet['name']}',
                                        onPressed: () => widget.onOpen(id, a),
                                        icon: const Icon(
                                          Icons.open_in_new,
                                          size: 16,
                                        ),
                                      ),
                                      IconButton(
                                        key: ValueKey('watch-inspect-$id-$a'),
                                        tooltip: 'Inspect formula in $a',
                                        onPressed: () =>
                                            widget.onInspect(id, a),
                                        icon: const Icon(
                                          Icons.account_tree_outlined,
                                          size: 16,
                                        ),
                                      ),
                                    ],
                                  ),
                                );
                              },
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
          ),
        ],
      ),
    );
  }
}
