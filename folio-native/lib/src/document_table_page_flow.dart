part of 'document_page_layout.dart';

/// Continuations store source positions, not a shared vertical cut through all
/// cells. Each paragraph is reshaped at the destination column's width.
class _CellState {
  final int block, textStart;
  final _TableState? table;
  final double clipped;
  const _CellState({
    this.block = 0,
    this.textStart = 0,
    this.table,
    this.clipped = 0,
  });
}

class _TableState {
  final int row;
  final Map<int, _CellState> cells;
  final double? minimumLeft;
  final double clipped;
  const _TableState({
    this.row = 0,
    this.cells = const {},
    this.minimumLeft,
    this.clipped = 0,
  });
  bool get started => cells.isNotEmpty || minimumLeft != null || clipped > 0;
}

class _TableGeometry {
  final DocumentTable grid;
  final List<double> xs, ys;
  final List<_MeasuredBlock> bodies;
  _TableGeometry(this.grid, this.xs, this.ys, this.bodies);
  int groupEnd(int row, int headers) {
    var end = row < headers ? headers : row + 1;
    for (;;) {
      final next = grid.cells
          .where((c) => c.row < end && c.row + c.rows > end)
          .fold(end, (a, c) => math.max(a, c.row + c.rows));
      if (next == end) return end;
      end = next;
    }
  }
}

class _TableFragment {
  final _MeasuredBlock block;
  final _TableState next;
  _TableFragment(this.block, this.next);
}

class _CellFragment {
  final _MeasuredBlock block;
  final _CellState next;
  _CellFragment(this.block, this.next);
}

/// Assemble clipped measured pieces without changing their source painters or
/// page-field identities. The same operation also handles nested table clips.
class _FragmentBuilder {
  final double width;
  double height = 0;
  final text = <_TextPiece>[];
  final cells = <_CellPiece>[];
  _FragmentBuilder(this.width);
  void append(_MeasuredBlock block, double from, double to) {
    final delta = Offset(0, height - from);
    final clip = Rect.fromLTWH(0, height, width, to - from);
    text.addAll(
      block.text
          .where((p) {
            var bounds = p.offset & p.painter.size;
            if (p.clip != null) bounds = bounds.intersect(p.clip!);
            return !bounds.isEmpty && bounds.shift(delta).overlaps(clip);
          })
          .map((p) => p.shifted(delta, within: clip)),
    );
    cells.addAll(
      block.cells
          .where((p) {
            final bounds = p.clip == null ? p.rect : p.rect.intersect(p.clip!);
            return !bounds.isEmpty && bounds.shift(delta).overlaps(clip);
          })
          .map((p) => p.shifted(delta, within: clip)),
    );
    height += to - from;
  }

  _MeasuredBlock build() =>
      _MeasuredBlock(width, height, [height], text, cells: cells);
}

extension _TablePageFlow on _PageFlow {
  _CellFragment _fragmentCell(
    List nodes,
    double width,
    double available,
    double capacity,
    _CellState start,
    String key,
    int si,
    int depth, {
    bool force = false,
  }) {
    final out = _FragmentBuilder(width);
    var state = start;
    while (state.block < nodes.length && out.height < available - .01) {
      final source = nodes[state.block] as Map;
      final remaining = available - out.height;
      final measured = measure(
        source,
        width,
        '$key:${state.block}',
        si,
        pages.last.number,
        depth: depth,
        textStart: state.textStart,
        previous: state.block == 0 ? null : nodes[state.block - 1] as Map,
      );
      if (source['type'] == 'table') {
        final f = _fragmentTable(
          measured,
          remaining,
          capacity,
          state.table ?? const _TableState(),
          '$key:${state.block}',
          si,
          depth,
          force: force && out.height == 0,
        );
        if (f.block.height <= .01) break;
        out.append(f.block, 0, f.block.height);
        if (f.next.row == (source['content'] as List).length) {
          state = _CellState(block: state.block + 1);
          continue;
        }
        state = _CellState(block: state.block, table: f.next);
        break;
      }
      final from = state.clipped;
      if (measured.height - from <= remaining + .01) {
        out.append(measured, from, measured.height);
        state = _CellState(block: state.block + 1);
        continue;
      }
      final keep =
          measured.properties['keepLines'] == true &&
          state.textStart == 0 &&
          from == 0 &&
          measured.height <= capacity;
      var breaks = measured.breaks
          .where((b) => b > from + .01 && b - from <= remaining + .01)
          .toList();
      if (keep && !force) breaks = [];
      if (measured.properties['widowControl'] != false &&
          !force &&
          breaks.isNotEmpty) {
        final left = measured.breaks.where((b) => b > breaks.last + .01).length;
        if (left == 1 && breaks.length > 1) breaks.removeLast();
        if (state.textStart == 0 &&
            breaks.length < 2 &&
            measured.breaks.length > 1) {
          breaks = [];
        }
      }
      if (breaks.isEmpty) {
        if (!force || out.height > 0) break;
        // Even a single glyph can be taller than the printable page. Keep an
        // explicit geometric fallback and a visible limitation for that case.
        final to = math.min(measured.height, from + remaining);
        out.append(measured, from, to);
        state = _CellState(
          block: state.block,
          textStart: state.textStart,
          clipped: to,
        );
        notices.add(
          'A line taller than the available table cell area is clipped across pages.',
        );
      } else {
        final to = breaks.last;
        out.append(measured, from, to);
        state = _CellState(
          block: state.block,
          textStart: measured.consumedText(to),
        );
      }
      break;
    }
    return _CellFragment(out.build(), state);
  }

  _TableFragment _fragmentTable(
    _MeasuredBlock measured,
    double available,
    double capacity,
    _TableState start,
    String key,
    int si,
    int depth, {
    bool force = false,
  }) {
    final g = measured.tableGeometry!;
    final out = _FragmentBuilder(measured.width);
    var state = start;
    while (state.row < g.grid.rows.length && out.height < available - .01) {
      final row = state.row;
      final end = g.groupEnd(row, measured.headerRows);
      final wholeHeight = g.ys[end] - g.ys[row];
      final remaining = available - out.height;
      if (!state.started && wholeHeight <= remaining + .01) {
        out.append(measured, g.ys[row], g.ys[end]);
        state = _TableState(row: end);
        continue;
      }
      final rowNode = g.grid.rows[row] as Map;
      final atomic =
          end != row + 1 || rowNode['attrs']?['height']?['rule'] == 'exact';
      final keep = atomic || !tableRowCanSplit(rowNode);
      if (!state.started &&
          keep &&
          (out.height > 0 || available < capacity - .01)) {
        break;
      }
      if (atomic) {
        // Exact-height clipping and vertical spans retain their established
        // geometry. Oversized merged groups need a later joint row-span solver.
        if (!force && wholeHeight <= capacity + .01) break;
        final from = g.ys[row] + state.clipped;
        final to = math.min(g.ys[end], from + remaining);
        out.append(measured, from, to);
        state = to >= g.ys[end] - .01
            ? _TableState(row: end)
            : _TableState(row: row, clipped: to - g.ys[row]);
        if (end != row + 1) {
          notices.add(
            'An oversized merged table row group crosses a page boundary; its cell text needs further continuation layout.',
          );
        }
        break;
      }
      if (remaining <= 16.01) break;
      final entries = <(TablePosition, _CellFragment)>[];
      var used = 0.0, advanced = false, done = true;
      final nextCells = <int, _CellState>{};
      for (final p in g.grid.cells.where((c) => c.row == row)) {
        final old = state.cells[p.col] ?? const _CellState();
        final nodes = p.cell['content'] as List? ?? [];
        final f = _fragmentCell(
          nodes,
          math.max(1, g.xs[p.col + p.cols] - g.xs[p.col] - 16),
          remaining - 16,
          math.max(1, capacity - 16),
          old,
          '$key:${p.row}:${p.col}',
          si,
          depth + 1,
          force: force && out.height == 0,
        );
        entries.add((p, f));
        nextCells[p.col] = f.next;
        used = math.max(used, f.block.height);
        advanced |= f.block.height > .01;
        done &= f.next.block == nodes.length;
      }
      final spec = rowNode['attrs']?['height'];
      final minimum =
          state.minimumLeft ??
          (spec?['rule'] == 'atLeast'
              ? (spec['value'] as num).toDouble() / 15
              : 0.0);
      if (!advanced && !done) break;
      final height = math.min(remaining, math.max(used + 16, minimum));
      final left = math.max(0.0, minimum - height);
      final rowPieces = _FragmentBuilder(measured.width)..height = height;
      for (final (p, f) in entries) {
        if (['center', 'bottom'].contains(p.cell['attrs']?['verticalAlign'])) {
          notices.add(
            'Split table cells use top alignment; center and bottom alignment across pages need further layout support.',
          );
        }
        final rect = Rect.fromLTRB(
          g.xs[p.col],
          0,
          g.xs[p.col + p.cols],
          height,
        );
        rowPieces.cells.add(
          _CellPiece(rect, wordTextColor(p.cell['attrs']?['fill'])),
        );
        // Whole rows retain their original vertical alignment. Continuations
        // begin at the top so no source lines disappear in an artificial gap.
        final offset = Offset(g.xs[p.col] + 8, 8);
        rowPieces.text.addAll(
          f.block.text.map((t) => t.shifted(offset, within: rect)),
        );
        rowPieces.cells.addAll(
          f.block.cells.map((c) => c.shifted(offset, within: rect)),
        );
      }
      out.append(rowPieces.build(), 0, height);
      state = done && left <= .01
          ? _TableState(row: end)
          : _TableState(
              row: row,
              cells: Map.unmodifiable(nextCells),
              minimumLeft: left,
            );
      if (state.row == row) break;
    }
    return _TableFragment(out.build(), state);
  }

  void _tableFields(_MeasuredBlock block, int si) {
    for (final t in block.text) {
      for (final site in t.fields) {
        final boxes = t.painter.getBoxesForSelection(
          TextSelection(baseOffset: site.start, extentOffset: site.end),
        );
        final visible = boxes.any(
          (b) => t.clip == null || t.clip!.overlaps(b.toRect().shift(t.offset)),
        );
        if (visible) {
          nextFields[site.key] = fieldValue(site.field, si, pages.last.number);
        }
      }
    }
  }

  void _flowTable(Map node, int bi, int si, _ColumnResume? resume) {
    var state = resume?.table ?? _TableState(row: resume?.rowIndex ?? 0);
    final count = (node['content'] as List).length;
    while (state.row < count) {
      final measured = table(
        node,
        columnGeometry.width,
        'body:$bi',
        si,
        pages.last.number,
        false,
        0,
      );
      final capacity = pages.last.bottom - columnTop;
      var available = columnBottom - cursor;
      final g = measured.tableGeometry!;
      _MeasuredBlock? header;
      if (measured.headerRows > 0 && state.row >= measured.headerRows) {
        header = table(
          {
            ...node,
            'content': (node['content'] as List)
                .take(measured.headerRows)
                .toList(),
          },
          columnGeometry.width,
          'repeated:$bi:${pages.last.physicalNumber}:$column',
          si,
          pages.last.number,
          true,
          0,
        );
        // A repeating prefix must leave room for meaningful body content.
        final end = g.groupEnd(state.row, measured.headerRows);
        final whole = g.ys[end] - g.ys[state.row];
        final keep =
            !tableRowCanSplit(g.grid.rows[state.row]) || end != state.row + 1;
        final minimum =
            keep && !state.started && whole <= capacity - header.height
            ? whole
            : math.min(whole, 36.0);
        if (header.height + minimum > available + .01) {
          if (balancingTrial) throw _ColumnOverflow();
          if (cursor > columnTop + .01 &&
              header.height + minimum <= capacity + .01) {
            nextColumn(si);
            continue;
          }
          notices.add(
            'A table header and its next row group do not fit together; the header is omitted on that continuation.',
          );
          header = null;
        }
      } else if (state.row == 0 &&
          measured.headerRows > 0 &&
          measured.headerRows < count) {
        final end = g.groupEnd(measured.headerRows, measured.headerRows);
        final needed = g.ys[end];
        if (needed <= capacity + .01 && needed > available + .01) {
          if (balancingTrial) throw _ColumnOverflow();
          if (cursor > columnTop + .01) {
            nextColumn(si);
            continue;
          }
        }
      }
      available -= header?.height ?? 0;
      var f = _fragmentTable(
        measured,
        available,
        capacity - (header?.height ?? 0),
        state,
        'body:$bi',
        si,
        0,
      );
      if (f.block.height <= .01) {
        if (cursor > columnTop + .01) {
          nextColumn(si);
          continue;
        }
        if (balancingTrial) throw _ColumnOverflow();
        f = _fragmentTable(
          measured,
          available,
          capacity - (header?.height ?? 0),
          state,
          'body:$bi',
          si,
          0,
          force: true,
        );
        if (f.block.height <= .01) {
          // No room even for the cell margins: drop the derived heading and
          // retry; never emit an unbounded stream of empty continuation pages.
          if (header != null) {
            header = null;
            f = _fragmentTable(
              measured,
              columnBottom - cursor,
              capacity,
              state,
              'body:$bi',
              si,
              0,
              force: true,
            );
          }
          if (f.block.height <= .01) {
            throw const FormatException(
              'The page text area is too short for a table row.',
            );
          }
        }
      }
      final page = pages.last;
      page.sections.add(si);
      if (header != null) {
        page.content.add(
          PagePlacement._(
            header,
            Offset(page.margins.left + columnGeometry.left, cursor),
            0,
            header.height,
            bi,
            'tableHeader',
          ),
        );
        cursor += header.height;
      }
      page.content.add(
        PagePlacement._(
          f.block,
          Offset(page.margins.left + columnGeometry.left, cursor),
          0,
          f.block.height,
          bi,
          'body',
          state,
        ),
      );
      _tableFields(f.block, si);
      cursor += f.block.height;
      band.usedBottom = math.max(band.usedBottom, cursor);
      state = f.next;
      if (state.row < count) nextColumn(si);
    }
  }
}
