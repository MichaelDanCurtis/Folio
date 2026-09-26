import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/main.dart';
import 'package:folio_native/src/core.dart';
import 'package:folio_native/src/document_codec.dart';

void main() {
  testWidgets('footnotes, captions, cross-references and a table of contents', (
    t,
  ) async {
    t.view.physicalSize = const Size(1440, 1100);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.resetPhysicalSize);
    addTearDown(t.view.resetDevicePixelRatio);
    late WorkspaceModel model;
    late Directory folder;
    await t.runAsync(() async {
      await CoreBridge.initialize();
      folder = await Directory.systemTemp.createTemp('folio-refs-ui-');
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
                {'type': 'text', 'text': 'Method'},
              ],
            },
            {
              'type': 'paragraph',
              'content': [
                {'type': 'text', 'text': 'A claim.'},
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
    final screen = t.state<WorkspaceScreenState>(find.byType(WorkspaceScreen));
    addTearDown(() async {
      await t.pumpWidget(const SizedBox());
      model.dispose();
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
      await until(
        () => done && find.byType(LinearProgressIndicator).evaluate().isEmpty,
      );
      if (error != null) throw error!;
      await t.pumpAndSettle();
    }

    List<Map> nodes(String type) {
      final out = <Map>[];
      void walk(Map n) {
        if (n['type'] == type) out.add(n);
        for (final c in n['content'] as List? ?? []) {
          walk(c as Map);
        }
      }

      walk(model.workspace['document'] as Map);
      return out;
    }

    Future<void> menu(String item) async {
      await t.tap(find.byKey(const Key('document-references')));
      await t.pumpAndSettle();
      await t.tap(find.text(item).last);
      await t.pumpAndSettle();
    }

    // "Method\n" is 7; the cursor after "A claim" is at 14.
    void cursor(int offset) => screen.editor.updateSelection(
      TextSelection.collapsed(offset: offset),
      q.ChangeSource.local,
    );

    // A footnote: its text, its mark, and editing it.
    cursor(14);
    await t.pump();
    await menu('Insert footnote');
    await t.enterText(find.byKey(const Key('note-text')), 'The source.');
    await t.tap(find.byKey(const Key('note-save')));
    await t.pumpAndSettle();
    await drive(screen.flush);
    await until(() => nodes('noteRef').isNotEmpty);
    expect(nodes('noteRef').single['attrs']['text'], 'The source.');
    expect(find.byKey(const Key('note-footnote-1')), findsOneWidget);
    await t.tap(find.byKey(const Key('note-footnote-1')));
    await t.pumpAndSettle();
    await t.enterText(find.byKey(const Key('note-text')), 'A better source.');
    await t.tap(find.byKey(const Key('note-save')));
    await t.pumpAndSettle();
    await drive(screen.flush);
    await until(
      () => nodes('noteRef').single['attrs']['text'] == 'A better source.',
    );

    // A caption after the paragraph.
    cursor(9);
    await t.pump();
    await menu('Insert caption…');
    await t.enterText(find.byKey(const Key('caption-text')), 'A map');
    await t.tap(find.byKey(const Key('caption-insert')));
    await until(
      () => (model.workspace['document']['content'] as List).length == 3,
    );
    await t.pumpAndSettle();
    expect(
      nodeText(model.workspace['document']['content'][2]),
      'Figure 1: A map',
    );

    // A cross-reference to the heading, at the end of the paragraph.
    cursor(15);
    await t.pump();
    await menu('Cross-reference…');
    await until(
      () => find.byKey(const Key('xref-item-0')).evaluate().isNotEmpty,
    );
    expect(find.text('Method'), findsWidgets);
    await t.tap(find.byKey(const Key('xref-insert')));
    await until(
      () => nodes('pageField').any((f) => f['attrs']['kind'] == 'REF'),
    );
    await t.pumpAndSettle();
    final reference = nodes(
      'pageField',
    ).firstWhere((f) => f['attrs']['kind'] == 'REF');
    expect(reference['text'], 'Method');
    expect(
      (model.workspace['document']['attrs']['bookmarks'] as List)
          .single['name'],
      startsWith('_Ref'),
    );

    // A table of contents at the top, with page numbers from the layout.
    cursor(0);
    await t.pump();
    await menu('Table of contents');
    await until(() => nodes('tableOfContents').isNotEmpty);
    await until(
      () =>
          (nodes('tableOfContents').single['attrs']['entries'] as List)
              .isNotEmpty &&
          nodes('tableOfContents').single['attrs']['entries'][0]['page'] == '1',
    );
    await t.pumpAndSettle();
    expect(find.byKey(const Key('table-of-contents')), findsOneWidget);
    expect(find.byKey(const Key('toc-update')), findsOneWidget);
    await t.pump(const Duration(seconds: 5));
  });
}
