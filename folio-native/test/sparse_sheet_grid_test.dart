import 'dart:math';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sparse_sheet_grid.dart';
import 'package:folio_native/src/table_controls.dart';

class CountedLayout extends SheetAxisLayout {
  int offsets = 0, extents = 0;
  CountedLayout(super.visible, super.defaultExtent, super.sizes);
  @override
  double offset(int physical) {
    offsets++;
    return super.offset(physical);
  }

  @override
  double extent(int physical) {
    extents++;
    return super.extent(physical);
  }

  void reset() {
    offsets = 0;
    extents = 0;
  }
}

void main() {
  test(
    'sparse lookup agrees with a linear oracle across hidden and sized axes',
    () {
      final random = Random(82341);
      for (var trial = 0; trial < 80; trial++) {
        final hidden = <int>{};
        final sizes = <int, double>{};
        for (var i = 0; i < 90; i++) {
          if (random.nextInt(4) == 0) hidden.add(i);
          if (random.nextBool()) sizes[i] = 1.0 + random.nextInt(80);
        }
        final axis = SheetGridAxis(
          SheetAxisLayout(VisibleAxis(hidden, 90), 20, sizes),
          32,
          1,
        );
        final lengths = [
          32.0,
          for (var i = 0; i < 90; i++)
            if (!hidden.contains(i)) sizes[i] ?? 20.0,
        ];
        double start = 0;
        for (var i = 0; i < lengths.length; i++) {
          expect(axis.offset(i), start);
          expect(axis.extent(i), lengths[i]);
          expect(axis.indexAt(start), i);
          expect(axis.indexAt(start + lengths[i] - 0.01), i);
          start += lengths[i];
        }
        expect(axis.totalExtent, start);
        expect(axis.indexAt(-20), 0);
        expect(axis.indexAt(start + 10), lengths.length - 1);
      }
      final empty = SheetGridAxis(
        SheetAxisLayout(VisibleAxis([0, 1], 2), 20, {}),
        32,
        1,
      );
      expect(empty.count, 1);
      expect(empty.totalExtent, 32);
      expect(empty.visibleSpans(0, 100).length, 1);
    },
  );

  testWidgets(
    'last Excel cell and shrink at the end require bounded layout work',
    (tester) async {
      final vertical = ScrollController(), horizontal = ScrollController();
      final rows = CountedLayout(VisibleAxis([1, 5, 500000], 1048576), 20, {
        2: 45,
        1048575: 37,
      });
      final columns = CountedLayout(VisibleAxis([1, 3], 16384), 70, {
        2: 100,
        16383: 110,
      });
      var built = 0;
      Widget view(SheetAxisLayout r, SheetAxisLayout c) => MaterialApp(
        home: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 600,
            height: 400,
            child: SparseSheetGrid(
              rows: SheetGridAxis(r, 32, 2),
              columns: SheetGridAxis(c, 43, 2),
              verticalDetails: ScrollableDetails.vertical(controller: vertical),
              horizontalDetails: ScrollableDetails.horizontal(
                controller: horizontal,
              ),
              cellBuilder: (context, v) {
                built++;
                return ColoredBox(
                  key: ValueKey((v.xIndex, v.yIndex)),
                  color: Colors.white,
                );
              },
            ),
          ),
        ),
      );
      rows.reset();
      columns.reset();
      await tester.pumpWidget(view(rows, columns));
      expect(built, lessThan(600));
      expect(
        rows.offsets + rows.extents + columns.offsets + columns.extents,
        lessThan(800),
      );
      // These expectations use independent arithmetic, including hidden positions
      // and explicit dimensions, rather than the viewport's own totals.
      expect(
        vertical.position.maxScrollExtent,
        32 + (1048576 - 3) * 20 + 25 + 17 - 400,
      );
      expect(
        horizontal.position.maxScrollExtent,
        43 + (16384 - 2) * 70 + 30 + 40 - 600,
      );
      final corner = tester.getTopLeft(find.byKey(const ValueKey((1, 1))));
      built = 0;
      rows.reset();
      columns.reset();
      vertical.jumpTo(vertical.position.maxScrollExtent);
      horizontal.jumpTo(horizontal.position.maxScrollExtent);
      await tester.pumpAndSettle();
      expect(built, lessThan(600));
      expect(
        rows.offsets + rows.extents + columns.offsets + columns.extents,
        lessThan(1000),
      );
      expect(tester.getTopLeft(find.byKey(const ValueKey((1, 1)))), corner);
      final last = find.byKey(const ValueKey((16382, 1048573)));
      expect(last, findsOneWidget);
      expect(tester.getBottomRight(last), const Offset(600, 400));
      final endRows = SheetAxisLayout(
        VisibleAxis([1, 5, 500000, 1048575], 1048576),
        20,
        {2: 45},
      );
      final endColumns = SheetAxisLayout(
        VisibleAxis([1, 3, 16383], 16384),
        70,
        {2: 100},
      );
      await tester.pumpWidget(view(endRows, endColumns));
      await tester.pumpAndSettle();
      expect(vertical.offset, vertical.position.maxScrollExtent);
      expect(horizontal.offset, horizontal.position.maxScrollExtent);
      expect(
        tester.getBottomRight(find.byKey(const ValueKey((16381, 1048572)))),
        const Offset(600, 400),
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      vertical.dispose();
      horizontal.dispose();
    },
  );

  testWidgets(
    'frozen quadrants clip paint, clicks, semantics and implicit reveal',
    (tester) async {
      final semantics = tester.ensureSemantics();
      final vertical = ScrollController(), horizontal = ScrollController();
      final boundary = GlobalKey();
      (int, int)? selected;
      const colors = [Colors.red, Colors.green, Colors.blue, Colors.yellow];
      await tester.pumpWidget(
        MaterialApp(
          home: Align(
            alignment: Alignment.topLeft,
            child: RepaintBoundary(
              key: boundary,
              child: SizedBox(
                width: 300,
                height: 180,
                child: SparseSheetGrid(
                  rows: SheetGridAxis(
                    SheetAxisLayout(VisibleAxis([], 100), 20, {}),
                    20,
                    2,
                  ),
                  columns: SheetGridAxis(
                    SheetAxisLayout(VisibleAxis([], 100), 60, {}),
                    20,
                    2,
                  ),
                  verticalDetails: ScrollableDetails.vertical(
                    controller: vertical,
                  ),
                  horizontalDetails: ScrollableDetails.horizontal(
                    controller: horizontal,
                  ),
                  cellBuilder: (context, v) => Semantics(
                    label: 'slot ${v.xIndex},${v.yIndex}',
                    child: GestureDetector(
                      key: ValueKey((v.xIndex, v.yIndex)),
                      onTap: () => selected = (v.xIndex, v.yIndex),
                      child: ColoredBox(
                        color:
                            colors[(v.xIndex < 2 ? 1 : 0) +
                                (v.yIndex < 2 ? 2 : 0)],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      horizontal.jumpTo(35);
      vertical.jumpTo(55);
      await tester.pumpAndSettle();
      // Scrolled children overlap these coordinates geometrically, but must
      // neither paint nor receive pointer events through a frozen pane.
      final clicks = <(Offset, (int, int), Color)>[
        (const Offset(65, 35), (1, 1), Colors.yellow),
        (const Offset(65, 55), (1, 5), Colors.green),
        (const Offset(90, 35), (2, 1), Colors.blue),
        (const Offset(90, 55), (2, 5), Colors.red),
      ];
      for (final (position, cell, _) in clicks) {
        await tester.tapAt(position);
        expect(selected, cell);
      }
      final labels = tester.semantics
          .simulatedAccessibilityTraversal()
          .map((node) => node.label)
          .toList();
      expect(labels, contains('slot 1,1'));
      expect(labels, contains('slot 2,5'));
      expect(labels, isNot(contains('slot 2,2')));
      final render =
          boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
      await tester.runAsync(() async {
        final image = await render.toImage(pixelRatio: 1);
        final bytes = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        for (final (position, _, color) in clicks) {
          final at =
              (position.dy.toInt() * image.width + position.dx.toInt()) * 4;
          expect(
            bytes.getUint32(at),
            ((color.toARGB32() & 0xffffff) << 8) | 0xff,
          );
        }
        image.dispose();
      });
      // Revealing a pinned cell must not move either axis. Revealing a cached
      // cell must align it outside the frozen header area, not beneath it.
      await Scrollable.ensureVisible(
        tester.element(find.byKey(const ValueKey((1, 1)))),
        alignment: 0,
      );
      await tester.pumpAndSettle();
      expect(horizontal.offset, 35);
      expect(vertical.offset, 55);
      await Scrollable.ensureVisible(
        tester.element(find.byKey(const ValueKey((2, 4)))),
        alignment: 0,
      );
      await tester.pumpAndSettle();
      expect(
        tester.getTopLeft(find.byKey(const ValueKey((2, 4)))),
        const Offset(80, 40),
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      semantics.dispose();
      vertical.dispose();
      horizontal.dispose();
    },
  );
}
