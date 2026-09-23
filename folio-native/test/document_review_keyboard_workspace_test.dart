import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/history_commands.dart';
import 'package:folio_native/src/workspace_file_commands.dart';

WorkspaceScreenState screenFor(WidgetTester tester) =>
    tester.state<WorkspaceScreenState>(find.byType(WorkspaceScreen));

void main() {
  testWidgets(
    'review shortcuts retain document history when focused review controls disappear',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-moves-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'importOffice',
          'path': File('test/fixtures/moved-revisions.docx').absolute.path,
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
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
        for (var i = 0; i < 500; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 6)),
          );
          if (done && find.byType(LinearProgressIndicator).evaluate().isEmpty) {
            break;
          }
        }
        if (error != null) throw error!;
        expect(done, isTrue);
        expect(find.byType(LinearProgressIndicator), findsNothing);
        await tester.pumpAndSettle();
      }

      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      final states = <Map>[];
      messenger.setMockMethodCallHandler(historyMenuChannel, (call) async {
        states.add(call.arguments as Map);
        return null;
      });
      const selector = MethodChannel('plugins.flutter.io/file_selector');
      messenger.setMockMethodCallHandler(
        selector,
        (call) async => '${folder.path}/saved-review.folio',
      );
      Future<void> native(String command, {int? generation}) async {
        final done = Completer<void>();
        messenger.handlePlatformMessage(
          historyMenuChannel.name,
          const StandardMethodCodec().encodeMethodCall(
            MethodCall('command', {
              'command': command,
              'generation': generation ?? states.last['generation'],
            }),
          ),
          (_) => done.complete(),
        );
        await done.future;
      }

      Future<void> shortcut({bool redo = false}) async {
        await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
        if (redo) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
        if (redo) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
        await drive(screenFor(tester).flush);
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
        messenger.setMockMethodCallHandler(historyMenuChannel, null);
        messenger.setMockMethodCallHandler(selector, null);
      });
      Future<void> tap(String key) async {
        final finder = find.byKey(Key(key));
        await tester.ensureVisible(finder);
        await tester.tap(finder);
        await drive(screen.flush);
      }

      await tester.tap(find.text('Review').first);
      await tester.pumpAndSettle();
      await tap('document-review-view');
      expect(find.text('1 of 3 changes'), findsOneWidget);
      await tap('review-page-reject');
      expect(find.text('1 of 1 changes'), findsOneWidget);
      final revision = model.revision;
      await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
      await drive(screen.flush);
      expect(model.revision, greaterThan(revision));
      expect(find.text('1 of 3 changes'), findsOneWidget);
      await shortcut(redo: true);
      expect(find.text('1 of 1 changes'), findsOneWidget);
      await drive(() => screen.fileCommand(WorkspaceFileCommand.saveCopy));
      expect(File('${folder.path}/saved-review.folio').existsSync(), isTrue);
      await shortcut();
      expect(find.text('1 of 3 changes'), findsOneWidget);
      expect(states.last['canRedo'], isTrue);
      await drive(() => native('redo'));
      expect(find.text('1 of 1 changes'), findsOneWidget);
      await drive(() => native('undo'));
      expect(find.text('1 of 3 changes'), findsOneWidget);
      final unchanged = model.revision;
      final documentGeneration = states.last['generation'] as int;
      await tester.enterText(find.byKey(const Key('prompt')), 'Draft text');
      await tester.pumpAndSettle();
      expect(states.last['canUndo'], isTrue);
      expect(states.last['canRedo'], isFalse);
      // An old menu click cannot cross into a newly focused field.
      await drive(() => native('undo', generation: documentGeneration));
      expect(screen.prompt.text, 'Draft text');
      await drive(() => native('undo'));
      expect(screen.prompt.text, isEmpty);
      expect(states.last['canUndo'], isFalse);
      expect(states.last['canRedo'], isTrue);
      await drive(() => native('undo'));
      expect(model.revision, unchanged);
      await drive(() => native('redo'));
      expect(screen.prompt.text, 'Draft text');
      expect(model.revision, unchanged);
      final originalTitle = model.workspace['title'];
      final rename = screen.rename();
      await tester.pumpAndSettle();
      final field = find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextField),
      );
      await tester.enterText(field, 'Temporary name');
      await tester.pumpAndSettle();
      expect(states.last['available'], isTrue);
      await drive(() => native('undo'));
      expect(tester.widget<TextField>(field).controller!.text, originalTitle);
      await drive(() => native('redo'));
      expect(
        tester.widget<TextField>(field).controller!.text,
        'Temporary name',
      );
      await tester.tap(find.text('Cancel').last);
      await drive(() => rename);
      expect(model.workspace['title'], originalTitle);
      expect(model.revision, unchanged);
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
