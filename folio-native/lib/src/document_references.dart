// Captions, cross-references and tables of contents (W10.03–W10.05): the
// dialogs, and Update Fields, which lays the document out for page numbers.
// The engine's `documentReferences` and `documentUpdateFields` ops number
// captions, resolve references and fill tables of contents.
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'core.dart';
import 'document_bookmarks.dart';
import 'document_codec.dart';
import 'document_page_layout.dart';

const captionLabels = ['Figure', 'Table', 'Equation'];

/// The selection's body-story offset, as the engine counts it.
int? storyOffset(q.QuillController controller) => DocumentBookmarkIndex(
  deltaToDocument(controller.document.toDelta()),
).logicalOffset(controller.selection.start);

/// Recalculates captions, cross-references and tables of contents, with page
/// numbers from the page layout.
Future<void> updateFields(WorkspaceModel model, {Locale? locale}) async {
  await model.flush();
  final snapshot = await model.core.call({'op': 'documentPagePreview'});
  final blocks = (snapshot['document']?['content'] as List? ?? []).length;
  final pages = List<String>.filled(blocks, '');
  final layout = DocumentPageLayout.build(snapshot, locale: locale);
  try {
    for (final page in layout.pages) {
      for (final placement in page.content) {
        final i = placement.bodyIndex;
        if (placement.area == 'body' &&
            i != null &&
            i < blocks &&
            pages[i].isEmpty) {
          pages[i] = '${page.number}';
        }
      }
    }
  } finally {
    layout.dispose();
  }
  await model.mutate({
    'op': 'documentUpdateFields',
    'blockPages': pages,
    'baseRevision': model.revision,
  }, refreshDocument: true);
}

Future<void> _references(
  WorkspaceModel model,
  q.QuillController controller,
  Map<String, dynamic> request,
) async {
  await model.flush();
  final offset = storyOffset(controller);
  if (offset == null) {
    throw const FormatException('Place the cursor in the text first.');
  }
  await model.mutate({
    'op': 'documentReferences',
    'offset': offset,
    'baseRevision': model.revision,
    ...request,
  }, refreshDocument: true);
}

Future<void> insertCaption(
  BuildContext context,
  WorkspaceModel model,
  q.QuillController controller,
) async {
  final chosen = await showDialog<(String, String)>(
    context: context,
    builder: (context) => const _CaptionDialog(),
  );
  if (chosen == null) return;
  await _references(model, controller, {
    'action': 'caption',
    'label': chosen.$1,
    'text': chosen.$2,
  });
}

class _CaptionDialog extends StatefulWidget {
  const _CaptionDialog();
  @override
  State<_CaptionDialog> createState() => _CaptionDialogState();
}

class _CaptionDialogState extends State<_CaptionDialog> {
  final text = TextEditingController();
  var label = captionLabels.first;

  @override
  void dispose() {
    text.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Insert Caption'),
    content: SizedBox(
      width: 420,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          DropdownButtonFormField<String>(
            key: const Key('caption-label'),
            initialValue: label,
            decoration: const InputDecoration(labelText: 'Label'),
            items: [
              for (final l in captionLabels)
                DropdownMenuItem(value: l, child: Text(l)),
            ],
            onChanged: (v) => setState(() => label = v ?? label),
          ),
          const SizedBox(height: 12),
          TextField(
            key: const Key('caption-text'),
            controller: text,
            autofocus: true,
            decoration: InputDecoration(
              labelText: 'Caption',
              helperText:
                  'Shown as “$label 1: …”. Numbers update with Update Fields.',
            ),
            onSubmitted: (_) => Navigator.pop(context, (label, text.text)),
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
        key: const Key('caption-insert'),
        onPressed: () => Navigator.pop(context, (label, text.text)),
        child: const Text('Insert'),
      ),
    ],
  );
}

Future<void> insertCrossReference(
  BuildContext context,
  WorkspaceModel model,
  q.QuillController controller,
) async {
  await model.flush();
  final targets = await model.core.call({
    'op': 'documentReferences',
    'action': 'targets',
  });
  if (!context.mounted) return;
  final options = <(String, Map<String, dynamic>)>[
    for (final h in targets['headings'] as List)
      (
        '${'  ' * (((h['level'] as num?)?.toInt() ?? 1) - 1)}${h['text']}',
        {'kind': 'heading', 'index': h['index']},
      ),
  ];
  final kinds = {
    'Heading': options,
    'Caption': [
      for (final c in targets['captions'] as List)
        (c['text'] as String, {'kind': 'caption', 'index': c['index']}),
    ],
    'Bookmark': [
      for (final b in targets['bookmarks'] as List)
        (b as String, {'kind': 'bookmark', 'name': b}),
    ],
  };
  var kind = kinds.entries
      .firstWhere((e) => e.value.isNotEmpty, orElse: () => kinds.entries.first)
      .key;
  var selected = 0;
  var page = false;
  final chosen = await showDialog<Map<String, dynamic>>(
    context: context,
    builder: (context) => StatefulBuilder(
      builder: (context, setState) {
        final items = kinds[kind]!;
        return AlertDialog(
          title: const Text('Cross-reference'),
          content: SizedBox(
            width: 460,
            height: 320,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        key: const Key('xref-kind'),
                        initialValue: kind,
                        decoration: const InputDecoration(
                          labelText: 'Reference type',
                        ),
                        items: [
                          for (final k in kinds.keys)
                            DropdownMenuItem(value: k, child: Text(k)),
                        ],
                        onChanged: (v) => setState(() {
                          kind = v ?? kind;
                          selected = 0;
                        }),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: DropdownButtonFormField<bool>(
                        key: const Key('xref-show'),
                        initialValue: page,
                        decoration: const InputDecoration(
                          labelText: 'Insert reference to',
                        ),
                        items: [
                          DropdownMenuItem(
                            value: false,
                            child: Text(
                              kind == 'Caption'
                                  ? 'Label and number'
                                  : kind == 'Heading'
                                  ? 'Heading text'
                                  : 'Bookmark text',
                            ),
                          ),
                          const DropdownMenuItem(
                            value: true,
                            child: Text('Page number'),
                          ),
                        ],
                        onChanged: (v) => setState(() => page = v ?? false),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                Expanded(
                  child: items.isEmpty
                      ? Center(
                          child: Text(
                            'This document has no ${kind.toLowerCase()}s to refer to.',
                          ),
                        )
                      : ListView(
                          children: [
                            for (var i = 0; i < items.length; i++)
                              ListTile(
                                key: Key('xref-item-$i'),
                                dense: true,
                                selected: i == selected,
                                title: Text(items[i].$1),
                                onTap: () => setState(() => selected = i),
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
              child: const Text('Cancel'),
            ),
            FilledButton(
              key: const Key('xref-insert'),
              onPressed: items.isEmpty
                  ? null
                  : () => Navigator.pop(context, {
                      'target': items[selected].$2,
                      'page': page,
                    }),
              child: const Text('Insert'),
            ),
          ],
        );
      },
    ),
  );
  if (chosen == null) return;
  await _references(model, controller, {'action': 'crossReference', ...chosen});
  // Page numbers need the layout.
  if (chosen['page'] == true) await updateFields(model);
}

Future<void> insertTableOfContents(
  WorkspaceModel model,
  q.QuillController controller, {
  int levels = 3,
}) async {
  await _references(model, controller, {'action': 'toc', 'levels': levels});
  await updateFields(model);
}

/// A table of contents in the editor: its entries and Update Table.
class TableOfContentsView extends StatelessWidget {
  final Map node;
  final VoidCallback? onUpdate;
  const TableOfContentsView({required this.node, this.onUpdate, super.key});

  @override
  Widget build(BuildContext context) {
    final entries = (node['attrs']?['entries'] as List? ?? []).cast<Map>();
    return Container(
      key: const Key('table-of-contents'),
      margin: const EdgeInsets.symmetric(vertical: 8),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        border: Border.all(color: const Color(0xffd7dde5)),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Expanded(
                child: Text(
                  'Contents',
                  style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                ),
              ),
              if (onUpdate != null)
                TextButton.icon(
                  key: const Key('toc-update'),
                  onPressed: onUpdate,
                  icon: const Icon(Icons.refresh, size: 16),
                  label: const Text('Update table'),
                ),
            ],
          ),
          if (entries.isEmpty)
            const Text('No table of contents entries found.')
          else
            for (final e in entries)
              Padding(
                padding: EdgeInsets.only(
                  left: (((e['level'] as num?)?.toInt() ?? 1) - 1) * 18.0,
                  top: 3,
                ),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        '${e['text']}',
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    Text('${e['page']}'),
                  ],
                ),
              ),
        ],
      ),
    );
  }
}

/// A table of contents in the page preview: one paragraph of entries with
/// right-aligned, dot-led page numbers.
Map<String, dynamic> tableOfContentsPreview(Map node) {
  final content = <Map<String, dynamic>>[];
  for (final e in (node['attrs']?['entries'] as List? ?? []).cast<Map>()) {
    if (content.isNotEmpty) {
      content.add({
        'type': 'hardBreak',
        'attrs': {'breakType': 'textWrapping'},
      });
    }
    final indent = '    ' * (((e['level'] as num?)?.toInt() ?? 1) - 1);
    content.add({'type': 'text', 'text': '$indent${e['text']}\t${e['page']}'});
  }
  if (content.isEmpty) {
    content.add({
      'type': 'text',
      'text': 'No table of contents entries found.',
    });
  }
  return {
    'type': 'paragraph',
    'attrs': {
      'tabs': {
        'inherited': [],
        'direct': [
          {'position': 9350, 'alignment': 'right', 'leader': 'dot'},
        ],
      },
    },
    'content': content,
  };
}
