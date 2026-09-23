import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_lists.dart';

Map<String, dynamic> paragraph(String text, [int? heading]) => {
  'type': heading == null ? 'paragraph' : 'heading',
  if (heading != null) 'attrs': {'level': heading},
  'content': [
    {'type': 'text', 'text': text},
  ],
};

void main() {
  testWidgets(
    'first custom style preserves native text, geometry and table drafts',
    (tester) async {
      tester.view.physicalSize = const Size(1500, 2200);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory directory;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        directory = await Directory.systemTemp.createTemp(
          'folio-native-styles-',
        );
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [
              for (var level = 1; level <= 6; level++) ...[
                paragraph('Native heading $level', level),
                paragraph('Native body after heading $level'),
              ],
              {
                'type': 'bulletList',
                'content': [
                  for (var i = 1; i <= 2; i++)
                    {
                      'type': 'listItem',
                      'content': [paragraph('Native bullet $i')],
                    },
                ],
              },
              {
                'type': 'table',
                'content': [
                  {
                    'type': 'tableRow',
                    'content': [
                      {
                        'type': 'tableCell',
                        'content': [
                          paragraph('Native cell heading', 2),
                          paragraph('Native cell body'),
                        ],
                      },
                    ],
                  },
                ],
              },
              paragraph('Native ending'),
            ],
          },
        });
        model = WorkspaceModel(core, '${directory.path}/workspace.folio');
        await model.initialize();
      });
      Future<void> drive(Future<void> Function() action) async {
        var done = false;
        Object? error;
        unawaited(
          action().then(
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
        expect(done, true);
        await tester.pumpAndSettle();
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${directory.path}/agent'),
      );
      await tester.pumpAndSettle();
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
      });
      Map<String, dynamic> geometry() => {
        for (final w in tester.widgetList<q.QuillLayoutText>(
          find.byType(q.QuillLayoutText),
        ))
          if (w.text.toPlainText().startsWith('Native '))
            w.text.toPlainText(): {
              'rect': tester.getRect(find.byWidget(w)),
              'size': w.text.style?.fontSize,
              'family': w.text.style?.fontFamily,
              'weight': w.text.style?.fontWeight,
              'height': w.text.style?.height,
              'align': w.textAlign,
            },
      };
      final before = geometry();
      expect(before.length, 17);
      final text = tester
          .state<WorkspaceScreenState>(find.byType(WorkspaceScreen))
          .editor
          .document
          .toPlainText();
      // Compare the table editor before catalog materialization as well as its
      // static view. Both use the same paragraph defaults as the main editor.
      await tester.tap(find.byKey(const ValueKey('table-cell-0-0')).first);
      await tester.pumpAndSettle();
      final cellBefore = geometry();
      await tester.tap(find.byKey(const Key('table-paragraph')));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<TextField>(find.byKey(const Key('paragraph-line')))
            .controller!
            .text,
        '1.4',
      );
      await tester.enterText(find.byKey(const Key('paragraph-left')), '12');
      await tester.pump();
      await tester.tap(find.byKey(const Key('apply-paragraph')));
      await tester.pumpAndSettle();
      final cellEditor = tester
          .widgetList<q.QuillEditor>(find.byType(q.QuillEditor))
          .last
          .controller;
      final direct = listMetadata(
        listLines(cellEditor.document.toDelta()).first.attrs['token'],
      )['paragraphFormat']['direct'];
      expect(direct['line'], 336);
      expect(direct['before'], 270);
      expect(direct['after'], 180);
      await tester.tap(find.widgetWithText(TextButton, 'Cancel').last);
      await tester.pumpAndSettle();
      await drive(
        () => model.mutate({
          'op': 'documentStyle',
          'workspaceId': model.workspace['id'],
          'baseRevision': model.revision,
          'action': 'upsert',
          'style': {
            'id': 'UnusedNativeStyle',
            'name': 'Unused native style',
            'type': 'paragraph',
            'basedOn': 'Normal',
            'next': 'Normal',
            'properties': {'paragraph': {}, 'tabs': [], 'run': {}},
          },
        }, refreshDocument: true),
      );
      expect(
        geometry(),
        before,
        reason:
            'Adding an unused style must not restyle or move existing native text',
      );
      expect(
        tester
            .state<WorkspaceScreenState>(find.byType(WorkspaceScreen))
            .editor
            .document
            .toPlainText(),
        text,
      );
      await tester.tap(find.byKey(const ValueKey('table-cell-0-0')).first);
      await tester.pumpAndSettle();
      expect(
        geometry(),
        cellBefore,
        reason: 'Cell draft defaults also stay stable',
      );
      await tester.tap(find.widgetWithText(TextButton, 'Cancel').last);
      await tester.pumpAndSettle();
      final screen = tester.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      await drive(() => screen.history(false));
      expect(geometry(), before);
      await drive(() => screen.history(true));
      expect(geometry(), before);
      await drive(screen.flush);
      expect(tester.takeException(), null);
    },
    timeout: const Timeout(Duration(minutes: 2)),
  );
}
