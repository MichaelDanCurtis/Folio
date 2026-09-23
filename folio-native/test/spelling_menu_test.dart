import 'dart:async';
import 'dart:io';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_codec.dart';

void main() {
  var bridgeReady = false;
  for (final (tabbed, leaders) in [
    (false, false),
    (true, false),
    (true, true),
  ]) {
    final prefix = tabbed ? '😀\tWe\t' : '😀 We ';
    testWidgets(
      'mouse can correct spelling and undo after ${leaders
          ? 'tabs with leaders'
          : tabbed
          ? 'tabs'
          : 'spaces'}',
      (tester) async {
        tester.view.physicalSize = const Size(1440, 960);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        late Directory dir;
        late WorkspaceModel model;
        await tester.runAsync(() async {
          if (!bridgeReady) {
            await CoreBridge.initialize();
            bridgeReady = true;
          }
          dir = await Directory.systemTemp.createTemp('folio-spelling-menu-');
          final core = CoreBridge();
          await core.call({'op': 'new'});
          await core.call({
            'op': 'document',
            'document': {
              'type': 'doc',
              'content': [
                {
                  'type': 'paragraph',
                  if (leaders)
                    'attrs': {
                      'tabs': {
                        'inherited': [],
                        'direct': [
                          {
                            'position': 2160,
                            'alignment': 'left',
                            'leader': 'dot',
                          },
                          {
                            'position': 4320,
                            'alignment': 'left',
                            'leader': 'dot',
                          },
                        ],
                      },
                    },
                  'content': [
                    {'type': 'text', 'text': prefix},
                    {
                      'type': 'text',
                      'text': 'recieved',
                      'marks': [
                        {'type': 'bold'},
                      ],
                    },
                    {'type': 'text', 'text': ' a letter.'},
                  ],
                },
              ],
            },
          });
          model = WorkspaceModel(core, '${dir.path}/workspace.folio');
          await model.initialize();
        });
        Future<T> drive<T>(Future<T> Function() job) async {
          var done = false;
          T? value;
          Object? error;
          unawaited(
            Future.sync(job).then(
              (v) {
                value = v;
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
          if (!done) throw StateError('Native operation did not finish');
          if (error != null) throw error!;
          return value as T;
        }

        await tester.pumpWidget(
          FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
        );
        await tester.pumpAndSettle();
        final screen = tester.state<WorkspaceScreenState>(
          find.byType(WorkspaceScreen),
        );
        await drive(screen.proofreading.checkNow);
        for (var i = 0; i < 500 && screen.proofreading.busy; i++) {
          await tester.pump(const Duration(milliseconds: 20));
          await tester.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 10)),
          );
        }
        expect(
          screen.proofreading.issues.any((i) => i.text == 'recieved'),
          isTrue,
        );
        final before = model.workspace['document'];
        final raw = tester.state<q.QuillRawEditorState>(
          find.byType(q.QuillRawEditor),
        );
        final rect = raw.renderEditor.getLocalRectForCaret(
          const TextPosition(offset: 9),
        );
        final point = raw.renderEditor.localToGlobal(rect.center);
        await tester.tapAt(
          point,
          buttons: kSecondaryMouseButton,
          kind: PointerDeviceKind.mouse,
        );
        await tester.pumpAndSettle();
        expect(find.text('received'), findsOneWidget);
        final divider = find.byKey(const Key('proofreading-menu-divider'));
        expect(divider, findsOneWidget);
        expect(
          tester.getRect(divider).top,
          greaterThan(tester.getRect(find.text('received')).bottom),
        );
        expect(
          tester.getRect(divider).bottom,
          lessThanOrEqualTo(tester.getRect(find.text('Cut')).top),
        );
        final menuIssue = screen.proofreading.issues.firstWhere(
          (i) => i.text == 'recieved',
        );
        final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
        await mouse.down(tester.getCenter(find.text('received')));
        await tester.pump();
        // A real mouse waits between press and release. The editor must not dismiss
        // its menu on pointer-down and discard the pending button activation.
        expect(find.text('received'), findsOneWidget);
        await mouse.up();
        await drive(screen.flush);
        await tester.pumpAndSettle();
        expect(
          nodeText(model.workspace['document']),
          '${prefix}received a letter.',
        );
        expect(
          screen.editor.document.toDelta().toJson().any(
            (op) =>
                op['insert'] == 'received' && op['attributes']?['bold'] == true,
          ),
          isTrue,
        );
        expect(find.text('received'), findsNothing);
        await drive(() => screen.history(false));
        expect(model.workspace['document'], before);
        await drive(screen.proofreading.checkNow);
        // Rechecking creates new diagnostic objects. An unchanged correction from
        // an already-open menu remains valid; validation compares content, not identity.
        expect(screen.proofreading.issues.contains(menuIssue), isFalse);
        await drive(() => screen.fixWriting(menuIssue, 'received'));
        expect(nodeText(model.workspace['document']), contains('received'));
        await drive(() => screen.history(false));
        expect(model.workspace['document'], before);
        await tester.pumpAndSettle();
        final ordinaryPoint = raw.renderEditor.localToGlobal(
          raw.renderEditor
              .getLocalRectForCaret(const TextPosition(offset: 20))
              .center,
        );
        await tester.tapAt(
          ordinaryPoint,
          buttons: kSecondaryMouseButton,
          kind: PointerDeviceKind.mouse,
        );
        await tester.pumpAndSettle();
        expect(
          find.byKey(const Key('proofreading-menu-divider')),
          findsNothing,
        );
        expect(find.text('Select All'), findsOneWidget);
        await mouse.down(tester.getCenter(find.text('Select All')));
        await tester.pump();
        expect(find.text('Select All'), findsOneWidget);
        await mouse.up();
        await tester.pumpAndSettle();
        expect(screen.editor.selection.start, 0);
        expect(screen.editor.selection.end, screen.editor.document.length - 1);
        expect(tester.takeException(), isNull);
        await drive(screen.flush);
        await tester.pumpWidget(const SizedBox());
        model.dispose();
        await tester.runAsync(() => dir.delete(recursive: true));
      },
      variant: TargetPlatformVariant({TargetPlatform.macOS}),
    );
  }
}
