import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_table_view.dart';
import 'package:folio_native/src/document_tables.dart';

void main() {
  testWidgets(
    'selected table rows preview, cancel, convert with drafts, undo and export',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 960);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final capture = Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1';
      if (capture) {
        await tester.runAsync(() async {
          for (final (family, path) in [
            ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
            ('Aptos', '/System/Library/Fonts/Supplemental/Arial.ttf'),
            ('Arial', '/System/Library/Fonts/Supplemental/Arial.ttf'),
            ('Georgia', '/System/Library/Fonts/Supplemental/Georgia.ttf'),
            (
              'MaterialIcons',
              '${Platform.environment['FLUTTER_ROOT'] ?? '/opt/homebrew/share/flutter'}/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
            ),
          ]) {
            final loader = FontLoader(family)
              ..addFont(
                Future.value(
                  ByteData.sublistView(await File(path).readAsBytes()),
                ),
              );
            await loader.load();
          }
        });
      }
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = Directory(
          '${Directory.systemTemp.path}/folio-word-row-conversion-tests',
        );
        await folder.create(recursive: true);
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path':
              '${Directory.current.path}/test/fixtures/word-row-conversion.docx',
        });
        model = WorkspaceModel(
          core,
          '${folder.path}/workspace-${DateTime.now().microsecondsSinceEpoch}.folio',
        );
        await model.initialize();
      });
      Future<void> drive(Future<void> Function() action) async {
        var done = false;
        Object? error;
        unawaited(
          Future.sync(action).then(
            (_) => done = true,
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

      Future<void> revision(int wanted) async {
        for (var i = 0; i < 150 && model.revision != wanted; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        await drive(model.flush);
        expect(model.revision, wanted);
      }

      await tester.pumpWidget(
        RepaintBoundary(
          key: const Key('rows-preview-frame'),
          child: FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
        ),
      );
      await tester.pumpAndSettle();
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
      });
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      final original = jsonEncode(model.workspace['document']);
      final source = copyTableNode(model.workspace['document']['content'][1]);
      final initialRevision = model.revision;
      Finder cell(int r, int c, {bool dialog = false}) {
        final key = find.byKey(ValueKey('table-cell-$r-$c'));
        return dialog
            ? find
                  .descendant(
                    of: find.byType(DocumentTableDialog),
                    matching: key,
                  )
                  .first
            : key.first;
      }

      Future<void> tap(Finder target) async {
        await tester.ensureVisible(target);
        await tester.tap(target);
        await tester.pumpAndSettle();
      }

      Future<void> preview(String name) async {
        if (!capture) return;
        final boundary = tester.renderObject<RenderRepaintBoundary>(
          find.byKey(const Key('rows-preview-frame')),
        );
        await tester.runAsync(() async {
          final image = await boundary.toImage();
          final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
          image.dispose();
          final file = File(
            '${Directory.current.path}/release/previews/$name.png',
          );
          await file.parent.create(recursive: true);
          await file.writeAsBytes(bytes!.buffer.asUint8List());
        });
      }

      // Full-row selection includes a vertical merge outside the clicked columns.
      await tap(cell(3, 1));
      await tap(find.byKey(const Key('table-convert-rows')));
      expect(find.text('Rows 4–5 will become text.'), findsOneWidget);
      expect(
        find.text('The range includes extra rows to keep merged cells intact.'),
        findsOneWidget,
      );
      await tester.enterText(
        find.byKey(const Key('conversion-first-row')),
        '0',
      );
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<FilledButton>(find.byKey(const Key('conversion-apply')))
            .onPressed,
        isNull,
      );
      await tap(find.text('Cancel').last);
      await tap(find.text('Cancel').last);
      expect(model.revision, initialRevision);

      Future<void> draftRows() async {
        await tap(cell(1, 0));
        final rich = tester
            .widget<q.QuillEditor>(find.byType(q.QuillEditor).last)
            .controller;
        rich.replaceText(
          0,
          0,
          'Revised ',
          const TextSelection.collapsed(offset: 8),
        );
        await tester.pumpAndSettle();
        // Imported paragraph spacing may make this row taller than the table
        // viewport; reveal the cell before extending the selection.
        await tester.ensureVisible(cell(2, 2, dialog: true));
        await tester.pumpAndSettle();
        await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
        await tester.tap(cell(2, 2, dialog: true));
        await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
        await tester.pumpAndSettle();
        await tap(find.byKey(const Key('table-convert-rows')));
        expect(find.text('Rows 2–3 will become text.'), findsOneWidget);
        expect(find.text('1 row above remains in a table.'), findsOneWidget);
        expect(find.text('2 rows below remain in a table.'), findsOneWidget);
        expect(find.text('Convert and apply'), findsOneWidget);
      }

      await draftRows();
      await preview('selected-row-conversion');
      await tap(find.text('Cancel').last);
      final draft = tester
          .widget<DocumentTableView>(
            find
                .descendant(
                  of: find.byType(DocumentTableDialog),
                  matching: find.byType(DocumentTableView),
                )
                .first,
          )
          .node;
      expect(nodeText(DocumentTable(draft).at(1, 0).cell), 'Revised R2C1');
      await tap(find.text('Cancel').last);
      expect(jsonEncode(model.workspace['document']), original);
      await draftRows();
      await tap(find.byKey(const Key('conversion-apply')));
      await revision(initialRevision + 1);
      expect(find.byType(DocumentTableDialog), findsNothing);
      final converted = jsonEncode(model.workspace['document']);
      await preview('converted-row-document');
      final blocks = model.workspace['document']['content'] as List;
      final tables = blocks.where((n) => n['type'] == 'table').toList();
      expect(tables.length, 2);
      expect(tables.first['content'], [source['content'][0]]);
      expect(tables.last['content'], (source['content'] as List).sublist(3));
      expect(tables.first['attrs'], source['attrs']);
      expect(tables.last['attrs'], source['attrs']);
      expect(
        nodeText(model.workspace['document']),
        contains('Revised R2C1\tR2C2'),
      );
      expect(DocumentTable(tables.last).at(0, 0).rows, 2);
      expect(DocumentTable(tables.last).at(0, 1).cols, 2);
      expect(screen.editor.selection.isCollapsed, isTrue);
      Future<void> export(String name) => drive(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/$name.docx',
        });
      });
      await export('rows');
      final caret = screen.editor.selection.baseOffset;
      expect(caret, screen.editor.document.toPlainText().indexOf('After rows'));
      screen.editor.replaceText(
        caret,
        0,
        'X',
        TextSelection.collapsed(offset: caret + 1),
      );
      await revision(initialRevision + 2);
      expect(
        (model.workspace['document']['content'] as List)
            .where((n) => n['type'] == 'table')
            .length,
        2,
      );
      await tap(find.byKey(const Key('undo')));
      await revision(initialRevision + 3);
      expect(jsonEncode(model.workspace['document']), converted);
      await tap(find.byKey(const Key('undo')));
      await revision(initialRevision + 4);
      expect(jsonEncode(model.workspace['document']), original);
      await export('undone');
      await tap(find.byKey(const Key('redo')));
      await revision(initialRevision + 5);
      expect(jsonEncode(model.workspace['document']), converted);
      await drive(() async {
        await model.core.call({'op': 'load', 'path': model.recoveryPath});
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/reopened.docx',
        });
      });
      final saved = await tester.runAsync(
        () => File(model.recoveryPath).readAsString(),
      );
      expect(jsonEncode(jsonDecode(saved!)['document']), converted);
      late String reimportedText;
      await drive(() async {
        final imported = await model.core.call({
          'op': 'importOffice',
          'path': '${folder.path}/rows.docx',
        });
        reimportedText = nodeText(imported['workspace']['document']);
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/reimported.docx',
        });
        await model.core.call({'op': 'load', 'path': model.recoveryPath});
      });
      expect(reimportedText, nodeText(jsonDecode(converted)));

      // A source-table replacement while the preview is open must win over the
      // captured conversion and produce a visible explanation instead of applying.
      await tap(cell(0, 0));
      await tap(find.byKey(const Key('table-convert-rows')));
      final updated = copyTableNode(tables.first);
      updated['attrs']['alignment'] = 'center';
      final at = screen.editor.document.toPlainText().indexOf('\uFFFC');
      final beforeExternal = model.revision;
      screen.editor.replaceText(
        at,
        1,
        q.BlockEmbed('folioNode', jsonEncode(updated)),
        TextSelection.collapsed(offset: at + 1),
      );
      await revision(beforeExternal + 1);
      await tap(find.byKey(const Key('conversion-apply')));
      await drive(screen.flush);
      expect(model.revision, beforeExternal + 1);
      expect(
        find.text('The table changed. Reopen it and try again.'),
        findsOneWidget,
      );
      final retained = (model.workspace['document']['content'] as List)
          .where((n) => n['type'] == 'table')
          .toList();
      expect(retained.length, 2);
      expect(retained.first['attrs']['alignment'], 'center');
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
