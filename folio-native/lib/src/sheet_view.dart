import 'dart:async';
import 'dart:collection';
import 'sheet_entry_key.dart';
import 'sheet_text_overflow.dart';
import 'sheet_data_bar.dart';
import 'sheet_icon_set.dart';
import 'draft_text_field.dart';
import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart' show defaultTargetPlatform, mapEquals;
import 'package:flutter/gestures.dart'
    show DragStartBehavior, kPrimaryMouseButton;
import 'package:flutter/services.dart';
import 'package:flutter/semantics.dart';
import 'sparse_sheet_grid.dart';
import 'sheet_number_format.dart';
export 'sheet_number_format.dart' show displayCell, formatCellValue;
import 'sheet_number_format_dialog.dart';
import 'workbook_block.dart' show parseCell;
import 'table_controls.dart';
import 'sheet_font.dart';
import 'sheet_font_dialog.dart';
import 'sheet_alignment_dialog.dart';
import 'sheet_autofit.dart';
import 'sheet_column_metrics.dart';
import 'sheet_editing_keys.dart';
import 'sheet_navigation.dart';
import 'sheet_go_to.dart';
import 'sheet_clipboard.dart';
import 'sheet_reviews.dart';
import 'sheet_validation_dialog.dart';
import 'sheet_conditional_dialog.dart';

String columnLabel(int col) {
  var n = col + 1;
  var text = '';
  while (n > 0) {
    n--;
    text = String.fromCharCode(65 + n % 26) + text;
    n = n ~/ 26;
  }
  return text;
}

String cellAddress(int row, int col) => '${columnLabel(col)}${row + 1}';

class SheetView extends StatefulWidget {
  final Map sheet;
  final Map calculated;
  final Map formattedCells;
  final List conditionalWarnings;
  final Future<Map<String, dynamic>> Function(Map<String, dynamic>)?
  onConditionalRange;
  final Future<void> Function(Map<String, dynamic>)? onConditionalFormat;
  final Future<Map<String, dynamic>> Function(Map<String, dynamic>)?
  onConditionalCheck;
  final Future<Map<String, dynamic>> Function(Map<String, dynamic>)?
  onNumberPreview;
  final int? revision;
  final int calculationGeneration;
  final String? workspaceId;
  final Future<void> Function(List<Map<String, dynamic>>) onChanges;
  final Future<Map<String, dynamic>> Function(
    Map<String, dynamic>,
  )? onEntryCheck,
      onValidationChoices;
  final Future<void> Function(Map<String, dynamic>)? onEntry, onValidation;
  final void Function(String) onSelection;
  final Future<void> Function(String address, String? kind)? onReview;
  final Future<void> Function(String address)? onAudit;
  final Future<void> Function(String range)? onWatch;
  final Future<void> Function(
    List<String>,
    Map<String, dynamic>, {
    int? baseRevision,
  })?
  onFormat;
  final Future<void> Function(Map<String, dynamic>)? onFormatText;
  final Future<void> Function(Map<String, dynamic>)? onStructure;
  final Future<void> Function(Map<String, dynamic>)? onGeometry;
  final Future<void> Function(Map<String, dynamic>)? onView;
  final Future<void> Function(Map<String, dynamic>)? onFill;
  final Future<Map<String, dynamic>> Function(Map<String, dynamic>)? onCopy;
  final Future<void> Function(Map<String, dynamic>)? onPaste;
  final Future<void> Function(Map<String, dynamic>)? onTable;
  final Future<void> Function(Map<String, dynamic>)? onSortRange;
  final Future<void> Function(Map<String, dynamic>)? onAutoFilter;
  final Future<Map<String, dynamic>> Function(Map<String, dynamic>)?
  onFilterChoices;

  /// The workbook's defined names, for the name box and Go To.
  final Future<List<Map>> Function()? onDefinedNames;

  /// Go to a reference on another sheet; returns false if there is none.
  final Future<bool> Function(SheetTarget target)? onGoToSheet;
  const SheetView({
    super.key,
    required this.sheet,
    required this.calculated,
    this.formattedCells = const {},
    this.conditionalWarnings = const [],
    this.onConditionalRange,
    this.onConditionalFormat,
    this.onConditionalCheck,
    this.onNumberPreview,
    this.revision,
    this.calculationGeneration = 0,
    this.workspaceId,
    required this.onChanges,
    required this.onSelection,
    this.onReview,
    this.onAudit,
    this.onWatch,
    this.onEntryCheck,
    this.onEntry,
    this.onValidation,
    this.onValidationChoices,
    this.onFormat,
    this.onFormatText,
    this.onStructure,
    this.onGeometry,
    this.onView,
    this.onFill,
    this.onCopy,
    this.onPaste,
    this.onTable,
    this.onSortRange,
    this.onAutoFilter,
    this.onFilterChoices,
    this.onDefinedNames,
    this.onGoToSheet,
  });
  @override
  State<SheetView> createState() => SheetViewState();
}

class SheetViewState extends State<SheetView> {
  int _conditionalEpoch = 0;
  final _conditionalQueried = <String>{};
  final _conditionalPending = <String>{};
  final _conditionalDisplay = <String, Map>{};
  bool _conditionalScheduled = false;
  String? _conditionalViewError;

  Map? displayAt(String address) {
    final existing = widget.formattedCells[address] as Map?;
    if (existing != null) return existing;
    if (widget.onConditionalRange != null &&
        (widget.sheet['conditionalFormats'] as List? ?? []).isNotEmpty &&
        !_conditionalQueried.contains(address)) {
      _conditionalPending.add(address);
      if (!_conditionalScheduled) {
        _conditionalScheduled = true;
        WidgetsBinding.instance.addPostFrameCallback(
          (_) => _loadConditionalView(),
        );
      }
    }
    return _conditionalDisplay[address];
  }

  Future<void> _loadConditionalView() async {
    _conditionalScheduled = false;
    if (!mounted || _conditionalPending.isEmpty) return;
    final epoch = _conditionalEpoch;
    final cells = _conditionalPending.toList();
    _conditionalPending.clear();
    _conditionalQueried.addAll(cells);
    try {
      final result = await widget.onConditionalRange!({
        'sheetId': widget.sheet['id'],
        'baseRevision': widget.revision,
        'cells': cells,
      });
      if (!mounted || epoch != _conditionalEpoch) return;
      setState(() {
        if (_conditionalQueried.length > 4096) {
          _conditionalQueried.clear();
          _conditionalDisplay.clear();
          _conditionalQueried.addAll(cells);
        }
        for (final entry in (result['formatted'] as Map? ?? {}).entries) {
          _conditionalDisplay[entry.key as String] = entry.value as Map;
        }
        _conditionalViewError = (result['warnings'] as List?)?.firstOrNull
            ?.toString();
      });
    } catch (e) {
      if (mounted && epoch == _conditionalEpoch) {
        setState(() => _conditionalViewError = e.toString());
      }
    }
  }

  Future<void> editConditionalFormats() async {
    try {
      await flush();
      if (!mounted) return;
      final id = widget.sheet['id'], revision = widget.revision;
      final result = await showDialog<Map<String, dynamic>>(
        context: context,
        builder: (_) => SheetConditionalDialog(
          range: selectedRange,
          rules: widget.sheet['conditionalFormats'] as List? ?? [],
          extended: widget.sheet['extendedConditionalFormats'] == true,
          warnings: [
            ...widget.conditionalWarnings,
            if (_conditionalViewError != null) _conditionalViewError,
          ],
          onValidate: (request) async {
            await widget.onConditionalCheck?.call({
              'sheetId': id,
              'baseRevision': revision,
              ...request,
            });
          },
        ),
      );
      if (result == null) return;
      await widget.onConditionalFormat?.call({
        'sheetId': id,
        'baseRevision': revision,
        ...result,
      });
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    }
  }

  static const _keyboardChannel = MethodChannel('folio/keyboard');
  Future<void> _syncDesktopGridFocus([bool? focused]) async {
    if (defaultTargetPlatform != TargetPlatform.macOS) return;
    try {
      await _keyboardChannel.invokeMethod<void>(
        'setGridFocused',
        focused ?? gridFocus.hasPrimaryFocus,
      );
    } on MissingPluginException {
      // Widget tests and hosts without the macOS window bridge.
    }
  }

  void _gridFocusChanged() => _syncDesktopGridFocus();
  int get frozenRows =>
      (widget.sheet['frozenPane']?['rows'] as num?)?.toInt() ?? 0;
  int get frozenColumns =>
      (widget.sheet['frozenPane']?['columns'] as num?)?.toInt() ?? 0;
  int displayedFrozenRows = 0, displayedFrozenColumns = 0;
  int get pinnedRows => 1 + visibleRows.toVisible(displayedFrozenRows);
  int get pinnedColumns => 1 + visibleColumns.toVisible(displayedFrozenColumns);

  void restorePaneOrigin() {
    if (!mounted) return;
    final origin =
        widget.sheet['frozenPane']?['topLeftCell'] as String? ?? 'A1';
    final (r, c) = parseCell(origin);
    if (vertical.hasClients) {
      vertical.jumpTo(
        (rowLayout.offset(r) - rowLayout.offset(displayedFrozenRows)).clamp(
          0,
          vertical.position.maxScrollExtent,
        ),
      );
    }
    if (horizontal.hasClients) {
      horizontal.jumpTo(
        (columnLayout.offset(c) - columnLayout.offset(displayedFrozenColumns))
            .clamp(0, horizontal.position.maxScrollExtent),
      );
    }
  }

  Future<void> freezePanes(String action) async {
    try {
      await flush();
      await widget.onView?.call(
        action == 'unfreeze'
            ? {'action': 'unfreeze'}
            : {
                'action': 'freeze',
                'rows': action == 'topRow'
                    ? 1
                    : action == 'firstColumn'
                    ? 0
                    : row,
                'columns': action == 'firstColumn'
                    ? 1
                    : action == 'topRow'
                    ? 0
                    : col,
              },
      );
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    }
  }

  Widget frozenGrid(Widget Function() buildGrid) => LayoutBuilder(
    builder: (context, constraints) {
      // Retain the exact saved boundary even if this window cannot contain it.
      // Temporarily unpin an oversized axis rather than build millions of
      // pinned cells or conceal the entire editable viewport.
      final rowsFit =
          rowLayout.offset(frozenRows) + 32 + 64 < constraints.maxHeight;
      final columnsFit =
          columnLayout.offset(frozenColumns) + 43 + 64 < constraints.maxWidth;
      final overflow = !rowsFit || !columnsFit;
      displayedFrozenRows = rowsFit ? frozenRows : 0;
      displayedFrozenColumns = columnsFit ? frozenColumns : 0;
      return Column(
        children: [
          if (overflow)
            Container(
              key: const Key('frozen-pane-size-notice'),
              width: double.infinity,
              color: const Color(0xfffff3cd),
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
              child: const Text(
                'Frozen headings exceed this window. Oversized panes scroll temporarily; enlarge the window or unfreeze panes.',
                style: TextStyle(fontSize: 11),
              ),
            ),
          Expanded(child: buildGrid()),
        ],
      );
    },
  );
  int row = 3, col = 1;
  late VisibleRows visibleRows;
  late VisibleColumns visibleColumns;
  late SheetAxisLayout rowLayout, columnLayout;
  double? dragExtent;
  int? dragIndex;
  bool dragRows = false;
  late SheetColumnMetrics columnMetrics;
  Map<String, dynamic>? _columnFont;
  void updateGeometry() {
    final font = normalSheetFont(widget.sheet);
    if (!mapEquals(font, _columnFont)) {
      _columnFont = font;
      columnMetrics = SheetColumnMetrics.fromFont(font);
    }
    visibleRows = VisibleRows(widget.sheet);
    visibleColumns = VisibleColumns(widget.sheet);
    rowLayout = SheetAxisLayout(
      visibleRows,
      widget.sheet['defaultRowHeight'] is num
          ? (widget.sheet['defaultRowHeight'] as num).toDouble() * 4 / 3
          : 32,
      {
        for (final e in (widget.sheet['rowHeights'] as Map? ?? {}).entries)
          int.parse(e.key) - 1: (e.value as num).toDouble() * 4 / 3,
      },
    );
    columnLayout = SheetAxisLayout(
      visibleColumns,
      widget.sheet['defaultColumnWidth'] is num
          ? columnMetrics.pixels(widget.sheet['defaultColumnWidth'] as num)
          : 122,
      {
        for (final e in (widget.sheet['columnWidths'] as Map? ?? {}).entries)
          int.parse(e.key) - 1: columnMetrics.pixels(e.value as num),
      },
    );
  }

  double rowExtent(int r) =>
      dragRows && dragIndex == r ? dragExtent! : rowLayout.extent(r);
  double columnExtent(int c) =>
      !dragRows && dragIndex == c ? dragExtent! : columnLayout.extent(c);
  List<Map> get tables => (widget.sheet['tables'] as List? ?? []).cast<Map>();
  Map? tableAt(int r, int c) {
    for (final table in tables) {
      if (TableArea.parse(table['range']).contains(r, c)) return table;
    }
    return null;
  }

  Map? get selectedTable => tableAt(row, col);
  int? rangeRow, rangeCol;

  /// Whole rows (Shift+Space, a row heading) or whole columns (Ctrl+Space,
  /// a column heading) spanned by the selection; both means the whole sheet.
  bool wholeRows = false, wholeColumns = false;

  /// A selected region that keeps the active cell inside it rather than at a
  /// corner (Ctrl+A over data). Cleared by the next selection.
  TableArea? regionSelection;
  bool get hasRangeSelection =>
      rangeRow != null || wholeRows || wholeColumns || regionSelection != null;

  /// The selected rectangle: the active cell and the anchor, widened to
  /// whole rows or columns when those are selected.
  TableArea get selectionArea {
    if (regionSelection != null) return regionSelection!;
    final anchorRow = rangeRow ?? row, anchorCol = rangeCol ?? col;
    return TableArea(
      wholeColumns ? 0 : (row < anchorRow ? row : anchorRow),
      wholeRows ? 0 : (col < anchorCol ? col : anchorCol),
      wholeColumns ? 1048575 : (row > anchorRow ? row : anchorRow),
      wholeRows ? 16383 : (col > anchorCol ? col : anchorCol),
    );
  }

  String get selectedRange {
    if (!hasRangeSelection) return selected;
    final area = selectionArea;
    return '${cellAddress(area.top, area.left)}:${cellAddress(area.bottom, area.right)}';
  }

  bool editing = false, chart = false, formulaExpanded = false;
  bool extendSelectionMode = false;
  bool formulaDirty = false;
  final cellController = TextEditingController(),
      formulaController = TextEditingController();
  final cellFocus = FocusNode(),
      gridFocus = FocusNode(),
      formulaFocus = FocusNode();
  final vertical = ScrollController(), horizontal = ScrollController();
  String get selected => cellAddress(row, col);
  String raw(String a) => widget.sheet['cells'][a]?.toString() ?? '';
  @override
  void initState() {
    super.initState();
    updateGeometry();
    if (visibleRows.isHidden(row) && visibleRows.hidden.length < 1048576) {
      row = visibleRows.toPhysical(
        visibleRows
            .toVisible(row)
            .clamp(0, 1048575 - visibleRows.hidden.length),
      );
    }
    if (visibleColumns.isHidden(col) && visibleColumns.hidden.length < 16384) {
      col = visibleColumns.toPhysical(
        visibleColumns
            .toVisible(col)
            .clamp(0, 16383 - visibleColumns.hidden.length),
      );
    }
    formulaController.text = raw(selected);
    cellFocus.addListener(_blurCell);
    formulaFocus.addListener(_blurFormula);
    gridFocus.addListener(_gridFocusChanged);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) widget.onSelection(selectedRange);
      restorePaneOrigin();
    });
  }

  int? _focusedEditSession;
  void _blurCell() {
    if (cellFocus.hasFocus) {
      _focusedEditSession = _editSession;
    } else if (editing && _focusedEditSession == _editSession) {
      // The previous cell's EditableText can detach after the queued next draft
      // has started. Its blur must not commit that new, unfinished draft.
      commit();
    }
  }

  void _blurFormula() {
    if (!formulaFocus.hasFocus) commitFormula();
  }

  @override
  void didUpdateWidget(covariant SheetView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.sheet['id'] != widget.sheet['id'] ||
        oldWidget.workspaceId != widget.workspaceId) {
      extendSelectionMode = false;
    }
    _overflowIndex = null;
    if (oldWidget.revision != widget.revision ||
        oldWidget.calculationGeneration != widget.calculationGeneration ||
        oldWidget.sheet['id'] != widget.sheet['id'] ||
        oldWidget.workspaceId != widget.workspaceId) {
      _conditionalEpoch++;
      _conditionalQueried.clear();
      _conditionalPending.clear();
      _conditionalDisplay.clear();
      _conditionalViewError = null;
    }
    final wasRows = visibleRows, wasColumns = visibleColumns;
    updateGeometry();
    if (oldWidget.sheet['id'] != widget.sheet['id'] ||
        !mapEquals(
          oldWidget.sheet['frozenPane'] as Map?,
          widget.sheet['frozenPane'] as Map?,
        )) {
      WidgetsBinding.instance.addPostFrameCallback((_) => restorePaneOrigin());
    }
    if (visibleRows.isHidden(row) &&
        !wasRows.isHidden(row) &&
        !editing &&
        visibleRows.hidden.length < 1048576) {
      row = visibleRows.toPhysical(
        visibleRows
            .toVisible(row)
            .clamp(0, 1048575 - visibleRows.hidden.length),
      );
      rangeRow = null;
      rangeCol = null;
      wholeRows = wholeColumns = false;
      regionSelection = null;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) widget.onSelection(selectedRange);
      });
    }
    if (visibleColumns.isHidden(col) &&
        !wasColumns.isHidden(col) &&
        !editing &&
        visibleColumns.hidden.length < 16384) {
      col = visibleColumns.toPhysical(
        visibleColumns
            .toVisible(col)
            .clamp(0, 16383 - visibleColumns.hidden.length),
      );
      rangeRow = null;
      rangeCol = null;
      wholeRows = wholeColumns = false;
      regionSelection = null;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) widget.onSelection(selectedRange);
      });
    }
    if (!formulaFocus.hasFocus) formulaController.text = raw(selected);
  }

  void fail(Object e) {
    if (mounted) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(e.toString())));
    }
  }

  bool validationHintDismissed = false;
  Map? validationAt(String cell) {
    final (r, c) = parseCell(cell);
    for (final rule
        in (widget.sheet['dataValidations'] as List? ?? const [])
            .whereType<Map>()) {
      for (final ref in '${rule['sqref'] ?? ''}'.split(RegExp(r'\s+'))) {
        try {
          final area = TableArea.parse(ref.replaceAll(r'$', ''));
          if (r >= area.top &&
              r <= area.bottom &&
              c >= area.left &&
              c <= area.right) {
            return rule;
          }
        } catch (_) {
          /* Unsupported imported ranges stay guarded in the core. */
        }
      }
    }
    return null;
  }

  bool get hasValidationDropdown {
    final rule = validationAt(selected);
    return widget.onValidationChoices != null &&
        widget.onEntry != null &&
        rule?['type'] == 'list' &&
        rule?['showDropDown'] != true;
  }

  Future<void> editValidation() async {
    try {
      await flush();
      final id = widget.sheet['id'], range = selectedRange;
      final area = TableArea.parse(range);
      final cell = cellAddress(area.top, area.left);
      final settings = await widget.onValidationChoices?.call({
        'sheetId': id,
        'address': cell,
        'settingsOnly': true,
      });
      if (!mounted || widget.sheet['id'] != id) return;
      final result = await showDialog<Map>(
        context: context,
        builder: (_) => SheetValidationDialog(
          range: range,
          rule: settings?['rule'] as Map?,
        ),
      );
      if (result == null) return;
      await widget.onValidation?.call({
        'sheetId': id,
        'range': range,
        'baseRevision': settings?['revision'] ?? widget.revision,
        ...Map<String, dynamic>.from(result),
      });
      if (mounted) {
        setState(() => validationHintDismissed = false);
        gridFocus.requestFocus();
      }
    } catch (e) {
      fail(e);
    }
  }

  Future<String> enterValue(String a, String value, {int? revision}) async {
    if (widget.onEntry == null || widget.onEntryCheck == null) {
      await widget.onChanges([
        {'sheetId': widget.sheet['id'], 'address': a, 'value': value},
      ]);
      _overflowIndex = null;
      return 'accept';
    }
    final request = {
      'sheetId': widget.sheet['id'],
      'address': a,
      'value': value,
      'baseRevision': revision ?? widget.revision,
    };
    final checked = await widget.onEntryCheck!(request);
    if (!mounted) return 'retry';
    var allowInvalid = false;
    if (checked['valid'] != true && checked['shouldAlert'] == true) {
      final rule = checked['rule'] as Map? ?? const {};
      final override = checked['canOverride'] == true;
      final info = rule['errorStyle'] == 'information';
      final title = '${rule['errorTitle'] ?? ''}';
      final message = '${checked['reason'] ?? rule['error'] ?? ''}';
      final answer = await showDialog<String>(
        context: context,
        barrierDismissible: false,
        builder: (context) => AlertDialog(
          key: const Key('validation-alert'),
          icon: Icon(
            override
                ? info
                      ? Icons.info_outline
                      : Icons.warning_amber
                : Icons.error_outline,
          ),
          title: Text(title.isEmpty ? 'Check this entry' : title),
          content: Text(
            message.isEmpty
                ? 'This value does not match the validation rule for $a.'
                : message,
          ),
          actions: [
            TextButton(
              key: const Key('validation-discard'),
              onPressed: () => Navigator.pop(context, 'discard'),
              child: const Text('Cancel entry'),
            ),
            if (!info || !override)
              TextButton(
                key: const Key('validation-retry'),
                onPressed: () => Navigator.pop(context, 'retry'),
                child: const Text('Edit value'),
              ),
            if (override)
              FilledButton(
                key: const Key('validation-override'),
                onPressed: () => Navigator.pop(context, 'accept'),
                child: const Text('Use anyway'),
              ),
          ],
        ),
      );
      if (answer != 'accept') return answer ?? 'retry';
      allowInvalid = true;
    }
    await widget.onEntry!({...request, 'allowInvalid': allowInvalid});
    _overflowIndex = null;
    return 'accept';
  }

  bool _validationChoicesBusy = false;
  Future<void> chooseValidationValue() async {
    if (_validationChoicesBusy || !hasValidationDropdown) return;
    _validationChoicesBusy = true;
    try {
      await flush();
      final id = widget.sheet['id'], a = selected;
      final response = await widget.onValidationChoices!({
        'sheetId': id,
        'address': a,
      });
      if (!mounted || widget.sheet['id'] != id || selected != a) return;
      final choice = await showDialog<Map>(
        context: context,
        builder: (_) => ValidationChoicesDialog(
          choices: (response['values'] as List).cast<Map>(),
        ),
      );
      if (choice == null || !mounted || widget.sheet['id'] != id) return;
      final v = choice['value'];
      final input = v is String ? "'$v" : choice['label'] as String;
      await enterValue(a, input, revision: response['revision'] as int?);
      if (mounted) {
        formulaController.text = raw(selected);
        gridFocus.requestFocus();
      }
    } catch (e) {
      fail(e);
    } finally {
      _validationChoicesBusy = false;
    }
  }

  Future<bool>? _cellCommit, _formulaCommit;
  Future<bool> commit({bool keepEditing = false}) =>
      _cellCommit ??= _commitCell(
        keepEditing: keepEditing,
      ).whenComplete(() => _cellCommit = null);

  Future<bool> _commitCell({bool keepEditing = false}) async {
    if (!editing) return true;
    final value = cellController.text, a = selected;
    final session = _editSession;
    var discarded = false;
    final sheetId = widget.sheet['id'], workspaceId = widget.workspaceId;
    bool sameCell() =>
        mounted &&
        selected == a &&
        widget.sheet['id'] == sheetId &&
        widget.workspaceId == workspaceId;
    if (!formulaDirty) formulaController.text = value;
    if (value != raw(a)) {
      try {
        final outcome = await enterValue(a, value);
        if (outcome == 'retry') {
          if (sameCell()) {
            setState(() => editing = true);
            cellFocus.requestFocus();
          }
          return false;
        }
        if (outcome == 'discard') {
          discarded = true;
          cellController.text = raw(a);
          formulaController.text = raw(a);
        }
      } catch (e) {
        if (sameCell() && cellController.text == value) {
          setState(() => editing = true);
        }
        fail(e);
        return false;
      }
    }
    if (!keepEditing &&
        sameCell() &&
        session == _editSession &&
        (discarded || cellController.text == value)) {
      setState(() => editing = false);
    }
    return true;
  }

  Future<bool> commitFormula() => _formulaCommit ??= _commitFormula()
      .whenComplete(() => _formulaCommit = null);

  Future<bool> _commitFormula() async {
    if (!formulaDirty) return true;
    formulaDirty = false;
    final a = selected, value = formulaController.text;
    if (value == raw(a)) return true;
    try {
      final outcome = await enterValue(a, value);
      if (outcome == 'retry') {
        formulaDirty = true;
        formulaFocus.requestFocus();
        return false;
      }
      if (outcome == 'discard') formulaController.text = raw(a);
    } catch (e) {
      if (mounted && selected == a && formulaController.text == value) {
        formulaDirty = true;
      }
      fail(e);
      return false;
    }
    return true;
  }

  Future<bool> select(
    int r,
    int c, {
    bool requestFocus = true,
    bool extend = true,
    bool? shiftPressed,
  }) async {
    // A pending save may finish after Shift is released. Keep the intent of
    // this selection event instead of reading modifiers after the await.
    final selectionShift =
        shiftPressed ??
        (extendSelectionMode || HardwareKeyboard.instance.isShiftPressed);
    final scope = (widget.workspaceId, widget.sheet['id']);
    if (_flushTask != null) {
      try {
        await _flushTask;
      } catch (_) {
        return false;
      }
    }
    if (!await _waitForEntrySequence()) return false;
    if (!mounted || scope != (widget.workspaceId, widget.sheet['id'])) {
      return false;
    }
    return _select(
      r,
      c,
      requestFocus: requestFocus,
      extend: extend,
      shiftPressed: selectionShift,
    );
  }

  Future<bool> _select(
    int r,
    int c, {
    bool requestFocus = true,
    bool extend = true,
    bool? shiftPressed,
  }) async {
    final scope = (widget.workspaceId, widget.sheet['id']);
    if (!await commit()) return false;
    if (!mounted || scope != (widget.workspaceId, widget.sheet['id'])) {
      return false;
    }
    if (!await commitFormula()) return false;
    if (!mounted || scope != (widget.workspaceId, widget.sheet['id'])) {
      return false;
    }
    setState(() {
      if (!extend) extendSelectionMode = false;
      if (extend &&
          (shiftPressed ?? HardwareKeyboard.instance.isShiftPressed)) {
        rangeRow ??= row;
        rangeCol ??= col;
      } else {
        rangeRow = null;
        rangeCol = null;
        wholeRows = wholeColumns = false;
      }
      regionSelection = null;
      validationHintDismissed = false;
      row = r.clamp(0, 1048575);
      col = c.clamp(0, 16383);
      formulaController.text = raw(selected);
    });
    widget.onSelection(selectedRange);
    if (requestFocus) gridFocus.requestFocus();
    void reveal(
      ScrollController scroll,
      SheetAxisLayout layout,
      int physical,
      int frozen,
      double heading,
    ) {
      if (!scroll.hasClients || physical < frozen) return;
      final pinnedExtent = layout.offset(frozen);
      final start = layout.offset(physical) - pinnedExtent;
      final extent = layout.extent(physical);
      final end = start + extent;
      final view = (scroll.position.viewportDimension - heading - pinnedExtent)
          .clamp(1.0, double.infinity);
      final target = start < scroll.offset || extent > view
          ? start
          : end > scroll.offset + view
          ? end - view
          : scroll.offset;
      scroll.jumpTo(target.clamp(0, scroll.position.maxScrollExtent));
    }

    reveal(vertical, rowLayout, row, displayedFrozenRows, 32);
    reveal(horizontal, columnLayout, col, displayedFrozenColumns, 43);
    return true;
  }

  int _editSession = 0;
  void edit([String? initial]) {
    final session = ++_editSession;
    setState(() {
      extendSelectionMode = false;
      editing = true;
      final text = initial ?? raw(selected);
      cellController.value = TextEditingValue(
        text: text,
        selection: initial == null
            ? TextSelection(baseOffset: 0, extentOffset: text.length)
            : TextSelection.collapsed(offset: text.length),
      );
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      // Enter, Escape, another selection, or disposal can precede this frame.
      // An old callback must not steal focus from a newer editor or a dialog.
      if (mounted &&
          editing &&
          session == _editSession &&
          gridFocus.hasPrimaryFocus) {
        cellFocus.requestFocus();
      }
    });
  }

  KeyEventResult _keyBeforeEditorAttachment(
    KeyEvent event, {
    SheetEntryKey? input,
  }) {
    final result = SheetEditingKeys.handle(
      event,
      controller: cellController,
      focus: cellFocus,
      onCommit: (r, c) => finishEditing(r, c),
      onCancel: cancelEditing,
      beforeAttachment: true,
      input: input,
    );
    if (result != KeyEventResult.ignored) return result;
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    final keyboard = input ?? SheetEntryKey(event);
    if (keyboard.isMetaPressed || keyboard.isControlPressed) {
      return KeyEventResult.ignored;
    }
    final value = cellController.value;
    if (!value.selection.isValid || !value.composing.isCollapsed) {
      return KeyEventResult.ignored;
    }
    var start = value.selection.start, end = value.selection.end;
    final key = event.logicalKey;
    var text = event.character;
    if (key == LogicalKeyboardKey.backspace ||
        key == LogicalKeyboardKey.delete) {
      if (start == end) {
        if (key == LogicalKeyboardKey.backspace && start > 0) {
          start -= value.text.substring(0, start).characters.last.length;
        } else if (key == LogicalKeyboardKey.delete &&
            end < value.text.length) {
          end += value.text.substring(end).characters.first.length;
        }
      }
      text = '';
    } else if (text == null ||
        text.isEmpty ||
        text.runes.any((r) => r < 32 || r == 127)) {
      return KeyEventResult.ignored;
    }
    final updated = value.copyWith(
      text: value.text.replaceRange(start, end, text),
      selection: TextSelection.collapsed(offset: start + text.length),
      composing: TextRange.empty,
    );
    cellController.value = LengthLimitingTextInputFormatter(
      10000,
    ).formatEditUpdate(value, updated);
    return KeyEventResult.handled;
  }

  bool _finishingEdit = false;
  Completer<void>? _entryFinished;
  final _entryKeys = Queue<SheetEntryKey>();
  Future<void>? _entryDrain;
  Future<bool>? _entryNavigation;
  ({String? workspace, Object? sheet})? _entryContext;
  ({int rows, int columns})? _retryDirection;
  bool _entryPaused = false;
  bool _entryDetached = false;

  bool get _sameEntryContext =>
      !_entryDetached &&
      (_entryContext == null ||
          _entryContext ==
              (workspace: widget.workspaceId, sheet: widget.sheet['id']));

  Future<bool> _waitForEntrySequence() async {
    while (_finishingEdit || _entryDrain != null) {
      if (_finishingEdit) await _entryFinished!.future;
      if (_entryDrain != null) await _entryDrain;
    }
    return _entryKeys.isEmpty;
  }

  Future<void> _drainEntryKeys() =>
      _entryDrain ??= _drainEntries().whenComplete(() {
        _entryDrain = null;
      });

  Future<void> _drainEntries() async {
    while (mounted &&
        !_entryPaused &&
        _sameEntryContext &&
        _entryKeys.isNotEmpty) {
      final input = _entryKeys.removeFirst();
      _entryNavigation = null;
      if (editing && input.caretArrow) {
        // Wrapped lines and grapheme boundaries belong to EditableText. Wait
        // for its layout instead of approximating caret motion in the buffer.
        await WidgetsBinding.instance.endOfFrame;
        if (!mounted || !_sameEntryContext) {
          _entryKeys.addFirst(input);
          _entryPaused = true;
          _entryDetached = true;
          break;
        }
        final focusContext = cellFocus.context;
        if (focusContext != null && focusContext.mounted) {
          final k = input.event.logicalKey;
          if (k == LogicalKeyboardKey.arrowLeft ||
              k == LogicalKeyboardKey.arrowRight) {
            Actions.maybeInvoke(
              focusContext,
              ExtendSelectionByCharacterIntent(
                forward: k == LogicalKeyboardKey.arrowRight,
                collapseSelection: !input.isShiftPressed,
              ),
            );
          } else {
            Actions.maybeInvoke(
              focusContext,
              ExtendSelectionVerticallyToAdjacentLineIntent(
                forward: k == LogicalKeyboardKey.arrowDown,
                collapseSelection: !input.isShiftPressed,
              ),
            );
          }
          continue;
        }
      }
      key(input.event, replay: input);
      if (_finishingEdit) await _entryFinished!.future;
      if (_entryNavigation != null) await _entryNavigation;
    }
    if (!mounted) return;
    if (_entryKeys.isEmpty) _entryContext = null;
    setState(() {});
  }

  void _discardQueuedTyping() {
    setState(() {
      _entryKeys.clear();
      _entryContext = null;
      _retryDirection = null;
      _entryPaused = false;
      _entryDetached = false;
    });
  }

  Future<void> _copyWaitingKeys() => Clipboard.setData(
    ClipboardData(
      text: _entryKeys
          .map(
            (input) => input.text
                ? input.event.character!
                : '\n[${input.isShiftPressed ? 'Shift+' : ''}${input.isAltPressed ? 'Alt+' : ''}${input.event.logicalKey.keyLabel}]\n',
          )
          .join(),
    ),
  );

  Future<void> finishEditing(
    int rows,
    int columns, {
    bool formula = false,
  }) async {
    if (_finishingEdit) return;
    if (!_sameEntryContext) {
      setState(() => _entryPaused = true);
      return;
    }
    _finishingEdit = true;
    _entryFinished = Completer<void>();
    final address = selected,
        sheetId = widget.sheet['id'],
        workspaceId = widget.workspaceId;
    final direction = _retryDirection ?? (rows: rows, columns: columns);
    var advanced = false;
    try {
      final success = formula ? await commitFormula() : await commit();
      if (!mounted ||
          selected != address ||
          widget.sheet['id'] != sheetId ||
          widget.workspaceId != workspaceId ||
          !_sameEntryContext) {
        _entryPaused = true;
        _entryDetached = _entryKeys.isNotEmpty;
        return;
      }
      if (!success) {
        if (_entryKeys.isNotEmpty) {
          _entryPaused = true;
          _retryDirection = direction;
        }
        (formula ? formulaFocus : cellFocus).requestFocus();
        return;
      }
      // The accepted edit may have changed revision, filters and geometry.
      // Install the model's new widget snapshot before validating the next cell.
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted ||
          selected != address ||
          widget.sheet['id'] != sheetId ||
          widget.workspaceId != workspaceId ||
          !_sameEntryContext) {
        _entryPaused = true;
        _entryDetached = _entryKeys.isNotEmpty;
        return;
      }
      advanced = await _select(
        direction.rows == 0 ? row : visibleRows.next(row, direction.rows),
        direction.columns == 0
            ? col
            : visibleColumns.next(col, direction.columns),
        extend: false,
      );
      if (advanced) {
        _entryPaused = false;
        _retryDirection = null;
      }
    } finally {
      _finishingEdit = false;
      _entryFinished!.complete();
      if (mounted) {
        setState(() {});
        if (advanced && _entryKeys.isNotEmpty) unawaited(_drainEntryKeys());
      }
    }
  }

  void cancelEditing({bool formula = false}) {
    if (_finishingEdit) return;
    if (formula) {
      formulaDirty = false;
      formulaController.text = raw(selected);
    } else {
      setState(() => editing = false);
    }
    gridFocus.requestFocus();
    if (_entryPaused && _sameEntryContext && _retryDirection != null) {
      final direction = _retryDirection!;
      unawaited(
        finishEditing(direction.rows, direction.columns, formula: formula),
      );
    }
  }

  bool _clipboardBusy = false;
  Future<void> copy() async {
    if (_clipboardBusy || widget.onCopy == null) return;
    _clipboardBusy = true;
    final id = widget.sheet['id'], range = selectedRange;
    try {
      await flush();
      if (!mounted || widget.sheet['id'] != id) return;
      final copied = await widget.onCopy!({
        'sheetId': id,
        'range': range,
        'sourceDefaultColumnWidth':
            widget.sheet['defaultColumnWidth'] ??
            columnMetrics.fitPixels(columnLayout.defaultExtent),
      });
      await SheetClipboard.write(copied);
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    } finally {
      _clipboardBusy = false;
    }
  }

  Future<void> paste({
    String mode = 'all',
    bool transpose = false,
    bool skipBlanks = false,
    bool textOnly = false,
  }) async {
    if (_clipboardBusy || widget.onPaste == null) return;
    _clipboardBusy = true;
    final id = widget.sheet['id'], range = selectedRange;
    final workspaceId = widget.workspaceId;
    try {
      await flush();
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted || widget.workspaceId != workspaceId) return;
      final revision = widget.revision;
      final data = await SheetClipboard.read();
      if (data == null ||
          !mounted ||
          widget.sheet['id'] != id ||
          widget.workspaceId != workspaceId) {
        return;
      }
      await widget.onPaste!({
        'sheetId': id,
        'workspaceId': ?workspaceId,
        'baseRevision': ?revision,
        'range': range,
        'mode': mode,
        'transpose': transpose,
        'skipBlanks': skipBlanks,
        if (!textOnly && data['token'] != null)
          'token': data['token']
        else
          'text': data['text'],
      });
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    } finally {
      _clipboardBusy = false;
    }
  }

  void setExtendSelection(bool enabled) {
    setState(() => extendSelectionMode = enabled);
    gridFocus.requestFocus();
  }

  bool _cellActionsOpen = false;
  Future<void> cellActions(Offset position) async {
    if (_cellActionsOpen) return;
    _cellActionsOpen = true;
    final target = (widget.workspaceId, widget.sheet['id'], row, col);
    final previousRange = (rangeRow, rangeCol);
    try {
      final action = await showMenu<String>(
        context: context,
        position: RelativeRect.fromSize(
          Rect.fromLTWH(position.dx, position.dy, 0, 0),
          MediaQuery.sizeOf(context),
        ),
        items: [
          const PopupMenuItem(value: 'edit', child: Text('Edit cell')),
          if (widget.onWatch != null)
            const PopupMenuItem(value: 'watch', child: Text('Add watch…')),
          if (widget.onAudit != null)
            const PopupMenuItem(
              value: 'audit',
              child: Text('Formula inspector…'),
            ),
          const PopupMenuDivider(),
          CheckedPopupMenuItem(
            value: 'extend',
            checked: extendSelectionMode,
            child: const Text('Extend selection'),
          ),
          const PopupMenuItem(
            value: 'single',
            child: Text('Select active cell'),
          ),
        ],
      );
      if (!mounted ||
          action == null ||
          target != (widget.workspaceId, widget.sheet['id'], row, col) ||
          previousRange != (rangeRow, rangeCol)) {
        return;
      }
      if (action == 'watch') {
        await widget.onWatch?.call(selectedRange);
      } else if (action == 'extend') {
        // Keep the existing anchor; toggling the mode does not select another
        // cell. Any draft still follows the normal editor commit rules.
        setExtendSelection(!extendSelectionMode);
      } else if (await select(row, col, extend: false)) {
        if (action == 'edit') edit();
        if (action == 'audit') await widget.onAudit?.call(selected);
      }
    } finally {
      _cellActionsOpen = false;
    }
  }

  Future<void> cellMenu(int r, int c, Offset position) async {
    try {
      if (!await select(r, c, extend: false) || !mounted) return;
      final address = cellAddress(r, c), id = widget.sheet['id'];
      final review = cellReview(widget.sheet, address);
      final result = await showMenu<String>(
        context: context,
        position: RelativeRect.fromSize(
          Rect.fromLTWH(position.dx, position.dy, 0, 0),
          MediaQuery.sizeOf(context),
        ),
        items: [
          if (widget.onCopy != null)
            const PopupMenuItem(value: 'copy', child: Text('Copy')),
          if (widget.onPaste != null) ...[
            const PopupMenuItem(value: 'paste', child: Text('Paste')),
            const PopupMenuItem(
              value: 'special',
              child: Text('Paste special…'),
            ),
          ],
          if (widget.onCopy != null || widget.onPaste != null)
            const PopupMenuDivider(),
          if (review == null) ...[
            const PopupMenuItem(value: 'comment', child: Text('New comment')),
            const PopupMenuItem(value: 'note', child: Text('New note')),
          ] else
            PopupMenuItem(
              value: review['kind'],
              child: Text(
                review['kind'] == 'note' ? 'Edit note' : 'Show comment',
              ),
            ),
          const PopupMenuItem(
            value: 'reviews',
            child: Text('Comments & notes…'),
          ),
          if (widget.onAudit != null) ...[
            const PopupMenuDivider(),
            const PopupMenuItem(
              value: 'audit',
              child: Text('Formula inspector…'),
            ),
          ],
          if (widget.onWatch != null)
            const PopupMenuItem(value: 'watch', child: Text('Add watch…')),
        ],
      );
      if (!mounted || widget.sheet['id'] != id || result == null) return;
      if (result == 'copy') {
        await copy();
      } else if (result == 'paste') {
        await paste();
      } else if (result == 'special') {
        await pasteSpecial();
      } else if (result == 'audit') {
        await widget.onAudit?.call(address);
      } else if (result == 'watch') {
        await widget.onWatch?.call(address);
      } else {
        await widget.onReview?.call(
          address,
          result == 'reviews' ? null : result,
        );
      }
    } catch (e) {
      fail(e);
    }
  }

  Future<void> pasteSpecial() async {
    var mode = 'all', transpose = false, skipBlanks = false;
    final accepted = await showDialog<bool>(
      context: context,
      builder: (dialog) => StatefulBuilder(
        builder: (context, update) => AlertDialog(
          title: const Text('Paste special'),
          content: SizedBox(
            width: 340,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  DropdownButtonFormField<String>(
                    key: const Key('paste-special-mode'),
                    initialValue: mode,
                    isExpanded: true,
                    decoration: const InputDecoration(labelText: 'Paste'),
                    items: const [
                      DropdownMenuItem(
                        value: 'all',
                        child: Text('Cells and formatting'),
                      ),
                      DropdownMenuItem(
                        value: 'allMergeConditional',
                        child: Text('Cells and merged conditional formats'),
                      ),
                      DropdownMenuItem(
                        value: 'values',
                        child: Text('Values only'),
                      ),
                      DropdownMenuItem(
                        value: 'formulas',
                        child: Text('Formulas and values'),
                      ),
                      DropdownMenuItem(
                        value: 'formats',
                        child: Text('Formatting only'),
                      ),
                      DropdownMenuItem(
                        value: 'valuesNumberFormats',
                        child: Text('Values and number formats'),
                      ),
                      DropdownMenuItem(
                        value: 'formulasNumberFormats',
                        child: Text('Formulas and number formats'),
                      ),
                      DropdownMenuItem(
                        value: 'comments',
                        child: Text('Comments and notes only'),
                      ),
                      DropdownMenuItem(
                        value: 'validation',
                        child: Text('Validation rules only'),
                      ),
                      DropdownMenuItem(
                        value: 'columnWidths',
                        child: Text('Column widths only'),
                      ),
                      DropdownMenuItem(
                        value: 'allColumnWidths',
                        child: Text('Cells with source column widths'),
                      ),
                    ],
                    onChanged: (value) => update(() {
                      mode = value!;
                      if (mode == 'columnWidths' || mode == 'allColumnWidths') {
                        transpose = false;
                      }
                      if (mode == 'columnWidths') skipBlanks = false;
                    }),
                  ),
                  CheckboxListTile(
                    title: const Text('Skip blank cells'),
                    value: skipBlanks,
                    onChanged: mode == 'columnWidths'
                        ? null
                        : (value) => update(() => skipBlanks = value!),
                    contentPadding: EdgeInsets.zero,
                  ),
                  CheckboxListTile(
                    title: const Text('Transpose rows and columns'),
                    value: transpose,
                    onChanged:
                        mode == 'columnWidths' || mode == 'allColumnWidths'
                        ? null
                        : (value) => update(() => transpose = value!),
                    contentPadding: EdgeInsets.zero,
                  ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialog, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(dialog, true),
              child: const Text('Paste'),
            ),
          ],
        ),
      ),
    );
    if (accepted == true && mounted) {
      await paste(mode: mode, transpose: transpose, skipBlanks: skipBlanks);
    } else if (mounted) {
      gridFocus.requestFocus();
    }
  }

  bool _filling = false;
  Future<void> fillSelection(String direction) async {
    if (_filling || widget.onFill == null) return;
    _filling = true;
    final range = selectedRange;
    final id = widget.sheet['id'];
    try {
      if (!await commit() || !await commitFormula()) return;
      if (!mounted || widget.sheet['id'] != id) return;
      await widget.onFill!({
        'sheetId': id,
        'range': range,
        'direction': direction,
      });
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    } finally {
      _filling = false;
    }
  }

  KeyEventResult _capturePendingEntry(KeyEvent event) {
    if (!_finishingEdit && (_entryDrain == null || _entryPaused)) {
      return KeyEventResult.ignored;
    }
    final input = SheetEntryKey(event);
    if (input.completion &&
        (!input.down ||
            event is KeyRepeatEvent ||
            (_entryKeys.isEmpty && !input.isAltPressed))) {
      return KeyEventResult.handled;
    }
    if (!input.canQueue) return KeyEventResult.ignored;
    _entryContext ??= (
      workspace: widget.workspaceId,
      sheet: widget.sheet['id'],
    );
    setState(() => _entryKeys.add(input));
    return KeyEventResult.handled;
  }

  KeyEventResult key(KeyEvent e, {SheetEntryKey? replay}) {
    final keyboard = replay ?? SheetEntryKey(e);
    if (replay == null &&
        (_finishingEdit || (_entryDrain != null && !_entryPaused))) {
      return _capturePendingEntry(e);
    }
    // EditableText handles its own shortcuts and platform text input. A key
    // bubbling from the cell editor must never start another grid edit.
    if (editing) {
      // Before the first frame there is no EditableText client to receive these
      // keys. Consume them while the grid still owns focus; once the field is
      // focused, leave all ordinary text and IME input to the platform.
      return gridFocus.hasPrimaryFocus || replay != null
          ? _keyBeforeEditorAttachment(e, input: keyboard)
          : KeyEventResult.ignored;
    }
    if (e is! KeyDownEvent) return KeyEventResult.ignored;
    final k = e.logicalKey;
    if (k == LogicalKeyboardKey.f8 &&
        !keyboard.isShiftPressed &&
        !keyboard.isControlPressed &&
        !keyboard.isMetaPressed &&
        !keyboard.isAltPressed) {
      setExtendSelection(!extendSelectionMode);
      return KeyEventResult.handled;
    }
    if (k == LogicalKeyboardKey.escape && extendSelectionMode) {
      setExtendSelection(false);
      return KeyEventResult.handled;
    }
    if (k == LogicalKeyboardKey.f2 &&
        keyboard.isShiftPressed &&
        widget.onReview != null) {
      widget.onReview!(selected, 'note');
      return KeyEventResult.handled;
    }
    if (k == LogicalKeyboardKey.escape && !validationHintDismissed) {
      setState(() => validationHintDismissed = true);
      return KeyEventResult.handled;
    }
    if (k == LogicalKeyboardKey.arrowDown &&
        keyboard.isAltPressed &&
        hasValidationDropdown) {
      chooseValidationValue();
      return KeyEventResult.handled;
    }
    final fillModifier =
        (keyboard.isControlPressed && !keyboard.isMetaPressed) ||
        (defaultTargetPlatform == TargetPlatform.macOS &&
            keyboard.isMetaPressed &&
            !keyboard.isControlPressed);
    if (fillModifier &&
        !keyboard.isShiftPressed &&
        !keyboard.isAltPressed &&
        widget.onFormat != null) {
      final font = fontAt(selected);
      if (k == LogicalKeyboardKey.digit1) {
        editFont();
        return KeyEventResult.handled;
      }
      if (k == LogicalKeyboardKey.keyB ||
          k == LogicalKeyboardKey.keyI ||
          k == LogicalKeyboardKey.keyU) {
        format(
          k == LogicalKeyboardKey.keyU
              ? {
                  'underline':
                      font['underline'] == null || font['underline'] == 'none'
                      ? 'single'
                      : 'none',
                }
              : {
                  k == LogicalKeyboardKey.keyB ? 'bold' : 'italic':
                      font[k == LogicalKeyboardKey.keyB ? 'bold' : 'italic'] !=
                      true,
                },
        );
        return KeyEventResult.handled;
      }
    }
    if (fillModifier &&
        !keyboard.isShiftPressed &&
        !keyboard.isAltPressed &&
        widget.onFill != null &&
        (k == LogicalKeyboardKey.keyD || k == LogicalKeyboardKey.keyR)) {
      fillSelection(k == LogicalKeyboardKey.keyD ? 'down' : 'right');
      return KeyEventResult.handled;
    }
    final navigation = _navigationKey(k, keyboard, replay);
    if (navigation != null) return navigation;
    if (keyboard.isMetaPressed || keyboard.isControlPressed) {
      if (k == LogicalKeyboardKey.keyV) {
        if (keyboard.isShiftPressed) {
          pasteSpecial();
        } else {
          paste();
        }
        return KeyEventResult.handled;
      }
      if (k == LogicalKeyboardKey.keyC) {
        copy();
        return KeyEventResult.handled;
      }
      return KeyEventResult.ignored;
    }
    void navigate(int r, int c, {bool extend = true}) {
      _entryNavigation = replay != null
          ? _select(
              r,
              c,
              extend: extend,
              shiftPressed: extendSelectionMode || keyboard.isShiftPressed,
            )
          : select(r, c, extend: extend);
    }

    if (k == LogicalKeyboardKey.arrowDown) {
      navigate(visibleRows.next(row, 1), col);
    } else if (k == LogicalKeyboardKey.arrowUp) {
      navigate(visibleRows.next(row, -1), col);
    } else if (k == LogicalKeyboardKey.arrowRight) {
      navigate(row, visibleColumns.next(col, 1));
    } else if (k == LogicalKeyboardKey.arrowLeft) {
      navigate(row, visibleColumns.next(col, -1));
    } else if (k == LogicalKeyboardKey.enter ||
        k == LogicalKeyboardKey.numpadEnter ||
        k == LogicalKeyboardKey.f2) {
      edit();
    } else if (k == LogicalKeyboardKey.tab) {
      navigate(
        row,
        visibleColumns.next(col, keyboard.isShiftPressed ? -1 : 1),
        extend: false,
      );
    } else if (k == LogicalKeyboardKey.delete ||
        k == LogicalKeyboardKey.backspace) {
      if (replay != null) {
        edit('');
        finishEditing(0, 0);
        return KeyEventResult.handled;
      }
      final List<String> addresses;
      try {
        addresses = clearableAddresses();
      } catch (e) {
        fail(e);
        return KeyEventResult.handled;
      }
      widget
          .onChanges([
            for (final a in addresses)
              {'sheetId': widget.sheet['id'], 'address': a, 'value': ''},
          ])
          .catchError(fail);
    } else if (e.character != null && e.character!.isNotEmpty) {
      edit(e.character);
    } else {
      return KeyEventResult.ignored;
    }
    return KeyEventResult.handled;
  }

  /// Cells Delete clears: the whole selection when small, otherwise only
  /// its populated cells, so clearing whole rows or columns stays cheap.
  List<String> clearableAddresses() {
    final area = selectionArea;
    if ((area.bottom - area.top + 1) * (area.right - area.left + 1) <= 2000) {
      return selectedAddresses();
    }
    return [
      for (final entry in (widget.sheet['cells'] as Map).entries)
        if ((entry.value?.toString() ?? '').isNotEmpty &&
            (() {
              final (r, c) = parseCell(entry.key as String);
              return area.contains(r, c);
            })())
          entry.key as String,
    ];
  }

  int get _firstUnfrozenRow => frozenRows > 0
      ? visibleRows.next(frozenRows - 1, 1)
      : visibleRows.toPhysical(0);
  int get _firstUnfrozenColumn => frozenColumns > 0
      ? visibleColumns.next(frozenColumns - 1, 1)
      : visibleColumns.toPhysical(0);

  /// Selects whole rows (a row heading, Shift+Space) or columns (a column
  /// heading, Ctrl+Space), keeping any existing anchor when extending.
  Future<void> selectWhole({
    required bool rows,
    required bool columns,
    int? atRow,
    int? atColumn,
    bool extend = false,
  }) async {
    if (atRow != null || atColumn != null) {
      final ok = await select(
        atRow ?? row,
        atColumn ?? col,
        shiftPressed: extend,
      );
      if (!ok || !mounted) return;
    }
    setState(() {
      wholeRows = rows || (extend && wholeRows);
      wholeColumns = columns || (extend && wholeColumns);
      regionSelection = null;
    });
    widget.onSelection(selectedRange);
  }

  final _recentTargets = <String>[];

  /// Selects a target on this sheet: its range, keeping whole-row or
  /// whole-column selections as such.
  Future<void> selectTarget(SheetTarget target) async {
    final ok = await select(target.top, target.left, shiftPressed: false);
    if (!ok || !mounted) return;
    setState(() {
      final range = target.top != target.bottom || target.left != target.right;
      rangeRow = range ? target.bottom : null;
      rangeCol = range ? target.right : null;
      wholeRows = target.wholeRows;
      wholeColumns = target.wholeColumns;
      regionSelection = null;
    });
    widget.onSelection(selectedRange);
  }

  /// Name box and Go To: a cell, range, whole rows or columns, a reference
  /// on another sheet, or a defined name.
  Future<void> goTo(String text) async {
    try {
      final names = await widget.onDefinedNames?.call() ?? const [];
      final target = parseTarget(
        text,
        names: names,
        currentSheetId: widget.sheet['id'] as String?,
      );
      if (target == null) {
        throw FormatException('"$text" is not a cell, range or name.');
      }
      final elsewhere =
          target.sheetName != null &&
          target.sheetName!.toLowerCase() !=
              (widget.sheet['name'] as String).toLowerCase();
      if (!elsewhere) {
        await selectTarget(target);
      } else if (await widget.onGoToSheet?.call(target) != true) {
        throw FormatException('There is no sheet named ${target.sheetName}.');
      }
      _recentTargets
        ..remove(text.trim())
        ..insert(0, text.trim());
      if (_recentTargets.length > 4) _recentTargets.removeLast();
    } catch (e) {
      fail(e);
    }
  }

  /// Go To (F5, Ctrl+G): defined names, recent targets and a reference.
  Future<void> openGoTo() async {
    try {
      final names = await widget.onDefinedNames?.call() ?? const [];
      if (!mounted) return;
      final text = await showDialog<String>(
        context: context,
        builder: (_) => SheetGoToDialog(
          names: names,
          recent: List.of(_recentTargets),
          currentSheetId: widget.sheet['id'] as String?,
        ),
      );
      if (text != null) await goTo(text);
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    }
  }

  /// Excel's data-aware navigation and selection keys (X01.03):
  /// Ctrl/Command+arrow to data edges (Shift extends), Ctrl/Command+Home
  /// and +End, Home, Shift+Space, Ctrl+Space and Ctrl/Command+A.
  KeyEventResult? _navigationKey(
    LogicalKeyboardKey k,
    SheetEntryKey keyboard,
    SheetEntryKey? replay,
  ) {
    if (keyboard.isAltPressed) return null;
    final command = keyboard.isControlPressed || keyboard.isMetaPressed;
    final shift = keyboard.isShiftPressed || extendSelectionMode;
    if ((k == LogicalKeyboardKey.f5 && !command && !keyboard.isShiftPressed) ||
        (command && !keyboard.isShiftPressed && k == LogicalKeyboardKey.keyG)) {
      openGoTo();
      return KeyEventResult.handled;
    }
    void go(int r, int c) {
      _entryNavigation = replay != null
          ? _select(r, c, shiftPressed: shift)
          : select(r, c, shiftPressed: shift);
    }

    final cells = widget.sheet['cells'] as Map? ?? {};
    final vertical =
        k == LogicalKeyboardKey.arrowDown || k == LogicalKeyboardKey.arrowUp;
    final horizontal =
        k == LogicalKeyboardKey.arrowLeft || k == LogicalKeyboardKey.arrowRight;
    if (command && (vertical || horizontal)) {
      final forward =
          k == LogicalKeyboardKey.arrowDown ||
          k == LogicalKeyboardKey.arrowRight;
      final direction = forward ? 1 : -1;
      final axis = vertical ? visibleRows : visibleColumns;
      final limit = vertical ? 1048575 : 16383;
      final target = dataEdge(
        from: vertical ? row : col,
        direction: direction,
        populated: vertical
            ? populatedAlong(cells, column: col, hidden: axis.isHidden)
            : populatedAlong(cells, row: row, hidden: axis.isHidden),
        step: (p) => axis.next(p, direction),
        edge: forward
            ? axis.toPhysical(limit - axis.hidden.length)
            : axis.toPhysical(0),
      );
      go(vertical ? target : row, vertical ? col : target);
      return KeyEventResult.handled;
    }
    if (k == LogicalKeyboardKey.home) {
      go(command ? _firstUnfrozenRow : row, _firstUnfrozenColumn);
      return KeyEventResult.handled;
    }
    if (command && k == LogicalKeyboardKey.end) {
      final (r, c) = lastUsedCell(cells);
      go(r, c);
      return KeyEventResult.handled;
    }
    if (k == LogicalKeyboardKey.space &&
        (keyboard.isShiftPressed || keyboard.isControlPressed)) {
      if (keyboard.isMetaPressed) return null;
      selectWhole(
        rows: keyboard.isShiftPressed && !keyboard.isControlPressed
            ? true
            : wholeRows,
        columns: keyboard.isControlPressed ? true : wholeColumns,
        extend: true,
      );
      return KeyEventResult.handled;
    }
    if (command && !keyboard.isShiftPressed && k == LogicalKeyboardKey.keyA) {
      final region = currentRegion(cells, row, col);
      final area = region == null
          ? null
          : TableArea(region.$1, region.$2, region.$3, region.$4);
      final already =
          area != null &&
          !wholeRows &&
          !wholeColumns &&
          selectionArea.top == area.top &&
          selectionArea.left == area.left &&
          selectionArea.bottom == area.bottom &&
          selectionArea.right == area.right;
      if (area == null || already) {
        selectWhole(rows: true, columns: true);
      } else {
        setState(() {
          rangeRow = null;
          rangeCol = null;
          wholeRows = wholeColumns = false;
          regionSelection = area;
        });
        widget.onSelection(selectedRange);
      }
      return KeyEventResult.handled;
    }
    return null;
  }

  List<String> selectedAddresses() {
    final ends = selectedRange.split(':');
    final (r1, c1) = parseCell(ends.first);
    final (r2, c2) = parseCell(ends.last);
    if ((r2 - r1 + 1) * (c2 - c1 + 1) > 2000) {
      throw StateError('Select up to 2,000 cells.');
    }
    return [
      for (var r = r1; r <= r2; r++)
        for (var c = c1; c <= c2; c++) cellAddress(r, c),
    ];
  }

  Future<void> format(Map<String, dynamic> style) async {
    try {
      final addresses = selectedAddresses(), id = widget.sheet['id'];
      if (!await commit() || !await commitFormula()) return;
      if (!mounted || widget.sheet['id'] != id) return;
      await widget.onFormat?.call(addresses, style);
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    }
  }

  Future<void>? _fontDialogTask;
  Future<void> editFont() => _fontDialogTask ??= _editFont().whenComplete(
    () => _fontDialogTask = null,
  );
  Future<void> _editFont() async {
    if (widget.onFormat == null) return;
    final id = widget.sheet['id'], range = selectedRange;
    try {
      final addresses = selectedAddresses();
      if (!await commit() || !await commitFormula()) return;
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted || widget.sheet['id'] != id || selectedRange != range) {
        return;
      }
      final revision = widget.revision;
      final sample = formatted(addresses.first);
      final changes = await showDialog<Map<String, dynamic>>(
        context: context,
        builder: (_) => SheetFontDialog(
          fonts: [for (final a in addresses) fontAt(a)],
          sample: sample,
          previewFont: cellStyle(widget.sheet, addresses.first),
          allowCharacterSelection:
              addresses.length == 1 &&
              widget.onFormatText != null &&
              sample.isNotEmpty &&
              literalText(raw(addresses.first)) == sample,
        ),
      );
      if (!mounted || changes == null || changes.isEmpty) return;
      if (widget.sheet['id'] != id || widget.revision != revision) {
        throw StateError(
          'The workbook changed. Open Font again to review the current cells.',
        );
      }
      final textRange = changes.remove('_textRange') as Map?;
      if (textRange != null) {
        await widget.onFormatText!({
          'address': addresses.single,
          'expectedText': sample,
          'baseRevision': revision,
          'start': textRange['start'],
          'end': textRange['end'],
          'style': changes,
        });
      } else {
        await widget.onFormat!(addresses, changes, baseRevision: revision);
      }
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    }
  }

  String formatted(String address) => formatCellValue(
    widget.calculated[address] ?? raw(address),
    cellStyle(widget.sheet, address),
    display: displayAt(address),
  );

  Future<void>? _numberDialogTask;
  Future<void> editNumberFormat() => _numberDialogTask ??= _editNumberFormat()
      .whenComplete(() => _numberDialogTask = null);
  Future<void> _editNumberFormat() async {
    if (widget.onFormat == null || widget.onNumberPreview == null) return;
    final id = widget.sheet['id'],
        range = selectedRange,
        workspaceId = widget.workspaceId;
    try {
      final addresses = selectedAddresses();
      if (!await commit() || !await commitFormula()) return;
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted ||
          widget.sheet['id'] != id ||
          selectedRange != range ||
          widget.workspaceId != workspaceId) {
        return;
      }
      final revision = widget.revision;
      final codes = {
        for (final a in addresses)
          cellStyle(widget.sheet, a)['numFmt'] ?? 'General',
      };
      final code = await showDialog<String>(
        context: context,
        builder: (_) => SheetNumberFormatDialog(
          format: codes.length == 1 ? codes.single as String : null,
          value: widget.calculated[addresses.first] ?? raw(addresses.first),
          font: cellStyle(widget.sheet, addresses.first),
          preview: widget.onNumberPreview!,
        ),
      );
      if (!mounted || code == null) return;
      if (widget.sheet['id'] != id ||
          widget.revision != revision ||
          widget.workspaceId != workspaceId) {
        throw StateError(
          'The workbook changed. Open Number format again to review the current cells.',
        );
      }
      await widget.onFormat!(addresses, {
        'numFmt': code,
      }, baseRevision: revision);
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    }
  }

  Future<void>? _alignmentDialogTask;
  Future<void> editAlignment() => _alignmentDialogTask ??= _editAlignment()
      .whenComplete(() => _alignmentDialogTask = null);
  Future<void> _editAlignment() async {
    if (widget.onFormat == null) return;
    final id = widget.sheet['id'], range = selectedRange;
    try {
      final addresses = selectedAddresses();
      if (!await commit() || !await commitFormula()) return;
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted || widget.sheet['id'] != id || selectedRange != range) {
        return;
      }
      final revision = widget.revision,
          value = widget.calculated[addresses.first] ?? raw(addresses.first);
      final style = cellStyle(widget.sheet, addresses.first);
      final changes = await showDialog<Map<String, dynamic>>(
        context: context,
        builder: (_) => SheetAlignmentDialog(
          styles: [for (final a in addresses) cellStyle(widget.sheet, a)],
          previewStyle: style,
          sample: formatted(addresses.first),
          value: value,
          normalFont: normalSheetFont(widget.sheet),
        ),
      );
      if (!mounted || changes == null || changes.isEmpty) return;
      if (widget.sheet['id'] != id || widget.revision != revision) {
        throw StateError(
          'The workbook changed. Open Alignment again to review the current cells.',
        );
      }
      await widget.onFormat!(addresses, changes, baseRevision: revision);
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    }
  }

  String? literalText(String value) {
    if (value.startsWith("'")) return value.substring(1);
    if (value.startsWith('=') ||
        double.tryParse(value) != null ||
        const ['TRUE', 'FALSE'].contains(value.toUpperCase())) {
      return null;
    }
    return value;
  }

  Map<String, dynamic> fontAt(String address) {
    final style = cellStyle(widget.sheet, address);
    final text = formatted(address);
    return commonSheetFont(sheetFontRuns(text, style).map((r) => r.font));
  }

  Future<void> structure(String action, {int? at, int? count}) async {
    final isRows = action.endsWith('Rows');
    final ends = selectedRange.split(':');
    final (r1, c1) = parseCell(ends.first);
    final (r2, c2) = parseCell(ends.last);
    try {
      await flush();
      await widget.onStructure?.call({
        'action': action,
        'at': at ?? (isRows ? r1 : c1) + 1,
        'count': count ?? (isRows ? r2 - r1 : c2 - c1) + 1,
      });
      if (!mounted) return;
      await select(row, col, extend: false);
    } catch (e) {
      fail(e);
    }
  }

  Future<void> geometry(
    String action, {
    int? at,
    int? count,
    double? size,
  }) async {
    final rows = action.endsWith('Rows') || action == 'rowHeight';
    final area = TableArea.parse(selectedRange);
    try {
      await flush();
      if (!mounted) return;
      if (action == 'rowHeight' || action == 'columnWidth') {
        final index = (at ?? (rows ? area.top : area.left) + 1) - 1;
        final fromDialog = size == null;
        size ??= await showDialog<double>(
          context: context,
          builder: (_) => SheetSizeDialog(
            rows: rows,
            maxSize: rows ? 409 : columnMetrics.maxCharacters,
            size: rows
                ? rowLayout.extent(index) * 3 / 4
                : columnMetrics.characters(
                    widget.sheet['columnWidths']?['${index + 1}'] as num? ??
                        columnMetrics.fitPixels(columnLayout.extent(index)),
                  ),
          ),
        );
        if (size == null || !mounted) return;
        if (!rows && fromDialog) size = columnMetrics.fromCharacters(size);
      }
      await widget.onGeometry?.call({
        'axis': rows ? 'rows' : 'columns',
        'action': action.startsWith('unhideAll')
            ? 'unhideAll'
            : action.startsWith('unhide')
            ? 'unhide'
            : action.startsWith('hide')
            ? 'hide'
            : 'resize',
        'at': at ?? (rows ? area.top : area.left) + 1,
        'count':
            count ??
            (rows ? area.bottom - area.top : area.right - area.left) + 1,
        'size': ?size,
      });
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      fail(e);
    }
  }

  Future<void>? _autoFitTask;
  Future<void> autoFitRows({int? at, int? count}) =>
      autoFitDimensions(true, at: at, count: count);
  Future<void> autoFitColumns({int? at, int? count}) =>
      autoFitDimensions(false, at: at, count: count);
  Future<void> autoFitDimensions(bool rows, {int? at, int? count}) =>
      _autoFitTask ??= _autoFitDimensions(
        rows,
        at: at,
        count: count,
      ).whenComplete(() => _autoFitTask = null);
  Future<void> _autoFitDimensions(bool rows, {int? at, int? count}) async {
    if (widget.onGeometry == null) return;
    final id = widget.sheet['id'], range = selectedRange;
    final area = TableArea.parse(range);
    final first = at == null ? (rows ? area.top : area.left) : at - 1;
    final length =
        count ?? (rows ? area.bottom - area.top : area.right - area.left) + 1;
    ScaffoldFeatureController<SnackBar, SnackBarClosedReason>? progress;
    var cancelled = false;
    try {
      if (!await commit() || !await commitFormula()) return;
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted || widget.sheet['id'] != id || selectedRange != range) {
        return;
      }
      final revision = widget.revision;
      if (length > 200 || (widget.sheet['cells'] as Map).length > 2000) {
        progress = ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              rows ? 'Fitting row heights…' : 'Fitting column widths…',
            ),
            duration: const Duration(days: 1),
            action: SnackBarAction(
              label: 'Cancel',
              onPressed: () => cancelled = true,
            ),
          ),
        );
      }
      final scaler = MediaQuery.textScalerOf(context),
          direction = Directionality.of(context);
      Future<void> checkpoint() async {
        await Future<void>.delayed(Duration.zero);
        if (cancelled) {
          throw StateError('AutoFit cancelled. No dimensions changed.');
        }
        if (!mounted ||
            widget.sheet['id'] != id ||
            widget.revision != revision) {
          throw StateError('The workbook changed. Run AutoFit again.');
        }
      }

      final sizes = rows
          ? await measureSheetRows(
              sheet: widget.sheet,
              calculated: widget.calculated,
              formatted: widget.formattedCells,
              first: first,
              count: length,
              columns: columnLayout,
              scaler: scaler,
              direction: direction,
              checkpoint: checkpoint,
            )
          : await measureSheetColumns(
              sheet: widget.sheet,
              calculated: widget.calculated,
              formatted: widget.formattedCells,
              first: first,
              count: length,
              columns: columnLayout,
              scaler: scaler,
              direction: direction,
              checkpoint: checkpoint,
              metrics: columnMetrics,
            );
      progress?.close();
      progress = null;
      if (sizes.isEmpty) return;
      await widget.onGeometry!({
        'axis': rows ? 'rows' : 'columns',
        'action': 'autoFit',
        'at': first + 1,
        'count': length,
        'sizes': sizes,
        'baseRevision': revision,
        'sheetId': id,
      });
      if (mounted) gridFocus.requestFocus();
    } catch (e) {
      progress?.close();
      if (mounted) fail(e);
    }
  }

  Future<void> rowColumnAction(String action, {int? at, int? count}) =>
      action == 'autoFitRows'
      ? autoFitRows(at: at, count: count)
      : action == 'autoFitColumns'
      ? autoFitColumns(at: at, count: count)
      : action.startsWith('insert') || action.startsWith('delete')
      ? structure(action, at: at, count: count)
      : geometry(action, at: at, count: count);

  void beginResize(bool rows, int index) {
    setState(() {
      dragRows = rows;
      dragIndex = index;
      dragExtent = rows ? rowLayout.extent(index) : columnLayout.extent(index);
    });
  }

  void updateResize(double delta) {
    setState(
      () => dragExtent = (dragExtent! + delta).clamp(
        1.0,
        dragRows ? 409 * 4 / 3 : columnMetrics.pixels(255),
      ),
    );
  }

  Future<void> endResize() async {
    final rows = dragRows, index = dragIndex!, extent = dragExtent!;
    setState(() {
      dragIndex = null;
      dragExtent = null;
    });
    await geometry(
      rows ? 'rowHeight' : 'columnWidth',
      at: index + 1,
      count: 1,
      size: extent <= 5
          ? 0
          : rows
          ? double.parse((extent * 3 / 4).toStringAsFixed(2))
          : columnMetrics.fitPixels(extent),
    );
  }

  Future<void> headerMenu(int r, int c, Offset position) async {
    final selectedArea = TableArea.parse(selectedRange);
    final keepRows = c < 0
        ? r >= selectedArea.top && r <= selectedArea.bottom
        : r < 0 && c >= selectedArea.left && c <= selectedArea.right;
    if (!keepRows) {
      await select(r < 0 ? row : r, c < 0 ? col : c, extend: false);
    }
    if (!mounted) return;
    final rows = r >= 0;
    final action = await showMenu<String>(
      context: context,
      position: RelativeRect.fromLTRB(
        position.dx,
        position.dy,
        position.dx,
        position.dy,
      ),
      items: [
        PopupMenuItem(
          value: rows ? 'insertRows' : 'insertColumns',
          child: Text(rows ? 'Insert row above' : 'Insert column left'),
        ),
        PopupMenuItem(
          value: rows ? 'deleteRows' : 'deleteColumns',
          child: Text(rows ? 'Delete row' : 'Delete column'),
        ),
        if (widget.onGeometry != null) ...[
          const PopupMenuDivider(),
          PopupMenuItem(
            value: rows ? 'rowHeight' : 'columnWidth',
            child: Text(rows ? 'Row height…' : 'Column width…'),
          ),
          PopupMenuItem(
            value: rows ? 'autoFitRows' : 'autoFitColumns',
            child: Text(rows ? 'AutoFit Row Height' : 'AutoFit Column Width'),
          ),
          PopupMenuItem(
            value: rows ? 'hideRows' : 'hideColumns',
            child: Text(rows ? 'Hide row' : 'Hide column'),
          ),
          PopupMenuItem(
            value: rows ? 'unhideRows' : 'unhideColumns',
            child: Text(
              rows ? 'Unhide adjacent rows' : 'Unhide adjacent columns',
            ),
          ),
        ],
      ],
    );
    if (action != null && mounted) {
      if (action.startsWith('autoFit') && keepRows) {
        await autoFitDimensions(rows);
        return;
      }
      final visible = rows ? visibleRows : visibleColumns;
      var first = rows ? r : c, last = first;
      if (action.startsWith('unhide')) {
        while (first > 0 && visible.isHidden(first - 1)) {
          first--;
        }
        while (last + 1 < visible.limit && visible.isHidden(last + 1)) {
          last++;
        }
      }
      await rowColumnAction(action, at: first + 1, count: last - first + 1);
    }
  }

  Future<void> tableAction(Map<String, dynamic> action, {Map? table}) async {
    try {
      await flush();
      await widget.onTable?.call({
        if (table != null) 'tableId': table['id'],
        ...action,
      });
    } catch (e) {
      fail(e);
    }
  }

  String suggestedTableRange() {
    if (hasRangeSelection) return selectedRange;
    var top = row, bottom = row, left = col, right = col;
    while (top > 0 && raw(cellAddress(top - 1, col)).isNotEmpty) {
      top--;
    }
    while (left > 0 && raw(cellAddress(top, left - 1)).isNotEmpty) {
      left--;
    }
    while (right < 16383 && raw(cellAddress(top, right + 1)).isNotEmpty) {
      right++;
    }
    while (bottom < 1048575 && bottom - top < 100000) {
      var filled = false;
      for (var c = left; c <= right; c++) {
        if (raw(cellAddress(bottom + 1, c)).isNotEmpty) {
          filled = true;
          break;
        }
      }
      if (!filled) break;
      bottom++;
    }
    return '${cellAddress(top, left)}:${cellAddress(bottom.clamp(top < 1048575 ? top + 1 : top, 1048575), right)}';
  }

  Future<void> createTable() async {
    await flush();
    if (!mounted) return;
    final result = await tableCreateDialog(context, suggestedTableRange());
    if (result != null && mounted) await tableAction(result);
  }

  Future<void> sortTable(Map table) async {
    await flush();
    if (!mounted) return;
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => TableSortDialog(table: table),
    );
    if (result != null && mounted) await tableAction(result, table: table);
  }

  Future<void> sortSelection() async {
    await flush();
    if (!mounted) return;
    final table = selectedTable;
    if (table != null && !hasRangeSelection) {
      await sortTable(table);
      return;
    }
    final last = widget.sheet['rangeSort'] as Map?;
    final reuse =
        !hasRangeSelection &&
        last != null &&
        TableArea.parse(last['range']).contains(row, col);
    final initial = reuse ? last['range'] as String : suggestedTableRange();
    final a = TableArea.parse(initial);
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => TableSortDialog(
        table: {
          'range': initial,
          if (reuse) ...{
            'headers': last['headers'],
            'sort': last['keys'],
            'sortCaseSensitive': last['caseSensitive'],
          },
          'columns': [
            for (var c = a.left; c <= a.right; c++)
              {'name': 'Column ${columnLabel(c)}'},
          ],
        },
        cells: {
          for (final e in widget.calculated.entries)
            e.key: formatted(e.key as String),
        },
      ),
    );
    if (result != null && mounted) {
      try {
        await flush();
        await widget.onSortRange?.call(result);
      } catch (e) {
        fail(e);
      }
    }
  }

  Future<void> resizeTable(Map table) async {
    await flush();
    if (!mounted) return;
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => TableResizeDialog(table: table),
    );
    if (result != null && mounted) await tableAction(result, table: table);
  }

  Future<void> filterTable(Map table, int column) async {
    await flush();
    if (!mounted) return;
    final area = TableArea.parse(table['range']);
    final values = <String, String>{};
    final numericValues = <String>{};
    var truncated = false;
    if (widget.onFilterChoices != null) {
      try {
        final choices = await widget.onFilterChoices!({
          if (table['id'] != null) 'tableId': table['id'],
          'column': column,
        });
        for (final item in choices['values'] as List) {
          if (item['value'] is num) numericValues.add(item['key'] as String);
          values[item['key'] as String] = item['key'] == ''
              ? '(Blanks)'
              : item['display'] as String? ?? displayCell(item['value']);
        }
        truncated = choices['truncated'] == true;
      } catch (e) {
        fail(e);
        return;
      }
    } else {
      for (
        var r = area.top + (table['headerRows'] as int? ?? 1);
        r <= area.bottom - (table['totalsRows'] as int? ?? 0);
        r++
      ) {
        final value = widget.calculated[cellAddress(r, area.left + column)];
        final key = value?.toString() ?? '';
        if (value is num) numericValues.add(key);
        values[key] = key.isEmpty
            ? '(Blanks)'
            : formatted(cellAddress(r, area.left + column));
      }
    }
    if (!mounted) return;
    final sorted = values.entries.toList()
      ..sort((a, b) => a.value.toLowerCase().compareTo(b.value.toLowerCase()));
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => TableFilterDialog(
        table: table,
        column: column,
        values: [for (final v in sorted) (v.key, v.value)],
        truncated: truncated,
        numericValues: numericValues,
      ),
    );
    if (result != null && mounted) {
      if (table['id'] == null) {
        await rangeFilterAction(result);
      } else {
        await tableAction(result, table: table);
      }
    }
  }

  Map? get rangeFilter {
    final filter = widget.sheet['autoFilter'] as Map?;
    if (filter == null) return null;
    final a = TableArea.parse(filter['range']);
    return {
      ...filter,
      'columns': [
        for (var c = a.left; c <= a.right; c++)
          {
            'name': formatted(cellAddress(a.top, c)).trim().isEmpty
                ? 'Column ${columnLabel(c)}'
                : formatted(cellAddress(a.top, c)),
          },
      ],
    };
  }

  Future<void> rangeFilterAction(Map<String, dynamic> action) async {
    try {
      await flush();
      await widget.onAutoFilter?.call(action);
    } catch (e) {
      fail(e);
    }
  }

  Future<void> rangeFilterMenu(String action) async {
    await flush();
    if (!mounted) return;
    if (action == 'enable') {
      final range = await worksheetFilterDialog(context, suggestedTableRange());
      if (range != null && mounted) {
        await rangeFilterAction({'action': 'enable', 'range': range});
      }
    } else if (action == 'filter') {
      final f = rangeFilter;
      if (f != null) {
        final a = TableArea.parse(f['range']);
        await filterTable(f, (col - a.left).clamp(0, a.right - a.left));
      }
    } else {
      await rangeFilterAction({'action': action});
    }
  }

  Future<void> editTableText(
    Map table,
    bool formula, {
    bool total = false,
  }) async {
    await flush();
    if (!mounted) return;
    final area = TableArea.parse(table['range']);
    final ci = (col - area.left).clamp(0, area.right - area.left);
    final controller = TextEditingController(
      text: total
          ? table['columns'][ci]['totalsFormula'] ?? '='
          : formula
          ? table['columns'][ci]['formula'] ?? '='
          : table['name'],
    );
    final text = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(
          total
              ? 'Total for ${table['columns'][ci]['name']}'
              : formula
              ? 'Formula for ${table['columns'][ci]['name']}'
              : 'Rename table',
        ),
        content: SizedBox(
          width: 380,
          child: draftTextField(
            key: const Key('table-edit-text'),
            controller: controller,
            autofocus: true,
            decoration: InputDecoration(
              labelText: total
                  ? 'Custom total formula'
                  : formula
                  ? 'Calculated column formula'
                  : 'Table name',
              helperText: total
                  ? 'Example: =SUM(Sales[Amount])*0.08'
                  : formula
                  ? 'Fills every data row. Example: =[@Qty]*[@Price]'
                  : null,
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('table-text-apply'),
            onPressed: () => Navigator.pop(context, controller.text),
            child: const Text('Apply'),
          ),
        ],
      ),
    );
    await Future<void>.delayed(const Duration(milliseconds: 250));
    controller.dispose();
    if (text != null && mounted) {
      await tableAction(
        total
            ? {
                'action': 'totalFunction',
                'column': ci,
                'function': 'custom',
                'formula': text,
              }
            : formula
            ? {'action': 'columnFormula', 'column': ci, 'formula': text}
            : {'action': 'rename', 'name': text},
        table: table,
      );
    }
  }

  Future<void> tableMenuAction(String action) async {
    final table = selectedTable;
    if (table == null) return;
    switch (action) {
      case 'sort':
        await sortTable(table);
      case 'filter':
        await filterTable(table, col - TableArea.parse(table['range']).left);
      case 'clearFilters':
        await tableAction({'action': 'clearFilters'}, table: table);
      case 'reapply':
        final filters = table['filters'] as Map? ?? {};
        if (filters.isNotEmpty) {
          final col = filters.keys.first;
          await tableAction({
            'action': 'filter',
            'column': int.parse(col),
            'rule': filters[col],
          }, table: table);
        }
      case 'totals':
        await tableAction({
          'action': 'totals',
          'enabled': table['totalsRows'] != 1,
        }, table: table);
      case 'totalSum':
      case 'totalAverage':
      case 'totalCount':
        await tableAction({
          'action': 'totalFunction',
          'column': col - TableArea.parse(table['range']).left,
          'function': action == 'totalSum'
              ? 'sum'
              : action == 'totalAverage'
              ? 'average'
              : 'count',
        }, table: table);
      case 'rename':
        await editTableText(table, false);
      case 'resize':
        await resizeTable(table);
      case 'columnFormula':
        await editTableText(table, true);
      case 'customTotal':
        await editTableText(table, true, total: true);
      default:
        if (action.startsWith('TableStyle')) {
          await tableAction({'action': 'style', 'style': action}, table: table);
        }
    }
  }

  String visibleTableRows(Map table) {
    final a = TableArea.parse(table['range']);
    final first = a.top + (table['headerRows'] as int? ?? 1);
    final end = a.bottom + 1 - (table['totalsRows'] as int? ?? 0);
    final count =
        end - first - (visibleRows.before(end) - visibleRows.before(first));
    return '$count ${count == 1 ? 'row' : 'rows'} shown';
  }

  Color tableColor(Map table) => switch (table['style']) {
    'TableStyleMedium4' => const Color(0xff26734b),
    'TableStyleMedium9' => const Color(0xffa34f19),
    'TableStyleLight1' => const Color(0xff565e69),
    _ => const Color(0xff285e91),
  };

  Color? color(dynamic value) {
    if (value is! String) return null;
    final n = int.tryParse(value.replaceFirst('#', ''), radix: 16);
    return n == null ? null : Color(0xff000000 | n);
  }

  SheetOverflowIndex? _overflowIndex;
  final _overflowPainters = <String, SheetFontRenderBox?>{};
  TextScaler _overflowScaler = TextScaler.noScaling;
  TextDirection _overflowDirection = TextDirection.ltr;

  void _clearOverflowPainters() {
    for (final painter in _overflowPainters.values) {
      painter?.dispose();
    }
    _overflowPainters.clear();
  }

  SheetFontRenderBox? _overflowPainter(int r, int c) {
    final address = cellAddress(r, c);
    if (_overflowPainters.containsKey(address)) {
      final cached = _overflowPainters.remove(address);
      _overflowPainters[address] = cached;
      return cached;
    }
    if (_overflowPainters.length >= 512) {
      _overflowPainters.remove(_overflowPainters.keys.first)?.dispose();
    }
    _overflowPainters[address] = null;
    if ((editing || hasValidationDropdown) && r == row && c == col) return null;
    if (displayAt(address)?['isError'] == true) return null;
    final value = widget.calculated[address] ?? raw(address);
    if (value is! String || value.isEmpty) return null;
    if (!widget.calculated.containsKey(address) &&
        (value.startsWith('=') ||
            num.tryParse(value) != null ||
            const ['TRUE', 'FALSE'].contains(value.toUpperCase()))) {
      return null;
    }
    final style = numberDisplayStyle(
      cellStyle(widget.sheet, address),
      displayAt(address),
    );
    final alignment = style['horizontalAlignment'] ?? 'general';
    if (!const ['general', 'left', 'right', 'center'].contains(alignment) ||
        style['wrapText'] == true ||
        style['shrinkToFit'] == true ||
        (style['textRotation'] as num? ?? 0) != 0 ||
        (style['_dataBar'] as Map?)?['showValue'] == false) {
      return null;
    }
    final width = columnExtent(c), active = r == row && c == col;
    final padding = width < 42 ? 0.0 : 9.0;
    final painter = SheetFontRenderBox(
      sheetFontRuns(formatted(address), style),
      color(style['color']) ?? const Color(0xff242424),
      _overflowDirection,
      _overflowScaler,
      CellTextLayout(
        style,
        value: value,
        normalFont: normalSheetFont(widget.sheet),
      ),
    )..overflowPaint = true;
    painter.layout(
      BoxConstraints.tight(
        Size(
          (width - padding * 2 - (active ? 3.2 : 1)).clamp(0, double.infinity),
          (rowExtent(r) - 4 - (active ? 3.2 : 1)).clamp(0, double.infinity),
        ),
      ),
    );
    if (painter.textBounds.left >= 0 &&
        painter.textBounds.right <= painter.size.width) {
      painter.dispose();
      return null;
    }
    _overflowPainters[address] = painter;
    return painter;
  }

  void _paintOverflow(
    PaintingContext context,
    Offset offset,
    List<SheetGridPaintCell> cells,
  ) {
    for (final target in cells) {
      final r = target.row, c = target.column;
      if (r < 0 || c < 0 || (editing && r == row && c == col)) continue;
      final candidates =
          _overflowIndex?.candidates(r, c, displayedFrozenColumns) ??
          const <int>[];
      final origins = <int, Offset>{};
      final fragments = <SheetOverflowFragment>[];
      for (final source in candidates) {
        final painter = _overflowPainter(r, source);
        if (painter == null) continue;
        final active = r == row && source == col;
        final padding = columnExtent(source) < 42 ? 0.0 : 9.0;
        final origin =
            offset +
            target.bounds.topLeft +
            Offset(
              columnLayout.offset(source) -
                  columnLayout.offset(c) +
                  padding +
                  (active ? 1.6 : 0),
              2 + (active ? 1.6 : 0),
            );
        origins[source] = origin;
        fragments.add(
          SheetOverflowFragment(source, painter.textBounds.shift(origin)),
        );
      }
      if (fragments.isEmpty) continue;
      final active = r == row && c == col;
      final selectedArea = hasRangeSelection ? selectionArea : null;
      final style = numberDisplayStyle(
        cellStyle(widget.sheet, cellAddress(r, c)),
        displayAt(cellAddress(r, c)),
      );
      var background = color(style['fill']) ?? const Color(0xffffffff);
      if (active || selectedArea?.contains(r, c) == true) {
        background = Color.alphaBlend(const Color(0x18107c41), background);
      }
      var targetRect = target.bounds.shift(offset);
      if (active) targetRect = targetRect.deflate(1.6);
      for (final fragment in clipOverflowFragments(fragments)) {
        final painter = _overflowPainters[cellAddress(r, fragment.source)]!;
        final origin = origins[fragment.source]!;
        final content = origin & painter.size;
        final regions = fragment.source == c
            ? [
                Rect.fromLTRB(
                  targetRect.left,
                  targetRect.top,
                  content.left,
                  targetRect.bottom,
                ),
                Rect.fromLTRB(
                  content.right,
                  targetRect.top,
                  targetRect.right,
                  targetRect.bottom,
                ),
              ]
            : [targetRect];
        for (final region in regions) {
          final clip = region.intersect(fragment.ink);
          if (clip.isEmpty) continue;
          context.canvas.drawRect(clip, Paint()..color = background);
          painter.paintWithClip(context, origin, clip);
        }
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    _clearOverflowPainters();
    _overflowScaler = MediaQuery.textScalerOf(context);
    _overflowDirection = Directionality.of(context);
    final worksheetFilter = rangeFilter;
    final filterArea = worksheetFilter == null
        ? null
        : TableArea.parse(worksheetFilter['range']);
    _overflowIndex ??= SheetOverflowIndex(
      {...widget.calculated, ...widget.sheet['cells'] as Map},
      visibleColumns,
      [
        if (filterArea != null)
          TableArea(
            filterArea.top,
            filterArea.left,
            filterArea.top,
            filterArea.right,
          ),
        for (final range in widget.sheet['mergedRanges'] as List? ?? [])
          TableArea.parse(range as String),
        for (final table in widget.sheet['tables'] as List? ?? [])
          TableArea.parse(table['range'] as String),
      ],
    );
    final activeFont = fontAt(selected);
    const line = Color(0xffdfe3e8);
    return Column(
      children: [
        if (_entryPaused && _entryKeys.isNotEmpty)
          Material(
            key: const Key('pending-sheet-entries'),
            color: const Color(0xfffff4d6),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      _sameEntryContext
                          ? 'More typing is waiting. Correct this cell and press Enter, or press Esc to skip it.'
                          : 'The worksheet changed. Waiting typing has not been applied.',
                    ),
                  ),
                  if (!_sameEntryContext)
                    TextButton(
                      key: const Key('copy-waiting-keys'),
                      onPressed: _copyWaitingKeys,
                      child: const Text('Copy typed keys'),
                    ),
                  TextButton(
                    key: const Key('discard-queued-typing'),
                    onPressed: _discardQueuedTyping,
                    child: const Text('Discard waiting entries'),
                  ),
                ],
              ),
            ),
          ),
        Container(
          height: 82,
          color: const Color(0xfffafafa),
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
          child: LayoutBuilder(
            builder: (context, viewport) {
              final wide = viewport.maxWidth >= 900;
              return SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(
                  children: [
                    Column(
                      children: [
                        Row(
                          children: [
                            IconButton(
                              tooltip: 'Copy cells',
                              constraints: const BoxConstraints(
                                minWidth: 32,
                                minHeight: 40,
                              ),
                              padding: const EdgeInsets.all(4),
                              visualDensity: VisualDensity.compact,
                              onPressed: copy,
                              icon: const Icon(Icons.content_copy, size: 20),
                            ),
                            IconButton(
                              tooltip: 'Paste cells',
                              constraints: const BoxConstraints(
                                minWidth: 32,
                                minHeight: 40,
                              ),
                              padding: const EdgeInsets.all(4),
                              visualDensity: VisualDensity.compact,
                              onPressed: () => paste(),
                              icon: const Icon(Icons.content_paste, size: 22),
                            ),
                            SizedBox(
                              width: 24,
                              child: PopupMenuButton<String>(
                                key: const Key('paste-cells-menu'),
                                tooltip: 'Paste options',
                                icon: const Icon(
                                  Icons.arrow_drop_down,
                                  size: 18,
                                ),
                                onSelected: (value) {
                                  if (value == 'special') {
                                    pasteSpecial();
                                  } else if (value == 'text') {
                                    paste(textOnly: true);
                                  } else {
                                    paste(mode: value);
                                  }
                                },
                                itemBuilder: (_) => const [
                                  PopupMenuItem(
                                    value: 'all',
                                    child: Text('Cells and formatting'),
                                  ),
                                  PopupMenuItem(
                                    value: 'values',
                                    child: Text('Values only'),
                                  ),
                                  PopupMenuItem(
                                    value: 'formulas',
                                    child: Text('Formulas and values'),
                                  ),
                                  PopupMenuItem(
                                    value: 'formats',
                                    child: Text('Formatting only'),
                                  ),
                                  PopupMenuDivider(),
                                  PopupMenuItem(
                                    value: 'text',
                                    child: Text('Paste text'),
                                  ),
                                  PopupMenuItem(
                                    value: 'special',
                                    child: Text('Paste special…'),
                                  ),
                                ],
                              ),
                            ),
                          ],
                        ),
                        const Text(
                          'Clipboard',
                          style: TextStyle(fontSize: 9, color: Colors.grey),
                        ),
                      ],
                    ),
                    const VerticalDivider(width: 24),
                    Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        SizedBox(
                          width: 120,
                          height: 24,
                          child: Row(
                            children: [
                              Expanded(
                                child: TextButton(
                                  key: const Key('cell-font-button'),
                                  onPressed: editFont,
                                  style: TextButton.styleFrom(
                                    padding: const EdgeInsets.symmetric(
                                      horizontal: 2,
                                    ),
                                    minimumSize: Size.zero,
                                  ),
                                  child: Text(
                                    '${activeFont['fontName'] ?? 'Mixed fonts'} · ${activeFont['fontSize'] == null ? 'Mixed sizes' : fontPoints(activeFont['fontSize'] as num)}',
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                    style: const TextStyle(fontSize: 11),
                                  ),
                                ),
                              ),
                              if (!wide)
                                SizedBox(
                                  width: 24,
                                  height: 24,
                                  child: IconButton(
                                    key: const Key('cell-alignment-button'),
                                    tooltip: 'Alignment…',
                                    onPressed: editAlignment,
                                    padding: EdgeInsets.zero,
                                    icon: const Icon(
                                      Icons.format_align_left,
                                      size: 17,
                                    ),
                                  ),
                                ),
                            ],
                          ),
                        ),
                        SizedBox(
                          height: 40,
                          child: Row(
                            children: [
                              IconButton(
                                tooltip: 'Bold',
                                onPressed: () => format({
                                  'bold': activeFont['bold'] != true,
                                }),
                                icon: const Icon(Icons.format_bold, size: 19),
                              ),
                              IconButton(
                                tooltip: 'Italic',
                                onPressed: () => format({
                                  'italic': activeFont['italic'] != true,
                                }),
                                icon: const Icon(Icons.format_italic, size: 19),
                              ),
                              IconButton(
                                tooltip: 'Highlight cells',
                                onPressed: () => format({'fill': '#FFF2CC'}),
                                icon: const Icon(
                                  Icons.format_color_fill,
                                  size: 19,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                    if (wide) ...[
                      const VerticalDivider(width: 24),
                      Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          SizedBox(
                            height: 24,
                            child: TextButton(
                              key: const Key('cell-alignment-button'),
                              onPressed: editAlignment,
                              style: TextButton.styleFrom(
                                padding: EdgeInsets.zero,
                                minimumSize: Size.zero,
                              ),
                              child: const Text(
                                'Alignment…',
                                style: TextStyle(fontSize: 11),
                              ),
                            ),
                          ),
                          SizedBox(
                            width: 128,
                            height: 32,
                            child: Row(
                              children: [
                                for (final (value, icon, label) in const [
                                  (
                                    'left',
                                    Icons.format_align_left,
                                    'Align left',
                                  ),
                                  (
                                    'center',
                                    Icons.format_align_center,
                                    'Center text',
                                  ),
                                  (
                                    'right',
                                    Icons.format_align_right,
                                    'Align right',
                                  ),
                                ])
                                  SizedBox(
                                    width: 32,
                                    height: 32,
                                    child: IconButton(
                                      tooltip: label,
                                      padding: EdgeInsets.zero,
                                      onPressed: () => format({
                                        'horizontalAlignment': value,
                                      }),
                                      icon: Icon(icon, size: 18),
                                    ),
                                  ),
                                SizedBox(
                                  width: 32,
                                  height: 32,
                                  child: IconButton(
                                    tooltip: 'Wrap text',
                                    padding: EdgeInsets.zero,
                                    onPressed: () => format({
                                      'wrapText':
                                          cellStyle(
                                            widget.sheet,
                                            cellAddress(row, col),
                                          )['wrapText'] !=
                                          true,
                                      'shrinkToFit': false,
                                    }),
                                    icon: const Icon(Icons.wrap_text, size: 18),
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ],
                    const VerticalDivider(width: 24),
                    Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        DropdownButtonHideUnderline(
                          child: DropdownButton<String>(
                            isDense: true,
                            hint: const Text(
                              'Number format',
                              style: TextStyle(fontSize: 12),
                            ),
                            items: const [
                              DropdownMenuItem(
                                value: 'General',
                                child: Text('General'),
                              ),
                              DropdownMenuItem(
                                value: '#,##0.00',
                                child: Text('Number'),
                              ),
                              DropdownMenuItem(
                                value: r'$#,##0.00',
                                child: Text('Currency'),
                              ),
                              DropdownMenuItem(
                                value: '0.00%',
                                child: Text('Percentage'),
                              ),
                            ],
                            onChanged: (v) {
                              if (v != null) format({'numFmt': v});
                            },
                          ),
                        ),
                        SizedBox(
                          height: 24,
                          child: TextButton(
                            onPressed: widget.onNumberPreview == null
                                ? null
                                : editNumberFormat,
                            child: const Text(
                              'More formats…',
                              style: TextStyle(fontSize: 10),
                            ),
                          ),
                        ),
                      ],
                    ),
                    const VerticalDivider(width: 24),
                    Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        SizedBox(
                          width: 48,
                          height: 32,
                          child: IconButton(
                            tooltip: 'Chart',
                            padding: const EdgeInsets.all(4),
                            onPressed: () => setState(() => chart = !chart),
                            icon: const Icon(Icons.bar_chart, size: 19),
                          ),
                        ),
                        if (widget.onFill != null)
                          PopupMenuButton<String>(
                            key: const Key('fill-cells-menu'),
                            tooltip: 'Fill cells',
                            onSelected: fillSelection,
                            itemBuilder: (_) => [
                              PopupMenuItem(
                                value: 'down',
                                child: Text(
                                  defaultTargetPlatform == TargetPlatform.macOS
                                      ? 'Fill down  ·  ⌘D'
                                      : 'Fill down  ·  Ctrl+D',
                                ),
                              ),
                              PopupMenuItem(
                                value: 'right',
                                child: Text(
                                  defaultTargetPlatform == TargetPlatform.macOS
                                      ? 'Fill right  ·  ⌘R'
                                      : 'Fill right  ·  Ctrl+R',
                                ),
                              ),
                              const PopupMenuItem(
                                value: 'up',
                                child: Text('Fill up'),
                              ),
                              const PopupMenuItem(
                                value: 'left',
                                child: Text('Fill left'),
                              ),
                            ],
                            child: const SizedBox(
                              width: 48,
                              height: 32,
                              child: Center(
                                child: Text(
                                  'Fill ▾',
                                  style: TextStyle(fontSize: 12),
                                ),
                              ),
                            ),
                          ),
                      ],
                    ),
                    Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        if (widget.onValidation != null)
                          SizedBox(
                            width: 48,
                            height: 32,
                            child: IconButton(
                              key: const Key('data-validation-button'),
                              tooltip: 'Data validation',
                              icon: const Icon(Icons.rule, size: 20),
                              onPressed: editValidation,
                            ),
                          ),
                        if (widget.onView != null)
                          SizedBox(
                            width: 48,
                            height: 32,
                            child: PopupMenuButton<String>(
                              key: const Key('freeze-panes-menu'),
                              tooltip: 'Freeze panes',
                              icon: const Icon(
                                Icons.view_quilt_outlined,
                                size: 20,
                              ),
                              onSelected: freezePanes,
                              itemBuilder: (_) => [
                                PopupMenuItem(
                                  value: 'selection',
                                  enabled: row > 0 || col > 0,
                                  child: const Text(
                                    'Freeze panes at selection',
                                  ),
                                ),
                                const PopupMenuItem(
                                  value: 'topRow',
                                  child: Text('Freeze top row'),
                                ),
                                const PopupMenuItem(
                                  value: 'firstColumn',
                                  child: Text('Freeze first column'),
                                ),
                                const PopupMenuDivider(),
                                PopupMenuItem(
                                  value: 'unfreeze',
                                  enabled: widget.sheet['frozenPane'] != null,
                                  child: const Text('Unfreeze panes'),
                                ),
                              ],
                            ),
                          ),
                      ],
                    ),
                    if (widget.onTable != null ||
                        widget.onConditionalFormat != null)
                      Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          if (widget.onConditionalFormat != null)
                            SizedBox(
                              width: 48,
                              height: 32,
                              child: IconButton(
                                key: const Key('conditional-format-button'),
                                tooltip:
                                    _conditionalViewError ??
                                    (widget.conditionalWarnings.isNotEmpty
                                        ? 'Conditional formatting · some rules are not displayed'
                                        : 'Conditional formatting'),
                                onPressed: editConditionalFormats,
                                icon: Icon(
                                  _conditionalViewError != null ||
                                          widget.conditionalWarnings.isNotEmpty
                                      ? Icons.warning_amber
                                      : Icons.format_color_fill,
                                  size: 20,
                                ),
                              ),
                            ),
                          if (widget.onTable != null)
                            SizedBox(
                              width: 48,
                              height: 32,
                              child: IconButton(
                                key: const Key('table-create'),
                                tooltip: 'Format as table',
                                onPressed: createTable,
                                icon: const Icon(
                                  Icons.table_chart_outlined,
                                  size: 21,
                                ),
                              ),
                            ),
                        ],
                      ),
                    if (widget.onSortRange != null)
                      IconButton(
                        key: const Key('sort-range-button'),
                        tooltip: 'Sort',
                        onPressed: sortSelection,
                        icon: const Icon(Icons.sort_by_alpha, size: 20),
                      ),
                    if (widget.onAutoFilter != null)
                      PopupMenuButton<String>(
                        key: const Key('range-filter-menu'),
                        tooltip: 'Worksheet filters',
                        icon: Icon(
                          rangeFilter == null
                              ? Icons.filter_alt_outlined
                              : Icons.filter_alt,
                          size: 20,
                        ),
                        onSelected: rangeFilterMenu,
                        itemBuilder: (_) => [
                          if (rangeFilter == null)
                            const PopupMenuItem(
                              value: 'enable',
                              child: Text('Filter a range'),
                            )
                          else ...[
                            const PopupMenuItem(
                              value: 'filter',
                              child: Text('Filter selected column'),
                            ),
                            const PopupMenuItem(
                              value: 'reapply',
                              child: Text('Reapply worksheet filters'),
                            ),
                            const PopupMenuItem(
                              value: 'clearFilters',
                              child: Text('Clear worksheet filters'),
                            ),
                            const PopupMenuDivider(),
                            const PopupMenuItem(
                              value: 'remove',
                              child: Text('Remove worksheet filter'),
                            ),
                          ],
                        ],
                      ),
                    if (widget.onTable != null && selectedTable != null)
                      PopupMenuButton<String>(
                        key: const Key('table-menu'),
                        tooltip: 'Table options',
                        icon: const Icon(Icons.filter_list, size: 21),
                        onSelected: tableMenuAction,
                        itemBuilder: (_) => const [
                          PopupMenuItem(
                            value: 'sort',
                            child: Text('Sort table'),
                          ),
                          PopupMenuItem(
                            value: 'filter',
                            child: Text('Filter selected column'),
                          ),
                          PopupMenuItem(
                            value: 'reapply',
                            child: Text('Reapply filters'),
                          ),
                          PopupMenuItem(
                            value: 'clearFilters',
                            child: Text('Clear all filters'),
                          ),
                          PopupMenuDivider(),
                          PopupMenuItem(
                            value: 'rename',
                            child: Text('Rename table'),
                          ),
                          PopupMenuItem(
                            value: 'resize',
                            child: Text('Resize table'),
                          ),
                          PopupMenuItem(
                            value: 'columnFormula',
                            child: Text('Calculated column'),
                          ),
                          PopupMenuDivider(),
                          PopupMenuItem(
                            value: 'totals',
                            child: Text('Show / hide totals row'),
                          ),
                          PopupMenuItem(
                            value: 'totalSum',
                            child: Text('Total selected column: Sum'),
                          ),
                          PopupMenuItem(
                            value: 'totalAverage',
                            child: Text('Total selected column: Average'),
                          ),
                          PopupMenuItem(
                            value: 'totalCount',
                            child: Text('Total selected column: Count'),
                          ),
                          PopupMenuItem(
                            value: 'customTotal',
                            child: Text('Custom total formula'),
                          ),
                          PopupMenuDivider(),
                          PopupMenuItem(
                            value: 'TableStyleMedium2',
                            child: Text('Blue table style'),
                          ),
                          PopupMenuItem(
                            value: 'TableStyleMedium4',
                            child: Text('Green table style'),
                          ),
                          PopupMenuItem(
                            value: 'TableStyleMedium9',
                            child: Text('Orange table style'),
                          ),
                          PopupMenuItem(
                            value: 'TableStyleLight1',
                            child: Text('Gray table style'),
                          ),
                        ],
                      ),
                    if (widget.onStructure != null)
                      PopupMenuButton<String>(
                        key: const Key('structure-menu'),
                        tooltip: 'Rows and columns',
                        icon: const Icon(Icons.table_rows_outlined, size: 21),
                        onSelected: (action) => rowColumnAction(action),
                        itemBuilder: (_) => [
                          const PopupMenuItem(
                            value: 'insertRows',
                            child: Text('Insert selected rows above'),
                          ),
                          PopupMenuItem(
                            value: 'insertColumns',
                            child: Text('Insert selected columns left'),
                          ),
                          PopupMenuDivider(),
                          PopupMenuItem(
                            value: 'deleteRows',
                            child: Text('Delete selected rows'),
                          ),
                          PopupMenuItem(
                            value: 'deleteColumns',
                            child: Text('Delete selected columns'),
                          ),
                          if (widget.onGeometry != null) ...const [
                            PopupMenuDivider(),
                            PopupMenuItem(
                              value: 'rowHeight',
                              child: Text('Row height…'),
                            ),
                            PopupMenuItem(
                              value: 'autoFitRows',
                              child: Text('AutoFit Row Height'),
                            ),
                            PopupMenuItem(
                              value: 'columnWidth',
                              child: Text('Column width…'),
                            ),
                            PopupMenuItem(
                              value: 'autoFitColumns',
                              child: Text('AutoFit Column Width'),
                            ),
                            PopupMenuItem(
                              value: 'hideRows',
                              child: Text('Hide selected rows'),
                            ),
                            PopupMenuItem(
                              value: 'hideColumns',
                              child: Text('Hide selected columns'),
                            ),
                            PopupMenuItem(
                              value: 'unhideRows',
                              child: Text('Unhide selected rows'),
                            ),
                            PopupMenuItem(
                              value: 'unhideColumns',
                              child: Text('Unhide selected columns'),
                            ),
                            PopupMenuDivider(),
                            PopupMenuItem(
                              value: 'unhideAllRows',
                              child: Text('Unhide all rows'),
                            ),
                            PopupMenuItem(
                              value: 'unhideAllColumns',
                              child: Text('Unhide all columns'),
                            ),
                          ],
                        ],
                      ),
                  ],
                ),
              );
            },
          ),
        ),
        Container(
          height: formulaExpanded ? 132 : 39,
          color: const Color(0xffffffff),
          padding: const EdgeInsets.symmetric(horizontal: 15),
          child: Row(
            children: [
              SizedBox(
                width: 100,
                child: draftTextField(
                  formField: true,
                  key: ValueKey(selectedRange),
                  initialValue: selectedRange,
                  style: const TextStyle(fontSize: 12),
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    isDense: true,
                    contentPadding: EdgeInsets.all(7),
                  ),
                  onFieldSubmitted: goTo,
                ),
              ),
              const SizedBox(width: 12),
              const Text(
                'ƒx',
                style: TextStyle(
                  fontFamily: 'Georgia',
                  fontSize: 21,
                  color: Color(0xffa49b88),
                ),
              ),
              const SizedBox(width: 15),
              Expanded(
                // A draft history belongs to this cell and its committed value.
                // Reusing it after navigation can copy the previous cell's text
                // into the newly selected cell when the user presses Undo.
                child: KeyedSubtree(
                  key: ValueKey((widget.sheet['id'], selected, raw(selected))),
                  child: SheetEditingKeys(
                    intercept: _capturePendingEntry,
                    controller: formulaController,
                    focus: formulaFocus,
                    onCommit: (r, c) => finishEditing(r, c, formula: true),
                    onCancel: () => cancelEditing(formula: true),
                    child: draftTextField(
                      key: const Key('formula'),
                      onChanged: (_) => formulaDirty = true,
                      controller: formulaController,
                      focusNode: formulaFocus,
                      maxLines: null,
                      keyboardType: TextInputType.multiline,
                      textInputAction: TextInputAction.done,
                      style: const TextStyle(fontFamily: 'Menlo', fontSize: 12),
                      decoration: const InputDecoration(
                        border: InputBorder.none,
                        hintText: 'Value or formula',
                      ),
                      onSubmitted: (_) => finishEditing(1, 0, formula: true),
                    ),
                  ),
                ),
              ),
              TextFieldTapRegion(
                child: ExcludeFocus(
                  child: IconButton(
                    key: const Key('expand-formula'),
                    tooltip: formulaExpanded
                        ? 'Collapse formula bar'
                        : 'Expand formula bar',
                    onPressed: () =>
                        setState(() => formulaExpanded = !formulaExpanded),
                    icon: Icon(
                      formulaExpanded ? Icons.expand_less : Icons.expand_more,
                      size: 18,
                    ),
                  ),
                ),
              ),
              IconButton(
                tooltip: 'Chart from columns A and B',
                onPressed: () => setState(() => chart = !chart),
                icon: const Icon(Icons.bar_chart, size: 18),
              ),
              Builder(
                builder: (buttonContext) => TextButton(
                  key: const Key('cell-actions'),
                  onPressed: () {
                    final box = buttonContext.findRenderObject() as RenderBox;
                    cellActions(box.localToGlobal(Offset(0, box.size.height)));
                  },
                  child: const Text('Cell actions ▾'),
                ),
              ),
            ],
          ),
        ),
        if (extendSelectionMode)
          Container(
            key: const Key('extend-selection-status'),
            color: const Color(0xffedf4ef),
            padding: const EdgeInsets.only(left: 14, right: 8),
            child: Row(
              children: [
                const Expanded(
                  child: Text(
                    'Extend selection · click a cell or use arrow keys',
                    style: TextStyle(fontSize: 12),
                  ),
                ),
                TextButton(
                  key: const Key('finish-selection'),
                  onPressed: () => setExtendSelection(false),
                  child: const Text('Finish selection'),
                ),
              ],
            ),
          ),
        if (!validationHintDismissed &&
            validationAt(selected)?['showInputMessage'] == true &&
            ('${validationAt(selected)?['promptTitle'] ?? ''}${validationAt(selected)?['prompt'] ?? ''}')
                .isNotEmpty)
          Container(
            key: const Key('validation-input-message'),
            color: const Color(0xfffff8df),
            padding: const EdgeInsets.only(left: 14),
            child: Row(
              children: [
                const Icon(Icons.info_outline, size: 16),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    [
                      validationAt(selected)?['promptTitle'],
                      validationAt(selected)?['prompt'],
                    ].where((s) => s != null && s != '').join(' — '),
                    maxLines: 3,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontSize: 12),
                  ),
                ),
                IconButton(
                  tooltip: 'Dismiss input message',
                  icon: const Icon(Icons.close, size: 16),
                  onPressed: () =>
                      setState(() => validationHintDismissed = true),
                ),
              ],
            ),
          ),
        if (selectedTable case final Map table)
          Container(
            height: 30,
            color: const Color(0xffedf4ef),
            padding: const EdgeInsets.symmetric(horizontal: 15),
            child: Row(
              children: [
                Icon(Icons.table_chart, size: 15, color: tableColor(table)),
                const SizedBox(width: 8),
                Text(
                  '${table['name']}  ·  ${table['range']}',
                  style: const TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const Spacer(),
                if ((table['filters'] as Map? ?? {}).isNotEmpty)
                  Text(
                    visibleTableRows(table),
                    style: const TextStyle(fontSize: 11),
                  ),
                if ((table['filters'] as Map? ?? {}).isNotEmpty)
                  TextButton(
                    onPressed: () =>
                        tableAction({'action': 'clearFilters'}, table: table),
                    child: const Text(
                      'Clear filters',
                      style: TextStyle(fontSize: 11),
                    ),
                  ),
              ],
            ),
          ),
        if (worksheetFilter != null)
          Container(
            height: 30,
            color: const Color(0xffedf4ef),
            padding: const EdgeInsets.symmetric(horizontal: 15),
            child: Row(
              children: [
                const Icon(
                  Icons.filter_alt_outlined,
                  size: 15,
                  color: Color(0xff217346),
                ),
                const SizedBox(width: 8),
                Text(
                  'Filter · ${worksheetFilter['range']}',
                  style: const TextStyle(fontSize: 11),
                ),
                const Spacer(),
                Text(
                  visibleTableRows(worksheetFilter),
                  style: const TextStyle(fontSize: 11),
                ),
                if ((worksheetFilter['filters'] as Map).isNotEmpty)
                  TextButton(
                    onPressed: () =>
                        rangeFilterAction({'action': 'clearFilters'}),
                    child: const Text(
                      'Clear filters',
                      style: TextStyle(fontSize: 11),
                    ),
                  ),
              ],
            ),
          ),
        if (chart) _chart(),
        Expanded(
          child: Semantics(
            key: const Key('sheet-grid-focus'),
            container: true,
            label:
                '${widget.sheet['name']} worksheet, selection $selectedRange',
            child: Focus(
              focusNode: gridFocus,
              onKeyEvent: (_, e) => key(e),
              child: frozenGrid(
                () => SparseSheetGrid(
                  foreground: _paintOverflow,
                  key: const Key('sheet-grid'),
                  verticalDetails: ScrollableDetails.vertical(
                    controller: vertical,
                  ),
                  horizontalDetails: ScrollableDetails.horizontal(
                    controller: horizontal,
                  ),
                  rows: SheetGridAxis(rowLayout, 32, pinnedRows),
                  columns: SheetGridAxis(columnLayout, 43, pinnedColumns),
                  cellBuilder: (context, vicinity) {
                    final r = vicinity.yIndex == 0
                            ? -1
                            : visibleRows.toPhysical(vicinity.yIndex - 1),
                        c = vicinity.xIndex == 0
                            ? -1
                            : visibleColumns.toPhysical(vicinity.xIndex - 1);
                    final header = r < 0 || c < 0;
                    if (header) {
                      return DecoratedBox(
                        position: DecorationPosition.foreground,
                        decoration: const BoxDecoration(
                          border: Border(
                            right: BorderSide(color: line),
                            bottom: BorderSide(color: line),
                          ),
                        ),
                        child: Stack(
                          fit: StackFit.expand,
                          children: [
                            GestureDetector(
                              key: Key(
                                r < 0 ? 'column-header-$c' : 'row-header-$r',
                              ),
                              onTap: () => selectWhole(
                                rows: c < 0,
                                columns: r < 0,
                                atRow: r < 0 ? _firstUnfrozenRow : r,
                                atColumn: c < 0 ? _firstUnfrozenColumn : c,
                                extend:
                                    HardwareKeyboard.instance.isShiftPressed,
                              ),
                              onSecondaryTapDown:
                                  widget.onStructure == null || (r < 0 && c < 0)
                                  ? null
                                  : (details) => headerMenu(
                                      r,
                                      c,
                                      details.globalPosition,
                                    ),
                              child: ColoredBox(
                                color: const Color(0xfff1f3f4),
                                child: Center(
                                  child: FittedBox(
                                    fit: BoxFit.scaleDown,
                                    child: Text(
                                      r < 0
                                          ? (c < 0 ? '' : columnLabel(c))
                                          : '${r + 1}',
                                      maxLines: 1,
                                      softWrap: false,
                                      style: const TextStyle(
                                        fontSize: 10,
                                        color: Color(0xff9a9586),
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ),
                            if (widget.onGeometry != null && !(r < 0 && c < 0))
                              Positioned(
                                right: 0,
                                bottom: 0,
                                top: r < 0 ? 0 : null,
                                left: c < 0 ? 0 : null,
                                width: r < 0 ? 6 : null,
                                height: c < 0 ? 6 : null,
                                child: MouseRegion(
                                  cursor: r < 0
                                      ? SystemMouseCursors.resizeColumn
                                      : SystemMouseCursors.resizeRow,
                                  child: GestureDetector(
                                    dragStartBehavior: DragStartBehavior.down,
                                    key: Key(
                                      r < 0
                                          ? 'column-resize-$c'
                                          : 'row-resize-$r',
                                    ),
                                    behavior: HitTestBehavior.opaque,
                                    onDoubleTap: () {
                                      final area = TableArea.parse(
                                        selectedRange,
                                      );
                                      final rows = c < 0;
                                      final within = rows
                                          ? r >= area.top && r <= area.bottom
                                          : c >= area.left && c <= area.right;
                                      autoFitDimensions(
                                        rows,
                                        at: within ? null : (rows ? r : c) + 1,
                                        count: within ? null : 1,
                                      );
                                    },
                                    onHorizontalDragStart: r < 0
                                        ? (_) => beginResize(false, c)
                                        : null,
                                    onHorizontalDragUpdate: r < 0
                                        ? (d) => updateResize(d.delta.dx)
                                        : null,
                                    onHorizontalDragEnd: r < 0
                                        ? (_) => endResize()
                                        : null,
                                    onVerticalDragStart: c < 0
                                        ? (_) => beginResize(true, r)
                                        : null,
                                    onVerticalDragUpdate: c < 0
                                        ? (d) => updateResize(d.delta.dy)
                                        : null,
                                    onVerticalDragEnd: c < 0
                                        ? (_) => endResize()
                                        : null,
                                    onHorizontalDragCancel: r < 0
                                        ? () => setState(() {
                                            dragIndex = null;
                                            dragExtent = null;
                                          })
                                        : null,
                                    onVerticalDragCancel: c < 0
                                        ? () => setState(() {
                                            dragIndex = null;
                                            dragExtent = null;
                                          })
                                        : null,
                                  ),
                                ),
                              ),
                          ],
                        ),
                      );
                    }
                    final a = cellAddress(r, c), active = r == row && c == col;
                    final value = widget.calculated[a] ?? raw(a);
                    final style = numberDisplayStyle(
                      cellStyle(widget.sheet, a),
                      displayAt(a),
                    );
                    final table = tableAt(r, c);
                    final filterHeader =
                        filterArea != null &&
                        r == filterArea.top &&
                        c >= filterArea.left &&
                        c <= filterArea.right;
                    final area = table == null
                        ? null
                        : TableArea.parse(table['range']);
                    final tableHeader =
                        table != null &&
                        table['headerRows'] != 0 &&
                        r == area!.top;
                    final tableTotal =
                        table != null &&
                        table['totalsRows'] == 1 &&
                        r == area!.bottom;
                    final banded =
                        table != null &&
                        table['rowStripes'] != false &&
                        (r - area!.top) % 2 == 1;
                    final fill = tableHeader
                        ? tableColor(table)
                        : tableTotal
                        ? const Color(0xffe7edf3)
                        : banded
                        ? tableColor(table).withValues(alpha: 0.07)
                        : const Color(0xffffffff);
                    final inRange =
                        hasRangeSelection && selectionArea.contains(r, c);
                    return Semantics(
                      key: ValueKey('cell-semantics-$a'),
                      container: true,
                      label: '$a cell',
                      selected: active || inRange,
                      // Accessibility activation is an explicit single-cell
                      // action, independent of stale native modifier keys.
                      onTap: active && editing
                          ? null
                          : () =>
                                select(r, c, shiftPressed: extendSelectionMode),
                      customSemanticsActions: active && editing
                          ? null
                          : {
                              const CustomSemanticsAction(
                                label: 'Extend selection to this cell',
                              ): () =>
                                  select(r, c, shiftPressed: true),
                              const CustomSemanticsAction(
                                label: 'Edit cell',
                              ): () async {
                                if (await select(r, c, extend: false)) edit();
                              },
                            },
                      child: Listener(
                        key: Key('cell-$a'),
                        behavior: HitTestBehavior.opaque,
                        // Selection must precede the next keystroke, without
                        // waiting for the double-click recognizer's timeout.
                        onPointerDown: (event) {
                          final filterButton =
                              columnExtent(c) >= 42 &&
                              ((tableHeader && widget.onTable != null) ||
                                  (filterHeader &&
                                      widget.onAutoFilter != null));
                          if (filterButton &&
                              event.localPosition.dx >= columnExtent(c) - 33) {
                            // Selecting a table can insert its toolbar and move
                            // the filter button before the pointer is released.
                            return;
                          }
                          if (event.buttons == kPrimaryMouseButton &&
                              !(active && editing)) {
                            select(r, c);
                          }
                        },
                        child: GestureDetector(
                          excludeFromSemantics: true,
                          onSecondaryTapDown:
                              widget.onReview == null || active && editing
                              ? null
                              : (details) =>
                                    cellMenu(r, c, details.globalPosition),
                          onDoubleTap: () async {
                            if (active && editing) return;
                            await select(r, c);
                            edit();
                          },
                          child: Container(
                            decoration: BoxDecoration(
                              color: active || inRange
                                  ? Color.alphaBlend(
                                      const Color(0x18107c41),
                                      color(style['fill']) ?? fill,
                                    )
                                  : color(style['fill']) ?? fill,
                              border: active
                                  ? Border.all(
                                      color: const Color(0xff107c41),
                                      width: 1.6,
                                    )
                                  : const Border(
                                      right: BorderSide(color: line),
                                      bottom: BorderSide(color: line),
                                    ),
                            ),
                            alignment: Alignment.centerLeft,
                            child: SheetReviewMarker(
                              key: Key('review-marker-$a'),
                              review: cellReview(widget.sheet, a),
                              child: SheetDataBar(
                                key: Key('data-bar-$a'),
                                bar: active && editing
                                    ? null
                                    : style['_dataBar'] as Map?,
                                child: Padding(
                                  padding: EdgeInsets.symmetric(
                                    horizontal: columnExtent(c) < 42 ? 0 : 9,
                                    vertical: 2,
                                  ),
                                  child: active && editing
                                      ? SheetEditingKeys(
                                          intercept: _capturePendingEntry,
                                          controller: cellController,
                                          focus: cellFocus,
                                          onCommit: (r, c) =>
                                              finishEditing(r, c),
                                          onCancel: cancelEditing,
                                          child: draftTextField(
                                            key: const Key('cell-editor'),
                                            controller: cellController,
                                            focusNode: cellFocus,
                                            maxLines: null,
                                            keyboardType:
                                                TextInputType.multiline,
                                            textInputAction:
                                                TextInputAction.done,
                                            maxLength: 10000,
                                            style: sheetTextStyle(style),
                                            decoration: const InputDecoration(
                                              isDense: true,
                                              border: InputBorder.none,
                                              counterText: '',
                                              contentPadding: EdgeInsets.zero,
                                            ),
                                            onSubmitted: (_) =>
                                                finishEditing(1, 0),
                                          ),
                                        )
                                      : Row(
                                          crossAxisAlignment:
                                              CrossAxisAlignment.stretch,
                                          children: [
                                            Expanded(
                                              child: Tooltip(
                                                message:
                                                    [
                                                          if (cellReview(
                                                                widget.sheet,
                                                                a,
                                                              ) !=
                                                              null)
                                                            reviewTooltip(
                                                              cellReview(
                                                                widget.sheet,
                                                                a,
                                                              ),
                                                            ),
                                                          widget.formattedCells[a]?['warning']
                                                                  as String? ??
                                                              formatted(a),
                                                        ]
                                                        .where(
                                                          (text) =>
                                                              text.isNotEmpty,
                                                        )
                                                        .join('\n\n'),
                                                child: SheetIconValue(
                                                  address: a,
                                                  style: style,
                                                  child: SheetFontText(
                                                    formatted(a),
                                                    key: Key('sheet-font-$a'),
                                                    font: style,
                                                    layout: CellTextLayout(
                                                      style,
                                                      value: value,
                                                      normalFont:
                                                          normalSheetFont(
                                                            widget.sheet,
                                                          ),
                                                    ),
                                                    bold:
                                                        (style['bold']
                                                            as bool?) ??
                                                        (tableHeader ||
                                                            tableTotal),
                                                    color:
                                                        color(style['color']) ??
                                                        (tableHeader
                                                            ? Colors.white
                                                            : const Color(
                                                                0xff242424,
                                                              )),
                                                  ),
                                                ),
                                              ),
                                            ),
                                            if (active &&
                                                hasValidationDropdown &&
                                                columnExtent(c) >= 42)
                                              SizedBox(
                                                width: 24,
                                                child: IconButton(
                                                  key: const Key(
                                                    'validation-dropdown',
                                                  ),
                                                  tooltip:
                                                      'Choose a valid value (Alt+↓)',
                                                  padding: EdgeInsets.zero,
                                                  icon: const Icon(
                                                    Icons.arrow_drop_down,
                                                    size: 18,
                                                  ),
                                                  onPressed:
                                                      chooseValidationValue,
                                                ),
                                              ),
                                            if (filterHeader &&
                                                widget.onAutoFilter != null &&
                                                columnExtent(c) >= 42)
                                              SizedBox(
                                                width: 24,
                                                height: 28,
                                                child: IconButton(
                                                  key: Key(
                                                    'range-filter-${c - filterArea.left}',
                                                  ),
                                                  tooltip:
                                                      'Filter ${worksheetFilter!['columns'][c - filterArea.left]['name']}',
                                                  padding: EdgeInsets.zero,
                                                  icon: Icon(
                                                    (worksheetFilter['filters']
                                                                as Map)
                                                            .containsKey(
                                                              '${c - filterArea.left}',
                                                            )
                                                        ? Icons.filter_alt
                                                        : Icons.arrow_drop_down,
                                                    size: 17,
                                                    color: const Color(
                                                      0xff217346,
                                                    ),
                                                  ),
                                                  onPressed: () => filterTable(
                                                    worksheetFilter,
                                                    c - filterArea.left,
                                                  ),
                                                ),
                                              ),
                                            if (tableHeader &&
                                                widget.onTable != null &&
                                                columnExtent(c) >= 42)
                                              SizedBox(
                                                width: 24,
                                                height: 28,
                                                child: IconButton(
                                                  key: Key(
                                                    'table-filter-${table['id']}-${c - area.left}',
                                                  ),
                                                  tooltip:
                                                      'Filter ${table['columns'][c - area.left]['name']}',
                                                  padding: EdgeInsets.zero,
                                                  icon: Icon(
                                                    (table['filters'] as Map? ??
                                                                {})
                                                            .containsKey(
                                                              '${c - area.left}',
                                                            )
                                                        ? Icons.filter_alt
                                                        : Icons.arrow_drop_down,
                                                    size: 17,
                                                    color: active || inRange
                                                        ? tableColor(table)
                                                        : Colors.white,
                                                  ),
                                                  onPressed: () => filterTable(
                                                    table,
                                                    c - area.left,
                                                  ),
                                                ),
                                              ),
                                          ],
                                        ),
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                    );
                  },
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }

  Widget _chart() {
    final items = <Map<String, dynamic>>[];
    for (var r = 0; r < 10000 && items.length < 10; r++) {
      final label = formatted('A${r + 1}'),
          value = widget.calculated['B${r + 1}'];
      if (label.isNotEmpty && value is num) {
        items.add({
          'label': label,
          'value': value,
          'display': formatted('B${r + 1}'),
        });
      }
    }
    final max = items.fold<double>(
      1,
      (a, b) => a > (b['value'] as num).abs()
          ? a
          : (b['value'] as num).abs().toDouble(),
    );
    return Container(
      height: 200,
      padding: const EdgeInsets.all(20),
      child: items.isEmpty
          ? const Center(
              child: Text('Add labels in A and numbers in B to see a chart.'),
            )
          : ListView(
              children: [
                const Text(
                  'COLUMN B OVERVIEW',
                  style: TextStyle(fontSize: 10, letterSpacing: 1.4),
                ),
                const SizedBox(height: 10),
                ...items.map(
                  (p) => Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    child: Row(
                      children: [
                        SizedBox(
                          width: 110,
                          child: Text(
                            p['label'],
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(fontSize: 11),
                          ),
                        ),
                        Expanded(
                          child: FractionallySizedBox(
                            alignment: Alignment.centerLeft,
                            widthFactor: (p['value'] as num).abs() / max,
                            child: Container(
                              height: 12,
                              color: const Color(0xffa5b18f),
                            ),
                          ),
                        ),
                        const SizedBox(width: 12),
                        SizedBox(
                          width: 70,
                          child: Text(
                            p['display'],
                            textAlign: TextAlign.right,
                            style: const TextStyle(fontSize: 11),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
    );
  }

  Future<void>? _flushTask;
  Future<void> flush() =>
      _flushTask ??= _flushEntries().whenComplete(() => _flushTask = null);

  Future<void> _flushEntries() async {
    while (mounted) {
      if (!await _waitForEntrySequence() ||
          !await commit(keepEditing: true) ||
          !await commitFormula()) {
        throw StateError(
          _entryDetached
              ? 'The worksheet changed. Copy or discard the waiting typing before continuing.'
              : 'The cell edit could not be saved. Correct it or press Escape before continuing.',
        );
      }
      // A close request can arrive between the hardware key and its platform
      // text update. Keep the text client alive through the save and next frame;
      // if more text arrived, persist that draft too before allowing close.
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted) return;
      if (_finishingEdit ||
          _entryDrain != null ||
          _entryKeys.isNotEmpty ||
          formulaDirty ||
          (editing && cellController.text != raw(selected))) {
        continue;
      }
      if (editing) setState(() => editing = false);
      return;
    }
  }

  @override
  void dispose() {
    _clearOverflowPainters();
    gridFocus.removeListener(_gridFocusChanged);
    _syncDesktopGridFocus(false);
    cellFocus.removeListener(_blurCell);
    formulaFocus.removeListener(_blurFormula);
    cellController.dispose();
    formulaController.dispose();
    cellFocus.dispose();
    formulaFocus.dispose();
    gridFocus.dispose();
    vertical.dispose();
    horizontal.dispose();
    super.dispose();
  }
}
