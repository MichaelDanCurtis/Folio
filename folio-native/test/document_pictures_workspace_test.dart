import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_codec.dart';
import 'package:folio_native/src/document_pictures.dart';

void main() {
  testWidgets(
    'Insert Picture stores the file, shows it, undoes in one step and exports',
    (t) async {
      t.view.physicalSize = const Size(1440, 1100);
      t.view.devicePixelRatio = 1;
      addTearDown(t.view.resetPhysicalSize);
      addTearDown(t.view.resetDevicePixelRatio);
      late WorkspaceModel model;
      late Directory folder;
      await t.runAsync(() async {
        await CoreBridge.initialize();
        folder = await Directory.systemTemp.createTemp('folio-pictures-ui-');
        final core = CoreBridge();
        await core.call({'op': 'new', 'blank': true});
        await core.call({
          'op': 'document',
          'document': {
            'type': 'doc',
            'content': [
              {
                'type': 'heading',
                'attrs': {'level': 1},
                'content': [
                  {'type': 'text', 'text': 'Intro'},
                ],
              },
              {
                'type': 'paragraph',
                'content': [
                  {'type': 'text', 'text': 'Back to top'},
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
      // A 2000×1000 PNG header: the engine reads its size.
      final png = Uint8List.fromList([
        0x89,
        0x50,
        0x4e,
        0x47,
        0x0d,
        0x0a,
        0x1a,
        0x0a,
        0,
        0,
        0,
        13,
        0x49,
        0x48,
        0x44,
        0x52,
        0,
        0,
        0x07,
        0xd0,
        0,
        0,
        0x03,
        0xe8,
        8,
        6,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
      ]);
      final previous = choosePictureFile;
      choosePictureFile = () async => (name: 'wide.png', bytes: png);
      addTearDown(() => choosePictureFile = previous);
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

      List<Map> pictures() {
        final out = <Map>[];
        void walk(Map n) {
          if (n['type'] == 'image') out.add(n['attrs'] as Map);
          for (final c in n['content'] as List? ?? []) {
            walk(c as Map);
          }
        }

        walk(model.workspace['document'] as Map);
        return out;
      }

      screen.editor.updateSelection(
        const TextSelection.collapsed(offset: 4),
        q.ChangeSource.local,
      );
      await t.pump();
      final revision = model.revision;
      await t.tap(find.byKey(const Key('insert-picture')));
      await until(() => pictures().isNotEmpty);
      await drive(screen.flush);
      final picture = pictures().single;
      // Scaled to the 6.5-inch text width: 624 × 312.
      expect(picture['width'], 624);
      expect(picture['height'], 312);
      expect(model.revision, revision + 1);
      final shown = find.byKey(ValueKey('document-picture-${picture['src']}'));
      expect(shown, findsOneWidget);
      expect(t.widget<Semantics>(shown).properties.label, 'Picture');
      expect(nodeText(model.workspace['document']['content'][0]), 'Intro');

      await drive(() => screen.history(false));
      expect(pictures(), isEmpty);
      await drive(() => screen.history(true));
      expect(pictures().single['src'], picture['src']);

      await t.runAsync(() async {
        await model.core.call({
          'op': 'exportOffice',
          'format': 'docx',
          'path': '${folder.path}/pictured.docx',
        });
      });
      await drive(screen.flush);
      await t.runAsync(() async {
        final reopened = CoreBridge();
        final imported = await reopened.call({
          'op': 'importOffice',
          'path': '${folder.path}/pictured.docx',
        });
        final para = imported['workspace']['document']['content'][0] as Map;
        final image = (para['content'] as List).firstWhere(
          (n) => n['type'] == 'image',
        );
        expect(image['attrs']['src'], picture['src']);
        expect(image['attrs']['width'], 624);
      });
      await t.pump(const Duration(seconds: 5));
    },
  );
}
