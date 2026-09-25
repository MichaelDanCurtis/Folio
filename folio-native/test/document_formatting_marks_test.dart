import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_formatting_marks.dart';
import 'package:folio_native/src/document_view.dart';

void main() {
  test('marks for spaces, tabs, line breaks and paragraphs', () {
    final ops =
        (Delta()
              ..insert('a b\tc')
              ..insert({'lineBreak': '{}'})
              ..insert('d\n')
              ..insert({'folioNode': '{}'})
              ..insert('\n'))
            .toList();
    expect(formattingMarks(ops), [
      (mark: '·', offset: 1),
      (mark: '→', offset: 3),
      (mark: '↵', offset: 5),
      (mark: '¶', offset: 7),
    ]);
  });

  testWidgets('Show/Hide ¶ paints marks without changing the text', (t) async {
    final controller = q.QuillController(
      document: q.Document.fromDelta(Delta()..insert('one two\tthree\nnext\n')),
      selection: const TextSelection.collapsed(offset: 0),
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
    final before = controller.document.toDelta();
    t
        .widget<q.QuillEditor>(find.byType(q.QuillEditor))
        .focusNode
        .requestFocus();
    await t.pump();
    Future<void> toggle() async {
      await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      await t.sendKeyEvent(LogicalKeyboardKey.digit8);
      await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await t.pumpAndSettle();
    }

    expect(find.byKey(const Key('formatting-marks')), findsNothing);
    await toggle();
    final marks = t.renderObject<RenderFormattingMarks>(
      find.byKey(const Key('formatting-marks')),
    );
    expect(marks.painted.map((m) => m.mark), ['·', '→', '¶', '¶']);

    // Each mark sits between the characters around it.
    final raw = t.state<q.QuillRawEditorState>(find.byType(q.QuillRawEditor));
    final toMarks = raw.renderEditor.getTransformTo(marks);
    double x(int offset) => MatrixUtils.transformPoint(
      toMarks,
      raw.renderEditor
          .getLocalRectForCaret(TextPosition(offset: offset))
          .center,
    ).dx;
    expect(marks.painted[0].at.dx, greaterThan(x(3)));
    expect(marks.painted[0].at.dx, lessThan(x(4)));
    expect(marks.painted[2].at.dx, greaterThanOrEqualTo(x(13)));
    expect(marks.painted[3].at.dy, greaterThan(marks.painted[2].at.dy));

    // Marks are never part of the document, a copy, or a count.
    expect(controller.document.toDelta(), before);
    controller.updateSelection(
      const TextSelection(baseOffset: 0, extentOffset: 13),
      q.ChangeSource.local,
    );
    await t.pump();
    final copied = <String>[];
    t.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          copied.add((call.arguments as Map)['text'] as String);
        }
        return null;
      },
    );
    addTearDown(
      () => t.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await t.sendKeyEvent(LogicalKeyboardKey.keyC);
    await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await t.pumpAndSettle();
    expect(copied.last, 'one two\tthree');

    await t.tap(find.byKey(const Key('document-formatting-marks')));
    await t.pumpAndSettle();
    expect(find.byKey(const Key('formatting-marks')), findsNothing);
  });
}
