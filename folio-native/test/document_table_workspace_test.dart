import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_table_view.dart';
import 'package:folio_native/src/document_tables.dart';
import 'package:folio_native/src/document_codec.dart';

void main() {
  testWidgets(
    'merged table layout, rich cell editing, structure, cancel and one-step Undo',
    (tester) async {
      tester.view.physicalSize = const Size(1152, 768);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory dir;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-word-table-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path': '${Directory.current.path}/test/fixtures/word-tables.docx',
        });
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
      });
      Future<void> drive(Future<void> Function() action) async {
        var done = false;
        Object? error;
        unawaited(
          Future.sync(action).then(
            (_) {
              done = true;
            },
            onError: (Object e) {
              error = e;
              done = true;
            },
          ),
        );
        for (var i = 0; i < 500 && !done; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        if (error != null) throw error!;
        expect(done, isTrue);
        await tester.pumpAndSettle();
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await tester.pumpAndSettle();
      final original = jsonEncode(model.workspace['document']);
      expect(find.text('35 words'), findsOneWidget);
      final revision = model.revision;
      // The top-left 2x2 cell is one interactive surface, not duplicated text.
      final large = tester.getRect(
        find.byKey(const ValueKey('table-cell-0-0')).first,
      );
      final below = tester.getRect(
        find.byKey(const ValueKey('table-cell-2-0')),
      );
      final neighbor = tester.getRect(
        find.byKey(const ValueKey('table-cell-1-2')),
      );
      expect(large.bottom, closeTo(below.top, .1));
      expect(large.width, greaterThan(neighbor.width));
      await tester.ensureVisible(find.byKey(const ValueKey('table-cell-2-1')));
      await tester.tap(find.byKey(const ValueKey('table-cell-2-1')));
      await tester.pumpAndSettle();
      expect(find.byType(DocumentTableDialog), findsOneWidget);
      var rich = tester
          .widget<q.QuillEditor>(find.byType(q.QuillEditor).last)
          .controller;
      expect(rich.document.toPlainText(), contains('Second paragraph'));
      rich.replaceText(
        0,
        5,
        'Updated cell extra',
        const TextSelection.collapsed(offset: 18),
      );
      await tester.pumpAndSettle();
      // Structure and merge are draft edits until Apply.
      await tester.tap(find.byKey(const Key('table-structure')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Insert row below'));
      await tester.pumpAndSettle();
      Finder gridCell(int r, int c) => find.descendant(
        of: find.byType(DocumentTableDialog),
        matching: find.byKey(ValueKey('table-cell-$r-$c')),
      );
      await tester.ensureVisible(gridCell(3, 0));
      await tester.tap(gridCell(3, 0));
      await tester.pumpAndSettle();
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await tester.tap(gridCell(3, 1));
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('table-merge')));
      await tester.pumpAndSettle();
      expect(gridCell(3, 1), findsNothing);
      expect(model.revision, revision);
      await tester.tap(find.byKey(const Key('table-apply')));
      await tester.pumpAndSettle();
      await drive(() => model.flush());
      expect(model.revision, revision + 1);
      expect(find.text('36 words'), findsOneWidget);
      final table = DocumentTable(model.workspace['document']['content'][1]);
      expect(table.rows.length, 4);
      expect(table.at(3, 0).cols, 2);
      final content = table.at(2, 1).cell['content'];
      expect(nodeText(content[0]), startsWith('Updated cell'));
      expect(content[0]['content'][0]['marks'], [
        {'type': 'italic'},
      ]);
      expect(content[1]['content'][0]['marks'], [
        {'type': 'bold'},
      ]);
      final saved = await tester.runAsync(
        () => File('${dir.path}/workspace.folio').readAsString(),
      );
      expect(jsonDecode(saved!)['document'], model.workspace['document']);
      await drive(
        () => model.core
            .call({
              'op': 'exportOffice',
              'format': 'docx',
              'path': '${dir.path}/edited-table.docx',
            })
            .then((_) {}),
      );
      await tester.runAsync(() async {
        final output = Directory(
          '${Directory.systemTemp.path}/folio-word-table-tests',
        );
        await output.create(recursive: true);
        await File(
          '${dir.path}/edited-table.docx',
        ).copy('${output.path}/ui-edited.docx');
      });
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), original);
      expect(find.byKey(const ValueKey('table-cell-3-0')), findsNothing);
      expect(find.text('35 words'), findsOneWidget);
      await drive(() => screen.history(true));
      expect(find.byKey(const ValueKey('table-cell-3-0')), findsOneWidget);
      expect(find.text('36 words'), findsOneWidget);
      expect(
        DocumentTable(model.workspace['document']['content'][1]).rows.length,
        4,
      );
      // Split and Cancel must not alter the committed document.
      final applied = jsonEncode(model.workspace['document']);
      await tester.ensureVisible(
        find.byKey(const ValueKey('table-cell-0-0')).first,
      );
      await tester.tap(find.byKey(const ValueKey('table-cell-0-0')).first);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('table-split')));
      await tester.pumpAndSettle();
      final draft = tester.widget<DocumentTableView>(
        find
            .descendant(
              of: find.byType(DocumentTableDialog),
              matching: find.byType(DocumentTableView),
            )
            .first,
      );
      expect(DocumentTable(draft.node).at(0, 0).rows, 1);
      expect(DocumentTable(draft.node).at(0, 1).col, 1);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(jsonEncode(model.workspace['document']), applied);
      // Formatting is a draft until the enclosing table is applied.
      final beforeFormat = jsonEncode(model.workspace['document']);
      final formatRevision = model.revision;
      await tester.ensureVisible(
        find.byKey(const ValueKey('table-cell-0-0')).first,
      );
      await tester.tap(find.byKey(const ValueKey('table-cell-0-0')).first);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('table-format')));
      await tester.pumpAndSettle();
      await tester.enterText(find.byKey(const Key('table-column-width')), '0');
      await tester.tap(find.byKey(const Key('table-format-apply')));
      await tester.pumpAndSettle();
      expect(find.textContaining('Enter a column width'), findsOneWidget);
      await tester.enterText(find.byKey(const Key('table-column-width')), '90');
      Future<void> choose(String key, String label) async {
        await tester.ensureVisible(find.byKey(Key(key)));
        await tester.tap(find.byKey(Key(key)));
        await tester.pumpAndSettle();
        await tester.tap(find.text(label).last);
        await tester.pumpAndSettle();
      }

      await choose('table-height-rule', 'Exactly');
      await tester.enterText(find.byKey(const Key('table-row-height')), '24');
      await tester.ensureVisible(find.byKey(const Key('table-repeat-header')));
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const Key('table-repeat-header')),
            )
            .value,
        isNull,
      );
      await tester.tap(find.byKey(const Key('table-repeat-header')));
      await tester.pumpAndSettle();
      await choose('table-cell-fill', 'Light yellow');
      await choose('table-cell-valign', 'Bottom');
      await choose('table-alignment', 'Right');
      await tester.tap(find.byKey(const Key('table-format-apply')));
      await tester.pumpAndSettle();
      expect(model.revision, formatRevision);
      expect(jsonEncode(model.workspace['document']), beforeFormat);
      await tester.tap(find.byKey(const Key('table-apply')));
      await tester.pumpAndSettle();
      await drive(() => model.flush());
      expect(model.revision, formatRevision + 1);
      final formatted = DocumentTable(
        model.workspace['document']['content'][1],
      );
      expect(formatted.headerRows, 2);
      expect(formatted.node['attrs']['columnWidths'], [1800, 1800, 2100, 1700]);
      expect(formatted.node['attrs']['layout'], 'fixed');
      expect(formatted.at(0, 0).cell['attrs']['fill'], 'FFF2CC');
      expect(formatted.rows[0]['attrs']['height'], {
        'rule': 'exact',
        'value': 480,
      });
      expect(formatted.rows[1]['attrs']['height'], {
        'rule': 'exact',
        'value': 480,
      });
      final mergedRect = tester.getRect(
        find.byKey(const ValueKey('table-cell-0-0')).first,
      );
      expect(mergedRect.width, closeTo(240, .1));
      expect(mergedRect.height, closeTo(64, .1));
      expect(find.text('36 words'), findsOneWidget);
      await drive(
        () => model.core
            .call({
              'op': 'exportOffice',
              'format': 'docx',
              'path':
                  '${Directory.systemTemp.path}/folio-word-table-tests/layout-ui.docx',
            })
            .then((_) {}),
      );
      await drive(() => screen.history(false));
      expect(jsonEncode(model.workspace['document']), beforeFormat);
      await drive(() => screen.history(true));
      expect(
        DocumentTable(
          model.workspace['document']['content'][1],
        ).at(0, 0).cell['attrs']['fill'],
        'FFF2CC',
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      model.dispose();
      await tester.runAsync(() => dir.delete(recursive: true));
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
