// Table drag-resize (W07.06): drag a column border to resize the columns on
// either side, or a row's bottom border to set its height.
import 'dart:convert';
import 'dart:math' as math;
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

/// The grid's column and row edges from its last layout, in pixels.
class TableGridGeometry {
  List<double> xs = const [], ys = const [];

  /// Each cell's first row and column and its spans.
  List<({int row, int col, int rows, int cols})> cells = const [];
}

/// The narrowest column and shortest row a drag leaves, in twips.
const minColumnTwips = 360, minRowTwips = 60;

/// The border at [p]: a column border between two columns (the index of the
/// column on its left) or a row's bottom border.
({bool column, int index})? tableBorderAt(
  TableGridGeometry g,
  Offset p, {
  double slop = 4,
}) {
  if (g.xs.length < 2 || g.ys.length < 2) return null;
  if (p.dy < -slop || p.dy > g.ys.last + slop) return null;
  int at(List<double> edges, double v) {
    var i = 0;
    while (i < edges.length - 2 && edges[i + 1] <= v) {
      i++;
    }
    return i;
  }

  final row = at(g.ys, p.dy), column = at(g.xs, p.dx);
  // A border is only where a cell has an edge: not inside a merged cell.
  for (var i = 1; i < g.xs.length - 1; i++) {
    if ((p.dx - g.xs[i]).abs() <= slop &&
        g.cells.any(
          (c) => c.col == i && c.row <= row && row < c.row + c.rows,
        )) {
      return (column: true, index: i - 1);
    }
  }
  if (p.dx < 0 || p.dx > g.xs.last) return null;
  for (var i = 1; i < g.ys.length; i++) {
    if ((p.dy - g.ys[i]).abs() <= slop &&
        g.cells.any(
          (c) =>
              c.row + c.rows == i && c.col <= column && column < c.col + c.cols,
        )) {
      return (column: false, index: i - 1);
    }
  }
  return null;
}

Map<String, dynamic> _copy(Map node) =>
    jsonDecode(jsonEncode(node)) as Map<String, dynamic>;

/// [node] with the border right of [column] moved by [dx] pixels, when the
/// table is [width] pixels wide. The columns on either side share the change.
Map<String, dynamic> resizeTableColumns(
  Map node,
  List<num> widths,
  int column,
  double dx,
  double width,
) {
  final twips = [for (final w in widths) w.toDouble()];
  final total = twips.fold<double>(0, (a, b) => a + b);
  var delta = dx * total / width;
  delta = math.max(delta, minColumnTwips - twips[column]);
  delta = math.min(delta, twips[column + 1] - minColumnTwips);
  twips[column] += delta;
  twips[column + 1] -= delta;
  final next = _copy(node);
  next['attrs'] = {
    ...?next['attrs'] as Map?,
    'columnWidths': [for (final w in twips) w.round()],
  };
  return next;
}

/// [node] with [row] at least [height] pixels tall (1px is 15 twips). An
/// exact height stays exact.
Map<String, dynamic> resizeTableRow(Map node, int row, double height) {
  final next = _copy(node);
  final r = next['content'][row] as Map;
  final old = r['attrs']?['height'] as Map?;
  r['attrs'] = {
    ...?r['attrs'] as Map?,
    'height': {
      'rule': old?['rule'] == 'exact' ? 'exact' : 'atLeast',
      'value': math.max(minRowTwips, (height * 15).round()),
    },
  };
  return next;
}

class _BorderPan extends PanGestureRecognizer {
  _BorderPan() : super(debugOwner: null);
  @override
  void addAllowedPointer(PointerDownEvent event) {
    super.addAllowedPointer(event);
    // The editor's own selection drag must not take a border drag.
    resolve(GestureDisposition.accepted);
  }
}

/// Over a table grid: claims pointers only near its borders, shows the
/// resize cursor there, and reports a finished drag.
class TableResizeLayer extends StatefulWidget {
  final TableGridGeometry geometry;
  final void Function(int column, double dx) onColumn;
  final void Function(int row, double height) onRow;
  const TableResizeLayer({
    super.key,
    required this.geometry,
    required this.onColumn,
    required this.onRow,
  });
  @override
  State<TableResizeLayer> createState() => _TableResizeLayerState();
}

class _TableResizeLayerState extends State<TableResizeLayer> {
  ({bool column, int index})? hover, dragging;
  double delta = 0;

  @override
  Widget build(BuildContext context) {
    final g = widget.geometry;
    final active = dragging ?? hover;
    return MouseRegion(
      // Only the borders take the pointer; the cells keep their clicks.
      opaque: false,
      hitTestBehavior: HitTestBehavior.deferToChild,
      cursor: active == null
          ? MouseCursor.defer
          : active.column
          ? SystemMouseCursors.resizeColumn
          : SystemMouseCursors.resizeRow,
      onHover: (e) {
        final border = tableBorderAt(g, e.localPosition);
        if (border != hover) setState(() => hover = border);
      },
      onExit: (_) => setState(() => hover = null),
      child: RawGestureDetector(
        behavior: HitTestBehavior.deferToChild,
        gestures: {
          _BorderPan: GestureRecognizerFactoryWithHandlers<_BorderPan>(
            _BorderPan.new,
            (pan) => pan
              ..onStart = (e) {
                setState(() {
                  dragging = tableBorderAt(g, e.localPosition);
                  delta = 0;
                });
              }
              ..onUpdate = (e) {
                setState(
                  () => delta += dragging!.column ? e.delta.dx : e.delta.dy,
                );
              }
              ..onEnd = (_) {
                final border = dragging;
                setState(() => dragging = null);
                if (border == null || delta == 0) return;
                if (border.column) {
                  widget.onColumn(border.index, delta);
                } else {
                  final top = g.ys[border.index],
                      bottom = g.ys[border.index + 1];
                  widget.onRow(border.index, bottom - top + delta);
                }
              }
              ..onCancel = () {
                setState(() => dragging = null);
              },
          ),
        },
        child: _BorderHits(
          geometry: g,
          guide: dragging == null
              ? null
              : (column: dragging!.column, at: _guideAt(g, dragging!)),
        ),
      ),
    );
  }

  double _guideAt(TableGridGeometry g, ({bool column, int index}) border) =>
      (border.column ? g.xs[border.index + 1] : g.ys[border.index + 1]) + delta;
}

/// Hit only near a border; paints the guide line while dragging.
class _BorderHits extends LeafRenderObjectWidget {
  final TableGridGeometry geometry;
  final ({bool column, double at})? guide;
  const _BorderHits({required this.geometry, this.guide});
  @override
  _RenderBorderHits createRenderObject(BuildContext context) =>
      _RenderBorderHits(geometry, guide);
  @override
  void updateRenderObject(BuildContext context, _RenderBorderHits r) {
    r
      ..geometry = geometry
      ..guide = guide
      ..markNeedsPaint();
  }
}

class _RenderBorderHits extends RenderBox {
  _RenderBorderHits(this.geometry, this.guide);
  TableGridGeometry geometry;
  ({bool column, double at})? guide;
  @override
  bool get sizedByParent => true;
  @override
  Size computeDryLayout(BoxConstraints constraints) => constraints.biggest;
  @override
  bool hitTestSelf(Offset position) =>
      tableBorderAt(geometry, position) != null;
  @override
  void paint(PaintingContext context, Offset offset) {
    final g = guide;
    if (g == null) return;
    final paint = Paint()
      ..color = const Color(0xff2f6fd6)
      ..strokeWidth = 1;
    context.canvas.drawLine(
      offset + (g.column ? Offset(g.at, 0) : Offset(0, g.at)),
      offset +
          (g.column ? Offset(g.at, size.height) : Offset(size.width, g.at)),
      paint,
    );
  }
}
