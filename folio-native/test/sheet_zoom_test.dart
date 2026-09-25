import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_view.dart';
import 'package:folio_native/src/sheet_zoom.dart';

void main() {
  test('zoom steps, limits and Zoom to Selection', () {
    expect(stepZoom(100, up: true), 125);
    expect(stepZoom(100, up: false), 75);
    expect(stepZoom(110, up: false), 100);
    expect(stepZoom(400, up: true), 400);
    expect(stepZoom(10, up: false), 10);
    expect(clampZoom(5), 10);
    expect(clampZoom(1000), 400);
    expect(
      zoomToFit(
        selectionWidth: 400,
        selectionHeight: 100,
        viewportWidth: 800,
        viewportHeight: 600,
      ),
      200,
    );
    expect(
      zoomToFit(
        selectionWidth: 100000,
        selectionHeight: 100,
        viewportWidth: 800,
        viewportHeight: 600,
      ),
      10,
    );
  });

  Future<(SheetViewState, List<int>, Map<String, dynamic>)> mount(
    WidgetTester t,
    int zoom,
  ) async {
    final zooms = <int>[];
    final sheet = <String, dynamic>{
      'id': 'data',
      'name': 'Data',
      'zoom': zoom,
      'cells': <String, dynamic>{'A1': 'Item', 'B3': '42'},
    };
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SheetView(
            sheet: sheet,
            calculated: const {},
            onSelection: (_) {},
            onChanges: (_) async {},
            onZoom: (percent) async => zooms.add(percent),
          ),
        ),
      ),
    );
    await t.pumpAndSettle();
    return (t.state<SheetViewState>(find.byType(SheetView)), zooms, sheet);
  }

  testWidgets('cells scale with the zoom and clicks land on them', (t) async {
    for (final zoom in [50, 200]) {
      final (state, _, _) = await mount(t, zoom);
      final cell = t.getRect(find.byKey(const Key('cell-B3')));
      final corner = t.getRect(find.byKey(const Key('cell-A1')));
      // Column B starts one column after A, at the zoomed column width.
      expect(cell.left - corner.left, closeTo(corner.width, 0.01));
      await t.tapAt(cell.center);
      await t.pumpAndSettle();
      expect(state.selectedRange, 'B3', reason: '$zoom%');
      await t.pumpWidget(const SizedBox());
    }
    final (_, _, _) = await mount(t, 100);
    final normal = t.getRect(find.byKey(const Key('cell-A1')));
    await t.pumpWidget(const SizedBox());
    final (_, _, _) = await mount(t, 200);
    final zoomed = t.getRect(find.byKey(const Key('cell-A1')));
    expect(zoomed.width, closeTo(normal.width * 2, 0.01));
    expect(zoomed.height, closeTo(normal.height * 2, 0.01));
    await t.pumpWidget(const SizedBox());
  });

  testWidgets('Command/Ctrl with + and −, and Zoom to Selection', (t) async {
    final (state, zooms, _) = await mount(t, 100);
    await t.tap(find.byKey(const Key('cell-A1')));
    await t.pumpAndSettle();
    await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await t.sendKeyEvent(LogicalKeyboardKey.equal);
    await t.sendKeyEvent(LogicalKeyboardKey.minus);
    await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await t.pumpAndSettle();
    expect(zooms, [125, 75]);

    await state.goTo('A1:B2');
    await t.pumpAndSettle();
    await state.zoomToSelection();
    await t.pumpAndSettle();
    expect(zooms.last, greaterThan(200), reason: 'a few cells fill the grid');
    await state.goTo('A1:Z200');
    await t.pumpAndSettle();
    await state.zoomToSelection();
    await t.pumpAndSettle();
    expect(zooms.last, lessThan(100));
    await t.pumpWidget(const SizedBox());
  });

  testWidgets('the status-bar control steps, slides and offers presets', (
    t,
  ) async {
    final calls = <(int, bool)>[];
    var toSelection = 0;
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: SheetZoomControl(
              zoom: 100,
              onZoom: (p, {required commit}) => calls.add((p, commit)),
              onZoomToSelection: () => toSelection++,
            ),
          ),
        ),
      ),
    );
    await t.tap(find.byKey(const Key('zoom-in')));
    await t.tap(find.byKey(const Key('zoom-out')));
    expect(calls, [(125, true), (75, true)]);
    calls.clear();
    await t.drag(find.byKey(const Key('zoom-slider')), const Offset(30, 0));
    await t.pumpAndSettle();
    expect(calls.any((c) => !c.$2), isTrue, reason: 'previews while sliding');
    expect(calls.last.$2, isTrue, reason: 'commits on release');
    expect(calls.last.$1, greaterThan(100));
    await t.tap(find.byKey(const Key('zoom-percent')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('zoom-200')));
    await t.pumpAndSettle();
    expect(calls.last, (200, true));
    await t.tap(find.byKey(const Key('zoom-percent')));
    await t.pumpAndSettle();
    await t.tap(find.byKey(const Key('zoom-selection')));
    await t.pumpAndSettle();
    expect(toSelection, 1);
  });
}
