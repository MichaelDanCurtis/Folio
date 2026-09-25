import 'dart:convert';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_view.dart';
import 'package:folio_native/src/format_painter.dart';

void main() {
  test('captures and paints character and paragraph formatting', () {
    final token = jsonEncode({
      'officeId': 'source',
      'paragraphFormat': {
        'direct': {'left': 720},
      },
    });
    final doc = Delta()
      ..insert('Loud', {'bold': true, 'color': '#c00000'})
      ..insert('\n', {'align': 'center', 'token': token})
      ..insert('quiet ', {'italic': true})
      ..insert('words')
      ..insert('\n', {
        'token': jsonEncode({'officeId': 'target'}),
      });
    final format = captureFormat(
      doc,
      const TextSelection(baseOffset: 0, extentOffset: 2),
    );
    expect(format.inline['bold'], true);
    expect(format.inline['italic'], isNull);
    final after = doc
        .compose(
          paintFormat(
            doc,
            const TextSelection(baseOffset: 5, extentOffset: 16),
            format,
          ),
        )
        .toList();
    expect(after[2].data, 'quiet words');
    expect(after[2].attributes, {'bold': true, 'color': '#c00000'});
    expect(after[3].attributes!['align'], 'center');
    expect(jsonDecode(after[3].attributes!['token']), {
      'officeId': 'target',
      'paragraphFormat': {
        'direct': {'left': 720},
      },
    });
  });

  testWidgets('one paint, or many after a double-click until Esc', (t) async {
    final controller = q.QuillController(
      document: q.Document.fromDelta(
        Delta()
          ..insert('Loud', {'bold': true})
          ..insert(' then plain words here\n'),
      ),
      selection: const TextSelection.collapsed(offset: 2),
    );
    addTearDown(controller.dispose);
    await t.pumpWidget(
      MaterialApp(
        localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
        home: Scaffold(
          body: DocumentView(controller: controller, undo: () {}, redo: () {}),
        ),
      ),
    );
    await t.pumpAndSettle();
    final view = t.state<DocumentViewState>(find.byType(DocumentView));
    final raw = t.state<q.QuillRawEditorState>(find.byType(q.QuillRawEditor));
    final box = t.renderObject<RenderBox>(find.byType(q.QuillRawEditor));
    Offset at(int offset) => box.localToGlobal(
      raw.renderEditor
          .getLocalRectForCaret(TextPosition(offset: offset))
          .center,
    );
    bool bold(int offset) =>
        controller.document.collectStyle(offset, 1).attributes['bold']?.value ==
        true;
    Future<void> click(int offset) async {
      await t.tapAt(at(offset));
      await t.pump(kDoubleTapTimeout);
      await t.pumpAndSettle();
    }

    t
        .widget<q.QuillEditor>(find.byType(q.QuillEditor))
        .focusNode
        .requestFocus();
    await t.pump();
    await t.tap(find.byKey(const Key('document-format-painter')));
    await t.pump(kDoubleTapTimeout);
    await t.pumpAndSettle();
    expect(view.painter, isNotNull);
    await click(12); // "plain"
    expect(bold(10), true);
    expect(bold(14), true);
    expect(bold(16), false);
    expect(view.painter, isNull);
    controller.undo();
    expect(bold(12), false);

    await click(2);
    await t.tap(find.byKey(const Key('document-format-painter')));
    await t.pump(const Duration(milliseconds: 50));
    await t.tap(find.byKey(const Key('document-format-painter')));
    await t.pumpAndSettle();
    expect(view.painterSticky, true);
    await click(12);
    await click(18); // "words"
    expect(bold(12) && bold(18), true);
    expect(view.painter, isNotNull);
    await t.sendKeyEvent(LogicalKeyboardKey.escape);
    await t.pumpAndSettle();
    expect(view.painter, isNull);
    await click(24); // "here" stays plain
    expect(bold(24), false);
  });
}
