import 'dart:math' as math;
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'table_controls.dart' show SheetAxisLayout;

class SheetGridPaintCell {
  final int row, column;
  final Rect bounds;
  const SheetGridPaintCell(this.row, this.column, this.bounds);
}

typedef SheetGridForeground =
    void Function(
      PaintingContext context,
      Offset offset,
      List<SheetGridPaintCell> cells,
    );

/// One heading followed by the visible worksheet positions. Only explicit
/// dimensions and hidden positions are stored by [SheetAxisLayout].
class SheetGridAxis {
  final SheetAxisLayout layout;
  final double heading;
  final int pinnedCount;
  SheetGridAxis(this.layout, this.heading, this.pinnedCount)
    : assert(pinnedCount >= 1),
      assert(
        pinnedCount <= 1 + layout.visible.limit - layout.visible.hidden.length,
      );

  int get count => 1 + layout.visible.limit - layout.visible.hidden.length;
  double extent(int index) => index == 0
      ? heading
      : layout.extent(layout.visible.toPhysical(index - 1));
  double offset(int index) => index == 0
      ? 0
      : heading +
            layout.offset(
              index == count
                  ? layout.visible.limit
                  : layout.visible.toPhysical(index - 1),
            );
  double get pinnedExtent => offset(pinnedCount);
  double get totalExtent => offset(count);

  /// Binary search also works at the last Excel row without traversing the
  /// intervening rows. At an exact boundary, return the following position.
  int indexAt(double position) {
    var low = 0, high = count - 1;
    while (low < high) {
      final mid = (low + high) ~/ 2;
      if (offset(mid + 1) <= position) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }
    return low;
  }

  List<({int index, double start, double extent})> visibleSpans(
    double scroll,
    double viewport,
  ) {
    final result = <({int index, double start, double extent})>[];
    void add(int index, double shift) => result.add((
      index: index,
      start: offset(index) - shift,
      extent: extent(index),
    ));
    for (var i = 0; i < pinnedCount; i++) {
      add(i, 0);
    }
    // A small pixel cache supports text-field reveal and smooth scrolling,
    // without storing per-row layout records for the rest of the worksheet.
    const cache = 120.0;
    final first = math.max(pinnedCount, indexAt(scroll + pinnedExtent - cache));
    for (
      var i = first;
      i < count && offset(i) < scroll + viewport + cache;
      i++
    ) {
      add(i, scroll);
    }
    return result;
  }
}

class SparseSheetGrid extends TwoDimensionalScrollView {
  final SheetGridAxis rows, columns;
  final SheetGridForeground? foreground;
  SparseSheetGrid({
    super.key,
    required this.rows,
    required this.columns,
    this.foreground,
    required TwoDimensionalIndexedWidgetBuilder cellBuilder,
    required super.verticalDetails,
    required super.horizontalDetails,
  }) : super(
         delegate: TwoDimensionalChildBuilderDelegate(
           maxXIndex: columns.count - 1,
           maxYIndex: rows.count - 1,
           builder: cellBuilder,
         ),
       );

  @override
  Widget buildViewport(
    BuildContext context,
    ViewportOffset verticalOffset,
    ViewportOffset horizontalOffset,
  ) => _SheetViewport(
    rows: rows,
    columns: columns,
    verticalOffset: verticalOffset,
    horizontalOffset: horizontalOffset,
    delegate: delegate,
    foreground: foreground,
  );
}

class _SheetViewport extends TwoDimensionalViewport {
  final SheetGridAxis rows, columns;
  final SheetGridForeground? foreground;
  const _SheetViewport({
    required this.rows,
    required this.columns,
    this.foreground,
    required super.verticalOffset,
    required super.horizontalOffset,
    required super.delegate,
  }) : super(
         mainAxis: Axis.vertical,
         verticalAxisDirection: AxisDirection.down,
         horizontalAxisDirection: AxisDirection.right,
       );

  @override
  RenderSparseSheetViewport createRenderObject(BuildContext context) =>
      RenderSparseSheetViewport(
        rows,
        columns,
        verticalOffset: verticalOffset,
        horizontalOffset: horizontalOffset,
        delegate: delegate,
        childManager: context as TwoDimensionalChildManager,
        foreground: foreground,
      );

  @override
  void updateRenderObject(
    BuildContext context,
    RenderSparseSheetViewport object,
  ) {
    object
      ..verticalOffset = verticalOffset
      ..horizontalOffset = horizontalOffset
      ..delegate = delegate
      ..foreground = foreground
      ..geometry = (rows, columns);
  }
}

class RenderSparseSheetViewport extends RenderTwoDimensionalViewport {
  SheetGridForeground? _foreground;
  set foreground(SheetGridForeground? value) {
    if (identical(_foreground, value)) return;
    _foreground = value;
    markNeedsPaint();
  }

  SheetGridAxis _rows, _columns;
  double _pinWidth = 0, _pinHeight = 0;
  // regular cells, frozen columns, frozen rows, frozen corner
  final _groups = List.generate(4, (_) => <RenderBox>[]);
  final _clips = List.generate(4, (_) => LayerHandle<ClipRectLayer>());
  RenderSparseSheetViewport(
    this._rows,
    this._columns, {
    required super.verticalOffset,
    required super.horizontalOffset,
    required super.delegate,
    required super.childManager,
    SheetGridForeground? foreground,
    // Public constructor name differs from the private cached callback.
    // ignore: prefer_initializing_formals
  }) : _foreground = foreground,
       super(
         mainAxis: Axis.vertical,
         verticalAxisDirection: AxisDirection.down,
         horizontalAxisDirection: AxisDirection.right,
       );

  set geometry((SheetGridAxis, SheetGridAxis) axes) {
    if (identical(_rows, axes.$1) && identical(_columns, axes.$2)) return;
    _rows = axes.$1;
    _columns = axes.$2;
    markNeedsLayout();
  }

  @override
  void layoutChildSequence() {
    _pinWidth = _columns.pinnedExtent.clamp(0.0, size.width);
    _pinHeight = _rows.pinnedExtent.clamp(0.0, size.height);
    final maxX = math.max(0.0, _columns.totalExtent - size.width);
    final maxY = math.max(0.0, _rows.totalExtent - size.height);
    // Scroll positions may correct themselves when hiding/resizing dimensions
    // or enlarging the window. Use the corrected positions for this layout.
    horizontalOffset.applyContentDimensions(0, maxX);
    verticalOffset.applyContentDimensions(0, maxY);
    final rows = _rows.visibleSpans(verticalOffset.pixels, size.height);
    final columns = _columns.visibleSpans(horizontalOffset.pixels, size.width);
    for (final group in _groups) {
      group.clear();
    }
    for (final row in rows) {
      for (final column in columns) {
        final child = buildOrObtainChildFor(
          ChildVicinity(xIndex: column.index, yIndex: row.index),
        );
        if (child == null) continue;
        child.layout(
          BoxConstraints.tight(Size(column.extent, row.extent)),
          parentUsesSize: true,
        );
        parentDataOf(child).layoutOffset = Offset(column.start, row.start);
        _groups[_group(child)].add(child);
      }
    }
  }

  int _group(RenderBox child) {
    final vicinity = parentDataOf(child).vicinity;
    return (vicinity.xIndex < _columns.pinnedCount ? 1 : 0) +
        (vicinity.yIndex < _rows.pinnedCount ? 2 : 0);
  }

  Rect _clip(int group) {
    return Rect.fromLTRB(
      group.isOdd ? 0 : _pinWidth,
      group >= 2 ? 0 : _pinHeight,
      group.isOdd ? _pinWidth : size.width,
      group >= 2 ? _pinHeight : size.height,
    );
  }

  bool _visible(RenderBox child, Rect clip) =>
      clip.overlaps(parentDataOf(child).paintOffset! & child.size);

  @override
  void paint(PaintingContext context, Offset offset) {
    context.canvas.drawRect(
      offset & size,
      Paint()..color = const Color(0xffffffff),
    );
    for (var group = 0; group < 4; group++) {
      final clip = _clip(group);
      _clips[group].layer = context.pushClipRect(
        needsCompositing,
        offset,
        clip,
        (context, offset) {
          for (final child in _groups[group]) {
            if (_visible(child, clip)) {
              context.paintChild(
                child,
                offset + parentDataOf(child).paintOffset!,
              );
            }
          }
          _foreground?.call(context, offset, [
            for (final child in _groups[group])
              if (_visible(child, clip))
                SheetGridPaintCell(
                  parentDataOf(child).vicinity.yIndex == 0
                      ? -1
                      : _rows.layout.visible.toPhysical(
                          parentDataOf(child).vicinity.yIndex - 1,
                        ),
                  parentDataOf(child).vicinity.xIndex == 0
                      ? -1
                      : _columns.layout.visible.toPhysical(
                          parentDataOf(child).vicinity.xIndex - 1,
                        ),
                  parentDataOf(child).paintOffset! & child.size,
                ),
          ]);
        },
        oldLayer: _clips[group].layer,
      );
    }
    final line = Paint()
      ..color = const Color(0xff74818b)
      ..strokeWidth = 1.5;
    if (_columns.pinnedCount > 1 && _pinWidth < size.width) {
      final x = offset.dx + _pinWidth;
      context.canvas.drawLine(
        Offset(x, offset.dy),
        Offset(x, offset.dy + size.height),
        line,
      );
    }
    if (_rows.pinnedCount > 1 && _pinHeight < size.height) {
      final y = offset.dy + _pinHeight;
      context.canvas.drawLine(
        Offset(offset.dx, y),
        Offset(offset.dx + size.width, y),
        line,
      );
    }
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) {
    for (var group = 0; group < 4; group++) {
      if (!_clip(group).contains(position)) continue;
      for (final child in _groups[group]) {
        if (result.addWithPaintOffset(
          offset: parentDataOf(child).paintOffset,
          position: position,
          hitTest: (result, transformed) =>
              child.hitTest(result, position: transformed),
        )) {
          return true;
        }
      }
    }
    return false;
  }

  @override
  void visitChildrenForSemantics(RenderObjectVisitor visitor) {
    var child = firstChild;
    while (child != null) {
      if (_visible(child, _clip(_group(child)))) visitor(child);
      child = childAfter(child);
    }
  }

  @override
  Rect describeApproximatePaintClip(RenderObject child) =>
      _clip(_group(child as RenderBox));

  @override
  Rect describeSemanticsClip(RenderObject? child) =>
      child == null ? Offset.zero & size : _clip(_group(child as RenderBox));

  @override
  RevealedOffset getOffsetToReveal(
    RenderObject target,
    double alignment, {
    Rect? rect,
    Axis? axis,
  }) {
    axis ??= mainAxis;
    var child = target;
    while (child.parent != this) {
      child = child.parent!;
    }
    final vicinity = parentDataOf(child as RenderBox).vicinity;
    final vertical = axis == Axis.vertical;
    final geometry = vertical ? _rows : _columns;
    final index = vertical ? vicinity.yIndex : vicinity.xIndex;
    final current = vertical ? verticalOffset.pixels : horizontalOffset.pixels;
    final bounds = MatrixUtils.transformRect(
      target.getTransformTo(this),
      rect ?? target.paintBounds,
    );
    if (index < geometry.pinnedCount) {
      return RevealedOffset(offset: current, rect: bounds);
    }
    final viewport = vertical ? size.height : size.width;
    final leading = vertical ? bounds.top : bounds.left;
    final extent = vertical ? bounds.height : bounds.width;
    final available = viewport - geometry.pinnedExtent;
    final next =
        (current +
                leading -
                geometry.pinnedExtent -
                (available - extent) * alignment)
            .clamp(0.0, math.max(0.0, geometry.totalExtent - viewport))
            .toDouble();
    return RevealedOffset(
      offset: next,
      rect: bounds.translate(
        vertical ? 0 : current - next,
        vertical ? current - next : 0,
      ),
    );
  }

  @override
  void dispose() {
    for (final clip in _clips) {
      clip.layer = null;
    }
    super.dispose();
  }
}
