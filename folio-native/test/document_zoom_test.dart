import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:flutter_test/flutter_test.dart';
import 'package:folio_native/src/document_view.dart';

void main() {
  testWidgets('zoom scales the page, keeps clicks on the right text, and '
      'fits the page', (t) async {
    t.view.physicalSize = const Size(1200, 900);
    t.view.devicePixelRatio = 1;
    addTearDown(t.view.resetPhysicalSize);
    addTearDown(t.view.resetDevicePixelRatio);
    final controller = q.QuillController(
      document: q.Document.fromDelta(
        Delta()..insert('Zoomed text keeps its caret\n'),
      ),
      selection: const TextSelection.collapsed(offset: 0),
    );
    addTearDown(controller.dispose);
    final zoom = ValueNotifier<int>(100);
    addTearDown(zoom.dispose);
    Widget app(Key key) => MaterialApp(
      localizationsDelegates: const [q.FlutterQuillLocalizations.delegate],
      home: Scaffold(
        body: DocumentView(
          key: key,
          controller: controller,
          undo: () {},
          redo: () {},
          zoom: zoom,
        ),
      ),
    );
    await t.pumpWidget(app(const Key('first')));
    await t.pumpAndSettle();
    final paper = find.byKey(const Key('document-paper'));
    final width = t.getSize(paper).width;
    double shown() => t.getRect(find.byKey(const Key('document-zoom'))).width;
    expect(shown(), width);

    t
        .widget<q.QuillEditor>(find.byType(q.QuillEditor))
        .focusNode
        .requestFocus();
    await t.pump();
    Future<void> key(LogicalKeyboardKey k) async {
      await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await t.sendKeyEvent(k);
      await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await t.pumpAndSettle();
    }

    await key(LogicalKeyboardKey.equal);
    expect(zoom.value, 125);
    await key(LogicalKeyboardKey.equal);
    await key(LogicalKeyboardKey.equal);
    expect(zoom.value, 200);
    expect(shown(), closeTo(width * 2, .5));

    // The caret's on-screen rectangles are scaled, and a click lands on the
    // character drawn there.
    final raw = t.state<q.QuillRawEditorState>(find.byType(q.QuillRawEditor));
    final box = t.renderObject<RenderBox>(find.byType(q.QuillRawEditor));
    Offset caret(int offset) => box.localToGlobal(
      raw.renderEditor
          .getLocalRectForCaret(TextPosition(offset: offset))
          .center,
    );
    final at100 =
        raw.renderEditor
            .getLocalRectForCaret(const TextPosition(offset: 12))
            .left -
        raw.renderEditor
            .getLocalRectForCaret(const TextPosition(offset: 0))
            .left;
    expect(caret(12).dx - caret(0).dx, closeTo(at100 * 2, .5));
    await t.tapAt(caret(12));
    await t.pumpAndSettle();
    expect(controller.selection, const TextSelection.collapsed(offset: 12));

    final view = t.state<DocumentViewState>(find.byType(DocumentView));
    view.setZoom(5);
    await t.pump();
    expect(zoom.value, 10);
    view.setZoom(900);
    await t.pump();
    expect(zoom.value, 500);
    for (var i = 0; i < 12; i++) {
      await key(LogicalKeyboardKey.minus);
    }
    expect(zoom.value, 10);

    view.zoomToPageWidth();
    await t.pumpAndSettle();
    final frame = t.getSize(find.byType(DocumentView));
    expect(shown(), lessThanOrEqualTo(frame.width - 48));
    expect(shown(), greaterThan(frame.width - 48 - width / 100));
    view.zoomToWholePage();
    await t.pumpAndSettle();
    final whole = zoom.value;
    expect(whole, lessThan(100));

    // A new view of the same document starts at its zoom for the session.
    zoom.value = 100;
    await t.pumpWidget(app(const Key('second')));
    await t.pumpAndSettle();
    expect(zoom.value, whole);
  });
}
