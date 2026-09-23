import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_named_styles.dart';
import 'package:folio_native/src/document_view.dart';

void main() {
  testWidgets(
    'replace an in-use style through its dialog, cancel, undo and reopen',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1100);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory dir;
      await tester.runAsync(() async {
        await CoreBridge.initialize();
        dir = await Directory.systemTemp.createTemp('folio-replace-style-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new'});
        await core.call({
          'op': 'importOffice',
          'path':
              '${Directory.current.path}/test/fixtures/word-named-styles.docx',
        });
        model = WorkspaceModel(core, '${dir.path}/recovery.folio');
        await model.initialize();
      });
      Future<void> until(bool Function() ready) async {
        for (var i = 0; i < 500 && !ready(); i++) {
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
        await until(() => done);
        if (error != null) throw error!;
      }

      await tester.pumpWidget(
        FolioApp(model: model, agentDirectory: '${dir.path}/agent'),
      );
      await tester.pumpAndSettle();
      addTearDown(() async {
        await tester.pumpWidget(const SizedBox.shrink());
        model.dispose();
      });
      final view = tester.state<DocumentViewState>(find.byType(DocumentView));
      Map doc() => model.workspace['document'] as Map;
      List<Map> styles() =>
          (doc()['attrs']['wordStyles']['styles'] as List).cast<Map>();
      // Fixture uses ReportBody in the body; ReportHeading uses it as its following style.
      final sourceStyle = styles().firstWhere((s) => s['id'] == 'ReportBody');
      final before = jsonEncode(doc());
      final sheets = jsonEncode(model.workspace['sheets']);
      Future<void> openDelete() async {
        unawaited(view.editStyles());
        await until(
          () => find.byType(DocumentStylesDialog).evaluate().isNotEmpty,
        );
        await tester.enterText(
          find.byKey(const Key('named-style-search')),
          sourceStyle['name'] as String,
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('named-style-ReportBody')));
        await tester.tap(find.byKey(const Key('named-style-delete')));
        await tester.pumpAndSettle();
      }

      await openDelete();
      expect(find.text('Replace with'), findsOneWidget);
      await tester.tap(
        find.descendant(
          of: find.byType(DeleteDocumentStyleDialog),
          matching: find.text('Cancel'),
        ),
      );
      await tester.pumpAndSettle();
      expect(jsonEncode(doc()), before);
      await tester.tap(find.byKey(const Key('named-style-delete')));
      await tester.pumpAndSettle();
      final revision = model.revision;
      await tester.tap(find.byKey(const Key('named-style-delete-confirm')));
      await until(() => model.revision != revision);
      await drive(() => model.flush());
      expect(styles().any((s) => s['id'] == 'ReportBody'), isFalse);
      expect(
        styles().firstWhere((s) => s['id'] == 'ReportHeading')['next'],
        'Normal',
      );
      expect(jsonEncode(model.workspace['sheets']), sheets);
      void check(Map node) {
        expect(
          node['attrs']?['paragraphFormat']?['styleId'],
          isNot('ReportBody'),
        );
        for (final child in node['content'] as List? ?? []) {
          check(child as Map);
        }
      }

      check(doc());
      final after = jsonEncode(doc());
      await drive(() => model.mutate({'op': 'undo'}, refreshDocument: true));
      expect(jsonEncode(doc()), before);
      await drive(() => model.mutate({'op': 'redo'}, refreshDocument: true));
      expect(jsonEncode(doc()), after);
      final editor = tester
          .widget<q.QuillEditor>(find.byType(q.QuillEditor).first)
          .controller;
      expect(editor.document.toPlainText(), contains('Revenue'));
      final output = Directory(
        '${Directory.systemTemp.path}/folio-style-replacement-ui',
      );
      await drive(() async {
        await output.create(recursive: true);
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${output.path}/replaced.docx',
        });
        await model.core.call({
          'op': 'save',
          'path': '${output.path}/replaced.folio',
        });
        await model.open('${output.path}/replaced.folio');
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${output.path}/reopened.docx',
        });
      });
      expect(jsonEncode(doc()), after);
      expect(tester.takeException(), isNull);
    },
    timeout: const Timeout(Duration(minutes: 3)),
  );
}
