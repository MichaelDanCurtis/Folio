import 'package:flutter/foundation.dart';
import 'document_page_numbers.dart';
import 'document_fields.dart';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'document_codec.dart';
import 'document_edit_tracking.dart';
import 'document_list_rules.dart';
import 'document_lists.dart';
import 'document_styles.dart';
import 'document_text_style.dart';
import 'document_view.dart';
import 'history_commands.dart';

class DocumentHeadersDialog extends StatefulWidget {
  final int initialSection;
  final Future<Map<String, dynamic>> Function(Map<String, dynamic>) read;
  final Future<void> Function(Map<String, dynamic>) apply;
  const DocumentHeadersDialog({
    super.key,
    required this.initialSection,
    required this.read,
    required this.apply,
  });
  @override
  State<DocumentHeadersDialog> createState() => _DocumentHeadersState();
}

class _HeaderDraft {
  final Map baseline;
  late final q.QuillController editor;
  late final DocumentEditCapture capture;
  late final DocumentEditDraft journal;
  late Map previous;
  _HeaderDraft(this.baseline, VoidCallback changed) {
    editor = q.QuillController(
      document: q.Document.fromDelta(documentToDelta(baseline)),
      selection: const TextSelection.collapsed(offset: 0),
    );
    configureDocumentLists(
      editor.document,
      styleCatalog: () => baseline['attrs']?['wordStyles'] as Map?,
    );
    capture = DocumentEditCapture.attach(editor.document);
    previous = document;
    journal = DocumentEditDraft(previous);
    editor.addListener(() {
      if (capture.hasChanges) {
        final next = document;
        journal.record(previous, next, capture.take(previous, next));
        previous = next;
      }
      changed();
    });
  }
  Map<String, dynamic> get document => deltaToDocument(
    editor.document.toDelta(),
    attributes: baseline['attrs'] as Map?,
  );
  bool get changed => !sameDocumentContent(baseline, document);
}

class _DocumentHeadersState extends State<DocumentHeadersDialog> {
  late int section;
  String kind = 'header', variant = 'default';
  Map<String, dynamic>? data;
  bool busy = true, linked = false, first = false, even = false;
  Map<String, dynamic> pageNumbering = Map.of(defaultPageNumbering);
  String? error;
  final focus = FocusNode(), scroll = ScrollController();
  final retired = <_HeaderDraft>[];
  final drafts = <String, _HeaderDraft>{};
  _HeaderDraft get draft =>
      drafts[data?['linked'] == true
          ? 'current'
          : linked
          ? 'previous'
          : 'current']!;
  String? get unavailable {
    if (data?['protected'] == true) {
      return 'This document is protected. Headers and footers cannot be changed.';
    }
    if (data?['needsAuthor'] == true) {
      return 'Choose a review author in Recording settings before editing this document.';
    }
    if (data?['recordingEnabled'] == true) {
      return 'Recording header and footer edits is not supported yet. Turn recording off explicitly in Review to make an unrecorded change.';
    }
    return null;
  }

  bool get dirty =>
      data != null &&
      (draft.changed ||
          !mapEquals(
            pageNumbering,
            data!['pageNumbering'] as Map? ?? defaultPageNumbering,
          ) ||
          linked != data!['linked'] ||
          first != data!['differentFirstPage'] ||
          even != data!['differentOddEven']);
  @override
  void initState() {
    super.initState();
    section = widget.initialSection;
    focus.addListener(_publishHistory);
    load();
  }

  void _publishHistory() {
    if (!mounted || !focus.hasFocus || data == null || drafts.isEmpty) {
      if (identical(focusedDraftHistory.value?.owner, this)) {
        focusedDraftHistory.value = null;
      }
      return;
    }
    final active = draft.editor;
    final editable = !busy && !active.readOnly;
    final undo = editable && active.hasUndo, redo = editable && active.hasRedo;
    final previous = focusedDraftHistory.value;
    if (identical(previous?.owner, this) &&
        previous?.canUndo == undo &&
        previous?.canRedo == redo) {
      return;
    }
    focusedDraftHistory.value = DraftHistoryTarget(
      owner: this,
      canUndo: undo,
      canRedo: redo,
      undo: () {
        if (focus.hasFocus && !busy && !draft.editor.readOnly) {
          draft.editor.undo();
        }
      },
      redo: () {
        if (focus.hasFocus && !busy && !draft.editor.readOnly) {
          draft.editor.redo();
        }
      },
    );
  }

  @override
  void dispose() {
    if (identical(focusedDraftHistory.value?.owner, this)) {
      focusedDraftHistory.value = null;
    }
    focus.removeListener(_publishHistory);
    for (final d in {...retired, ...drafts.values}) {
      d.editor.dispose();
    }
    focus.dispose();
    scroll.dispose();
    super.dispose();
  }

  Future<bool> load() async {
    focus.unfocus();
    for (final d in drafts.values) {
      d.editor.readOnly = true;
    }
    setState(() => busy = true);
    try {
      final next = await widget.read({
        'section': section,
        'kind': kind,
        'variant': variant,
      });
      if (!mounted) return false;
      retired.addAll(drafts.values);
      drafts.clear();
      data = next;
      linked = next['linked'] == true;
      first = next['differentFirstPage'] == true;
      even = next['differentOddEven'] == true;
      pageNumbering = Map<String, dynamic>.from(
        next['pageNumbering'] as Map? ?? defaultPageNumbering,
      );
      void changed() {
        if (mounted) {
          setState(() {});
          _publishHistory();
        }
      }

      drafts['current'] = _HeaderDraft(next['document'] as Map, changed);
      drafts['previous'] = _HeaderDraft(
        next['previousDocument'] as Map,
        changed,
      );
      for (final d in drafts.values) {
        d.editor.readOnly = unavailable != null;
      }
      error = null;
    } catch (e) {
      error = '$e';
      for (final d in drafts.values) {
        d.editor.readOnly = unavailable != null;
      }
    }
    if (mounted) setState(() => busy = false);
    return error == null;
  }

  Future<bool> apply({bool close = true}) async {
    if (!dirty || unavailable != null || busy) return false;
    final value = draft.document;
    focus.unfocus();
    draft.editor.readOnly = true;
    setState(() => busy = true);
    try {
      await widget.apply({
        'section': section,
        'kind': kind,
        'variant': variant,
        'storyKey': data!['storyKey'],
        'workspaceId': data!['workspaceId'],
        'baseRevision': data!['baseRevision'],
        'linked': linked,
        'differentFirstPage': first,
        'differentOddEven': even,
        'pageNumbering': pageNumbering,
        'document': value,
        'textEdits': ?draft.journal.finish(value),
      });
      if (!mounted) return true;
      if (close) {
        Navigator.pop(context);
      } else {
        return await load();
      }
      return true;
    } catch (e) {
      if (mounted) {
        setState(() {
          error = '$e';
          busy = false;
          draft.editor.readOnly = unavailable != null;
        });
      }
      return false;
    }
  }

  Future<void> choose({
    int? nextSection,
    String? nextKind,
    String? nextVariant,
  }) async {
    if (busy) return;
    if (dirty) {
      final choice = await showDialog<String>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('Keep these edits?'),
          content: const Text(
            'Apply this header or footer before switching, or discard this draft.',
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Keep editing'),
            ),
            TextButton(
              onPressed: () => Navigator.pop(context, 'discard'),
              child: const Text('Discard draft'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, 'apply'),
              child: const Text('Apply'),
            ),
          ],
        ),
      );
      if (!mounted || choice == null) return;
      if (choice == 'apply' && !await apply(close: false)) return;
      if (!mounted) return;
    }
    final previousSection = section,
        previousKind = kind,
        previousVariant = variant;
    section = nextSection ?? section;
    kind = nextKind ?? kind;
    variant = nextVariant ?? variant;
    if (!await load() && mounted) {
      setState(() {
        section = previousSection;
        kind = previousKind;
        variant = previousVariant;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final value = data;
    final active = value == null ? null : draft;
    final affected = value?['affectedSections'] as List? ?? [];
    final blocked = unavailable;
    final canEdit = !busy && blocked == null;
    final labels = active == null
        ? <int, String>{}
        : documentListLabels(active.editor.document.toDelta());
    final inactive =
        (variant == 'first' && !first) || (variant == 'even' && !even);
    return Dialog(
      child: SizedBox(
        width: 980,
        height: 740,
        child: Padding(
          padding: const EdgeInsets.all(22),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'Headers & footers',
                style: TextStyle(fontSize: 24, fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 14),
              if (value != null) ...[
                Row(
                  children: [
                    Expanded(
                      child: DropdownButtonFormField<int>(
                        key: ValueKey('header-section-$section'),
                        initialValue: section,
                        isExpanded: true,
                        decoration: const InputDecoration(labelText: 'Section'),
                        items: List.generate(
                          (value['sections'] as List).length,
                          (i) => DropdownMenuItem(
                            value: i,
                            child: Text('Section ${i + 1}'),
                          ),
                        ),
                        onChanged: busy ? null : (v) => choose(nextSection: v),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        key: ValueKey('header-kind-$kind'),
                        initialValue: kind,
                        isExpanded: true,
                        decoration: const InputDecoration(labelText: 'Area'),
                        items: const [
                          DropdownMenuItem(
                            value: 'header',
                            child: Text('Header'),
                          ),
                          DropdownMenuItem(
                            value: 'footer',
                            child: Text('Footer'),
                          ),
                        ],
                        onChanged: busy ? null : (v) => choose(nextKind: v),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        key: ValueKey('header-variant-$variant'),
                        initialValue: variant,
                        isExpanded: true,
                        decoration: const InputDecoration(labelText: 'Pages'),
                        items: [
                          DropdownMenuItem(
                            value: 'default',
                            child: Text(even ? 'Odd pages' : 'Default pages'),
                          ),
                          const DropdownMenuItem(
                            value: 'first',
                            child: Text('First page'),
                          ),
                          const DropdownMenuItem(
                            value: 'even',
                            child: Text('Even pages'),
                          ),
                        ],
                        onChanged: busy ? null : (v) => choose(nextVariant: v),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Wrap(
                  spacing: 16,
                  runSpacing: 4,
                  children: [
                    SizedBox(
                      width: 220,
                      child: CheckboxListTile(
                        key: const Key('header-link'),
                        dense: true,
                        contentPadding: EdgeInsets.zero,
                        title: const Text('Link to Previous'),
                        value: linked,
                        onChanged: canEdit && section > 0
                            ? (v) => setState(() => linked = v!)
                            : null,
                      ),
                    ),
                    SizedBox(
                      width: 230,
                      child: CheckboxListTile(
                        key: const Key('header-first'),
                        dense: true,
                        contentPadding: EdgeInsets.zero,
                        title: const Text('Different first page'),
                        value: first,
                        onChanged: canEdit
                            ? (v) => setState(() => first = v!)
                            : null,
                      ),
                    ),
                    SizedBox(
                      width: 330,
                      child: CheckboxListTile(
                        key: const Key('header-even'),
                        dense: true,
                        contentPadding: EdgeInsets.zero,
                        title: const Text('Different odd and even pages'),
                        subtitle: const Text('Applies to the whole document'),
                        value: even,
                        onChanged: canEdit
                            ? (v) => setState(() => even = v!)
                            : null,
                      ),
                    ),
                    TextButton.icon(
                      key: const Key('header-page-numbers'),
                      icon: const Icon(Icons.format_list_numbered, size: 18),
                      label: const Text('Page number format…'),
                      onPressed: canEdit
                          ? () async {
                              final result = await showDialog<Map>(
                                context: context,
                                builder: (_) => PageNumberFormatDialog(
                                  initial: pageNumbering,
                                  section: section,
                                ),
                              );
                              if (mounted && result != null) {
                                setState(
                                  () => pageNumbering =
                                      Map<String, dynamic>.from(result),
                                );
                              }
                            }
                          : null,
                    ),
                  ],
                ),
                Text(
                  linked && value['linked'] != true
                      ? 'Apply will replace this section’s $kind with the previous section’s content. Edits here also change the linked previous content.'
                      : !linked && value['linked'] == true
                      ? 'This section will start its own $kind. Later sections that link to it will follow these edits.'
                      : 'Editing this $kind affects ${affected.length == 1 ? 'Section' : 'Sections'} ${affected.map((i) => i + 1).join(', ')}.',
                  key: const Key('header-impact'),
                  style: const TextStyle(fontSize: 12),
                ),
                if (inactive)
                  Padding(
                    padding: const EdgeInsets.only(top: 6),
                    child: Text(
                      'This content is stored but hidden until ${variant == 'first' ? 'Different first page' : 'Different odd and even pages'} is enabled.',
                      style: const TextStyle(fontSize: 12),
                    ),
                  ),
                if (active != null) ...[
                  IgnorePointer(
                    ignoring: !canEdit,
                    child: DocumentTextToolbar(
                      controller: active.editor,
                      focus: focus,
                    ),
                  ),
                  const Divider(),
                  Expanded(
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        color: Colors.white,
                        border: Border.all(color: const Color(0xffb9c1cc)),
                      ),
                      child: q.QuillEditor.basic(
                        key: ValueKey(active),
                        controller: active.editor,
                        focusNode: focus,
                        scrollController: scroll,
                        config: q.QuillEditorConfig(
                          padding: const EdgeInsets.all(18),
                          customStyles: documentEditorStyles(),
                          customShortcuts: documentMarkShortcuts,
                          customActions: documentMarkActions(
                            active.editor,
                            focus,
                          ),
                          embedBuilders: [
                            const PageFieldEmbedBuilder(),
                            FolioEmbedBuilder(),
                          ],
                          // ignore: experimental_member_use
                          customLeadingBlockBuilder: (node, config) {
                            final label = labels[node.documentOffset];
                            return label == null
                                ? null
                                : SizedBox(
                                    width: config.width,
                                    child: Text(
                                      label,
                                      style: config.style,
                                      textAlign: TextAlign.end,
                                    ),
                                  );
                          },
                        ),
                      ),
                    ),
                  ),
                ],
              ] else
                const Spacer(),
              if (busy) const LinearProgressIndicator(),
              if (blocked != null || error != null)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 8),
                  child: Text(
                    error ?? blocked!,
                    key: const Key('header-error'),
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
              const SizedBox(height: 10),
              const Text(
                'Headers and footers are saved in the Word file. The document editor still uses a continuous view.',
                style: TextStyle(fontSize: 12),
              ),
              const SizedBox(height: 10),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    key: const Key('header-cancel'),
                    onPressed: busy ? null : () => Navigator.pop(context),
                    child: const Text('Cancel'),
                  ),
                  const SizedBox(width: 10),
                  FilledButton(
                    key: const Key('header-apply'),
                    onPressed: canEdit && dirty ? () => apply() : null,
                    child: const Text('Apply'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
