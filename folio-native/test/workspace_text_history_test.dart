import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/history_commands.dart';

void main() {
  var initialized = false;
  for (final useNativeMenu in [false, true]) {
    testWidgets(
      'focused fields and workspace history via ${useNativeMenu ? 'native menu' : 'shortcuts'}',
      (tester) async {
        tester.view.physicalSize = const Size(1440, 960);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        late Directory dir;
        late WorkspaceModel model;
        await tester.runAsync(() async {
          if (!initialized) {
            await CoreBridge.initialize();
            initialized = true;
          }
          dir = await Directory.systemTemp.createTemp('folio-field-history-');
          final core = CoreBridge();
          await core.call({'op': 'new'});
          await core.call({
            'op': 'importOffice',
            'path':
                '${Directory.current.path}/test/fixtures/word-conversion.docx',
          });
          model = WorkspaceModel(core, '${dir.path}/workspace.folio');
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

        final messenger =
            TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
        Map? menuState;
        messenger.setMockMethodCallHandler(historyMenuChannel, (call) async {
          menuState = call.arguments as Map;
          return null;
        });
        await tester.pumpWidget(
          FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
        );
        await tester.pumpAndSettle();
        final screen = tester.state<WorkspaceScreenState>(
          find.byType(WorkspaceScreen),
        );
        addTearDown(() async {
          await tester.pumpWidget(const SizedBox.shrink());
          model.dispose();
          messenger.setMockMethodCallHandler(historyMenuChannel, null);
          await tester.runAsync(() => dir.delete(recursive: true));
        });
        final original = jsonEncode(model.workspace['document']);
        screen.editor.replaceText(
          0,
          0,
          'Saved ',
          const TextSelection.collapsed(offset: 6),
        );
        await drive(screen.flush);
        final edited = jsonEncode(model.workspace['document']);
        final revision = model.revision;
        final saved = await tester.runAsync(
          () => File(model.recoveryPath).readAsString(),
        );

        Future<void> historyKey({bool redo = false}) async {
          if (useNativeMenu) {
            final done = Completer<void>();
            messenger.handlePlatformMessage(
              historyMenuChannel.name,
              const StandardMethodCodec().encodeMethodCall(
                MethodCall('command', {
                  'command': redo ? 'redo' : 'undo',
                  'generation': menuState!['generation'],
                }),
              ),
              (_) => done.complete(),
            );
            await drive(() => done.future);
            return;
          }
          await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
          if (redo) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
          await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
          if (redo) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
          await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
          await tester.pumpAndSettle();
        }

        Future<void> localHistory(String key, String draft) async {
          final field = find.byKey(Key(key));
          await tester.tap(field);
          // Start typing as soon as focus arrives; no history-seeding delay.
          await tester.pump();
          final controller = tester.widget<TextField>(field).controller!;
          final initial = controller.text;
          await tester.enterText(field, draft);
          await tester.pump(const Duration(milliseconds: 600));
          await historyKey();
          expect(
            controller.text,
            initial,
            reason: '$key Undo must edit the focused field',
          );
          expect(
            model.revision,
            revision,
            reason: '$key must not undo a saved document change',
          );
          await historyKey(redo: true);
          expect(
            controller.text,
            draft,
            reason: '$key Redo must restore its draft',
          );
          await historyKey(redo: true);
          expect(controller.text, draft);
          expect(model.revision, revision);
          await historyKey();
          expect(controller.text, initial);
          // Exhausting the field's history must not fall through to workspace Undo.
          await historyKey();
          expect(model.revision, revision);
          expect(jsonEncode(model.workspace['document']), edited);
        }

        await localHistory('prompt', 'Explain the budget');
        await drive(() => screen.showSearch(true));
        await localHistory('find-query', 'Research');
        await localHistory('replace-text', 'Investigation');
        await tester.tap(find.byTooltip('Close search'));
        await tester.pumpAndSettle();

        final text = screen.editor.document.toPlainText();
        screen.editor.updateSelection(
          TextSelection(
            baseOffset: text.indexOf('Name'),
            extentOffset: text.indexOf('After conversion'),
          ),
          q.ChangeSource.local,
        );
        await tester.tap(find.byKey(const Key('text-to-table')));
        await tester.pumpAndSettle();
        await localHistory('conversion-width', '120');
        await localHistory('conversion-columns', '3');
        await tester.tap(find.text('Cancel'));
        await tester.pumpAndSettle();
        expect(model.revision, revision);

        await tester.tap(find.byTooltip('Workbook'));
        await tester.pumpAndSettle();
        await localHistory('formula', '=SUM(10,20)');
        await tester.tap(find.byKey(const Key('cell-A1')));
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key('formula')));
        await tester.pump(const Duration(milliseconds: 600));
        final formula = tester
            .widget<TextField>(find.byKey(const Key('formula')))
            .controller!;
        final cellA1 = formula.text;
        await historyKey();
        expect(
          formula.text,
          cellA1,
          reason: 'A new cell must not inherit the previous cell draft history',
        );
        await historyKey(redo: true);
        expect(formula.text, cellA1);
        expect(model.revision, revision);
        await tester.tap(find.byKey(const Key('cell-B4')));
        await tester.pumpAndSettle();
        await tester.sendKeyEvent(LogicalKeyboardKey.f2);
        await tester.pumpAndSettle();
        await localHistory('cell-editor', 'Draft cell');
        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
        await drive(screen.flush);
        expect(
          await tester.runAsync(() => File(model.recoveryPath).readAsString()),
          saved,
        );

        Future<void> waitForRevision(int before) async {
          for (var i = 0; i < 100 && model.revision == before; i++) {
            await tester.pump(const Duration(milliseconds: 20));
            await tester.runAsync(
              () => Future<void>.delayed(const Duration(milliseconds: 10)),
            );
          }
          await drive(screen.flush);
          expect(model.revision, before + 1);
        }

        Future<void> workspaceHistory({bool redo = false}) async {
          final before = model.revision;
          await historyKey(redo: redo);
          await waitForRevision(before);
        }

        // Grid focus has no draft text history: these keys undo committed edits.
        await workspaceHistory();
        expect(jsonEncode(model.workspace['document']), original);
        await workspaceHistory(redo: true);
        expect(jsonEncode(model.workspace['document']), edited);
        await tester.tap(find.byTooltip('Document'));
        await tester.pumpAndSettle();
        tester
            .widget<q.QuillEditor>(find.byType(q.QuillEditor).first)
            .focusNode
            .requestFocus();
        await tester.pumpAndSettle();
        await workspaceHistory();
        expect(jsonEncode(model.workspace['document']), original);
        await workspaceHistory(redo: true);
        expect(jsonEncode(model.workspace['document']), edited);

        // Committing a formula creates one workspace edit. On refocusing the
        // formula bar, its draft history starts from the newly committed value.
        await tester.tap(find.byTooltip('Workbook'));
        await tester.pumpAndSettle();
        final oldCell = screen.currentSheet['cells']['B4'];
        await tester.tap(find.byKey(const Key('formula')));
        await tester.pump(const Duration(milliseconds: 600));
        await tester.enterText(find.byKey(const Key('formula')), '4800');
        await tester.pump(const Duration(milliseconds: 600));
        final beforeCommit = model.revision;
        await tester.testTextInput.receiveAction(TextInputAction.done);
        await waitForRevision(beforeCommit);
        expect(screen.currentSheet['cells']['B4'], '4800');
        await workspaceHistory();
        expect(screen.currentSheet['cells']['B4'], oldCell);
        await workspaceHistory(redo: true);
        expect(screen.currentSheet['cells']['B4'], '4800');
        final committedRevision = model.revision;
        // Enter now moves below the committed cell, as the inline editor does.
        // Return to B4 before checking its fresh field-local history.
        await tester.tap(find.byKey(const Key('cell-B4')));
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key('formula')));
        await tester.pump(const Duration(milliseconds: 600));
        await historyKey();
        expect(
          tester
              .widget<TextField>(find.byKey(const Key('formula')))
              .controller!
              .text,
          '4800',
        );
        expect(model.revision, committedRevision);
        expect(tester.takeException(), isNull);
      },
      variant: TargetPlatformVariant.only(TargetPlatform.macOS),
    );
  }
}
