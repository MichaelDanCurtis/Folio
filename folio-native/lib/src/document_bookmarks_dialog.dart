import 'package:flutter/material.dart';
import 'document_bookmarks.dart';
import 'draft_text_field.dart';

class DocumentBookmarksDialog extends StatefulWidget {
  final Map document;
  final TextSelection? selection;
  final Future<Map> Function(Map<String, dynamic>) onChange;
  const DocumentBookmarksDialog({
    super.key,
    required this.document,
    required this.selection,
    required this.onChange,
  });
  @override
  State<DocumentBookmarksDialog> createState() =>
      _DocumentBookmarksDialogState();
}

class _DocumentBookmarksDialogState extends State<DocumentBookmarksDialog> {
  late Map document = widget.document;
  final name = TextEditingController();
  bool byLocation = false, hidden = false, busy = false;
  String? selected, error;
  @override
  void dispose() {
    name.dispose();
    super.dispose();
  }

  List<Map<String, dynamic>> get bookmarks {
    final items = documentBookmarks(
      document,
    ).where((b) => hidden || !(b['name'] as String).startsWith('_')).toList();
    items.sort((a, b) {
      final location = (a['start'] as int).compareTo(b['start']);
      return byLocation && location != 0
          ? location
          : (a['name'] as String).toLowerCase().compareTo(
              (b['name'] as String).toLowerCase(),
            );
    });
    return items;
  }

  Map<String, dynamic>? get current => documentBookmarks(
    document,
  ).where((b) => b['name'] == selected).firstOrNull;
  Future<void> change(bool delete) async {
    if (busy) return;
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final next = await widget.onChange({
        'action': delete ? 'delete' : 'set',
        'name': delete ? selected : name.text.trim(),
        if (!delete) 'start': widget.selection?.start,
        if (!delete) 'end': widget.selection?.end,
      });
      if (!mounted) return;
      setState(() {
        document = next;
        selected = delete ? null : name.text.trim();
      });
    } catch (e) {
      if (mounted) {
        setState(() => error = e.toString().replaceFirst('Bad state: ', ''));
      }
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final index = DocumentBookmarkIndex(document);
    final target = current;
    final exists = documentBookmarks(document).any(
      (b) =>
          (b['name'] as String).toLowerCase() == name.text.trim().toLowerCase(),
    );
    return PopScope(
      canPop: !busy,
      child: AlertDialog(
        title: const Text('Bookmarks'),
        content: SizedBox(
          width: 540,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                draftTextField(
                  key: const Key('bookmark-name'),
                  controller: name,
                  autofocus: true,
                  enabled: !busy,
                  decoration: const InputDecoration(
                    labelText: 'Bookmark name',
                    helperText:
                        'Start with a letter. Use letters, numbers or underscores.',
                  ),
                  onChanged: (_) => setState(() {}),
                  onSubmitted: (_) {
                    if (widget.selection != null &&
                        name.text.trim().isNotEmpty) {
                      change(false);
                    }
                  },
                ),
                const SizedBox(height: 10),
                Wrap(
                  crossAxisAlignment: WrapCrossAlignment.center,
                  spacing: 12,
                  children: [
                    const Text('Sort by'),
                    ChoiceChip(
                      label: const Text('Name'),
                      selected: !byLocation,
                      onSelected: busy
                          ? null
                          : (_) => setState(() => byLocation = false),
                    ),
                    ChoiceChip(
                      label: const Text('Location'),
                      selected: byLocation,
                      onSelected: busy
                          ? null
                          : (_) => setState(() => byLocation = true),
                    ),
                    FilterChip(
                      key: const Key('bookmark-show-hidden'),
                      label: const Text('Hidden bookmarks'),
                      selected: hidden,
                      onSelected: busy
                          ? null
                          : (v) => setState(() {
                              hidden = v;
                              if (!v && (selected?.startsWith('_') ?? false)) {
                                selected = null;
                              }
                            }),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                Container(
                  height: 180,
                  decoration: BoxDecoration(
                    border: Border.all(color: const Color(0xffdfe3e8)),
                  ),
                  child: bookmarks.isEmpty
                      ? const Center(child: Text('No bookmarks to show.'))
                      : ListView(
                          children: [
                            for (final b in bookmarks)
                              ListTile(
                                key: ValueKey('bookmark-${b['name']}'),
                                dense: true,
                                selected: selected == b['name'],
                                title: Text(b['name']),
                                subtitle: Text(
                                  index.paragraph(b['start'])?.label ??
                                      'Document',
                                ),
                                onTap: busy
                                    ? null
                                    : () => setState(() {
                                        selected = b['name'];
                                        name.text = b['name'];
                                        error = null;
                                      }),
                              ),
                          ],
                        ),
                ),
                const SizedBox(height: 12),
                if (target != null) ...[
                  Text(
                    target['start'] == target['end']
                        ? 'Saved insertion point'
                        : 'Bookmarked text',
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    index.excerpt(target['start'], target['end']),
                    maxLines: 4,
                    overflow: TextOverflow.ellipsis,
                  ),
                ] else if (widget.selection != null)
                  Text(
                    widget.selection!.isCollapsed
                        ? 'Add a bookmark at the current insertion point.'
                        : 'Add a bookmark around the selected text.',
                  ),
                if (widget.selection == null)
                  const Text(
                    'Select document text or place the cursor in a paragraph to add a bookmark.',
                  ),
                if (exists && widget.selection != null)
                  const Padding(
                    padding: EdgeInsets.only(top: 8),
                    child: Text(
                      'Move updates this bookmark to the current document selection.',
                    ),
                  ),
                if (error != null)
                  Padding(
                    padding: const EdgeInsets.only(top: 10),
                    child: Text(
                      error!,
                      key: const Key('bookmark-error'),
                      style: const TextStyle(color: Colors.red),
                    ),
                  ),
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('bookmark-delete'),
            onPressed: !busy && current != null ? () => change(true) : null,
            child: const Text('Delete'),
          ),
          TextButton(
            key: const Key('bookmark-add'),
            onPressed:
                !busy && widget.selection != null && name.text.trim().isNotEmpty
                ? () => change(false)
                : null,
            child: Text(exists ? 'Move' : 'Add'),
          ),
          FilledButton(
            key: const Key('bookmark-go-to'),
            onPressed: !busy && current != null
                ? () => Navigator.pop(context, current!['name'])
                : null,
            child: const Text('Go To'),
          ),
          TextButton(
            onPressed: busy ? null : () => Navigator.pop(context),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }
}
