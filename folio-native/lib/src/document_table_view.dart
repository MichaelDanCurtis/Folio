import 'document_named_styles.dart';
import 'document_revisions.dart';
import 'document_styles.dart';
import 'core.dart';
import 'document_text_style.dart';
import 'document_paragraph_format.dart';
import 'document_paragraph_dialog.dart';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'document_table_resize.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'document_codec.dart';
import 'document_bookmarks.dart';
import 'document_edit_tracking.dart';
import 'document_tabs.dart';
import 'document_list_rules.dart';
import 'document_lists.dart';
import 'document_tables.dart';
import 'document_table_geometry.dart';
import 'document_table_format_dialog.dart';
import 'document_table_conversion_dialog.dart';
import 'document_table_sort.dart';
import 'document_table_sort_dialog.dart';

class DocumentTableView extends StatelessWidget {
  final Map node;
  final void Function(int, int)? onCell;
  final void Function(int, int)? onAccessibleCell, onExtendCell;
  final TableArea? selection;

  /// Called with the table after a border is dragged (W07.06).
  final void Function(Map<String, dynamic> next)? onResize;
  const DocumentTableView({
    super.key,
    required this.node,
    this.onCell,
    this.onAccessibleCell,
    this.onExtendCell,
    this.selection,
    this.onResize,
  });
  @override
  Widget build(BuildContext context) {
    final table = DocumentTable(node);
    final widths = List<num>.from(
      node['attrs']?['columnWidths'] ?? List.filled(table.columns, 2400),
    );
    final geometry = onResize == null ? null : TableGridGeometry();
    final Widget cells = _SpanTableLayout(
      geometry: geometry,
      positions: table.cells,
      rowHeights: [
        for (final row in table.rows) row['attrs']?['height'] as Map?,
      ],
      widths: widths,
      children: [
        for (final p in table.cells)
          Semantics(
            key: ValueKey('table-cell-semantics-${p.row}-${p.col}'),
            label:
                'Table row ${p.row + 1}, column ${p.col + 1}${p.rows > 1 || p.cols > 1 ? ', spans ${p.rows} rows and ${p.cols} columns' : ''}',
            selected: selection?.includes(p) ?? false,
            onTap: (onAccessibleCell ?? onCell) == null
                ? null
                : () => (onAccessibleCell ?? onCell)!(p.row, p.col),
            customSemanticsActions: onExtendCell == null
                ? null
                : {
                    const CustomSemanticsAction(
                      label: 'Extend selection to this cell',
                    ): () =>
                        onExtendCell!(p.row, p.col),
                  },
            child: Material(
              color: (selection?.includes(p) ?? false)
                  ? const Color(0xffe4efff)
                  : _fill(p.cell) ??
                        (p.cell['type'] == 'tableHeader'
                            ? const Color(0xffeff2f6)
                            : Colors.white),
              child: InkWell(
                excludeFromSemantics: true,
                key: ValueKey('table-cell-${p.row}-${p.col}'),
                onTap: onCell == null ? null : () => onCell!(p.row, p.col),
                child: Container(
                  decoration: BoxDecoration(
                    border: Border.all(
                      color: const Color(0xffb9c1cc),
                      width: .5,
                    ),
                  ),
                  child: ClipRect(
                    child: OverflowBox(
                      alignment: _vertical(p.cell),
                      minHeight: 0,
                      maxHeight: double.infinity,
                      fit: OverflowBoxFit.deferToChild,
                      child: Padding(
                        padding: const EdgeInsets.all(8),
                        child: DocumentBlockContent(p.cell),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
      ],
    );
    final grid = geometry == null
        ? cells
        : Stack(
            children: [
              cells,
              Positioned.fill(
                child: TableResizeLayer(
                  key: const Key('table-resize'),
                  geometry: geometry,
                  onColumn: (column, dx) => onResize!(
                    resizeTableColumns(
                      node,
                      widths,
                      column,
                      dx,
                      geometry.xs.last,
                    ),
                  ),
                  onRow: (row, height) =>
                      onResize!(resizeTableRow(node, row, height)),
                ),
              ),
            ],
          );
    return LayoutBuilder(
      builder: (context, constraints) {
        final available = constraints.hasBoundedWidth
            ? constraints.maxWidth
            : 680.0;
        final width = node['attrs']?['layout'] == 'fixed'
            ? widths.fold<double>(0, (a, b) => a + b) / 15
            : available;
        final sized = SizedBox(width: width, child: grid);
        if (width > available) {
          return SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: sized,
          );
        }
        final alignment = switch (node['attrs']?['alignment']) {
          'center' => Alignment.topCenter,
          'right' => Alignment.topRight,
          _ => Alignment.topLeft,
        };
        return Align(alignment: alignment, child: sized);
      },
    );
  }

  Color? _fill(Map cell) {
    final fill = cell['attrs']?['fill'];
    if (fill == 'auto') return Colors.white;
    final value = fill is String ? int.tryParse(fill, radix: 16) : null;
    return value == null ? null : Color(0xff000000 | value);
  }

  Alignment _vertical(Map cell) => switch (cell['attrs']?['verticalAlign']) {
    'center' => Alignment.centerLeft,
    'bottom' => Alignment.bottomLeft,
    _ => Alignment.topLeft,
  };
}

/// Shared static document renderer for table cells and review projections.
class DocumentBlockContent extends StatelessWidget {
  final Map cell;
  final Widget Function(Map block, Widget content)? decorate;
  const DocumentBlockContent(this.cell, {super.key, this.decorate});
  Color? color(dynamic value) {
    if (value is! String) return null;
    final hex = value.replaceFirst('#', '');
    final n = int.tryParse(hex, radix: 16);
    return n == null ? null : Color(n | (hex.length == 6 ? 0xff000000 : 0));
  }

  @override
  Widget build(BuildContext context) {
    final blocks = cell['content'] as List? ?? [];
    final doc = {'type': 'doc', 'content': blocks};
    final labels = documentListLabels(documentToDelta(doc));
    var offset = 0;
    final children = <Widget>[];
    Map? previous;
    for (final block in blocks) {
      if (block['attrs']?['hiddenReviewMarker'] == true) continue;
      final content = buildBlock(block, labels[offset], previous);
      children.add(decorate?.call(block as Map, content) ?? content);
      previous = block as Map;
      offset += documentToDelta({
        'type': 'doc',
        'content': [block],
      }).length;
    }
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: children,
    );
  }

  Widget buildBlock(Map block, String? label, Map? previous) {
    if (block['attrs']?['revisionPreview'] is List) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: RevisionPreview(spans: block['attrs']['revisionPreview']),
      );
    }
    if (block['type'] == 'table') return DocumentTableView(node: block);
    if (!['paragraph', 'heading'].contains(block['type'])) {
      return Text(nodeText(block), style: const TextStyle(fontSize: 14));
    }
    final inheritedRun =
        block['attrs']?['paragraphFormat']?['inheritedRun'] as Map? ?? {};
    final spans = <InlineSpan>[if (label != null) TextSpan(text: '$label ')];
    for (final inline in block['content'] as List? ?? []) {
      var style = const TextStyle();
      for (final mark in inline['marks'] as List? ?? []) {
        switch (mark['type']) {
          case 'bold':
            style = style.copyWith(fontWeight: FontWeight.bold);
          case 'italic':
            style = style.copyWith(fontStyle: FontStyle.italic);
          case 'underline':
            style = style.copyWith(
              decoration: TextDecoration.combine([
                if (style.decoration != null) style.decoration!,
                TextDecoration.underline,
              ]),
            );
          case 'strike':
            style = style.copyWith(
              decoration: TextDecoration.combine([
                if (style.decoration != null) style.decoration!,
                TextDecoration.lineThrough,
              ]),
            );
          case 'textStyle':
            final a = mark['attrs'];
            style = style.copyWith(
              fontSize: double.tryParse('${a?['fontSize']}'),
              fontFamily: a?['fontFamily'],
              color: color(a?['color']),
            );
          case 'highlight':
            style = style.copyWith(
              backgroundColor: color(mark['attrs']?['color']),
            );
          case 'link':
            style = style.copyWith(
              color: const Color(0xff185abd),
              decoration: TextDecoration.underline,
            );
        }
      }
      final direct = <String, dynamic>{};
      for (final mark in inline['marks'] as List? ?? []) {
        if (['bold', 'italic', 'underline', 'strike'].contains(mark['type'])) {
          direct[mark['type']] = mark['attrs']?['value'] != false;
        }
        if (mark['type'] == 'textStyle' && mark['attrs']?['fontSize'] != null) {
          direct['size'] = mark['attrs']['fontSize'];
        }
        if (mark['type'] == 'link') direct['link'] = mark['attrs']?['href'];
      }
      style = directRunStyle(inheritedRun, direct, style);
      spans.add(
        TextSpan(
          text: inline['type'] == 'hardBreak'
              ? '\n'
              : inline['text']?.toString() ?? nodeText(inline),
          style: style,
        ),
      );
    }
    final alignment = switch (block['attrs']?['textAlign'] ??
        block['attrs']?['paragraphFormat']?['inheritedAlignment']) {
      'center' => TextAlign.center,
      'right' => TextAlign.right,
      'justify' || 'both' => TextAlign.justify,
      _ => TextAlign.left,
    };
    final p = paragraphProperties(block['attrs']?['paragraphFormat']);
    final old = paragraphProperties(previous?['attrs']?['paragraphFormat']);
    final native = nativeParagraphStyle(
      block['attrs']?['paragraphFormat']?['inheritedRun'] is! Map &&
              block['type'] == 'heading'
          ? (block['attrs']?['level'] as int? ?? 1)
          : null,
    );
    final oldNative = nativeParagraphStyle(
      previous?['attrs']?['paragraphFormat']?['inheritedRun'] is! Map &&
              previous?['type'] == 'heading'
          ? (previous?['attrs']?['level'] as int? ?? 1)
          : null,
    );
    double twips(Map values, String key, double fallback) => values[key] is num
        ? math.max(0, (values[key] as num).toDouble() / 15)
        : fallback;
    final left = twips(p, p.containsKey('start') ? 'start' : 'left', 0);
    return Padding(
      padding: EdgeInsets.fromLTRB(
        left,
        math.max(
          0,
          twips(p, 'before', native.verticalSpacing.top) -
              (previous == null
                  ? 0
                  : twips(old, 'after', oldNative.verticalSpacing.bottom)),
        ),
        twips(p, p.containsKey('end') ? 'end' : 'right', 0),
        twips(p, 'after', native.verticalSpacing.bottom),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final scaler = MediaQuery.textScalerOf(context);
          final direction = Directionality.of(context);
          final source = TextSpan(
            children: spans.isEmpty ? [const TextSpan(text: ' ')] : spans,
            style: paragraphStyle(
              p,
              inheritedRunStyle(
                inheritedRun,
                DefaultTextStyle.of(context).style.merge(native.style),
              ),
            ),
          );
          return q.QuillLayoutText(
            text: layoutTabStops(
              source,
              maxWidth: constraints.maxWidth,
              origin: left,
              interval: DocumentTabSettings.intervalOf(context) / 15,
              stops: effectiveTabStops(block['attrs']?['tabs']),
              textDirection: direction,
              textScaler: scaler,
              locale: Localizations.maybeLocaleOf(context),
            ),
            strutStyle: paragraphStrutStyle(
              p,
              StrutStyle.fromTextStyle(source.style!),
            ),
            textAlign: alignment,
            textDirection: direction,
            textScaler: scaler,
          );
        },
      ),
    );
  }
}

class _SpanTableLayout extends MultiChildRenderObjectWidget {
  final List<TablePosition> positions;
  final List<Map?> rowHeights;
  final List<num> widths;
  final TableGridGeometry? geometry;
  const _SpanTableLayout({
    required this.positions,
    required this.rowHeights,
    required this.widths,
    required super.children,
    this.geometry,
  });
  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderSpanTable(positions, rowHeights, widths)..geometry = geometry;
  @override
  void updateRenderObject(
    BuildContext context,
    covariant _RenderSpanTable renderObject,
  ) {
    renderObject.positions = positions;
    renderObject.rowHeights = rowHeights;
    renderObject.widths = widths;
    renderObject.geometry = geometry;
    renderObject.markNeedsLayout();
  }
}

class _TableParentData extends ContainerBoxParentData<RenderBox> {}

class _RenderSpanTable extends RenderBox
    with
        ContainerRenderObjectMixin<RenderBox, _TableParentData>,
        RenderBoxContainerDefaultsMixin<RenderBox, _TableParentData> {
  List<TablePosition> positions;
  List<Map?> rowHeights;
  List<num> widths;
  TableGridGeometry? geometry;
  int get rowCount => rowHeights.length;
  _RenderSpanTable(this.positions, this.rowHeights, this.widths);
  @override
  void setupParentData(RenderBox child) {
    if (child.parentData is! _TableParentData) {
      child.parentData = _TableParentData();
    }
  }

  @override
  void performLayout() {
    final width = constraints.hasBoundedWidth ? constraints.maxWidth : 680.0;
    final total = widths.fold<double>(0, (a, b) => a + b);
    final xs = <double>[0];
    for (final w in widths) {
      xs.add(xs.last + width * w / total);
    }
    var child = firstChild;
    var i = 0;
    final sizes = <double>[];
    while (child != null) {
      final p = positions[i++];
      child.layout(
        BoxConstraints.tightFor(width: xs[p.col + p.cols] - xs[p.col]),
        parentUsesSize: true,
      );
      sizes.add(child.size.height);
      child = (child.parentData! as _TableParentData).nextSibling;
    }
    final heights = documentTableRowHeights(rowHeights, positions, sizes);
    final ys = <double>[0];
    for (final h in heights) {
      ys.add(ys.last + h);
    }
    child = firstChild;
    i = 0;
    while (child != null) {
      final p = positions[i++];
      child.layout(
        BoxConstraints.tight(
          Size(xs[p.col + p.cols] - xs[p.col], ys[p.row + p.rows] - ys[p.row]),
        ),
        parentUsesSize: true,
      );
      final data = child.parentData! as _TableParentData;
      data.offset = Offset(xs[p.col], ys[p.row]);
      child = data.nextSibling;
    }
    size = constraints.constrain(Size(width, ys.last));
    geometry
      ?..xs = xs
      ..ys = ys
      ..cells = [
        for (final p in positions)
          (row: p.row, col: p.col, rows: p.rows, cols: p.cols),
      ];
  }

  @override
  void paint(PaintingContext context, Offset offset) =>
      defaultPaint(context, offset);
  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) =>
      defaultHitTestChildren(result, position: position);
}

class DocumentTableDialog extends StatefulWidget {
  final WorkspaceModel? model;
  final Map node;
  final int row, col;
  final TextSelection? initialSelection;
  final void Function(LogicalTextEdits?)? onTextEdits;
  final List<q.EmbedBuilder> embedBuilders;
  const DocumentTableDialog({
    super.key,
    required this.node,
    this.model,
    this.row = 0,
    this.col = 0,
    this.initialSelection,
    this.onTextEdits,
    required this.embedBuilders,
  });
  @override
  State<DocumentTableDialog> createState() => _DocumentTableDialogState();
}

class _DocumentTableDialogState extends State<DocumentTableDialog> {
  late Map<String, dynamic> node;
  late q.QuillController editor;
  final focus = FocusNode(), scroll = ScrollController();
  final retired = <q.QuillController>[];
  late DocumentEditCapture edits;
  late final DocumentEditDraft draft;
  late int row, col, anchorRow, anchorCol;
  bool selectCells = false;
  int epoch = 0;
  Map<int, String> listLabels = {};
  String? error;
  DocumentTable get table => DocumentTable(node);
  TableArea get selection => table.area(anchorRow, anchorCol, row, col);
  @override
  void initState() {
    super.initState();
    node = copyTableNode(widget.node);
    draft = DocumentEditDraft({
      'type': 'doc',
      'content': [node],
    });
    row = anchorRow = widget.row;
    col = anchorCol = widget.col;
    loadCell();
    final selection = widget.initialSelection;
    if (selection != null) {
      final limit = editor.document.length - 1;
      editor.updateSelection(
        TextSelection(
          baseOffset: selection.start.clamp(0, limit),
          extentOffset: selection.end.clamp(0, limit),
        ),
        q.ChangeSource.local,
      );
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) focus.requestFocus();
      });
    }
  }

  void loadCell() {
    final cell = table.at(row, col);
    row = cell.row;
    col = cell.col;
    editor = q.QuillController(
      document: q.Document.fromDelta(
        documentToDelta({'type': 'doc', 'content': cell.cell['content']}),
      ),
      selection: const TextSelection.collapsed(offset: 0),
    );
    configureDocumentLists(
      editor.document,
      styleCatalog: () =>
          widget.model?.workspace['document']?['attrs']?['wordStyles'] as Map?,
    );
    edits = DocumentEditCapture.attach(editor.document);
    listLabels = documentListLabels(editor.document.toDelta());
    editor.addListener(saveCell);
  }

  void saveCell() {
    if (!edits.hasChanges) return;
    final selected = table.at(row, col);
    final before = {'type': 'doc', 'content': selected.cell['content']};
    final after = deltaToDocument(editor.document.toDelta());
    final cellEdits = edits.take(before, after);
    if (sameDocumentContent(before, after)) return;
    final oldTable = {
      'type': 'doc',
      'content': [copyTableNode(node)],
    };
    final offset = DocumentBookmarkIndex(
      oldTable,
    ).rangeAtPath([0, selected.row, selected.index])?.start;
    node['content'][selected.row]['content'][selected.index]['content'] =
        after['content'];
    draft.record(
      oldTable,
      {
        'type': 'doc',
        'content': [node],
      },
      cellEdits == null || offset == null
          ? null
          : [
              for (final edit in cellEdits)
                {
                  ...edit,
                  'start': edit['start'] + offset,
                  'end': edit['end'] + offset,
                },
            ],
    );
    listLabels = documentListLabels(editor.document.toDelta());
    if (mounted) setState(() {});
  }

  Future<void> editStyles() async {
    if (widget.model == null) return;
    final selection = editor.selection;
    final before = editor.document.toDelta();
    try {
      final data = await widget.model!.core.call({'op': 'documentStyles'});
      if (!mounted) return;
      final document = deltaToDocument(before);
      final paths = selectedStylePaths(document, selection);
      final result = await showDialog<Map<String, dynamic>>(
        context: context,
        builder: (_) => DocumentStylesDialog(
          data: data,
          currentId: selectionStyleId(document, paths),
          paragraphs: paths.length,
          manage: false,
        ),
      );
      if (result == null || !mounted) return;
      if (before != editor.document.toDelta()) {
        throw const FormatException(
          'The cell changed. Reopen Styles and try again.',
        );
      }
      final preview = await widget.model!.core.call({
        'op': 'documentStylePreview',
        ...result,
        'document': document,
        'paths': paths,
        'workspaceId': data['workspaceId'],
        'baseRevision': data['revision'],
      });
      if (!mounted) return;
      if (before != editor.document.toDelta()) {
        throw const FormatException(
          'The cell changed. Reopen Styles and try again.',
        );
      }
      final after = documentToDelta(preview['document'] as Map);
      editor.compose(before.diff(after), selection, q.ChangeSource.local);
      editor.updateSelection(selection, q.ChangeSource.local);
    } catch (e) {
      if (mounted) {
        setState(() => error = e is FormatException ? e.message : '$e');
      }
    } finally {
      if (mounted) focus.requestFocus();
    }
  }

  Future<void> editParagraphFormat() async {
    try {
      final sourceEditor = editor;
      final selection = editor.selection;
      final before = editor.document.toDelta();
      final styles = documentEditorStyles();
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
          formats: [
            for (final line in lines)
              paragraphFormatForAttrs(line.attrs, defaults: styles),
          ],
          sample: before.slice(lines.first.start, lines.first.end + 1),
          styles: styles,
        ),
      );
      if (!mounted || patch == null || patch.isEmpty) return;
      if (editor != sourceEditor || editor.document.toDelta() != before) {
        throw const FormatException(
          'The cell changed. Reopen Paragraph and try again.',
        );
      }
      final change = changeParagraphFormat(
        before,
        selection,
        patch,
        defaults: styles,
      );
      if (change.isEmpty) return;
      editor.compose(change, selection, q.ChangeSource.local);
      focus.requestFocus();
    } catch (e) {
      if (mounted) {
        setState(() => error = e is FormatException ? e.message : '$e');
      }
    }
  }

  void retireEditor() {
    saveCell();
    editor.removeListener(saveCell);
    final old = editor;
    retired.add(old);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (retired.remove(old)) {
        old.dispose();
      }
    });
  }

  void select(int r, int c, {bool? extend}) {
    retireEditor();
    final extendSelection =
        extend ?? (selectCells || HardwareKeyboard.instance.isShiftPressed);
    setState(() {
      row = r;
      col = c;
      if (!extendSelection) {
        anchorRow = r;
        anchorCol = c;
      }
      epoch++;
      error = null;
      loadCell();
    });
  }

  void command(String action) {
    try {
      saveCell();
      final next = table.change(action, selection);
      retireEditor();
      draft.invalidate();
      setState(() {
        node = next;
        selectCells = false;
        final grid = table;
        row = row.clamp(0, grid.rows.length - 1);
        col = col.clamp(0, grid.columns - 1);
        final cell = grid.at(row, col);
        row = anchorRow = cell.row;
        col = anchorCol = cell.col;
        epoch++;
        error = null;
        loadCell();
      });
    } on FormatException catch (e) {
      setState(() => error = e.message);
    }
  }

  Future<void> formatSelection() async {
    saveCell();
    final patch = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => DocumentTableFormatDialog(table: table, area: selection),
    );
    if (!mounted || patch == null || patch.isEmpty) return;
    try {
      final next = table.format(
        selection,
        columnWidth: patch['columnWidth'],
        equalColumns: patch['equalColumns'] == true,
        height: patch['height'],
        fill: patch['fill'],
        verticalAlign: patch['verticalAlign'],
        alignment: patch['alignment'],
        header: patch['header'],
        cantSplit: patch['cantSplit'],
      );
      setState(() {
        node = next;
        error = null;
      });
    } on FormatException catch (e) {
      setState(() => error = e.message);
    }
  }

  Future<void> convertRows() async {
    saveCell();
    final area = selection;
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => DocumentTableConversionDialog(
        table: node,
        firstRow: area.top,
        lastRow: area.bottom,
        applyTableEdits: true,
      ),
    );
    if (!mounted || result == null) return;
    Navigator.pop(context, result);
  }

  Future<void> sortRows() async {
    saveCell();
    final result = await showDialog<TableSortResult>(
      context: context,
      builder: (_) =>
          DocumentTableSortDialog(table: table, selection: selection),
    );
    if (!mounted || result == null || sameDocumentContent(node, result.node)) {
      return;
    }
    retireEditor();
    draft.invalidate();
    setState(() {
      node = result.node;
      row = anchorRow = result.order.indexOf(row);
      col = anchorCol = table.at(row, col).col;
      selectCells = false;
      epoch++;
      error = null;
      loadCell();
    });
  }

  @override
  void dispose() {
    editor.removeListener(saveCell);
    editor.dispose();
    for (final c in retired) {
      c.dispose();
    }
    focus.dispose();
    scroll.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Dialog(
    child: SizedBox(
      width: 900,
      height: 640,
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const Text(
              'Edit table',
              style: TextStyle(fontSize: 22, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 4,
              runSpacing: 4,
              children: [
                PopupMenuButton<String>(
                  key: const Key('table-structure'),
                  tooltip: 'Rows and columns',
                  onSelected: command,
                  itemBuilder: (_) => [
                    for (final e in {
                      'rowBefore': 'Insert row above',
                      'rowAfter': 'Insert row below',
                      'columnBefore': 'Insert column left',
                      'columnAfter': 'Insert column right',
                      'deleteRows': 'Delete selected rows',
                      'deleteColumns': 'Delete selected columns',
                    }.entries)
                      PopupMenuItem(value: e.key, child: Text(e.value)),
                  ],
                  child: const Padding(
                    padding: EdgeInsets.all(10),
                    child: Text('Rows and columns ▾'),
                  ),
                ),
                FilterChip(
                  key: const Key('table-select-cells'),
                  label: const Text('Select cells'),
                  selected: selectCells,
                  onSelected: (v) => setState(() => selectCells = v),
                ),
                TextButton(
                  key: const Key('table-merge'),
                  onPressed: () => command('merge'),
                  child: const Text('Merge cells'),
                ),
                TextButton(
                  key: const Key('table-split'),
                  onPressed: () => command('split'),
                  child: const Text('Split cell'),
                ),
                TextButton(
                  key: const Key('table-format'),
                  onPressed: formatSelection,
                  child: const Text('Table properties'),
                ),
                TextButton(
                  key: const Key('table-convert-rows'),
                  onPressed: convertRows,
                  child: const Text('Convert rows to text'),
                ),
                TextButton.icon(
                  key: const Key('table-sort'),
                  onPressed: sortRows,
                  icon: const Icon(Icons.sort_by_alpha, size: 16),
                  label: const Text('Sort…'),
                ),
                Padding(
                  padding: const EdgeInsets.all(10),
                  child: Text(
                    '${table.rows.length} rows × ${table.columns} columns',
                  ),
                ),
              ],
            ),
            const Text(
              'Click a cell to edit it. Shift-click, or use Select cells, to extend the selection.',
              style: TextStyle(fontSize: 12),
            ),
            if (error != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(error!, style: const TextStyle(color: Colors.red)),
              ),
            const SizedBox(height: 8),
            Expanded(
              flex: 4,
              child: SingleChildScrollView(
                child: DocumentTableView(
                  node: node,
                  selection: selection,
                  onCell: select,
                  // Accessible activation has no pointer event to refresh
                  // native modifiers. Range selection is an explicit action.
                  onAccessibleCell: (r, c) => select(r, c, extend: selectCells),
                  onExtendCell: (r, c) => select(r, c, extend: true),
                ),
              ),
            ),
            const Divider(height: 24),
            Row(
              children: [
                Expanded(
                  child: Text(
                    'Row ${row + 1}, column ${col + 1}',
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                ),
                TextButton(
                  key: const Key('table-edit-cell'),
                  onPressed: () {
                    setState(() {
                      selectCells = false;
                      anchorRow = row;
                      anchorCol = col;
                    });
                    focus.requestFocus();
                  },
                  child: const Text('Edit cell'),
                ),
              ],
            ),
            Row(
              children: [
                Expanded(
                  child: DocumentTextToolbar(controller: editor, focus: focus),
                ),
                TextButton(
                  key: const Key('table-styles'),
                  onPressed: widget.model == null ? null : editStyles,
                  child: const Text('Styles…'),
                ),
                TextButton.icon(
                  key: const Key('table-paragraph'),
                  onPressed: editParagraphFormat,
                  icon: const Icon(Icons.format_line_spacing, size: 16),
                  label: const Text('Paragraph…'),
                ),
              ],
            ),
            Expanded(
              flex: 3,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  border: Border.all(color: const Color(0xffb9c1cc)),
                ),
                child: q.QuillEditor.basic(
                  key: ValueKey('table-rich-editor-$epoch'),
                  controller: editor,
                  focusNode: focus,
                  scrollController: scroll,
                  config: q.QuillEditorConfig(
                    // Pinned Quill 11.6.0; retain imported numbering in the cell editor.
                    // ignore: experimental_member_use
                    customLeadingBlockBuilder: (node, config) {
                      final label = listLabels[node.documentOffset];
                      if (label == null) return null;
                      return SizedBox(
                        width: config.width,
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
                    padding: const EdgeInsets.all(12),
                    customStyles: documentEditorStyles(),
                    customShortcuts: documentMarkShortcuts,
                    customActions: documentMarkActions(editor, focus),
                    embedBuilders: widget.embedBuilders,
                  ),
                ),
              ),
            ),
            const SizedBox(height: 14),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: () => Navigator.pop(context),
                  child: const Text('Cancel'),
                ),
                const SizedBox(width: 8),
                FilledButton(
                  key: const Key('table-apply'),
                  onPressed: () {
                    saveCell();
                    widget.onTextEdits?.call(
                      draft.finish({
                        'type': 'doc',
                        'content': [node],
                      }),
                    );
                    Navigator.pop(context, node);
                  },
                  child: const Text('Apply changes'),
                ),
              ],
            ),
          ],
        ),
      ),
    ),
  );
}
