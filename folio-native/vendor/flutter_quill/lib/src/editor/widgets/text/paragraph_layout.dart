import 'dart:ui' as ui;
import 'package:flutter/widgets.dart';
import 'package:flutter/rendering.dart';
import '../../../document/nodes/line.dart';
import '../../../document/nodes/block.dart';
import '../../../common/structs/vertical_spacing.dart';

/// Application-owned, character-preserving layout of a fully styled paragraph.
/// Called during layout with the actual available width, after composing and
/// diagnostic spans have been built. Never change the source text or offsets.
typedef QuillParagraphLayoutBuilder =
    InlineSpan Function(
      BuildContext context,
      Line line,
      InlineSpan source,
      double width,
    );

class QuillParagraphLayout extends InheritedWidget {
  const QuillParagraphLayout({
    required this.builder,
    this.paddingBuilder,
    this.styleBuilder,
    this.inlineStyleBuilder,
    this.alignmentBuilder,
    this.strutBuilder,
    this.blockSpacingBuilder,
    required super.child,
    super.key,
  });

  final QuillParagraphLayoutBuilder builder;
  final EdgeInsets Function(BuildContext, Line, EdgeInsets)? paddingBuilder;
  final TextStyle Function(BuildContext, Line, TextStyle)? styleBuilder;
  final TextStyle Function(BuildContext, Line, Map<String, dynamic>, TextStyle)?
  inlineStyleBuilder;
  final TextAlign Function(BuildContext, Line, TextAlign)? alignmentBuilder;
  final StrutStyle Function(BuildContext, Line, StrutStyle)? strutBuilder;
  final VerticalSpacing Function(BuildContext, Block, VerticalSpacing)?
  blockSpacingBuilder;

  static QuillParagraphLayout? scopeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<QuillParagraphLayout>();

  static QuillParagraphLayoutBuilder? maybeOf(BuildContext context) => context
      .dependOnInheritedWidgetOfExactType<QuillParagraphLayout>()
      ?.builder;

  @override
  bool updateShouldNotify(QuillParagraphLayout oldWidget) =>
      oldWidget.builder != builder ||
      oldWidget.paddingBuilder != paddingBuilder ||
      oldWidget.styleBuilder != styleBuilder ||
      oldWidget.inlineStyleBuilder != inlineStyleBuilder ||
      oldWidget.alignmentBuilder != alignmentBuilder ||
      oldWidget.strutBuilder != strutBuilder ||
      oldWidget.blockSpacingBuilder != blockSpacingBuilder;
}

/// A single source character drawn as a spacer, with no inline child widget.
abstract interface class QuillInlineSpacerSpan {}

/// Paint associated with a source character without adding text, semantics or
/// inline widget children. Bounds are the final shaped character rectangle.
abstract interface class QuillInlinePaintSpan {
  bool get hasInlinePaint;
  void paintInline(Canvas canvas, Rect bounds, TextScaler scaler);
}

/// RichText with explicit support for non-widget inline spacers. All text
/// painting, selection, semantics and actual inline widgets stay in Flutter.
class QuillLayoutText extends RichText {
  QuillLayoutText({
    required super.text,
    super.textAlign,
    super.textDirection,
    super.strutStyle,
    super.textScaler,
    super.key,
  });

  @override
  RenderParagraph createRenderObject(BuildContext context) => _LayoutParagraph(
    text,
    textAlign: textAlign,
    textDirection: textDirection ?? Directionality.of(context),
    softWrap: softWrap,
    overflow: overflow,
    textScaler: textScaler,
    maxLines: maxLines,
    strutStyle: strutStyle,
    textWidthBasis: textWidthBasis,
    textHeightBehavior: textHeightBehavior,
    locale: locale ?? Localizations.maybeLocaleOf(context),
    registrar: selectionRegistrar,
    selectionColor: selectionColor,
  );
}

class _LayoutParagraph extends RenderParagraph {
  _LayoutParagraph(
    super.text, {
    required super.textDirection,
    super.textAlign,
    super.softWrap,
    super.overflow,
    super.textScaler,
    super.maxLines,
    super.strutStyle,
    super.textWidthBasis,
    super.textHeightBehavior,
    super.locale,
    super.registrar,
    super.selectionColor,
  });

  ui.Picture? _inlinePaint;
  InlineSpan? _paintedText;

  void _clearInlinePaint() {
    _inlinePaint?.dispose();
    _inlinePaint = null;
    _paintedText = null;
  }

  @override
  void performLayout() {
    _clearInlinePaint();
    super.performLayout();
  }

  @override
  void dispose() {
    _clearInlinePaint();
    super.dispose();
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    super.paint(context, offset);
    if (_paintedText != text) {
      _clearInlinePaint();
      final decorations = <({QuillInlinePaintSpan span, int start, int end})>[];
      var position = 0;
      text.visitChildren((span) {
        final length = span is TextSpan ? (span.text?.length ?? 0) : 1;
        if (span is QuillInlinePaintSpan &&
            (span as QuillInlinePaintSpan).hasInlinePaint &&
            length > 0) {
          decorations.add((
            span: span as QuillInlinePaintSpan,
            start: position,
            end: position + length,
          ));
        }
        position += length;
        return true;
      });
      if (decorations.isNotEmpty) {
        final recorder = ui.PictureRecorder();
        final canvas = Canvas(recorder)..clipRect(Offset.zero & size);
        for (final decoration in decorations) {
          final boxes = getBoxesForSelection(
            TextSelection(
              baseOffset: decoration.start,
              extentOffset: decoration.end,
            ),
          );
          for (final box in boxes) {
            decoration.span.paintInline(canvas, box.toRect(), textScaler);
          }
        }
        _inlinePaint = recorder.endRecording();
      }
      _paintedText = text;
    }
    // Cache the decoration picture until text or layout changes. A caret blink
    // must not reshape hundreds of leader glyphs or leak native paragraphs.
    if (_inlinePaint != null) {
      context.canvas.save();
      context.canvas.translate(offset.dx, offset.dy);
      context.canvas.drawPicture(_inlinePaint!);
      context.canvas.restore();
    }
  }

  @override
  void positionInlineChildren(List<ui.TextBox> boxes) {
    final widgets = <bool>[];
    text.visitChildren((span) {
      if (span is QuillInlineSpacerSpan) {
        widgets.add(false);
      } else if (span is PlaceholderSpan) {
        widgets.add(true);
      }
      return true;
    });
    if (widgets.length != boxes.length) {
      // Keep Flutter's checks for malformed ordinary inline children.
      super.positionInlineChildren(boxes);
      return;
    }
    super.positionInlineChildren([
      for (var i = 0; i < boxes.length; i++)
        if (widgets[i]) boxes[i],
    ]);
  }

  @override
  TextPosition getPositionForOffset(Offset offset) {
    final position = super.getPositionForOffset(offset);
    if (text.getSpanForPosition(position) is! QuillInlineSpacerSpan) {
      return position;
    }
    final boxes = getBoxesForSelection(
      TextSelection(
        baseOffset: position.offset,
        extentOffset: position.offset + 1,
      ),
    );
    for (final box in boxes) {
      if (!box.toRect().contains(offset)) continue;
      final right = offset.dx >= (box.left + box.right) / 2;
      final after = box.direction == TextDirection.ltr ? right : !right;
      return TextPosition(offset: position.offset + (after ? 1 : 0));
    }
    return position;
  }
}
