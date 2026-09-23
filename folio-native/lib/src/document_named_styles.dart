import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'document_bookmarks.dart';
import 'document_text_style.dart';
import 'document_paragraph_dialog.dart';
import 'draft_text_field.dart';

Map<String, dynamic> styleCopy(Map value) =>
    Map<String, dynamic>.from(jsonDecode(jsonEncode(value)));
List<List<int>> selectedStylePaths(Map document, TextSelection selection) {
  final index = DocumentBookmarkIndex(document);
  final selected = index.paragraphs
      .where(
        (p) => selection.isCollapsed
            ? !p.embedded &&
                  p.editorOffset <= selection.start &&
                  p.editorOffset + p.text.length >= selection.start
            : p.editorOffset < selection.end &&
                  p.editorOffset + (p.embedded ? 1 : p.text.length) >=
                      selection.start,
      )
      .toList();
  if (selected.isEmpty) {
    throw const FormatException(
      'Select text, or open a table cell, to choose a style.',
    );
  }
  return [for (final p in selected) p.path];
}

Map nodeAtStylePath(Map doc, List<int> path) {
  var node = doc;
  for (final index in path) {
    node = node['content'][index] as Map;
  }
  return node;
}

String? selectionStyleId(Map document, List<List<int>> paths) {
  final ids = paths.map((p) {
    final n = nodeAtStylePath(document, p);
    return n['attrs']?['paragraphFormat']?['styleId'] ??
        (n['type'] == 'heading'
            ? 'Heading${n['attrs']?['level'] ?? 1}'
            : document['attrs']?['wordStyles']?['defaultStyle'] ?? 'Normal');
  }).toSet();
  return ids.length == 1 ? ids.single as String? : null;
}

class DocumentStylesDialog extends StatefulWidget {
  final Map data;
  final String? currentId;
  final int paragraphs;
  final bool manage;
  const DocumentStylesDialog({
    required this.data,
    required this.currentId,
    required this.paragraphs,
    this.manage = true,
    super.key,
  });
  @override
  State<DocumentStylesDialog> createState() => _DocumentStylesDialogState();
}

class _DocumentStylesDialogState extends State<DocumentStylesDialog> {
  late final catalog = widget.data['catalog'] as Map;
  late final availableIds = {
    for (final s in widget.data['availableStyles'] as List? ?? []) s['id'],
  };
  late final styles =
      [
        ...catalog['styles'] as List,
        ...widget.data['availableStyles'] as List? ?? [],
      ].where((s) => s['type'] == 'paragraph').cast<Map>().toList()..sort(
        (a, b) => (a['name'] as String).toLowerCase().compareTo(
          (b['name'] as String).toLowerCase(),
        ),
      );
  late String? selected = styles.any((s) => s['id'] == widget.currentId)
      ? widget.currentId
      : null;
  String query = '';
  late final scroll = ScrollController(
    initialScrollOffset:
        styles.indexWhere((s) => s['id'] == selected).clamp(0, styles.length) *
        70.0,
  );
  List<Map> get shown => styles
      .where(
        (s) =>
            (s['name'] as String).toLowerCase().contains(query.toLowerCase()),
      )
      .toList();
  @override
  void dispose() {
    scroll.dispose();
    super.dispose();
  }

  Map? get current => styles.where((s) => s['id'] == selected).firstOrNull;
  Future<void> edit(bool create) async {
    final seed = create
        ? <String, dynamic>{
            'id': 'FolioStyle${DateTime.now().microsecondsSinceEpoch}',
            'name': '',
            'type': 'paragraph',
            if (selected != null && !availableIds.contains(selected))
              'basedOn': selected,
            'next':
                catalog['defaultStyle'] ??
                (availableIds.contains(selected) ? null : selected),
            'properties': {'paragraph': {}, 'tabs': [], 'run': {}},
          }
        : styleCopy(current!);
    if (seed['next'] == null) seed.remove('next');
    final style = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) =>
          DocumentStyleEditor(data: widget.data, style: seed, create: create),
    );
    if (style != null && mounted) {
      Navigator.pop(context, {'action': 'upsert', 'style': style});
    }
  }

  Future<void> delete() async {
    final replacement = await showDialog<String>(
      context: context,
      builder: (_) => DeleteDocumentStyleDialog(
        style: current!,
        styles: styles.where((s) => !availableIds.contains(s['id'])).toList(),
        defaultId: catalog['defaultStyle'] as String?,
      ),
    );
    if (replacement != null && mounted) {
      Navigator.pop(context, {
        'action': 'delete',
        'styleId': selected,
        'replacementStyleId': replacement,
      });
    }
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Styles'),
    content: SizedBox(
      width: 610,
      height: 420,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '${widget.paragraphs} ${widget.paragraphs == 1 ? 'paragraph' : 'paragraphs'} selected${widget.currentId == null ? ' · Mixed styles' : ''}',
          ),
          const SizedBox(height: 10),
          TextField(
            key: const Key('named-style-search'),
            decoration: const InputDecoration(
              hintText: 'Find a style',
              prefixIcon: Icon(Icons.search),
              isDense: true,
            ),
            onChanged: (value) {
              setState(() => query = value);
              if (scroll.hasClients) scroll.jumpTo(0);
            },
          ),
          const SizedBox(height: 10),
          Expanded(
            child: styles.isEmpty
                ? const Center(child: Text('Create a style to get started.'))
                : shown.isEmpty
                ? const Center(child: Text('No matching styles.'))
                : ListView.builder(
                    controller: scroll,
                    itemExtent: 70,
                    itemCount: shown.length,
                    itemBuilder: (context, index) {
                      final s = shown[index],
                          run =
                              widget.data['resolved']?[s['id']]?['run']
                                  as Map? ??
                              {};
                      return ListTile(
                        key: ValueKey('named-style-${s['id']}'),
                        selected: selected == s['id'],
                        selectedTileColor: const Color(0xffeaf0fa),
                        leading: Icon(
                          selected == s['id']
                              ? Icons.radio_button_checked
                              : Icons.radio_button_unchecked,
                          size: 18,
                        ),
                        title: Text(
                          s['name'] as String,
                          style: inheritedRunStyle(
                            run,
                            const TextStyle(fontSize: 16),
                          ).copyWith(fontSize: 18),
                        ),
                        subtitle: availableIds.contains(s['id'])
                            ? const Text('Added when applied')
                            : s['basedOn'] == null
                            ? null
                            : Text(
                                'Based on ${styles.where((b) => b['id'] == s['basedOn']).firstOrNull?['name'] ?? s['basedOn']}',
                              ),
                        onTap: () =>
                            setState(() => selected = s['id'] as String),
                      );
                    },
                  ),
          ),
          if (widget.manage)
            Row(
              children: [
                TextButton(
                  key: const Key('named-style-new'),
                  onPressed: () => edit(true),
                  child: const Text('New style…'),
                ),
                TextButton(
                  key: const Key('named-style-modify'),
                  onPressed: current == null || availableIds.contains(selected)
                      ? null
                      : () => edit(false),
                  child: const Text('Modify…'),
                ),
                TextButton(
                  key: const Key('named-style-delete'),
                  onPressed:
                      current == null ||
                          current?['default'] == true ||
                          current?['customStyle'] == false
                      ? null
                      : delete,
                  child: const Text('Delete'),
                ),
              ],
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
        key: const Key('named-style-apply'),
        onPressed: selected == null
            ? null
            : () => Navigator.pop(context, {
                'action': 'apply',
                'styleId': selected,
              }),
        child: const Text('Apply style'),
      ),
    ],
  );
}

class DeleteDocumentStyleDialog extends StatefulWidget {
  final Map style;
  final List<Map> styles;
  final String? defaultId;
  const DeleteDocumentStyleDialog({
    required this.style,
    required this.styles,
    required this.defaultId,
    super.key,
  });
  @override
  State<DeleteDocumentStyleDialog> createState() =>
      _DeleteDocumentStyleDialogState();
}

class _DeleteDocumentStyleDialogState extends State<DeleteDocumentStyleDialog> {
  // A descendant cannot replace its own base without making a cycle.
  late final choices = widget.styles.where((style) {
    final seen = <String>{};
    Map? current = style;
    while (current != null) {
      final id = current['id'] as String;
      if (id == widget.style['id'] || !seen.add(id)) return false;
      final base = current['basedOn'];
      current = widget.styles.where((s) => s['id'] == base).firstOrNull;
    }
    return true;
  }).toList();
  late String? replacement =
      choices.where((s) => s['id'] == widget.defaultId).firstOrNull?['id']
          as String? ??
      choices.firstOrNull?['id'] as String?;
  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text('Delete ${widget.style['name']}?'),
    content: SizedBox(
      width: 440,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Choose the style to use in its place. This updates text and related styles throughout the document, including headers and footnotes. Direct formatting stays. You can undo this change.',
          ),
          const SizedBox(height: 20),
          if (choices.isEmpty)
            const Text(
              'Create an independent paragraph style before deleting this one.',
            )
          else
            DropdownButtonFormField<String>(
              key: const Key('named-style-replacement'),
              initialValue: replacement,
              isExpanded: true,
              decoration: const InputDecoration(labelText: 'Replace with'),
              items: [
                for (final style in choices)
                  DropdownMenuItem(
                    value: style['id'] as String,
                    child: Text(style['name'] as String),
                  ),
              ],
              onChanged: (value) => setState(() => replacement = value),
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
        key: const Key('named-style-delete-confirm'),
        onPressed: replacement == null
            ? null
            : () => Navigator.pop(context, replacement),
        child: const Text('Replace and delete'),
      ),
    ],
  );
}

class DocumentStyleEditor extends StatefulWidget {
  final Map data;
  final Map<String, dynamic> style;
  final bool create;
  const DocumentStyleEditor({
    required this.data,
    required this.style,
    required this.create,
    super.key,
  });
  @override
  State<DocumentStyleEditor> createState() => _DocumentStyleEditorState();
}

class _DocumentStyleEditorState extends State<DocumentStyleEditor> {
  late final draft = styleCopy(widget.style);
  late final name = TextEditingController(text: draft['name'] as String);
  late final font = TextEditingController(
    text: draft['properties']['run']['rFonts']?['ascii'] as String? ?? '',
  );
  late final size = TextEditingController(
    text: draft['properties']['run']['sz'] == null
        ? ''
        : '${(draft['properties']['run']['sz'] as num) / 2}'.replaceFirst(
            RegExp(r'\.0$'),
            '',
          ),
  );
  late final color = TextEditingController(
    text: draft['properties']['run']['color']?['val'] == null
        ? ''
        : draft['properties']['run']['color']['val'] == 'auto'
        ? 'auto'
        : '#${draft['properties']['run']['color']['val']}',
  );
  final changed = <String>{};
  late final modes = <String, String>{
    for (final key in ['bold', 'italic', 'strike'])
      key: run[key] != true
          ? 'inherit'
          : (effectiveRun()[key] == true ? 'on' : 'off'),
  };
  Map get properties => draft['properties'] as Map;
  Map get run => properties['run'] as Map;
  Map get base =>
      widget.data['resolved']?[draft['basedOn']] as Map? ??
      widget.data['defaults'] as Map? ??
      widget.data['catalog']['defaults'] as Map;
  Map effectiveRun() {
    final result = styleCopy(base['run'] as Map? ?? {});
    for (final e in run.entries) {
      if (['bold', 'italic', 'strike'].contains(e.key)) {
        if (e.value == true) result[e.key] = result[e.key] != true;
      } else if (e.key == 'rFonts') {
        result[e.key] = {...result[e.key] as Map? ?? {}, ...e.value as Map};
      } else {
        result[e.key] = e.value;
      }
    }
    return result;
  }

  bool get valid =>
      name.text.trim().isNotEmpty &&
      name.text.trim().length <= 100 &&
      (size.text.trim().isEmpty ||
          (double.tryParse(size.text) ?? 0) >= .5 &&
              (double.tryParse(size.text) ?? 2000) <= 1638) &&
      (color.text.trim().isEmpty ||
          color.text.trim().toLowerCase() == 'auto' ||
          wordTextColor(color.text) != null);
  @override
  void dispose() {
    name.dispose();
    font.dispose();
    size.dispose();
    color.dispose();
    super.dispose();
  }

  void update(String key) => setState(() => changed.add(key));
  void save() {
    draft['name'] = name.text.trim();
    if (changed.contains('font')) {
      final fonts = Map<String, dynamic>.from(run['rFonts'] as Map? ?? {});
      for (final key in ['ascii', 'hAnsi', 'asciiTheme', 'hAnsiTheme']) {
        fonts.remove(key);
      }
      if (font.text.trim().isNotEmpty) {
        fonts.addAll({'ascii': font.text.trim(), 'hAnsi': font.text.trim()});
      }
      if (fonts.isEmpty) {
        run.remove('rFonts');
      } else {
        run['rFonts'] = fonts;
      }
    }
    if (changed.contains('size')) {
      if (size.text.trim().isEmpty) {
        run.remove('sz');
      } else {
        run['sz'] = (double.parse(size.text) * 2).round();
      }
    }
    if (changed.contains('color')) {
      if (color.text.trim().isEmpty) {
        run.remove('color');
      } else {
        run['color'] = {
          'val': color.text.trim().toLowerCase() == 'auto'
              ? 'auto'
              : color.text.trim().replaceFirst('#', '').toUpperCase(),
        };
      }
    }
    for (final key in ['bold', 'italic', 'strike']) {
      if (changed.contains(key) || changed.contains('basedOn')) {
        if (modes[key] == 'inherit') {
          run.remove(key);
        } else {
          run[key] = (modes[key] == 'on') != (base['run']?[key] == true);
        }
      }
    }
    Navigator.pop(context, draft);
  }

  Future<void> paragraph() async {
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => DocumentParagraphDialog(
        formats: [
          {
            'inherited': base['paragraph'] ?? {},
            'direct': properties['paragraph'],
            'inheritedRun': effectiveRun(),
          },
        ],
        sample: Delta()..insert('A paragraph in this style.\n'),
      ),
    );
    if (result != null && mounted) {
      setState(() {
        final direct = properties['paragraph'] as Map;
        for (final e in result.entries) {
          if (e.value == null) {
            direct.remove(e.key);
          } else {
            direct[e.key] = e.value;
          }
        }
      });
    }
  }

  Widget textField(
    String key,
    String label,
    TextEditingController controller, {
    String? hint,
  }) => SizedBox(
    width: 250,
    child: draftTextField(
      key: ValueKey('style-editor-$key'),
      controller: controller,
      onChanged: (_) => update(key),
      decoration: InputDecoration(labelText: label, hintText: hint),
    ),
  );
  @override
  Widget build(BuildContext context) {
    final styles = (widget.data['catalog']['styles'] as List)
        .where((s) => s['type'] == 'paragraph')
        .cast<Map>()
        .toList();
    final preview = effectiveRun();
    if (changed.contains('font') && font.text.trim().isNotEmpty) {
      preview['rFonts'] = {'ascii': font.text.trim()};
    }
    if (changed.contains('size') &&
        (double.tryParse(size.text)?.isFinite ?? false)) {
      preview['sz'] = (double.parse(size.text) * 2).round();
    }
    if (changed.contains('color') && wordTextColor(color.text) != null) {
      preview['color'] = {'val': color.text.replaceFirst('#', '')};
    }
    for (final key in ['bold', 'italic', 'strike']) {
      if (changed.contains(key) || changed.contains('basedOn')) {
        preview[key] = modes[key] == 'inherit'
            ? (base['run']?[key] == true)
            : (modes[key] == 'on');
      }
    }
    return AlertDialog(
      title: Text(widget.create ? 'New style' : 'Modify style'),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              draftTextField(
                key: const Key('style-editor-name'),
                controller: name,
                autofocus: widget.create,
                onChanged: (_) => setState(() {}),
                decoration: const InputDecoration(labelText: 'Style name'),
              ),
              const SizedBox(height: 14),
              DropdownButtonFormField<String>(
                isExpanded: true,
                key: const Key('style-editor-base'),
                initialValue: draft['basedOn'] as String? ?? '',
                decoration: const InputDecoration(labelText: 'Based on'),
                items: [
                  const DropdownMenuItem(
                    value: '',
                    child: Text('No base style'),
                  ),
                  if (draft['basedOn'] != null &&
                      !styles.any(
                        (s) =>
                            s['id'] == draft['basedOn'] &&
                            s['id'] != draft['id'],
                      ))
                    DropdownMenuItem(
                      value: draft['basedOn'] as String,
                      child: Text('Unavailable: ${draft['basedOn']}'),
                    ),
                  for (final s in styles.where((s) => s['id'] != draft['id']))
                    DropdownMenuItem(
                      value: s['id'] as String,
                      child: Text(s['name'] as String),
                    ),
                ],
                onChanged: (v) => setState(() {
                  if (v == '') {
                    draft.remove('basedOn');
                  } else {
                    draft['basedOn'] = v;
                  }
                  changed.add('basedOn');
                }),
              ),
              const SizedBox(height: 12),
              DropdownButtonFormField<String>(
                isExpanded: true,
                key: const Key('style-editor-next'),
                initialValue: draft['next'] as String? ?? '',
                decoration: const InputDecoration(
                  labelText: 'Following paragraph style',
                ),
                items: [
                  const DropdownMenuItem(value: '', child: Text('Same style')),
                  if (draft['next'] != null &&
                      !styles.any((s) => s['id'] == draft['next']))
                    DropdownMenuItem(
                      value: draft['next'] as String,
                      child: Text('Unavailable: ${draft['next']}'),
                    ),
                  for (final s in styles)
                    DropdownMenuItem(
                      value: s['id'] as String,
                      child: Text(s['name'] as String),
                    ),
                ],
                onChanged: (v) => setState(() {
                  if (v == '') {
                    draft.remove('next');
                  } else {
                    draft['next'] = v;
                  }
                }),
              ),
              const SizedBox(height: 16),
              Wrap(
                spacing: 16,
                runSpacing: 12,
                children: [
                  textField('font', 'Font', font, hint: 'Use base style'),
                  textField(
                    'size',
                    'Size (points)',
                    size,
                    hint: 'Use base style',
                  ),
                  textField(
                    'color',
                    'Text color',
                    color,
                    hint: 'Use base style, or #185ABD',
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 14,
                children: [
                  for (final entry in const {
                    'bold': 'Bold',
                    'italic': 'Italic',
                    'strike': 'Strikethrough',
                  }.entries)
                    SizedBox(
                      width: 164,
                      child: DropdownButtonFormField<String>(
                        isExpanded: true,
                        key: ValueKey('style-editor-${entry.key}'),
                        initialValue: modes[entry.key],
                        decoration: InputDecoration(labelText: entry.value),
                        items: const [
                          DropdownMenuItem(
                            value: 'inherit',
                            child: Text('Inherited'),
                          ),
                          DropdownMenuItem(value: 'on', child: Text('On')),
                          DropdownMenuItem(value: 'off', child: Text('Off')),
                        ],
                        onChanged: (v) => setState(() {
                          modes[entry.key] = v!;
                          changed.add(entry.key);
                        }),
                      ),
                    ),
                ],
              ),
              const SizedBox(height: 12),
              TextButton(
                key: const Key('style-editor-paragraph'),
                onPressed: paragraph,
                child: const Text('Paragraph formatting…'),
              ),
              const SizedBox(height: 8),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(18),
                color: Colors.white,
                child: Text(
                  'A clear idea starts with a few words.',
                  style:
                      inheritedRunStyle(
                        preview,
                        const TextStyle(fontSize: 16),
                      ).copyWith(
                        fontSize: ((preview['sz'] as num? ?? 24) / 2 * 4 / 3)
                            .clamp(10, 44)
                            .toDouble(),
                      ),
                ),
              ),
              if (!widget.create)
                const Padding(
                  padding: EdgeInsets.only(top: 10),
                  child: Text(
                    'Saving updates every paragraph that uses this style. Direct formatting stays in place.',
                  ),
                ),
              if (!valid)
                const Padding(
                  padding: EdgeInsets.only(top: 8),
                  child: Text(
                    'Enter a name, a valid point size and a six-digit color, or leave formatting fields blank to inherit.',
                    style: TextStyle(color: Colors.red),
                  ),
                ),
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
          key: const Key('style-editor-save'),
          onPressed: valid ? save : null,
          child: Text(widget.create ? 'Create style' : 'Save style'),
        ),
      ],
    );
  }
}
