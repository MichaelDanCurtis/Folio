import 'package:flutter/material.dart';

/// Plain-language revision markup is shared by the document and review window.
class RevisionPreview extends StatelessWidget {
  final List spans;
  final bool selectable;
  const RevisionPreview({
    super.key,
    required this.spans,
    this.selectable = true,
  });
  @override
  Widget build(BuildContext context) {
    final text = TextSpan(
      style: const TextStyle(
        fontSize: 15,
        height: 1.6,
        color: Color(0xff242424),
      ),
      children: spans
          .map(
            (s) => TextSpan(
              text: s['text'] as String? ?? '',
              style: TextStyle(
                color: switch (s['kind']) {
                  'insert' => const Color(0xff107c41),
                  'delete' => const Color(0xffb32727),
                  _ => null,
                },
                decoration: switch (s['kind']) {
                  'insert' => TextDecoration.underline,
                  'delete' => TextDecoration.lineThrough,
                  _ => TextDecoration.none,
                },
              ),
            ),
          )
          .toList(),
    );
    return selectable ? SelectableText.rich(text) : Text.rich(text);
  }
}

class DocumentRevisionsDialog extends StatefulWidget {
  final Map data;
  final Future<Map> Function(Map<String, dynamic>) onChange;
  final Future<Map> Function(bool redo) onHistory;
  const DocumentRevisionsDialog({
    super.key,
    required this.data,
    required this.onChange,
    required this.onHistory,
  });
  @override
  State<DocumentRevisionsDialog> createState() =>
      _DocumentRevisionsDialogState();
}

class _DocumentRevisionsDialogState extends State<DocumentRevisionsDialog> {
  late Map data = widget.data;
  String? author, kind, story, selected, error;
  bool busy = false;
  List<Map> get records => (data['revisions'] as List? ?? []).cast<Map>();
  List<Map> get shown => records
      .where(
        (r) =>
            (author == null || r['author'] == author) &&
            (kind == null || r['kind'] == kind) &&
            (story == null || r['story'] == story),
      )
      .toList();
  Map? get current =>
      shown.where((r) => r['key'] == selected).firstOrNull ?? shown.firstOrNull;
  bool can(List<Map> rows, bool accept) =>
      !busy &&
      rows.isNotEmpty &&
      rows.every((r) => r[accept ? 'canAccept' : 'canReject'] == true);
  Future<void> run(Future<Map> Function() action) async {
    if (busy) return;
    final index = shown.indexWhere((r) => r['key'] == current?['key']);
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final next = await action();
      if (!mounted) return;
      setState(() {
        data = next;
        if (author != null && !records.any((r) => r['author'] == author)) {
          author = null;
        }
        if (kind != null && !records.any((r) => r['kind'] == kind)) kind = null;
        if (story != null && !records.any((r) => r['story'] == story)) {
          story = null;
        }
        final rows = shown;
        selected = rows.isEmpty
            ? null
            : rows[index.clamp(0, rows.length - 1)]['key'];
      });
    } catch (e) {
      if (mounted) setState(() => error = e.toString());
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  Future<void> review(List<Map> rows, bool accept) async {
    if (!can(rows, accept)) return;
    await run(
      () => widget.onChange({
        'action': accept ? 'accept' : 'reject',
        'keys': rows.map((r) => r['key']).toList(),
      }),
    );
  }

  Widget filter(
    String field,
    String label,
    String? value,
    ValueChanged<String?> change,
  ) {
    final values = records.map((r) => r[field] as String).toSet().toList()
      ..sort();
    return SizedBox(
      width: 185,
      child: DropdownButtonFormField<String>(
        key: ValueKey('revision-filter-$field-$value'),
        initialValue: value ?? '\u0000',
        isExpanded: true,
        decoration: InputDecoration(
          labelText: label,
          isDense: true,
          border: const OutlineInputBorder(),
        ),
        items: [
          DropdownMenuItem(
            value: '\u0000',
            child: Text('All ${label.toLowerCase()}'),
          ),
          ...values.map(
            (v) => DropdownMenuItem(
              value: v,
              child: Text(
                v.isEmpty ? 'Unknown author' : v,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ),
        ],
        onChanged: busy
            ? null
            : (v) => setState(() {
                change(v == '\u0000' ? null : v);
                selected = null;
              }),
      ),
    );
  }

  Widget changeList() => ListView.separated(
    key: const Key('revision-list'),
    itemCount: shown.length,
    separatorBuilder: (_, _) => const Divider(height: 1),
    itemBuilder: (_, i) {
      final r = shown[i];
      final excerpt = (r['change'] as List? ?? []).map((s) => s['text']).join();
      return ListTile(
        key: ValueKey('revision-row-$i'),
        selected: r['key'] == current?['key'],
        selectedTileColor: const Color(0xffedf3fc),
        title: Text(
          r['kind'],
          style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
        ),
        subtitle: Text(
          '${r['author'].toString().isEmpty ? 'Unknown author' : r['author']} · ${r['story']}${excerpt.isEmpty ? '' : '\n$excerpt'}',
          maxLines: 3,
          overflow: TextOverflow.ellipsis,
        ),
        trailing: r['canAccept'] != true || r['canReject'] != true
            ? const Tooltip(
                message: 'Some review actions need additional support',
                child: Icon(Icons.info_outline, size: 17),
              )
            : null,
        onTap: busy ? null : () => setState(() => selected = r['key']),
      );
    },
  );
  Widget details(Map r) => SingleChildScrollView(
    padding: const EdgeInsets.all(20),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(r['kind'], style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 6),
        Text(
          '${r['author'].toString().isEmpty ? 'Unknown author' : r['author']} · ${r['story']}',
          style: const TextStyle(color: Color(0xff616161)),
        ),
        if (r['date'].toString().isNotEmpty)
          Text(
            r['date'],
            style: const TextStyle(fontSize: 12, color: Color(0xff616161)),
          ),
        const SizedBox(height: 20),
        if ((r['preview'] as List).isEmpty)
          const Text('This change affects document formatting or structure.')
        else
          RevisionPreview(spans: r['preview']),
        const SizedBox(height: 16),
        if (r['move'] is Map) ...[
          const Text(
            'Accept or reject resolves the linked source and destination together.',
          ),
          TextButton.icon(
            key: const Key('revision-linked-location'),
            onPressed: busy
                ? null
                : () => setState(() {
                    author = null;
                    kind = null;
                    story = null;
                    selected = r['move']['counterpartKey'];
                  }),
            icon: const Icon(Icons.swap_horiz, size: 18),
            label: const Text('Go to linked location'),
          ),
        ],
        if ((r['formatChanges'] as List? ?? []).isNotEmpty) ...[
          const Text(
            'Formatting changes',
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 8),
          for (final change in r['formatChanges'])
            Padding(
              padding: const EdgeInsets.only(bottom: 10),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    change['property'],
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  Text('Previous: ${change['before']}'),
                  Text('Current: ${change['after']}'),
                ],
              ),
            ),
        ],
        if (r['containsChanges'] == true)
          const Text(
            'This change contains other tracked edits. Removing its content also removes those edits.',
            style: TextStyle(fontSize: 12),
          ),
        if (r['canAccept'] != true)
          Padding(
            padding: const EdgeInsets.only(top: 12),
            child: Text('Accept unavailable: ${r['acceptReason']}'),
          ),
        if (r['canReject'] != true)
          Padding(
            padding: const EdgeInsets.only(top: 12),
            child: Text('Reject unavailable: ${r['rejectReason']}'),
          ),
        const SizedBox(height: 16),
        Wrap(
          spacing: 8,
          children: [
            FilledButton.icon(
              key: const Key('revision-accept'),
              onPressed: can([r], true) ? () => review([r], true) : null,
              icon: const Icon(Icons.check, size: 17),
              label: const Text('Accept'),
            ),
            OutlinedButton.icon(
              key: const Key('revision-reject'),
              onPressed: can([r], false) ? () => review([r], false) : null,
              icon: const Icon(Icons.close, size: 17),
              label: const Text('Reject'),
            ),
          ],
        ),
      ],
    ),
  );
  @override
  Widget build(BuildContext context) {
    final rows = shown, r = current;
    final index = rows.indexWhere((v) => v['key'] == r?['key']);
    return PopScope(
      canPop: !busy,
      child: Dialog(
        insetPadding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 1050, maxHeight: 730),
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    const Expanded(
                      child: Text(
                        'Track Changes',
                        style: TextStyle(
                          fontSize: 23,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    IconButton(
                      tooltip: 'Undo review',
                      onPressed: !busy && data['canUndo'] == true
                          ? () => run(() => widget.onHistory(false))
                          : null,
                      icon: const Icon(Icons.undo),
                    ),
                    IconButton(
                      tooltip: 'Redo review',
                      onPressed: !busy && data['canRedo'] == true
                          ? () => run(() => widget.onHistory(true))
                          : null,
                      icon: const Icon(Icons.redo),
                    ),
                    IconButton(
                      key: const Key('revision-close'),
                      tooltip: 'Close',
                      onPressed: busy ? null : () => Navigator.pop(context),
                      icon: const Icon(Icons.close),
                    ),
                  ],
                ),
                Text(
                  data['tracking']?['enabled'] == true
                      ? 'Recording changes as ${data['tracking']['author']}. Review changes below.'
                      : 'Recording is off. Use Record changes to track new edits.',
                  style: const TextStyle(color: Color(0xff616161)),
                ),
                const SizedBox(height: 14),
                Wrap(
                  spacing: 10,
                  runSpacing: 10,
                  children: [
                    filter('author', 'Authors', author, (v) => author = v),
                    filter('kind', 'Types', kind, (v) => kind = v),
                    filter('story', 'Locations', story, (v) => story = v),
                  ],
                ),
                const SizedBox(height: 12),
                Text(
                  '${rows.length} of ${records.length} changes · Green underline: insertion · Red strike-through: deletion',
                  style: const TextStyle(fontSize: 12),
                ),
                const SizedBox(height: 10),
                if (rows.any((r) => r['move'] is Map))
                  const Padding(
                    padding: EdgeInsets.only(bottom: 8),
                    child: Text(
                      'Moves include both linked locations, even when one is filtered out.',
                      style: TextStyle(fontSize: 12),
                    ),
                  ),
                if (error != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 10),
                    child: Text(
                      error!,
                      key: const Key('revision-error'),
                      style: const TextStyle(color: Colors.red),
                    ),
                  ),
                if (busy) const LinearProgressIndicator(),
                Expanded(
                  child: rows.isEmpty
                      ? Center(
                          child: Text(
                            records.isEmpty
                                ? 'No tracked changes in this document.'
                                : 'No changes match these filters.',
                          ),
                        )
                      : LayoutBuilder(
                          builder: (context, c) {
                            if (c.maxWidth < 700) {
                              return Column(
                                children: [
                                  SizedBox(height: 150, child: changeList()),
                                  const Divider(height: 1),
                                  Expanded(child: details(r!)),
                                ],
                              );
                            }
                            return Row(
                              crossAxisAlignment: CrossAxisAlignment.stretch,
                              children: [
                                SizedBox(width: 290, child: changeList()),
                                const VerticalDivider(width: 1),
                                Expanded(child: details(r!)),
                              ],
                            );
                          },
                        ),
                ),
                const Divider(),
                Wrap(
                  spacing: 8,
                  runSpacing: 4,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    IconButton(
                      tooltip: 'Previous change',
                      onPressed: !busy && index > 0
                          ? () => setState(
                              () => selected = rows[index - 1]['key'],
                            )
                          : null,
                      icon: const Icon(Icons.chevron_left),
                    ),
                    Text(
                      rows.isEmpty ? '0 / 0' : '${index + 1} / ${rows.length}',
                    ),
                    IconButton(
                      tooltip: 'Next change',
                      onPressed: !busy && index >= 0 && index < rows.length - 1
                          ? () => setState(
                              () => selected = rows[index + 1]['key'],
                            )
                          : null,
                      icon: const Icon(Icons.chevron_right),
                    ),
                    OutlinedButton(
                      key: const Key('revision-accept-shown'),
                      onPressed: can(rows, true)
                          ? () => review(rows, true)
                          : null,
                      child: Text('Accept all shown (${rows.length})'),
                    ),
                    OutlinedButton(
                      key: const Key('revision-reject-shown'),
                      onPressed: can(rows, false)
                          ? () => review(rows, false)
                          : null,
                      child: Text('Reject all shown (${rows.length})'),
                    ),
                  ],
                ),
                if (rows.any(
                  (r) => r['canAccept'] != true || r['canReject'] != true,
                ))
                  const Text(
                    'Bulk actions are disabled when any shown change cannot be reviewed.',
                    style: TextStyle(fontSize: 11, color: Color(0xff616161)),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
