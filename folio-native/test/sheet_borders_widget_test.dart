import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/sheet_view.dart';

void main() {
  testWidgets('Borders and Fill Color apply to the selection', (t) async {
    final borders = <Map>[], formats = <(List<String>, Map)>[];
    final sheet = {
      'id': 'data',
      'name': 'Data',
      'cells': <String, dynamic>{'A1': 'Item'},
      'cellStyles': <String, dynamic>{
        'C3': {
          'border': {
            'bottom': {'style': 'thick', 'color': '#FF0000'},
          },
        },
      },
    };
    await t.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SheetView(
            sheet: sheet,
            calculated: const {},
            onSelection: (_) {},
            onChanges: (_) async {},
            onFormat: (addresses, style, {baseRevision}) async =>
                formats.add((addresses, style)),
            onBorders: (action) async => borders.add(action),
          ),
        ),
      ),
    );
    await t.pumpAndSettle();
    final state = t.state<SheetViewState>(find.byType(SheetView));

    // Imported borders are painted.
    final decoration =
        t
                .widget<Container>(
                  find
                      .descendant(
                        of: find.byKey(const Key('cell-C3')),
                        matching: find.byType(Container),
                      )
                      .first,
                )
                .decoration
            as BoxDecoration;
    final border = decoration.border as Border;
    expect(border.bottom.color, const Color(0xffff0000));
    expect(border.bottom.width, 3);

    await state.goTo('B2:C4');
    await t.pumpAndSettle();
    Future<void> choose(String menu, String item) async {
      await t.tap(find.byKey(Key(menu)));
      await t.pumpAndSettle();
      await t.ensureVisible(find.byKey(Key(item)));
      await t.pumpAndSettle();
      await t.tap(find.byKey(Key(item)));
      await t.pumpAndSettle();
    }

    await choose('borders-menu', 'border-line-medium');
    await choose('borders-menu', 'color:#0070C0');
    await choose('borders-menu', 'border-outside');
    expect(borders.single, {
      'sheetId': 'data',
      'range': 'B2:C4',
      'preset': 'outside',
      'line': 'medium',
      'color': '#0070C0',
    });
    await choose('borders-menu', 'border-thickBox');
    expect(borders.last['line'], 'thick');

    await choose('fill-color-menu', '#FFFF00');
    expect(formats.last.$2, {'fill': '#FFFF00'});
    expect(formats.last.$1.length, 6);
    await choose('fill-color-menu', 'fill-none');
    expect(formats.last.$2, {'fill': 'none'});
    await t.pumpWidget(const SizedBox());
  });
}
