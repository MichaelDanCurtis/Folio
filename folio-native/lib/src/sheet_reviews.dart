import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'draft_text_field.dart';
import 'workbook_block.dart' show parseCell;

class NewNoteIntent extends Intent {
  const NewNoteIntent();
}

Map? cellReview(Map sheet, String address) =>
    (sheet['cellReviews'] as Map?)?[address] as Map?;

String reviewTooltip(Map? review) {
  if (review == null) return '';
  final kind = review['kind'] == 'note' ? 'Note' : 'Comment';
  final messages = review['messages'] as List;
  final text = messages.map((m) => '${m['author']}: ${m['text']}').join('\n\n');
  return '$kind${review['resolved'] == true ? ' · Resolved' : ''}\n'
      '${text.length > 1200 ? '${text.substring(0, 1200)}…' : text}';
}

/// The marker does not intercept selection, editing, or filter-button clicks.
class SheetReviewMarker extends StatelessWidget {
  final Map? review;
  final Widget child;
  const SheetReviewMarker({
    super.key,
    required this.review,
    required this.child,
  });
  @override
  Widget build(BuildContext context) => review == null
      ? child
      : Semantics(
          label: review!['kind'] == 'note'
              ? 'Cell has a note'
              : 'Cell has a ${review!['resolved'] == true ? 'resolved ' : ''}comment',
          child: CustomPaint(
            foregroundPainter: _Corner(
              review!['kind'] == 'note'
                  ? const Color(0xffc43131)
                  : const Color(0xff8064a2),
            ),
            child: child,
          ),
        );
}

class _Corner extends CustomPainter {
  final Color color;
  const _Corner(this.color);
  @override
  void paint(Canvas canvas, Size size) {
    final side = size.shortestSide.clamp(0.0, 7.0);
    canvas.drawPath(
      Path()
        ..moveTo(size.width - side, 0)
        ..lineTo(size.width, 0)
        ..lineTo(size.width, side)
        ..close(),
      Paint()..color = color,
    );
  }

  @override
  bool shouldRepaint(_Corner oldDelegate) => color != oldDelegate.color;
}

class SheetReviewsDialog extends StatefulWidget {
  final Map workbook;
  final String sheetId, address;
  final String? newKind;
  final Future<Map> Function(Map<String, dynamic>) onChange;
  final Future<Map> Function(bool redo) onHistory;
  const SheetReviewsDialog({
    super.key,
    required this.workbook,
    required this.sheetId,
    required this.address,
    required this.onChange,
    required this.onHistory,
    this.newKind,
  });
  @override
  State<SheetReviewsDialog> createState() => _SheetReviewsDialogState();
}

class _SheetReviewsDialogState extends State<SheetReviewsDialog> {
  late Map workbook = widget.workbook;
  final author = TextEditingController(), draft = TextEditingController();
  String? selected, adding, editing, error;
  late String sheetFilter = widget.sheetId;
  String filter = 'All', draftBaseline = '';
  bool get hasDraft => draft.text.isNotEmpty && draft.text != draftBaseline;
  bool busy = false;
  List<Map> get sheets => (workbook['sheets'] as List).cast<Map>();
  List<Map> get entries {
    final result = <Map>[];
    for (final sheet in sheets) {
      final reviews = (sheet['cellReviews'] as Map? ?? {}).entries.toList()
        ..sort((a, b) {
          final x = parseCell(a.key), y = parseCell(b.key);
          return x.$1 != y.$1 ? x.$1.compareTo(y.$1) : x.$2.compareTo(y.$2);
        });
      for (final entry in reviews) {
        result.add({
          ...entry.value as Map,
          'address': entry.key,
          'sheetId': sheet['id'],
          'sheetName': sheet['name'],
        });
      }
    }
    return result;
  }

  bool visible(Map t) =>
      (sheetFilter == '*' || t['sheetId'] == sheetFilter) &&
      switch (filter) {
        'Comments' => t['kind'] == 'comment',
        'Notes' => t['kind'] == 'note',
        'Active' => t['kind'] == 'comment' && t['resolved'] != true,
        'Resolved' => t['kind'] == 'comment' && t['resolved'] == true,
        _ => true,
      };
  Map? get current =>
      entries.where((e) => e['id'] == selected && visible(e)).firstOrNull;
  Map? get targetSheet =>
      sheets.where((s) => s['id'] == widget.sheetId).firstOrNull;
  bool get canAdd =>
      targetSheet != null && cellReview(targetSheet!, widget.address) == null;

  @override
  void initState() {
    super.initState();
    selected = targetSheet == null
        ? null
        : cellReview(targetSheet!, widget.address)?['id'];
    selected ??= entries.where(visible).firstOrNull?['id'];
    if (canAdd) adding = widget.newKind;
    if (widget.newKind == 'note' &&
        current?['kind'] == 'note' &&
        current?['messages'][0]['readOnly'] != true) {
      editing = current!['messages'][0]['id'];
      draft.text = current!['messages'][0]['text'];
      draftBaseline = draft.text;
    }
  }

  @override
  void dispose() {
    author.dispose();
    draft.dispose();
    super.dispose();
  }

  Future<bool> leaveDraft() async {
    if (!hasDraft) return true;
    return await showDialog<bool>(
          context: context,
          builder: (context) => AlertDialog(
            title: const Text('Discard this draft?'),
            content: const Text(
              'Your changes to this comment or note have not been saved.',
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context, false),
                child: const Text('Keep writing'),
              ),
              FilledButton(
                onPressed: () => Navigator.pop(context, true),
                child: const Text('Discard'),
              ),
            ],
          ),
        ) ??
        false;
  }

  void clearDraft() {
    adding = null;
    editing = null;
    draft.clear();
    draftBaseline = '';
    error = null;
  }

  Future<void> navigate(VoidCallback action) async {
    if (busy || !await leaveDraft() || !mounted) return;
    setState(() {
      clearDraft();
      action();
    });
  }

  Future<void> close([Map? target]) async {
    if (busy || !await leaveDraft() || !mounted) return;
    setState(clearDraft);
    await WidgetsBinding.instance.endOfFrame;
    if (mounted) Navigator.pop(context, target);
  }

  Future<void> change(Map<String, dynamic> request) async {
    if (busy) return;
    if (!['add', 'reply', 'edit'].contains(request['action']) &&
        (!await leaveDraft() || !mounted)) {
      return;
    }
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final next = await widget.onChange(request);
      if (!mounted) return;
      setState(() {
        workbook = next;
        clearDraft();
        if (request['action'] == 'add') {
          sheetFilter = request['sheetId'];
          filter = 'All';
          selected = entries
              .where(
                (e) =>
                    e['sheetId'] == request['sheetId'] &&
                    e['address'] == request['address'],
              )
              .firstOrNull?['id'];
        }
        if (current == null) {
          selected = entries.where(visible).firstOrNull?['id'];
        }
      });
    } catch (e) {
      if (mounted) {
        setState(() => error = e.toString().replaceFirst('Bad state: ', ''));
      }
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  Future<void> history(bool redo) async {
    if (busy || !await leaveDraft() || !mounted) return;
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final next = await widget.onHistory(redo);
      if (mounted) {
        setState(() {
          workbook = next;
          clearDraft();
          if (!sheets.any((s) => s['id'] == sheetFilter)) sheetFilter = '*';
          if (current == null) {
            selected = entries.where(visible).firstOrNull?['id'];
          }
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() => error = e.toString().replaceFirst('Bad state: ', ''));
      }
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  bool get canPost =>
      !busy &&
      draft.text.trim().isNotEmpty &&
      (editing != null || author.text.trim().isNotEmpty) &&
      (adding != null && canAdd ||
          current != null &&
              current!['resolved'] != true &&
              (editing != null || current!['kind'] == 'comment'));
  void post() {
    if (!canPost) return;
    final t = current;
    change({
      'action': adding != null
          ? 'add'
          : editing != null
          ? 'edit'
          : 'reply',
      'sheetId': adding != null ? widget.sheetId : t!['sheetId'],
      'address': adding != null ? widget.address : t!['address'],
      'kind': adding ?? t!['kind'],
      'author': author.text.trim(),
      'text': draft.text,
      if (editing != null) 'messageId': editing,
    });
  }

  @override
  Widget build(BuildContext context) {
    final t = current, list = entries.where(visible).toList();
    final composing =
        adding != null ||
        editing != null ||
        t != null && t['kind'] == 'comment' && t['resolved'] != true;
    Map<String, dynamic> action(String name) => {
      'action': name,
      'sheetId': t!['sheetId'],
      'address': t['address'],
      'kind': t['kind'],
    };
    return PopScope(
      canPop: !busy && !hasDraft,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop && !busy) close();
      },
      child: CallbackShortcuts(
        bindings: {
          const SingleActivator(LogicalKeyboardKey.enter, control: true): post,
          const SingleActivator(LogicalKeyboardKey.enter, meta: true): post,
        },
        child: AlertDialog(
          title: Row(
            children: [
              const Expanded(child: Text('Comments & notes')),
              IconButton(
                key: const Key('sheet-reviews-undo'),
                tooltip: 'Undo',
                onPressed: busy ? null : () => history(false),
                icon: const Icon(Icons.undo),
              ),
              IconButton(
                key: const Key('sheet-reviews-redo'),
                tooltip: 'Redo',
                onPressed: busy ? null : () => history(true),
                icon: const Icon(Icons.redo),
              ),
            ],
          ),
          content: SizedBox(
            width: 800,
            height: (MediaQuery.sizeOf(context).height - 200).clamp(180, 680),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Wrap(
                  spacing: 10,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    for (final kind in ['comment', 'note'])
                      TextButton.icon(
                        key: Key('sheet-review-new-$kind'),
                        onPressed: busy || !canAdd
                            ? null
                            : () => navigate(() {
                                adding = kind;
                                sheetFilter = widget.sheetId;
                                filter = 'All';
                              }),
                        icon: Icon(
                          kind == 'note'
                              ? Icons.sticky_note_2_outlined
                              : Icons.add_comment_outlined,
                          size: 18,
                        ),
                        label: Text('New $kind at ${widget.address}'),
                      ),
                    DropdownButton<String>(
                      key: const Key('sheet-review-sheet-filter'),
                      value: sheetFilter,
                      items: [
                        const DropdownMenuItem(
                          value: '*',
                          child: Text('All sheets'),
                        ),
                        for (final s in sheets)
                          DropdownMenuItem(
                            value: s['id'],
                            child: Text(s['name']),
                          ),
                      ],
                      onChanged: busy
                          ? null
                          : (v) => navigate(() {
                              sheetFilter = v!;
                              selected = entries
                                  .where(visible)
                                  .firstOrNull?['id'];
                            }),
                    ),
                    DropdownButton<String>(
                      key: const Key('sheet-review-kind-filter'),
                      value: filter,
                      items: ['All', 'Comments', 'Notes', 'Active', 'Resolved']
                          .map(
                            (v) => DropdownMenuItem(value: v, child: Text(v)),
                          )
                          .toList(),
                      onChanged: busy
                          ? null
                          : (v) => navigate(() {
                              filter = v!;
                              selected = entries
                                  .where(visible)
                                  .firstOrNull?['id'];
                            }),
                    ),
                  ],
                ),
                if (list.isNotEmpty)
                  SizedBox(
                    height: 100,
                    child: ListView.builder(
                      itemCount: list.length,
                      itemBuilder: (_, i) {
                        final e = list[i], first = e['messages'][0];
                        return ListTile(
                          dense: true,
                          selected: adding == null && selected == e['id'],
                          key: ValueKey(
                            'sheet-review-${e['sheetId']}-${e['address']}',
                          ),
                          leading: Icon(
                            e['kind'] == 'note'
                                ? Icons.sticky_note_2_outlined
                                : e['resolved'] == true
                                ? Icons.check_circle_outline
                                : Icons.chat_bubble_outline,
                          ),
                          title: Text(
                            '${e['sheetName']}!${e['address']} · ${first['text']}',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          subtitle: Text(
                            '${first['author']} · ${e['kind'] == 'note'
                                ? 'Note'
                                : e['resolved'] == true
                                ? 'Resolved comment'
                                : 'Comment'}',
                          ),
                          onTap: busy
                              ? null
                              : () => navigate(() => selected = e['id']),
                        );
                      },
                    ),
                  ),
                const Divider(),
                Expanded(
                  child: SingleChildScrollView(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        if (adding != null)
                          Text(
                            'New $adding · ${targetSheet?['name']}!${widget.address}',
                            style: const TextStyle(fontWeight: FontWeight.w600),
                          )
                        else if (t == null)
                          const Text(
                            'No matching comments or notes. Select a cell to add one.',
                          )
                        else ...[
                          Text(
                            '${t['sheetName']}!${t['address']}',
                            key: const Key('sheet-review-anchor'),
                            style: const TextStyle(fontWeight: FontWeight.w600),
                          ),
                          Wrap(
                            children: [
                              TextButton(
                                key: const Key('sheet-review-go-to'),
                                onPressed: busy ? null : () => close(t),
                                child: const Text('Go to cell'),
                              ),
                              if (t['kind'] == 'comment')
                                TextButton(
                                  key: const Key('sheet-review-resolve'),
                                  onPressed: busy
                                      ? null
                                      : () => change(
                                          action(
                                            t['resolved'] == true
                                                ? 'reopen'
                                                : 'resolve',
                                          ),
                                        ),
                                  child: Text(
                                    t['resolved'] == true
                                        ? 'Reopen'
                                        : 'Resolve',
                                  ),
                                ),
                              TextButton(
                                key: const Key('sheet-review-delete'),
                                onPressed: busy
                                    ? null
                                    : () => change(action('delete')),
                                child: Text(
                                  t['kind'] == 'note'
                                      ? 'Delete note'
                                      : 'Delete thread',
                                ),
                              ),
                            ],
                          ),
                          for (final (i, m)
                              in (t['messages'] as List).indexed) ...[
                            const Divider(),
                            Wrap(
                              spacing: 16,
                              children: [
                                Text(
                                  m['author'].toString().isEmpty
                                      ? 'Unknown author'
                                      : m['author'],
                                  style: const TextStyle(
                                    fontWeight: FontWeight.w600,
                                  ),
                                ),
                                if ((m['date'] ?? '').toString().isNotEmpty)
                                  Text(
                                    m['date']
                                        .toString()
                                        .replaceFirst('T', ' ')
                                        .replaceFirst('Z', ' UTC'),
                                    style: Theme.of(
                                      context,
                                    ).textTheme.bodySmall,
                                  ),
                              ],
                            ),
                            SelectableText(m['text']),
                            if (m['readOnly'] == true)
                              const Text(
                                'Rich content is preserved. Open in Excel to edit its formatting or mentions.',
                                style: TextStyle(fontSize: 12),
                              ),
                            if (t['resolved'] != true)
                              Wrap(
                                children: [
                                  if (m['readOnly'] != true)
                                    TextButton(
                                      key: Key('sheet-review-edit-$i'),
                                      onPressed: busy
                                          ? null
                                          : () => navigate(() {
                                              editing = m['id'];
                                              draft.text = m['text'];
                                              draftBaseline = draft.text;
                                            }),
                                      child: const Text('Edit text'),
                                    ),
                                  if (i > 0)
                                    TextButton(
                                      key: Key('sheet-review-delete-reply-$i'),
                                      onPressed: busy
                                          ? null
                                          : () => change({
                                              ...action('deleteReply'),
                                              'messageId': m['id'],
                                            }),
                                      child: const Text('Delete reply'),
                                    ),
                                ],
                              ),
                          ],
                          if (t['resolved'] == true)
                            const Text('Resolved. Reopen to reply or edit.'),
                        ],
                        if (composing) ...[
                          const Divider(),
                          if (editing == null)
                            draftTextField(
                              key: const Key('sheet-review-author'),
                              controller: author,
                              enabled: !busy,
                              decoration: const InputDecoration(
                                labelText: 'Your name',
                                helperText:
                                    'Saved as local author attribution. Posting here does not notify anyone.',
                              ),
                              onChanged: (_) => setState(() {}),
                            ),
                          const SizedBox(height: 8),
                          draftTextField(
                            key: const Key('sheet-review-draft'),
                            controller: draft,
                            enabled: !busy,
                            minLines: 2,
                            maxLines: 5,
                            decoration: InputDecoration(
                              labelText: editing != null
                                  ? 'Edit text'
                                  : adding == 'note'
                                  ? 'Note'
                                  : adding == 'comment'
                                  ? 'Comment'
                                  : 'Reply',
                            ),
                            onChanged: (_) => setState(() {}),
                          ),
                          Align(
                            alignment: Alignment.centerRight,
                            child: FilledButton(
                              key: const Key('sheet-review-post'),
                              onPressed: canPost ? post : null,
                              child: Text(
                                editing != null || adding == 'note'
                                    ? 'Save'
                                    : 'Post',
                              ),
                            ),
                          ),
                          if (editing != null)
                            TextButton(
                              onPressed: busy ? null : () => navigate(() {}),
                              child: const Text('Cancel edit'),
                            ),
                        ],
                      ],
                    ),
                  ),
                ),
                if (error != null)
                  Text(
                    error!,
                    key: const Key('sheet-review-error'),
                    style: const TextStyle(color: Colors.red),
                  ),
              ],
            ),
          ),
          actions: [
            TextButton(
              key: const Key('sheet-reviews-close'),
              onPressed: busy ? null : close,
              child: const Text('Close'),
            ),
          ],
        ),
      ),
    );
  }
}
