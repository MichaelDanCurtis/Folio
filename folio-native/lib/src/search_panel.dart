import 'draft_text_field.dart';
import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'core.dart';
import 'document_codec.dart';
import 'document_bookmarks.dart';
import 'search.dart';

class SearchPanel extends StatefulWidget {
  final WorkspaceModel model;
  final q.QuillController editor;
  final bool spreadsheet;
  final String sheetId;
  final Future<void> Function() flush;
  final Future<void> Function(SearchHit) onNavigate;
  final VoidCallback onClose;
  const SearchPanel({
    super.key,
    required this.model,
    required this.editor,
    required this.spreadsheet,
    required this.sheetId,
    required this.flush,
    required this.onNavigate,
    required this.onClose,
  });
  @override
  State<SearchPanel> createState() => SearchPanelState();
}

class SearchPanelState extends State<SearchPanel> {
  final query = TextEditingController(), replacement = TextEditingController();
  final queryFocus = FocusNode(), resultScroll = ScrollController();
  bool replaceMode = false,
      headings = false,
      matchCase = false,
      wholeWords = false,
      entireCell = false,
      values = false,
      allSheets = false,
      byColumns = false;
  bool busy = false, pending = false;
  String? message;
  Timer? _timer;
  DocumentSearchIndex? _index;
  SearchResults results = const SearchResults([]);
  int selected = -1, _revision = -1;
  String _document = '', _workspaceId = '';
  SearchOptions get options => SearchOptions(
    matchCase: matchCase,
    wholeWords: !widget.spreadsheet && wholeWords,
    entireCell: widget.spreadsheet && entireCell,
    values: values,
    allSheets: allSheets,
    byColumns: byColumns,
  );

  @override
  void initState() {
    super.initState();
    widget.model.addListener(_workspaceChanged);
    widget.editor.addListener(_editorChanged);
    query.addListener(_schedule);
    _refresh();
  }

  @override
  void didUpdateWidget(covariant SearchPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.spreadsheet != widget.spreadsheet ||
        (!allSheets && oldWidget.sheetId != widget.sheetId)) {
      if (widget.spreadsheet) headings = false;
      _refresh();
    }
  }

  void _workspaceChanged() {
    if (widget.model.revision != _revision ||
        widget.model.workspace['id'] != _workspaceId) {
      _schedule();
    }
  }

  void _editorChanged() {
    if (!widget.spreadsheet &&
        jsonEncode(deltaToDocument(widget.editor.document.toDelta())) !=
            _document) {
      _schedule();
    }
  }

  void _schedule() {
    _timer?.cancel();
    if (!mounted) return;
    setState(() {
      pending = true;
      message = null;
    });
    _timer = Timer(const Duration(milliseconds: 180), _refresh);
  }

  void _refresh() {
    _timer?.cancel();
    if (!mounted) return;
    final previous = selected >= 0 && selected < results.hits.length
        ? results.hits[selected].identity
        : null;
    _revision = widget.model.revision;
    _workspaceId = widget.model.workspace['id'];
    final document = deltaToDocument(widget.editor.document.toDelta());
    _document = jsonEncode(document);
    if (widget.spreadsheet) {
      results = findInWorkbook(
        widget.model.sheets,
        widget.model.snapshot['calculated'] ?? {},
        widget.sheetId,
        query.text,
        options,
        formatted: widget.model.snapshot['formatted'] as Map? ?? {},
      );
    } else {
      _index = DocumentSearchIndex(document);
      results = headings
          ? SearchResults([
              for (final block in _index!.blocks)
                if (block.heading > 0 && block.text.isNotEmpty)
                  SearchHit(
                    label: block.label,
                    text: block.text,
                    start: 0,
                    end: block.text.length,
                    block: block,
                  ),
            ])
          : _index!.find(query.text, options);
    }
    selected = previous == null
        ? -1
        : results.hits.indexWhere((h) => h.identity == previous);
    setState(() => pending = false);
  }

  void focusQuery({bool replace = false, String? initial}) {
    setState(() {
      replaceMode = replace;
      headings = false;
      if (replace) values = false;
    });
    if (initial != null &&
        initial.isNotEmpty &&
        initial.length <= 200 &&
        !initial.contains('\n')) {
      query.text = initial;
    }
    _refresh();
    query.selection = TextSelection(
      baseOffset: 0,
      extentOffset: query.text.length,
    );
    queryFocus.requestFocus();
  }

  Future<void> navigate(int index) async {
    if (busy || pending || index < 0 || index >= results.hits.length) return;
    final hit = results.hits[index];
    setState(() {
      selected = index;
      message = null;
    });
    try {
      await widget.onNavigate(hit);
      if (mounted && resultScroll.hasClients) {
        resultScroll.jumpTo(
          (index * 76.0).clamp(0, resultScroll.position.maxScrollExtent),
        );
      }
    } catch (e) {
      if (mounted) setState(() => message = e.toString());
    }
  }

  void next([bool backwards = false]) {
    if (pending) _refresh();
    if (results.hits.isEmpty) return;
    final index = selected < 0
        ? (backwards ? results.hits.length - 1 : 0)
        : (selected + (backwards ? -1 : 1)) % results.hits.length;
    unawaited(navigate(index));
  }

  Future<void> replaceMatches(bool all) async {
    if (busy ||
        pending ||
        headings ||
        results.hits.isEmpty ||
        (all && results.truncated) ||
        (widget.spreadsheet && values)) {
      return;
    }
    final hits = all
        ? results.hits
        : [results.hits[selected < 0 ? 0 : selected]];
    final index = _index, baseDocument = _document;
    final baseRevision = _revision, workspaceId = _workspaceId;
    final searchText = query.text,
        replacementText = replacement.text,
        searchOptions = options;
    setState(() {
      busy = true;
      message = null;
    });
    try {
      await widget.flush();
      if (widget.model.revision != baseRevision ||
          widget.model.workspace['id'] != workspaceId ||
          (!widget.spreadsheet &&
              jsonEncode(deltaToDocument(widget.editor.document.toDelta())) !=
                  baseDocument)) {
        throw StateError(
          'The content changed. Review the refreshed results and try again.',
        );
      }
      final proposal = <String, dynamic>{
        'baseRevision': baseRevision,
        '_workspaceId': workspaceId,
        'summary': all ? 'Replace all matches' : 'Replace match',
      };
      var count = hits.length;
      if (widget.spreadsheet) {
        final edits = workbookReplacements(
          widget.model.sheets,
          hits,
          searchText,
          replacementText,
          searchOptions,
        );
        count = edits.length;
        proposal['cells'] = edits;
      } else {
        final document = index!.replace(
          deltaToDocument(widget.editor.document.toDelta()),
          hits,
          replacementText,
        );
        if (jsonEncode(document) == baseDocument) count = 0;
        proposal['document'] = document;
        final before = deltaToDocument(widget.editor.document.toDelta());
        final locations = DocumentBookmarkIndex(before);
        final exact = checkedTextEdits(before, document, [
          for (final hit in hits)
            {
              'start': locations.atPath(hit.block!.path)!.start + hit.start,
              'end': locations.atPath(hit.block!.path)!.start + hit.end,
              'text': replacementText,
            },
        ]);
        if (exact == null) {
          throw StateError('The search locations changed. Search again.');
        }
        proposal['textEdits'] = exact;
      }
      if (count > 0) {
        // User-directed replacement uses the same atomic, revision-checked
        // transaction as other workspace edits, with one shared Undo entry.
        await widget.model.mutate({
          'op': 'apply',
          'proposal': proposal,
        }, refreshDocument: !widget.spreadsheet);
      }
      if (!mounted) return;
      _refresh();
      setState(
        () => message = count == 0
            ? 'No changes needed.'
            : 'Replaced $count ${widget.spreadsheet ? 'cell${count == 1 ? '' : 's'}' : 'match${count == 1 ? '' : 'es'}'}. Undo restores this action.',
      );
    } catch (e) {
      if (!mounted) return;
      _refresh();
      setState(() => message = e.toString());
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    Widget toggle(String label, bool value, void Function(bool) change) =>
        FilterChip(
          label: Text(label, style: const TextStyle(fontSize: 12)),
          selected: value,
          onSelected: busy
              ? null
              : (v) {
                  setState(() => change(v));
                  _refresh();
                },
          visualDensity: VisualDensity.compact,
        );
    final canReplace = !busy && !pending && results.hits.isNotEmpty && !values;
    return Material(
      color: Colors.white,
      child: Shortcuts(
        shortcuts: const {
          SingleActivator(LogicalKeyboardKey.escape): DismissIntent(),
        },
        child: Actions(
          actions: {
            DismissIntent: CallbackAction<DismissIntent>(
              onInvoke: (_) {
                if (!busy) widget.onClose();
                return null;
              },
            ),
          },
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 8, 4),
                child: Row(
                  children: [
                    const Expanded(
                      child: Text(
                        'Navigation',
                        style: TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    IconButton(
                      tooltip: 'Close search',
                      onPressed: busy ? null : widget.onClose,
                      icon: const Icon(Icons.close),
                    ),
                  ],
                ),
              ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: Wrap(
                  spacing: 4,
                  children: [
                    ChoiceChip(
                      label: const Text('Find'),
                      selected: !replaceMode && !headings,
                      onSelected: busy ? null : (_) => focusQuery(),
                    ),
                    ChoiceChip(
                      label: const Text('Replace'),
                      selected: replaceMode,
                      onSelected: busy
                          ? null
                          : (_) => focusQuery(replace: true),
                    ),
                    if (!widget.spreadsheet)
                      ChoiceChip(
                        label: const Text('Headings'),
                        selected: headings,
                        onSelected: busy
                            ? null
                            : (_) {
                                setState(() {
                                  headings = true;
                                  replaceMode = false;
                                });
                                _refresh();
                              },
                      ),
                  ],
                ),
              ),
              if (!headings)
                Padding(
                  padding: const EdgeInsets.all(12),
                  child: Column(
                    children: [
                      draftTextField(
                        key: const Key('find-query'),
                        controller: query,
                        focusNode: queryFocus,
                        enabled: !busy,
                        maxLength: 200,
                        decoration: const InputDecoration(
                          labelText: 'Find',
                          border: OutlineInputBorder(),
                          counterText: '',
                          isDense: true,
                        ),
                        onSubmitted: (_) =>
                            next(HardwareKeyboard.instance.isShiftPressed),
                      ),
                      if (replaceMode) ...[
                        const SizedBox(height: 10),
                        draftTextField(
                          key: const Key('replace-text'),
                          controller: replacement,
                          enabled: !busy,
                          maxLength: 10000,
                          decoration: const InputDecoration(
                            labelText: 'Replace with',
                            helperText: 'Leave empty to remove matches',
                            border: OutlineInputBorder(),
                            counterText: '',
                            isDense: true,
                          ),
                        ),
                      ],
                      const SizedBox(height: 8),
                      Wrap(
                        spacing: 5,
                        children: [
                          toggle('Match case', matchCase, (v) => matchCase = v),
                          if (!widget.spreadsheet)
                            toggle(
                              'Whole words',
                              wholeWords,
                              (v) => wholeWords = v,
                            ),
                          if (widget.spreadsheet)
                            toggle(
                              'Entire cell',
                              entireCell,
                              (v) => entireCell = v,
                            ),
                        ],
                      ),
                      if (widget.spreadsheet) ...[
                        const SizedBox(height: 8),
                        Row(
                          children: [
                            Expanded(
                              child: DropdownButtonFormField<bool>(
                                isExpanded: true,
                                style: const TextStyle(
                                  fontSize: 12,
                                  color: Colors.black87,
                                ),
                                initialValue: allSheets,
                                decoration: const InputDecoration(
                                  labelText: 'Within',
                                  isDense: true,
                                ),
                                items: const [
                                  DropdownMenuItem(
                                    value: false,
                                    child: Text('Current sheet'),
                                  ),
                                  DropdownMenuItem(
                                    value: true,
                                    child: Text('Workbook'),
                                  ),
                                ],
                                onChanged: busy
                                    ? null
                                    : (v) {
                                        allSheets = v!;
                                        _refresh();
                                      },
                              ),
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: DropdownButtonFormField<bool>(
                                isExpanded: true,
                                style: const TextStyle(
                                  fontSize: 12,
                                  color: Colors.black87,
                                ),
                                initialValue: byColumns,
                                decoration: const InputDecoration(
                                  labelText: 'Search order',
                                  isDense: true,
                                ),
                                items: const [
                                  DropdownMenuItem(
                                    value: false,
                                    child: Text('By rows'),
                                  ),
                                  DropdownMenuItem(
                                    value: true,
                                    child: Text('By columns'),
                                  ),
                                ],
                                onChanged: busy
                                    ? null
                                    : (v) {
                                        byColumns = v!;
                                        _refresh();
                                      },
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 8),
                        DropdownButtonFormField<bool>(
                          isExpanded: true,
                          style: const TextStyle(
                            fontSize: 12,
                            color: Colors.black87,
                          ),
                          key: ValueKey('search-values-$replaceMode'),
                          initialValue: values,
                          decoration: const InputDecoration(
                            labelText: 'Look in',
                            isDense: true,
                          ),
                          items: const [
                            DropdownMenuItem(
                              value: false,
                              child: Text('Formulas & text'),
                            ),
                            DropdownMenuItem(
                              value: true,
                              child: Text('Displayed values'),
                            ),
                          ],
                          onChanged: busy || replaceMode
                              ? null
                              : (v) {
                                  values = v!;
                                  _refresh();
                                },
                        ),
                      ],
                      if (replaceMode)
                        Padding(
                          padding: const EdgeInsets.only(top: 10),
                          child: Row(
                            children: [
                              Expanded(
                                child: OutlinedButton(
                                  key: const Key('replace-one'),
                                  onPressed: canReplace
                                      ? () => replaceMatches(false)
                                      : null,
                                  child: const Text('Replace'),
                                ),
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: FilledButton(
                                  key: const Key('replace-all'),
                                  onPressed: canReplace && !results.truncated
                                      ? () => replaceMatches(true)
                                      : null,
                                  child: const Text('Replace all'),
                                ),
                              ),
                            ],
                          ),
                        ),
                    ],
                  ),
                ),
              if (busy || pending) const LinearProgressIndicator(minHeight: 2),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        headings
                            ? '${results.hits.length} headings'
                            : query.text.isEmpty
                            ? 'Enter text to search'
                            : results.hits.isEmpty
                            ? 'No matches'
                            : '${selected < 0 ? '' : '${selected + 1} of '}${results.hits.length} ${widget.spreadsheet ? 'cells' : 'matches'}',
                        style: const TextStyle(fontSize: 12),
                      ),
                    ),
                    IconButton(
                      tooltip: 'Previous match',
                      onPressed: busy || pending || results.hits.isEmpty
                          ? null
                          : () => next(true),
                      icon: const Icon(Icons.keyboard_arrow_up),
                    ),
                    IconButton(
                      tooltip: 'Next match',
                      onPressed: busy || pending || results.hits.isEmpty
                          ? null
                          : next,
                      icon: const Icon(Icons.keyboard_arrow_down),
                    ),
                  ],
                ),
              ),
              if (message != null || results.truncated)
                Padding(
                  padding: const EdgeInsets.all(12),
                  child: Text(
                    results.truncated
                        ? 'Showing the first 10,000 results. Narrow the search to use Replace all.'
                        : message!,
                    style: const TextStyle(fontSize: 12),
                  ),
                ),
              const Divider(height: 1),
              Expanded(
                child: ListView.builder(
                  controller: resultScroll,
                  itemExtent: 76,
                  itemCount: results.hits.length,
                  itemBuilder: (context, index) {
                    final hit = results.hits[index];
                    return ListTile(
                      key: ValueKey('search-hit-$index'),
                      selected: index == selected,
                      selectedTileColor: const Color(0xffeaf1fb),
                      dense: true,
                      contentPadding: EdgeInsets.only(
                        left: headings
                            ? 16 + (hit.block!.heading - 1) * 12
                            : 16,
                        right: 12,
                      ),
                      title: Text(
                        hit.label,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      subtitle: Text(
                        headings ? hit.text : hit.excerpt,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                      ),
                      onTap: busy || pending ? null : () => navigate(index),
                    );
                  },
                ),
              ),
              Padding(
                padding: const EdgeInsets.all(12),
                child: Text(
                  widget.spreadsheet
                      ? 'Literal text search. Replace edits formulas and text; calculated values are read-only.'
                      : 'Searches body text, headings, lists and table cells. Matches stay within a paragraph.',
                  style: const TextStyle(fontSize: 11, color: Colors.black54),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    _timer?.cancel();
    widget.model.removeListener(_workspaceChanged);
    widget.editor.removeListener(_editorChanged);
    query.dispose();
    replacement.dispose();
    queryFocus.dispose();
    resultScroll.dispose();
    super.dispose();
  }
}
