// Show/Hide ¶ (W17.07): paragraph marks, spaces, tabs and line breaks drawn
// over the editor. The marks are painted only, so they are never copied,
// saved or counted.
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;

class ToggleFormattingMarks extends Intent {
  const ToggleFormattingMarks();
}

const formattingMarkShortcuts = <ShortcutActivator, Intent>{
  SingleActivator(LogicalKeyboardKey.digit8, control: true, shift: true):
      ToggleFormattingMarks(),
  SingleActivator(LogicalKeyboardKey.digit8, meta: true, shift: true):
      ToggleFormattingMarks(),
  SingleActivator(LogicalKeyboardKey.asterisk, control: true, shift: true):
      ToggleFormattingMarks(),
  SingleActivator(LogicalKeyboardKey.asterisk, meta: true, shift: true):
      ToggleFormattingMarks(),
};

/// A mark and the document offset of the character it stands for.
typedef FormattingMark = ({String mark, int offset});

/// The marks for a document's [ops]: `·` for a space, `→` for a tab, `↵` for a line
/// break and `¶` at each paragraph's end. Embedded blocks get none.
List<FormattingMark> formattingMarks(List<Operation> ops) {
  final marks = <FormattingMark>[];
  var at = 0;
  for (final op in ops) {
    final data = op.data;
    if (data is String) {
      for (var i = 0; i < data.length; i++) {
        final mark = switch (data[i]) {
          ' ' || ' ' => '·',
          '\t' => '→',
          '\n' => '¶',
          _ => null,
        };
        if (mark != null) marks.add((mark: mark, offset: at + i));
      }
    } else if (data is Map && data.containsKey('lineBreak')) {
      marks.add((mark: '↵', offset: at));
    }
    at += op.length!;
  }
  // A preserved block's own line holds nothing to mark.
  final embeds = <int>{};
  at = 0;
  for (final op in ops) {
    if (op.data is Map && (op.data as Map).containsKey('folioNode')) {
      embeds.add(at + 1);
    }
    at += op.length!;
  }
  return marks
      .where((m) => !(m.mark == '¶' && embeds.contains(m.offset)))
      .toList();
}

/// Paints the formatting marks of [controller]'s document over the editor
/// found through [editorKey].
class FormattingMarksOverlay extends LeafRenderObjectWidget {
  final q.QuillController controller;
  final GlobalKey<q.EditorState> editorKey;
  final Color color;
  const FormattingMarksOverlay({
    super.key,
    required this.controller,
    required this.editorKey,
    this.color = const Color(0xff4a78c2),
  });
  @override
  RenderFormattingMarks createRenderObject(BuildContext context) =>
      RenderFormattingMarks(controller, editorKey, color);
  @override
  void updateRenderObject(
    BuildContext context,
    RenderFormattingMarks renderObject,
  ) {
    renderObject
      ..controller = controller
      ..editorKey = editorKey
      ..color = color;
  }
}

class RenderFormattingMarks extends RenderBox {
  RenderFormattingMarks(this._controller, this.editorKey, this.color);
  q.QuillController _controller;
  GlobalKey<q.EditorState> editorKey;
  Color color;

  /// What was painted last, in this box's coordinates; for tests.
  final painted = <({String mark, Offset at})>[];

  set controller(q.QuillController value) {
    if (identical(value, _controller)) return;
    if (attached) {
      _controller.removeListener(markNeedsPaint);
      value.addListener(markNeedsPaint);
    }
    _controller = value;
    markNeedsPaint();
  }

  @override
  void attach(PipelineOwner owner) {
    super.attach(owner);
    _controller.addListener(markNeedsPaint);
  }

  @override
  void detach() {
    _controller.removeListener(markNeedsPaint);
    super.detach();
  }

  @override
  bool get sizedByParent => true;
  @override
  Size computeDryLayout(BoxConstraints constraints) => constraints.biggest;
  @override
  bool hitTestSelf(Offset position) => false;

  @override
  void paint(PaintingContext context, Offset offset) {
    painted.clear();
    final editor = editorKey.currentState?.renderEditor;
    if (editor == null || !editor.attached || !editor.hasSize) return;
    final toHere = editor.getTransformTo(this);
    final canvas = context.canvas;
    for (final m in formattingMarks(_controller.document.toDelta().toList())) {
      final caret = editor.getLocalRectForCaret(TextPosition(offset: m.offset));
      final Offset at;
      if (m.mark == '·' || m.mark == '→') {
        final next = editor.getLocalRectForCaret(
          TextPosition(offset: m.offset + 1),
        );
        // A space that wraps has its end on the next line; mark its start.
        final end = (next.top - caret.top).abs() < 1
            ? next.left
            : caret.left + 4;
        at = Offset((caret.left + end) / 2, caret.center.dy);
      } else {
        at = Offset(caret.right + 2, caret.center.dy);
      }
      final local = MatrixUtils.transformPoint(toHere, at);
      final size = (caret.height * .6).clamp(6.0, 40.0);
      final text = TextPainter(
        text: TextSpan(
          text: m.mark,
          style: TextStyle(color: color, fontSize: size, height: 1),
        ),
        textDirection: TextDirection.ltr,
      )..layout();
      final origin = m.mark == '·' || m.mark == '→'
          ? local - Offset(text.width / 2, text.height / 2)
          : local - Offset(0, text.height / 2);
      text.paint(canvas, offset + origin);
      text.dispose();
      painted.add((mark: m.mark, at: local));
    }
  }
}
