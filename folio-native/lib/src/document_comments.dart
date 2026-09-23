import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'document_bookmarks.dart';
import 'draft_text_field.dart';

class NewCommentIntent extends Intent {
  const NewCommentIntent();
}

const commentShortcuts = <ShortcutActivator, Intent>{
  SingleActivator(LogicalKeyboardKey.keyA, meta: true, alt: true):
      NewCommentIntent(),
  SingleActivator(LogicalKeyboardKey.keyM, control: true, alt: true):
      NewCommentIntent(),
};
List<Map<String, dynamic>> documentComments(Map document) =>
    (document['attrs']?['comments'] as List? ?? [])
        .map((t) => Map<String, dynamic>.from(t))
        .toList();

class DocumentCommentsDialog extends StatefulWidget {
  final Map document;
  final TextSelection? selection;
  final bool newComment;
  final Future<Map> Function(Map<String, dynamic>) onChange;
  final Future<Map> Function(bool redo) onHistory;
  const DocumentCommentsDialog({
    super.key,
    required this.document,
    required this.selection,
    required this.onChange,
    required this.onHistory,
    this.newComment = false,
  });
  @override
  State<DocumentCommentsDialog> createState() => _DocumentCommentsDialogState();
}

class _DocumentCommentsDialogState extends State<DocumentCommentsDialog> {
  late Map document = widget.document;
  final author = TextEditingController(), draft = TextEditingController();
  String? selected, editing, error;
  String filter = 'All';
  bool adding = false, busy = false, selectionValid = true;
  @override
  void initState() {
    super.initState();
    adding = widget.newComment && widget.selection != null;
    final threads = documentComments(document);
    selected =
        threads
            .where(
              (t) =>
                  widget.selection != null &&
                  t['start'] <= widget.selection!.start &&
                  t['end'] >= widget.selection!.end,
            )
            .firstOrNull?['id'] ??
        threads.firstOrNull?['id'];
  }

  @override
  void dispose() {
    author.dispose();
    draft.dispose();
    super.dispose();
  }

  Map<String, dynamic>? get current =>
      documentComments(document).where((t) => t['id'] == selected).firstOrNull;
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
        document = next;
        if ([
          'delete',
          'deleteReply',
          'resolve',
          'reopen',
        ].contains(request['action'])) {
          draft.clear();
          editing = null;
        }
        if (request['action'] == 'add') {
          selected = documentComments(next).last['id'];
          filter = 'All';
        }
        if (current == null) {
          selected = documentComments(next).firstOrNull?['id'];
        }
        if (['add', 'reply', 'edit'].contains(request['action'])) {
          draft.clear();
          editing = null;
          adding = false;
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
          selectionValid =
              selectionValid &&
              DocumentBookmarkIndex(document).text ==
                  DocumentBookmarkIndex(next).text;
          document = next;
          draft.clear();
          editing = null;
          adding = false;
          if (current == null) {
            selected = documentComments(next).firstOrNull?['id'];
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

  Future<bool> leaveDraft() async {
    if (draft.text.isEmpty) return true;
    return await showDialog<bool>(
          context: context,
          builder: (context) => AlertDialog(
            title: const Text('Discard this draft?'),
            content: const Text('Your unposted comment has not been saved.'),
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

  Future<void> select(String? id, {bool add = false}) async {
    if (!await leaveDraft() || !mounted) return;
    setState(() {
      selected = id;
      adding = add;
      editing = null;
      draft.clear();
      error = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final index = DocumentBookmarkIndex(document);
    final thread =
        current != null &&
            (filter == 'All' ||
                (current!['resolved'] == true) == (filter == 'Resolved'))
        ? current
        : null;
    final threads =
        documentComments(document)
            .where(
              (t) =>
                  filter == 'All' ||
                  (t['resolved'] == true) == (filter == 'Resolved'),
            )
            .toList()
          ..sort((a, b) => (a['start'] as int).compareTo(b['start'] as int));
    final composing = adding || thread != null && thread['resolved'] != true;
    return PopScope(
      canPop: !busy && draft.text.isEmpty,
      child: AlertDialog(
        title: Row(
          children: [
            const Expanded(child: Text('Comments')),
            IconButton(
              key: const Key('comments-undo'),
              tooltip: 'Undo',
              onPressed: busy ? null : () => history(false),
              icon: const Icon(Icons.undo),
            ),
            IconButton(
              key: const Key('comments-redo'),
              tooltip: 'Redo',
              onPressed: busy ? null : () => history(true),
              icon: const Icon(Icons.redo),
            ),
          ],
        ),
        content: SizedBox(
          width: 760,
          height: (MediaQuery.sizeOf(context).height - 230).clamp(260, 640),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Wrap(
                spacing: 12,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  FilledButton.tonalIcon(
                    key: const Key('comments-new'),
                    onPressed:
                        busy || widget.selection == null || !selectionValid
                        ? null
                        : () => select(null, add: true),
                    icon: const Icon(Icons.add_comment_outlined),
                    label: const Text('New comment'),
                  ),
                  DropdownButton<String>(
                    value: filter,
                    items: ['All', 'Active', 'Resolved']
                        .map((v) => DropdownMenuItem(value: v, child: Text(v)))
                        .toList(),
                    onChanged: busy
                        ? null
                        : (v) async {
                            if (!await leaveDraft() || !mounted) return;
                            setState(() {
                              filter = v!;
                              adding = false;
                              editing = null;
                              draft.clear();
                              final visible = documentComments(document).where(
                                (t) =>
                                    filter == 'All' ||
                                    (t['resolved'] == true) ==
                                        (filter == 'Resolved'),
                              );
                              if (!visible.any((t) => t['id'] == selected)) {
                                selected = visible.firstOrNull?['id'];
                              }
                            });
                          },
                  ),
                  Text(
                    '${documentComments(document).length} ${documentComments(document).length == 1 ? 'thread' : 'threads'}',
                  ),
                ],
              ),
              if (threads.isNotEmpty)
                SizedBox(
                  height: 115,
                  child: ListView.builder(
                    itemCount: threads.length,
                    itemBuilder: (context, i) {
                      final t = threads[i], first = t['messages'][0] as Map;
                      return ListTile(
                        dense: true,
                        selected: !adding && selected == t['id'],
                        key: ValueKey('comment-thread-${t['id']}'),
                        leading: Icon(
                          t['resolved'] == true
                              ? Icons.check_circle_outline
                              : Icons.chat_bubble_outline,
                        ),
                        title: Text(
                          first['text'],
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        subtitle: Text(
                          '${first['author']} · ${index.excerpt(t['start'], t['end'])}',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        onTap: busy ? null : () => select(t['id']),
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
                      if (adding) ...[
                        const Text(
                          'New comment',
                          style: TextStyle(fontWeight: FontWeight.w600),
                        ),
                        Text(
                          index.excerpt(
                            widget.selection!.start,
                            widget.selection!.end,
                          ),
                          key: const Key('comment-anchor'),
                        ),
                      ] else if (thread != null) ...[
                        Text(
                          index.excerpt(thread['start'], thread['end']),
                          key: const Key('comment-anchor'),
                          style: const TextStyle(fontStyle: FontStyle.italic),
                        ),
                        Wrap(
                          spacing: 4,
                          children: [
                            TextButton(
                              key: const Key('comments-go-to'),
                              onPressed: busy
                                  ? null
                                  : () async {
                                      if (await leaveDraft() &&
                                          context.mounted) {
                                        draft.clear();
                                        setState(() {});
                                        await WidgetsBinding
                                            .instance
                                            .endOfFrame;
                                        if (context.mounted) {
                                          Navigator.pop(context, thread['id']);
                                        }
                                      }
                                    },
                              child: const Text('Go to text'),
                            ),
                            TextButton(
                              key: const Key('comments-resolve'),
                              onPressed: busy
                                  ? null
                                  : () => change({
                                      'action': thread['resolved'] == true
                                          ? 'reopen'
                                          : 'resolve',
                                      'id': selected,
                                    }),
                              child: Text(
                                thread['resolved'] == true
                                    ? 'Reopen'
                                    : 'Resolve',
                              ),
                            ),
                            TextButton(
                              key: const Key('comments-delete'),
                              onPressed: busy
                                  ? null
                                  : () => change({
                                      'action': 'delete',
                                      'id': selected,
                                    }),
                              child: const Text('Delete thread'),
                            ),
                          ],
                        ),
                        for (final (i, raw)
                            in (thread['messages'] as List).indexed) ...[
                          const Divider(),
                          Row(
                            children: [
                              Expanded(
                                child: Text(
                                  raw['author'].toString().isEmpty
                                      ? 'Unknown author'
                                      : raw['author'],
                                  style: const TextStyle(
                                    fontWeight: FontWeight.w600,
                                  ),
                                ),
                              ),
                              Text(
                                raw['date']
                                    .toString()
                                    .replaceFirst('T', ' ')
                                    .replaceFirst('Z', ' UTC'),
                                style: Theme.of(context).textTheme.bodySmall,
                              ),
                            ],
                          ),
                          SelectableText(raw['text']),
                          if (raw['readOnly'] == true)
                            const Text(
                              'Rich comment content is preserved. Open in Word to edit its formatting.',
                              style: TextStyle(fontSize: 12),
                            ),
                          if (thread['resolved'] != true)
                            Wrap(
                              children: [
                                if (raw['readOnly'] != true)
                                  TextButton(
                                    key: ValueKey('comment-edit-$i'),
                                    onPressed: busy
                                        ? null
                                        : () async {
                                            if (!await leaveDraft() ||
                                                !mounted) {
                                              return;
                                            }
                                            setState(() {
                                              editing = raw['id'];
                                              draft.text = raw['text'];
                                            });
                                          },
                                    child: const Text('Edit text'),
                                  ),
                                if (i > 0)
                                  TextButton(
                                    key: ValueKey('comment-delete-reply-$i'),
                                    onPressed: busy
                                        ? null
                                        : () => change({
                                            'action': 'deleteReply',
                                            'id': selected,
                                            'messageId': raw['id'],
                                          }),
                                    child: const Text('Delete reply'),
                                  ),
                              ],
                            ),
                        ],
                        if (thread['resolved'] == true)
                          const Text('Resolved. Reopen to reply or edit.'),
                      ] else
                        const Padding(
                          padding: EdgeInsets.symmetric(vertical: 18),
                          child: Text(
                            'No matching comments. Select text or place the cursor in a paragraph, then choose New comment.',
                          ),
                        ),
                      if (composing) ...[
                        const Divider(),
                        if (editing == null)
                          draftTextField(
                            key: const Key('comment-author'),
                            controller: author,
                            enabled: !busy,
                            decoration: const InputDecoration(
                              labelText: 'Your name',
                              helperText:
                                  'This name will be saved with your comment.',
                            ),
                            onChanged: (_) => setState(() {}),
                          ),
                        const SizedBox(height: 8),
                        draftTextField(
                          key: const Key('comment-draft'),
                          controller: draft,
                          enabled: !busy,
                          minLines: 2,
                          maxLines: 4,
                          decoration: InputDecoration(
                            labelText: editing != null
                                ? 'Edit comment'
                                : adding
                                ? 'Comment'
                                : 'Reply',
                          ),
                          onChanged: (_) => setState(() {}),
                        ),
                        Align(
                          alignment: Alignment.centerRight,
                          child: FilledButton(
                            key: const Key('comment-post'),
                            onPressed:
                                busy ||
                                    draft.text.trim().isEmpty ||
                                    editing == null &&
                                        author.text.trim().isEmpty
                                ? null
                                : () => change({
                                    'action': editing != null
                                        ? 'edit'
                                        : adding
                                        ? 'add'
                                        : 'reply',
                                    'id': selected,
                                    'messageId': editing,
                                    'author': author.text,
                                    'text': draft.text,
                                    if (adding)
                                      'start': widget.selection!.start,
                                    if (adding) 'end': widget.selection!.end,
                                  }),
                            child: Text(
                              editing != null
                                  ? 'Save comment'
                                  : adding
                                  ? 'Post comment'
                                  : 'Post reply',
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
              ),
              if (error != null)
                Text(
                  error!,
                  key: const Key('comment-error'),
                  style: const TextStyle(color: Colors.red),
                ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: busy
                ? null
                : () async {
                    if (await leaveDraft() && context.mounted) {
                      draft.clear();
                      setState(() {});
                      await WidgetsBinding.instance.endOfFrame;
                      if (context.mounted) Navigator.pop(context);
                    }
                  },
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }
}
