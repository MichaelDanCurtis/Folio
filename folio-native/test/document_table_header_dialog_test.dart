import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_table_format_dialog.dart';
import 'package:folio_native/src/document_tables.dart';
import 'document_table_headers_test.dart' as f;

void main() {
  testWidgets(
    'header checkbox handles mixed selection, no-op, Cancel, and off on small windows',
    (tester) async {
      tester.view.physicalSize = const Size(700, 480);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final grid = DocumentTable(f.table(headers: 1, bodyRows: 3));
      Map? result;
      var selection = const TableArea(0, 0, 1, 1);
      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () async {
                  result = await showDialog<Map>(
                    context: context,
                    builder: (_) =>
                        DocumentTableFormatDialog(table: grid, area: selection),
                  );
                },
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      );
      Future<void> open() async {
        await tester.tap(find.text('Open'));
        await tester.pumpAndSettle();
        await tester.ensureVisible(
          find.byKey(const Key('table-repeat-header')),
        );
        await tester.pumpAndSettle();
      }

      CheckboxListTile checkbox() =>
          tester.widget(find.byKey(const Key('table-repeat-header')));
      await open();
      expect(checkbox().value, isNull);
      await tester.tap(find.byKey(const Key('table-format-apply')));
      await tester.pumpAndSettle();
      expect(result, isEmpty);
      await open();
      await tester.tap(find.byKey(const Key('table-repeat-header')));
      await tester.pumpAndSettle();
      expect(checkbox().value, true);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(result, isNull);
      expect(grid.headerRows, 1);
      await open();
      await tester.tap(find.byKey(const Key('table-repeat-header')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('table-format-apply')));
      await tester.pumpAndSettle();
      expect(result, {'header': true});
      selection = const TableArea(0, 0, 0, 1);
      await open();
      expect(checkbox().value, true);
      await tester.tap(find.byKey(const Key('table-repeat-header')));
      await tester.pumpAndSettle();
      expect(checkbox().value, false);
      await tester.tap(find.byKey(const Key('table-format-apply')));
      await tester.pumpAndSettle();
      expect(result, {'header': false});
      selection = const TableArea(2, 0, 2, 1);
      await open();
      expect(
        find.textContaining('Only consecutive header rows'),
        findsOneWidget,
      );
      expect(checkbox().value, false);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    },
  );
}
