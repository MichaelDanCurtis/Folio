import 'document_paragraph_format.dart';
import 'dart:math' as math;
import 'dart:ui' as ui;
import 'document_lists.dart';
import 'document_tab_leaders.dart';
import 'document_tab_settings.dart';
export 'document_tab_settings.dart';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;

/// Layout never changes the underlying document or the number of characters.
InlineSpan layoutDocumentTabs(
  BuildContext context,
  q.Line line,
  InlineSpan source,
  double width,
) {
  final p = lineParagraphProperties(line);
  final origin = math.max(
    0.0,
    ((p['start'] ?? p['left'] ?? 0) as num).toDouble() / 15,
  );
  return layoutTabStops(
    source,
    origin: origin,
    maxWidth: width,
    interval: DocumentTabSettings.intervalOf(context) / 15,
    stops: effectiveTabStops(
      listMetadata(line.style.attributes['token']?.value)['tabs'],
    ),
    textDirection: Directionality.of(context),
    textScaler: MediaQuery.textScalerOf(context),
    locale: Localizations.maybeLocaleOf(context),
  );
}

/// The paragraph shaper receives one measured placeholder for this one tab.
/// Text traversal, selection, semantics and Quill still see literal U+0009.
/// There is no child widget and no replacement of tabs with spaces or embeds.
class _TabSpan extends TextSpan
    implements q.QuillInlineSpacerSpan, q.QuillInlinePaintSpan {
  _TabSpan(
    TextSpan original,
    this.advance,
    this.height,
    this.baseline, [
    this.decoration,
  ]) : super(
         text: '\t',
         style: original.style,
         recognizer: original.recognizer,
         mouseCursor: original.mouseCursor,
         onEnter: original.onEnter,
         onExit: original.onExit,
         locale: original.locale,
         spellOut: original.spellOut,
       );
  final double advance, height, baseline;
  final TabLeaderDecoration? decoration;
  @override
  bool get hasInlinePaint => decoration != null;
  @override
  void paintInline(Canvas canvas, Rect bounds, TextScaler scaler) =>
      decoration?.paint(canvas, bounds, scaler);

  @override
  void build(
    ui.ParagraphBuilder builder, {
    TextScaler textScaler = TextScaler.noScaling,
    List<PlaceholderDimensions>? dimensions,
  }) {
    if (style != null) {
      builder.pushStyle(style!.getTextStyle(textScaler: textScaler));
    }
    builder.addPlaceholder(
      advance,
      height,
      ui.PlaceholderAlignment.baseline,
      baseline: TextBaseline.alphabetic,
      baselineOffset: baseline,
    );
    if (style != null) builder.pop();
  }

  @override
  RenderComparison compareTo(InlineSpan other) {
    final comparison = super.compareTo(other);
    if (other is _TabSpan) {
      if (advance != other.advance ||
          height != other.height ||
          baseline != other.baseline) {
        return RenderComparison.layout;
      }
      if (decoration != other.decoration &&
          comparison.index < RenderComparison.paint.index) {
        return RenderComparison.paint;
      }
    }
    return comparison;
  }

  @override
  bool operator ==(Object other) =>
      other is _TabSpan &&
      super == other &&
      advance == other.advance &&
      height == other.height &&
      baseline == other.baseline &&
      decoration == other.decoration;
  @override
  int get hashCode =>
      Object.hash(super.hashCode, advance, height, baseline, decoration);
}

// RTL keeps its native glyph and spacing so Flutter's bidi caret rectangles
// remain correct. Painting uses the same character decoration as LTR spacers.
class _TabGlyphSpan extends TextSpan implements q.QuillInlinePaintSpan {
  _TabGlyphSpan(TextSpan original, TextStyle style, this.decoration)
    : super(
        text: '\t',
        style: style,
        recognizer: original.recognizer,
        mouseCursor: original.mouseCursor,
        onEnter: original.onEnter,
        onExit: original.onExit,
        locale: original.locale,
        spellOut: original.spellOut,
      );
  final TabLeaderDecoration? decoration;
  @override
  bool get hasInlinePaint => decoration != null;
  @override
  void paintInline(Canvas canvas, Rect bounds, TextScaler scaler) =>
      decoration?.paint(canvas, bounds, scaler);
  @override
  RenderComparison compareTo(InlineSpan other) {
    final comparison = super.compareTo(other);
    if (other is _TabGlyphSpan &&
        decoration != other.decoration &&
        comparison.index < RenderComparison.paint.index) {
      return RenderComparison.paint;
    }
    return comparison;
  }

  @override
  bool operator ==(Object other) =>
      other is _TabGlyphSpan &&
      super == other &&
      decoration == other.decoration;
  @override
  int get hashCode => Object.hash(super.hashCode, decoration);
}

InlineSpan layoutTabStops(
  InlineSpan source, {
  required double maxWidth,
  double interval = 48,
  double origin = 0,
  List<DocumentTabStop> stops = const [],
  TextDirection textDirection = TextDirection.ltr,
  TextScaler textScaler = TextScaler.noScaling,
  Locale? locale,
}) {
  final plain = source.toPlainText(includeSemanticsLabels: false);
  if (!plain.contains('\t') ||
      !maxWidth.isFinite ||
      maxWidth <= 0 ||
      !interval.isFinite ||
      interval <= 0) {
    return source;
  }
  final tabs = <int, double>{};
  final leaders = <int, String>{};
  final styles = <int, TextStyle>{};
  final segments = <int, List<InlineSpan>>{};
  final pending = <InlineSpan>[];
  var offset = 0;
  var supported = true;
  void collect(InlineSpan span, TextStyle inherited) {
    if (span is! TextSpan || span.semanticsLabel != null) {
      supported = false;
      return;
    }
    final style = inherited.merge(span.style);
    final text = span.text ?? '';
    var from = 0;
    for (var i = 0; i < text.length; i++) {
      if (text.codeUnitAt(i) == 9) {
        tabs[offset + i] = 0;
        styles[offset + i] = style;
        if (i > from) {
          pending.add(TextSpan(text: text.substring(from, i), style: style));
        }
        segments[offset + i] = List.of(pending);
        pending.clear();
        from = i + 1;
      }
    }
    if (from < text.length) {
      pending.add(TextSpan(text: text.substring(from), style: style));
    }
    offset += text.length;
    for (final child in span.children ?? <InlineSpan>[]) {
      collect(child, style);
    }
  }

  collect(source, const TextStyle());
  // Inline widgets require their actual laid-out dimensions. Do not guess them.
  if (!supported) return source;

  final metrics = <TextStyle, ({double height, double baseline})>{};
  for (final style in styles.values) {
    if (metrics.containsKey(style)) continue;
    final measure = TextPainter(
      text: TextSpan(text: ' ', style: style),
      textDirection: textDirection,
      textScaler: textScaler,
      locale: locale,
    )..layout();
    metrics[style] = (
      height: measure.height,
      baseline: measure.computeDistanceToActualBaseline(
        TextBaseline.alphabetic,
      ),
    );
    measure.dispose();
  }

  InlineSpan build() {
    var position = 0;
    TextSpan copy(
      TextSpan span,
      String? text,
      List<InlineSpan>? children, [
      TextStyle? overrideStyle,
    ]) => TextSpan(
      text: text,
      children: children,
      style: overrideStyle ?? span.style,
      recognizer: span.recognizer,
      mouseCursor: span.mouseCursor,
      onEnter: span.onEnter,
      onExit: span.onExit,
      locale: span.locale,
      spellOut: span.spellOut,
    );
    InlineSpan visit(InlineSpan raw) {
      final span = raw as TextSpan;
      final text = span.text ?? '';
      final start = position;
      position += text.length;
      final nested = (span.children ?? <InlineSpan>[]).map(visit).toList();
      if (!text.contains('\t')) return copy(span, span.text, nested);
      final children = <InlineSpan>[];
      var from = 0;
      for (var i = 0; i < text.length; i++) {
        if (text.codeUnitAt(i) != 9) continue;
        if (i > from) children.add(copy(span, text.substring(from, i), null));
        final metric = metrics[styles[start + i]]!;
        final leader = leaders[start + i] ?? 'none';
        final decoration =
            leader != 'none' && tabLeaderNames.containsKey(leader)
            ? TabLeaderDecoration(leader, styles[start + i]!, metric.baseline)
            : null;
        children.add(
          textDirection == TextDirection.rtl
              ? _TabGlyphSpan(
                  span,
                  (span.style ?? const TextStyle()).copyWith(
                    letterSpacing: tabs[start + i],
                  ),
                  decoration,
                )
              : _TabSpan(
                  span,
                  tabs[start + i]!,
                  metric.height,
                  metric.baseline,
                  decoration,
                ),
        );
        from = i + 1;
      }
      if (from < text.length) {
        children.add(copy(span, text.substring(from), null));
      }
      children.addAll(nested);
      return copy(span, null, children);
    }

    return visit(source);
  }

  final custom =
      stops
          .map(
            (s) => DocumentTabStop(
              s.position - (origin * 15).round(),
              alignment: s.alignment,
              leader: s.leader,
            ),
          )
          .where(
            (s) => [
              'left',
              'right',
              'center',
              'decimal',
              'start',
              'end',
              'num',
            ].contains(s.alignment),
          )
          .toList()
        ..sort((a, b) => a.position.compareTo(b.position));
  final tabOffsets = tabs.keys.toList();
  double followingWidth(int at, String alignment) {
    if (['left', 'start', 'num'].contains(alignment)) return 0;
    final i = tabOffsets.indexOf(at);
    // The field ends at the next tab or line break, whichever comes first;
    // text after a break belongs to the next line, not this stop.
    final field = <InlineSpan>[];
    for (final span
        in i + 1 < tabOffsets.length ? segments[tabOffsets[i + 1]]! : pending) {
      final text = (span as TextSpan).text ?? '';
      final end = text.indexOf('\n');
      if (end < 0) {
        field.add(span);
        continue;
      }
      if (end > 0) {
        field.add(TextSpan(text: text.substring(0, end), style: span.style));
      }
      break;
    }
    final m = TextPainter(
      text: TextSpan(style: source.style, children: field),
      textDirection: textDirection,
      textScaler: textScaler,
      locale: locale,
    )..layout();
    try {
      if (alignment == 'center') return m.width / 2;
      if (alignment == 'decimal') {
        final fieldText = field.map((s) => s.toPlainText()).join();
        final comma = const {
          'de',
          'fr',
          'es',
          'it',
          'pt',
          'nl',
          'ru',
          'pl',
          'da',
          'fi',
          'sv',
          'no',
        }.contains(locale?.languageCode);
        final decimal = fieldText.indexOf(comma ? ',' : '.');
        if (decimal >= 0) {
          final caret = m
              .getOffsetForCaret(TextPosition(offset: decimal), Rect.zero)
              .dx;
          return textDirection == TextDirection.rtl ? m.width - caret : caret;
        }
      }
      return m.width;
    } finally {
      m.dispose();
    }
  }

  final painter = TextPainter(
    textDirection: textDirection,
    textScaler: textScaler,
    locale: locale,
  );
  try {
    // Tabs divide text into independent wrapping segments. Measure each segment
    // once, using a temporary leading spacer for its starting position. This
    // avoids reshaping a whole long paragraph separately for every tab.
    var advance = 0.0;
    for (final at in tabs.keys.toList()) {
      final metric = metrics[styles[at]]!;
      final pieces = segments[at]!;
      painter.text = TextSpan(
        style: source.style,
        children: [
          if (textDirection == TextDirection.rtl)
            TextSpan(
              text: '\t',
              style: TextStyle(fontSize: 0, letterSpacing: advance),
            )
          else
            _TabSpan(const TextSpan(), advance, 0, 0),
          ...pieces,
          if (textDirection == TextDirection.rtl)
            TextSpan(text: '\t', style: styles[at]!.copyWith(letterSpacing: 0))
          else
            _TabSpan(const TextSpan(), 0, metric.height, metric.baseline),
        ],
      );
      painter.layout(
        minWidth: textDirection == TextDirection.rtl ? maxWidth : 0,
        maxWidth: maxWidth,
      );
      final index =
          1 +
          pieces.fold<int>(
            0,
            (n, span) =>
                n + span.toPlainText(includeSemanticsLabels: false).length,
          );
      final boxes = painter.getBoxesForSelection(
        TextSelection(baseOffset: index, extentOffset: index + 1),
      );
      if (boxes.isEmpty) continue;
      final box = boxes.first;
      final x = textDirection == TextDirection.ltr
          ? box.left
          : maxWidth - box.right;
      final stop = custom.where((s) => s.pixels > x + .01).firstOrNull;
      leaders[at] = stop?.leader ?? 'none';
      final target =
          stop?.pixels ??
          (((x + origin + .01) / interval).floor() * interval +
              interval -
              origin);
      final aligned =
          target - (stop == null ? 0 : followingWidth(at, stop.alignment));
      final next = math.max(x, math.min(aligned, maxWidth));
      final gap = math.max(0.0, next - x);
      tabs[at] = textDirection == TextDirection.rtl
          ? gap - (box.right - box.left)
          : gap;
      advance = next;
    }
    final result = build();
    assert(result.toPlainText(includeSemanticsLabels: false) == plain);
    return result;
  } finally {
    painter.dispose();
  }
}
