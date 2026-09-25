import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';

void main() {
  testWidgets(
    'dragging table borders resizes columns and rows in one Undo step each',
    (t) async {
      t.view.physicalSize = const Size(1440, 1100);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await t.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp(
          'folio-table-resize-ui-',
        );
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [
              {
                'type': 'table',
                'attrs': {
                  'columnWidths': [4000, 4000],
                },
                'content': [
                  {
                    'type': 'tableRow',
                    'content': [
                      {
                        'type': 'tableCell',
                        'attrs': {'colspan': 1, 'rowspan': 1},
                        'content': [
                          {
                            'type': 'paragraph',
                            'content': [
                              {'type': 'text', 'text': 'A'},
                            ],
                          },
                        ],
                      },
                      {
                        'type': 'tableCell',
                        'attrs': {'colspan': 1, 'rowspan': 1},
                        'content': [
                          {
                            'type': 'paragraph',
                            'content': [
                              {'type': 'text', 'text': 'B'},
                            ],
                          },
                        ],
                      },
                    ],
                  },
                  {
                    'type': 'tableRow',
                    'content': [
                      {
                        'type': 'tableCell',
                        'attrs': {'colspan': 1, 'rowspan': 1},
                        'content': [
                          {
                            'type': 'paragraph',
                            'content': [
                              {'type': 'text', 'text': 'C'},
                            ],
                          },
                        ],
                      },
                      {
                        'type': 'tableCell',
                        'attrs': {'colspan': 1, 'rowspan': 1},
                        'content': [
                          {
                            'type': 'paragraph',
                            'content': [
                              {'type': 'text', 'text': 'D'},
                            ],
                          },
                        ],
                      },
                    ],
                  },
                ],
              },
            ],
          },
        });
        await core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/source.docx',
        });
        await core.call({
          'op': 'importOffice',
          'path': '${folder.path}/source.docx',
        });
        model = WorkspaceModel(core, '${folder.path}/workspace.folio');
        await model.initialize();
        await model.flush();
      });
      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 500 && !ready(); i++) {
          await t.pump(const Duration(milliseconds: 20));
          await t.runAsync(
            () => Future<void>.delayed(const Duration(milliseconds: 6)),
          );
        }
        expect(ready(), true);
      }

      await t.pumpWidget(
        FolioApp(model: model, agentDirectory: '${folder.path}/agent'),
      );
      await t.pumpAndSettle();
      final screen = t.state<WorkspaceScreenState>(
        find.byType(WorkspaceScreen),
      );
      addTearDown(() async {
        await t.pumpWidget(const SizedBox());
        model.dispose();
      });
      Map table() => model.workspace['document']['content'][0] as Map;
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
        await until(
          () => done && find.byType(LinearProgressIndicator).evaluate().isEmpty,
        );
        if (error != null) throw error!;
        await t.pumpAndSettle();
      }

      Future<void> settle() => drive(screen.flush);

      final original = jsonDecode(jsonEncode(table()));
      final a = t.getRect(find.byKey(const Key('table-cell-0-0')));
      final gesture = await t.startGesture(Offset(a.right, a.center.dy));
      await gesture.moveBy(const Offset(20, 0));
      await gesture.moveBy(const Offset(20, 0));
      await gesture.up();
      await settle();
      final widths = List<int>.from(table()['attrs']['columnWidths']);
      expect(widths[0], greaterThan(4000));
      expect(widths[0] + widths[1], 8000);
      await drive(() => screen.history(false));
      await settle();
      expect(jsonDecode(jsonEncode(table())), original);

      final c = t.getRect(find.byKey(const Key('table-cell-1-0')));
      final down = await t.startGesture(Offset(c.center.dx, c.bottom - 2));
      await down.moveBy(const Offset(0, 15));
      await down.moveBy(const Offset(0, 15));
      await down.up();
      await settle();
      final height = table()['content'][1]['attrs']['height'] as Map;
      expect(height['rule'], 'atLeast');
      expect(height['value'], ((c.height + 30) * 15).round());
      expect(table()['content'][0]['attrs']?['height'], isNull);

      // The column drag again, so both kinds are saved.
      final again = await t.startGesture(
        Offset(
          t.getRect(find.byKey(const Key('table-cell-0-0'))).right,
          a.center.dy,
        ),
      );
      await again.moveBy(const Offset(40, 0));
      await again.up();
      await settle();
      final resized = List<int>.from(table()['attrs']['columnWidths']);
      expect(resized, widths);
      await t.runAsync(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/resized.docx',
        });
        final reopened = CoreBridge();
        final imported = await reopened.call({
          'op': 'importOffice',
          'path': '${folder.path}/resized.docx',
        });
        final saved = imported['workspace']['document']['content'][0] as Map;
        expect(saved['content'][1]['attrs']['height'], {
          'rule': 'atLeast',
          'value': height['value'],
        });
      });
      await t.pump(const Duration(seconds: 5));
    },
  );
}
