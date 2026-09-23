import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/template_gallery.dart';
import 'package:folio_native/src/working_files.dart';
import 'package:folio_native/src/workspace_file_commands.dart';

void main() {
  testWidgets(
    'template gallery creates plain files, edits explicit templates and protects pending work',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1050);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final capture = Platform.environment['FOLIO_CAPTURE_PREVIEWS'] == '1';
      if (capture) {
        await tester.runAsync(() async {
          for (final (family, path) in [
            ('.AppleSystemUIFont', '/System/Library/Fonts/SFNS.ttf'),
            ('Aptos', '/System/Library/Fonts/Supplemental/Arial.ttf'),
            (
              'MaterialIcons',
              '/opt/homebrew/share/flutter/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
            ),
          ]) {
            await (FontLoader(family)..addFont(
                  Future.value(
                    ByteData.sublistView(await File(path).readAsBytes()),
                  ),
                ))
                .load();
          }
        });
      }
      late Directory dir;
      late WorkspaceModel model;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-template-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        model = WorkspaceModel(core, '${dir.path}/workspace.folio');
        await model.initialize();
      });
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      const selector = MethodChannel('plugins.flutter.io/file_selector');
      final choices = <String?>[];
      final suggested = <String>[];
      String? browse;
      var acquired = 0, released = 0;
      messenger.setMockMethodCallHandler(selector, (call) async {
        if (call.method == 'openFile') return browse == null ? null : [browse];
        suggested.add((call.arguments as Map)['suggestedName'] as String);
        return choices.isEmpty ? null : choices.removeAt(0);
      });
      messenger.setMockMethodCallHandler(fileAccessChannel, (call) async {
        final args = call.arguments as Map;
        if (call.method == 'capture') return args['path'];
        if (call.method == 'release') {
          released++;
          return null;
        }
        acquired++;
        return {
          'path': args['bookmark'],
          'bookmark': args['bookmark'],
          'lease': '$acquired',
        };
      });
      messenger.setMockMethodCallHandler(fileMenuChannel, (_) async => null);
      final renderKey = GlobalKey();
      await tester.pumpWidget(
        RepaintBoundary(
          key: renderKey,
          child: FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
        ),
      );
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
        messenger.setMockMethodCallHandler(selector, null);
        messenger.setMockMethodCallHandler(fileAccessChannel, null);
        messenger.setMockMethodCallHandler(fileMenuChannel, null);
      });
      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 800 && !ready(); i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 5)),
          );
        }
        expect(ready(), isTrue);
        await tester.pumpAndSettle();
      }

      Future<void> drive(Future<void> Function() action) async {
        var done = false;
        Object? failure;
        unawaited(
          Future.sync(action).then(
            (_) => done = true,
            onError: (Object e) {
              failure = e;
              done = true;
            },
          ),
        );
        await until(() => done);
        if (failure != null) throw failure!;
      }

      Future<void> command(WorkspaceFileCommand command) =>
          screen.fileCommand(command);
      Future<void> select(String title) async {
        final tile = find.widgetWithText(ListTile, title);
        if (tile.evaluate().isEmpty) {
          await tester.enterText(
            find.descendant(
              of: find.byType(TemplateGallery),
              matching: find.byType(TextField),
            ),
            title,
          );
          await tester.pumpAndSettle();
        }
        await tester.ensureVisible(tile);
        await tester.pumpAndSettle();
        await tester.tap(tile);
        await until(
          () =>
              tester
                  .widget<FilledButton>(
                    find.widgetWithText(FilledButton, 'Create'),
                  )
                  .onPressed !=
              null,
        );
      }

      Future<void> fromTemplate(String title, {bool edit = false}) async {
        var done = false;
        Object? failure;
        unawaited(
          command(WorkspaceFileCommand.fromTemplate).then(
            (_) => done = true,
            onError: (Object e) {
              failure = e;
              done = true;
            },
          ),
        );
        await until(() => find.byType(TemplateGallery).evaluate().isNotEmpty);
        await select(title);
        if (title == 'Budget') {
          expect(
            find.descendant(
              of: find.byType(TemplateGallery),
              matching: find.text(r'$0.00'),
            ),
            findsWidgets,
          );
          expect(
            find.descendant(
              of: find.byType(TemplateGallery),
              matching: find.text('=B2-C2'),
            ),
            findsNothing,
          );
        }
        if (title == 'Team budget') {
          expect(
            find.descendant(
              of: find.byType(TemplateGallery),
              matching: find.text(r'$875.00'),
            ),
            findsOneWidget,
          );
        }
        if (capture && ['Business letter', 'Budget'].contains(title)) {
          await tester.runAsync(() async {
            final image =
                await (renderKey.currentContext!.findRenderObject()
                        as RenderRepaintBoundary)
                    .toImage();
            final data = await image.toByteData(format: ui.ImageByteFormat.png);
            await File(
              '../../work/template-${title == 'Budget' ? 'excel' : 'word'}-widget.png',
            ).writeAsBytes(data!.buffer.asUint8List());
            image.dispose();
          });
        }
        await tester.tap(find.text(edit ? 'Edit template' : 'Create'));
        await until(() => done);
        if (failure != null) throw failure!;
      }

      List<PendingWorkingFile> pending() => screen.workingFiles.pending(
        model.workspace,
        model.snapshot['fileStates'] as Map,
        model.revision,
      );
      void type(String text) => screen.editor.replaceText(
        0,
        0,
        text,
        TextSelection.collapsed(offset: text.length),
      );
      await drive(() => screen.filesReady);
      final initialId = model.workspace['id'];
      await fromTemplate('Business letter');
      expect(model.workspace['id'], isNot(initialId));
      expect(
        screen.editor.document.toPlainText(),
        contains('Dear [Recipient]'),
      );
      expect(pending().single.kind, WorkingFileKind.docx);
      expect(screen.activeWorkingFile, isNull);
      expect(model.snapshot['canUndo'], isFalse);
      var done = false;
      final letterId = model.workspace['id'];
      choices.add(null);
      unawaited(
        command(WorkspaceFileCommand.newWorkspace).then((_) => done = true),
      );
      await until(
        () => find.text('Save changes to your files?').evaluate().isNotEmpty,
      );
      await tester.tap(find.widgetWithText(FilledButton, 'Save'));
      await until(() => done);
      expect(model.workspace['id'], letterId);
      expect(suggested.last, endsWith('.docx'));
      final letter = '${dir.path}/letter.docx';
      choices.add(letter);
      await drive(() => command(WorkspaceFileCommand.save));
      expect(pending(), isEmpty);

      type('Reusable greeting. ');
      final custom = '${dir.path}/Reusable.dotx';
      choices.add(custom);
      await drive(() => command(WorkspaceFileCommand.saveTemplate));
      expect(screen.activeWorkingFile!.path, letter);
      expect(
        pending().single.kind,
        WorkingFileKind.docx,
        reason: 'Saving a template copy does not mark the document saved',
      );
      await drive(() => command(WorkspaceFileCommand.save));
      final count = suggested.length;
      await fromTemplate('Reusable', edit: true);
      expect(screen.activeWorkingFile!.format, 'dotx');
      expect(screen.activeWorkingFile!.path, custom);
      type('Edited template. ');
      await drive(() => command(WorkspaceFileCommand.save));
      expect(
        suggested.length,
        count,
        reason: 'Explicit template editing saves in place',
      );
      expect(pending(), isEmpty);
      final templateBytes = File(custom).readAsBytesSync();
      choices.add('${dir.path}/Another.dotx');
      await drive(() => command(WorkspaceFileCommand.saveAs));
      expect(suggested.last, endsWith('.dotx'));
      expect(screen.activeWorkingFile!.format, 'dotx');
      expect(File(custom).readAsBytesSync(), templateBytes);
      final templateId = model.workspace['id'];
      await fromTemplate('Reusable');
      expect(model.workspace['id'], isNot(templateId));
      expect(screen.activeWorkingFile, isNull);
      expect(
        screen.editor.document.toPlainText(),
        startsWith('Edited template. Reusable greeting.'),
      );
      choices.add('${dir.path}/created.docx');
      await drive(() => command(WorkspaceFileCommand.save));
      expect(suggested.last, endsWith('.docx'));
      expect(File(custom).readAsBytesSync(), templateBytes);

      await fromTemplate('Budget');
      expect(screen.spreadsheet, isTrue);
      expect(pending().single.kind, WorkingFileKind.xlsx);
      expect(model.sheets.single['cells']['D2'], '=B2-C2');
      await drive(
        () => model.mutate({
          'op': 'cells',
          'cells': [
            {
              'sheetId': model.sheets.single['id'],
              'address': 'B2',
              'value': '1000',
            },
            {
              'sheetId': model.sheets.single['id'],
              'address': 'C2',
              'value': '125',
            },
          ],
        }),
      );
      expect(
        model.snapshot['calculated'][model.sheets.single['id']]['D2'],
        875,
      );
      choices.add('${dir.path}/budget.xlsx');
      await drive(() => command(WorkspaceFileCommand.save));
      final workbookTemplate = '${dir.path}/Team budget.xltx';
      choices.add(workbookTemplate);
      await drive(() => command(WorkspaceFileCommand.saveTemplate));
      expect(screen.activeWorkingFile!.path, '${dir.path}/budget.xlsx');
      expect(suggested.last, endsWith('.xltx'));
      final workbookBytes = File(workbookTemplate).readAsBytesSync();
      await fromTemplate('Team budget');
      expect(
        model.snapshot['calculated'][model.sheets.single['id']]['D2'],
        875,
      );
      expect(screen.activeWorkingFile, isNull);
      choices.add('${dir.path}/created.xlsx');
      await drive(() => command(WorkspaceFileCommand.save));
      expect(File(workbookTemplate).readAsBytesSync(), workbookBytes);

      // A file changed after Preview must not replace the current workspace.
      final before = jsonEncode(model.workspace);
      Object? staleError;
      done = false;
      unawaited(
        command(WorkspaceFileCommand.fromTemplate).then(
          (_) => done = true,
          onError: (Object e) {
            staleError = e;
            done = true;
          },
        ),
      );
      await until(() => find.byType(TemplateGallery).evaluate().isNotEmpty);
      await select('Team budget');
      File(workbookTemplate).writeAsBytesSync(
        File('assets/templates/budget.xltx').readAsBytesSync(),
      );
      await tester.tap(find.text('Create'));
      await until(() => done);
      expect('$staleError', contains('template changed'));
      expect(jsonEncode(model.workspace), before);
      expect(acquired, released);

      // Browse adds a personal template; cancelling the gallery leaves work alone.
      browse = '${Directory.current.path}/assets/templates/tracker.xltx';
      done = false;
      unawaited(
        command(WorkspaceFileCommand.fromTemplate).then((_) => done = true),
      );
      await until(() => find.byType(TemplateGallery).evaluate().isNotEmpty);
      await tester.tap(find.text('Browse templates…'));
      await until(() => screen.templates.files.entries.length == 4);
      await until(
        () =>
            tester
                .widget<FilledButton>(
                  find.widgetWithText(FilledButton, 'Create'),
                )
                .onPressed !=
            null,
      );
      await tester.tap(find.widgetWithText(TextButton, 'Cancel'));
      await until(() => done);
      expect(jsonEncode(model.workspace), before);
      expect(acquired, released);
      await drive(model.flush);
      final output = Directory(
        '${Directory.systemTemp.path}/folio-template-ui-output',
      );
      output.createSync(recursive: true);
      for (final name in [
        'Reusable.dotx',
        'letter.docx',
        'created.docx',
        'budget.xlsx',
        'created.xlsx',
        'workspace.folio',
      ]) {
        File('${dir.path}/$name').copySync('${output.path}/$name');
      }
      File('${output.path}/Team budget.xltx').writeAsBytesSync(workbookBytes);
      expect(tester.takeException(), isNull);
    },
  );
}
