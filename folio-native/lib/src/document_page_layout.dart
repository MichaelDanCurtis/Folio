import 'dart:math' as math;
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'document_codec.dart';
import 'document_columns.dart';
import 'document_lists.dart';
import 'document_page_setup.dart';
import 'document_paragraph_format.dart';
import 'document_styles.dart';
import 'document_tables.dart';
import 'document_table_geometry.dart';
import 'document_tabs.dart';
import 'document_text_style.dart';
part 'document_table_page_flow.dart';

/// A derived layout owns its native text painters; it never edits field caches,
/// document text, recovery, or workspace history.
class DocumentPageLayout {
  final List<LaidOutDocumentPage> pages;
  final Set<String> notices;
  final List<TextPainter> _painters;
  final bool converged;
  DocumentPageLayout(this.pages, this.notices, this._painters, this.converged);

  /// Lines of body text, as Word Count reports them.
  int get lineCount => pages
      .expand((page) => page.content)
      .where((p) => p.area == 'body')
      .expand((p) => p.text.split('\n'))
      .where((line) => line.trim().isNotEmpty)
      .length;

  void dispose() {
    for (final painter in _painters) {
      painter.dispose();
    }
  }

  static DocumentPageLayout build(Map snapshot, {Locale? locale}) {
    var fieldValues = <String, String>{};
    var total = 1;
    var sectionCounts = <int, int>{};
    String? previous;
    _PageFlow? result;
    for (var pass = 0; pass < 8; pass++) {
      result?.dispose();
      result = _PageFlow(snapshot, fieldValues, total, sectionCounts, locale);
      try {
        result.run();
      } catch (_) {
        result.dispose();
        rethrow;
      }
      final signature = result.signature;
      if (signature == previous) {
        return DocumentPageLayout(
          result.pages,
          result.notices,
          result.painters,
          true,
        );
      }
      previous = signature;
      fieldValues = result.nextFields;
      total = result.pages.length;
      sectionCounts = result.sectionCounts;
    }
    result!.notices.add(
      'Page fields did not settle. Counts and page breaks may change.',
    );
    return DocumentPageLayout(
      result.pages,
      result.notices,
      result.painters,
      false,
    );
  }
}

class LaidOutDocumentPage {
  final DocumentPageGeometry geometry;
  final EdgeInsets margins;
  final int section, physicalNumber, number;
  final bool blank;
  final List<PagePlacement> content = [];
  final List<DocumentColumnBand> columnBands = [];
  final Set<int> sections = {};
  late double top, bottom;
  LaidOutDocumentPage(
    this.geometry,
    this.margins,
    this.section,
    this.physicalNumber,
    this.number, {
    this.blank = false,
  });
  Size get size => Size(geometry.width, geometry.height);
  String get visibleText =>
      content.map((p) => p.text).where((s) => s.isNotEmpty).join('\n');
}

class DocumentColumnBand {
  final DocumentColumnGeometry geometry;
  final double left, top;
  double bottom, usedBottom;
  bool balanced = false;
  DocumentColumnBand(this.geometry, this.left, this.top, this.bottom)
    : usedBottom = top;
  void paint(Canvas canvas) {
    if (!geometry.separator) return;
    final paint = Paint()
      ..color = Colors.black
      ..strokeWidth = .67;
    for (var i = 0; i + 1 < geometry.columns.length; i++) {
      final a = geometry.columns[i], b = geometry.columns[i + 1];
      final x = left + (a.left + a.width + b.left) / 2;
      canvas.drawLine(Offset(x, top), Offset(x, bottom), paint);
    }
  }
}

class _ColumnResume {
  final int textStart;
  final int? rowIndex;
  final _TableState? table;
  const _ColumnResume({this.textStart = 0, this.rowIndex, this.table});
}

class _ColumnOverflow implements Exception {}

class PagePlacement {
  final _MeasuredBlock _block;
  final Offset offset;
  final double from, to;
  final int? bodyIndex;
  final String area;
  final _TableState? _tableState;
  PagePlacement._(
    this._block,
    this.offset,
    this.from,
    this.to,
    this.bodyIndex,
    this.area, [
    this._tableState,
  ]);
  double get height => to - from;
  late final String text = _block.fragmentText(from, to);
  void paint(Canvas canvas) {
    canvas.save();
    canvas.clipRect(Rect.fromLTWH(offset.dx, offset.dy, _block.width, height));
    canvas.translate(offset.dx, offset.dy - from);
    _block.paint(canvas);
    canvas.restore();
  }
}

Rect? _shiftClip(Rect? clip, Offset delta, Rect? within) {
  final shifted = clip?.shift(delta);
  return shifted == null
      ? within
      : within == null
      ? shifted
      : shifted.intersect(within);
}

class _TextPiece {
  final TextPainter painter;
  final Offset offset;
  final List<({String key, int start, int end, Map field})> fields;
  final Rect? clip;
  _TextPiece(this.painter, this.offset, this.fields, {this.clip});
  _TextPiece shifted(Offset delta, {Rect? within}) => _TextPiece(
    painter,
    offset + delta,
    fields,
    clip: _shiftClip(clip, delta, within),
  );
  void paint(Canvas canvas) {
    if (clip?.isEmpty == true) return;
    canvas.save();
    if (clip != null) canvas.clipRect(clip!);
    painter.paint(canvas, offset);
    var at = 0;
    painter.text!.visitChildren((span) {
      final length = span is TextSpan ? (span.text?.length ?? 0) : 1;
      if (span is q.QuillInlinePaintSpan &&
          (span as q.QuillInlinePaintSpan).hasInlinePaint) {
        for (final box in painter.getBoxesForSelection(
          TextSelection(baseOffset: at, extentOffset: at + length),
        )) {
          (span as q.QuillInlinePaintSpan).paintInline(
            canvas,
            box.toRect().shift(offset),
            TextScaler.noScaling,
          );
        }
      }
      at += length;
      return true;
    });
    canvas.restore();
  }
}

class _CellPiece {
  final Rect rect;
  final Color? fill;
  final Rect? clip;
  _CellPiece(this.rect, this.fill, {this.clip});
  _CellPiece shifted(Offset delta, {Rect? within}) => _CellPiece(
    rect.shift(delta),
    fill,
    clip: _shiftClip(clip, delta, within),
  );
  void paint(Canvas canvas) {
    if (clip?.isEmpty == true) return;
    canvas.save();
    if (clip != null) canvas.clipRect(clip!);
    if (fill != null) canvas.drawRect(rect, Paint()..color = fill!);
    canvas.drawRect(
      rect,
      Paint()
        ..color = const Color(0xffb9c1cc)
        ..style = PaintingStyle.stroke
        ..strokeWidth = .5,
    );
    canvas.restore();
  }
}

class _MeasuredBlock {
  final double width, height;
  final List<double> breaks;
  final List<({double y, String kind})> forceBreaks;
  final List<_TextPiece> text;
  final List<_CellPiece> cells;
  final Map properties;
  final bool paragraph;
  final int textStart;
  final List<double> rowBoundaries;
  final int headerRows;
  final _TableGeometry? tableGeometry;
  _MeasuredBlock(
    this.width,
    this.height,
    this.breaks,
    this.text, {
    this.forceBreaks = const [],
    this.cells = const [],
    this.properties = const {},
    this.paragraph = false,
    this.textStart = 0,
    this.rowBoundaries = const [],
    this.headerRows = 0,
    this.tableGeometry,
  });
  double get headerHeight => headerRows == 0 ? 0 : rowBoundaries[headerRows];
  void paint(Canvas canvas) {
    for (final cell in cells) {
      cell.paint(canvas);
    }
    for (final piece in text) {
      piece.paint(canvas);
    }
  }

  int consumedText(double to) {
    final piece = text.first;
    final plain = piece.painter.plainText;
    var end = 0;
    for (final line in piece.painter.computeLineMetrics()) {
      if (piece.offset.dy + line.baseline + line.descent > to + .01) break;
      final range = piece.painter.getLineBoundary(
        piece.painter.getPositionForOffset(
          Offset(line.left + .01, line.baseline - line.ascent / 2),
        ),
      );
      end = range.end;
      if (end < plain.length && plain[end] == '\n') end++;
    }
    return textStart + end;
  }

  String fragmentText(double from, double to) {
    final out = <String>[];
    for (final piece in text) {
      final plain = piece.painter.plainText;
      for (final line in piece.painter.computeLineMetrics()) {
        final y = piece.offset.dy + line.baseline - line.ascent / 2;
        if (y < from - .01 || y >= to - .01) continue;
        final clip = piece.clip;
        if (clip != null &&
            (clip.isEmpty ||
                y < clip.top ||
                y >= clip.bottom ||
                piece.offset.dx + line.left >= clip.right ||
                piece.offset.dx + line.left + line.width <= clip.left)) {
          continue;
        }
        final range = piece.painter.getLineBoundary(
          piece.painter.getPositionForOffset(
            Offset(line.left + .01, line.baseline - line.ascent / 2),
          ),
        );
        out.add(
          plain.substring(
            range.start.clamp(0, plain.length),
            range.end.clamp(0, plain.length),
          ),
        );
      }
    }
    return out.join('\n');
  }
}

String? formatPageNumber(int value, String format) {
  if (format == 'none') return '';
  if (format == 'decimal') return '$value';
  if (format == 'decimalZero') return value.toString().padLeft(2, '0');
  if (format == 'numberInDash') return '– $value –';
  if (format == 'hex') return value.toRadixString(16).toUpperCase();
  if (['lowerLetter', 'upperLetter'].contains(format)) {
    if (value < 1) return null;
    var n = value, out = '';
    while (n > 0) {
      n--;
      out = String.fromCharCode(65 + n % 26) + out;
      n ~/= 26;
    }
    return format == 'lowerLetter' ? out.toLowerCase() : out;
  }
  if (['lowerRoman', 'upperRoman'].contains(format)) {
    if (value < 1 || value > 3999) return null;
    var n = value, out = '';
    for (final item in [
      (1000, 'M'),
      (900, 'CM'),
      (500, 'D'),
      (400, 'CD'),
      (100, 'C'),
      (90, 'XC'),
      (50, 'L'),
      (40, 'XL'),
      (10, 'X'),
      (9, 'IX'),
      (5, 'V'),
      (4, 'IV'),
      (1, 'I'),
    ]) {
      while (n >= item.$1) {
        out += item.$2;
        n -= item.$1;
      }
    }
    return format == 'lowerRoman' ? out.toLowerCase() : out;
  }
  return null;
}

class _PageFlow {
  final Map snapshot;
  final Map<String, String> fields;
  final int total;
  final Map<int, int> counts;
  final Locale? locale;
  final pages = <LaidOutDocumentPage>[];
  final painters = <TextPainter>[];
  final notices = <String>{};
  final nextFields = <String, String>{};
  final sectionCounts = <int, int>{};
  late List sections;
  late Map document;
  late List blocks;
  late Map<int, String> labels;
  late List<String?> blockLabels;
  double cursor = 0;
  int column = 0;
  late DocumentColumnBand band;
  bool balancingTrial = false;
  int activeSection = 0;
  double get interval =>
      ((document['attrs']?['defaultTabStop'] as num?)?.toDouble() ?? 720) / 15;
  _PageFlow(this.snapshot, this.fields, this.total, this.counts, this.locale);
  void dispose() {
    for (final p in painters) {
      p.dispose();
    }
  }

  String get signature =>
      '${pages.map((p) => '${p.section}:${p.number}:${p.content.map((c) => '${c.bodyIndex}:${c.from}:${c.to}:${c.offset}:${c.text}').join('|')}').join(';')}::$nextFields';
  Map section(int index) => sections[index] as Map;
  ({double left, double width}) get columnGeometry =>
      band.geometry.columns[band.geometry.physicalIndex(column)];
  double get columnTop => band.top;
  double get columnBottom => band.bottom;
  void beginColumns(int index, double top) {
    final page = pages.last;
    final columns = DocumentColumnGeometry.resolve(
      columnSettings(section(index)),
      page.geometry.width - page.margins.horizontal,
    );
    if (columns.notice != null) notices.add(columns.notice!);
    band = DocumentColumnBand(columns, page.margins.left, top, page.bottom);
    page.columnBands.add(band);
    column = 0;
    cursor = top;
  }

  void nextColumn(int index, {bool first = false}) {
    if (column + 1 < band.geometry.columns.length) {
      column++;
      cursor = columnTop;
      pages.last.sections.add(index);
    } else {
      newPage(index, first: first);
    }
  }

  /// Reflow only the tail on the current physical page. Earlier pages and
  /// header/footer painters remain intact while bounded trial heights are tested.
  void balanceSection(int index) {
    if (snapshot['noColumnBalance'] == true ||
        band.geometry.columns.length < 2) {
      return;
    }
    final page = pages.last;
    final firstBlock = (section(index)['firstBlock'] as int? ?? 0).clamp(
      0,
      blocks.length,
    );
    final lastBlock = (section(index)['lastBlock'] as int? ?? blocks.length - 1)
        .clamp(-1, blocks.length - 1);
    final at = page.content.indexWhere(
      (p) =>
          p.area == 'body' &&
          p.bodyIndex! >= firstBlock &&
          p.bodyIndex! <= lastBlock,
    );
    if (at < 0) return;
    final originalContent = List<PagePlacement>.of(page.content);
    final seed = originalContent[at];
    final measured = seed._block;
    _ColumnResume resume;
    if (seed._tableState != null) {
      resume = _ColumnResume(table: seed._tableState);
    } else if (measured.paragraph) {
      if (seed.from > 0 &&
          !measured.breaks.any((y) => (y - seed.from).abs() < .01)) {
        notices.add(
          'An oversized paragraph fragment prevents column balancing.',
        );
        return;
      }
      resume = _ColumnResume(
        textStart: seed.from == 0
            ? measured.textStart
            : measured.consumedText(seed.from),
      );
    } else if (measured.rowBoundaries.isNotEmpty) {
      final row = measured.rowBoundaries.indexWhere(
        (y) => (y - seed.from).abs() < .01,
      );
      if (row < 0) {
        notices.add('An oversized table row prevents column balancing.');
        return;
      }
      resume = _ColumnResume(rowIndex: row);
    } else {
      notices.add('A preserved object prevents column balancing.');
      return;
    }
    final physical = band.geometry.columns.indexWhere(
      (c) => (page.margins.left + c.left - seed.offset.dx).abs() < .01,
    );
    if (physical < 0) return;
    final firstColumn = band.geometry.rtl
        ? band.geometry.columns.length - 1 - physical
        : physical;
    if (firstColumn + 1 == band.geometry.columns.length) return;
    final savedBottom = band.bottom,
        savedUsed = band.usedBottom,
        savedCursor = cursor;
    final savedColumn = column,
        savedFields = Map<String, String>.of(nextFields);
    final savedSections = Set<int>.of(page.sections),
        savedNotices = Set<String>.of(notices);
    final prefix = originalContent
        .take(at)
        .where(
          (p) => !(p.area == 'tableHeader' && p.bodyIndex == seed.bodyIndex),
        )
        .toList();
    final prefixBottom = prefix
        .where((p) => p.area == 'body' && p.offset.dy >= band.top - .01)
        .fold<double>(
          band.top,
          (end, p) => math.max(end, p.offset.dy + p.height),
        );
    var low = 0.0,
        high = math.min(savedBottom - band.top, savedUsed - band.top);
    var bestContent = originalContent, bestFields = savedFields;
    var bestUsed = savedUsed,
        bestCursor = savedCursor,
        bestColumn = savedColumn;
    var bestPainters = <TextPainter>[];
    var bestNotices = savedNotices;
    // A physical page is at most 2112 logical pixels high; 16 bisections
    // locate a fitting line boundary to better than one twip.
    try {
      for (var trial = 0; trial < 16 && high - low > .04; trial++) {
        final height = (low + high) / 2;
        band.bottom = band.top + height;
        band.usedBottom = prefixBottom;
        column = firstColumn;
        cursor = band.top;
        page.content
          ..clear()
          ..addAll(prefix);
        page.sections
          ..clear()
          ..addAll(savedSections);
        nextFields
          ..clear()
          ..addAll(savedFields);
        notices
          ..clear()
          ..addAll(savedNotices);
        final painterStart = painters.length;
        var fits = false;
        balancingTrial = true;
        try {
          layoutBlocks(index, seed.bodyIndex!, lastBlock, resume: resume);
          fits = true;
        } on _ColumnOverflow {
          // This height cannot hold the original tail without adding a page.
        } finally {
          balancingTrial = false;
        }
        final allocated = painters.sublist(painterStart);
        painters.removeRange(painterStart, painters.length);
        if (fits) {
          for (final painter in bestPainters) {
            painter.dispose();
          }
          bestPainters = allocated;
          bestContent = List.of(page.content);
          bestFields = Map.of(nextFields);
          bestNotices = Set.of(notices);
          bestUsed = band.usedBottom;
          bestCursor = cursor;
          bestColumn = column;
          high = height;
        } else {
          for (final painter in allocated) {
            painter.dispose();
          }
          low = height;
        }
      }
    } catch (_) {
      for (final painter in bestPainters) {
        painter.dispose();
      }
      rethrow;
    }
    painters.addAll(bestPainters);
    page.content
      ..clear()
      ..addAll(bestContent);
    page.sections
      ..clear()
      ..addAll(savedSections);
    nextFields
      ..clear()
      ..addAll(bestFields);
    notices
      ..clear()
      ..addAll(bestNotices);
    band.usedBottom = bestUsed;
    band.bottom = bestUsed;
    band.balanced = true;
    cursor = bestCursor;
    column = bestColumn;
  }

  DocumentPageGeometry geometry(int index) => DocumentPageGeometry(
    section(index)['geometry'] as Map? ?? defaultPageGeometry,
    gutterTop: snapshot['gutterTop'] == true,
    rtlGutter: section(index)['rtlGutter'] == true,
  );
  EdgeInsets margins(DocumentPageGeometry g, int physical) {
    final m = g.margins;
    return snapshot['mirrorMargins'] == true && physical.isEven
        ? EdgeInsets.fromLTRB(m.right, m.top, m.left, m.bottom)
        : m;
  }

  String fieldValue(Map field, int index, int number) {
    final a = field['attrs'] as Map? ?? {};
    if (a['locked'] == true) return '${field['text'] ?? '?'}';
    final kind = a['kind'];
    final value = switch (kind) {
      'PAGE' => number,
      'NUMPAGES' => total,
      'SECTIONPAGES' => counts[index] ?? 1,
      'SECTION' => index + 1,
      _ => null,
    };
    if (value == null) return '${field['text'] ?? '?'}';
    final settings = section(index)['pageNumbering'] as Map? ?? {};
    var format = kind == 'PAGE'
        ? settings['format'] as String? ?? 'decimal'
        : 'decimal';
    final instruction = '${a['instruction'] ?? ''}';
    final switches = RegExp(r'\\\*\s+(\w+)').allMatches(instruction);
    for (final match in switches) {
      final word = match.group(1);
      final override = {
        'Arabic': 'decimal',
        'ROMAN': 'upperRoman',
        'roman': 'lowerRoman',
        'ALPHABETIC': 'upperLetter',
        'alphabetic': 'lowerLetter',
        'Hex': 'hex',
      }[word];
      if (override != null) {
        format = override;
      } else if (!['MERGEFORMAT', 'CHARFORMAT'].contains(word)) {
        notices.add('Some field number formats are shown as saved values.');
        return '${field['text'] ?? '?'}';
      }
    }
    final result = formatPageNumber(value, format);
    if (result == null) {
      notices.add('Some page-number formats are shown as saved values.');
      return '${field['text'] ?? '?'}';
    }
    if (kind == 'PAGE' && settings['chapterStyle'] != null) {
      notices.add('Chapter prefixes are awaiting numbered-heading layout.');
      final separator =
          {
            'hyphen': '-',
            'period': '.',
            'colon': ':',
            'emDash': '—',
            'enDash': '–',
          }[settings['chapterSeparator']] ??
          '-';
      return '?$separator$result';
    }
    return result;
  }

  TextPainter painter(
    InlineSpan text,
    double width,
    Map props,
    TextAlign align,
  ) {
    final p = TextPainter(
      text: text,
      textDirection: props['bidi'] == true
          ? TextDirection.rtl
          : TextDirection.ltr,
      textAlign: align,
      textScaler: TextScaler.noScaling,
      locale: locale,
      strutStyle: paragraphStrutStyle(
        props,
        StrutStyle.fromTextStyle(text.style ?? const TextStyle(fontSize: 16)),
      ),
    )..layout(maxWidth: math.max(1, width));
    painters.add(p);
    return p;
  }

  _MeasuredBlock measure(
    Map block,
    double width,
    String key,
    int index,
    int number, {
    Map? previous,
    String? label,
    bool story = false,
    int depth = 0,
    int textStart = 0,
  }) {
    if (depth > 12) {
      throw const FormatException(
        'This document is nested too deeply to preview.',
      );
    }
    if (block['type'] == 'table') {
      return table(block, width, key, index, number, story, depth);
    }
    final supported = ['paragraph', 'heading'].contains(block['type']);
    if (!supported) {
      notices.add(
        'Some preserved objects use a text representation; their layout is not complete.',
      );
    }
    final props = paragraphProperties(block['attrs']?['paragraphFormat']);
    notices.addAll(
      paragraphPreviewLimits(
        props,
      ).map((s) => '$s need further layout support.'),
    );
    final native = nativeParagraphStyle(
      block['attrs']?['paragraphFormat']?['inheritedRun'] is! Map &&
              block['type'] == 'heading'
          ? (block['attrs']?['level'] as int? ?? 1)
          : null,
    );
    final inherited =
        block['attrs']?['paragraphFormat']?['inheritedRun'] as Map? ?? {};
    final base = paragraphStyle(
      props,
      inheritedRunStyle(inherited, native.style),
    );
    double twips(Map p, String k, double fallback) =>
        p[k] is num ? math.max(0, (p[k] as num).toDouble() / 15) : fallback;
    final left = twips(props, props.containsKey('start') ? 'start' : 'left', 0);
    final right = twips(props, props.containsKey('end') ? 'end' : 'right', 0);
    final old = paragraphProperties(previous?['attrs']?['paragraphFormat']);
    final oldNative = nativeParagraphStyle(
      previous?['type'] == 'heading'
          ? previous?['attrs']?['level'] as int? ?? 1
          : null,
    );
    final top = textStart > 0
        ? 0.0
        : math.max(
            0.0,
            twips(props, 'before', native.verticalSpacing.top) -
                (previous == null
                    ? 0
                    : twips(old, 'after', oldNative.verticalSpacing.bottom)),
          );
    final bottom = twips(props, 'after', native.verticalSpacing.bottom);
    final spans = <InlineSpan>[];
    final sites = <({String key, int start, int end, Map field})>[];
    var position = 0;
    final manualBreaks = <({int offset, String kind})>[];
    if (label != null) {
      final value = '$label ';
      if (textStart < value.length) {
        spans.add(TextSpan(text: value.substring(textStart)));
      }
      position += value.length;
    }
    final inline = supported
        ? block['content'] as List? ?? []
        : [
            {'type': 'text', 'text': nodeText(block)},
          ];
    for (var i = 0; i < inline.length; i++) {
      final n = inline[i] as Map;
      var style = const TextStyle();
      final attrs = <String, dynamic>{};
      for (final mark in n['marks'] as List? ?? []) {
        final type = mark['type'];
        final a = mark['attrs'] as Map? ?? {};
        if (['bold', 'italic', 'underline', 'strike'].contains(type)) {
          attrs[type] = a['value'] != false;
        }
        if (type == 'textStyle') {
          attrs['size'] = a['fontSize'];
          style = style.copyWith(
            fontFamily: a['fontFamily'] as String?,
            color: wordTextColor(a['color']),
          );
        }
        if (type == 'highlight') {
          style = style.copyWith(backgroundColor: wordTextColor(a['color']));
        }
        if (type == 'link') {
          attrs['link'] = a['href'];
          style = style.copyWith(color: const Color(0xff185abd));
        }
        if (['superscript', 'subscript'].contains(type)) {
          notices.add(
            'Superscript and subscript placement needs further layout support.',
          );
        }
      }
      style = directRunStyle(inherited, attrs, style);
      final id = '$key:$i';
      final value = n['type'] == 'pageField'
          ? (story
                ? fieldValue(n, index, number)
                : fields[id] ?? fieldValue(n, index, number))
          : n['type'] == 'hardBreak'
          ? '\n'
          : '${n['text'] ?? ''}';
      if (n['type'] == 'hardBreak' &&
          ['page', 'column'].contains(n['attrs']?['breakType'])) {
        if (position >= textStart) {
          manualBreaks.add((
            offset: position - textStart,
            kind: n['attrs']['breakType'] as String,
          ));
        }
      }
      if (n['type'] == 'pageField' && position + value.length > textStart) {
        sites.add((
          key: id,
          start: math.max(0, position - textStart),
          end: position + value.length - textStart,
          field: n,
        ));
      }
      if (position + value.length > textStart) {
        spans.add(
          TextSpan(
            text: value.substring(math.max(0, textStart - position)),
            style: style,
          ),
        );
      }
      position += value.length;
    }
    if (spans.isEmpty) spans.add(const TextSpan(text: ' '));
    final available = math.max(1.0, width - left - right);
    final text = layoutTabStops(
      TextSpan(style: base, children: spans),
      maxWidth: available,
      origin: left,
      interval: interval,
      stops: effectiveTabStops(block['attrs']?['tabs']),
      textDirection: props['bidi'] == true
          ? TextDirection.rtl
          : TextDirection.ltr,
      locale: locale,
    );
    final align = switch (block['attrs']?['textAlign'] ??
        block['attrs']?['paragraphFormat']?['inheritedAlignment']) {
      'center' => TextAlign.center,
      'right' || 'end' => TextAlign.right,
      'justify' || 'both' => TextAlign.justify,
      _ => TextAlign.left,
    };
    final p = painter(text, available, props, align);
    final height = top + p.height + bottom;
    final breaks = p
        .computeLineMetrics()
        .map((l) => top + l.baseline + l.descent)
        .toList();
    if (breaks.isEmpty) {
      breaks.add(height);
    } else {
      breaks[breaks.length - 1] = height;
    }
    final forced = <({double y, String kind})>[];
    final metrics = p.computeLineMetrics();
    for (final breakAt in manualBreaks) {
      final caret = p.getOffsetForCaret(
        TextPosition(offset: breakAt.offset),
        Rect.zero,
      );
      final line = metrics
          .where((l) => caret.dy < l.baseline + l.descent)
          .firstOrNull;
      if (line != null) {
        forced.add((y: top + line.baseline + line.descent, kind: breakAt.kind));
      }
    }
    return _MeasuredBlock(
      width,
      height,
      breaks,
      [_TextPiece(p, Offset(left, top), sites)],
      properties: props,
      forceBreaks: forced,
      paragraph: true,
      textStart: textStart,
    );
  }

  _MeasuredBlock table(
    Map node,
    double available,
    String key,
    int index,
    int number,
    bool story,
    int depth,
  ) {
    final grid = DocumentTable(node);
    if (grid.rows.any((r) => r['attrs']?['rowBreakStyleUnsupported'] == true)) {
      notices.add(
        'Some cell or corner table-style rules for row page breaks need further layout support.',
      );
    }
    bool hasManualBreak(Map value) =>
        value['type'] == 'hardBreak' &&
            ['page', 'column'].contains(value['attrs']?['breakType']) ||
        (value['content'] as List? ?? []).whereType<Map>().any(hasManualBreak);
    if (hasManualBreak(node)) {
      notices.add(
        'Manual breaks inside table cells are not shown in this preview.',
      );
    }
    final widths = List<num>.from(
      node['attrs']?['columnWidths'] ?? List.filled(grid.columns, 2400),
    );
    final sum = widths.fold<double>(0, (a, b) => a + b);
    final natural = node['attrs']?['layout'] == 'fixed' ? sum / 15 : available;
    if (natural > available + .01) {
      notices.add('Some tables are wider than the page text area.');
    }
    final width = math.max(1.0, natural);
    final xs = <double>[0];
    for (final w in widths) {
      xs.add(xs.last + width * w / sum);
    }
    final contents = <_MeasuredBlock>[];
    for (final cell in grid.cells) {
      final value = storyBlock(
        {'content': cell.cell['content']},
        math.max(1.0, xs[cell.col + cell.cols] - xs[cell.col] - 16),
        '$key:${cell.row}:${cell.col}',
        index,
        number,
        story,
        depth + 1,
      );
      contents.add(value);
    }
    final heights = documentTableRowHeights(
      [for (final row in grid.rows) row['attrs']?['height'] as Map?],
      grid.cells,
      [for (final content in contents) content.height + 16],
    );
    final ys = <double>[0];
    for (final h in heights) {
      ys.add(ys.last + h);
    }
    final pieces = <_TextPiece>[];
    final cells = <_CellPiece>[];
    for (var i = 0; i < grid.cells.length; i++) {
      final cell = grid.cells[i], body = contents[i];
      final rect = Rect.fromLTRB(
        xs[cell.col],
        ys[cell.row],
        xs[cell.col + cell.cols],
        ys[cell.row + cell.rows],
      );
      cells.add(_CellPiece(rect, wordTextColor(cell.cell['attrs']?['fill'])));
      final extra = rect.height - 16 - body.height;
      final dy = switch (cell.cell['attrs']?['verticalAlign']) {
        'center' => extra / 2,
        'bottom' => extra,
        _ => 0.0,
      };
      final offset = rect.topLeft + Offset(8, 8 + dy);
      for (final t in body.text) {
        pieces.add(t.shifted(offset, within: rect));
      }
      for (final nested in body.cells) {
        cells.add(nested.shifted(offset, within: rect));
      }
    }
    final breaks = <double>[];
    for (var r = 1; r <= grid.rows.length; r++) {
      if (!grid.cells.any((c) => c.row < r && c.row + c.rows > r)) {
        breaks.add(ys[r]);
      }
    }
    var headerRows = grid.headerRows;
    if (headerRows > 0 &&
        grid.cells.any(
          (c) => c.row < headerRows && c.row + c.rows > headerRows,
        )) {
      notices.add(
        'A repeating table header crosses a merged body row. Include the whole merged row group in the header.',
      );
      headerRows = 0;
    }
    return _MeasuredBlock(
      width,
      ys.last,
      breaks,
      pieces,
      cells: cells,
      rowBoundaries: ys,
      headerRows: headerRows,
      tableGeometry: _TableGeometry(grid, xs, ys, contents),
    );
  }

  _MeasuredBlock storyBlock(
    Map doc,
    double width,
    String key,
    int index,
    int number,
    bool story,
    int depth,
  ) {
    final blocks = doc['content'] as List? ?? [];
    final pieces = <_TextPiece>[];
    final cells = <_CellPiece>[];
    var y = 0.0;
    Map? previous;
    for (var i = 0; i < blocks.length; i++) {
      final b = measure(
        blocks[i] as Map,
        width,
        '$key:$i',
        index,
        number,
        previous: previous,
        story: story,
        depth: depth,
      );
      for (final t in b.text) {
        pieces.add(t.shifted(Offset(0, y)));
      }
      for (final c in b.cells) {
        cells.add(c.shifted(Offset(0, y)));
      }
      y += b.height;
      previous = blocks[i] as Map;
    }
    return _MeasuredBlock(width, y, [y], pieces, cells: cells);
  }

  LaidOutDocumentPage newPage(
    int index, {
    bool first = false,
    bool blank = false,
  }) {
    if (balancingTrial) throw _ColumnOverflow();
    if (pages.length >= 2000) {
      throw const FormatException(
        'Page preview currently supports up to 2,000 pages.',
      );
    }
    final g = geometry(index), m = margins(g, pages.length + 1);
    final restart = first
        ? (section(index)['pageNumbering']?['start'] as int?)
        : null;
    final number = restart ?? (pages.isEmpty ? 1 : pages.last.number + 1);
    final page = LaidOutDocumentPage(
      g,
      m,
      index,
      pages.length + 1,
      number,
      blank: blank,
    );
    page.sections.add(index);
    page.top = m.top;
    page.bottom = g.height - m.bottom;
    pages.add(page);
    if (!blank) {
      final variant = first && section(index)['differentFirstPage'] == true
          ? 'first'
          : snapshot['differentOddEven'] == true && number.isEven
          ? 'even'
          : 'default';
      final width = g.width - m.horizontal;
      for (final kind in ['header', 'footer']) {
        final doc = section(index)[kind]?[variant] as Map?;
        if (doc == null) continue;
        final block = storyBlock(
          doc,
          width,
          '$kind:${pages.length}',
          index,
          number,
          true,
          0,
        );
        if (block.text.every((t) => t.painter.plainText.trim().isEmpty)) {
          continue;
        }
        final distance = g.twips(kind).clamp(0, g.twips('height')) / 15;
        final y = kind == 'header'
            ? distance
            : g.height - distance - block.height;
        page.content.add(
          PagePlacement._(
            block,
            Offset(m.left, y),
            0,
            block.height,
            null,
            kind,
          ),
        );
        if (kind == 'header') {
          page.top = math.max(page.top, y + block.height);
        } else {
          page.bottom = math.min(page.bottom, y);
        }
      }
      if (page.bottom - page.top < 24) {
        notices.add(
          'Headers or footers leave too little space for body content.',
        );
        page.bottom = math.max(page.top + 24, page.bottom);
      }
    }
    beginColumns(index, page.top);
    return page;
  }

  void run() {
    document = snapshot['document'] as Map;
    blocks = document['content'] as List? ?? [];
    sections = snapshot['sections'] as List? ?? [];
    if (sections.isEmpty) {
      sections = [
        {
          'geometry': defaultPageGeometry,
          'firstBlock': 0,
          'lastBlock': math.max(0, blocks.length - 1),
        },
      ];
    }
    if (snapshot['hasPendingRevisions'] == true) {
      notices.add(
        'Pending tracked changes use the preserved review text; final pagination needs review resolution.',
      );
    }
    labels = documentListLabels(documentToDelta(document));
    blockLabels = [];
    var offset = 0;
    for (final block in blocks) {
      blockLabels.add(labels[offset]);
      offset += documentToDelta({
        'type': 'doc',
        'content': [block],
      }).length;
    }
    for (var si = 0; si < sections.length; si++) {
      activeSection = si;
      final start = section(si)['geometry']?['start'] ?? 'nextPage';
      final g = geometry(si);
      if (si == 0) {
        newPage(si, first: true);
      } else if (['continuous', 'nextColumn'].contains(start) &&
          pages.last.geometry.width == g.width &&
          pages.last.geometry.height == g.height &&
          pages.last.margins == margins(g, pages.last.physicalNumber)) {
        final sharedPage = pages.last;
        final closesColumns =
            start == 'continuous' &&
            band.geometry.columns.length > 1 &&
            snapshot['noColumnBalance'] != true;
        if (closesColumns) balanceSection(si - 1);
        final sameColumns =
            jsonEncode(columnSettings(section(si - 1))) ==
            jsonEncode(columnSettings(section(si)));
        if (start == 'nextColumn' && sameColumns) {
          nextColumn(si, first: true);
        } else if (!sameColumns || closesColumns) {
          band.bottom = band.usedBottom;
          if (start == 'nextColumn' ||
              pages.last.bottom - band.usedBottom < 24) {
            if (start == 'nextColumn') {
              notices.add(
                'A new-column section with changed column geometry begins on a new preview page.',
              );
            }
            newPage(si, first: true);
          } else {
            beginColumns(si, band.usedBottom);
          }
        }
        pages.last.sections.add(si);
        if (identical(pages.last, sharedPage) &&
            section(si)['pageNumbering']?['start'] != null) {
          notices.add(
            'Restarted numbering in a continuous section sharing a page needs further layout support.',
          );
        }
      } else {
        if (start == 'continuous') {
          notices.add(
            'A continuous section with changed page geometry begins on a new preview page.',
          );
        }
        if (start == 'oddPage' || start == 'evenPage') {
          final odd = start == 'oddPage';
          if ((pages.length + 1).isOdd != odd) newPage(si - 1, blank: true);
        }
        newPage(si, first: true);
      }
      final first = (section(si)['firstBlock'] as int? ?? 0).clamp(
        0,
        blocks.length,
      );
      final last = (section(si)['lastBlock'] as int? ?? blocks.length - 1)
          .clamp(-1, blocks.length - 1);
      layoutBlocks(si, first, last);
    }
    for (var i = 0; i < sections.length; i++) {
      sectionCounts[i] = pages.where((p) => p.sections.contains(i)).length;
    }
  }

  void layoutBlocks(int si, int first, int last, {_ColumnResume? resume}) {
    Map? previous;
    final sectionFirst = section(si)['firstBlock'] as int? ?? 0;
    for (var index = first - 1; index >= sectionFirst; index--) {
      if (blocks[index]['attrs']?['hiddenReviewMarker'] != true) {
        previous = blocks[index] as Map;
        break;
      }
    }
    for (var bi = first; bi <= last; bi++) {
      final node = blocks[bi] as Map;
      if (node['attrs']?['hiddenReviewMarker'] == true) continue;
      if (node['type'] == 'table') {
        _flowTable(node, bi, si, bi == first ? resume : null);
        previous = node;
        continue;
      }
      final prior = previous;
      _MeasuredBlock measureHere({int? textStart}) => measure(
        node,
        columnGeometry.width,
        'body:$bi',
        si,
        pages.last.number,
        previous: prior,
        label: blockLabels[bi],
        textStart: textStart ?? (bi == first ? resume?.textStart ?? 0 : 0),
      );
      var block = measureHere();
      previous = node;
      if (!(bi == first && resume != null) &&
          block.properties['pageBreakBefore'] == true &&
          (column > 0 || cursor > pages.last.top + .01)) {
        newPage(si);
        block = measureHere();
      }
      final space = columnBottom - cursor,
          capacity = columnBottom - columnTop,
          physicalCapacity = pages.last.bottom - columnTop;
      if (block.properties['keepLines'] == true &&
          block.height <= physicalCapacity &&
          block.height > space) {
        if (balancingTrial && block.height > capacity) throw _ColumnOverflow();
        if (cursor > columnTop + .01) {
          nextColumn(si);
          block = measureHere();
        }
      }
      if (block.properties['keepNext'] == true && bi < last) {
        final next = measure(
          blocks[bi + 1] as Map,
          columnGeometry.width,
          'lookahead:$bi',
          si,
          pages.last.number,
          previous: node,
          label: blockLabels[bi + 1],
        );
        final needed = block.height + (next.breaks.firstOrNull ?? next.height);
        if (needed <= physicalCapacity && needed > columnBottom - cursor) {
          if (balancingTrial && needed > capacity) throw _ColumnOverflow();
          if (cursor > columnTop + .01) {
            nextColumn(si);
            block = measureHere();
          }
        }
      }
      var from = bi == first && resume?.rowIndex != null
          ? block.rowBoundaries[resume!.rowIndex!]
          : 0.0;
      (int, int)? lastTableTarget;
      while (from < block.height - .01) {
        if ((block.width - columnGeometry.width).abs() > .01 &&
            (from == 0 || block.breaks.any((b) => (b - from).abs() < .01))) {
          if (block.paragraph) {
            final start = from == 0
                ? block.textStart
                : block.consumedText(from);
            block = measureHere(textStart: start);
            from = 0;
          } else if (block.rowBoundaries.isNotEmpty) {
            final row = from == 0
                ? 0
                : block.rowBoundaries.indexWhere((b) => (b - from).abs() < .01);
            if (row >= 0) {
              block = measureHere();
              from = block.rowBoundaries[row];
            }
          }
        }
        final target = (pages.last.physicalNumber, column);
        _MeasuredBlock? repeatedHeader;
        if (block.headerRows > 0 &&
            from >= block.headerHeight - .01 &&
            lastTableTarget != target) {
          final headerNode = {
            ...node,
            'content': (node['content'] as List)
                .take(block.headerRows)
                .toList(),
          };
          repeatedHeader = table(
            headerNode,
            columnGeometry.width,
            'repeated:$bi:${target.$1}:${target.$2}',
            si,
            pages.last.number,
            true,
            0,
          );
          final nextBreak =
              block.breaks.where((b) => b > from + .01).firstOrNull ??
              block.height;
          final needed = repeatedHeader.height + nextBreak - from;
          if (needed > columnBottom - cursor + .01) {
            if (balancingTrial) throw _ColumnOverflow();
            if (needed <= pages.last.bottom - columnTop + .01 &&
                cursor > columnTop + .01) {
              nextColumn(si);
              continue;
            }
            notices.add(
              'A table header and its next row group do not fit together; the header is omitted on that continuation.',
            );
            repeatedHeader = null;
          }
        }
        final available = columnBottom - cursor - (repeatedHeader?.height ?? 0);
        final forced = block.forceBreaks
            .where((b) => b.y > from + .01)
            .firstOrNull;
        var choices = block.breaks
            .where(
              (b) =>
                  b > from + .01 &&
                  b - from <= available + .01 &&
                  (forced == null || b <= forced.y + .01),
            )
            .toList();
        if (from == 0 &&
            block.headerRows > 0 &&
            block.headerRows + 1 < block.rowBoundaries.length &&
            choices.isNotEmpty &&
            choices.last <= block.headerHeight + .01) {
          // A header must travel with at least the next complete body row group
          // when those rows fit in a fresh column.
          final withBody = block.breaks
              .where((b) => b > block.headerHeight + .01)
              .firstOrNull;
          if (withBody != null &&
              withBody <= pages.last.bottom - columnTop + .01) {
            if (balancingTrial) throw _ColumnOverflow();
            if (cursor > columnTop + .01) choices = [];
          }
        }
        if (block.paragraph &&
            block.properties['widowControl'] != false &&
            (forced == null ||
                choices.isEmpty ||
                choices.last < forced.y - .01) &&
            choices.isNotEmpty &&
            choices.last < block.height - .01) {
          final remaining = block.breaks
              .where((b) => b > choices.last + .01)
              .length;
          if (remaining == 1 && choices.length > 1) choices.removeLast();
          if (from == 0 &&
              block.textStart == 0 &&
              choices.length < 2 &&
              block.breaks.length > 1) {
            if (balancingTrial) throw _ColumnOverflow();
            if (cursor > columnTop + .01) choices = [];
          }
        }
        if (choices.isEmpty && cursor > columnTop + .01) {
          nextColumn(si);
          continue;
        }
        if (choices.isEmpty && balancingTrial) throw _ColumnOverflow();
        final to = choices.isEmpty
            ? math.min(block.height, from + math.max(1.0, available))
            : choices.last;
        if (choices.isEmpty) {
          notices.add(
            'An oversized line or table row crosses a page boundary.',
          );
        }
        final page = pages.last;
        page.sections.add(si);
        if (repeatedHeader != null) {
          page.content.add(
            PagePlacement._(
              repeatedHeader,
              Offset(page.margins.left + columnGeometry.left, cursor),
              0,
              repeatedHeader.height,
              bi,
              'tableHeader',
            ),
          );
          cursor += repeatedHeader.height;
        }
        page.content.add(
          PagePlacement._(
            block,
            Offset(page.margins.left + columnGeometry.left, cursor),
            from,
            to,
            bi,
            'body',
          ),
        );
        for (final t in block.text) {
          for (final site in t.fields) {
            final boxes = t.painter.getBoxesForSelection(
              TextSelection(baseOffset: site.start, extentOffset: site.end),
            );
            final box = boxes
                .where(
                  (b) =>
                      t.clip == null ||
                      t.clip!.overlaps(b.toRect().shift(t.offset)),
                )
                .firstOrNull;
            if (box == null && t.clip != null) continue;
            final y =
                t.offset.dy + (box == null ? 0 : (box.top + box.bottom) / 2);
            if (y >= from - .01 && y < to - .01) {
              nextFields[site.key] = fieldValue(site.field, si, page.number);
            }
          }
        }
        cursor += to - from;
        band.usedBottom = math.max(band.usedBottom, cursor);
        lastTableTarget = target;
        from = to;
        if (from < block.height - .01) {
          if (forced != null &&
              (to - forced.y).abs() < .01 &&
              forced.kind == 'page') {
            newPage(si);
          } else {
            nextColumn(si);
          }
        }
      }
    }
  }
}
