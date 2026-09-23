import 'dart:math' as math;
import 'package:flutter/foundation.dart' show listEquals, mapEquals;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:intl/intl.dart' show Bidi;
import 'workbook_block.dart' show parseCell;
import 'sheet_number_width.dart';

const sheetFontDefaults = <String, dynamic>{
  'fontName': 'Aptos',
  'fontSize': 11.0,
  'bold': false,
  'italic': false,
  'strike': false,
  'underline': 'none',
  'verticalAlign': 'baseline',
  'color': 'automatic',
};

const sheetAlignmentDefaults = <String, dynamic>{
  'horizontalAlignment': 'general',
  'verticalAlignment': 'bottom',
  'wrapText': false,
  'shrinkToFit': false,
  'indent': 0,
  'textRotation': 0,
  'readingOrder': 0,
  'justifyLastLine': false,
};

Map<String, dynamic> commonSheetAlignment(Iterable<Map> styles) {
  final values = styles.map((s) => {...sheetAlignmentDefaults, ...s}).toList();
  return {
    for (final key in sheetAlignmentDefaults.keys)
      key: values.isEmpty
          ? sheetAlignmentDefaults[key]
          : values.every((s) => s[key] == values.first[key])
          ? values.first[key]
          : null,
  };
}

class CellTextLayout {
  final Map<String, dynamic> style;
  final Map<String, dynamic> normalFont;
  final bool number, boolean;
  final num? numericValue;
  final String? numberWidth;
  CellTextLayout(Map style, {dynamic value, Map normalFont = const {}})
    : style = Map.unmodifiable({
        for (final key in sheetAlignmentDefaults.keys)
          key: style[key] ?? sheetAlignmentDefaults[key],
      }),
      normalFont = Map.unmodifiable({
        for (final key in sheetFontDefaults.keys)
          key: normalFont[key] ?? sheetFontDefaults[key],
      }),
      numericValue = value is num ? value : null,
      numberWidth = style['_numberWidth'] as String?,
      number = value is num,
      boolean = value is bool;
  @override
  bool operator ==(Object other) =>
      other is CellTextLayout &&
      mapEquals(style, other.style) &&
      mapEquals(normalFont, other.normalFont) &&
      numericValue == other.numericValue &&
      numberWidth == other.numberWidth &&
      number == other.number &&
      boolean == other.boolean;
  @override
  int get hashCode => Object.hash(
    number,
    boolean,
    numericValue,
    numberWidth,
    Object.hashAll(style.values),
    Object.hashAll(normalFont.values),
  );
}

Map<String, dynamic> cellStyle(Map sheet, String address) {
  final defaults = sheet['styleDefaults'] as Map? ?? {};
  final (row, column) = parseCell(address);
  Map base = defaults['default'] as Map? ?? {};
  for (final c in defaults['columns'] as List? ?? []) {
    if (column + 1 >= c['min'] && column + 1 <= c['max']) {
      base = c['style'] as Map;
    }
  }
  base = defaults['rows']?['${row + 1}'] as Map? ?? base;
  // An explicit source XF replaces inherited formatting. A newly authored
  // partial cell style instead overrides only its specified properties.
  if (sheet['officeStyles']?[address] != null) {
    base = defaults['default'] as Map? ?? {};
  }
  return {
    ...Map<String, dynamic>.from(base),
    ...Map<String, dynamic>.from(sheet['cellStyles']?[address] as Map? ?? {}),
  };
}

Map<String, dynamic> normalizedSheetFont(Map style) => {
  ...sheetFontDefaults,
  ...Map<String, dynamic>.from(style),
};

Map<String, dynamic> normalSheetFont(Map sheet) => normalizedSheetFont(
  sheet['styleDefaults']?['normalFont'] as Map? ??
      sheet['styleDefaults']?['default'] as Map? ??
      {},
);

class SheetFontRun {
  final String text;
  final Map<String, dynamic> font;
  SheetFontRun(this.text, Map font)
    : font = Map.unmodifiable({
        for (final key in sheetFontDefaults.keys)
          key: font[key] ?? sheetFontDefaults[key],
        if (font['_numberFormatSkip'] == true) '_numberFormatSkip': true,
        if (font['_numberFormatFill'] == true) '_numberFormatFill': true,
      });
  @override
  bool operator ==(Object other) =>
      other is SheetFontRun &&
      text == other.text &&
      mapEquals(font, other.font);
  @override
  int get hashCode => Object.hash(text, Object.hashAll(font.values));
}

/// Imported runs belong to the original text. A changed value or formula
/// result must never inherit stale character positions from that text.
List<SheetFontRun> sheetFontRuns(String text, Map style) {
  final overrides = {
    ...style['_fontOverrides'] as Map? ?? const {},
    if (style['_numberFormatColor'] != null)
      'color': style['_numberFormatColor'],
    for (final key in sheetFontDefaults.keys)
      if ((style['_conditionalStyle'] as Map?)?.containsKey(key) == true)
        key: style['_conditionalStyle'][key],
  };
  final base = {...normalizedSheetFont(style), ...overrides};
  final segments = style['_numberFormatSegments'];
  if (segments is List && segments.isNotEmpty) {
    return [
      for (final segment in segments)
        SheetFontRun(segment['text'], {
          ...base,
          if (segment['kind'] == 'skip') '_numberFormatSkip': true,
          if (segment['kind'] == 'fill') '_numberFormatFill': true,
          if (segment['kind'] != 'text') 'underline': 'none',
        }),
    ];
  }
  final runs = style['_richText'];
  if (runs is List &&
      runs.isNotEmpty &&
      runs.every((r) => r is Map && r['text'] is String && r['style'] is Map) &&
      runs.map((r) => r['text']).join() == text) {
    return [
      for (final r in runs)
        if ((r['text'] as String).isNotEmpty)
          SheetFontRun(r['text'], {...base, ...r['style'], ...overrides}),
    ];
  }
  return [SheetFontRun(text, base)];
}

Map<String, dynamic> commonSheetFont(Iterable<Map> fonts) {
  final iterator = fonts.iterator;
  if (!iterator.moveNext()) return Map.of(sheetFontDefaults);
  final result = <String, dynamic>{
    for (final key in sheetFontDefaults.keys) key: iterator.current[key],
  };
  while (iterator.moveNext()) {
    for (final key in result.keys) {
      if (result[key] != iterator.current[key]) result[key] = null;
    }
  }
  return result;
}

List<SheetFontRun> selectedSheetFontRuns(
  List<SheetFontRun> runs,
  TextSelection selection,
) {
  var offset = 0;
  final selected = <SheetFontRun>[];
  for (final run in runs) {
    final start = math.max(0, selection.start - offset);
    final end = math.min(run.text.length, selection.end - offset);
    if (start < end) {
      selected.add(SheetFontRun(run.text.substring(start, end), run.font));
    }
    offset += run.text.length;
  }
  return selected;
}

Map<String, dynamic> previewCharacterFont(
  String text,
  Map source,
  TextSelection selection,
  Map changes,
) {
  final runs = sheetFontRuns(text, source);
  final parts = <Map<String, dynamic>>[];
  var offset = 0;
  for (final run in runs) {
    final start = offset, end = offset + run.text.length;
    final cuts = {
      start,
      end,
      selection.start.clamp(start, end),
      selection.end.clamp(start, end),
    }.toList()..sort();
    for (var i = 0; i < cuts.length - 1; i++) {
      final a = cuts[i], b = cuts[i + 1];
      parts.add({
        'text': run.text.substring(a - start, b - start),
        'style': {
          ...run.font,
          if (selection.start <= a && b <= selection.end) ...changes,
        },
      });
    }
    offset = end;
  }
  return {
    ...Map<String, dynamic>.from(source),
    '_richText': parts,
    '_fontOverrides': <String, dynamic>{},
  };
}

Color? sheetFontColor(dynamic value) {
  if (value is! String || !RegExp(r'^#[0-9a-fA-F]{6}$').hasMatch(value)) {
    return null;
  }
  return Color(0xff000000 | int.parse(value.substring(1), radix: 16));
}

String fontPoints(num size) =>
    size == size.roundToDouble() ? size.toInt().toString() : size.toString();

TextStyle sheetTextStyle(
  Map font, {
  Color color = const Color(0xff242424),
  bool bold = false,
}) {
  final points = font['fontSize'] is num
      ? (font['fontSize'] as num).toDouble()
      : 11.0;
  return TextStyle(
    fontFamily: font['fontName'] as String? ?? 'Aptos',
    fontFamilyFallback: const ['Calibri', 'Arial'],
    fontSize:
        (points.isFinite && points > 0 ? points.clamp(1.0, 409.0) : 11.0) *
        4 /
        3,
    fontWeight: font['bold'] == true || bold
        ? FontWeight.w700
        : FontWeight.normal,
    fontStyle: font['italic'] == true ? FontStyle.italic : FontStyle.normal,
    decoration: font['strike'] == true
        ? TextDecoration.lineThrough
        : TextDecoration.none,
    color: sheetFontColor(font['color']) ?? color,
  );
}

/// Each cell shares shaping across its runs. Script groups use identical
/// paragraphs with transparent non-group text, avoiding clipped italic glyphs
/// and preserving ligatures/bidi ordering across character-style boundaries.
class SheetFontText extends LeafRenderObjectWidget {
  final String text;
  final Map font;
  final Color color;
  final bool bold;
  final CellTextLayout? layout;
  const SheetFontText(
    this.text, {
    required this.font,
    this.color = const Color(0xff242424),
    this.bold = false,
    this.layout,
    super.key,
  });
  List<SheetFontRun> get runs => sheetFontRuns(text, {'bold': bold, ...font});
  @override
  SheetFontRenderBox createRenderObject(BuildContext context) =>
      SheetFontRenderBox(
        runs,
        color,
        Directionality.of(context),
        MediaQuery.textScalerOf(context),
        layout,
      );
  @override
  void updateRenderObject(
    BuildContext context,
    SheetFontRenderBox renderObject,
  ) => renderObject.update(
    runs,
    color,
    Directionality.of(context),
    MediaQuery.textScalerOf(context),
    layout,
  );
}

class _FontLayer {
  final double shift;
  final TextPainter painter;
  _FontLayer(this.shift, this.painter);
}

class _RunLine {
  final Rect bounds;
  final double baseline;
  final double pixels;
  final String underline;
  final Color color;
  _RunLine(this.bounds, this.baseline, this.pixels, this.underline, this.color);
}

class SheetFontRenderBox extends RenderBox {
  SheetFontRenderBox(
    this._runs,
    this._color,
    this._direction,
    this._scaler, [
    this._layout,
  ]);
  bool overflowPaint = false;
  List<SheetFontRun> _runs;
  Color _color;
  TextDirection _direction;
  TextScaler _scaler;
  CellTextLayout? _layout;
  List<SheetFontRun> _drawRuns = [];
  Offset _origin = Offset.zero;
  double paintScale = 1, paintRotation = 0;
  Rect textBounds = Rect.zero;
  int get lineCount => _lines.length;
  Offset get textOrigin => _origin;
  Map get _alignment => _layout?.style ?? const {};
  TextDirection get readingDirection => switch (_alignment['readingOrder']) {
    1 => TextDirection.ltr,
    2 => TextDirection.rtl,
    _ =>
      _layout == null
          ? _direction
          : Bidi.startsWithRtl(_text)
          ? TextDirection.rtl
          : TextDirection.ltr,
  };
  String get horizontalAlignment {
    final h = _alignment['horizontalAlignment'] ?? 'left';
    if (h != 'general') return h;
    if (_layout!.number) return 'right';
    if (_layout!.boolean) return 'center';
    return readingDirection == TextDirection.rtl ? 'right' : 'left';
  }

  TextAlign get _textAlign => switch (horizontalAlignment) {
    'right' => TextAlign.right,
    'center' => TextAlign.center,
    'justify' => TextAlign.justify,
    _ => TextAlign.left,
  };
  double get _horizontalFactor => switch (horizontalAlignment) {
    'right' => 1,
    'center' => .5,
    _ => 0,
  };
  double get _verticalFactor => switch (_alignment['verticalAlignment']) {
    'top' => 0,
    'center' => .5,
    _ => _layout == null ? .5 : 1,
  };
  bool get _wrap => _alignment['wrapText'] == true;
  bool get _stacked => _alignment['textRotation'] == 255;
  int? get _maxLines => _layout == null ? 1 : null;
  StrutStyle? get _strut => _layout == null
      ? null
      : StrutStyle.fromTextStyle(
          sheetTextStyle(
            _runs
                .reduce(
                  (a, b) =>
                      sheetTextStyle(a.font).fontSize! >=
                          sheetTextStyle(b.font).fontSize!
                      ? a
                      : b,
                )
                .font,
          ),
        );
  final _painter = TextPainter(maxLines: 1);
  final List<_FontLayer> _layers = [];
  final List<_RunLine> _underlines = [];
  bool _measuringHeight = false;
  bool _measuringWidth = false;
  double _measuredIndent = 0;
  double autoFitWidth() {
    _measuringWidth = true;
    layout(const BoxConstraints());
    return textBounds.width + _measuredIndent;
  }

  /// Uses the grid's shaping, rich runs, indentation and rotation without
  /// allocating paint layers. A quarter-turn fits its natural text length;
  /// constraining that length to the previous row height would be circular.
  double autoFitHeight(double width) {
    _measuringHeight = true;
    layout(BoxConstraints.tightFor(width: math.max(1, width)));
    return math.max(textBounds.height, _minHeight * paintScale);
  }

  double _singleShift = 0;
  List<LineMetrics> _lines = [];
  int layoutCount = 0;
  int get paintLayerCount => math.max(1, _layers.length);
  List<SheetFontRun> get runs => List.unmodifiable(_runs);
  double get fontPixels => _scaler.scale(textStyle.fontSize!);
  double get baselineShift => _shift(_runs.first);
  TextStyle get textStyle => sheetTextStyle(_runs.first.font, color: _color);
  String get underline => _runs.first.font['underline'] as String;
  double get paintedTextWidth => _lines.isEmpty ? 0 : _lines.first.width;

  /// Text actually painted after width adaptation; semantics keep the full text.
  String get paintedText => _drawRuns
      .where((r) => r.font['_numberFormatSkip'] != true)
      .map((r) => r.text)
      .join();
  String get _text => _runs
      .map(
        (r) => r.font['_numberFormatFill'] == true
            ? ''
            : r.font['_numberFormatSkip'] == true
            ? ' '
            : r.text,
      )
      .join();
  double _shift(SheetFontRun run) {
    final pixels = _scaler.scale(sheetTextStyle(run.font).fontSize!);
    return switch (run.font['verticalAlign']) {
      'superscript' => -pixels * .23,
      'subscript' => pixels * .16,
      _ => 0,
    };
  }

  TextStyle _style(SheetFontRun run) {
    var style = sheetTextStyle(run.font, color: _color);
    if (run.font['_numberFormatSkip'] == true) {
      style = style.copyWith(
        color: Colors.transparent,
        decoration: TextDecoration.none,
      );
    }
    return _shift(run) == 0
        ? style
        : style.copyWith(fontSize: style.fontSize! * .65);
  }

  TextSpan _span({double? group}) => TextSpan(
    children: [
      for (final run in _drawRuns)
        TextSpan(
          text: run.text,
          style: group == null || _shift(run) == group
              ? _style(run)
              : _style(run).copyWith(
                  color: Colors.transparent,
                  decorationColor: Colors.transparent,
                ),
        ),
    ],
  );
  double get _minHeight => _runs
      .map((r) => _scaler.scale(sheetTextStyle(r.font).fontSize!) * 1.2)
      .fold(0.0, math.max);
  void update(
    List<SheetFontRun> runs,
    Color color,
    TextDirection direction,
    TextScaler scaler, [
    CellTextLayout? layout,
  ]) {
    if (listEquals(_runs, runs) &&
        _color == color &&
        _direction == direction &&
        _scaler == scaler &&
        _layout == layout) {
      return;
    }
    _runs = runs;
    _color = color;
    _direction = direction;
    _scaler = scaler;
    _layout = layout;
    markNeedsLayout();
    markNeedsSemanticsUpdate();
  }

  void _disposeLayers() {
    for (final layer in _layers) {
      layer.painter.dispose();
    }
    _layers.clear();
  }

  @override
  void performLayout() {
    _disposeLayers();
    _underlines.clear();
    _drawRuns = [
      for (final r in _runs)
        if (r.font['_numberFormatFill'] != true) r,
    ];
    if (_drawRuns.isEmpty) _drawRuns = [SheetFontRun('', _runs.first.font)];
    if (_stacked) {
      _drawRuns = [];
      var offset = 0, runIndex = 0, runEnd = _runs.first.text.length;
      var buffer = StringBuffer(), font = _runs.first.font;
      var first = true;
      for (final character in _text.characters) {
        while (offset >= runEnd && runIndex < _runs.length - 1) {
          runEnd += _runs[++runIndex].text.length;
        }
        if (!mapEquals(font, _runs[runIndex].font)) {
          if (buffer.isNotEmpty) {
            _drawRuns.add(SheetFontRun(buffer.toString(), font));
          }
          buffer = StringBuffer();
          font = _runs[runIndex].font;
        }
        if (!first) buffer.write('\n');
        buffer.write(character);
        first = false;
        offset += character.length;
      }
      if (buffer.isNotEmpty) {
        _drawRuns.add(SheetFontRun(buffer.toString(), font));
      }
    }
    var indent = 0.0;
    if (_layout != null &&
        (_alignment['indent'] as num) > 0 &&
        ['left', 'right', 'distributed'].contains(horizontalAlignment)) {
      final indentPainter = TextPainter(
        text: TextSpan(
          text: '   ',
          style: sheetTextStyle(_layout?.normalFont ?? {}),
        ),
        textDirection: TextDirection.ltr,
        textScaler: _scaler,
      )..layout();
      indent = ((_alignment['indent'] as num).toDouble() * indentPainter.width)
          .clamp(0.0, constraints.maxWidth);
      indentPainter.dispose();
    }
    final available = math.max(0.0, constraints.maxWidth - indent);
    _fitNumberToWidth(available);
    if (!_measuringWidth &&
        !_stacked &&
        available.isFinite &&
        !_numberReplaced &&
        _runs.any((r) => r.font['_numberFormatFill'] == true)) {
      // Excel's *x fills the remaining width; _x is shaped at the actual glyph
      // width and painted transparently. AutoFit measures the unfilled text.
      final natural = TextPainter(
        text: _span(),
        textDirection: readingDirection,
        textScaler: _scaler,
      )..layout();
      final fill = _runs.lastWhere((r) => r.font['_numberFormatFill'] == true);
      final glyph = TextPainter(
        text: TextSpan(text: fill.text, style: _style(fill)),
        textDirection: readingDirection,
        textScaler: _scaler,
      )..layout();
      final count = glyph.width > 0
          ? ((available - natural.width) / glyph.width).floor().clamp(0, 10000)
          : 0;
      _drawRuns = [
        for (final r in _runs)
          if (r.font['_numberFormatFill'] != true)
            r
          else if (identical(r, fill))
            SheetFontRun(r.text * count, r.font),
      ];
      natural.dispose();
      glyph.dispose();
    }
    final rotation = (_alignment['textRotation'] as num?)?.toDouble() ?? 0;
    paintRotation = _stacked ? 0 : -rotation * math.pi / 180;
    final width = _layout == null
        ? constraints.maxWidth
        : _measuringWidth
        ? double.infinity
        : _measuringHeight && rotation.abs() == 90
        ? double.infinity
        : _wrap
        ? (rotation.abs() == 90 && constraints.hasBoundedHeight
              ? constraints.maxHeight
              : available)
        : double.infinity;
    _painter
      ..text = _span()
      ..textDirection = readingDirection
      ..textScaler = _scaler
      ..textAlign = _textAlign
      ..maxLines = _maxLines
      ..strutStyle = _strut
      ..layout(maxWidth: width);
    size = constraints.constrain(
      Size(_painter.width, math.max(_painter.height, _minHeight)),
    );
    _lines = _painter.computeLineMetrics();
    paintScale =
        _layout != null &&
            !_measuringWidth &&
            _alignment['shrinkToFit'] == true &&
            !_wrap &&
            !_stacked &&
            _lines.length == 1 &&
            _painter.width > 0
        ? math.min(1.0, available / _painter.width)
        : 1;
    final w = _painter.width, h = _painter.height;
    final cos = math.cos(paintRotation), sin = math.sin(paintRotation);
    final xs = [0.0, w * cos, -h * sin, w * cos - h * sin],
        ys = [0.0, w * sin, h * cos, w * sin + h * cos];
    final minX = xs.reduce(math.min), minY = ys.reduce(math.min);
    final boxWidth = (xs.reduce(math.max) - minX) * paintScale;
    final boxHeight = (ys.reduce(math.max) - minY) * paintScale;
    if (_measuringHeight || _measuringWidth) {
      textBounds = Rect.fromLTWH(0, 0, boxWidth, boxHeight);
      _measuredIndent = indent;
      layoutCount++;
      return;
    }
    final dx =
        (horizontalAlignment == 'left' ? indent : 0) +
        (available - boxWidth) * _horizontalFactor;
    final dy = (size.height - boxHeight) * _verticalFactor;
    _origin = Offset(dx - minX * paintScale, dy - minY * paintScale);
    textBounds = Rect.fromLTWH(dx, dy, boxWidth, boxHeight);
    final groups = <double>{};
    var start = 0;
    for (final run in _drawRuns) {
      final end = start + run.text.length;
      final underline = run.font['underline'] as String;
      final boxes = _painter.getBoxesForSelection(
        TextSelection(baseOffset: start, extentOffset: end),
      );
      if (overflowPaint ||
          boxes.any(
            (b) => placedBounds(b.toRect()).overlaps(Offset.zero & size),
          )) {
        groups.add(_shift(run));
      }
      if (underline != 'none' && _lines.isNotEmpty) {
        for (final box in boxes.where(
          (b) =>
              overflowPaint ||
              placedBounds(b.toRect()).overlaps(Offset.zero & size),
        )) {
          _underlines.add(
            _RunLine(
              box.toRect(),
              _baselineFor(box.top) + _shift(run),
              _scaler.scale(_style(run).fontSize!),
              underline,
              _style(run).color!,
            ),
          );
        }
      }
      start = end;
    }
    _singleShift = groups.firstOrNull ?? 0;
    if (groups.length > 1) {
      for (final shift in groups) {
        final painter = TextPainter(
          text: _span(group: shift),
          textDirection: readingDirection,
          textScaler: _scaler,
          maxLines: _maxLines,
          strutStyle: _strut,
          textAlign: _textAlign,
        )..layout(maxWidth: width);
        _layers.add(_FontLayer(shift, painter));
      }
    }
    layoutCount++;
  }

  bool _numberReplaced = false;
  void _fitNumberToWidth(double available) {
    _numberReplaced = false;
    final mode = _layout?.numberWidth;
    if (_measuringWidth ||
        _measuringHeight ||
        !available.isFinite ||
        mode == null ||
        _layout?.numericValue == null ||
        _text.isEmpty) {
      return;
    }
    final invalidDate = mode == 'invalidDate';
    if (!invalidDate &&
        (_wrap ||
            (_alignment['textRotation'] as num? ?? 0) != 0 ||
            _alignment['shrinkToFit'] == true)) {
      return;
    }
    final natural = TextPainter(
      text: _span(),
      textDirection: readingDirection,
      textScaler: _scaler,
    )..layout();
    final fits = natural.width <= available + .001;
    natural.dispose();
    if (fits && !invalidDate) return;

    // Use the numeric run's effective font, including conditional/format color;
    // accounting spacing/fill tokens must not become a transparent hash run.
    final font =
        Map<String, dynamic>.from(
          _runs
              .firstWhere(
                (r) =>
                    r.font['_numberFormatSkip'] != true &&
                    r.font['_numberFormatFill'] != true,
                orElse: () => _runs.first,
              )
              .font,
        )..removeWhere(
          (key, _) => key == '_numberFormatSkip' || key == '_numberFormatFill',
        );
    final measure = TextPainter(
      textDirection: readingDirection,
      textScaler: _scaler,
    );
    double width(String text) {
      measure
        ..text = TextSpan(text: text, style: _style(SheetFontRun(text, font)))
        ..layout();
      return measure.width;
    }

    String? replacement;
    if (mode == 'general') {
      for (final candidate in generalWidthCandidates(
        _layout!.numericValue!,
        _text,
      )) {
        if (width(candidate) <= available + .001) {
          replacement = candidate;
          break;
        }
      }
    }
    if (replacement == null) {
      final glyphWidth = width('#');
      final count = glyphWidth > 0
          ? (available / glyphWidth).floor().clamp(1, 10000)
          : 1;
      replacement = '#' * count;
    }
    measure.dispose();
    _drawRuns = [SheetFontRun(replacement, font)];
    _numberReplaced = true;
  }

  Rect placedBounds(Rect bounds) {
    final cos = math.cos(paintRotation), sin = math.sin(paintRotation);
    final points =
        [bounds.topLeft, bounds.topRight, bounds.bottomLeft, bounds.bottomRight]
            .map(
              (p) =>
                  _origin +
                  Offset(p.dx * cos - p.dy * sin, p.dx * sin + p.dy * cos) *
                      paintScale,
            )
            .toList();
    return Rect.fromLTRB(
      points.map((p) => p.dx).reduce(math.min),
      points.map((p) => p.dy).reduce(math.min),
      points.map((p) => p.dx).reduce(math.max),
      points.map((p) => p.dy).reduce(math.max),
    );
  }

  double _baselineFor(double top) {
    var low = 0, high = _lines.length;
    while (low < high) {
      final mid = (low + high) ~/ 2;
      if (_lines[mid].baseline - _lines[mid].ascent < top) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }
    final a = _lines[low.clamp(0, _lines.length - 1)],
        b = _lines[(low - 1).clamp(0, _lines.length - 1)];
    return ((a.baseline - a.ascent - top).abs() <
                (b.baseline - b.ascent - top).abs()
            ? a
            : b)
        .baseline;
  }

  @override
  Size computeDryLayout(BoxConstraints constraints) {
    // A grid cell fills its provided rectangle; no text-dependent intrinsic
    // height should collapse its top/center/bottom alignment area.
    if (_layout != null && constraints.isTight) return constraints.smallest;
    final previous = _drawRuns;
    _drawRuns = [
      for (final r in _runs)
        if (r.font['_numberFormatFill'] != true) r,
    ];
    if (_drawRuns.isEmpty) _drawRuns = [SheetFontRun('', _runs.first.font)];
    final painter = TextPainter(
      text: _span(),
      textDirection: readingDirection,
      textScaler: _scaler,
      maxLines: 1,
    )..layout(maxWidth: constraints.maxWidth);
    final result = constraints.constrain(
      Size(painter.width, math.max(painter.height, _minHeight)),
    );
    painter.dispose();
    _drawRuns = previous;
    return result;
  }

  @override
  double? computeDistanceToActualBaseline(TextBaseline baseline) =>
      _origin.dy +
      (_layers.isEmpty ? _singleShift : 0) +
      _painter.computeDistanceToActualBaseline(baseline);
  @override
  void paint(PaintingContext context, Offset offset) =>
      paintWithClip(context, offset, offset & size);

  /// Paint a neighboring-cell fragment using the owning cell's exact shaping.
  void paintWithClip(PaintingContext context, Offset offset, Rect clip) {
    final canvas = context.canvas;
    canvas.save();
    canvas.clipRect(clip);
    canvas.translate(offset.dx + _origin.dx, offset.dy + _origin.dy);
    canvas.scale(paintScale);
    canvas.rotate(paintRotation);
    if (_layers.isEmpty) {
      _painter.paint(canvas, Offset(0, _singleShift));
    } else {
      for (final layer in _layers) {
        layer.painter.paint(canvas, Offset(0, layer.shift));
      }
    }
    // A uniform accounting underline spans the cell. Mixed run effects stay
    // with their own glyph ranges; they must not underline adjacent plain text.
    final fullAccounting = _runs.every(
      (r) =>
          r.font['underline'] == _runs.first.font['underline'] &&
          r.font['color'] == _runs.first.font['color'],
    );
    for (final line in _underlines) {
      if (!const [
        'single',
        'double',
        'singleAccounting',
        'doubleAccounting',
      ].contains(line.underline)) {
        continue;
      }
      final accounting = line.underline.endsWith('Accounting');
      final doubleLine = line.underline.startsWith('double');
      final stroke = math.max(1.0, line.pixels / 16);
      final gap = stroke * 2;
      final bottom = paintRotation == 0
          ? (size.height - _origin.dy) / math.max(paintScale, .000001)
          : _painter.height;
      final y = accounting
          ? bottom - (doubleLine ? gap : 0) - stroke
          : math.min(
              bottom - (doubleLine ? gap : 0) - stroke,
              line.baseline + math.max(1.0, line.pixels * .07),
            );
      final left = accounting && fullAccounting
          ? (paintRotation == 0
                ? -_origin.dx / math.max(paintScale, .000001)
                : 0.0)
          : line.bounds.left;
      final right = accounting && fullAccounting
          ? (paintRotation == 0
                ? (size.width - _origin.dx) / math.max(paintScale, .000001)
                : _painter.width)
          : line.bounds.right;
      final paint = Paint()
        ..color = line.color
        ..strokeWidth = stroke;
      canvas.drawLine(Offset(left, y), Offset(right, y), paint);
      if (doubleLine) {
        canvas.drawLine(Offset(left, y + gap), Offset(right, y + gap), paint);
      }
      if (accounting && fullAccounting) break;
    }
    canvas.restore();
  }

  @override
  void describeSemanticsConfiguration(SemanticsConfiguration config) {
    super.describeSemanticsConfiguration(config);
    config
      ..label = _text
      ..textDirection = readingDirection
      ..isReadOnly = true;
  }

  @override
  void dispose() {
    _disposeLayers();
    _painter.dispose();
    super.dispose();
  }
}
