import 'dart:math' as math;
import 'document_fields.dart';
import 'document_formatting_marks.dart';
import 'document_links.dart';
import 'sheet_zoom.dart';
import 'document_page_setup.dart';
import 'document_revisions.dart';
import 'document_named_styles.dart';
import 'document_text_style.dart';
import 'document_paragraph_format.dart';
import 'document_paragraph_dialog.dart';
import 'draft_text_field.dart';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart' show OverflowBoxFit;
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'document_codec.dart';
import 'document_bookmarks.dart';
import 'document_edit_tracking.dart';
import 'document_tabs.dart';
import 'document_styles.dart';
import 'document_tabs_dialog.dart';
import 'document_table_view.dart';
import 'document_tables.dart';
import 'document_table_conversion.dart';
import 'document_table_conversion_dialog.dart';
import 'document_lists.dart';
import 'document_list_rules.dart';
import 'core.dart';
import 'workbook_block.dart';
import 'proofreading.dart';
import 'search.dart';

/// Command/Ctrl with + and − zoom the document (W17.02).
class ZoomDocumentIntent extends Intent {
  final bool up;
  const ZoomDocumentIntent(this.up);
}

const documentZoomSteps = [10, 25, 50, 75, 100, 125, 150, 200, 300, 400, 500];

const documentZoomShortcuts = <ShortcutActivator, Intent>{
  SingleActivator(LogicalKeyboardKey.equal, meta: true): ZoomDocumentIntent(
    true,
  ),
  SingleActivator(LogicalKeyboardKey.equal, control: true): ZoomDocumentIntent(
    true,
  ),
  SingleActivator(LogicalKeyboardKey.equal, meta: true, shift: true):
      ZoomDocumentIntent(true),
  SingleActivator(LogicalKeyboardKey.equal, control: true, shift: true):
      ZoomDocumentIntent(true),
  SingleActivator(LogicalKeyboardKey.numpadAdd, meta: true): ZoomDocumentIntent(
    true,
  ),
  SingleActivator(LogicalKeyboardKey.numpadAdd, control: true):
      ZoomDocumentIntent(true),
  SingleActivator(LogicalKeyboardKey.minus, meta: true): ZoomDocumentIntent(
    false,
  ),
  SingleActivator(LogicalKeyboardKey.minus, control: true): ZoomDocumentIntent(
    false,
  ),
  SingleActivator(LogicalKeyboardKey.numpadSubtract, meta: true):
      ZoomDocumentIntent(false),
  SingleActivator(LogicalKeyboardKey.numpadSubtract, control: true):
      ZoomDocumentIntent(false),
};

class UndoWorkspace extends Intent {
  final bool redo;
  const UndoWorkspace([this.redo = false]);
}

class DocumentView extends StatefulWidget {
  final q.QuillController controller;
  final VoidCallback undo, redo;
  final VoidCallback? onBookmarks;
  final WorkspaceModel? model;
  final void Function(String)? onContextAction;
  final void Function(String, String)? onOpenSource;
  final String ribbon;
  final ProofreadingController? proofreading;
  final void Function(WritingIssue, String)? onWritingFix;

  /// The zoom percentage, shared with the status bar.
  final ValueNotifier<int>? zoom;
  const DocumentView({
    super.key,
    required this.controller,
    required this.undo,
    required this.redo,
    this.onBookmarks,
    this.model,
    this.onContextAction,
    this.onOpenSource,
    this.ribbon = 'Home',
    this.proofreading,
    this.onWritingFix,
    this.zoom,
  });
  @override
  State<DocumentView> createState() => DocumentViewState();
}

class DocumentViewState extends State<DocumentView> {
  final _focus = FocusNode(), _scroll = ScrollController();
  final _editorKey = GlobalKey<q.EditorState>();

  // Zoom is kept per document for the session; Word doesn't save it.
  static final _zooms = <Object?, int>{};
  final _ownZoom = ValueNotifier<int>(100);
  ValueNotifier<int> get zoom => widget.zoom ?? _ownZoom;
  Object? _zoomDocument = const Object();
  Size? _viewport;

  void _restoreZoom() {
    final id = model?.workspace['id'];
    if (id == _zoomDocument) return;
    _zoomDocument = id;
    final saved = _zooms[id] ?? 100;
    if (zoom.value == saved) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) zoom.value = saved;
    });
  }

  void setZoom(int percent) {
    final value = clampZoom(percent, max: documentZoomSteps.last);
    _zooms[_zoomDocument] = value;
    zoom.value = value;
  }

  DocumentPageGeometry get _geometry =>
      DocumentPageGeometry.fromDocument(model?.workspace['document'] as Map?);

  /// Page Width: the page fills the view's width.
  void zoomToPageWidth() {
    final view = _viewport;
    if (view == null) return;
    setZoom(((view.width - 48) / _geometry.width * 100).floor());
  }

  /// Whole Page: one page fits the view.
  void zoomToWholePage() {
    final view = _viewport;
    if (view == null) return;
    final g = _geometry;
    setZoom(
      (math.min((view.width - 48) / g.width, (view.height - 64) / g.height) *
              100)
          .floor(),
    );
  }

  /// Show/Hide ¶, for the session.
  bool showMarks = false;
  void toggleMarks() => setState(() => showMarks = !showMarks);

  Map<int, String> _listLabels = {};
  q.Document? _configuredDocument;
  void _listsChanged() {
    if (!identical(_configuredDocument, controller.document)) {
      configureDocumentLists(
        controller.document,
        styleCatalog: () =>
            model?.workspace['document']?['attrs']?['wordStyles'] as Map?,
        sectionBoundaries: () => {
          for (final section
              in model?.workspace['document']?['attrs']?['pageLayout']?['sections']
                      as List? ??
                  [])
            if (section['endOfficeId'] is String)
              section['endOfficeId'] as String,
        },
      );
      _configuredDocument = controller.document;
    }
    _listLabels = documentListLabels(controller.document.toDelta());
  }

  @override
  void initState() {
    super.initState();
    _listsChanged();
    controller.addListener(_listsChanged);
  }

  @override
  void didUpdateWidget(DocumentView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != controller) {
      oldWidget.controller.removeListener(_listsChanged);
      controller.addListener(_listsChanged);
    }
    _listsChanged();
  }

  Future<void> _numbering(String action) async {
    final selection = controller.selection;
    var start = 1, format = 'decimal';
    final lines = listLines(controller.document.toDelta());
    final current = lines
        .where((l) => selection.start >= l.start && selection.start <= l.end)
        .firstOrNull;
    if (current?.attrs['list'] != 'ordered') {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Select a numbered paragraph first.')),
      );
      return;
    }
    final n = numberingForLine(current!.attrs);
    final level = current.attrs['indent'] as int? ?? 0;
    final config = n?['levels']?[level] as Map?;
    format =
        config?['format'] as String? ??
        ['decimal', 'lowerLetter', 'lowerRoman'][level % 3];
    if (action == 'set') {
      final result = await showDialog<(int, String)>(
        context: context,
        builder: (_) => _NumberingDialog(
          start: config?['start'] as int? ?? 1,
          format: config?['format'] as String? ?? 'decimal',
        ),
      );
      if (result == null || !mounted) return;
      start = result.$1;
      format = result.$2;
    }
    try {
      final delta = changeNumbering(
        controller.document.toDelta(),
        selection.start,
        start: start,
        format: format,
        continuePrevious: action == 'continue',
        instance: 'folio-${DateTime.now().microsecondsSinceEpoch}',
      );
      controller.compose(delta, selection, q.ChangeSource.local);
      _focus.requestFocus();
    } on FormatException catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(e.message)));
      }
    }
  }

  Future<void> editStyles() async {
    if (model == null) return;
    final selection = controller.selection;
    try {
      await model!.flush();
      final before = controller.document.toDelta();
      final data = await model!.core.call({'op': 'documentStyles'});
      if (!mounted) return;
      final document = model!.workspace['document'] as Map;
      final paths = selectedStylePaths(document, selection);
      final result = await showDialog<Map<String, dynamic>>(
        context: context,
        builder: (_) => DocumentStylesDialog(
          data: data,
          currentId: selectionStyleId(document, paths),
          paragraphs: paths.length,
        ),
      );
      if (result == null || !mounted) return;
      if (controller.document.toDelta() != before ||
          model!.revision != data['revision'] ||
          model!.workspace['id'] != data['workspaceId']) {
        throw const FormatException(
          'The document changed. Reopen Styles and try again.',
        );
      }
      await model!.mutate({
        'op': 'documentStyle',
        ...result,
        if (result['action'] == 'apply') 'paths': paths,
        'workspaceId': data['workspaceId'],
        'baseRevision': data['revision'],
      }, refreshDocument: true);
      controller.updateSelection(selection, q.ChangeSource.local);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(e is FormatException ? e.message : '$e')),
        );
      }
    } finally {
      if (mounted) _focus.requestFocus();
    }
  }

  Future<void> editParagraphFormat() async {
    try {
      await model?.flush();
      if (!mounted) return;
      final selection = controller.selection;
      final before = controller.document.toDelta();
      final revision = model?.revision;
      final lines = listLines(before)
          .where(
            (l) =>
                l.end >= selection.start &&
                (selection.isCollapsed
                    ? l.start <= selection.start
                    : l.start < selection.end),
          )
          .toList();
      if (lines.isEmpty) return;
      changeParagraphFormat(before, selection, {});
      final patch = await showDialog<Map<String, dynamic>>(
        context: context,
        builder: (_) => DocumentParagraphDialog(
          formats: [for (final l in lines) paragraphFormatForAttrs(l.attrs)],
          sample: before.slice(lines.first.start, lines.first.end + 1),
        ),
      );
      if (patch == null || patch.isEmpty || !mounted) return;
      if (before != controller.document.toDelta() ||
          revision != model?.revision) {
        throw const FormatException(
          'The document changed. Reopen Paragraph and try again.',
        );
      }
      final change = changeParagraphFormat(before, selection, patch);
      if (change.isEmpty) return;
      if (model != null) {
        final document = deltaToDocument(
          before.compose(change),
          attributes: model!.workspace['document']['attrs'] as Map?,
        );
        await model!.mutate({
          'op': 'document',
          'document': document,
          'baseRevision': revision,
        }, refreshDocument: true);
      } else {
        controller.compose(change, selection, q.ChangeSource.local);
      }
      controller.updateSelection(selection, q.ChangeSource.local);
      _focus.requestFocus();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(e is FormatException ? e.message : '$e')),
        );
      }
    }
  }

  Future<void> editTabStops() async {
    try {
      await model?.flush();
      if (!mounted) return;
      final selection = controller.selection;
      final before = controller.document.toDelta();
      final revision = model?.revision;
      final selected = listLines(before)
          .where(
            (l) =>
                l.end >= selection.start &&
                (selection.isCollapsed
                    ? l.start <= selection.start
                    : l.start < selection.end),
          )
          .toList();
      if (selected.isEmpty) return;
      // Reject preserved objects before opening a misleading paragraph preview.
      changeTabStops(before, selection, const []);
      final first = effectiveTabStops(
        listMetadata(selected.first.attrs['token'])['tabs'],
      );
      final firstStops = jsonEncode(first.map((s) => s.toJson()).toList());
      final oldInterval = model == null
          ? DocumentTabSettings.intervalOf(context)
          : documentTabInterval(model!.workspace['document']);
      final result = await showDialog<TabStopsResult>(
        context: context,
        builder: (_) => DocumentTabsDialog(
          stops: first,
          interval: oldInterval,
          paragraphCount: selected.length,
          sample: before.slice(selected.first.start, selected.first.end + 1),
          editDefault: model != null,
          mixed: selected
              .skip(1)
              .any(
                (l) =>
                    jsonEncode(
                      effectiveTabStops(
                        listMetadata(l.attrs['token'])['tabs'],
                      ).map((s) => s.toJson()).toList(),
                    ) !=
                    firstStops,
              ),
        ),
      );
      if (result == null || !mounted) return;
      if (before != controller.document.toDelta() ||
          revision != model?.revision) {
        throw const FormatException(
          'The document changed. Reopen Tab stops and try again.',
        );
      }
      final change = changeTabStops(before, selection, result.stops);
      if (change.isEmpty && oldInterval == result.interval) return;
      if (model != null) {
        final attrs = Map<String, dynamic>.from(
          model!.workspace['document']['attrs'] as Map? ?? {},
        );
        if (oldInterval != result.interval) {
          attrs['defaultTabStop'] = result.interval;
        }
        final document = deltaToDocument(
          before.compose(change),
          attributes: attrs.isEmpty ? null : attrs,
        );
        await model!.mutate({
          'op': 'document',
          'document': document,
          'baseRevision': revision,
        }, refreshDocument: true);
      } else {
        controller.compose(change, selection, q.ChangeSource.local);
      }
      controller.updateSelection(selection, q.ChangeSource.local);
      _focus.requestFocus();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(e is FormatException ? e.message : '$e')),
        );
      }
    }
  }

  /// Insert or edit the hyperlink at the selection (Ctrl/Cmd+K).
  Future<void> editLink() async {
    try {
      await model?.flush();
      if (!mounted) return;
      final before = controller.document.toDelta();
      final revision = model?.revision;
      var selection = controller.selection;
      final existing = selection.isCollapsed
          ? linkRangeAt(before, selection.start)
          : null;
      if (existing != null) {
        selection = TextSelection(
          baseOffset: existing.start,
          extentOffset: existing.end,
        );
      }
      final slice = before.slice(selection.start, selection.end).toList();
      final plain = slice.every((op) => op.data is String);
      final text = slice.map((op) => op.data is String ? op.data : '').join();
      if (!plain || text.contains('\n')) {
        throw const FormatException(
          'Select text within one paragraph to link it.',
        );
      }
      final document = model?.workspace['document'] as Map?;
      final result = await showDialog<LinkResult>(
        context: context,
        builder: (_) => DocumentLinkDialog(
          text: text,
          address: selection.isCollapsed
              ? ''
              : linkAt(before, selection.start) ?? '',
          existing: existing != null || linkAt(before, selection.start) != null,
          places: document == null ? const [] : linkPlaces(document),
        ),
      );
      if (result == null || !mounted) return;
      if (before != controller.document.toDelta() ||
          revision != model?.revision) {
        throw const FormatException(
          'The document changed. Select the text and try again.',
        );
      }
      if (result.remove) {
        final range = selection.isCollapsed
            ? null
            : TextRange(start: selection.start, end: selection.end);
        if (range != null) {
          controller.compose(
            removeLinkDelta(range),
            selection,
            q.ChangeSource.local,
          );
        }
        _focus.requestFocus();
        return;
      }
      var href = result.href;
      final place = result.place;
      if (place != null && place.bookmark == null) {
        // A heading gets Word's hidden bookmark the first time it is linked.
        final name = headingBookmarkName(
          place.label,
          documentBookmarks(document!).map((b) => b['name'] as String),
        );
        await model!.mutate({
          'op': 'bookmark',
          'action': 'set',
          'name': name,
          'hidden': true,
          'start': place.start,
          'end': place.end,
          'workspaceId': model!.workspace['id'],
          'baseRevision': revision,
        }, refreshDocument: true);
        if (!mounted) return;
        href = '#$name';
      }
      final change = setLinkDelta(
        controller.document.toDelta(),
        selection.start,
        selection.end,
        result.text,
        href,
      );
      final end = selection.start + result.text.length;
      controller.compose(
        change,
        TextSelection.collapsed(offset: end),
        q.ChangeSource.local,
      );
      controller.updateSelection(
        TextSelection.collapsed(offset: end),
        q.ChangeSource.local,
      );
      _focus.requestFocus();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(e is FormatException ? e.message : '$e')),
        );
      }
    }
  }

  /// Follows [href]: web and mail links open outside Folio, and a link to a
  /// bookmark moves the caret there.
  Future<void> followLink(String href) async {
    void say(String message) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(message)));
      }
    }

    switch (linkKind(href)) {
      case LinkKind.internal:
        final document = model?.workspace['document'] as Map?;
        final name = href.substring(1).toLowerCase();
        final bookmark = document == null
            ? null
            : documentBookmarks(document)
                  .where((b) => (b['name'] as String).toLowerCase() == name)
                  .firstOrNull;
        if (bookmark == null) {
          say('This link\'s destination is no longer in the document.');
          return;
        }
        await revealBookmark(document!, bookmark);
      case LinkKind.web || LinkKind.mail:
        await openExternalLink(Uri.parse(href));
      case LinkKind.blocked:
        say('Folio only follows web, email and in-document links.');
    }
  }

  void reveal(int start, int end, {bool requestFocus = false}) {
    if (requestFocus) _focus.requestFocus();
    final limit = controller.document.length - 1;
    controller.updateSelection(
      TextSelection(
        baseOffset: start.clamp(0, limit),
        extentOffset: end.clamp(0, limit),
      ),
      q.ChangeSource.local,
    );
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final render = _editorKey.currentState?.renderEditor;
      if (!mounted || render == null) return;
      render.showOnScreen(
        rect: render
            .getLocalRectForCaret(TextPosition(offset: start.clamp(0, limit)))
            .inflate(24),
        duration: const Duration(milliseconds: 150),
      );
    });
  }

  Future<void> revealBookmark(Map document, Map bookmark) async {
    final locations = DocumentBookmarkIndex(document);
    final start = bookmark['start'] as int, end = bookmark['end'] as int;
    final selected = locations.editorSelection(start, end);
    if (selected == null) {
      throw StateError('The bookmark destination is unavailable.');
    }
    reveal(selected.start, selected.end, requestFocus: true);
    final paragraph = locations.paragraph(start)!;
    final path = paragraph.path;
    final top = document['content'][path.first] as Map;
    if (!paragraph.embedded || top['type'] != 'table' || path.length < 4) {
      return;
    }
    final table = DocumentTable(top);
    final cell = table.cells
        .where((p) => p.row == path[1] && p.index == path[2])
        .firstOrNull;
    if (cell == null) return;
    final cellIndex = DocumentBookmarkIndex({
      'type': 'doc',
      'content': cell.cell['content'],
    });
    final localParagraph = cellIndex.atPath(path.sublist(3));
    if (localParagraph == null) return;
    final cellStart = paragraph.start - localParagraph.start;
    final cellEnd = cellIndex.text.length - 1;
    final selection = cellIndex.editorSelection(
      start - cellStart,
      (end - cellStart).clamp(0, cellEnd),
    );
    await FolioEmbedBuilder(model: model, onOpen: onOpenSource).editTableAt(
      context,
      controller,
      selected.start,
      top,
      cell.row,
      cell.col,
      initialSelection: selection,
    );
    if (mounted) _focus.requestFocus();
  }

  q.QuillController get controller => widget.controller;
  WorkspaceModel? get model => widget.model;
  String get ribbon => widget.ribbon;
  ProofreadingController? get proofreading => widget.proofreading;
  void Function(String)? get onContextAction => widget.onContextAction;
  void Function(String, String)? get onOpenSource => widget.onOpenSource;
  void Function(WritingIssue, String)? get onWritingFix => widget.onWritingFix;
  VoidCallback get undo => widget.undo;
  VoidCallback get redo => widget.redo;
  @override
  void dispose() {
    controller.removeListener(_listsChanged);
    _ownZoom.dispose();
    _focus.dispose();
    _scroll.dispose();
    super.dispose();
  }

  Widget _contextMenu(BuildContext context, q.QuillRawEditorState raw) {
    final selection = controller.selection;
    final suggestions = <ContextMenuButtonItem>[
      for (final issue in proofreading?.issues ?? <WritingIssue>[])
        if (selection.isCollapsed
            ? selection.start >= issue.start && selection.start < issue.end
            : selection.start < issue.end && selection.end > issue.start)
          for (final replacement in issue.suggestions.take(4))
            ContextMenuButtonItem(
              label: replacement.isEmpty ? 'Remove ${issue.text}' : replacement,
              onPressed: () {
                raw.hideToolbar();
                onWritingFix?.call(issue, replacement);
              },
            ),
    ];
    ContextMenuButtonItem action(String label, String name) =>
        ContextMenuButtonItem(
          label: label,
          onPressed: () {
            raw.hideToolbar();
            onContextAction?.call(name);
          },
        );
    final link = linkAt(controller.document.toDelta(), selection.start);
    ContextMenuButtonItem run(String label, VoidCallback onPressed) =>
        ContextMenuButtonItem(
          label: label,
          onPressed: () {
            raw.hideToolbar();
            onPressed();
          },
        );
    final actions = <ContextMenuButtonItem>[
      ...raw.contextMenuButtonItems,
      if (link != null) ...[
        run('Open link', () => followLink(link)),
        run('Edit link…', editLink),
        run('Remove link', () {
          final range = linkRangeAt(
            controller.document.toDelta(),
            selection.start,
          );
          if (range != null) {
            controller.compose(
              removeLinkDelta(range),
              selection,
              q.ChangeSource.local,
            );
          }
        }),
      ] else
        run('Link…', editLink),
      action('New comment', 'comment'),
      action('Spelling & grammar', 'proofread'),
      action('Check grammar with AI', 'grammar'),
      action('Ask assistant', 'ask'),
      action('Use workbook data', 'workbook'),
    ];
    return AdaptiveTextSelectionToolbar(
      anchors: raw.contextMenuAnchors,
      children: [
        // Quill dismisses its overlay on a desktop outside pointer-down.
        // Its menu buttons must belong to the same tap region as the editor
        // so mouse-up can activate the suggestion before the menu is removed.
        for (final button in AdaptiveTextSelectionToolbar.getAdaptiveButtons(
          context,
          suggestions,
        ))
          TextFieldTapRegion(child: button),
        if (suggestions.isNotEmpty)
          const TextFieldTapRegion(
            child: Divider(
              key: Key('proofreading-menu-divider'),
              height: 13,
              thickness: 1,
              indent: 4,
              endIndent: 4,
              color: Color(0xffc5cbd3),
            ),
          ),
        for (final button in AdaptiveTextSelectionToolbar.getAdaptiveButtons(
          context,
          actions,
        ))
          TextFieldTapRegion(child: button),
      ],
    );
  }

  void insertTable() {
    final node = {
      'type': 'table',
      'content': List.generate(
        3,
        (r) => newDocumentTableRow(
          List.generate(
            3,
            (_) => {
              'type': r == 0 ? 'tableHeader' : 'tableCell',
              'attrs': {'colspan': 1, 'rowspan': 1},
              'content': [
                {'type': 'paragraph'},
              ],
            },
          ),
        ),
      ),
    };
    final index = controller.selection.baseOffset.clamp(
      0,
      controller.document.length - 1,
    );
    controller.replaceText(
      index,
      0,
      q.BlockEmbed('folioNode', jsonEncode(node)),
      TextSelection.collapsed(offset: index + 1),
    );
  }

  Future<void> convertSelectionToTable() async {
    try {
      final before = controller.document.toDelta();
      final selected = controller.selection;
      final captured = TextTableSelection(before, selected.start, selected.end);
      final result = await showDialog<Map<String, dynamic>>(
        context: context,
        builder: (_) => DocumentTableConversionDialog(selection: captured),
      );
      if (!mounted || result == null) return;
      if (jsonEncode(controller.document.toDelta()) != jsonEncode(before)) {
        throw const FormatException(
          'The document changed. Select the text and try again.',
        );
      }
      final edit = captured.replaceWith(result);
      controller.compose(
        edit.change,
        TextSelection.collapsed(offset: edit.caret),
        q.ChangeSource.local,
      );
      // Quill 11 transforms the prior selection in compose, ignoring the
      // requested caret. Collapse it explicitly so typing keeps the new table.
      controller.updateSelection(
        TextSelection.collapsed(
          offset: edit.caret.clamp(0, controller.document.length - 1),
        ),
        q.ChangeSource.local,
      );
      _focus.requestFocus();
    } on FormatException catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(e.message)));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    _restoreZoom();
    final page = DocumentPageGeometry.fromDocument(
      model?.workspace['document'] as Map?,
    );
    return Column(
      children: [
        Container(
          color: const Color(0xfffafafa),
          constraints: const BoxConstraints(minHeight: 84),
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (ribbon == 'Home')
                DocumentTextToolbar(
                  controller: controller,
                  focus: _focus,
                  styles: () =>
                      model?.workspace['document']?['attrs']?['wordStyles']
                          as Map?,
                ),
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 4,
                children: [
                  TextButton(
                    key: const Key('document-styles'),
                    onPressed: model == null ? null : editStyles,
                    child: const Text('Styles…'),
                  ),
                  IconButton(
                    key: const Key('document-formatting-marks'),
                    tooltip: 'Show/Hide ¶ (Ctrl+Shift+8)',
                    isSelected: showMarks,
                    onPressed: toggleMarks,
                    icon: const Icon(
                      Icons.format_textdirection_l_to_r,
                      size: 19,
                    ),
                  ),
                  IconButton(
                    key: const Key('document-link'),
                    tooltip: 'Link (Ctrl+K)',
                    onPressed: editLink,
                    icon: const Icon(Icons.link, size: 19),
                  ),
                  IconButton(
                    key: const Key('insert-table'),
                    tooltip: 'Insert table',
                    onPressed: insertTable,
                    icon: const Icon(Icons.table_chart_outlined, size: 19),
                  ),
                  TextButton(
                    key: const Key('text-to-table'),
                    onPressed: convertSelectionToTable,
                    child: const Text('Text to table'),
                  ),
                  TextButton.icon(
                    onPressed: () => onContextAction?.call('workbook'),
                    icon: const Icon(Icons.add_link, size: 17),
                    label: const Text('Workbook data'),
                  ),
                  PopupMenuButton<String>(
                    key: const Key('document-numbering-menu'),
                    tooltip: 'Numbering',
                    onSelected: _numbering,
                    itemBuilder: (_) => const [
                      PopupMenuItem(
                        value: 'restart',
                        child: Text('Restart at 1'),
                      ),
                      PopupMenuItem(
                        value: 'continue',
                        child: Text('Continue previous list'),
                      ),
                      PopupMenuItem(
                        value: 'set',
                        child: Text('Set numbering…'),
                      ),
                    ],
                    child: const Padding(
                      padding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                      child: Text('Numbering ▾'),
                    ),
                  ),
                  TextButton(
                    key: const Key('document-paragraph'),
                    onPressed: editParagraphFormat,
                    child: const Text('Paragraph…'),
                  ),
                  TextButton(
                    key: const Key('document-tab-stops'),
                    onPressed: editTabStops,
                    child: const Text('Tab stops…'),
                  ),
                  TextButton.icon(
                    key: const Key('document-bookmarks'),
                    onPressed: widget.onBookmarks,
                    icon: const Icon(Icons.bookmark_border, size: 17),
                    label: const Text('Bookmarks…'),
                  ),
                  TextButton.icon(
                    onPressed: () => onContextAction?.call('ask'),
                    icon: const Icon(Icons.auto_awesome, size: 17),
                    label: const Text('Ask about selection'),
                  ),
                ],
              ),
            ],
          ),
        ),
        Expanded(
          child: ValueListenableBuilder<int>(
            valueListenable: zoom,
            builder: (context, percent, editor) => DocumentPageFrame(
              geometry: page,
              zoom: percent / 100,
              onViewport: (size) => _viewport = size,
              child: Stack(
                children: [
                  editor!,
                  if (showMarks)
                    Positioned.fill(
                      child: IgnorePointer(
                        child: FormattingMarksOverlay(
                          key: const Key('formatting-marks'),
                          controller: controller,
                          editorKey: _editorKey,
                        ),
                      ),
                    ),
                ],
              ),
            ),
            child: q.QuillEditor.basic(
              key: const Key('document-editor'),
              focusNode: _focus,
              scrollController: _scroll,
              controller: controller,
              config: q.QuillEditorConfig(
                editorKey: _editorKey,
                contextMenuBuilder: _contextMenu,
                scrollable: false,
                textSpanBuilder:
                    proofreading?.spanBuilder ?? q.defaultSpanBuilder,
                placeholder: 'Start writing…',
                onLaunchUrl: followLink,
                transformLink: (link) => link,
                // Pinned Quill 11.6.0; the callback has widget coverage.
                // ignore: experimental_member_use
                customLeadingBlockBuilder: (node, config) {
                  final label = _listLabels[node.documentOffset];
                  if (label == null) return null;
                  return Container(
                    key: ValueKey('list-marker-${node.documentOffset}'),
                    width: config.width,
                    padding: EdgeInsetsDirectional.only(
                      end: config.padding ?? 0,
                    ),
                    alignment: AlignmentDirectional.topEnd,
                    child: OverflowBox(
                      fit: OverflowBoxFit.deferToChild,
                      alignment: AlignmentDirectional.topEnd,
                      minWidth: 0,
                      maxWidth: double.infinity,
                      child: Text(
                        label,
                        style: config.style,
                        maxLines: 1,
                        softWrap: false,
                      ),
                    ),
                  );
                },
                minHeight: page.contentHeight,
                customStyles: documentEditorStyles(),
                embedBuilders: [
                  const PageFieldEmbedBuilder(),
                  FolioEmbedBuilder(model: model, onOpen: onOpenSource),
                ],
                customShortcuts: {
                  ...searchShortcuts,
                  ...bookmarkShortcuts,
                  ...documentMarkShortcuts,
                  ...linkShortcuts,
                  ...documentZoomShortcuts,
                  ...formattingMarkShortcuts,
                  const SingleActivator(LogicalKeyboardKey.keyZ, meta: true):
                      const UndoWorkspace(),
                  const SingleActivator(
                    LogicalKeyboardKey.keyZ,
                    meta: true,
                    shift: true,
                  ): const UndoWorkspace(
                    true,
                  ),
                },
                customActions: {
                  ...documentMarkActions(controller, _focus),
                  ToggleFormattingMarks: CallbackAction<ToggleFormattingMarks>(
                    onInvoke: (_) {
                      toggleMarks();
                      return null;
                    },
                  ),
                  ZoomDocumentIntent: CallbackAction<ZoomDocumentIntent>(
                    onInvoke: (intent) {
                      setZoom(
                        stepZoom(
                          zoom.value,
                          up: intent.up,
                          steps: documentZoomSteps,
                        ),
                      );
                      return null;
                    },
                  ),
                  EditLinkIntent: CallbackAction<EditLinkIntent>(
                    onInvoke: (_) {
                      editLink();
                      return null;
                    },
                  ),
                  UndoWorkspace: CallbackAction<UndoWorkspace>(
                    onInvoke: (intent) {
                      intent.redo ? redo() : undo();
                      return null;
                    },
                  ),
                },
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class FolioEmbedBuilder extends q.EmbedBuilder {
  final WorkspaceModel? model;
  final void Function(String, String)? onOpen;
  FolioEmbedBuilder({this.model, this.onOpen});
  @override
  String get key => 'folioNode';
  @override
  String toPlainText(q.Embed node) => nodeText(jsonDecode(node.value.data));
  @override
  Widget build(BuildContext context, q.EmbedContext embedContext) {
    final node = jsonDecode(embedContext.node.value.data) as Map;
    if (node['attrs']?['hiddenReviewMarker'] == true) {
      return const SizedBox.shrink();
    }
    if (node['type'] == 'workbookBlock' && model != null) {
      return WorkbookBlock(
        model: model!,
        attributes: node['attrs'],
        onOpen: onOpen,
      );
    }
    if (node['type'] == 'table') {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            DocumentTableView(
              node: node,
              onCell: (row, col) =>
                  _editTable(context, embedContext, node, row, col),
            ),
            Wrap(
              children: [
                TextButton(
                  key: const Key('table-to-text'),
                  onPressed: () => _convertTable(context, embedContext, node),
                  child: const Text(
                    'Convert to text',
                    style: TextStyle(fontSize: 10),
                  ),
                ),
                TextButton(
                  onPressed: () =>
                      _editTable(context, embedContext, node, 0, 0),
                  child: const Text(
                    'Edit table',
                    style: TextStyle(fontSize: 10),
                  ),
                ),
              ],
            ),
          ],
        ),
      );
    }
    return Container(
      padding: const EdgeInsets.all(15),
      color: const Color(0xfff3f0e6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            node['attrs']?['label']?.toString() ??
                'Preserved ${node['type']} block',
            style: const TextStyle(fontSize: 10, color: Color(0xff978d76)),
          ),
          const SizedBox(height: 8),
          if (node['attrs']?['revisionPreview'] is List)
            RevisionPreview(
              spans: node['attrs']['revisionPreview'],
              selectable: false,
            )
          else
            Text(nodeText(node), style: const TextStyle(fontSize: 14)),
        ],
      ),
    );
  }

  Future<void> _editTable(
    BuildContext context,
    q.EmbedContext embed,
    Map original,
    int row,
    int col,
  ) async {
    await editTableAt(
      context,
      embed.controller,
      embed.node.documentOffset,
      original,
      row,
      col,
    );
  }

  Future<void> editTableAt(
    BuildContext context,
    q.QuillController controller,
    int index,
    Map original,
    int row,
    int col, {
    TextSelection? initialSelection,
  }) async {
    final messenger = ScaffoldMessenger.of(context);
    final parentEditor = context
        .findAncestorStateOfType<q.QuillRawEditorState>();
    void changed() {
      if (messenger.mounted) {
        messenger.showSnackBar(
          const SnackBar(
            content: Text('The table changed. Reopen it and try again.'),
          ),
        );
      }
    }

    LogicalTextEdits? tableEdits;
    final initialDocument = controller.document;
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => DocumentTableDialog(
        node: original,
        model: model,
        row: row,
        col: col,
        initialSelection: initialSelection,
        onTextEdits: (edits) => tableEdits = edits,
        embedBuilders: [const PageFieldEmbedBuilder(), this],
      ),
    );
    if (context.mounted && parentEditor?.mounted == true) {
      parentEditor!.widget.config.focusNode.requestFocus();
    }
    if (result == null || jsonEncode(result) == jsonEncode(original)) return;
    // Do not replace another block if this document changed while the dialog was open.
    if (!identical(controller.document, initialDocument) ||
        index >= controller.document.length - 1) {
      changed();
      return;
    }
    final leaf = controller.document.querySegmentLeafNode(index).leaf;
    if (leaf is! q.Embed ||
        leaf.value.type != 'folioNode' ||
        leaf.value.data != jsonEncode(original)) {
      changed();
      return;
    }
    if (result['type'] == 'doc') {
      DocumentEditCapture.apply(
        controller,
        null,
        () => _replaceTableWithContent(controller, index, result),
      );
      return;
    }
    final offset = DocumentBookmarkIndex(
      deltaToDocument(controller.document.toDelta()),
    ).logicalOffset(index);
    DocumentEditCapture.apply(
      controller,
      tableEdits == null || offset == null
          ? null
          : [
              for (final edit in tableEdits!)
                {
                  ...edit,
                  'start': edit['start'] + offset,
                  'end': edit['end'] + offset,
                },
            ],
      () => controller.replaceText(
        index,
        1,
        q.BlockEmbed('folioNode', jsonEncode(result)),
        TextSelection.collapsed(offset: index + 1),
      ),
    );
  }

  Future<void> _convertTable(
    BuildContext context,
    q.EmbedContext embed,
    Map original,
  ) async {
    final before = embed.controller.document.toDelta();
    final index = embed.node.documentOffset;
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => DocumentTableConversionDialog(table: original),
    );
    if (!context.mounted || result == null) return;
    if (jsonEncode(embed.controller.document.toDelta()) != jsonEncode(before)) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text(
            'The document changed. Try converting the table again.',
          ),
        ),
      );
      return;
    }
    _replaceTableWithContent(embed.controller, index, result);
  }

  void _replaceTableWithContent(
    q.QuillController controller,
    int index,
    Map result,
  ) {
    final replacement = documentToDelta(result);
    // The embed's mandatory newline is replaced along with the table itself.
    final edit = TableConversionEdit(index, 2, replacement);
    controller.compose(
      edit.change,
      TextSelection.collapsed(offset: edit.caret),
      q.ChangeSource.local,
    );
    controller.updateSelection(
      TextSelection.collapsed(
        offset: edit.caret.clamp(0, controller.document.length - 1),
      ),
      q.ChangeSource.local,
    );
  }
}

class _NumberingDialog extends StatefulWidget {
  final int start;
  final String format;
  const _NumberingDialog({required this.start, required this.format});
  @override
  State<_NumberingDialog> createState() => _NumberingDialogState();
}

class _NumberingDialogState extends State<_NumberingDialog> {
  late final value = TextEditingController(text: '${widget.start}');
  late String format = formats.containsKey(widget.format)
      ? widget.format
      : 'decimal';
  String? error;
  static const formats = {
    'decimal': '1, 2, 3',
    'lowerLetter': 'a, b, c',
    'upperLetter': 'A, B, C',
    'lowerRoman': 'i, ii, iii',
    'upperRoman': 'I, II, III',
  };
  @override
  void dispose() {
    value.dispose();
    super.dispose();
  }

  void apply() {
    final start = int.tryParse(value.text.trim());
    if (start == null || start < 0 || start > 2147483647) {
      setState(() => error = 'Enter a number from 0 to 2,147,483,647.');
      return;
    }
    Navigator.pop(context, (start, format));
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Set numbering'),
    content: SizedBox(
      width: 320,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          DropdownButtonFormField<String>(
            key: const Key('numbering-format'),
            initialValue: format,
            decoration: const InputDecoration(labelText: 'Number style'),
            items: [
              for (final entry in formats.entries)
                DropdownMenuItem(value: entry.key, child: Text(entry.value)),
            ],
            onChanged: (v) => setState(() => format = v!),
          ),
          const SizedBox(height: 16),
          draftTextField(
            key: const Key('numbering-start'),
            controller: value,
            autofocus: true,
            decoration: InputDecoration(
              labelText: 'Start at',
              errorText: error,
            ),
            onSubmitted: (_) => apply(),
          ),
          const SizedBox(height: 12),
          const Text(
            'Applies from this paragraph through the following list items.',
            style: TextStyle(fontSize: 12),
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
        key: const Key('numbering-apply'),
        onPressed: apply,
        child: const Text('Apply'),
      ),
    ],
  );
}
